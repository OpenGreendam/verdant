//! Retained compiled collection with a native content slot. Menu/table content
//! and project transactions are application facts; source owns interaction.
use std::rc::Rc;
use gpui::{prelude::*, App, Context, ElementId, Entity, Focusable, IntoElement, Subscription, Window};
use serde_json::Value;
use verdant_collection_control::{CollectionControl, CollectionControlHandle};
use verdant_collection_control::native::gpui_native_owner::NativeSnapshot;

pub(crate) type Draw = Rc<dyn Fn(&CollectionControlHandle, &gpui::FocusHandle, &mut Window, &mut App) -> gpui::AnyElement>;
pub(crate) type Sink = Rc<dyn Fn(&str, &Value, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct Collection {
    id: ElementId, props: Value, items: Vec<u8>, draw: Draw, sink: Sink,
}
pub(crate) fn collection(id: impl Into<ElementId>, props: Value, items: Vec<u8>, draw: Draw, sink: Sink) -> Collection {
    Collection { id: id.into(), props, items, draw, sink }
}

pub(crate) fn call(handle: &CollectionControlHandle, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) {
    if let Err(error) = handle.call(method, args, window, cx) { eprintln!("compiled collection {method}: {error:?}"); }
}

struct Drawing { handle: CollectionControlHandle, focus: gpui::FocusHandle, draw: Draw }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.handle.owner, (self.draw)(&self.handle, &self.focus, window, cx)) }
}
struct Mount {
    root: Entity<CollectionControl>, drawing: Entity<Drawing>, props: Value,
    items: Vec<u8>, sink: Sink, _changes: Subscription,
}
fn props_mode(root: &Entity<CollectionControl>, cx: &App) -> bool {
    root.read(cx).handle().state("mode").ok().and_then(|value| value.as_str().map(str::to_string)).as_deref() == Some("menu")
}
impl RenderOnce for Collection {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let props = self.props.clone(); let items = self.items.clone(); let sink = self.sink.clone(); let draw = self.draw.clone();
        let mount = window.use_keyed_state(self.id, cx, move |window, cx| {
            let root = CollectionControl::create(props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled collection setup: {error:?}"));
            let handle = root.read(cx).handle();
            call(&handle, "setItems", vec![serde_json::json!(items)], window, cx);
            let focus = root.read(cx).focus_handle(cx);
            let drawing = cx.new(|_| Drawing { handle, focus, draw });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled collection slot: {error:?}"));
            let weak = cx.entity().downgrade(); let host = window.window_handle();
            let changes = cx.observe(&root, move |mount: &mut Mount, _, cx| {
                let handle = mount.root.read(cx).handle();
                let events = handle.take_events(); let failures = handle.take_failures();
                let revision = mount.props.get("revision").cloned(); let sink = mount.sink.clone();
                let epoch = handle.owner.borrow().epoch; let owner = handle.owner;
                mount.drawing.update(cx, |_, cx| cx.notify());
                let weak = weak.clone();
                cx.defer(move |cx| {
                    let Some(mount) = weak.upgrade() else { return };
                    if mount.read(cx).props.get("revision") != revision.as_ref() { return; }
                    let _ = host.update(cx, |_, window, cx| {
                        if !failures.is_empty() { eprintln!("compiled collection failed: {failures:?}"); return; }
                        for (name, payload) in events {
                            let live = { let owner = owner.borrow(); owner.present && owner.mounted && !owner.disposed && owner.epoch == epoch };
                            if !live { return; }
                            if payload.get("revision") == revision.as_ref() { sink(&name, &payload, window, cx); }
                        }
                    });
                });
                cx.notify();
            });
            Mount { root, drawing, props, items, sink, _changes: changes }
        });
        let root = mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled container parent: {error:?}"));
            }
            mount.sink = self.sink;
            if mount.props != self.props {
                mount.root.update(cx, |root, cx| root.set_props(self.props.clone(), window, cx))
                    .unwrap_or_else(|error| panic!("compiled collection props: {error:?}"));
                mount.props = self.props;
            }
            if mount.items != self.items {
                call(&mount.root.read(cx).handle(), "setItems", vec![serde_json::json!(self.items)], window, cx);
                mount.items = self.items;
            }
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; cx.notify(); });
            mount.root.clone()
        });
        if props_mode(&root, cx) { gpui::div().absolute().inset_0().child(root).into_any_element() }
        else { root.into_any_element() }
    }
}

/// Translate the native key spelling; traversal and command decisions remain authored.
pub(crate) fn key(handle: &CollectionControlHandle, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    dispatch_key(handle, None, event, window, cx);
}

pub(crate) fn key_at(handle: &CollectionControlHandle, index: usize, revision: &str, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    dispatch_key(handle, Some((index, revision)), event, window, cx);
}

fn dispatch_key(handle: &CollectionControlHandle, origin: Option<(usize, &str)>, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if window.default_prevented() { return; }
    let name = native_key_name(event.keystroke.key.as_str());
    let mods = event.keystroke.modifiers;
    let (method, args) = match origin {
        Some((index, revision)) => ("keyAt", vec![index.into(), name.into(), mods.shift.into(), (mods.control || mods.platform).into(), revision.into()]),
        None => ("key", vec![name.into(), mods.shift.into(), (mods.control || mods.platform).into()]),
    };
    invoke_key(handle, method, args, window, cx);
}

pub(crate) fn key_from(handle: &CollectionControlHandle, origin: &str, revision: &str,
    event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if window.default_prevented() { return; }
    let mods = event.keystroke.modifiers;
    invoke_key(handle, "keyFrom", vec![origin.into(), native_key_name(event.keystroke.key.as_str()).into(),
        mods.shift.into(), (mods.control || mods.platform).into(), revision.into()], window, cx);
}

fn native_key_name(key: &str) -> &str {
    match key {
        "up" => "ArrowUp", "down" => "ArrowDown", "left" => "ArrowLeft", "right" => "ArrowRight",
        "home" => "Home", "end" => "End", "pageup" => "PageUp", "pagedown" => "PageDown",
        "enter" => "Enter", "space" => " ", "escape" => "Escape", "tab" => "Tab", "f2" => "F2",
        "delete" => "Delete", "backspace" => "Backspace", other => other,
    }
}

fn invoke_key(handle: &CollectionControlHandle, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) {
    match handle.call(method, args, window, cx) {
        Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); }
        Ok(_) => {}, Err(error) => eprintln!("compiled collection key: {error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, WindowHandle, div, px, size, uniform_list};

    struct Harness {
        selected: usize, activations: usize, revision: usize,
        scroll: gpui::UniformListScrollHandle, native_scroll: crate::proto_scroll::NativeScroll,
        collection: Option<CollectionControlHandle>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity = cx.entity(); let revision = format!("rows:{}", self.revision);
            let draw: Draw = Rc::new(move |handle, _, _, cx| entity.update(cx, |view, _| {
                view.collection = Some(handle.clone());
                let scroll = view.scroll.clone();
                crate::proto_scroll::content("collection-test-scroll", "Virtual collection scroll", move |_, _| {
                    uniform_list("collection-test-rows", 100, |range, _, _| range.map(|index|
                        div().h(px(30.)).child(format!("Row {index}"))).collect::<Vec<_>>())
                        .track_scroll(&scroll).size_full().into_any_element()
                }).native_scroll(view.native_scroll.clone()).into_any_element()
            }));
            let entity = cx.entity(); let basis = revision.clone();
            let sink: Sink = Rc::new(move |name, payload, _, cx| {
                if payload.get("revision").and_then(Value::as_str) != Some(basis.as_str()) { return; }
                entity.update(cx, |view, cx| {
                    let Some(index) = crate::proto_controls::source_index(payload.get("index")) else { return; };
                    if name == "select" || name == "activate" {
                        view.selected = index; view.scroll.scroll_to_item(index, gpui::ScrollStrategy::Nearest);
                        if name == "activate" { view.activations += 1; }
                        cx.notify();
                    }
                });
            });
            div().size_full().child(collection("collection-test", serde_json::json!({"mode":"listbox","label":"Virtual collection",
                "cursor":self.selected,"revision":revision,"autoFocus":true}), vec![1;100], draw, sink))
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(320.), px(180.)), |_, _| {
            let scroll = gpui::UniformListScrollHandle::new();
            let native_scroll = crate::proto_scroll::NativeScroll::new(scroll.0.borrow().base_handle.clone());
            Harness { selected:0, activations:0, revision:1, scroll, native_scroll, collection:None }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx); window
    }

    #[gpui::test]
    fn native_collection_page_navigation_reveals_the_existing_virtual_list_and_rejects_stale_rows(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |view, window, cx| { view.collection.as_ref().unwrap().call("focusSelf", vec![], window, cx).unwrap(); }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "pagedown");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |view, _, cx| {
            assert_eq!(view.selected, 20);
            assert!(view.scroll.0.borrow().base_handle.offset().y < px(0.), "source traversal must reveal the selected native virtual row");
            view.revision = 2; cx.notify();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |view, window, cx| {
            let handle = view.collection.as_ref().unwrap();
            assert_eq!(handle.call("choose", vec![3.into(), false.into(), "rows:1".into()], window, cx).unwrap(), Value::Bool(false));
            assert_eq!(handle.call("keyAt", vec![30.into(), "z".into(), false.into(), true.into(), "rows:2".into()], window, cx).unwrap(), Value::Bool(false));
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |view, _, _| { assert_eq!(view.selected, 20); assert_eq!(view.activations, 0); }).unwrap();
        cx.simulate_keystrokes(window.into(), "end enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |view, _, _| { assert_eq!(view.selected, 99); assert_eq!(view.activations, 1); }).unwrap();
    }
}
