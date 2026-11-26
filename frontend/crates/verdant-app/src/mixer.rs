//! Mixer strips bind persisted inserts to real isolated hosts. Fader/pan
//! previews use audio atomics; one release is one undoable edit. Retained
//! child meters repaint independently of these controls and insert slots.
//! Master bus owns a real Master Fader, Master Pan, Master Mute, Master
//! Inserts and stereo peak/clip metering shared with offline WAV export.
use crate::ui_controls::AdwStates;

use gpui::{
    AccessibleAction, BorderStyle, Context, DispatchPhase, Entity, FontWeight,
    HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Role,
    SharedString, canvas, div, fill, point, prelude::*, px, quad,
};
use verdant_domain::TrackId;
use verdant_plugins::{InsertSlot, InsertSpec, InsertStatus};

use crate::editor::{Editor, MenuAction, MixerControl};
use crate::generated::controls::{CommandArg, CommandId, NumericId};
use crate::menu::MenuItem;
use crate::paint::{Texts, b, flush_texts};
use crate::theme::{Theme, mix};
use crate::ui_controls::{numeric_pill, selectable_button};
use crate::view::command_button;

pub const STRIP_W: f32 = 196.0;

pub fn fader_pos(mdb: i32) -> f64 {
    (10f64.powf(mdb as f64 / 20_000.0) / 2.0).powf(0.25).clamp(0.0, 1.0)
}

pub fn fader_mdb(pos: f64) -> i32 {
    if pos <= 0.02 {
        -96_000
    } else {
        (20_000.0 * (2.0 * pos.clamp(0.0, 1.0).powi(4)).log10()).round().clamp(-96_000.0, 6_000.0) as i32
    }
}

fn peak_db_label(peaks: (f32, f32)) -> (String, bool) {
    let p = peaks.0.max(peaks.1);
    if p >= 0.999 {
        ("CLIP".into(), true)
    } else if p <= 1e-4 {
        ("-∞".into(), false)
    } else {
        (format!("{:+.1}", 20.0 * p.log10()), false)
    }
}

pub fn mixer(ed: &Editor, entity: &Entity<Editor>, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    div()
        .id("mixer")
        .size_full()
        .flex()
        .bg(th.view)
        .p(px(8.0))
        .gap(px(8.0))
        .child(div().flex_1().min_w_0().h_full().child(MixerTracks { editor:entity.clone() }))
        .child(master_strip(ed, entity, cx))
        .children(parameter_panel(ed, cx))
}


fn mixer_revision(ed: &Editor) -> String {
    format!("{}:{}:{}", ed.project().project_id, ed.document_generation, ed.revision())
}

#[derive(Clone, Debug)]
enum ToggleTarget {
    TrackMute(TrackId), TrackSolo(TrackId), TrackAutomation(TrackId), MasterMute, MasterAutomation,
}
impl ToggleTarget {
    fn current(&self, ed: &Editor) -> Option<bool> {
        match self {
            Self::TrackMute(track) => ed.project().track(track).map(|track| track.muted),
            Self::TrackSolo(track) => ed.project().track(track).map(|track| track.solo),
            Self::TrackAutomation(track) => ed.project().track(track).map(|_| ed.automation_ui.expanded_tracks.contains(track)),
            Self::MasterMute => Some(crate::mix::MasterBusSettings::from_project(ed.project()).muted),
            Self::MasterAutomation => Some(ed.automation_ui.show_master),
        }
    }
    fn accept(&self, ed: &mut Editor, selected: bool, cx: &mut Context<Editor>) {
        match self {
            Self::TrackMute(track) => ed.edit_header(track, if selected { "静音" } else { "取消静音" }, cx, |header| header.muted = selected),
            Self::TrackSolo(track) => ed.edit_header(track, if selected { "独奏" } else { "取消独奏" }, cx, |header| header.solo = selected),
            Self::TrackAutomation(track) => {
                if selected { ed.automation_ui.expanded_tracks.insert(track.clone()); }
                else { ed.automation_ui.expanded_tracks.remove(track); }
            },
            Self::MasterMute => ed.edit_master_bus("主总线静音", cx, |settings| settings.muted = selected),
            Self::MasterAutomation => ed.automation_ui.show_master = selected,
        }
        cx.notify();
    }
}

/// Source-owned switch proposal with the same persisted or UI-only domain
/// transaction as the previous command. The domain never recomputes `!value`.
fn toggle_button(ed: &Editor, target: ToggleTarget, label: impl Into<SharedString>, key: impl Into<SharedString>, cx: &mut Context<Editor>) -> crate::proto_controls::Action {
    let selected = target.current(ed).unwrap_or(false);
    let basis = mixer_revision(ed);
    selectable_button(key, label, selected, ed.theme).role(Role::Switch)
        .authority(format!("{basis}:{target:?}"))
        .on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, _, cx| {
            // Retained/queued actions can outlive a project switch or a newer
            // accepted value even when the native element key is unchanged.
            if mixer_revision(ed) != basis || target.current(ed) != Some(selected) { return; }
            if event.selected() != selected { target.accept(ed, event.selected(), cx); }
        }))
}

/// Both master-insert surfaces consume the same source selection proposal.
/// The editor owns slot identity, pending host work and the persisted write.
pub(crate) fn master_insert_bypass_button(ed: &Editor, slot: usize, spec: &InsertSpec, key: &'static str, cx: &mut Context<Editor>) -> crate::proto_controls::Action {
    let basis = mixer_revision(ed);
    let runtime = ed.live_master_insert(slot).map(|handle| handle.key());
    let busy = ed.inserts_loading || runtime.is_some_and(|key| ed.plugin_busy.contains(&key));
    let expected = spec.clone();
    route_button(key, if spec.bypass { "启用" } else { "旁通" }, spec.bypass, ed.theme)
        .role(Role::Switch)
        .aria_label(format!("主总线插入 {} 旁通", slot + 1))
        .authority(format!("{basis}:master:{slot}:{}:{runtime:?}", spec.instance_id))
        .disabled(busy)
        .on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, _, cx| {
            cx.stop_propagation();
            if mixer_revision(ed) != basis { return; }
            ed.accept_master_insert_bypass(slot, &expected, runtime, event.selected(), cx);
        }))
}

#[derive(IntoElement)]
struct MixerTracks { editor: Entity<Editor> }
impl gpui::RenderOnce for MixerTracks {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let ed = self.editor.read(cx);
        let revision = mixer_revision(ed);
        let count = ed.project().tracks.len();
        let cursor = ed.project().tracks.iter().position(|track| track.id == ed.session.track).unwrap_or(0);
        let props = serde_json::json!({"mode":"tracklist", "label":"混音轨道", "cursor":cursor,
            "revision":revision, "orientation":"horizontal", "disabled":ed.menu.is_some() || ed.text_prompt.is_some() || ed.modal_dialog.is_some()});
        let engine = window.use_keyed_state(SharedString::from(format!("mixer-engine-{:?}",self.editor.entity_id())),cx,|_,_|gpui::ScrollHandle::new());
        let source = std::rc::Rc::new(std::cell::RefCell::new(None::<(verdant_collection_control::CollectionControlHandle,crate::proto_scroll::EngineBinding)>));
        let source_draw = source.clone(); let entity = self.editor.clone(); let draw_revision = revision.clone();
        let draw: crate::proto_collection::Draw = std::rc::Rc::new(move |handle, focus, engine, _, cx| {
            let engine = engine.expect("mixer collection viewport");
            *source_draw.borrow_mut() = Some((handle.clone(),engine.clone()));
            let physical = engine.handle(cx); let keyboard = handle.clone();
            entity.update(cx, |ed, cx| {
                let rows = ed.project().tracks.iter().enumerate().map(|(index,track)| {
                    let header = ed.mixer_drag.as_ref().filter(|drag|drag.track==track.id).map(|drag|drag.header.clone()).unwrap_or_else(||track.header());
                    strip(ed,&entity,index,track.id.clone(),header,handle,&draw_revision,cx).into_any_element()
                }).collect::<Vec<_>>();
                div().id("mixer-strips").size_full().overflow_x_scroll().track_scroll(&physical).track_focus(focus).flex().gap(px(8.0))
                    .on_key_down(move |event,window,cx|crate::proto_collection::key(&keyboard,event,window,cx))
                    .on_scroll_wheel(move |_,window,cx|engine.notify_input(window,cx)).children(rows).into_any_element()
            })
        });
        let entity = self.editor.downgrade(); let basis = revision.clone();
        let sink: crate::proto_collection::Sink = std::rc::Rc::new(move |name,payload,window,cx| {
            if payload.get("revision").and_then(serde_json::Value::as_str)!=Some(basis.as_str()){return;}
            let Some(index)=crate::proto_controls::source_index(payload.get("index"))else{return;};
            let Some((source,engine))=source.borrow().clone()else{return;};
            if name=="command"&&payload.get("key").and_then(serde_json::Value::as_str)==Some("reveal") {
                crate::proto_collection::reveal_item(source,index,basis.clone(),engine,true,window);return;
            }
            if !matches!(name,"select"|"activate"|"context"){return;}
            let _=entity.update(cx,|ed,cx|{
                if mixer_revision(ed)!=basis{return;}
                let Some(track)=ed.project().tracks.get(index).map(|track|track.id.clone())else{return;};
                if !ed.select_track(track.clone(),cx){crate::proto_collection::call(&source,"reconcile",vec![],window,cx);return;}
                if name=="context" {
                    let anchor=if payload.get("keyboard").and_then(serde_json::Value::as_bool)==Some(true){
                        let physical=engine.handle(cx);let viewport=physical.bounds();
                        physical.bounds_for_item(index).map(|bounds|{
                            let x=(bounds.center().x+physical.offset().x).clamp(viewport.left(),viewport.right());
                            point(x,viewport.top()+px(24.))
                        }).unwrap_or_else(||window.mouse_position())
                    }else{point(px(payload.get("x").and_then(serde_json::Value::as_f64).unwrap_or(0.)as f32),px(payload.get("y").and_then(serde_json::Value::as_f64).unwrap_or(0.)as f32))};
                    ed.track_menu(track,anchor,window,cx);
                }
            });
        });
        crate::proto_collection::collection("mixer-track-list",props,vec![1;count],draw,sink)
            .scroll_engine(crate::proto_scroll::Engine::Native(engine))
    }
}

fn insert_state(loading: bool, runtime: Option<&InsertSlot>, spec: &InsertSpec) -> (String, bool) {
    if loading {
        return ("正在重建插件链…".into(), false);
    }
    match runtime {
        Some(InsertSlot::Live(handle)) if handle.spec().id == spec.id && handle.spec().format == spec.format => match handle.status() {
            InsertStatus::Running => (
                format!(
                    "{} · DSP {} + 传输 {} 帧 · xrun {}",
                    if spec.bypass { "旁通" } else { "运行" },
                    handle.plugin_latency_samples(),
                    handle.transport_latency_samples(),
                    handle.xruns()
                ),
                false,
            ),
            InsertStatus::Crashed { detail } => (format!("崩溃已旁通 · {detail}"), true),
        },
        Some(InsertSlot::Missing { reason, .. }) => (format!("缺失 · {reason}"), true),
        _ => ("正在载入插件…".into(), false),
    }
}

fn master_strip(ed: &Editor, entity: &Entity<Editor>, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    let specs = ed.master_insert_specs();
    let master = ed.master_bus();
    let (peak_str, clipped) = peak_db_label(ed.master_meter);
    let mut slots = crate::proto_scroll::viewport("master-insert-slots", "主总线插入效果器", "vertical").h(px(108.0)).flex_none().flex().flex_col().gap(px(3.0));
    for (slot, spec) in specs.iter().enumerate() {
        let runtime = ed.master_insert_slots().get(slot);
        let (state, warning) = insert_state(ed.inserts_loading, runtime, spec);
        let busy = ed.inserts_loading || matches!(runtime, Some(InsertSlot::Live(handle)) if ed.plugin_busy.contains(&handle.key()));
        slots = slots.child(
            div()
                .id(SharedString::from(format!("master-insert-{slot}")))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .rounded(px(8.0))
                .bg(th.control)
                .p(px(4.0))
                .child(
                    crate::proto_controls::action("master-insert-parameters",format!("主总线插入 {}，{}，{}", slot + 1, spec.name, state)).authority(mixer_revision(ed))
                        .aria_description("打开主总线插件原生界面")
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(div().text_xs().truncate().child(format!("{} · {}", slot + 1, spec.name)))
                        .child(div().text_size(px(9.0)).truncate().text_color(if warning { th.warning } else { th.text_secondary }).child(state))
                        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                            ed.open_master_insert_editor(slot, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(3.0))
                        .child(
                            master_insert_bypass_button(ed, slot, spec, "master-insert-bypass", cx),
                        )
                        .child(
                            route_button("master-insert-remove", "移除", false, th).authority(mixer_revision(ed))
                                .disabled(busy)
                                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                                    if !busy { ed.remove_master_insert(slot, cx); }
                                    cx.stop_propagation();
                                })),
                        ),
                ),
        );
    }
    if specs.is_empty() {
        slots = slots.child(div().px(px(6.0)).py(px(4.0)).text_xs().text_color(th.text_tertiary).child("无主总线插入"));
    }

    div()
        .id("mixer-master")
        .w(px(STRIP_W + 12.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(12.0))
        .bg(th.card)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_sm().font_weight(FontWeight::BOLD).child("主总线"))
                .child(
                    div()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(6.0))
                        .bg(if clipped { th.error } else { th.view })
                        .text_size(px(10.0))
                        .font_family("monospace")
                        .text_color(if clipped { th.accent_text } else { th.text_secondary })
                        .child(peak_str),
                ),
        )
        .child(div().text_xs().text_color(th.text_secondary).child("立体声输出 · 软削波保护"))
        .child(slots)
        .child(
            route_button("add-master-insert", "+ 主总线插入…", false, th).authority(mixer_revision(ed))
                .flex_none()
                .on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, window, cx| {
                    master_plugin_picker(ed, window.mouse_position(), window, cx);
                    cx.stop_propagation();
                })),
        )
        .child(numeric_pill(ed, NumericId::MasterPan, CommandArg::None, "mix-master-pan", true, cx))
        .child(div().h(px(22.0)).flex_none().child(master_pan_control(entity.clone(), master.pan_permille)))
        .child(
            div()
                .flex_1()
                .min_h(px(96.0))
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(master_fader(entity.clone(), master.gain_millidecibels)))
                .child(div().w(px(24.0)).h_full().child(ed.playback.anchor(crate::clock::Surface::Master))),
        )
        .child(numeric_pill(ed, NumericId::MasterGainDb, CommandArg::None, "mix-master-gain", true, cx))
        .child(
            div()
                .flex()
                .p(px(2.0))
                .gap(px(2.0))
                .rounded(px(8.0))
                .bg(th.control)
                .child(
                    toggle_button(ed, ToggleTarget::MasterMute,
                        if master.muted { "M 已静音" } else { "M 静音" }, "master-mute", cx)
                    .flex_1(),
                )
                .child(
                    toggle_button(ed, ToggleTarget::MasterAutomation, "A 自动化", "master-auto", cx)
                    .flex_1(),
                ),
        )
        .child(command_button(
            ed,
            CommandId::ExportAudio,
            CommandArg::None,
            if ed.export_busy { "正在导出…" } else { "导出 WAV…" },
            "export-wav",
            cx,
        ))
}

fn master_plugin_picker(ed: &mut Editor, anchor: gpui::Point<gpui::Pixels>, window: &mut gpui::Window, cx: &mut Context<Editor>) {
    let mut items = ed
        .plugin_catalog()
        .into_iter()
        .map(|plugin| {
            MenuItem::action(
                format!("{} · {}", plugin.name, plugin.format.tag().to_uppercase()),
                MenuAction::AddMasterInsert { format: plugin.format.tag().into(), id: plugin.id },
            )
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(MenuItem::disabled("没有可用插件；请检查插件宿主"));
    }
    items.push(MenuItem::separator());
    items.push(MenuItem::action("重新扫描插件", MenuAction::RescanPlugins));
    ed.show_menu(anchor, items, window, cx);
}

fn strip(
    ed: &Editor,
    entity: &Entity<Editor>,
    index: usize,
    id: TrackId,
    header: verdant_domain::TrackHeader,
    source: &verdant_collection_control::CollectionControlHandle,
    revision: &str,
    cx: &mut Context<Editor>,
) -> impl IntoElement {
    let th = ed.theme;
    let color = th.track(header.color.as_deref());
    let selected = ed.session.track == id;
    let track_peaks = ed.meters.get(index).copied().unwrap_or((0.0, 0.0));
    let (peak_str, clipped) = peak_db_label(track_peaks);
    let header_menu = id.clone();
    let select=source.clone();let select_revision=revision.to_string();
    let menu=source.clone();let menu_revision=revision.to_string();
    let accessible=source.clone();let accessible_revision=revision.to_string();
    let specs = ed.insert_specs(&id);
    let mut slots = crate::proto_scroll::viewport("insert-slots", "轨道插入效果器", "vertical").h(px(108.0)).flex_none().flex().flex_col().gap(px(3.0));
    for (slot, spec) in specs.iter().enumerate() {
        let runtime = ed.inserts.get(&id).and_then(|s| s.get(slot));
        let (state, warning) = insert_state(ed.inserts_loading, runtime, spec);
        let (open_track, menu_track) = (id.clone(), id.clone());
        let native = matches!(runtime, Some(InsertSlot::Live(h)) if h.has_editor()
            || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)));
        let spec_bypass = spec.bypass;
        let expected_key = ed.live_insert(&id, slot).map(|handle| handle.key());
        let menu_editor=entity.downgrade();
        slots = slots.child(
            crate::proto_controls::action(SharedString::from(format!("insert-{slot}")),format!("插入 {}，{}，{}", slot + 1, spec.name, state)).authority(mixer_revision(ed))
                .aria_description("点击打开原生界面；右键打开旁通和移除菜单")
                .flex_none()
                .min_h(px(38.0))
                .px(px(6.0))
                .py(px(3.0))
                .rounded(px(8.0))
                .bg(th.control)
                .flex()
                .flex_col()
                .child(div().text_xs().truncate().child(format!("{} · {}", slot + 1, spec.name)))
                .child(div().text_size(px(9.0)).truncate().text_color(if warning { th.warning } else { th.text_secondary }).child(state))
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if expected_key.is_none_or(|key| ed.live_insert(&open_track, slot).is_some_and(|handle| handle.key() == key)) && ed.select_track(open_track.clone(), cx) {
                        ed.open_insert_editor(&open_track, slot, cx);
                    }
                    cx.stop_propagation();
                }))
                .on_context_menu(move |position,window,cx| {
                    let _=menu_editor.update(cx,|ed,cx| {
                        let items = vec![
                            MenuItem::action(
                                if spec_bypass { "启用插件" } else { "旁通插件" },
                                MenuAction::BypassInsert { track: menu_track.clone(), slot, expected_key },
                            ),
                            MenuItem::action("打开原生插件窗口", MenuAction::OpenInsertEditor { track: menu_track.clone(), slot, expected_key })
                                .enabled(native),
                            MenuItem::separator(),
                            MenuItem::action("移除插件", MenuAction::RemoveInsert { track: menu_track.clone(), slot, expected_key }),
                        ];
                        ed.show_menu(position, items, window, cx);
                    });
                }),
        );
    }
    if specs.is_empty() {
        slots = slots.child(div().px(px(6.0)).py(px(4.0)).text_xs().text_color(th.text_tertiary).child("无插入效果"));
    }

    div()
        .id(SharedString::from(format!("mixer-track-{id}")))
        .w(px(STRIP_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(12.0))
        .bg(if selected { mix(th.card, th.accent, 0.14) } else { th.card })
        .role(Role::ListBoxOption).aria_label(header.name.clone()).aria_selected(selected)
        .aria_position_in_set(index+1).aria_size_of_set(ed.project().tracks.len())
        .on_click(move |_,window,cx| {
            if window.default_prevented(){return;}
            crate::proto_collection::call(&select,"selectAndFocus",vec![index.into(),false.into(),select_revision.clone().into()],window,cx);
            window.prevent_default();cx.stop_propagation();
        })
        .on_a11y_action(AccessibleAction::Click,move|_,window,cx|{
            crate::proto_collection::call(&accessible,"selectAndFocus",vec![index.into(),false.into(),accessible_revision.clone().into()],window,cx);
        })
        .on_mouse_down(MouseButton::Right,move|event,window,cx|{
            if window.default_prevented(){return;}
            crate::proto_collection::call(&menu,"contextAt",vec![index.into(),menu_revision.clone().into(),
                f64::from(event.position.x).into(),f64::from(event.position.y).into(),false.into()],window,cx);
            window.prevent_default();cx.stop_propagation();
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(div().size(px(10.0)).rounded(px(5.0)).bg(color))
                .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).child(header.name.clone()))
                .child(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(6.0))
                        .bg(if clipped { th.error } else { th.view })
                        .text_size(px(9.5))
                        .font_family("monospace")
                        .text_color(if clipped { th.accent_text } else { th.text_secondary })
                        .child(peak_str),
                )
                .child(
                    crate::proto_controls::action("track-menu",format!("{}：轨道菜单", header.name)).authority(mixer_revision(ed))
                        .aria_description("重命名、上移、下移或复制轨道")
                        .size(px(26.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .hover(move |s| s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover)))
                        .child(gpui::svg().path(crate::assets::icon("menu")).size(px(14.0)).text_color(th.text))
                        .on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, window, cx| {
                            ed.track_menu(header_menu.clone(), event.position(), window, cx);
                            cx.stop_propagation();
                        })),
                ),
        )
        .child(slots)
        .child(command_button(ed, CommandId::PluginAdd, CommandArg::TrackSlot(id.clone(), specs.len() as u32), "+ 插入效果…", SharedString::from(format!("add-insert-{id}")), cx))
        .child(numeric_pill(ed, NumericId::TrackPan, CommandArg::Track(id.clone()), format!("mix-pan-{id}"), true, cx))
        .child(div().h(px(22.0)).flex_none().child(pan_control(entity.clone(), id.clone(), header.pan_permille)))
        .child(
            div()
                .flex_1()
                .min_h(px(96.0))
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(fader(entity.clone(), id.clone(), header.gain_millidecibels)))
                .child(div().w(px(24.0)).h_full().child(ed.playback.anchor(crate::clock::Surface::Meter(index)))),
        )
        .child(numeric_pill(ed, NumericId::TrackGainDb, CommandArg::Track(id.clone()), format!("mix-gain-{id}"), true, cx))
        .child(
            div()
                .flex()
                .p(px(2.0))
                .gap(px(2.0))
                .rounded(px(8.0))
                .bg(th.control)
                .child(toggle_button(ed, ToggleTarget::TrackMute(id.clone()), "M", format!("mix-mute-{id}"), cx).flex_1())
                .child(
                    toggle_button(ed, ToggleTarget::TrackSolo(id.clone()), "S", format!("mix-solo-{id}"), cx)
                        .flex_1()
                        .border_1()
                        .rounded(px(6.0))
                        .border_color(if header.solo { th.warning } else { th.control })
                        .when(header.solo, |d| d.bg(th.warning).text_color(th.on_track()).font_weight(FontWeight::SEMIBOLD)),
                )
                .child(
                    toggle_button(ed, ToggleTarget::TrackAutomation(id.clone()), "A", format!("mix-auto-{id}"), cx)
                    .flex_1(),
                ),
        )
}

fn route_button(key: &'static str, label: &'static str, selected: bool, th: Theme) -> crate::proto_controls::Action {
    crate::proto_controls::action(key, label)
        .aria_selected(selected)
        .h(px(crate::ui_controls::BAR_BTN_H))
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .raised_states(th, selected)
        .text_xs()
        .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(th.text)
        .child(label)
}

fn parameter_panel(ed: &Editor, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    if let Some(slot) = ed.master_insert_panel {
        return master_parameter_panel(ed, slot, cx);
    }
    let (track, slot) = ed.insert_panel.as_ref()?;
    let specs = ed.insert_specs(track);
    let spec = specs.get(*slot)?;
    let th = ed.theme;
    let mut panel = div()
        .id("plugin-parameters")
        .w(px(320.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(12.0))
        .bg(th.surface)
        .border_l_1()
        .border_color(th.shade)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(spec.name.clone()))
                .child(route_button("close-params", "关闭", false, th).authority(mixer_revision(ed)).flex_none().px(px(7.0)).on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.insert_panel = None;
                    cx.notify();
                }))),
        )
        .child(
            div()
                .flex()
                .gap(px(5.0))
                .child(command_button(
                    ed,
                    CommandId::PluginBypass,
                    CommandArg::TrackSlot(track.clone(), *slot as u32),
                    if spec.bypass { "启用" } else { "旁通" },
                    "bypass",
                    cx,
                ))
                .child(command_button(ed, CommandId::PluginRemove, CommandArg::TrackSlot(track.clone(), *slot as u32), "移除", "remove", cx)),
        );
    if let Some(handle) = ed.live_insert(track, *slot) {
        if handle.has_editor() || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)) {
            panel = panel.child(command_button(
                ed,
                CommandId::PluginOpenEditor,
                CommandArg::TrackSlot(track.clone(), *slot as u32),
                "打开原生插件窗口",
                "native-editor",
                cx,
            ));
        }
        let running = matches!(handle.status(), InsertStatus::Running);
        let busy = ed.inserts_loading || ed.plugin_busy.contains(&handle.key()) || !running;
        let (state, warning) = insert_state(ed.inserts_loading, ed.inserts.get(track).and_then(|slots| slots.get(*slot)), spec);
        let (refresh_track, refresh_slot) = (track.clone(), *slot);
        panel = panel
            .child(
                route_button(
                    "refresh-params",
                    if !running { "插件已停止" } else if busy { "正在读取 / 提交…" } else { "刷新插件参数" },
                    false,
                    th,
                ).authority(mixer_revision(ed))
                .flex_none()
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if !busy {
                        ed.show_insert_params(refresh_track.clone(), refresh_slot, cx);
                    }
                })),
            )
            .child(div().text_xs().text_color(if warning { th.warning } else { th.text_secondary }).child(state));
        let mut list = crate::proto_scroll::viewport("parameter-scroll", "插件参数", "vertical").flex_1().min_h_0().flex().flex_col().gap(px(8.0));
        if let Some(params) = ed.plugin_params.get(&handle.key()) {
            if params.is_empty() {
                list = list.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数"));
            }
            for param in params {
                list = list.child(ed.plugin_parameter_row(track.clone(), *slot, param.clone(), busy, cx));
            }
        }
        panel = panel.child(list);
    } else {
        let reason = match ed.inserts.get(track).and_then(|s| s.get(*slot)) {
            Some(InsertSlot::Missing { reason, .. }) => reason.as_str(),
            _ => "正在载入插件",
        };
        panel = panel
            .child(div().text_sm().text_color(th.warning).child(reason.to_string()))
            .child(div().text_xs().text_color(th.text_secondary).child("已保留此插入的标识、参数状态和未知字段。安装插件后可重新扫描恢复。"));
    }
    Some(panel.into_any_element())
}

fn master_parameter_panel(ed: &Editor, slot: usize, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let specs = ed.master_insert_specs();
    let spec = specs.get(slot)?;
    let th = ed.theme;
    let busy = ed.inserts_loading || ed.live_master_insert(slot).is_some_and(|handle| ed.plugin_busy.contains(&handle.key()));
    let mut panel = div()
        .id("master-plugin-parameters")
        .w(px(320.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(12.0))
        .bg(th.surface)
        .border_l_1()
        .border_color(th.shade)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("主总线 · {}", spec.name)))
                .child(route_button("close-master-params", "关闭", false, th).authority(mixer_revision(ed)).flex_none().px(px(7.0)).on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.master_insert_panel = None;
                    cx.notify();
                }))),
        )
        .child(
            div()
                .flex()
                .gap(px(5.0))
                .child(
                    master_insert_bypass_button(ed, slot, spec, "master-param-bypass", cx),
                )
                .child(
                    route_button("master-param-remove", "移除", false, th).authority(mixer_revision(ed))
                        .disabled(busy)
                        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                            if !busy { ed.remove_master_insert(slot, cx); }
                            cx.stop_propagation();
                        })),
                ),
        );
    if let Some(handle) = ed.live_master_insert(slot) {
        if handle.has_editor() || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)) {
            panel = panel.child(route_button("master-native-editor", "打开原生插件窗口", false, th).authority(mixer_revision(ed)).flex_none().on_click(cx.listener(
                move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.open_master_insert_editor(slot, cx);
                    cx.stop_propagation();
                },
            )));
        }
        let running = matches!(handle.status(), InsertStatus::Running);
        let busy = busy || !running;
        let (state, warning) = insert_state(ed.inserts_loading, ed.master_insert_slots().get(slot), spec);
        panel = panel
            .child(
                route_button(
                    "refresh-master-params",
                    if !running { "插件已停止" } else if busy { "正在读取 / 提交…" } else { "刷新插件参数" },
                    false,
                    th,
                ).authority(mixer_revision(ed))
                .flex_none()
                .disabled(busy)
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if !busy { ed.show_master_insert_params(slot, cx); }
                    cx.stop_propagation();
                })),
            )
            .child(div().text_xs().text_color(if warning { th.warning } else { th.text_secondary }).child(state));
        let mut list = crate::proto_scroll::viewport("master-parameter-scroll", "主总线插件参数", "vertical").flex_1().min_h_0().flex().flex_col().gap(px(8.0));
        if let Some(params) = ed.plugin_params.get(&handle.key()) {
            if params.is_empty() {
                list = list.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数"));
            }
            for param in params {
                list = list.child(ed.master_plugin_parameter_row(slot, param.clone(), busy, cx));
            }
        }
        panel = panel.child(list);
    } else {
        let reason = match ed.master_insert_slots().get(slot) {
            Some(InsertSlot::Missing { reason, .. }) => reason.as_str(),
            _ => "正在载入插件",
        };
        panel = panel
            .child(div().text_sm().text_color(th.warning).child(reason.to_string()))
            .child(div().text_xs().text_color(th.text_secondary).child("已保留此主总线插入的标识、参数状态和未知字段。安装插件后可重新扫描恢复。"));
    }
    Some(panel.into_any_element())
}

fn paint_fader_surface(window: &mut gpui::Window, cx: &mut gpui::App, bounds: gpui::Bounds<gpui::Pixels>, th: Theme, mdb: i32) {
    let o = bounds.origin;
    let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    let x = w * 0.58;
    let usable = (h - 14.0).max(1.0);
    let mut texts = Texts::new();
    for (db, lbl) in [(6.0_f64, "+6"), (0.0, " 0"), (-6.0, "-6"), (-12.0, "-12"), (-24.0, "-24"), (-48.0, "-48")] {
        let y = 7.0 + (1.0 - fader_pos((db * 1000.0) as i32)) * usable;
        let is_zero = db.abs() < 1e-6;
        window.paint_quad(fill(b(x - 15.0, y, if is_zero { 9.0 } else { 6.0 }, if is_zero { 1.5 } else { 1.0 }, o), if is_zero { th.text_secondary } else { th.text_tertiary }));
        if h >= 110.0 {
            texts.push((lbl.into(), point(o.x + px((x - 38.0).max(1.0) as f32), o.y + px((y - 5.5) as f32)), 8.5, if is_zero { th.text_secondary } else { th.text_tertiary }));
        }
    }
    // GtkScale: a rounded trough, the accent highlight up to the value, a
    // round white slider with a hairline outline.
    window.paint_quad(quad(b(x - 3.0, 7.0, 6.0, usable, o), px(3.0), th.control_pressed, px(0.0), th.control_pressed, BorderStyle::Solid));
    let y = 7.0 + (1.0 - fader_pos(mdb)) * usable;
    window.paint_quad(quad(b(x - 3.0, y, 6.0, (7.0 + usable - y).max(0.0), o), px(3.0), th.accent, px(0.0), th.accent, BorderStyle::Solid));
    window.paint_quad(quad(b(x - 10.0, y - 10.0, 20.0, 20.0, o), px(10.0), gpui::rgb(0xffffff), px(1.0), th.shade, BorderStyle::Solid));
    flush_texts(window, cx, bounds, texts);
}

/// Fader/pan geometry is native; the compiled control owns every edit phase,
/// keyboard and assistive route. MixerDrag remains only an audio preview buffer.
fn mixer_scalar(editor: Entity<Editor>, track: Option<TrackId>, control: MixerControl, raw: i32) -> impl IntoElement {
    use crate::proto_scalar::{EventKind, Sample, Spec, scalar};
    let pan = control == MixerControl::Pan;
    let value = raw as f64 / if pan { 10.0 } else { 1000.0 };
    let label = match (track.is_some(), pan) {
        (true, true) => "轨道声像", (true, false) => "轨道推子", (false, true) => "主声像", (false, false) => "主推子",
    };
    let mut spec = Spec::slider(value, if pan { -100.0 } else { -96.0 }, if pan { 100.0 } else { 6.0 }, label);
    spec.step = if pan { 1.0 } else { 0.5 };
    spec.fine_step = if pan { 0.1 } else { 0.1 };
    spec.default_value = 0.0;
    spec.preview_echo = true;
    spec.mixer_preview = true;
    let key = SharedString::from(format!("mixer-scalar-{:?}-{}", track, if pan { "pan" } else { "gain" }));
    let visual_editor = editor.clone();
    let valid_track = track.clone();
    scalar(editor, key, spec,
        move |sample, bounds, initial| {
            if pan {
                let delta = sample.total_delta.x / (f64::from(bounds.size.width) - 16.0).max(1.0) * 200.0;
                // The audio/project boundary stores integer permille. Give
                // the source that same value before it publishes a preview,
                // so a normal repaint echoes this gesture instead of looking
                // like a competing authoritative edit.
                Sample { value: ((initial + delta) * 10.0).round() / 10.0,
                    fine: ((initial + delta * 0.1) * 10.0).round() / 10.0, jump: false }
            } else {
                let delta = -sample.total_delta.y / (f64::from(bounds.size.height) - 14.0).max(1.0);
                let origin = fader_pos((initial * 1000.0).round() as i32);
                Sample { value: fader_mdb(origin + delta) as f64 / 1000.0,
                    fine: fader_mdb(origin + delta * 0.25) as f64 / 1000.0, jump: false }
            }
        },
        move |display, _, _, cx| {
            let th = visual_editor.read(cx).theme;
            canvas(|_, _, _| {}, move |bounds, _, window, cx| {
                if !pan { paint_fader_surface(window, cx, bounds, th, (display.value * 1000.0).round() as i32); }
                else {
                    let o = bounds.origin;
                    let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
                    window.paint_quad(quad(b(0.0, h / 2.0 - 3.0, w, 6.0, o), px(3.0), th.control_pressed, px(0.0), th.control_pressed, BorderStyle::Solid));
                    window.paint_quad(fill(b(w / 2.0 - 0.5, h / 2.0 - 6.0, 1.0, 12.0, o), th.text_tertiary));
                    let x = (display.value / 100.0 + 1.0) / 2.0 * (w - 16.0) + 8.0;
                    let (a, z) = if x < w / 2.0 { (x, w / 2.0) } else { (w / 2.0, x) };
                    window.paint_quad(quad(b(a, h / 2.0 - 3.0, z - a, 6.0, o), px(3.0), th.accent, px(0.0), th.accent, BorderStyle::Solid));
                    window.paint_quad(quad(b(x - 8.0, h / 2.0 - 8.0, 16.0, 16.0, o), px(8.0), gpui::rgb(0xffffff), px(1.0), th.shade, BorderStyle::Solid));
                }
                if display.focus_visible {
                    window.paint_quad(quad(bounds, px(4.0), gpui::rgba(0), px(1.5), th.accent, BorderStyle::Solid));
                }
            }).size_full().into_any_element()
        },
        move |ed, event, _, cx| {
            let raw = (event.value * if pan { 10.0 } else { 1000.0 }).round() as i32;
            let owned = if let Some(track) = &track { ed.mixer_drag.as_ref().is_some_and(|d| &d.track == track && d.control == control) }
                else { ed.master_drag.as_ref().is_some_and(|d| d.control == control) };
            match event.kind {
                EventKind::Start => {
                    if let Some(track) = &track { ed.begin_mixer_drag(control, track, 0.0, event.value); }
                    else { ed.begin_master_drag(control, 0.0, event.value); }
                }
                EventKind::Preview | EventKind::Commit if owned => {
                    if track.is_some() {
                        ed.update_mixer_drag(|header| if pan { header.pan_permille = raw; } else { header.gain_millidecibels = raw; });
                        if event.kind == EventKind::Commit { ed.end_mixer_drag(cx); }
                    } else {
                        ed.update_master_drag(|settings| if pan { settings.pan_permille = raw; } else { settings.gain_millidecibels = raw; });
                        if event.kind == EventKind::Commit { ed.end_master_drag(cx); }
                    }
                }
                EventKind::Commit => {
                    if let Some(track) = &track { ed.edit_header(track, label, cx,
                        |header| if pan { header.pan_permille = raw; } else { header.gain_millidecibels = raw; }); }
                    else { ed.edit_master_bus(label, cx,
                        |settings| if pan { settings.pan_permille = raw; } else { settings.gain_millidecibels = raw; }); }
                }
                EventKind::Cancel if owned => {
                    if track.is_some() { ed.mixer_drag = None; } else { ed.master_drag = None; }
                    ed.sync_mixer();
                }
                _ => {},
            }
            cx.notify();
        }).valid_when(move |ed| {
            valid_track.as_ref().is_none_or(|track| ed.project().track(track).is_some())
                && ed.mixer_drag.as_ref().is_none_or(|drag| valid_track.as_ref() == Some(&drag.track) && drag.control == control)
                && ed.master_drag.as_ref().is_none_or(|drag| valid_track.is_none() && drag.control == control)
        })
}
fn pan_control(editor: Entity<Editor>, id: TrackId, pan: i32) -> impl IntoElement { mixer_scalar(editor, Some(id), MixerControl::Pan, pan) }
fn fader(editor: Entity<Editor>, id: TrackId, mdb: i32) -> impl IntoElement { mixer_scalar(editor, Some(id), MixerControl::Fader, mdb) }
fn master_pan_control(editor: Entity<Editor>, pan: i32) -> impl IntoElement { mixer_scalar(editor, None, MixerControl::Pan, pan) }
fn master_fader(editor: Entity<Editor>, mdb: i32) -> impl IntoElement { mixer_scalar(editor, None, MixerControl::Fader, mdb) }

#[cfg(test)]
mod pointer_boundary_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, Window, WindowHandle, size};
    use verdant_continuous_control::{ContinuousControlHandle, native::gpui_native_owner::all_owners};

    struct Harness { editor: Entity<Editor>, master: bool, control: MixerControl }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let ed = self.editor.read(cx);
            let track = ed.session.track.clone();
            // Use the production preview/project readback. A constant value
            // would conceal precision loss when a gesture survives a frame.
            let raw = if self.master {
                let settings = ed.master_bus();
                if self.control == MixerControl::Pan { settings.pan_permille } else { settings.gain_millidecibels }
            } else {
                let header = ed.mixer_drag.as_ref().filter(|drag| drag.track == track)
                    .map(|drag| drag.header.clone()).unwrap_or_else(|| ed.project().track(&track).unwrap().header());
                if self.control == MixerControl::Pan { header.pan_permille } else { header.gain_millidecibels }
            };
            let control = match (self.master, self.control) {
                (false, MixerControl::Pan) => pan_control(self.editor.clone(), track, raw).into_any_element(),
                (false, MixerControl::Fader) => fader(self.editor.clone(), track, raw).into_any_element(),
                (true, MixerControl::Pan) => master_pan_control(self.editor.clone(), raw).into_any_element(),
                (true, MixerControl::Fader) => master_fader(self.editor.clone(), raw).into_any_element(),
            };
            div().size_full().child(div().w(px(140.0)).h(px(120.0)).child(control))
                .child(canvas(|_, _, _| {}, |_, _, window, _| {
                    window.on_mouse_event(|_: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Bubble { cx.stop_propagation(); }
                    });
                }).size(px(0.0)))
        }
    }
    fn open(cx: &mut TestAppContext, master: bool, control: MixerControl) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(320.0), px(200.0)), |window, cx| Harness {
            editor: cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx)), master, control,
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }
    fn source(host: &Harness, window: &Window) -> ContinuousControlHandle {
        let label = match (host.master, host.control) {
            (false, MixerControl::Pan) => "轨道声像", (false, MixerControl::Fader) => "轨道推子",
            (true, MixerControl::Pan) => "主声像", (true, MixerControl::Fader) => "主推子",
        };
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.declared_events.contains("interactionStart")
                && owner.props.get("label").and_then(serde_json::Value::as_str) == Some(label)
        }).expect("mounted production mixer scalar");
        ContinuousControlHandle { owner }
    }
    fn down(host: &Harness, window: &mut Window, cx: &mut gpui::App) -> ContinuousControlHandle {
        let source = source(host, window);
        let bounds = source.owner.borrow().native_hitbox.as_ref().expect("real native scalar target").bounds;
        assert!(bounds.size.width >= px(139.) && bounds.size.height >= px(119.), "canvas-only scalar root must fill the actual mixer control: {bounds:?}");
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: bounds.center(), button: MouseButton::Left, ..Default::default()
        }), cx);
        assert_eq!(source.state("phase").unwrap(), serde_json::json!("armed"), "the compiled source owns press admission");
        assert!(if host.master { host.editor.read(cx).master_drag.is_some() } else { host.editor.read(cx).mixer_drag.is_some() });
        assert!(window.captured_hitbox().is_some());
        source
    }
    #[gpui::test]
    fn all_mixer_controls_discard_outside_motion_or_terminal_sample(cx: &mut TestAppContext) {
        for master in [false, true] { for control in [MixerControl::Pan, MixerControl::Fader] { for terminal in [false, true] {
            let window = open(cx, master, control);
            window.update(cx, |host, window, cx| {
                let source = down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(100.0), px(30.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                let outside = point(px(-20.0), px(60.0));
                if terminal {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: outside, button: MouseButton::Left, ..Default::default() }), cx);
                } else {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent { position: outside, pressed_button: Some(MouseButton::Left), ..Default::default() }), cx);
                }
                assert!(host.editor.read(cx).mixer_drag.is_none() && host.editor.read(cx).master_drag.is_none());
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!(source.state("value").unwrap().as_f64(), Some(0.));
                assert!(window.captured_hitbox().is_none(), "outside motion cancels before a leave/up event");
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: point(px(100.0), px(30.0)), button: MouseButton::Left, ..Default::default() }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
            }).unwrap();
        } } }
    }
    #[gpui::test]
    fn all_mixer_controls_commit_the_in_window_up_sample_outside_the_control(cx: &mut TestAppContext) {
        for master in [false, true] { for control in [MixerControl::Pan, MixerControl::Fader] {
            let window = open(cx, master, control);
            window.update(cx, |host, window, cx| {
                let source = down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: point(px(240.0), px(170.0)), button: MouseButton::Left, ..Default::default()
                }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 1, "a stopped bubble cannot discard the valid terminal sample");
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                let ed = host.editor.read(cx);
                let (gain, pan) = if host.master {
                    let master = ed.master_bus(); (master.gain_millidecibels, master.pan_permille)
                } else {
                    let track = ed.project().track(&ed.session.track).unwrap(); (track.gain_millidecibels, track.pan_permille)
                };
                assert_eq!(if control == MixerControl::Pan { pan } else { gain },
                    if control == MixerControl::Pan { 1000 } else { -96_000 }, "the in-window final sample reaches the domain boundary exactly once");
                assert!(host.editor.read(cx).mixer_drag.is_none() && host.editor.read(cx).master_drag.is_none());
                assert!(window.captured_hitbox().is_none());
            }).unwrap();
        } }
    }

    #[gpui::test]
    fn pan_previews_survive_quantized_repaint_and_commit_once(cx: &mut TestAppContext) {
        for master in [false, true] { for fine in [false, true] {
            let window = open(cx, master, MixerControl::Pan);
            window.update(cx, |host, window, cx| down(host, window, cx)).unwrap();
            crate::native_test_support::frames(window, cx);
            for x in [80.0, 95.0] {
                window.update(cx, |_, window, cx| {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                        position: point(px(x), px(60.0)), pressed_button: Some(MouseButton::Left),
                        modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                    }), cx);
                    cx.notify();
                }).unwrap();
                crate::native_test_support::frames(window, cx);
                window.update(cx, |host, window, cx| {
                    let source = source(host, window);
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"),
                        "integer audio preview must remain the current gesture after repaint");
                    let ed = host.editor.read(cx);
                    let raw = if master { ed.master_drag.as_ref().expect("master preview survives").settings.pan_permille }
                        else { ed.mixer_drag.as_ref().expect("track preview survives").header.pan_permille };
                    assert_eq!(source.state("value").unwrap().as_f64(), Some(raw as f64 / 10.0));
                    assert_eq!(ed.revision().get(), 0, "preview does not write an undo step");
                }).unwrap();
            }
            window.update(cx, |host, window, cx| {
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: point(px(95.0), px(60.0)), button: MouseButton::Left,
                    modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                }), cx);
                let ed = host.editor.read(cx);
                assert_eq!(ed.revision().get(), 1, "one completed drag creates one transaction");
                let raw = if master { crate::mix::MasterBusSettings::from_project(ed.project()).pan_permille }
                    else { ed.project().track(&ed.session.track).unwrap().header().pan_permille };
                assert_eq!(raw, if fine { 40 } else { 403 });
                assert!(ed.mixer_drag.is_none() && ed.master_drag.is_none());
                assert!(window.captured_hitbox().is_none());
                cx.notify();
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert_eq!(source(host, window).state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!(host.editor.read(cx).revision().get(), 1);
            }).unwrap();
        } }
    }
}

#[cfg(test)]
mod collection_native_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, Window, size};
    use verdant_collection_control::{CollectionControlHandle,native::gpui_native_owner::{all_owners,resolved}};

    struct Harness { editor: Entity<Editor> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity=self.editor.clone();
            entity.update(cx,|ed,cx|mixer(ed,&entity,cx).into_any_element())
        }
    }
    fn source(window:&Window)->CollectionControlHandle{
        let owner=all_owners().into_iter().find(|owner|{
            let owner=owner.borrow();owner.mounted&&owner.window_id==Some(window.window_handle().window_id())
                &&owner.modules.accessible_text.get("name").is_some_and(|name|resolved(name).text()=="混音轨道")
        }).expect("rendered mixer uses the compiled track collection");
        CollectionControlHandle{owner}
    }
    #[gpui::test]
    fn mixer_collection_selects_reveals_and_opens_context_on_its_actual_horizontal_engine(cx:&mut TestAppContext){
        let window=cx.open_window(size(px(620.),px(560.)),|window,cx|{
            let mut project=verdant_domain::Project::new_empty();
            project.tracks=(0..8).map(|index|verdant_domain::Track::new(&format!("Mixer {index}"),"teal")).collect();
            let editor=cx.new(|cx|Editor::for_native_test(project,window,cx));
            Harness{editor}
        });
        window.update(cx,|_,window,_|window.activate_window()).unwrap();
        crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx|{source(window).call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        cx.simulate_keystrokes(window.into(),"end");
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx|{
            let ed=host.editor.read(cx);assert_eq!(ed.session.track,ed.project().tracks[7].id);
            let source=source(window);let owner=source.owner.borrow();
            assert_ne!(owner.modules.scroll.external_engine,0,"the collection leases the actual native strip viewport");
            assert!(owner.modules.scroll.handle.as_ref().unwrap().offset().x<px(-100.));
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"home");
        crate::native_test_support::frames(window,cx);
        cx.simulate_keystrokes(window.into(),"right");
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,cx|{
            let ed=host.editor.read(cx);assert_eq!(ed.session.track,ed.project().tracks[1].id,"one native arrow selects exactly one source row");
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"shift-f10");
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,cx|assert!(host.editor.read(cx).menu.is_some(),"source keyboard context uses the selected track")).unwrap();
    }
}


#[cfg(test)]
mod escape_route_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, size};
    use verdant_continuous_control::{ContinuousControlHandle, native::gpui_native_owner::resolved};

    #[gpui::test]
    fn native_editor_escape_cancels_mixer_contact_before_late_release(cx: &mut TestAppContext) {
        let mut previous_window: Option<gpui::WindowHandle<Editor>> = None;
        for (master, replace_capture) in [(false, false), (true, false), (false, true)] {
            // The actual Editor keymap resolves Escape before a raw key.down
            // handler. A bare scalar cannot exercise that application route.
            let mut project = crate::native_test_support::instrument_project();
            project.tracks.truncate(1);
            let note = verdant_domain::Note::new(0, 480, 60);
            let selected = note.id.clone();
            project.tracks[0].notes.push(note);
            let window = cx.open_window(size(px(1300.0), px(900.0)), |window, cx| {
                let bindings = crate::shortcuts::bindings(cx);
                cx.bind_keys(bindings);
                let mut editor = Editor::for_native_test(project, window, cx);
                editor.session.selection.insert(selected.clone());
                editor.bottom = crate::editor::Bottom::Mixer;
                editor.needs_fit = false;
                editor
            });
            cx.update_window(window.into(), |_, window, _| window.activate_window()).unwrap();
            crate::native_test_support::frames(window, cx);
            let initial_revision = window.update(cx, |editor, _, _| editor.revision().get()).unwrap();
            let (handle, accepted, start) = cx.update_window(window.into(), |_, window, cx| {
                let label = if master { "主声像" } else { "轨道声像" };
                // The numeric pill has the same name and a spinbutton role.
                // Locate the actual pan slider in this single-track fixture.
                let candidates = verdant_continuous_control::native::gpui_native_owner::all_owners().into_iter().filter(|owner| {
                    let owner = owner.borrow();
                    owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                        && owner.props.get("label").and_then(serde_json::Value::as_str) == Some(label)
                        && owner.props.get("role").and_then(serde_json::Value::as_str) == Some("slider")
                }).collect::<Vec<_>>();
                assert_eq!(candidates.len(), 1, "one Mixer slider: master={master} replace_capture={replace_capture} label={label}");
                let owner = candidates.into_iter().next().unwrap();
                let handle = ContinuousControlHandle { owner };
                let accepted = handle.state("value").unwrap().as_f64().unwrap();
                let start = handle.owner.borrow().bounds.expect("mounted pan bounds").center();
                window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                    position: start, button: MouseButton::Left, ..Default::default()
                }), cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: start + point(px(22.0), px(0.0)),
                    pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                assert_eq!(handle.state("phase").unwrap(), serde_json::json!("dragging"));
                assert_ne!(handle.state("value").unwrap().as_f64(), Some(accepted));
                assert!(window.captured_hitbox().is_some());
                (handle, accepted, start)
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            let contact = window.update(cx, |editor, _, _| {
                assert!(editor.has_proto_pointer_contact());
                assert!(if master { editor.master_drag.is_some() } else { editor.mixer_drag.is_some() },
                    "preview buffer: master={master} replace_capture={replace_capture} props={:?}", handle.owner.borrow().props);
                assert_eq!(editor.revision().get(), initial_revision, "preview never writes the project");
                assert!(editor.session.selection.contains(&selected));
                editor.proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade).unwrap()
            }).unwrap();
            if let Some(other) = previous_window {
                cx.update_window(other.into(), |_, window, cx| {
                    assert!(!contact.cancel_in_window(window, cx), "another window cannot cancel this contact");
                }).unwrap();
                assert!(contact.is_active());
                assert_eq!(handle.state("phase").unwrap(), serde_json::json!("dragging"));
            }
            let replacement = replace_capture.then(|| cx.update_window(window.into(), |_, window, cx| {
                // Reacquire the same hitbox under a new ownership generation.
                // Escape must not release this newer native acquisition.
                crate::host_pointer::OwnedPointerCapture::acquire(window.captured_hitbox().unwrap(), window, cx)
            }).unwrap());
            cx.simulate_keystrokes(window.into(), "escape");
            crate::native_test_support::frames(window, cx);
            cx.update_window(window.into(), |_, window, cx| {
                assert_eq!(handle.state("phase").unwrap(), serde_json::json!("idle"),
                    "production Mixer Escape must cancel the authored interaction; contexts={:?}", window.context_stack());
                assert_eq!(handle.state("value").unwrap().as_f64(), Some(accepted));
                assert_eq!(resolved(&handle.owner.borrow().accessible_states["valueNow"]).num(), accepted);
                assert!(!contact.cancel_in_window(window, cx), "an idle contact does not claim another Escape");
                if let Some(replacement) = replacement {
                    assert!(replacement.is_current(window, cx), "cancel preserves a newer capture of the same hitbox");
                    replacement.release(window, cx);
                } else {
                    assert!(window.captured_hitbox().is_none(), "cancel releases its owned capture before MouseUp");
                }
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: start + point(px(22.0), px(0.0)), button: MouseButton::Left, ..Default::default()
                }), cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |editor, _, _| {
                assert!(editor.mixer_drag.is_none() && editor.master_drag.is_none());
                assert!(!editor.has_proto_pointer_contact());
                assert_eq!(editor.revision().get(), initial_revision, "Escape and late MouseUp cannot commit");
                assert!(editor.session.selection.contains(&selected), "cancelling a live contact preserves note selection");
            }).unwrap();
            cx.simulate_keystrokes(window.into(), "escape");
            window.update(cx, |editor, _, _| {
                assert!(editor.session.selection.is_empty(), "an inactive contact falls through to the existing Escape action");
            }).unwrap();
            previous_window = Some(window);
        }
    }
}

#[cfg(test)]
mod source_toggle_tests {
    use super::*;
    use gpui::{AppContext, Render, Subscription, TestAppContext, Window, size};
    use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{all_owners, resolved, replace_props, dispatch_accessibility_action, drive_owners}};

    struct Harness { editor: Entity<Editor>, target: ToggleTarget, _changed: Subscription }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let target = self.target.clone();
            self.editor.update(cx, |ed, cx| {
                toggle_button(ed, target, "Source mixer switch", "source-mixer-switch", cx)
                    .w(px(220.)).h(px(40.)).into_any_element()
            })
        }
    }
    fn source(window: &Window) -> ActionControlHandle {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text() == "Source mixer switch")
        }).expect("production mixer switch uses ActionControl");
        ActionControlHandle { owner }
    }
    #[gpui::test]
    fn mixer_switches_accept_the_source_proposal_and_reject_retired_document_callbacks(cx: &mut TestAppContext) {
        for family in 0..5 {
            let window = cx.open_window(size(px(300.), px(160.)), |window, cx| {
                let editor = cx.new(|cx| Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx));
                let track = editor.read(cx).session.track.clone();
                let target = match family { 0 => ToggleTarget::TrackMute(track), 1 => ToggleTarget::TrackSolo(track),
                    2 => ToggleTarget::TrackAutomation(track), 3 => ToggleTarget::MasterMute, _ => ToggleTarget::MasterAutomation };
                let changed = cx.observe(&editor, |_, _, cx| cx.notify());
                Harness { editor, target, _changed: changed }
            });
            window.update(cx, |_, window, _| window.activate_window()).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert_eq!(host.target.current(host.editor.read(cx)), Some(false));
                let source = source(window);
                // Change the compiled source's accepted value, keeping the
                // domain false: its next real assistive activation now proposes
                // false. Re-inverting the host value would incorrectly write true.
                let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(true);
                replace_props(&source.owner, props); drive_owners(window, cx);
                dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert_eq!(host.target.current(host.editor.read(cx)), Some(false), "family {family} must accept source false instead of inverting the host");
                assert_eq!(host.editor.read(cx).revision().get(), 0, "same-value source proposal creates no project transaction");
                let source = source(window);
                let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(false);
                replace_props(&source.owner, props); drive_owners(window, cx);
                dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            let revision = window.update(cx, |host, window, cx| {
                assert_eq!(host.target.current(host.editor.read(cx)), Some(true), "source true reaches family {family}");
                let revision = host.editor.read(cx).revision().get();
                assert_eq!(revision, if family == 2 || family == 4 { 0 } else { 1 });
                let source = source(window);
                // Keep the old mounted callback while its document authority
                // changes before the deferred source response can be delivered.
                host.editor.update(cx, |ed, _| ed.document_generation += 1);
                dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
                revision
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, _, cx| {
                assert_eq!(host.target.current(host.editor.read(cx)), Some(true), "old document response cannot write family {family}");
                assert_eq!(host.editor.read(cx).revision().get(), revision);
            }).unwrap();
            window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
        }
    }
}
