//! In-window prompts as AdwAlertDialog: a dimmed scrim, a rounded card with
//! a centered heading and body, and pill buttons. The first non-cancel
//! answer is the suggested action (accent); Enter picks it and Escape picks
//! the cancel answer. Replaces GPUI's unstyled fallback prompt wherever the
//! platform has no native dialog.

use std::sync::Mutex;

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, FontWeight, PromptButton, PromptHandle, PromptLevel,
    PromptResponse, Render, RenderablePromptHandle, Window, div, prelude::*, px,
};

use crate::theme::{Theme, with_alpha};

/// The theme the editor last rendered with; prompts follow it.
static THEME: Mutex<Option<Theme>> = Mutex::new(None);

pub fn remember_theme(theme: Theme) { *THEME.lock().unwrap_or_else(|p| p.into_inner()) = Some(theme); }

fn suggested_response(handle: &verdant_overlay_control::OverlayControlHandle) -> Option<usize> {
    crate::proto_controls::source_index(handle.state("defaultChoice").ok().as_ref())
}

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

impl EventEmitter<PromptResponse> for AlertDialog {}

impl Focusable for AlertDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() }
}

impl Render for AlertDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let revision = format!("alert:{}", cx.entity_id());
        let props = serde_json::json!({"label":self.heading, "description":self.body.clone().unwrap_or_default(),
            "role":"alertdialog", "open":true, "revision":revision, "choices":self.actions.len(),
            "responseKinds":self.actions.iter().map(|action| if action.is_cancel() {2} else {1}).collect::<Vec<_>>()});
        let editor = cx.entity(); let basis = revision.clone();
        let draw = std::rc::Rc::new(move |handle: &verdant_overlay_control::OverlayControlHandle, focus: &FocusHandle, window: &mut Window, cx: &mut App| {
            editor.update(cx, |alert, cx| {
                alert.focus = focus.clone();
                alert.native_content(handle, &basis, window, cx).into_any_element()
            })
        });
        let editor = cx.entity().downgrade();
        let response = std::rc::Rc::new(move |index: usize, _window: &mut Window, cx: &mut App| {
            let _ = editor.update(cx, |alert, cx| { if index < alert.actions.len() { cx.emit(PromptResponse(index)); } });
        });
        crate::proto_overlay::overlay("compiled-alert-dialog", props, draw, response)
    }
}

impl AlertDialog {
    fn native_content(&self, handle: &verdant_overlay_control::OverlayControlHandle, revision: &str, window: &mut Window, _cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = theme(window);
        let suggested = suggested_response(handle);
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
            let response = handle.clone(); let basis = revision.to_string();
            let submit = handle.clone(); let submit_basis = revision.to_string();
            crate::proto_controls::action(ix, action.label().to_string())
                .h(px(40.0)).px(px(16.0)).flex().items_center().justify_center().rounded(px(10.0))
                .bg(bg).text_color(fg).text_sm().font_weight(FontWeight::SEMIBOLD).cursor_pointer()
                .hover(move |s| s.bg(with_alpha(bg, 0.85))).active(move |s| s.bg(with_alpha(bg, 0.7)))
                .when(!stacked, |b| b.flex_1())
                .child(action.label().clone())
                .on_click(move |_, window, cx| { crate::proto_overlay::choose(&response, ix, &basis, window, cx); cx.stop_propagation(); })
                .on_enter(move |window, cx| crate::proto_overlay::request_response(&submit, "Enter", &submit_basis, window, cx))
        });
        let keyboard = handle.clone();
        let card = div().id("alert-dialog").w(px(372.0)).max_w(px(560.0)).flex().flex_col().gap(px(6.0))
            .p(px(24.0)).rounded(px(14.0)).bg(th.popover).border_1().border_color(th.shade).shadow_lg()
            .on_key_down(move |event, window, cx| crate::proto_overlay::key(&keyboard, event, window, cx))
            .child(div().w_full().text_center().text_color(th.text).font_weight(FontWeight::BOLD).child(self.heading.clone()))
            .children(self.body.clone().map(|body| div().w_full().text_center().text_sm().text_color(th.text_secondary)
                .whitespace_normal().child(body)))
            .child(div().h(px(14.0)))
            .child(div().w_full().flex().gap(px(12.0)).when(stacked, |d| d.flex_col()).children(buttons));
        let size = window.viewport_size();
        div().w(size.width).h(size.height).occlude().bg(gpui::rgba(0x00000066)).flex().items_center().justify_center().child(card).into_any_element()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, Entity, Subscription, TestAppContext, size};
    use verdant_action_control::native::gpui_native_owner::{all_owners, resolved};

    struct Harness {
        alert: Entity<AlertDialog>, responses: Vec<usize>, _responses: Subscription,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.alert.clone()
        }
    }
    fn focus_action(label: &str, window: &mut Window, cx: &mut App) {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == label)
        }).expect("actual alert response action");
        verdant_action_control::ActionControlHandle { owner }.call("focusSelf", vec![], window, cx).unwrap();
    }
    fn release_key(window: gpui::WindowHandle<Harness>, cx: &mut TestAppContext, key: &'static str) {
        // simulate_keystrokes emits KeyDown/text only. Release each physical
        // key so a subsequent press starts its own source keyboard lease.
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse(key).unwrap(),
            }), cx);
        }).unwrap();
    }

    #[gpui::test]
    fn alert_enter_uses_source_default_while_space_and_escape_keep_their_response(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(620.), px(440.)), |_, cx| {
            let alert = cx.new(|cx| AlertDialog {
                level: PromptLevel::Warning, heading: "恢复工程？".into(), body: None,
                actions: vec![PromptButton::cancel("取消"), PromptButton::new("恢复"), PromptButton::new("放弃")],
                focus: cx.focus_handle(),
            });
            let responses = cx.subscribe(&alert, |host: &mut Harness, _, response: &PromptResponse, cx| {
                host.responses.push(response.0); cx.notify();
            });
            Harness { alert, responses: Vec::new(), _responses: responses }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, cx| {
            let owner = all_owners().into_iter().find(|owner| {
                let owner = owner.borrow();
                owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                    && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "恢复工程？")
            }).expect("compiled alert owner");
            let source = verdant_overlay_control::OverlayControlHandle { owner };
            assert_eq!(suggested_response(&source), Some(1), "the native source number selects the emphasized response style");
            focus_action("取消", window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "enter");
        release_key(window, cx, "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.responses, vec![1], "Enter resolves the source default, even when the cancel button has focus")).unwrap();
        cx.simulate_keystrokes(window.into(), "space");
        release_key(window, cx, "space");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.responses, vec![1, 0], "Space activates the focused response")).unwrap();
        cx.simulate_keystrokes(window.into(), "escape");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.responses, vec![1, 0, 0], "Escape selects the source cancel response exactly once")).unwrap();
    }
}
