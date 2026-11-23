//! Retained native document slot inside a compiled ScrollSurface and scope.
use std::rc::Rc;
use gpui::{prelude::*, App, Context, ElementId, Entity, FocusHandle, Focusable, IntoElement, Subscription, Window};
use serde_json::Value;
use verdant_scroll_region::{ScrollRegion, ScrollRegionHandle};
use verdant_scroll_region::native::gpui_native_owner::{NativeSnapshot, OwnerRef, ScrollEngineLease, attach_scroll_engine};

/// An exact native viewport engine. UniformList retains its own virtualization;
/// the source attaches to its actual base ScrollHandle, without mirroring offsets.
#[derive(Clone)]
pub(crate) enum Engine { Native(Entity<gpui::ScrollHandle>), Uniform(gpui::UniformListScrollHandle) }
impl Engine {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Native(a), Self::Native(b)) => a.entity_id() == b.entity_id(),
            (Self::Uniform(a), Self::Uniform(b)) => Rc::ptr_eq(&a.0, &b.0), _ => false,
        }
    }
    pub(crate) fn handle(&self, cx: &App) -> gpui::ScrollHandle {
        match self { Self::Native(engine) => engine.read(cx).clone(), Self::Uniform(engine) => engine.0.borrow().base_handle.clone() }
    }
}
#[derive(Clone)]
pub(crate) struct EngineBinding { engine: Engine, lease: Rc<ScrollEngineLease> }
impl EngineBinding {
    pub(crate) fn handle(&self, cx: &App) -> gpui::ScrollHandle { self.engine.handle(cx) }
    pub(crate) fn notify_input(&self, window: &mut Window, cx: &mut App) { self.lease.notify_input(window, cx); }
    pub(crate) fn sync(current: &mut Option<Self>, engine: Option<Engine>, owner: &OwnerRef, cx: &App) {
        if match (current.as_ref(), engine.as_ref()) { (Some(current),Some(engine)) => current.engine.same(engine), (None,None) => true, _ => false } { return; }
        if let Some(previous) = current.take() { previous.lease.dispose(); }
        if let Some(engine) = engine {
            let lease = attach_scroll_engine(Rc::downgrade(owner), engine.handle(cx)).unwrap_or_else(|error| panic!("compiled viewport attachment: {error}"));
            *current = Some(Self { engine, lease:Rc::new(lease) });
        }
    }
}

pub(crate) type Draw = Rc<dyn Fn(&ScrollRegionHandle, &FocusHandle, Option<EngineBinding>, &mut Window, &mut App) -> gpui::AnyElement>;
#[derive(IntoElement)]
pub(crate) struct Region { id: ElementId, props: Value, draw: Draw, engine: Option<Engine> }
pub(crate) fn region(id: impl Into<ElementId>, props: Value, draw: Draw) -> Region { Region { id: id.into(), props, draw, engine:None } }
impl Region { pub(crate) fn scroll_engine(mut self, engine: Engine) -> Self { self.engine = Some(engine); self } }
struct Drawing { handle: ScrollRegionHandle, focus: FocusHandle, draw: Draw, engine: Option<EngineBinding> }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.handle.owner, (self.draw)(&self.handle, &self.focus, self.engine.clone(), window, cx)) }
}
struct Mount { root: Entity<ScrollRegion>, drawing: Entity<Drawing>, props: Value, engine: Option<EngineBinding>, _changes: Subscription }
impl RenderOnce for Region {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let props = self.props.clone(); let draw = self.draw.clone(); let initial_engine = self.engine.clone();
        let mount = window.use_keyed_state(self.id, cx, move |window, cx| {
            let root = ScrollRegion::create(props.clone(), initial_parent, window, cx).unwrap_or_else(|error| panic!("compiled scroll region: {error:?}"));
            let handle = root.read(cx).handle(); let focus = root.read(cx).focus_handle(cx);
            let mut engine = None; EngineBinding::sync(&mut engine, initial_engine, &handle.owner, cx);
            let drawing = cx.new(|_| Drawing { handle, focus, draw, engine:engine.clone() });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled scroll slot: {error:?}"));
            let changes = cx.observe(&root, |mount: &mut Mount, _, cx| {
                let failures = mount.root.read(cx).handle().take_failures();
                if !failures.is_empty() { eprintln!("compiled scroll failed: {failures:?}"); }
                cx.notify();
            });
            Mount { root, drawing, props, engine, _changes: changes }
        });
        let root = mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled container parent: {error:?}"));
            }
            if mount.props != self.props {
                mount.root.update(cx, |root, cx| root.set_props(self.props.clone(), window, cx)).unwrap_or_else(|error| panic!("compiled scroll props: {error:?}"));
                mount.props = self.props;
            }
            EngineBinding::sync(&mut mount.engine, self.engine, &mount.root.read(cx).handle().owner, cx);
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; drawing.engine = mount.engine.clone(); cx.notify(); });
            mount.root.clone()
        });
        let source = root.read(cx).handle();
        gpui::div().size_full().on_key_down(move |event, window, cx| key(&source,event,window,cx)).child(root)
    }
}

/// Native key spelling is transport; ScrollRegion owns navigation policy.
pub(crate) fn key(source: &ScrollRegionHandle, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if window.default_prevented() { return; }
    let name = match event.keystroke.key.as_str() {
        "up" => "ArrowUp", "down" => "ArrowDown", "left" => "ArrowLeft", "right" => "ArrowRight",
        "pageup" => "PageUp", "pagedown" => "PageDown", "home" => "Home", "end" => "End", "space" => " ", "tab" => "Tab", _ => return,
    };
    match source.call("keyInput", vec![name.into(), event.keystroke.modifiers.shift.into(),
        (event.keystroke.modifiers.control || event.keystroke.modifiers.platform).into(), event.keystroke.modifiers.alt.into()],window,cx) {
        Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); },
        Ok(_) => {}, Err(error) => eprintln!("compiled scroll key: {error:?}"),
    }
}

/// Fluent native viewport projection for application layouts. The compiled
/// root owns keyboard/focus/scroll semantics and the exact engine used by the
/// div. Native children are associated with that source owner in every phase.
#[derive(IntoElement)]
pub(crate) struct Viewport {
    shell: gpui::Stateful<gpui::Div>, children: Vec<gpui::AnyElement>, label: gpui::SharedString, axes: &'static str,
    reveal: Option<(usize, String)>,
}
pub(crate) fn viewport(id: impl Into<ElementId>, label: impl Into<gpui::SharedString>, axes: &'static str) -> Viewport {
    assert!(matches!(axes,"vertical"|"horizontal"|"both"));
    Viewport { shell:gpui::div().id(id).relative(), children:Vec::new(), label:label.into(), axes, reveal:None }
}
impl Viewport {
    /// Reveal a domain-selected direct child. Index zero is the first supplied
    /// content child; the internal semantic projection is excluded.
    pub(crate) fn reveal_item(mut self, index:usize, authority:impl Into<String>)->Self { self.reveal=Some((index,authority.into())); self }
}
impl Styled for Viewport { fn style(&mut self)->&mut gpui::StyleRefinement { self.shell.style() } }
impl ParentElement for Viewport { fn extend(&mut self, children:impl IntoIterator<Item=gpui::AnyElement>) { self.children.extend(children); } }
impl InteractiveElement for Viewport { fn interactivity(&mut self)->&mut gpui::Interactivity { self.shell.interactivity() } }
impl StatefulInteractiveElement for Viewport {}
struct ViewportMount { root:Entity<ScrollRegion>, engine:EngineBinding, props:Value, reveal:Option<(usize,String)>, _changes:Subscription }
impl RenderOnce for Viewport {
    fn render(self, window:&mut Window, cx:&mut App)->impl IntoElement {
        let mut shell=self.shell;
        let key_id=shell.interactivity().element_id.clone().expect("viewport id");
        let props=serde_json::json!({"label":self.label.as_ref(),"role":"group","trapFocus":false,"tabStop":false,"axes":self.axes,
            "revealIndex":self.reveal.as_ref().map_or(-1.,|(index,_)|*index as f64),"revealRevision":self.reveal.as_ref().map_or("",|(_,revision)|revision.as_str())});
        let parent=crate::proto_parent::current(); let initial_parent=parent.clone(); let initial_props=props.clone();
        let mount=window.use_keyed_state(key_id,cx,move|window,cx| {
            let root=ScrollRegion::create(initial_props.clone(),initial_parent,window,cx).unwrap_or_else(|error|panic!("compiled viewport: {error:?}"));
            let engine=cx.new(|_|gpui::ScrollHandle::new()); let mut binding=None;
            EngineBinding::sync(&mut binding,Some(Engine::Native(engine)),&root.read(cx).handle().owner,cx);
            let changes=cx.observe(&root,|_:&mut ViewportMount,_,cx|cx.notify());
            ViewportMount {root,engine:binding.expect("native scroll lease"),props:initial_props,reveal:None,_changes:changes}
        });
        let (root,engine,reveal)=mount.update(cx,|mount,cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner,&parent) {
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).unwrap_or_else(|error|panic!("compiled viewport parent: {error:?}"));
            }
            if mount.props!=props { mount.root.update(cx,|root,cx|root.set_props(props.clone(),window,cx)).unwrap_or_else(|error|panic!("compiled viewport props: {error:?}")); mount.props=props; }
            let reveal=if mount.reveal!=self.reveal { mount.reveal=self.reveal; mount.reveal.clone() } else { None };
            (mount.root.clone(),mount.engine.clone(),reveal)
        });
        let source=root.read(cx).handle(); let owner=source.owner.clone();
        if let Some((index,revision))=reveal {
            let source=source.clone();let engine=engine.clone();let horizontal=self.axes=="horizontal";
            window.on_next_frame(move|window,cx| {
                let physical=engine.handle(cx);
                // One absolute semantic child precedes the user content.
                let Some(item)=index.checked_add(1).and_then(|index|physical.bounds_for_item(index)) else {return};
                let viewport=physical.bounds();let offset=physical.offset();
                let (start,end,viewport_start,viewport_end)=if horizontal {
                    (item.left()+offset.x,item.right()+offset.x,viewport.left(),viewport.right())
                } else {(item.top()+offset.y,item.bottom()+offset.y,viewport.top(),viewport.bottom())};
                if let Err(error)=source.call("revealGeometry",vec![index.into(),f64::from(start.as_f32()).into(),f64::from(end.as_f32()).into(),
                    f64::from(viewport_start.as_f32()).into(),f64::from(viewport_end.as_f32()).into(),revision.into()],window,cx) { eprintln!("compiled viewport reveal: {error:?}"); }
            });
            window.request_animation_frame();
        }
        let physical=engine.handle(cx);
        if self.axes!="horizontal" { shell=shell.overflow_y_scroll(); }
        if self.axes!="vertical" { shell=shell.overflow_x_scroll(); }
        // A first, absolute semantic child sits behind the native content. Its
        // declared ScrollSurface is leased to the surrounding physical div;
        // the compiler therefore creates no second scrolling viewport.
        let shell=shell.track_scroll(&physical)
            .on_scroll_wheel(move|_,window,cx|engine.notify_input(window,cx))
            .on_key_down(move|event,window,cx|key(&source,event,window,cx))
            .child(gpui::div().absolute().inset_0().child(root)).children(self.children);
        crate::proto_parent::slot(&owner,shell.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, px, size};
    use verdant_scroll_region::native::gpui_native_owner::{all_owners, resolved};

    struct Harness;
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let draw: Draw = Rc::new(|_, _, _, _, _| {
                gpui::div().w_full().flex().flex_col()
                    .child(crate::proto_controls::action("scroll-child", "Scroll child").h(px(36.)).flex_none().child("Child"))
                    .children((0..40).map(|i| gpui::div().h(px(40.)).flex_none().child(format!("Row {i}"))))
                    .into_any_element()
            });
            region("native-scroll", serde_json::json!({"label":"Native document"}), draw)
        }
    }
    fn source(name: &str, window: &Window) -> verdant_scroll_region::native::gpui_native_owner::OwnerRef {
        all_owners().into_iter().find(|owner| {
            let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == name)
        }).expect("mounted source root")
    }

    struct ViewportHarness { clicks:usize }
    impl Render for ViewportHarness {
        fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            viewport("native-fluent-scroll","Fluent source viewport","vertical").size_full().flex().flex_col()
                .child(crate::proto_controls::action("fluent-child","Fluent child").h(px(40.)).flex_none().child("Action")
                    .on_click(cx.listener(|host,_,_,cx| { host.clicks+=1; cx.notify(); })))
                .children((0..50).map(|i|gpui::div().h(px(28.)).flex_none().child(format!("Row {i}"))))
        }
    }
    #[gpui::test]
    fn native_fluent_viewport_preserves_nested_action_hits_and_shares_real_scroll_engine(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(320.),px(224.)),|_,_|ViewportHarness{clicks:0});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        let mut hover_after_mount=serde_json::Value::Null;
        let mut hover_after_move=serde_json::Value::Null;
        cx.update_window(window.into(),|_,window,cx| {
            let child=source("Fluent child",window); let container=source("Fluent source viewport",window);
            assert!(child.borrow().parent.ptr_eq(&Rc::downgrade(&container)));
            let position=child.borrow().bounds.unwrap().center();
            let action=verdant_action_control::ActionControlHandle{owner:child.clone()};
            assert!(child.borrow().native_hitbox.as_ref().unwrap().is_hovered_at(window.mouse_position(),window),"the first native frame already has a pointer over this root");
            hover_after_mount=action.state("hovered").unwrap();
            window.dispatch_event(gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent{position,..Default::default()}),cx);
            hover_after_move=action.state("hovered").unwrap();
            // Real leave/re-enter must work independently of the initial host
            // observation, which can precede authored onMounted delivery.
            window.dispatch_event(gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent{position:gpui::point(px(330.),px(230.)),..Default::default()}),cx);
            assert_eq!(action.state("hovered").unwrap(),serde_json::json!(false));
            window.dispatch_event(gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent{position,..Default::default()}),cx);
            assert_eq!(action.state("hovered").unwrap(),serde_json::json!(true),"a native outside-to-inside transition reaches the source");
            assert!(action.take_failures().is_empty());
            window.dispatch_event(gpui::PlatformInput::MouseDown(gpui::MouseDownEvent{position,button:gpui::MouseButton::Left,..Default::default()}),cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent{position,button:gpui::MouseButton::Left,..Default::default()}),cx);
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx| {
            assert_eq!(hover_after_mount,serde_json::json!(true),"a pre-mount native hover fact must reach the mounted source");
            assert_eq!(hover_after_move,serde_json::json!(true),"native move reaches the nested source hover route");
            assert_eq!(host.clicks,1,"source viewport must not intercept its native content");
            let child=verdant_action_control::ActionControlHandle{owner:source("Fluent child",window)};
            assert_eq!(child.state("hovered").unwrap(),serde_json::json!(true));
            child.call("focusSelf",vec![],window,cx).unwrap();
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"space");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_|assert_eq!(host.clicks,2,"child keyboard action retains ownership")).unwrap();
        cx.simulate_keystrokes(window.into(),"pagedown");crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {
            let owner=source("Fluent source viewport",window);let state=owner.borrow();
            assert_ne!(state.modules.scroll.external_engine,0);
            let engine=state.modules.scroll.handle.as_ref().unwrap();
            assert!(engine.max_offset().y>px(1000.));assert!(engine.offset().y<px(-100.));
            assert!(source("Fluent child",window).borrow().bounds.unwrap().origin.y<px(0.),"actual native content moved with the engine");
        }).unwrap();
    }

    struct RevealHarness { index:usize }
    impl Render for RevealHarness {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
            viewport("native-reveal-scroll","Revealed source viewport","vertical").size_full().flex().flex_col()
                .reveal_item(self.index,format!("lane:{}",self.index))
                .children((0..20).map(|i|gpui::div().h(px(40.)).flex_none().child(format!("Lane {i}"))))
        }
    }
    #[gpui::test]
    fn native_fluent_viewport_reveals_domain_selected_content_using_its_actual_measured_engine(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(280.),px(160.)),|_,_|RevealHarness{index:0});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,cx| {host.index=15;cx.notify();}).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {
            let owner=source("Revealed source viewport",window);let state=owner.borrow();
            let engine=state.modules.scroll.handle.as_ref().unwrap();
            assert!(engine.offset().y<px(-400.),"source nearest reveal moves the actual content engine");
            let item=engine.bounds_for_item(16).unwrap();let viewport=engine.bounds();let offset=engine.offset();
            assert!(item.top()+offset.y>=viewport.top()-px(1.));
            assert!(item.bottom()+offset.y<=viewport.bottom()+px(1.));
        }).unwrap();
        window.update(cx,|host,_,cx| {host.index=0;cx.notify();}).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {
            let owner=source("Revealed source viewport",window);let state=owner.borrow();
            assert_eq!(state.modules.scroll.handle.as_ref().unwrap().offset().y,px(0.));
        }).unwrap();
    }

    #[gpui::test]
    fn native_scroll_region_moves_the_real_viewport_from_child_page_keys(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(320.), px(240.)), |_, _| Harness);
        window.update(cx, |_, window, _| window.activate_window()).unwrap(); crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, cx| {
            let child = verdant_action_control::ActionControlHandle { owner: source("Scroll child", window) };
            child.call("focusSelf", vec![], window, cx).unwrap();
            let owner = source("Native document", window); let owner = owner.borrow();
            let scroll = owner.modules.scroll.handle.as_ref().expect("the source owns a native viewport");
            assert!(scroll.max_offset().y > px(1000.), "native slot content must contribute real overflow");
            assert_eq!(scroll.offset().y, px(0.));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "pagedown"); crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, _| {
            let handle = ScrollRegionHandle { owner: source("Native document", window) };
            assert!(handle.owner.borrow().modules.scroll.handle.as_ref().unwrap().offset().y < px(-100.));
            assert!(handle.state("position").unwrap().as_f64().unwrap() > 0.);
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "end"); crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, _| {
            let handle = ScrollRegionHandle { owner: source("Native document", window) };
            let owner = handle.owner.borrow(); let scroll = owner.modules.scroll.handle.as_ref().unwrap();
            assert!((scroll.offset().y + scroll.max_offset().y).as_f32().abs() < 1.);
            drop(owner); assert_eq!(handle.state("atEnd").unwrap(), serde_json::json!(true));
        }).unwrap();
    }
}

#[cfg(test)]
mod input_boundary_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, px, size};
    use verdant_scroll_region::native::gpui_native_owner::{all_owners,resolved};
    fn source(name:&str,window:&Window)->OwnerRef {
        all_owners().into_iter().find(|owner| {let owner=owner.borrow();owner.mounted && owner.window_id==Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|value|resolved(value).text()==name)}).unwrap()
    }
    struct Keys { consume:bool }
    impl Render for Keys {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
            let consume=self.consume;
            let draw:Draw=Rc::new(move|_,_,_,_,_|gpui::div().w_full().flex().flex_col()
                .child(crate::proto_controls::action("scroll-key-child","Key child").h(px(36.)).flex_none().child("Child")
                    .on_key_down(move|event,window,_| {if consume && event.keystroke.key=="pagedown" {window.prevent_default();}}))
                .children((0..40).map(|index|gpui::div().h(px(40.)).flex_none().child(format!("Row {index}"))))
                .into_any_element());
            region("scroll-input-policy",serde_json::json!({"label":"Key viewport"}),draw)
        }
    }
    #[gpui::test]
    fn native_scroll_fallback_keeps_modifiers_and_consumed_child_keys_out_of_the_physical_engine(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(320.),px(224.)),|_,_|Keys{consume:true});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx| {verdant_action_control::ActionControlHandle{owner:source("Key child",window)}.call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        for key in ["ctrl-pagedown","cmd-pagedown","alt-end","pagedown"] {
            cx.simulate_keystrokes(window.into(),key);crate::native_test_support::frames(window,cx);
            window.update(cx,|_,window,_| {let owner=source("Key viewport",window);let owner=owner.borrow();
                let engine=owner.modules.scroll.handle.as_ref().unwrap();assert!(engine.max_offset().y>px(1000.));
                assert_eq!(engine.offset().y,px(0.),"modified or child-consumed key must not scroll: {key}");}).unwrap();
        }
        window.update(cx,|host,_,cx| {host.consume=false;cx.notify();}).unwrap();crate::native_test_support::frames(window,cx);
        cx.simulate_keystrokes(window.into(),"pagedown");crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {assert!(source("Key viewport",window).borrow().modules.scroll.handle.as_ref().unwrap().offset().y<px(-100.),"unconsumed child page key still reaches the real source engine");}).unwrap();
    }
    struct TabDocument;
    impl Render for TabDocument {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
            let draw:Draw=Rc::new(|_,_,_,_,_|gpui::div().child(crate::proto_controls::action("tab-document-child","Tab document child").h(px(36.)).child("Child")).into_any_element());
            gpui::div().size_full().flex().flex_col()
                .child(gpui::div().h(px(150.)).child(region("tab-document",serde_json::json!({"label":"Ordinary document"}),draw)))
                .child(crate::proto_controls::action("after-document","After document").h(px(36.)).child("After"))
        }
    }
    #[gpui::test]
    fn native_ordinary_scroll_region_allows_tab_to_leave_its_document(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(320.),px(240.)),|_,_|TabDocument);
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx| {let source=ScrollRegionHandle{owner:source("Ordinary document",window)};
            assert_eq!(source.state("scopeActive").unwrap(),serde_json::json!(false));source.call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        cx.simulate_keystrokes(window.into(),"tab");crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {assert_eq!(verdant_action_control::ActionControlHandle{owner:source("Tab document child",window)}.state("focused").unwrap(),serde_json::json!(true));}).unwrap();
        cx.simulate_keystrokes(window.into(),"tab");crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,_| {assert_eq!(verdant_action_control::ActionControlHandle{owner:source("After document",window)}.state("focused").unwrap(),serde_json::json!(true),"ordinary documents must not trap Tab");}).unwrap();
    }
}
