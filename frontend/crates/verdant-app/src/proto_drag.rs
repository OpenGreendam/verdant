//! Native drag transport for a compiled, single-consumer transfer source.
//! Payload transport and the drag image are GPUI mechanisms; lifecycle,
//! admission, cancellation, focus and accessible activation belong to source.
use std::{cell::{Cell,RefCell},rc::Rc};
use gpui::{AnyElement,App,AppContext,Context,Entity,IntoElement,Render,RenderOnce,SharedString,Subscription,Window};
use serde_json::{Value,json};
use verdant_drag_source::{DragSource,DragSourceHandle,native::gpui_native_owner::NativeSnapshot};

type Choose=Rc<dyn Fn(&mut Window,&mut App)>;
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
                let callback=self.choose.borrow().clone();callback(window,cx);
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
    draw:impl Fn(Input)->AnyElement+'static,choose:impl Fn(&mut Window,&mut App)+'static)->Source {
    Source{key:key.into(),label:label.into(),revision:revision.into(),disabled,draw:Rc::new(draw),choose:Rc::new(choose)}
}
struct Drawing { input:Input,draw:Draw }
impl Render for Drawing {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        crate::proto_parent::slot(&self.input.handle.owner,(self.draw)(self.input.clone()))
    }
}
struct Mount {root:Entity<DragSource>,drawing:Entity<Drawing>,input:Input,props:Value,_changes:Subscription}
impl Drop for Mount {fn drop(&mut self){self.input.live.set(false);}}
impl RenderOnce for Source {
    fn render(self,window:&mut Window,cx:&mut App)->impl IntoElement {
        let parent=crate::proto_parent::current();let initial_parent=parent.clone();
        let props=json!({"label":self.label,"revision":self.revision,"disabled":self.disabled});
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
            Mount{root,drawing,input,props:initial,_changes:changes}
        });
        mount.update(cx,|mount,cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner,&parent) {
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled drag source parent");
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
