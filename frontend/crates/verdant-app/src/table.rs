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
    fn sync(&mut self, track: &TrackId, note: Option<&Note>, revision: u64, window: &mut Window, table: &FocusHandle, scroll: &gpui::UniformListScrollHandle, cx: &mut Context<Editor>) {
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
            scroll.scroll_to_item(0, gpui::ScrollStrategy::Nearest);
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
            } else {
                scroll.scroll_to_item(self.cursor_index, gpui::ScrollStrategy::Nearest);
            }
        }
    }

    fn handle(&mut self, key: &PointKey, cx: &mut Context<Editor>) -> FocusHandle {
        self.handles.entry(key.clone()).or_insert_with(|| cx.focus_handle()).clone()
    }
}

pub fn note_table(ed: &mut Editor, entity: Entity<Editor>, window: &mut Window, cx: &mut Context<Editor>) -> impl IntoElement {
    let project = ed.store().snapshot();
    let note = project.track(&ed.session.track).and_then(|t| t.notes.get(ed.table_cursor));
    let revision = ed.revision().get();
    ed.table_focus.sync(&ed.session.track, note, revision, window, &ed.focus_table, &ed.point_scroll, cx);
    let count = ed.active_track().map(|t| t.notes.len()).unwrap_or(0);
    let track = ed.session.track.to_string();
    let notes = entity.clone();
    let point_note = note.map(|n| n.id.clone());
    let point_count = ed.table_focus.order.len();
    let points = entity;
    div().id(SharedString::from(format!("note-table-{track}"))).role(Role::Table).aria_label("音符表")
        .aria_description("上下键选择，Shift 扩展选择，Home/End 跳转，Ctrl 方向键移动音符，Enter 编辑歌词。Tab 进入当前音符控制点，Shift Tab 返回轨道区。")
        .aria_row_count(count).aria_column_count(5).track_focus(&ed.focus_table).key_context("NoteTable")
        .size_full().flex().flex_col().bg(ed.theme.surface)
        .on_key_down(cx.listener(table_key))
        .child(div().h(px(32.0)).px(px(12.0)).flex().items_center().gap(px(20.0)).text_sm()
            .child(format!("音符表 · {count} 个音符"))
            .child("起点 / 长度 / 音高 / 歌词 · Enter 编辑 · Tab 控制点 · Shift Tab 轨道区"))
        .child(uniform_list(SharedString::from(format!("note-rows-{track}")), count, move |range, _, cx| {
            notes.update(cx, |ed, cx| range.filter_map(|i| note_row(ed, i, cx)).collect::<Vec<_>>())
        }).track_scroll(&ed.table_scroll).flex_1().min_h_0())
        .child(div().h(px(28.0)).px(px(12.0)).flex().items_center().text_xs().text_color(ed.theme.text_secondary)
            .child(format!("全部参数曲线 · {point_count} 点 · Tab / ←→ 选点 · ↑↓ 步进，Shift 精调 PIT/DYN · Esc 返回音符")))
        .child(div().h(px(140.0)).min_h_0().child(uniform_list(
            SharedString::from(format!("curve-points-{track}")), point_count, move |range, _, cx| {
                points.update(cx, |ed, cx| range.filter_map(|i| point_row(ed, point_note.as_ref()?, i, cx)).collect::<Vec<_>>())
            }).track_scroll(&ed.point_scroll).size_full()))
}

fn table_key(ed: &mut Editor, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Editor>) {
    let key = event.keystroke.key.as_str();
    let modifiers = event.keystroke.modifiers;
    let secondary = modifiers.control || modifiers.platform;
    if !matches!(key, "up" | "down" | "left" | "right" | "home" | "end" | "pageup" | "pagedown" | "enter" | "f2" | "delete" | "backspace" | "tab") { return; }
    window.prevent_default(); cx.stop_propagation();
    match key {
        "tab" => {
            if modifiers.shift || ed.table_focus.order.is_empty() {
                window.focus(&ed.focus_arrange, cx);
            } else {
                focus_point(ed, ed.table_focus.cursor_index, window, cx);
            }
        }
        "enter" | "f2" => {
            let id = ed.active_track().and_then(|t| t.notes.get(ed.table_cursor)).map(|n| n.id.clone());
            if let Some(id) = id { ed.bottom = Bottom::Editor; ed.begin_lyric(id, window, cx); }
        }
        "delete" | "backspace" => ed.run_menu_action(MenuAction::DeleteSelection, window, cx),
        "up" | "down" | "left" | "right" if secondary => {
            let project = ed.store().snapshot();
            let ds = match key { "up" => if modifiers.shift { 12 } else { 1 }, "down" => if modifiers.shift { -12 } else { -1 }, _ => 0 };
            let grid = if modifiers.shift { ed.project().ppq * 4 } else { ed.session.snap.grid_ticks.max(1) };
            let dt = match key { "left" => -grid, "right" => grid, _ => 0 };
            let effects = ed.session.nudge_selection(&project, ed.revision(), dt, ds);
            ed.apply_effects(effects, window, cx);
        }
        "up" | "left" => ed.table_step(false, modifiers.shift, cx),
        "down" | "right" => ed.table_step(true, modifiers.shift, cx),
        "home" => ed.table_select(0, modifiers.shift, cx),
        "end" => ed.table_select(ed.active_track().map(|t| t.notes.len().saturating_sub(1)).unwrap_or(0), modifiers.shift, cx),
        "pageup" => ed.table_select(ed.table_cursor.saturating_sub(20), modifiers.shift, cx),
        "pagedown" => ed.table_select(ed.table_cursor.saturating_add(20), modifiers.shift, cx),
        _ => {},
    }
}

fn note_row(ed: &Editor, index: usize, cx: &mut Context<Editor>) -> Option<gpui::Stateful<gpui::Div>> {
    let note = ed.active_track()?.notes.get(index)?;
    let (id, name) = (note.id.clone(), verdant_domain::timing::note_name(note.midi_note));
    let current = index == ed.table_cursor;
    let selected = ed.session.selection.contains(&id);
    let focus_id = id.clone();
    let increment = id.clone();
    let decrement = id.clone();
    let click_id = id.clone();
    let reading = verdant_domain::pronunciation_display(&note.pronunciation);
    let label = format!("音符 {}，{}，起点 {} tick，长度 {} tick，歌词 {}，读音 {}{}", id, name, note.start_tick, note.duration_tick,
        note.lyric, reading, if ed.error_notes.contains(&id) { "，渲染预检错误" } else { "" });
    Some(div().id(SharedString::from(format!("note-{id}"))).role(Role::Row).accessibility_id(SharedString::from(format!("note-{id}")))
        .aria_label(label).aria_selected(selected).aria_row_index(index).aria_position_in_set(index + 1)
        .aria_size_of_set(ed.active_track()?.notes.len()).when(current, |d| d.aria_active_descendant())
        .h(px(34.0)).px(px(12.0)).flex().items_center().gap(px(12.0)).text_sm()
        .bg(if selected { crate::theme::mix(ed.theme.surface, ed.theme.accent, 0.18) } else { ed.theme.surface })
        .border_b_1().border_color(ed.theme.shade)
        .on_a11y_action(AccessibleAction::Focus, crate::view::a11y_listener(cx, move |ed, _, window, cx| {
            if let Some(i) = ed.index.position(&focus_id) { ed.table_select(i, false, cx); window.focus(&ed.focus_table, cx); }
        }))
        .on_a11y_action(AccessibleAction::Increment, crate::view::a11y_listener(cx, move |ed, _, _, cx| ed.step_notes_pitch(vec![increment.clone()], 1, cx)))
        .on_a11y_action(AccessibleAction::Decrement, crate::view::a11y_listener(cx, move |ed, _, _, cx| ed.step_notes_pitch(vec![decrement.clone()], -1, cx)))
        .on_click(cx.listener(move |ed, ev: &gpui::ClickEvent, window, cx| {
            if let Some(i) = ed.index.position(&click_id) { ed.table_select(i, ev.modifiers().shift, cx); }
            window.focus(&ed.focus_table, cx);
        }))
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
    ed.point_scroll.scroll_to_item(index, gpui::ScrollStrategy::Nearest);
    window.focus(&handle, cx);
    cx.notify();
}

fn focus_point_key(ed: &mut Editor, note: &NoteId, key: &PointKey, window: &mut Window, cx: &mut Context<Editor>) {
    if !ed.table_focus.note.as_ref().is_some_and(|(track, id)| track == &ed.session.track && id == note) { return; }
    if let Some(&index) = ed.table_focus.positions.get(key) { focus_point(ed, index, window, cx); }
}

fn point_key(ed: &mut Editor, note: &NoteId, point: &PointKey, step: f64, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Editor>) {
    let key = event.keystroke.key.as_str();
    if !matches!(key, "up" | "down" | "+" | "=" | "-" | "left" | "right" | "home" | "end" | "pageup" | "pagedown" | "tab" | "escape" | "enter") { return; }
    window.prevent_default(); cx.stop_propagation();
    let Some(&index) = ed.table_focus.positions.get(point) else { return };
    ed.table_focus.cursor = Some(point.clone());
    ed.table_focus.cursor_index = index;
    let last = ed.table_focus.order.len().saturating_sub(1);
    match key {
        "up" | "down" | "+" | "=" | "-" => {
            let sign = if matches!(key, "up" | "+" | "=") { 1.0 } else { -1.0 };
            let fine = event.keystroke.modifiers.shift && !point.param.is_expression();
            ed.step_curve_point(note, &point.id, point.param, sign * step * if fine { 0.1 } else { 1.0 }, cx);
        }
        "left" => focus_point(ed, index.saturating_sub(1), window, cx),
        "right" => focus_point(ed, index.saturating_add(1).min(last), window, cx),
        "home" => focus_point(ed, 0, window, cx),
        "end" => focus_point(ed, last, window, cx),
        "pageup" => focus_point(ed, index.saturating_sub(4), window, cx),
        "pagedown" => focus_point(ed, index.saturating_add(4).min(last), window, cx),
        "tab" if event.keystroke.modifiers.shift && index > 0 => focus_point(ed, index - 1, window, cx),
        "tab" if !event.keystroke.modifiers.shift && index < last => focus_point(ed, index + 1, window, cx),
        "tab" | "escape" | "enter" => { window.focus(&ed.focus_table, cx); cx.notify(); }
        _ => {},
    }
}

fn point_row(ed: &mut Editor, note: &NoteId, index: usize, cx: &mut Context<Editor>) -> Option<gpui::Stateful<gpui::Div>> {
    let n = ed.active_track().and_then(|t| ed.index.note(t, note))?;
    let selected = ed.table_focus.order.get(index)?;
    let param = selected.param;
    let (point, tick, value) = param.point_samples(n).into_iter().find(|p| p.0 == selected.id)?;
    let step = if param == Param::Dyn { 0.1 } else { 1.0 };
    let unit = match param { Param::Pit => "cents", Param::Dyn => "dB", _ => "/127" };
    let key = SharedString::from(format!("point-{note}-{}-{point}", param.short()));
    let identity = PointKey { param, id: point.clone() };
    let handle = ed.table_focus.handle(&identity, cx);
    let (inc_note, dec_note, key_note) = (note.clone(), note.clone(), note.clone());
    let (inc, dec) = (point.clone(), point);
    let (focus_note, click_note) = (note.clone(), note.clone());
    let (focus_key, click_key) = (identity.clone(), identity.clone());
    let label = if param == Param::Portamento { format!("POR 音符 {}，按音符起点生效", note) }
        else { format!("{} 控制点，音符 {}，偏移 {} tick", param.short(), note, tick) };
    let display = if param == Param::Portamento { format!("POR · 按音符起点生效 · {value:.0} {unit}") }
        else { format!("{} · {tick} tick · {value:.1} {unit}", param.short()) };
    Some(div().id(key.clone()).accessibility_id(key).role(Role::SpinButton)
        .aria_label(label).aria_numeric_value(value)
        .aria_numeric_value_step(step).aria_value(format!("{value:.1} {unit}"))
        .aria_position_in_set(index + 1).aria_size_of_set(ed.table_focus.order.len())
        .track_focus(&handle).key_context("NoteTablePoint")
        .focus(|style| style.bg(crate::theme::mix(ed.theme.surface, ed.theme.accent, 0.18)))
        .h(px(32.0)).px(px(12.0)).flex().items_center().gap(px(12.0)).text_sm()
        .child(display)
        .on_a11y_action(AccessibleAction::Focus, crate::view::a11y_listener(cx, move |ed, _, window, cx| focus_point_key(ed, &focus_note, &focus_key, window, cx)))
        .on_a11y_action(AccessibleAction::Increment, crate::view::a11y_listener(cx, move |ed, _, _, cx| ed.step_curve_point(&inc_note, &inc, param, step, cx)))
        .on_a11y_action(AccessibleAction::Decrement, crate::view::a11y_listener(cx, move |ed, _, _, cx| ed.step_curve_point(&dec_note, &dec, param, -step, cx)))
        .on_click(cx.listener(move |ed, _, window, cx| { focus_point_key(ed, &click_note, &click_key, window, cx); cx.stop_propagation(); }))
        .on_key_down(cx.listener(move |ed, ev: &KeyDownEvent, window, cx| point_key(ed, &key_note, &identity, step, ev, window, cx))))
}
