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

/// Retire every authored contact in this window's registered keyboard scope.
///
/// Window deactivation and hiding a keyboard are host lifecycle facts. Cancel
/// each source synchronously so an older queued delivery observes its released
/// state; deliver the resulting audio-domain changes after the host view lease.
pub(crate) fn cancel_scope(scope:&str,window:&mut Window,cx:&mut App) {
    let entries:Vec<_>=cx.try_global::<Keys>().map(|keys|keys.0.iter().filter(|((win,id,_),_)|
        *win==window.window_handle().window_id()&&id==scope).map(|(_,entry)|entry.clone()).collect()).unwrap_or_default();
    for entry in entries {
        let _=entry.handle.call("cancel",vec![],window,cx);
        cx.defer(move |cx|drain(&entry,cx));
    }
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

#[cfg(test)]
mod native_tests {
    use super::*;
    use gpui::{div, px, size, FocusHandle, InteractiveElement, KeyDownEvent, KeyUpEvent,
        Keystroke, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, PlatformInput, Styled,
        StatefulInteractiveElement, TestAppContext, WindowHandle};
    use verdant_performance_key::native::gpui_native_owner::{drive_owners,dispatch_accessibility_action};

    struct Harness {
        visible: bool, revision: String, disabled: bool, momentary: bool,
        focus: FocusHandle, outside: FocusHandle, delivered: Rc<RefCell<Vec<(String,bool)>>>,
    }
    impl Render for Harness {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
            let mut root=div().id("performance-test").size_full().track_focus(&self.focus)
                .on_key_down(|event:&KeyDownEvent,window,cx|{route_key("native-performance",&event.keystroke.key,true,window,cx);})
                .on_key_up(|event:&KeyUpEvent,window,cx|{route_key("native-performance",&event.keystroke.key,false,window,cx);});
            if self.visible {
                let delivered=self.delivered.clone();let revision=self.revision.clone();
                let mut authored=key("native-performance",60,"C4",revision.clone(),self.disabled,
                    |_|div().size_full().into_any_element(),
                    move |pressed,_|delivered.borrow_mut().push((revision.clone(),pressed)));
                if self.momentary {authored=authored.momentary();}
                root=root.child(div().w(px(120.)).h(px(90.)).child(authored));
            }
            root.child(div().id("outside-performance").track_focus(&self.outside).w(px(100.)).h(px(30.)))
        }
    }
    fn open(cx:&mut TestAppContext,momentary:bool)->WindowHandle<Harness> {
        let host=cx.open_window(size(px(260.),px(180.)),|_,cx|Harness {
            visible:true,revision:"instrument:one".into(),disabled:false,momentary,
            focus:cx.focus_handle(),outside:cx.focus_handle(),delivered:Rc::default(),
        });
        host.update(cx,|host,window,cx|{window.activate_window();window.focus(&host.focus,cx);}).unwrap();
        crate::native_test_support::frames(host,cx);host
    }
    fn physical_key(down:bool,held:bool,window:&mut Window,cx:&mut App) {
        let keystroke=Keystroke::parse("a").unwrap();
        window.dispatch_event(if down {PlatformInput::KeyDown(KeyDownEvent {keystroke,is_held:held,prefer_character_input:false})}
            else {PlatformInput::KeyUp(KeyUpEvent {keystroke})},cx);
    }
    fn send_key(host:WindowHandle<Harness>,down:bool,held:bool,cx:&mut TestAppContext) {
        // Native dispatch re-enters the parent view for its key listener. Do
        // not hold a typed Harness lease while injecting that same input.
        cx.update_window(host.into(),|_,window,cx|physical_key(down,held,window,cx)).unwrap();
    }
    fn send_key_pair(host:WindowHandle<Harness>,cx:&mut TestAppContext) {
        cx.update_window(host.into(),|_,window,cx|{physical_key(true,false,window,cx);physical_key(false,false,window,cx);}).unwrap();
    }
    fn entry(window:&Window,cx:&App)->Entry {
        cx.global::<Keys>().0.get(&(window.window_handle().window_id(),"native-performance".into(),60)).unwrap().clone()
    }
    fn click_sample(down:bool,window:&mut Window,cx:&mut App) {
        let position=entry(window,cx).handle.owner.borrow().bounds.expect("laid-out authored key").center();
        window.dispatch_event(if down {PlatformInput::MouseDown(MouseDownEvent {position,button:MouseButton::Left,..Default::default()})}
            else {PlatformInput::MouseUp(MouseUpEvent {position,button:MouseButton::Left,..Default::default()})},cx);
    }
    #[gpui::test]
    fn native_held_key_repeats_and_pointer_overlap_release_only_the_last_contact(cx:&mut TestAppContext) {
        let host=open(cx,false);
        send_key(host,true,false,cx);send_key(host,true,true,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|{
            assert_eq!(&*host.delivered.borrow(),&[("instrument:one".into(),true)]);
            click_sample(true,window,cx);
        }).unwrap();
        send_key(host,false,false,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|{
            assert_eq!(host.delivered.borrow().len(),1,"typing release does not stop a physical pointer hold");
            assert_eq!(entry(window,cx).handle.state("pressed").unwrap(),Value::Bool(true));
            click_sample(false,window,cx);
        }).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,_|assert_eq!(&*host.delivered.borrow(),&[("instrument:one".into(),true),("instrument:one".into(),false)])).unwrap();
    }
    #[gpui::test]
    fn native_revision_change_and_detach_release_the_held_target_without_late_note_on(cx:&mut TestAppContext) {
        let host=open(cx,false);
        send_key(host,true,false,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,cx|{host.revision="instrument:two".into();cx.notify();}).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,_|{
            assert_eq!(&*host.delivered.borrow(),&[("instrument:one".into(),true),("instrument:one".into(),false)]);
        }).unwrap();
        send_key(host,false,false,cx);send_key(host,true,false,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,cx|{host.visible=false;cx.notify();}).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|{
            assert_eq!(&*host.delivered.borrow(),&[("instrument:one".into(),true),("instrument:one".into(),false),
                ("instrument:two".into(),true),("instrument:two".into(),false)]);
            assert!(!route_key("native-performance","a",true,window,cx));
            host.visible=true;cx.notify();
        }).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|window.focus(&host.focus,cx)).unwrap();
        send_key_pair(host,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,_|assert_eq!(host.delivered.borrow().len(),4,"a down/up batch retired before delivery never starts audio")).unwrap();
    }
    #[gpui::test]
    fn native_focus_loss_and_disable_release_sustained_keys(cx:&mut TestAppContext) {
        for disable in [false,true] {
            let host=open(cx,false);
            host.update(cx,|_,window,cx|click_sample(true,window,cx)).unwrap();
            crate::native_test_support::frames(host,cx);
            host.update(cx,|host,window,cx|{
                assert_eq!(host.delivered.borrow().last(),Some(&("instrument:one".into(),true)));
                assert_eq!(entry(window,cx).handle.state("focused").unwrap(),Value::Bool(true),"a physical key hold owns its authored focus");
                assert!(entry(window,cx).handle.owner.borrow().modules.focus.native.as_ref().is_some_and(|focus|focus.is_focused(window)),"the held key retains the actual native leaf focus");
                if disable {host.disabled=true;cx.notify();} else {window.focus(&host.outside,cx);}
            }).unwrap();
            crate::native_test_support::frames(host,cx);
            host.update(cx,|host,window,cx|{
                let key=entry(window,cx);
                if !disable {assert!(host.outside.is_focused(window));}
                let physical_focus=key.handle.owner.borrow().modules.focus.native.as_ref().map(|focus|focus.is_focused(window));
                assert_eq!(key.handle.state("focused").unwrap(),Value::Bool(false),"key source focus after disable={disable}; physical_focus={:?}; failures={:?}",
                    physical_focus,key.handle.take_failures());
                assert_eq!(key.handle.state("pressed").unwrap(),Value::Bool(false),"source releases all contacts before its audio delivery");
                assert_eq!(host.delivered.borrow().last(),Some(&("instrument:one".into(),false)));
                click_sample(false,window,cx);
                assert_eq!(host.delivered.borrow().len(),2);
            }).unwrap();
        }
    }
    #[gpui::test]
    fn native_detached_owner_cannot_release_a_replacement_key_with_the_same_registration(cx:&mut TestAppContext) {
        let host=open(cx,false);
        send_key(host,true,false,cx);
        let retired=host.update(cx,|_,window,cx|entry(window,cx)).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,cx|{host.visible=false;cx.notify();}).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,cx|{host.visible=true;cx.notify();}).unwrap();
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|window.focus(&host.focus,cx)).unwrap();
        send_key(host,true,false,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,window,cx|{
            let replacement=entry(window,cx);
            assert!(!Rc::ptr_eq(&retired.handle.owner,&replacement.handle.owner));
            assert_eq!(&*host.delivered.borrow(),&[("instrument:one".into(),true),("instrument:one".into(),false),("instrument:one".into(),true)]);
            // A real retained native handle can outlive its physical view.
            let _=retired.handle.call("cancel",vec![],window,cx);
            drain(&retired,cx);
            assert_eq!(replacement.handle.state("pressed").unwrap(),Value::Bool(true));
            assert_eq!(host.delivered.borrow().len(),3,"old cleanup cannot emit a second release after the replacement starts");
        }).unwrap();
        send_key(host,false,false,cx);
        crate::native_test_support::frames(host,cx);
        host.update(cx,|host,_,_|assert_eq!(host.delivered.borrow().last(),Some(&("instrument:one".into(),false)))).unwrap();
    }
    #[gpui::test]
    fn native_accessibility_activation_obeys_sustain_or_momentary_policy(cx:&mut TestAppContext) {
        for momentary in [false,true] {
            let host=open(cx,momentary);
            host.update(cx,|_,window,cx|{
                let source=entry(window,cx);dispatch_accessibility_action(&source.handle.owner,"activate",None);drive_owners(window,cx);
            }).unwrap();
            crate::native_test_support::frames(host,cx);
            host.update(cx,|host,window,cx|{
                assert_eq!(host.delivered.borrow().len(),if momentary{2}else{1});
                assert_eq!(entry(window,cx).handle.state("pressed").unwrap(),Value::Bool(!momentary));
                host.disabled=true;cx.notify();
            }).unwrap();
            crate::native_test_support::frames(host,cx);
            host.update(cx,|host,window,cx|{
                let source=entry(window,cx);dispatch_accessibility_action(&source.handle.owner,"activate",None);drive_owners(window,cx);
                assert_eq!(host.delivered.borrow().len(),2);
            }).unwrap();
        }
    }
}
