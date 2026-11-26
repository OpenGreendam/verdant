//! Virtualized keyboard note table: the semantic source for notes and curve
//! points. NoteId/PointId form GPUI IDs; visible array positions never do.
//!
//! T focuses the note list. Tab enters its current note's points; Shift-Tab
//! returns to Arrange. Points use Tab/Shift-Tab or Left/Right for navigation,
//! Home/End and PageUp/PageDown for larger jumps, Up/Down for one undoable
//! value step, and Escape/Enter to return to notes. Tab at either end also
//! returns to notes; re-entry restores the same PointId, not its old index.
use std::collections::HashMap;

use gpui::{AccessibleAction, Context, Entity, FocusHandle, IntoElement, KeyDownEvent, Role, SharedString, Window, div, prelude::*, px, uniform_list};
use verdant_backend::params::Param;
use verdant_domain::{Note, NoteId, PointId, TrackId};
use crate::editor::{Bottom, Editor, MenuAction};
use crate::generated::controls::{CommandArg, NumericId};

#[derive(Clone, PartialEq, Eq, Hash)]
struct PointKey {
    param: Param,
    id: PointId,
}

/// Only the current note owns retained point focus. Handles are acquired on
/// first display/navigation and survive virtualization and value edits.
#[derive(Default)]
pub struct TableFocus {
    note: Option<(TrackId, NoteId)>,
    revision: Option<u64>,
    order: Vec<PointKey>,
    positions: HashMap<PointKey, usize>,
    handles: HashMap<PointKey, FocusHandle>,
    cursor: Option<PointKey>,
    cursor_index: usize,
}

impl TableFocus {
    fn sync(&mut self, track: &TrackId, note: Option<&Note>, revision: u64, window: &mut Window, table: &FocusHandle, cx: &mut Context<Editor>) {
        let same_note = match (&self.note, note) {
            (Some((old_track, old_note)), Some(note)) => old_track == track && old_note == &note.id,
            (None, None) => true,
            _ => false,
        };
        if same_note && self.revision == Some(revision) { return; }
        let focused = self.handles.iter().find(|(_, handle)| handle.is_focused(window)).map(|(key, _)| key.clone());
        if let Some(key) = &focused {
            self.cursor = Some(key.clone());
            self.cursor_index = self.positions.get(key).copied().unwrap_or(0);
        }
        if !same_note {
            self.handles.clear();
            self.cursor = None;
            self.cursor_index = 0;
        }
        self.note = note.map(|n| (track.clone(), n.id.clone()));
        self.revision = Some(revision);
        self.order.clear();
        self.positions.clear();
        if let Some(note) = note {
            for param in verdant_backend::params::ALL {
                for (id, _, _) in param.point_samples(note) {
                    let key = PointKey { param, id };
                    self.positions.insert(key.clone(), self.order.len());
                    self.order.push(key);
                }
            }
        }
        self.handles.retain(|key, _| self.positions.contains_key(key));
        let retained = self.cursor.as_ref().and_then(|key| self.positions.get(key)).copied();
        self.cursor_index = retained.unwrap_or(self.cursor_index).min(self.order.len().saturating_sub(1));
        self.cursor = self.order.get(self.cursor_index).cloned();
        // A removed row must not leave native focus attached to an absent
        // point. Return to notes; Tab will reach the retained/adjacent point.
        if focused.is_some() {
            if !same_note || retained.is_none() {
                window.focus(table, cx);
            }
        }
    }

    fn handle(&mut self, key: &PointKey, cx: &mut Context<Editor>) -> FocusHandle {
        self.handles.entry(key.clone()).or_insert_with(|| cx.focus_handle()).clone()
    }
}

fn table_revision(ed: &Editor) -> String {
    format!("{}:{}:{}", ed.document_generation, ed.revision().get(), ed.session.track)
}


pub fn note_table(ed: &mut Editor, entity: Entity<Editor>, _window: &mut Window, _cx: &mut Context<Editor>) -> crate::proto_collection::Collection {
    let count = ed.active_track().map_or(0, |track| track.notes.len());
    let revision = table_revision(ed);
    let props = serde_json::json!({"mode":"table", "label":"音符表", "cursor":ed.table_cursor,
        "columns":5, "revision":revision, "autoFocus":false, "disabled":false});
    let draw_editor = entity.clone(); let basis = revision.clone();
    let draw = std::rc::Rc::new(move |handle: &verdant_collection_control::CollectionControlHandle, focus: &FocusHandle, engine: Option<crate::proto_scroll::EngineBinding>, window: &mut Window, cx: &mut gpui::App| {
        draw_editor.update(cx, |ed, cx| {
            let was_focused = ed.focus_table.is_focused(window);
            ed.focus_table = focus.clone();
            if was_focused { window.focus(focus, cx); }
            native_note_table(ed, cx.entity(), handle.clone(), basis.clone(), engine.clone().expect("note viewport binding"), window, cx).into_any_element()
        })
    });
    let editor = entity.downgrade();
    let sink = std::rc::Rc::new(move |name: &str, payload: &serde_json::Value, window: &mut Window, cx: &mut gpui::App| {
        let _ = editor.update(cx, |ed, cx| {
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(table_revision(ed).as_str()) { return; }
            let index = crate::proto_controls::source_index(payload.get("index")).unwrap_or(0);
            let extend = payload.get("extend").and_then(serde_json::Value::as_bool).unwrap_or(false);
            match name {
                "select" => ed.table_select(index, extend, cx),
                "activate" => {
                    ed.table_select(index, extend, cx);
                    let id = ed.active_track().and_then(|track| track.notes.get(index)).map(|note| note.id.clone());
                    if let Some(id) = id { ed.bottom = Bottom::Editor; ed.begin_lyric(id, window, cx); }
                },
                "command" => {
                    match payload.get("key").and_then(serde_json::Value::as_str).unwrap_or("") {
                        "Tab" => {
                            if extend || ed.table_focus.order.is_empty() { window.focus(&ed.focus_arrange, cx); }
                            else { focus_point(ed, ed.table_focus.cursor_index, window, cx); }
                        },
                        "Delete" | "Backspace" => ed.run_menu_action(MenuAction::DeleteSelection, window, cx),
                        key @ ("ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight") => {
                            let project = ed.store().snapshot();
                            let ds = match key { "ArrowUp" => if extend { 12 } else { 1 }, "ArrowDown" => if extend { -12 } else { -1 }, _ => 0 };
                            let grid = if extend { ed.project().ppq * 4 } else { ed.session.snap.grid_ticks.max(1) };
                            let dt = match key { "ArrowLeft" => -grid, "ArrowRight" => grid, _ => 0 };
                            let effects = ed.session.nudge_selection(&project, ed.revision(), dt, ds);
                            ed.apply_effects(effects, window, cx);
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        });
    });
    crate::proto_collection::collection("compiled-note-table", props, vec![1;count], draw, sink)
        .scroll_engine(crate::proto_scroll::Engine::Uniform(ed.table_scroll.clone()))
}

fn native_note_table(ed: &mut Editor, entity: Entity<Editor>, handle: verdant_collection_control::CollectionControlHandle,
    revision: String, engine: crate::proto_scroll::EngineBinding, window: &mut Window, cx: &mut Context<Editor>) -> gpui::AnyElement {

    let project = ed.store().snapshot();
    let note = project.track(&ed.session.track).and_then(|t| t.notes.get(ed.table_cursor));
    let project_revision = ed.revision().get();
    ed.table_focus.sync(&ed.session.track, note, project_revision, window, &ed.focus_table, cx);
    let count = ed.active_track().map(|t| t.notes.len()).unwrap_or(0);
    crate::proto_collection::reveal_uniform_item("note-table-reveal",&handle,ed.table_cursor,&revision,count,&ed.table_scroll,window,cx);
    let track = ed.session.track.to_string();
    let notes = entity.clone();
    let point_note = note.map(|n| n.id.clone());
    let point_count = ed.table_focus.order.len();
    let points = entity;
    let key_handle = handle.clone();
    div().id(SharedString::from(format!("note-table-{track}")))
        .aria_description("上下键选择，Shift 扩展选择，Home/End 跳转，Ctrl 方向键移动音符，Enter 编辑歌词。Tab 进入当前音符控制点，Shift Tab 返回轨道区。")
        .key_context("NoteTable")
        .size_full().flex().flex_col().bg(ed.theme.surface).rounded_bl(crate::window_chrome::corner_radii(ed.window_chrome).bottom_left)
        .on_key_down(move |event, window, cx| crate::proto_collection::key(&key_handle, event, window, cx))
        .child(div().h(px(32.0)).px(px(12.0)).flex().items_center().gap(px(20.0)).text_sm()
            .child(format!("音符表 · {count} 个音符"))
            .child("起点 / 长度 / 音高 / 歌词 · Enter 编辑 · Tab 控制点 · Shift Tab 轨道区"))
        .child(uniform_list(SharedString::from(format!("note-rows-{track}")), count, move |range, _, cx| {
            notes.update(cx, |ed, cx| range.filter_map(|i| note_row(ed, i, &handle, &revision, cx)).collect::<Vec<_>>())
        }).track_scroll(&ed.table_scroll).on_scroll_wheel(move |_,window,cx| engine.notify_input(window,cx)).flex_1().min_h_0())
        .child(div().h(px(28.0)).px(px(12.0)).flex().items_center().text_xs().text_color(ed.theme.text_secondary)
            .child(format!("全部参数曲线 · {point_count} 点 · Tab / ←→ 选点 · ↑↓ 步进，Shift 精调 PIT/DYN · Esc 返回音符")))
        .child(div().h(px(140.0)).min_h_0().children(point_note.map(|note| point_collection(ed, points, note)))).into_any_element()

}

fn note_row(ed: &Editor, index: usize, collection: &verdant_collection_control::CollectionControlHandle, revision: &str, cx: &mut Context<Editor>) -> Option<gpui::Stateful<gpui::Div>> {
    let note = ed.active_track()?.notes.get(index)?;
    let (id, name) = (note.id.clone(), verdant_domain::timing::note_name(note.midi_note));
    let current = index == ed.table_cursor;
    let selected = ed.session.selection.contains(&id);
    let focus = collection.clone(); let focus_revision = revision.to_string();
    let increment = collection.clone(); let increment_revision = revision.to_string();
    let decrement = collection.clone(); let decrement_revision = revision.to_string();
    let select = collection.clone(); let select_revision = revision.to_string();
    let reading = verdant_domain::pronunciation_display(&note.pronunciation);
    let label = format!("音符 {}，{}，起点 {} tick，长度 {} tick，歌词 {}，读音 {}{}", id, name, note.start_tick, note.duration_tick,
        note.lyric, reading, if ed.error_notes.contains(&id) { "，渲染预检错误" } else { "" });
    Some(div().id(SharedString::from(format!("note-{id}"))).role(Role::Row).accessibility_id(SharedString::from(format!("note-{id}")))
        .aria_label(label).aria_selected(selected).aria_row_index(index).aria_position_in_set(index + 1)
        .aria_size_of_set(ed.active_track()?.notes.len()).when(current, |d| d.aria_active_descendant())
        .h(px(34.0)).px(px(12.0)).flex().items_center().gap(px(12.0)).text_sm()
        .bg(if selected { crate::theme::mix(ed.theme.surface, ed.theme.accent, 0.18) } else { ed.theme.surface })
        .border_b_1().border_color(ed.theme.shade)
        .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            crate::proto_collection::call(&focus, "selectAndFocus", vec![index.into(), false.into(), focus_revision.clone().into()], window, cx);
        })
        .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
            crate::proto_collection::call(&increment, "keyAt", vec![index.into(), "ArrowUp".into(), false.into(), true.into(), increment_revision.clone().into()], window, cx);
        })
        .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
            crate::proto_collection::call(&decrement, "keyAt", vec![index.into(), "ArrowDown".into(), false.into(), true.into(), decrement_revision.clone().into()], window, cx);
        })
        .on_click(move |event: &gpui::ClickEvent, window, cx| {
            crate::proto_collection::call(&select, "selectAndFocus", vec![index.into(), event.modifiers().shift.into(), select_revision.clone().into()], window, cx);
        })
        .child(div().id("start").role(Role::Cell).w(px(100.0)).child(note.start_tick.to_string()))
        .child(div().id("duration").role(Role::Cell).w(px(100.0)).child(note.duration_tick.to_string()))
        .child(crate::view::numeric_field(ed, NumericId::NotePitchSemitone, CommandArg::Note(id), "pitch", cx))
        .child(div().id("lyric").role(Role::Cell).flex_1().truncate().child(format!("{} / {}", note.lyric, reading))))
}

fn focus_point(ed: &mut Editor, index: usize, window: &mut Window, cx: &mut Context<Editor>) {
    let Some(key) = ed.table_focus.order.get(index).cloned() else { return };
    let handle = ed.table_focus.handle(&key, cx);
    ed.table_focus.cursor = Some(key);
    ed.table_focus.cursor_index = index;
    window.focus(&handle, cx);
    cx.notify();
}

fn focus_point_key(ed: &mut Editor, note: &NoteId, key: &PointKey, window: &mut Window, cx: &mut Context<Editor>) {
    if !ed.table_focus.note.as_ref().is_some_and(|(track, id)| track == &ed.session.track && id == note) { return; }
    if let Some(&index) = ed.table_focus.positions.get(key) { focus_point(ed, index, window, cx); }
}

fn point_revision(ed: &Editor, note: &NoteId) -> String { format!("{}:{note}", table_revision(ed)) }

fn point_collection(ed: &Editor, entity: Entity<Editor>, note: NoteId) -> crate::proto_collection::Collection {
    let count = ed.table_focus.order.len();
    let revision = point_revision(ed, &note);
    let props = serde_json::json!({"mode":"points", "label":"曲线控制点", "cursor":ed.table_focus.cursor_index,
        "revision":revision, "autoFocus":false, "disabled":false});
    let draw_editor = entity.clone(); let draw_note = note.clone(); let basis = revision.clone();
    let draw = std::rc::Rc::new(move |handle: &verdant_collection_control::CollectionControlHandle, _focus: &FocusHandle, engine: Option<crate::proto_scroll::EngineBinding>, window: &mut Window, cx: &mut gpui::App| {
        let handle = handle.clone(); let editor = draw_editor.clone(); let note = draw_note.clone(); let revision = basis.clone();
        let engine = engine.expect("point viewport binding");
        draw_editor.update(cx, |ed, cx| {
            crate::proto_collection::reveal_uniform_item("curve-point-reveal",&handle,ed.table_focus.cursor_index,&revision,count,&ed.point_scroll,window,cx);
            uniform_list(SharedString::from(format!("compiled-point-rows-{note}")), count, move |range, _, cx| {
                editor.update(cx, |ed, cx| range.filter_map(|index| point_row(ed, &note, index, &handle, &revision, cx)).collect::<Vec<_>>())
            }).track_scroll(&ed.point_scroll).on_scroll_wheel(move |_,window,cx| engine.notify_input(window,cx)).size_full().into_any_element()
        })
    });
    let editor = entity.downgrade();
    let sink = std::rc::Rc::new(move |name: &str, payload: &serde_json::Value, window: &mut Window, cx: &mut gpui::App| {
        let _ = editor.update(cx, |ed, cx| {
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(point_revision(ed, &note).as_str()) { return; }
            if name == "close" { window.focus(&ed.focus_table, cx); cx.notify(); }
            if name == "select" || name == "activate" {
                if let Some(index) = crate::proto_controls::source_index(payload.get("index")) { focus_point(ed, index, window, cx); }
            }
        });
    });
    crate::proto_collection::collection("compiled-curve-point-list", props, vec![1;count], draw, sink)
        .scroll_engine(crate::proto_scroll::Engine::Uniform(ed.point_scroll.clone()))
}

fn point_row(ed: &mut Editor, note: &NoteId, index: usize, collection: &verdant_collection_control::CollectionControlHandle,
    revision: &str, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    use crate::proto_scalar::{self, EventKind, Sample, Spec};
    let n = ed.active_track().and_then(|track| ed.index.note(track, note))?;
    let selected = ed.table_focus.order.get(index)?;
    let param = selected.param;
    let (point, tick, value) = param.point_samples(n).into_iter().find(|point| point.0 == selected.id)?;
    let step = if param == Param::Dyn { 0.1 } else { 1.0 };
    let fine_step = if param.is_expression() { step } else { step * 0.1 };
    let unit = match param { Param::Pit => "cents", Param::Dyn => "dB", _ => "/127" };
    let key = SharedString::from(format!("point-{note}-{}-{point}", param.short()));
    let identity = PointKey { param, id: point.clone() };
    let label = if param == Param::Portamento { format!("POR 音符 {note}，按音符起点生效") }
        else { format!("{} 控制点，音符 {note}，偏移 {tick} tick", param.short()) };
    let (min, max) = match param { Param::Pit => (-1200., 1200.), Param::Dyn => (-24., 6.), _ => (0., 127.) };
    let mut spec = Spec::slider(value, min, max, label);
    spec.role = "spinbutton"; spec.step = step; spec.fine_step = fine_step; spec.key_mode = "table-point";
    spec.drag_mode = "relative"; spec.tap_action = "activate";
    spec.disabled = !verdant_backend::params::support(param, ed.route(), ed.scheduler.capabilities.as_ref()).is_effective()
        || (crate::editor::curves::region_parameter(param) && crate::editor::curves::lane_disabled(n, param));
    let editor = cx.entity(); let focus_editor = editor.clone();
    let action_note = note.clone(); let focus_note = note.clone(); let action_identity = identity.clone();
    let valid_note = note.clone(); let valid_point = point.clone(); let track = ed.session.track.clone();
    let key_collection = collection.clone(); let key_revision = revision.to_string(); let th = ed.theme;
    Some(proto_scalar::scalar(editor, key.clone(), spec,
        move |sample, _, initial| Sample { value: initial + sample.total_delta.x * step,
            fine: initial + sample.total_delta.x * fine_step, jump: false },
        move |display, actions, _, _| {
            let collection = key_collection.clone(); let revision = key_revision.clone();
            div().id(key.clone()).key_context("NoteTablePoint").h(px(32.)).px(px(12.)).flex().items_center().text_sm()
                .bg(if display.focus_visible { crate::theme::mix(th.surface, th.accent, 0.18) } else { th.surface })
                .child(if param == Param::Portamento { format!("POR · 按音符起点生效 · {:.0} {unit}", display.value) }
                    else { format!("{} · {tick} tick · {:.1} {unit}", param.short(), display.value) })
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if actions.key(event, window, cx) { return; }
                    crate::proto_collection::key_at(&collection, index, &revision, event, window, cx);
                }).into_any_element()
        },
        move |ed, event, window, cx| {
            if event.kind == EventKind::Commit { ed.step_curve_point(&action_note, &point, param, event.value - value, cx); }
            if event.kind == EventKind::Activate { focus_point_key(ed, &action_note, &action_identity, window, cx); }
        })
        .valid_when(move |ed| ed.session.track == track && ed.active_track().and_then(|track| ed.index.note(track, &valid_note))
            .is_some_and(|note| param.point_samples(note).iter().any(|point| point.0 == valid_point)))
        .on_focus_handle(move |focus, window, cx| {
            focus_editor.update(cx, |ed, cx| {
                if !ed.table_focus.note.as_ref().is_some_and(|(track, note)| track == &ed.session.track && note == &focus_note) { return; }
                let previous = ed.table_focus.handles.insert(identity.clone(), focus.clone());
                if previous.is_some_and(|previous| previous.is_focused(window) && previous != *focus) { window.focus(focus, cx); }
            });
        }).into_any_element())
}

#[cfg(test)]
mod native_reveal_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, size};
    use std::rc::Rc;
    use verdant_collection_control::{CollectionControlHandle,native::gpui_native_owner::{all_owners,resolved}};

    struct Harness {cursor:usize,scroll:gpui::UniformListScrollHandle,last_row:Rc<std::cell::RefCell<Option<gpui::Bounds<gpui::Pixels>>>>}
    impl Render for Harness {
        fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            let entity=cx.entity();let draw_entity=entity.clone();
            let draw:crate::proto_collection::Draw=Rc::new(move|source,_,engine,window,cx| {
                draw_entity.update(cx,|host,cx| {
                    crate::proto_collection::reveal_uniform_item("uniform-test-reveal",source,host.cursor,"native-table",64,&host.scroll,window,cx);
                    let engine=engine.expect("same uniform engine");
                    let last_row=host.last_row.clone();
                    uniform_list("measured-native-rows",64,move|range,_,_|range.map(|index|div().relative().h(px(32.)).child(format!("Row {index}"))
                        .when(index==63,|row| {let measured=last_row.clone();
                            // Anchor the measuring canvas to the row; auto
                            // insets would use its static position after text.
                            row.child(gpui::canvas(move|bounds,_,_| {*measured.borrow_mut()=Some(bounds);},|_,_,_,_|{}).absolute().inset_0().size_full())})).collect::<Vec<_>>())
                        .track_scroll(&host.scroll).on_scroll_wheel(move|_,window,cx|engine.notify_input(window,cx)).size_full().into_any_element()
                })
            });
            let sink:crate::proto_collection::Sink=Rc::new(move|name,payload,_,cx| {
                if name=="select" { if let Some(index)=crate::proto_controls::source_index(payload.get("index")) {
                    entity.update(cx,|host,cx|{host.cursor=index;cx.notify();});
                }}
            });
            crate::proto_collection::collection("native-table-reveal",serde_json::json!({"mode":"table","label":"Measured table reveal","cursor":self.cursor,"revision":"native-table"}),vec![1;64],draw,sink)
                .scroll_engine(crate::proto_scroll::Engine::Uniform(self.scroll.clone()))
        }
    }
    fn source(window:&Window)->CollectionControlHandle {
        CollectionControlHandle{owner:all_owners().into_iter().find(|owner| {
            let owner=owner.borrow();owner.mounted&&owner.window_id==Some(window.window_handle().window_id())
                &&owner.modules.accessible_text.get("name").is_some_and(|name|resolved(name).text()=="Measured table reveal")
        }).expect("mounted measured collection")}
    }
    #[gpui::test]
    fn native_uniform_selection_reveal_uses_measured_offscreen_rows_and_source_policy(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(280.),px(160.)),|_,_|Harness{cursor:0,scroll:gpui::UniformListScrollHandle::new(),last_row:Default::default()});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        cx.update_window(window.into(),|_,window,cx| {
            assert_eq!(source(window).call("selectIndex",vec![63.into(),false.into(),"native-table".into()],window,cx).unwrap(),serde_json::json!(true));
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,_| {
            assert_eq!(host.cursor,63);
            let state=host.scroll.0.borrow();let offset=state.base_handle.offset().y;
            assert!(offset<px(-1800.),"offscreen row reveal is decided by the source on the real virtualizer engine");
            assert!(state.deferred_scroll_to_item.is_none(),"no native ScrollStrategy is queued");
            let row=host.last_row.borrow().expect("the formerly offscreen row is rendered by the native virtualizer");
            let viewport=state.base_handle.bounds();
            assert_eq!(row.size.height,px(32.),"the probe measures the complete native row");
            assert!(row.top()>=viewport.top()-px(1.)&&row.bottom()<=viewport.bottom()+px(1.),
                "the actual last-row bounds are inside the viewport: row={row:?}, viewport={viewport:?}, offset={offset:?}, max_offset={:?}, measured_size={:?}",
                state.base_handle.max_offset(),state.last_item_size.map(|size|(size.item,size.contents)));
            // ScrollSurface publishes a normalized fraction; raw pixel offset
            // remains a native engine fact and is checked independently above.
            let fraction=source(window).state("scrollPosition").unwrap().as_f64().unwrap();
            let actual=f64::from(-offset.as_f32()/state.base_handle.max_offset().y.as_f32());
            assert!((0.0..=1.0).contains(&fraction));
            assert!((fraction-actual).abs()<1e-6,"source scroll fraction must reflect this exact native engine: source={fraction}, actual={actual}");
        }).unwrap();
        cx.update_window(window.into(),|_,window,cx| {
            source(window).call("selectIndex",vec![0.into(),false.into(),"native-table".into()],window,cx).unwrap();
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_| {
            assert_eq!(host.cursor,0);assert_eq!(host.scroll.0.borrow().base_handle.offset().y,px(0.));
        }).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, Render, Subscription, TestAppContext, size};

    struct Harness { editor: Entity<Editor>, _changed: Subscription }
    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.editor.update(cx, |ed, cx| note_table(ed, cx.entity(), window, cx))
        }
    }

    #[gpui::test]
    fn native_note_table_selects_first_row_and_collapses_a_range_at_the_current_cursor(cx: &mut TestAppContext) {
        let mut project = verdant_domain::Project::new_empty();
        let notes = vec![Note::new(0, 240, 60), Note::new(480, 240, 62), Note::new(960, 240, 64)];
        let ids = notes.iter().map(|note| note.id.clone()).collect::<Vec<_>>();
        project.tracks[0].notes = notes;
        project.tracks[0].refresh_index();
        let window = cx.open_window(size(px(640.0), px(360.0)), |window, cx| {
            let editor = cx.new(|cx| Editor::for_native_test(project, window, cx));
            let changed = cx.observe(&editor, |_, _, cx| cx.notify());
            Harness { editor, _changed: changed }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let editor = host.editor.read(cx);
            assert_eq!(editor.table_cursor, 0);
            assert!(editor.session.selection.is_empty(), "initial cursor is not yet a domain selection");
        }).unwrap();
        for (index, extend, count) in [(0, false, 1), (2, true, 3), (2, false, 1)] {
            // The production list has a 32 px header and 34 px rows. Use the
            // start-time cell, away from its independently editable pitch.
            let position = gpui::point(px(20.0), px(32.0 + index as f32*34.0 + 17.0));
            cx.update_window(window.into(), |_, window, cx| {
                let modifiers = gpui::Modifiers { shift: extend, ..Default::default() };
                window.dispatch_event(gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                    position, button: gpui::MouseButton::Left, modifiers, click_count: 1, ..Default::default()
                }), cx);
                window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                    position, button: gpui::MouseButton::Left, modifiers, click_count: 1, ..Default::default()
                }), cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                let editor = host.editor.read(cx);
                assert_eq!(editor.table_cursor, index);
                assert_eq!(editor.session.selection.len(), count, "row={index}, extend={extend}");
                assert!(editor.session.selection.contains(&ids[index]));
                if extend { assert!(ids.iter().all(|id| editor.session.selection.contains(id))); }
                assert!(editor.focus_table.is_focused(window));
                assert_eq!(editor.revision().get(), 0, "selection never edits the document");
            }).unwrap();
        }
        for (key, count) in [("shift-home", 3), ("home", 1)] {
            cx.simulate_keystrokes(window.into(), key);
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, _, cx| {
                let editor = host.editor.read(cx);
                assert_eq!(editor.table_cursor, 0);
                assert_eq!(editor.session.selection.len(), count, "{key} must explicitly select at the unchanged endpoint");
                assert!(editor.session.selection.contains(&ids[0]));
                assert_eq!(editor.revision().get(), 0);
            }).unwrap();
        }
    }

    #[gpui::test]
    fn native_point_table_separates_scalar_value_keys_from_collection_navigation(cx: &mut TestAppContext) {
        let mut project = verdant_domain::Project::new_empty();
        let mut note = Note::new(0, 960, 60);
        note.curves.gain_millidecibel_points = (0..10).map(|index| verdant_domain::model::GainPoint {
            id: PointId::generate(), tick: index * 48, millidecibels: 0, unknown: Default::default(),
        }).collect();
        project.tracks[0].notes = vec![note];
        project.tracks[0].refresh_index();
        let window = cx.open_window(size(px(640.), px(360.)), |window, cx| {
            let editor = cx.new(|cx| {
                let mut editor = Editor::for_native_test(project, window, cx);
                editor.scheduler.capabilities = Some(verdant_backend::Capabilities::default());
                editor
            });
            let changed = cx.observe(&editor, |_, _, cx| cx.notify());
            Harness { editor, _changed: changed }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        let original_revision = window.update(cx, |host, window, cx| {
            let editor = host.editor.read(cx);
            let focus = editor.focus_table.clone(); let revision = editor.revision();
            window.focus(&focus, cx); revision
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "tab");
        crate::native_test_support::frames(window, cx);
        for (key, index) in [("right", 1), ("pagedown", 5), ("end", 9), ("home", 0)] {
            cx.simulate_keystrokes(window.into(), key);
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                let editor = host.editor.read(cx);
                assert_eq!(editor.table_focus.cursor_index, index, "{key} must traverse the point collection");
                let point = &editor.table_focus.order[index];
                assert!(editor.table_focus.handles[point].is_focused(window), "{key} must focus the selected compiled scalar");
                assert_eq!(editor.revision(), original_revision, "navigation must not propose a numeric edit");
                if index >= 5 {
                    assert!(editor.point_scroll.0.borrow().base_handle.offset().y < px(0.), "virtual point navigation must reveal its row");
                }
            }).unwrap();
        }
        cx.simulate_keystrokes(window.into(), "up");
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "shift-down");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let editor = host.editor.read(cx);
            assert_eq!(editor.project().tracks[0].notes[0].curves.gain_millidecibel_points[0].millidecibels, 90);
            assert_eq!(editor.table_focus.cursor_index, 0, "value steps must retain the point cursor");
            assert!(!editor.focus_table.is_focused(window));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let editor = host.editor.read(cx);
            assert!(editor.focus_table.is_focused(window), "Enter returns from a scalar to the note collection");
            assert!(editor.lyric.is_none(), "point Enter must not activate the ancestor note editor");
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "tab escape");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).focus_table.is_focused(window), "idle scalar Escape belongs to the collection");
        }).unwrap();
    }
}
