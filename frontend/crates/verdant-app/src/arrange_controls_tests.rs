//! Actual track header consumer: source list selection/context and nested action ownership.
use super::*;
use gpui::{AppContext, Render, TestAppContext, size};
use verdant_collection_control::{CollectionControlHandle, native::gpui_native_owner::{all_owners, resolved}};

struct Harness { editor: Entity<Editor> }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.editor.update(cx, |editor, cx| track_headers(editor, cx).into_any_element())
    }
}
fn source(window: &Window) -> CollectionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "编曲轨道")
    }).expect("mounted actual track list");
    CollectionControlHandle { owner }
}

#[gpui::test]
fn native_track_headers_own_nested_activation_arrow_selection_and_keyboard_context(cx: &mut TestAppContext) {
    let window = cx.open_window(size(px(320.), px(420.)), |window, cx| Harness {
        editor: cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx)),
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap(); crate::native_test_support::frames(window, cx);
    let original_color = window.update(cx, |host, _, cx| host.editor.read(cx).project().tracks[1].color.clone()).unwrap();
    window.update(cx, |_, window, cx| {
        let mut colors = all_owners().into_iter().filter(|owner| {
            let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "切换轨道颜色")
        }).collect::<Vec<_>>();
        colors.sort_by(|a,b| a.borrow().bounds.unwrap().origin.y.partial_cmp(&b.borrow().bounds.unwrap().origin.y).unwrap());
        assert_eq!(colors.len(), 2);
        let position = colors[1].borrow().bounds.unwrap().center();
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
    }).unwrap(); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let editor = host.editor.read(cx);
        assert_eq!(editor.session.track, editor.project().tracks[0].id, "child action must not also select its containing row");
        assert_ne!(editor.project().tracks[1].color, original_color);
        source(window).call("focusSelf", vec![], window, cx).unwrap();
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "down"); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let editor = host.editor.read(cx); assert_eq!(editor.session.track, editor.project().tracks[1].id);
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "shift-f10"); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.is_some(), "source context request reaches the admitted track command")).unwrap();
}
