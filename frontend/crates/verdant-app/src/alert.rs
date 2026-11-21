//! In-window prompts as AdwAlertDialog: a dimmed scrim, a rounded card with
//! a centered heading and body, and pill buttons. The first non-cancel
//! answer is the suggested action (accent); Enter picks it and Escape picks
//! the cancel answer. Replaces GPUI's unstyled fallback prompt wherever the
//! platform has no native dialog.

use std::sync::Mutex;

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, FontWeight, KeyDownEvent, PromptButton, PromptHandle, PromptLevel,
    PromptResponse, Render, RenderablePromptHandle, Window, div, prelude::*, px,
};

use crate::theme::{Theme, with_alpha};

/// The theme the editor last rendered with; prompts follow it.
static THEME: Mutex<Option<Theme>> = Mutex::new(None);

pub fn remember_theme(theme: Theme) { *THEME.lock().unwrap_or_else(|p| p.into_inner()) = Some(theme); }

fn theme(window: &Window) -> Theme {
    THEME.lock().unwrap_or_else(|p| p.into_inner()).unwrap_or_else(|| Theme::for_appearance(window.appearance()))
}

pub fn install(cx: &mut App) { cx.set_prompt_builder(build); }

fn build(level: PromptLevel, message: &str, detail: Option<&str>, actions: &[PromptButton], handle: PromptHandle,
    window: &mut Window, cx: &mut App) -> RenderablePromptHandle {
    let view = cx.new(|cx| AlertDialog {
        level, heading: message.to_string(), body: detail.map(str::to_string), actions: actions.to_vec(), focus: cx.focus_handle(),
    });
    handle.with_view(view, window, cx)
}

pub struct AlertDialog {
    level: PromptLevel,
    heading: String,
    body: Option<String>,
    actions: Vec<PromptButton>,
    focus: FocusHandle,
}

impl AlertDialog {
    /// The suggested answer: the first that is not a cancel.
    pub fn suggested(actions: &[PromptButton]) -> Option<usize> { actions.iter().position(|a| !a.is_cancel()) }
    pub fn cancel(actions: &[PromptButton]) -> Option<usize> { actions.iter().position(PromptButton::is_cancel) }
}

impl EventEmitter<PromptResponse> for AlertDialog {}

impl Focusable for AlertDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() }
}

impl Render for AlertDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = theme(window);
        let suggested = Self::suggested(&self.actions);
        let destructive = self.level == PromptLevel::Critical;
        // Two short answers sit side by side; more stack, as libadwaita does.
        let stacked = self.actions.len() > 2 || self.actions.iter().any(|a| a.label().chars().count() > 8);
        let buttons = self.actions.iter().enumerate().map(|(ix, action)| {
            let primary = Some(ix) == suggested;
            let (bg, fg) = match (primary, destructive) {
                (true, true) => (th.error, gpui::rgb(0xffffff)),
                (true, false) => (th.accent, gpui::rgb(0xffffff)),
                _ => (th.control, th.text),
            };
            div().id(ix).role(gpui::Role::Button).aria_label(action.label().to_string())
                .h(px(40.0)).px(px(16.0)).flex().items_center().justify_center().rounded(px(10.0))
                .bg(bg).text_color(fg).text_sm().font_weight(FontWeight::SEMIBOLD).cursor_pointer()
                .hover(move |s| s.bg(with_alpha(bg, 0.85))).active(move |s| s.bg(with_alpha(bg, 0.7)))
                .when(!stacked, |b| b.flex_1())
                .child(action.label().clone())
                .on_click(cx.listener(move |_, _, _, cx| { cx.emit(PromptResponse(ix)); cx.stop_propagation(); }))
        });
        let actions = self.actions.clone();
        let card = div().id("alert-dialog").role(gpui::Role::AlertDialog).aria_label(self.heading.clone())
            .track_focus(&self.focus).w(px(372.0)).max_w(px(560.0)).flex().flex_col().gap(px(6.0))
            .p(px(24.0)).rounded(px(14.0)).bg(th.popover).border_1().border_color(th.shade).shadow_lg()
            .on_key_down(cx.listener(move |_, ev: &KeyDownEvent, _, cx| {
                let pick = match ev.keystroke.key.as_str() {
                    "enter" => Self::suggested(&actions),
                    "escape" => Self::cancel(&actions),
                    _ => None,
                };
                if let Some(ix) = pick { cx.emit(PromptResponse(ix)); cx.stop_propagation(); }
            }))
            .child(div().w_full().text_center().text_color(th.text).font_weight(FontWeight::BOLD).child(self.heading.clone()))
            .children(self.body.clone().map(|body| div().w_full().text_center().text_sm().text_color(th.text_secondary)
                .whitespace_normal().child(body)))
            .child(div().h(px(14.0)))
            .child(div().w_full().flex().gap(px(12.0)).when(stacked, |d| d.flex_col()).children(buttons));
        let size = window.viewport_size();
        div().w(size.width).h(size.height).occlude().bg(gpui::rgba(0x00000066)).flex().items_center().justify_center().child(card)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_is_the_first_non_cancel_answer_and_escape_finds_cancel() {
        let actions = [PromptButton::cancel("取消"), PromptButton::new("恢复"), PromptButton::ok("放弃")];
        assert_eq!(AlertDialog::suggested(&actions), Some(1));
        assert_eq!(AlertDialog::cancel(&actions), Some(0));
        assert_eq!(AlertDialog::cancel(&[PromptButton::new("好")]), None);
    }
}
