//! Native drag transport for a compiled, single-consumer transfer source.
//! Payload transport and the drag image are GPUI mechanisms; lifecycle,
//! admission, cancellation, focus and accessible activation belong to source.
use std::{cell::{Cell,RefCell},rc::Rc};
use gpui::{AnyElement,App,AppContext,Context,Entity,IntoElement,Render,RenderOnce,SharedString,Subscription,Window};
use serde_json::{Value,json};
use verdant_drag_source::{DragSource,DragSourceHandle,native::gpui_native_owner::NativeSnapshot};

type Choose=Rc<dyn Fn(gpui::Point<gpui::Pixels>,&mut Window,&mut App)>;
type Draw=Rc<dyn Fn(Input)->AnyElement>;
#[derive(Clone)]
pub(crate) struct Input {
    handle:DragSourceHandle,
    live:Rc<Cell<bool>>,
    lease:Rc<Cell<f64>>,
    revision:Rc<RefCell<String>>,
    choose:Rc<RefCell<Choose>>,
}
#[derive(Clone)]
pub(crate) struct Lease { input:Input,token:f64,basis:String,window:gpui::AnyWindowHandle }
impl Input {
    fn drain(&self,window:&mut Window,cx:&mut App) {
        let failures=self.handle.take_failures();
        let events=self.handle.take_events();
        let mut cancel=!failures.is_empty();
        for (name,payload) in events {
            if name=="cancelled"&&payload.get("lease").and_then(Value::as_f64)==Some(self.lease.get()) { cancel=true; }
            if name=="choose"&&self.live.get()&&payload.get("revision").and_then(Value::as_str)==Some(self.revision.borrow().as_str()) {
                let anchor={
                    let owner=self.handle.owner.borrow();
                    (owner.present && owner.mounted && !owner.disposed
                        && owner.window_id==Some(window.window_handle().window_id())
                        && owner.props.get("disabled").and_then(Value::as_bool)!=Some(true)
                        && owner.props.get("revision").and_then(Value::as_str)==payload.get("revision").and_then(Value::as_str))
                        .then(||owner.bounds.map(|bounds|bounds.bottom_left())).flatten()
                };
                if let Some(anchor)=anchor {
                    let callback=self.choose.borrow().clone();callback(anchor,window,cx);
                }
            }
        }
        if cancel&&self.lease.replace(0.)!=0. {
            // A native drag image clears this lease synchronously on Drop.
            // Hence late source cancellation cannot stop a later foreign drag.
            cx.stop_active_drag(window);
        }
    }
    pub(crate) fn begin(&self,window:&mut Window,cx:&mut App)->Option<Lease> {
        if !self.live.get()||self.lease.get()!=0. { return None; }
        let value=self.handle.call("begin",vec![],window,cx).ok()?.as_f64()?;
        self.drain(window,cx);
        if !value.is_finite()||value<=0.||!self.live.get(){return None;}
        self.lease.set(value);
        Some(Lease{input:self.clone(),token:value,basis:self.revision.borrow().clone(),window:window.window_handle()})
    }
}
impl Lease {
    pub(crate) fn accept(&self,window:&mut Window,cx:&mut App)->bool {
        if !self.input.live.get()||self.input.lease.get()!=self.token||*self.input.revision.borrow()!=self.basis {return false;}
        let accepted=self.input.handle.call("accept",vec![self.token.into(),self.basis.clone().into()],window,cx)
            .ok().and_then(|v|v.as_bool()).unwrap_or(false);
        if accepted {self.input.lease.set(0.);}
        self.input.drain(window,cx);
        accepted
    }
    /// Native transport has ended, including a drop outside all target areas.
    /// Clear its identity synchronously and retire the source outside a GPUI
    /// entity destructor; the token still rejects a replacement acquisition.
    pub(crate) fn end(&self,app:&gpui::AsyncApp) {
        if self.input.lease.get()!=self.token { return; }
        self.input.lease.set(0.);
        let lease=self.clone();
        app.spawn(async move |cx| { let _=cx.update(|cx| {
            let _=lease.window.update(cx,|_,window,cx| {
                let _=lease.input.handle.call("cancel",vec![lease.token.into()],window,cx);
                lease.input.drain(window,cx);
            });
        }); }).detach();
    }
}

#[derive(IntoElement)]
pub(crate) struct Source {key:String,label:String,revision:String,disabled:bool,draw:Draw,choose:Choose}
pub(crate) fn source(key:impl Into<String>,label:impl Into<String>,revision:impl Into<String>,disabled:bool,
    draw:impl Fn(Input)->AnyElement+'static,choose:impl Fn(gpui::Point<gpui::Pixels>,&mut Window,&mut App)+'static)->Source {
    Source{key:key.into(),label:label.into(),revision:revision.into(),disabled,draw:Rc::new(draw),choose:Rc::new(choose)}
}
struct Drawing { input:Input,draw:Draw }
impl Render for Drawing {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement { crate::proto_parent::slot(&self.input.handle.owner,(self.draw)(self.input.clone())) }
}
struct Mount {root:Entity<DragSource>,drawing:Entity<Drawing>,input:Input,props:Value,_changes:Subscription,window:gpui::AnyWindowHandle,app:gpui::AsyncApp}
impl Drop for Mount {
    fn drop(&mut self){
        self.input.live.set(false);
        let token=self.input.lease.get();if token==0.{return;}
        let input=self.input.clone();let window=self.window;
        self.app.spawn(async move|cx|{let _=cx.update(|cx|{
            if input.lease.get()!=token{return;}
            // The native drag image clears its lease immediately on Drop,
            // including a completed transfer followed by another native drag.
            input.lease.set(0.);let _=window.update(cx,|_,window,cx|{cx.stop_active_drag(window);});
        });}).detach();
    }
}
impl RenderOnce for Source {
    fn render(self,window:&mut Window,cx:&mut App)->impl IntoElement {
        let props=json!({"label":self.label,"revision":self.revision,"disabled":self.disabled});
        let parent=crate::proto_parent::current();let initial_parent=parent.clone();
        let initial=props.clone();let draw=self.draw.clone();let choose=self.choose.clone();
        let key=SharedString::from(format!("proto-transfer-{}",self.key));
        let mount=window.use_keyed_state(key,cx,move |window,cx| {
            let root=DragSource::create(initial.clone(),initial_parent,window,cx)
                .unwrap_or_else(|e|panic!("compiled transfer source setup failed: {e:?}"));
            let input=Input{handle:root.read(cx).handle(),live:Rc::new(Cell::new(true)),lease:Rc::new(Cell::new(0.)),
                revision:Rc::new(RefCell::new(initial["revision"].as_str().unwrap_or_default().into())),choose:Rc::new(RefCell::new(choose))};
            let drawing=cx.new(|_|Drawing{input:input.clone(),draw});
            root.update(cx,|root,cx|root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())],cx))
                .unwrap_or_else(|e|panic!("compiled transfer source slot failed: {e:?}"));
            let handle=window.window_handle();
            let changes=cx.observe(&root,move |mount:&mut Mount,_,cx| {
                let input=mount.input.clone();cx.defer(move |cx| {let _=handle.update(cx,|_,window,cx|input.drain(window,cx));});
            });
            Mount{root,drawing,input,props:initial,_changes:changes,window:handle,app:cx.to_async()}
        });
        mount.update(cx,|mount,cx| {
            if crate::proto_parent::changed(&mount.input.handle.owner,&parent) {
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled transfer parent");
            }
            if mount.props!=props {
                let _=mount.root.update(cx,|root,cx|root.set_props(props.clone(),window,cx));
                mount.input.drain(window,cx);mount.props=props;
            }
            *mount.input.revision.borrow_mut()=mount.props["revision"].as_str().unwrap_or_default().into();
            *mount.input.choose.borrow_mut()=self.choose;
            mount.drawing.update(cx,|drawing,cx|{drawing.draw=self.draw;cx.notify();});
            mount.root.clone()
        })
    }
}


#[cfg(test)]
mod chooser_anchor_tests {
    use super::*;
    use gpui::{prelude::*, point, px, size, MouseMoveEvent, PlatformInput, Point, Pixels, TestAppContext};
    use verdant_drag_source::native::gpui_native_owner::{all_owners, dispatch_accessibility_action, drive_owners};

    struct Harness { anchors: Rc<RefCell<Vec<Point<Pixels>>>> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let anchors = self.anchors.clone();
            gpui::div().size_full().p(px(16.)).child(
                gpui::div().w(px(120.)).h(px(32.)).child(source(
                    "chooser-anchor", "Chooser anchor", "revision:1", false,
                    |_| gpui::div().size_full().into_any_element(),
                    move |anchor, _, _| anchors.borrow_mut().push(anchor),
                )),
            )
        }
    }
    fn handle(window: &Window) -> DragSourceHandle {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.present && !owner.disposed
                && owner.window_id == Some(window.window_handle().window_id())
                && owner.props.get("label").and_then(Value::as_str) == Some("Chooser anchor")
                && owner.declared_events.contains("choose")
        }).expect("the actual mounted compiled transfer source");
        DragSourceHandle { owner }
    }
    #[gpui::test]
    fn keyboard_and_assistive_chooser_anchor_follow_the_live_source_not_the_mouse(cx: &mut TestAppContext) {
        let anchors = Rc::new(RefCell::new(Vec::new()));
        let observed = anchors.clone();
        let window = cx.open_window(size(px(400.), px(240.)), move |_, _| Harness { anchors });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        let expected = window.update(cx, |_, window, cx| {
            let source = handle(window);
            let bounds = source.owner.borrow().bounds.expect("source has physical bounds");
            let focus = source.owner.borrow().modules.focus.native.clone().expect("compiled focus handle");
            let mouse = point(px(360.), px(210.));
            assert!(!bounds.contains(&mouse));
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: mouse, ..Default::default() }), cx);
            window.focus(&focus, cx);
            bounds.bottom_left()
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        assert_eq!(*observed.borrow(), vec![expected], "keyboard activation anchors the chooser at its current source");
        window.update(cx, |_, window, cx| {
            let source = handle(window);
            dispatch_accessibility_action(&source.owner, "activate", None);
            drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        assert_eq!(*observed.borrow(), vec![expected, expected], "assistive activation uses the same live semantic anchor");
    }
}
