//! Exact, unit-aware entry shared by native effect and instrument controls.
//! The dialog holds an immutable target/revision; a changed document cannot
//! redirect an old value entry to a different insert at the same slot.

use super::*;
use gpui::{App, Bounds, ElementInputHandler, EntityInputHandler, FocusHandle, Point, Render, TextRun, UTF16Selection, WindowBounds, WindowOptions, point, size};
use std::ops::Range;

pub(super) fn parse_value(text: &str, desc: &ParamDesc) -> Result<f64, String> {
    let text = text.trim().to_lowercase();
    let (number, scale) = if desc.unit == "Hz" && text.ends_with("khz") {
        (text.strip_suffix("khz").unwrap().trim(), 1000.0)
    } else if desc.unit == "ms" && text.ends_with("ms") {
        (text.strip_suffix("ms").unwrap().trim(), 1.0)
    } else if desc.unit == "ms" && text.ends_with('s') {
        (text.strip_suffix('s').unwrap().trim(), 1000.0)
    } else if desc.unit == "s" && text.ends_with("ms") {
        (text.strip_suffix("ms").unwrap().trim(), 0.001)
    } else if !desc.unit.is_empty() && text.ends_with(&desc.unit.to_lowercase()) {
        (text[..text.len() - desc.unit.len()].trim(), 1.0)
    } else { (text.as_str(), 1.0) };
    let value = number.parse::<f64>().map_err(|_| "请输入数值，可附 Hz / kHz、ms / s、dB 或 % 单位".to_string())? * scale;
    if !value.is_finite() || value < desc.min || value > desc.max {
        return Err(format!("{} 须在 {} 至 {} {} 之间", desc.name, desc.min, desc.max, desc.unit));
    }
    verdant_plugins::insert::parameter_editor_value(desc, value).map_err(|e| e.to_string())
}

pub(super) fn open(editor: &Entity<Editor>, target: Target, desc: ParamDesc, cx: &mut App) {
    let Some(params) = current_params(editor.read(cx), &target) else { return };
    let Some(desc) = param(&params, &desc.id).cloned() else { return };
    let base = editor.read(cx).revision();
    let generation = editor.read(cx).document_generation;
    let owner = editor.downgrade();
    let bounds = Bounds::centered(None, size(px(440.), px(224.)), cx);
    if let Err(error) = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title(&format!("{} — Verdant", desc.name));
        cx.new(|cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            let mut text = verdant_session::TextEditState::new(&format!("{}", desc.value));
            text.select_all();
            ValueEntry { editor: owner, target, desc, base, generation, text, focus, line: None, bounds: None, error: String::new() }
        })
    }) {
        editor.update(cx, |ed, cx| { ed.notice(format!("无法打开数值输入：{error}")); cx.notify(); });
    }
}

struct ValueEntry {
    editor: gpui::WeakEntity<Editor>,
    target: Target,
    desc: ParamDesc,
    base: verdant_domain::Revision,
    generation: u64,
    text: verdant_session::TextEditState,
    focus: FocusHandle,
    line: Option<gpui::ShapedLine>,
    bounds: Option<Bounds<Pixels>>,
    error: String,
}

impl ValueEntry {
    fn edit(&mut self, cx: &mut Context<Self>, apply: impl FnOnce(&mut verdant_session::TextEditState)) {
        if !self.text.is_composing() { apply(&mut self.text); self.error.clear(); cx.notify(); }
    }
    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.text.is_composing() { return; }
        let value = match parse_value(&self.text.text, &self.desc) {
            Ok(value) => value,
            Err(error) => { self.error = error; cx.notify(); return; }
        };
        let target = self.target.clone();
        let id = self.desc.id.clone();
        let base = self.base;
        let generation = self.generation;
        let result = self.editor.update(cx, |ed, cx| {
            if ed.document_generation != generation || ed.revision() != base || current_params(ed, &target).is_none() {
                return Err("工程或插件已改变，请重新打开数值输入".to_string());
            }
            if ed.plugin_busy.contains(&target.key) {
                return Err("正在提交上一项修改，请稍后再应用".to_string());
            }
            if ed.set_device_params(target.track, target.slot, vec![(id, value)], cx) { Ok(()) }
            else { Err("参数尚未提交，请等待插件载入完成后重试".to_string()) }
        });
        match result {
            Ok(Ok(())) => window.remove_window(),
            Ok(Err(error)) => { self.error = error; cx.notify(); }
            Err(_) => { self.error = "工程窗口已关闭".into(); cx.notify(); }
        }
    }
}

impl Render for ValueEntry {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.editor.upgrade().map(|ed| ed.read(cx).theme).unwrap_or_else(|| Theme::for_appearance(gpui::WindowAppearance::Dark));
        let entity = cx.entity();
        div().id("device-value-entry").role(Role::Dialog).aria_label(self.desc.name.clone()).size_full().p(px(18.))
            .bg(th.surface).text_color(th.text).flex().flex_col().gap(px(10.)).key_context("LyricEditor").track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &crate::actions::LyricCancel, window, _| { if !view.text.is_composing() { window.remove_window(); } }))
            .on_action(cx.listener(|view, _: &crate::actions::LyricConfirm, window, cx| view.apply(window, cx)))
            .on_action(cx.listener(|view, _: &crate::actions::Backspace, _, cx| view.edit(cx, |s| s.backspace())))
            .on_action(cx.listener(|view, _: &crate::actions::Delete, _, cx| view.edit(cx, |s| s.delete())))
            .on_action(cx.listener(|view, _: &crate::actions::Left, _, cx| view.edit(cx, |s| s.left())))
            .on_action(cx.listener(|view, _: &crate::actions::Right, _, cx| view.edit(cx, |s| s.right())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectLeft, _, cx| view.edit(cx, |s| s.select_left())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectRight, _, cx| view.edit(cx, |s| s.select_right())))
            .on_action(cx.listener(|view, _: &crate::actions::Home, _, cx| view.edit(cx, |s| s.home())))
            .on_action(cx.listener(|view, _: &crate::actions::End, _, cx| view.edit(cx, |s| s.end())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectAllText, _, cx| view.edit(cx, |s| s.select_all())))
            .on_action(cx.listener(|view, _: &crate::actions::PasteText, _, cx| {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) { view.edit(cx, |s| s.replace_text_in_range(None, &text)); }
            }))
            .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "enter" { window.prevent_default(); view.apply(window, cx); cx.stop_propagation(); }
            }))
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(self.desc.name.clone()))
            .child(div().id("device-value-input").role(Role::TextInput).aria_label(self.desc.name.clone()).aria_value(self.text.text.clone())
                .h(px(38.)).flex_none().rounded(px(5.)).bg(th.control).border_1().border_color(th.accent).overflow_hidden()
                .child(canvas(|_, _, _| (), move |bounds, _, window, cx| paint(&entity, bounds, window, cx)).size_full()))
            .child(div().text_xs().text_color(th.text_secondary).child(format!("{} … {} {} · Enter 应用 · Esc 取消", self.desc.min, self.desc.max, self.desc.unit)))
            .child(div().flex_1().text_xs().text_color(th.error).child(self.error.clone()))
            .child(div().flex().justify_end().gap(px(8.))
                .child(crate::ui_controls::selectable_button("value-cancel", "取消", false, th).on_click(cx.listener(|_, _, window, _| window.remove_window())))
                .child(crate::ui_controls::selectable_button("value-apply", "应用", true, th).on_click(cx.listener(|view, _, window, cx| view.apply(window, cx)))))
    }
}

impl EntityInputHandler for ValueEntry {
    fn text_for_range(&mut self, range: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let (text, adjusted) = self.text.text_for_range_utf16(range); *actual = Some(adjusted); Some(text)
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.text.range_to_utf16(&self.text.selection), reversed: self.text.reversed })
    }
    fn set_selected_text_range(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        self.text.selection = self.text.range_from_utf16(&range); self.text.reversed = false;
        window.notify_text_selection_changed(&self.focus); cx.notify();
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> { Some(self.text.text.encode_utf16().count()) }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> { self.text.marked.as_ref().map(|range| self.text.range_to_utf16(range)) }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) { self.text.unmark_text(); window.notify_text_input_changed(&self.focus); cx.notify(); }
    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.text.replace_text_in_range(range, text); self.error.clear(); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selected: Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) {
        self.text.replace_and_mark_text_in_range(range, text, selected); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn bounds_for_range(&mut self, range: Range<usize>, bounds: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let line = self.line.as_ref()?; let range = self.text.range_from_utf16(&range);
        Some(Bounds::from_corners(point(bounds.left() + px(6.) + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + px(6.) + line.x_for_index(range.end), bounds.bottom())))
    }
    fn character_index_for_point(&mut self, point: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let local = self.bounds?.localize(&point)?;
        Some(self.text.offset_to_utf16(self.line.as_ref()?.closest_index_for_x(local.x - px(6.))))
    }
}

fn paint(entity: &Entity<ValueEntry>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let view = entity.read(cx);
    let (state, focus) = (view.text.clone(), view.focus.clone());
    let th = view.editor.upgrade().map(|ed| ed.read(cx).theme).unwrap_or_else(|| Theme::for_appearance(window.appearance()));
    window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
    let shown: SharedString = state.text.clone().into();
    let base = TextRun { len: shown.len(), font: window.text_style().font(), color: th.text.into(), background_color: None, underline: None, strikethrough: None };
    let runs = if let Some(marked) = &state.marked {
        vec![TextRun { len: marked.start, ..base.clone() }, TextRun { len: marked.end - marked.start,
            underline: Some(gpui::UnderlineStyle { color: Some(th.accent.into()), thickness: px(1.), wavy: false }), ..base.clone() },
            TextRun { len: shown.len() - marked.end, ..base }]
    } else { vec![base] };
    let line = window.text_system().shape_line(shown, px(15.), &runs, None);
    let origin = point(bounds.left() + px(6.), bounds.top() + px(8.));
    if !state.selection.is_empty() {
        let start = line.x_for_index(state.selection.start); let end = line.x_for_index(state.selection.end);
        window.paint_quad(fill(Bounds::new(point(origin.x + start, origin.y), size(end - start, px(20.))), with_alpha(th.accent, 0.3)));
    }
    let _ = line.paint(origin, px(20.), gpui::TextAlign::Left, None, window, cx);
    if focus.is_focused(window) && state.selection.is_empty() {
        let x = line.x_for_index(state.cursor());
        window.paint_quad(fill(Bounds::new(point(origin.x + x, origin.y), size(px(1.), px(20.))), th.accent));
    }
    entity.update(cx, |view, _| { view.line = Some(line); view.bounds = Some(bounds); });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn units_convert_before_range_validation_and_nonfinite_is_rejected() {
        let hz = ParamDesc { id: "freq".into(), name: "频率".into(), value: 1000., min: 20., max: 20000., unit: "Hz".into(), step_count: 0, default_value: None };
        assert_eq!(parse_value("1.25 kHz", &hz).unwrap(), 1250.);
        assert_eq!(parse_value("1000 Hz", &hz).unwrap(), 1000.);
        assert!(parse_value("25 kHz", &hz).is_err());
        for bad in ["NaN", "inf", "-inf", "", "12xyz", "1khzkhz", "20 Hz Hz"] { assert!(parse_value(bad, &hz).is_err()); }
        let ms = ParamDesc { unit: "ms".into(), min: 0., max: 2000., ..hz.clone() };
        assert_eq!(parse_value("0.3 s", &ms).unwrap(), 300.);
        let seconds = ParamDesc { unit: "s".into(), min: 0., max: 20., ..hz };
        assert_eq!(parse_value("250 ms", &seconds).unwrap(), 0.25);
    }
}
