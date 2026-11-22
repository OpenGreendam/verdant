//! Accessible, IME-aware single-line field used by preset and instance windows.
use super::*;
use gpui::{App, AppContext, Bounds, ElementInputHandler, EntityInputHandler, FocusHandle, Point, Render, TextRun, UTF16Selection, point, size};
use std::ops::Range;

pub(in crate::devices) struct TextField {
    pub text: verdant_session::TextEditState,
    pub focus: FocusHandle,
    label: String,
    theme: Theme,
    line: Option<gpui::ShapedLine>,
    bounds: Option<Bounds<Pixels>>,
    offset: Pixels,
}
impl TextField {
    pub fn new(label: &str, value: &str, theme: Theme, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self { text: verdant_session::TextEditState::new(value), focus: cx.focus_handle(), label: label.into(), theme, line: None, bounds: None, offset: px(0.) })
    }
    pub fn set(&mut self, text: &str, cx: &mut Context<Self>) {
        self.text = verdant_session::TextEditState::new(text); cx.notify();
    }
    fn edit(&mut self, cx: &mut Context<Self>, apply: impl FnOnce(&mut verdant_session::TextEditState)) {
        if !self.text.is_composing() { apply(&mut self.text); cx.notify(); }
    }
}
impl Render for TextField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        let entity = cx.entity();
        div().id("plugin-text-field").role(Role::TextInput).aria_label(self.label.clone()).aria_value(self.text.text.clone()).tab_index(0)
            .h(px(36.)).w_full().flex_none().rounded(px(6.)).bg(th.control).border_1().border_color(th.shade)
            .focus(move |s| s.border_color(th.accent)).overflow_hidden().key_context("LyricEditor").track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, window, cx| window.focus(&view.focus, cx)))
            .on_action(cx.listener(|view, _: &crate::actions::Backspace, _, cx| view.edit(cx, |s| s.backspace())))
            .on_action(cx.listener(|view, _: &crate::actions::Delete, _, cx| view.edit(cx, |s| s.delete())))
            .on_action(cx.listener(|view, _: &crate::actions::Left, _, cx| view.edit(cx, |s| s.left())))
            .on_action(cx.listener(|view, _: &crate::actions::Right, _, cx| view.edit(cx, |s| s.right())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectLeft, _, cx| view.edit(cx, |s| s.select_left())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectRight, _, cx| view.edit(cx, |s| s.select_right())))
            .on_action(cx.listener(|view, _: &crate::actions::Home, _, cx| view.edit(cx, |s| s.home())))
            .on_action(cx.listener(|view, _: &crate::actions::End, _, cx| view.edit(cx, |s| s.end())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectAllText, _, cx| view.edit(cx, |s| s.select_all())))
            .on_action(cx.listener(|view, _: &crate::actions::LyricSpace, _, cx| view.edit(cx, |s| s.replace_text_in_range(None, " "))))
            .on_action(cx.listener(|view, _: &crate::actions::PasteText, _, cx| {
                if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) { view.edit(cx, |s| s.replace_text_in_range(None, &text)); }
            }))
            .child(canvas(|_, _, _| (), move |bounds, _, window, cx| paint(&entity, bounds, window, cx)).size_full())
    }
}
impl EntityInputHandler for TextField {
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
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> { self.text.marked.as_ref().map(|r| self.text.range_to_utf16(r)) }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) { self.text.unmark_text(); window.notify_text_input_changed(&self.focus); cx.notify(); }
    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.text.replace_text_in_range(range, text); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selected: Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) {
        self.text.replace_and_mark_text_in_range(range, text, selected); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn bounds_for_range(&mut self, range: Range<usize>, bounds: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let line = self.line.as_ref()?; let range = self.text.range_from_utf16(&range);
        Some(Bounds::from_corners(point(bounds.left() + px(8.) - self.offset + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + px(8.) - self.offset + line.x_for_index(range.end), bounds.bottom())))
    }
    fn character_index_for_point(&mut self, point: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let local = self.bounds?.localize(&point)?;
        Some(self.text.offset_to_utf16(self.line.as_ref()?.closest_index_for_x(local.x - px(8.) + self.offset)))
    }
}
fn paint(entity: &Entity<TextField>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let view = entity.read(cx);
    let (state, focus, th) = (view.text.clone(), view.focus.clone(), view.theme);
    window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
    let shown: SharedString = state.text.clone().into();
    let base = TextRun { len: shown.len(), font: window.text_style().font(), color: th.text.into(), background_color: None, underline: None, strikethrough: None };
    let runs = if let Some(marked) = &state.marked {
        vec![TextRun { len: marked.start, ..base.clone() }, TextRun { len: marked.end - marked.start,
            underline: Some(gpui::UnderlineStyle { color: Some(th.accent.into()), thickness: px(1.), wavy: false }), ..base.clone() },
            TextRun { len: shown.len() - marked.end, ..base }]
    } else { vec![base] };
    let line = window.text_system().shape_line(shown, px(15.), &runs, None);
    let cursor = line.x_for_index(state.cursor());
    let offset = (cursor - bounds.size.width + px(20.)).max(px(0.));
    let origin = point(bounds.left() + px(8.) - offset, bounds.top() + px(8.));
    if focus.is_focused(window) && !state.selection.is_empty() {
        let start = line.x_for_index(state.selection.start); let end = line.x_for_index(state.selection.end);
        window.paint_quad(fill(Bounds::new(point(origin.x + start, origin.y), size(end - start, px(20.))), with_alpha(th.accent, 0.3)));
    }
    if state.text.is_empty() {
        let placeholder: SharedString = view.label.clone().into();
        let run = TextRun { len: placeholder.len(), font: window.text_style().font(), color: th.text_secondary.into(), background_color: None, underline: None, strikethrough: None };
        let hint = window.text_system().shape_line(placeholder, px(14.), &[run], None);
        let _ = hint.paint(origin, px(20.), gpui::TextAlign::Left, None, window, cx);
    }
    let _ = line.paint(origin, px(20.), gpui::TextAlign::Left, None, window, cx);
    if focus.is_focused(window) && state.selection.is_empty() {
        window.paint_quad(fill(Bounds::new(point(origin.x + cursor, origin.y), size(px(1.), px(20.))), th.accent));
    }
    entity.update(cx, |view, _| { view.line = Some(line); view.bounds = Some(bounds); view.offset = offset; });
}
