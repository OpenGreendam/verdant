//! Retained compiled collection with a native content slot. Menu/table content
//! and project transactions are application facts; source owns interaction.
use std::rc::Rc;
use gpui::{prelude::*, App, Context, ElementId, Entity, Focusable, IntoElement, Subscription, Window};
use serde_json::Value;
use crate::proto_scroll::{Engine, EngineBinding};
use verdant_collection_control::{CollectionControl, CollectionControlHandle};
use verdant_collection_control::native::gpui_native_owner::NativeSnapshot;

pub(crate) type Draw = Rc<dyn Fn(&CollectionControlHandle, &gpui::FocusHandle, Option<EngineBinding>, &mut Window, &mut App) -> gpui::AnyElement>;
pub(crate) type Sink = Rc<dyn Fn(&str, &Value, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct Collection {
    id: ElementId, props: Value, items: Vec<u8>, draw: Draw, sink: Sink, engine: Option<Engine>,
}
pub(crate) fn collection(id: impl Into<ElementId>, props: Value, items: Vec<u8>, draw: Draw, sink: Sink) -> Collection {
    Collection { id: id.into(), props, items, draw, sink, engine:None }
}

impl Collection { pub(crate) fn scroll_engine(mut self, engine: Engine) -> Self { self.engine = Some(engine); self } }

pub(crate) fn call(handle: &CollectionControlHandle, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) {
    if let Err(error) = handle.call(method, args, window, cx) { eprintln!("compiled collection {method}: {error:?}"); }
}

struct Drawing { handle: CollectionControlHandle, focus: gpui::FocusHandle, draw: Draw, engine: Option<EngineBinding> }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.handle.owner, (self.draw)(&self.handle, &self.focus, self.engine.clone(), window, cx)) }
}
struct Mount {
    root: Entity<CollectionControl>, drawing: Entity<Drawing>, props: Value,
    items: Vec<u8>, sink: Sink, engine: Option<EngineBinding>, _changes: Subscription,
}
fn props_mode(root: &Entity<CollectionControl>, cx: &App) -> bool {
    root.read(cx).handle().state("mode").ok().and_then(|value| value.as_str().map(str::to_string)).as_deref() == Some("menu")
}
impl RenderOnce for Collection {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let props = self.props.clone(); let items = self.items.clone(); let sink = self.sink.clone(); let draw = self.draw.clone(); let initial_engine = self.engine.clone();
        let mount = window.use_keyed_state(self.id, cx, move |window, cx| {
            let root = CollectionControl::create(props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled collection setup: {error:?}"));
            let handle = root.read(cx).handle();
            call(&handle, "setItems", vec![serde_json::json!(items)], window, cx);
            let focus = root.read(cx).focus_handle(cx);
            let mut engine = None; EngineBinding::sync(&mut engine, initial_engine, &handle.owner, cx);
            let drawing = cx.new(|_| Drawing { handle, focus, draw, engine:engine.clone() });
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
                            let live = { let owner = owner.borrow(); owner.present && owner.mounted && !owner.disposed && owner.epoch == epoch
                                && owner.props.get("revision") == revision.as_ref()
                                && owner.props.get("disabled").and_then(Value::as_bool) != Some(true) };
                            if !live { return; }
                            if payload.get("revision") == revision.as_ref() { sink(&name, &payload, window, cx); }
                        }
                    });
                });
                cx.notify();
            });
            Mount { root, drawing, props, items, sink, engine, _changes: changes }
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
            EngineBinding::sync(&mut mount.engine, self.engine, &mount.root.read(cx).handle().owner, cx);
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; drawing.engine = mount.engine.clone(); cx.notify(); });
            mount.root.clone()
        });
        let menu = props_mode(&root, cx);
        let keyboard = root.read(cx).handle();
        let body = gpui::div().on_key_down(move |event, window, cx| key(&keyboard, event, window, cx)).child(root);
        if menu { body.absolute().inset_0().into_any_element() }
        else { body.size_full().min_h_0().into_any_element() }
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
    let name = native_key(event);
    let mods = event.keystroke.modifiers;
    let (method, args) = match origin {
        Some((index, revision)) => ("keyAt", vec![index.into(), name.into(), mods.shift.into(), (mods.control || mods.platform).into(), revision.into()]),
        None => ("key", vec![name.into(), mods.shift.into(), (mods.control || mods.platform).into()]),
    };
    match handle.call(method, args, window, cx) {
        Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); }
        Ok(_) => {}, Err(error) => eprintln!("compiled collection key: {error:?}"),
    }
}

fn native_key(event: &gpui::KeyDownEvent) -> &str {
    match event.keystroke.key.as_str() {
        "up" => "ArrowUp", "down" => "ArrowDown", "left" => "ArrowLeft", "right" => "ArrowRight",
        "home" => "Home", "end" => "End", "pageup" => "PageUp", "pagedown" => "PageDown",
        "enter" => "Enter", "space" => " ", "escape" => "Escape", "tab" => "Tab", "f2" => "F2",
        "delete" => "Delete", "backspace" => "Backspace", "f10" => "F10", "contextmenu" => "ContextMenu", other => other,
    }
}

/// Capture transports focus/composition facts before the text engine handles
/// its default key. The compiled collection decides whether it owns that key.
pub(crate) fn filter_key(handle: &CollectionControlHandle, event: &gpui::KeyDownEvent,
    composing: bool, from_filter: bool, revision: &str, window: &mut Window, cx: &mut App) {
    let mods = event.keystroke.modifiers;
    if window.default_prevented() { return; }
    let args = vec![native_key(event).into(), mods.shift.into(), (mods.control || mods.platform).into(),
        composing.into(), from_filter.into(), revision.into(), mods.alt.into()];
    match handle.call("filterKey", args, window, cx) {
        Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); }
        Ok(_) => {}, Err(error) => eprintln!("compiled filter key: {error:?}"),
    }
}

/// Measure this physical engine after layout. No native offset policy lives
/// here: the source validates authority and computes the ScrollSurface request.
pub(crate) fn reveal_item(handle: CollectionControlHandle, index: usize, revision: String,
    engine: EngineBinding, horizontal: bool, window: &mut Window) {
    window.on_next_frame(move |window, cx| {
        let scroll = engine.handle(cx);
        let Some(item) = scroll.bounds_for_item(index) else { return };
        let viewport = scroll.bounds(); let offset = scroll.offset();
        let (start, end, viewport_start, viewport_end) = if horizontal {
            (item.left() + offset.x, item.right() + offset.x, viewport.left(), viewport.right())
        } else {
            (item.top() + offset.y, item.bottom() + offset.y, viewport.top(), viewport.bottom())
        };
        call(&handle, "revealGeometry", vec![index.into(), f64::from(start.as_f32()).into(), f64::from(end.as_f32()).into(),
            f64::from(viewport_start.as_f32()).into(), f64::from(viewport_end.as_f32()).into(), revision.into()], window, cx);
    });
    // on_next_frame schedules its own platform frame. request_animation_frame
    // is a render-only convenience (it reads current_view) and cannot be used
    // from this deferred source-event delivery path.
}

/// The virtualizer supplies measured geometry, including offscreen rows; the
/// compiled Collection decides whether and how far its actual engine reveals.
/// These uniform viewports have no padding/border, so their measured content
/// starts at the base engine's bounds origin. No native ScrollStrategy runs.
pub(crate) fn reveal_uniform_item(key:&'static str, source:&CollectionControlHandle,
    index:usize, revision:&str, count:usize, scroll:&gpui::UniformListScrollHandle, window:&mut Window, cx:&mut App) {
    let request=(index,revision.to_string(),count);
    let remembered=window.use_keyed_state(key,cx,|_,_|None::<(usize,String,usize)>);
    let changed=remembered.update(cx,|last,_| {if last.as_ref()==Some(&request) {false} else {*last=Some(request);true}});
    if !changed || index>=count || count==0 {return;}
    let source=source.clone();let revision=revision.to_string();let scroll=scroll.clone();
    window.on_next_frame(move|window,cx| {
        let (start,end,top,bottom)={
            let scroll=scroll.0.borrow();let Some(size)=scroll.last_item_size else {return};
            let viewport=scroll.base_handle.bounds();let offset=scroll.base_handle.offset();
            let row_height=size.contents.height/count as f32;
            let physical_index=if scroll.y_flipped {count-index-1} else {index};
            let start=viewport.top()+offset.y+row_height*physical_index as f32;
            (start,start+row_height,viewport.top(),viewport.bottom())
        };
        call(&source,"revealGeometry",vec![index.into(),f64::from(start.as_f32()).into(),
            f64::from(end.as_f32()).into(),f64::from(top.as_f32()).into(),f64::from(bottom.as_f32()).into(),revision.into()],window,cx);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, UniformListScrollHandle, div, px, size, uniform_list};
    use verdant_collection_control::native::gpui_native_owner::{all_owners, resolved};

    struct Harness { scroll: UniformListScrollHandle, cursor: usize }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let list = self.scroll.clone();
            let draw: Draw = Rc::new(move |_,_,engine,_,_| {
                let engine = engine.expect("virtual list viewport");
                uniform_list("virtual-scroll-rows", 200, |range,_,_| range.map(|index| div().h(px(28.)).child(format!("Row {index}"))).collect())
                    .track_scroll(&list).size_full().on_scroll_wheel(move |_,window,cx| engine.notify_input(window,cx)).into_any_element()
            });
            let entity = cx.entity();
            let sink: Sink = Rc::new(move |name,payload,_,cx| { if name == "select" {
                if let Some(index) = crate::proto_controls::source_index(payload.get("index")) { entity.update(cx, |host,cx| { host.cursor = index; cx.notify(); }); }
            }});
            collection("virtual-scroll-source", serde_json::json!({"mode":"table", "label":"Virtual source viewport", "cursor":self.cursor, "revision":"list:1"}), vec![1;200], draw, sink)
                .scroll_engine(Engine::Uniform(self.scroll.clone()))
        }
    }
    fn source(window: &Window) -> CollectionControlHandle {
        let owner = all_owners().into_iter().find(|owner| { let owner=owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "Virtual source viewport")
        }).expect("mounted generated collection"); CollectionControlHandle { owner }
    }

    #[gpui::test]
    fn native_virtual_list_reuses_its_actual_engine_for_source_page_and_edge_navigation(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(300.), px(224.)), |_,_| Harness { scroll:Default::default(), cursor:0 });
        window.update(cx, |_,window,_| window.activate_window()).unwrap(); crate::native_test_support::frames(window,cx);
        window.update(cx, |host,window,cx| {
            let source=source(window); source.call("focusSelf",vec![],window,cx).unwrap();
            assert!(host.scroll.0.borrow().base_handle.max_offset().y > px(4000.));
            assert_ne!(source.owner.borrow().modules.scroll.external_engine,0,"only the virtual list owns viewport layout");
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"pagedown"); crate::native_test_support::frames(window,cx);
        window.update(cx, |host,window,_| {
            let source=source(window);
            let physical=host.scroll.0.borrow().base_handle.offset();
            assert!(physical.y < px(-100.));
            assert_eq!(source.owner.borrow().modules.scroll.handle.as_ref().unwrap().offset(),physical,"source and virtualizer share one engine");
            assert_eq!(host.cursor,20); assert!(source.state("scrollPosition").unwrap().as_f64().unwrap() > 0.);
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"home"); crate::native_test_support::frames(window,cx);
        window.update(cx, |host,_,_| { assert_eq!(host.cursor,0); assert_eq!(host.scroll.0.borrow().base_handle.offset().y,px(0.)); }).unwrap();
    }
}

#[cfg(test)]
mod incoming_admission_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, WindowHandle, div, px, size, uniform_list};

    struct Harness {
        selected: usize, activations: usize, revision: usize,
        scroll: gpui::UniformListScrollHandle,
        collection: Option<CollectionControlHandle>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity = cx.entity(); let revision = format!("rows:{}", self.revision);
            let draw: Draw = Rc::new(move |handle, _, engine, window, cx| {
                let (scroll, selected, revision) = entity.update(cx, |view, _| {
                    view.collection = Some(handle.clone());
                    (view.scroll.clone(), view.selected, format!("rows:{}", view.revision))
                });
                reveal_uniform_item("collection-admission-reveal", handle, selected, &revision, 100, &scroll, window, cx);
                let engine = engine.expect("the collection leases the virtual list's actual engine");
                uniform_list("collection-test-rows", 100, |range, _, _| range.map(|index|
                    div().h(px(30.)).child(format!("Row {index}"))).collect::<Vec<_>>())
                    .track_scroll(&scroll).size_full()
                    .on_scroll_wheel(move |_, window, cx| engine.notify_input(window, cx)).into_any_element()
            });
            let entity = cx.entity(); let basis = revision.clone();
            let sink: Sink = Rc::new(move |name, payload, _, cx| {
                if payload.get("revision").and_then(Value::as_str) != Some(basis.as_str()) { return; }
                entity.update(cx, |view, cx| {
                    let Some(index) = crate::proto_controls::source_index(payload.get("index")) else { return; };
                    if name == "select" || name == "activate" {
                        view.selected = index;
                        if name == "activate" { view.activations += 1; }
                        cx.notify();
                    }
                });
            });
            div().size_full().child(collection("collection-test", serde_json::json!({"mode":"listbox","label":"Virtual collection",
                "cursor":self.selected,"revision":revision,"autoFocus":true}), vec![1;100], draw, sink)
                .scroll_engine(Engine::Uniform(self.scroll.clone())))
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(320.), px(180.)), |_, _| {
            let scroll = gpui::UniformListScrollHandle::new();
            Harness { selected:0, activations:0, revision:1, scroll, collection:None }
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
