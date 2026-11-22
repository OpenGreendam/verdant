//! Compiled sustained keys with native drawing and an audio-domain callback.
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use gpui::{AnyElement, App, AppContext, Context, Entity, Global, IntoElement, Render, RenderOnce, SharedString, Subscription, Window, WindowId};
use serde_json::{Value,json};
use verdant_performance_key::{PerformanceKey,PerformanceKeyHandle,native::gpui_native_owner::NativeSnapshot};

type Callback=Rc<dyn Fn(bool,&mut App)>;
type Draw=Rc<dyn Fn(bool)->AnyElement>;
#[derive(Clone)]
struct Entry { handle:PerformanceKeyHandle, delivery:Rc<RefCell<Delivery>> }
struct Delivery { callback:Callback, revision:String, pressed:bool, live:bool }
fn deliver(entry:&Entry,pressed:bool,cx:&mut App) {
    let callback={let mut delivery=entry.delivery.borrow_mut();
        if delivery.pressed==pressed{return;}delivery.pressed=pressed;delivery.callback.clone()};
    callback(pressed,cx);
}
#[derive(Default)]
struct Keys(HashMap<(WindowId,String,u8),Entry>);
impl Global for Keys {}

fn drain(entry:&Entry,cx:&mut App) {
    let failures=entry.handle.take_failures();
    let events=entry.handle.take_events();
    if !entry.delivery.borrow().live || !failures.is_empty() { deliver(entry,false,cx);return; }
    let revision=entry.delivery.borrow().revision.clone();
    if events.iter().any(|(name,payload)|name=="trigger"&&payload.get("revision").and_then(Value::as_str)==Some(revision.as_str())) {
        let callback=entry.delivery.borrow().callback.clone();callback(true,cx);callback(false,cx);
    }
    if events.iter().any(|(name,_)|name=="pressedChange") {
        let pressed=entry.handle.state("pressed").ok().and_then(|value|value.as_bool()).unwrap_or(false);
        // Collapsing a down/up batch prevents an already-released physical key
        // from starting a deferred audio note. Props settlement drains against
        // the old delivery target before exposing a replacement target.
        deliver(entry,pressed,cx);
    }
}

/// Transport the physical key string to each authored key. Mapping, modifiers
/// admission by the parent, contact state and repeat suppression stay separate.
pub(crate) fn route_key(scope:&str,key:&str,down:bool,window:&mut Window,cx:&mut App)->bool {
    let entries:Vec<_>=cx.try_global::<Keys>().map(|keys|keys.0.iter().filter(|((win,id,_),_)|
        *win==window.window_handle().window_id()&&id==scope).map(|(_,entry)|entry.clone()).collect()).unwrap_or_default();
    let mut consumed=false;
    for entry in entries {
        let accepted=entry.handle.call(if down {"keyDown"}else{"keyUp"},vec![key.into()],window,cx)
            .ok().and_then(|v|v.as_bool()).unwrap_or(false);
        consumed|=accepted;
        // Queue consumer delivery outside the active NativePluginWindow borrow;
        // a release before delivery is collapsed to the actual authored state.
        if accepted { let entry=entry.clone();cx.defer(move |cx|drain(&entry,cx)); }
    }
    consumed
}

#[derive(IntoElement)]
pub(crate) struct Key { pub scope:String,pub note:u8,pub label:String,pub revision:String,pub disabled:bool,
    draw:Draw,callback:Callback,momentary:bool }
pub(crate) fn key(scope:impl Into<String>,note:u8,label:impl Into<String>,revision:impl Into<String>,disabled:bool,
    draw:impl Fn(bool)->AnyElement+'static,callback:impl Fn(bool,&mut App)+'static)->Key {
    Key{scope:scope.into(),note,label:label.into(),revision:revision.into(),disabled,draw:Rc::new(draw),callback:Rc::new(callback),momentary:false}
}
impl Key { pub(crate) fn momentary(mut self)->Self { self.momentary=true;self } }
struct Drawing { handle:PerformanceKeyHandle,draw:Draw }
impl Render for Drawing {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        let pressed=self.handle.state("pressed").ok().and_then(|v|v.as_bool()).unwrap_or(false);
        crate::proto_parent::slot(&self.handle.owner,(self.draw)(pressed))
    }
}
struct Mount { root:Entity<PerformanceKey>,drawing:Entity<Drawing>,entry:Entry,props:Value,_changes:Subscription,
    registration:(WindowId,String,u8),app:gpui::AsyncApp }
impl Drop for Mount {
    fn drop(&mut self) {
        let registration=self.registration.clone();let entry=self.entry.clone();
        // Retire input synchronously, before a queued observer can turn an
        // already detached key back on. Release belongs to this delivery even
        // if a replacement key registered under the same note in the meantime.
        entry.delivery.borrow_mut().live=false;
        self.app.spawn(async move |cx| { let _=cx.update(|cx| {
            let same=cx.try_global::<Keys>().and_then(|keys|keys.0.get(&registration))
                .is_some_and(|current|Rc::ptr_eq(&current.handle.owner,&entry.handle.owner));
            if same { cx.global_mut::<Keys>().0.remove(&registration); }
            deliver(&entry,false,cx);
        }); }).detach();
    }
}
impl RenderOnce for Key {
    fn render(self,window:&mut Window,cx:&mut App)->impl IntoElement {
        let parent=crate::proto_parent::current();let initial_parent=parent.clone();
        let props=json!({"note":self.note,"label":self.label,"revision":self.revision,"disabled":self.disabled,"momentary":self.momentary});
        let initial=props.clone();let draw=self.draw.clone();let callback=self.callback.clone();
        let registration=(window.window_handle().window_id(),self.scope.clone(),self.note);
        let initial_registration=registration.clone();
        let key=SharedString::from(format!("proto-performance-key-{}-{}",self.scope,self.note));
        let mount=window.use_keyed_state(key,cx,move |window,cx| {
            let root=PerformanceKey::create(initial.clone(),initial_parent,window,cx)
                .unwrap_or_else(|e|panic!("compiled performance key setup failed: {e:?}"));
            let entry=Entry{handle:root.read(cx).handle(),delivery:Rc::new(RefCell::new(Delivery{callback,revision:initial["revision"].as_str().unwrap_or_default().into(),pressed:false,live:true}))};
            let drawing=cx.new(|_|Drawing{handle:entry.handle.clone(),draw});
            root.update(cx,|root,cx|root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())],cx))
                .unwrap_or_else(|e|panic!("compiled performance key drawing failed: {e:?}"));
            let changes=cx.observe(&root,|mount:&mut Mount,_,cx| {
                mount.drawing.update(cx,|_,cx|cx.notify());let entry=mount.entry.clone();cx.defer(move |cx|drain(&entry,cx));
            });
            Mount{root,drawing,entry,props:initial,_changes:changes,registration:initial_registration,app:cx.to_async()}
        });
        mount.update(cx,|mount,cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner,&parent) {
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled performance key parent");
            }
            if mount.props!=props {
                let _=mount.root.update(cx,|root,cx|root.set_props(props.clone(),window,cx));
                drain(&mount.entry,cx);
                mount.props=props;
            }
            let next_revision=mount.props["revision"].as_str().unwrap_or_default().to_string();
            if mount.entry.delivery.borrow().revision!=next_revision { deliver(&mount.entry,false,cx); }
            {let mut delivery=mount.entry.delivery.borrow_mut();delivery.callback=self.callback;delivery.revision=next_revision;}
            mount.drawing.update(cx,|drawing,cx|{drawing.draw=self.draw;cx.notify();});
            if cx.try_global::<Keys>().is_none(){cx.set_global(Keys::default());}
            let previous=cx.global_mut::<Keys>().0.insert(registration,mount.entry.clone());
            if let Some(previous)=previous.filter(|previous|!Rc::ptr_eq(&previous.handle.owner,&mount.entry.handle.owner)) {
                previous.delivery.borrow_mut().live=false;
                // This runs before input can start the replacement; a later
                // Drop release is idempotent because delivery records the off.
                cx.defer(move |cx|deliver(&previous,false,cx));
            }
            mount.root.clone()
        })
    }
}
