//! Native content boundary for the compiled modal owner. Native drawing never
//! decides response keys, focus trapping, accepted visibility or dismissal.
use std::rc::Rc;
use gpui::{prelude::*, App, Context, ElementId, Entity, FocusHandle, Focusable, IntoElement, Subscription, Window};
use serde_json::Value;
use verdant_overlay_control::{OverlayControl, OverlayControlHandle};
use verdant_overlay_control::native::gpui_native_owner::NativeSnapshot;

pub(crate) type Draw = Rc<dyn Fn(&OverlayControlHandle, &FocusHandle, &mut Window, &mut App) -> gpui::AnyElement>;
pub(crate) type Response = Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct Overlay { id: ElementId, props: Value, draw: Draw, response: Response }
pub(crate) fn overlay(id: impl Into<ElementId>, props: Value, draw: Draw, response: Response) -> Overlay {
    Overlay { id: id.into(), props, draw, response }
}
pub(crate) fn choose(handle: &OverlayControlHandle, index: usize, revision: &str, window: &mut Window, cx: &mut App) {
    if let Err(error) = handle.call("choose", vec![index.into(), revision.into()], window, cx) {
        eprintln!("compiled overlay choice: {error:?}");
    }
}
pub(crate) fn key(handle: &OverlayControlHandle, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if window.default_prevented() { return; }
    let name = match event.keystroke.key.as_str() { "enter" => "Enter", "escape" => "Escape", _ => return };
    match handle.call("key", vec![name.into()], window, cx) {
        Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); },
        Ok(_) => {}, Err(error) => eprintln!("compiled overlay key: {error:?}"),
    }
}
struct Drawing { handle: OverlayControlHandle, focus: FocusHandle, draw: Draw }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.handle.owner, (self.draw)(&self.handle, &self.focus, window, cx)) }
}
struct Mount {
    root: Entity<OverlayControl>, drawing: Entity<Drawing>, props: Value,
    response: Response, _changes: Subscription,
}
impl RenderOnce for Overlay {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let props = self.props.clone(); let response = self.response.clone(); let draw = self.draw.clone();
        let mount = window.use_keyed_state(self.id, cx, move |window, cx| {
            let root = OverlayControl::create(props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled overlay setup: {error:?}"));
            let handle = root.read(cx).handle(); let focus = root.read(cx).focus_handle(cx);
            let drawing = cx.new(|_| Drawing { handle, focus, draw });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled overlay slot: {error:?}"));
            let weak = cx.entity().downgrade(); let host = window.window_handle();
            let changes = cx.observe(&root, move |mount: &mut Mount, _, cx| {
                let handle = mount.root.read(cx).handle(); let events = handle.take_events(); let failures = handle.take_failures();
                let revision = mount.props.get("revision").cloned(); let response = mount.response.clone();
                let epoch = handle.owner.borrow().epoch; let owner = handle.owner;
                let weak = weak.clone();
                cx.defer(move |cx| {
                    let Some(mount) = weak.upgrade() else { return };
                    if mount.read(cx).props.get("revision") != revision.as_ref() { return; }
                    let _ = host.update(cx, |_, window, cx| {
                        if !failures.is_empty() { eprintln!("compiled overlay failed: {failures:?}"); return; }
                        for (name, payload) in events {
                            let live = { let owner = owner.borrow(); owner.present && owner.mounted && !owner.disposed && owner.epoch == epoch };
                            if !live { return; }
                            if name != "response" || payload.get("revision") != revision.as_ref() { continue; }
                            if let Some(index) = crate::proto_controls::source_index(payload.get("index")) { response(index, window, cx); }
                        }
                    });
                });
                mount.drawing.update(cx, |_, cx| cx.notify()); cx.notify();
            });
            Mount { root, drawing, props, response, _changes: changes }
        });
        let root = mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled container parent: {error:?}"));
            }
            mount.response = self.response;
            if mount.props != self.props {
                mount.root.update(cx, |root, cx| root.set_props(self.props.clone(), window, cx))
                    .unwrap_or_else(|error| panic!("compiled overlay props: {error:?}"));
                mount.props = self.props;
            }
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; cx.notify(); });
            mount.root.clone()
        });
        let keyboard = root.read(cx).handle();
        gpui::div().absolute().inset_0()
            // Native descendant keys bubble through this physical boundary.
            // The source chooses default/cancel, after any focused text or
            // action source has consumed the sample (including marked text).
            .on_key_down(move |event, window, cx| key(&keyboard, event, window, cx))
            .child(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, WindowHandle, div, px, size};
    use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{all_owners, resolved}};
    use verdant_overlay_control::native::gpui_native_owner::focus_members;

    struct Harness { open: bool, response: Option<usize> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let mut body = div().size_full().child(crate::proto_controls::action("outside", "Outside modal").w(px(100.)).h(px(30.)).child("Outside"));
            if self.open {
                let draw: Draw = Rc::new(|handle, _, _, _| {
                    let first = handle.clone(); let last = handle.clone();
                    div().size_full().flex().flex_col()
                        .child(crate::proto_controls::action("first", "First modal action").w(px(140.)).h(px(32.)).child("First")
                            .on_click(move |_, w, cx| choose(&first, 0, "scope:1", w, cx)))
                        .child(crate::proto_controls::action("last", "Last modal action").w(px(140.)).h(px(32.)).child("Last")
                            .on_click(move |_, w, cx| choose(&last, 1, "scope:1", w, cx))).into_any_element()
                });
                let entity = cx.entity();
                let response: Response = Rc::new(move |index, _, cx| entity.update(cx, |host, cx| { host.response = Some(index); host.open = false; cx.notify(); }));
                body = body.child(overlay("modal", serde_json::json!({"label":"Native scope", "open":true, "revision":"scope:1", "choices":2}), draw, response));
            }
            body
        }
    }
    fn named(name: &str, window: &Window) -> ActionControlHandle {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == name)
        }).expect("mounted generated semantic root");
        ActionControlHandle { owner }
    }
    fn frames(window: WindowHandle<Harness>, cx: &mut TestAppContext) { crate::native_test_support::frames(window, cx); }

    #[gpui::test]
    fn native_overlay_slot_composes_child_focus_trap_and_restores_previous_source(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(320.), px(240.)), |_, _| Harness { open: false, response: None });
        window.update(cx, |_, window, _| window.activate_window()).unwrap(); frames(window, cx);
        window.update(cx, |_, window, cx| { named("Outside modal", window).call("focusSelf", vec![], window, cx).unwrap(); }).unwrap(); frames(window, cx);
        window.update(cx, |host, _, cx| { host.open = true; cx.notify(); }).unwrap(); frames(window, cx);
        window.update(cx, |_, window, cx| {
            let modal = named("Native scope", window); let first = named("First modal action", window); let last = named("Last modal action", window);
            assert!(first.owner.borrow().parent.ptr_eq(&Rc::downgrade(&modal.owner)));
            assert!(last.owner.borrow().parent.ptr_eq(&Rc::downgrade(&modal.owner)));
            assert_eq!(focus_members(&modal.owner, false).len(), 2, "native slot roots are logical scope members");
            first.call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap(); frames(window, cx);
        cx.simulate_keystrokes(window.into(), "tab"); frames(window, cx);
        window.update(cx, |_, window, _| assert_eq!(named("Last modal action", window).state("focused").unwrap(), serde_json::json!(true))).unwrap();
        cx.simulate_keystrokes(window.into(), "tab"); frames(window, cx);
        window.update(cx, |_, window, _| assert_eq!(named("First modal action", window).state("focused").unwrap(), serde_json::json!(true))).unwrap();
        cx.simulate_keystrokes(window.into(), "escape"); frames(window, cx);
        window.update(cx, |host, window, _| {
            assert_eq!(host.response, Some(0)); assert!(!host.open);
            assert_eq!(named("Outside modal", window).state("focused").unwrap(), serde_json::json!(true));
        }).unwrap();
    }
}
