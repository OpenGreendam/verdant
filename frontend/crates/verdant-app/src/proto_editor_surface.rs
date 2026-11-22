//! Stateful compiler-owned editing surfaces; native code only maps coordinates
//! and applies the admitted music-domain operations. The native contact ledger
//! protects capture cleanup from a detached/old surface in another window.
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use gpui::{AnyElement, App, AppContext, Context, Entity, EntityId, Global, IntoElement,
    Render, RenderOnce, SharedString, Subscription, WeakEntity, Window};
use serde_json::{Value, json};
use verdant_editor_surface::{EditorSurface, EditorSurfaceHandle,
    native::gpui_native_owner::NativeSnapshot};
use verdant_session::PointerId;
use crate::editor::Editor;

type Cancel = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type Draw = Rc<dyn Fn(Input) -> AnyElement>;

#[derive(Default)]
struct Contacts { next: u64, active: HashMap<EntityId, Contact> }
struct Contact {
    owner: EntityId, generation: u64, window: gpui::AnyWindowHandle, input: Input,
    pointer: String, revision: String, lease: f64, cancel: Cancel,
    hitbox: Option<gpui::HitboxId>, touch_start: Option<gpui::Point<gpui::Pixels>>,
    capture: Option<crate::host_pointer::OwnedPointerCapture>,
}
impl Global for Contacts {}
#[derive(Default)]
struct AudioSurfaces { inputs: HashMap<(EntityId, String), (gpui::AnyWindowHandle, Input)> }
impl Global for AudioSurfaces {}

pub(crate) fn has_contact(editor:EntityId,cx:&App)->bool {
    cx.try_global::<Contacts>().is_some_and(|contacts|contacts.active.contains_key(&editor))
}

#[derive(Clone)]
struct Target { editor: WeakEntity<Editor>, revision: String, cancel: Cancel }

fn authority(editor: &Editor) -> String {
    format!("{}:{}:{}", editor.project().project_id, editor.document_generation, editor.revision())
}

#[derive(Clone)]
pub(crate) struct Input {
    handle: EditorSurfaceHandle,
    owner: EntityId,
    target: Rc<RefCell<Target>>,
}

fn pointer(pointer: PointerId) -> String { format!("{pointer:?}") }

impl Input {
    fn editor_id(&self) -> Option<EntityId> { self.target.borrow().editor.upgrade().map(|e| e.entity_id()) }
    fn owns(&self, window: &Window, cx: &App) -> bool {
        self.editor_id().is_some_and(|id| cx.try_global::<Contacts>()
            .and_then(|c| c.active.get(&id)).is_some_and(|entry| entry.owner == self.owner && entry.window == window.window_handle()))
    }
    fn contact<'a>(&self, window: &Window, cx: &'a App) -> Option<&'a Contact> {
        self.editor_id().and_then(|id| cx.try_global::<Contacts>()?.active.get(&id))
            .filter(|contact| contact.owner == self.owner && contact.window == window.window_handle())
    }
    fn clear(&self, window: &mut Window, cx: &mut App) -> Option<Contact> {
        if !self.owns(window, cx) { return None; }
        let id = self.editor_id()?;
        let mut contact = cx.global_mut::<Contacts>().active.remove(&id)?;
        if let Some(capture) = contact.capture.take() { capture.release(window, cx); }
        Some(contact)
    }
    fn capture(&self, hitbox: gpui::HitboxId, window: &mut Window, cx: &mut App) {
        if !self.owns(window, cx) { return; }
        let Some(id) = self.editor_id() else { return; };
        // Touch is claimed through the native touch arena. It must never own
        // the mouse capture, which would make an unrelated mouse release able
        // to finish or release a touch gesture.
        let mouse = self.contact(window, cx).is_some_and(|contact| contact.pointer == pointer(PointerId::Mouse));
        let capture = mouse.then(|| crate::host_pointer::OwnedPointerCapture::acquire(hitbox, window, cx));
        if let Some(contact) = cx.global_mut::<Contacts>().active.get_mut(&id) {
            contact.hitbox = Some(hitbox); contact.capture = capture;
        }
    }
    fn owns_mouse_capture(&self, window: &Window, cx: &App) -> bool {
        self.contact(window, cx).is_some_and(|contact|
            contact.pointer == pointer(PointerId::Mouse)
                && contact.capture.as_ref().is_some_and(|capture| capture.is_current(window, cx)))
    }
    fn set_touch_start(&self, start: gpui::Point<gpui::Pixels>, window: &Window, cx: &mut App) {
        if !self.owns(window, cx) { return; }
        if let Some(id) = self.editor_id() {
            if let Some(contact) = cx.global_mut::<Contacts>().active.get_mut(&id) { contact.touch_start = Some(start); }
        }
    }
    fn owns_touch(&self, start: gpui::Point<gpui::Pixels>, window: &Window, cx: &App) -> bool {
        self.contact(window, cx).is_some_and(|contact|
            contact.pointer == pointer(PointerId::Touch(0)) && contact.touch_start == Some(start))
    }
    fn valid(&self, cx: &App) -> bool {
        let target = self.target.borrow();
        target.editor.upgrade().is_some_and(|e| {
            let editor = e.read(cx);
            authority(editor) == target.revision && !editor.has_proto_pointer_contact() && editor.menu.is_none() && editor.text_prompt.is_none() && editor.modal_dialog.is_none()
        })
    }
    fn call(&self, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) -> bool {
        match self.handle.call(method, args, window, cx) {
            Ok(Value::Bool(accepted)) => {
                // Settle native ownership before draining a successful finish;
                // an idle source otherwise means cancellation, including when
                // a public state observer retired a suspended source callback.
                if accepted && method == "finish" { self.clear(window, cx); }
                self.drain(window, cx);
                accepted
            }
            Ok(_) | Err(_) => {
                let _ = self.handle.call("cancel", vec![], window, cx);
                self.drain(window, cx);
                false
            }
        }
    }
    fn drain(&self, window: &mut Window, cx: &mut App) {
        let failures = self.handle.take_failures();
        // Decode the entire batch before invoking a domain callback. The
        // authored lease distinguishes two contacts of the same pointer and
        // revision; a deferred cancellation can never retire its replacement.
        let events: Result<Vec<Cancellation>, String> = self.handle.take_events().into_iter().map(|(name, payload)| {
            if name != "interactionCancel" { return Err(format!("unknown editing surface event: {name}")); }
            let event: Cancellation = serde_json::from_value(payload).map_err(|e| e.to_string())?;
            if !event.lease.is_finite() || event.lease <= 0.0 || event.lease.fract() != 0.0 || event.pointer.is_empty() {
                return Err("invalid editing contact cancellation".into());
            }
            Ok(event)
        }).collect();
        let malformed = events.is_err();
        if let Ok(events) = events {
            for event in events {
                let matches = self.contact(window, cx).is_some_and(|contact| contact.lease == event.lease
                    && contact.pointer == event.pointer && contact.revision == event.revision);
                if matches {
                    if let Some(contact) = self.clear(window, cx) { (contact.cancel)(&contact.pointer, window, cx); }
                }
            }
        }
        let settled = self.handle.state("phase").ok().as_ref().and_then(Value::as_str) == Some("idle");
        if !failures.is_empty() || malformed || settled {
            if let Some(contact) = self.clear(window, cx) { (contact.cancel)(&contact.pointer, window, cx); }
        }
    }
    pub(crate) fn begin(&self, pointer_id: PointerId, x: f64, y: f64, window: &mut Window, cx: &mut App) -> bool {
        if !self.valid(cx) { self.cancel(window, cx); return false; }
        let Some(id) = self.editor_id() else { return false };
        if cx.try_global::<Contacts>().and_then(|contacts| contacts.active.get(&id)).is_some() { return false; }
        if !self.call("begin", vec![pointer(pointer_id).into(), x.into(), y.into()], window, cx) { return false; }
        let Some(lease) = self.handle.state("lease").ok().and_then(|value| value.as_f64())
            .filter(|lease| lease.is_finite() && *lease > 0.0 && lease.fract() == 0.0) else {
            self.cancel(window, cx); return false;
        };
        if !self.valid(cx) { self.cancel(window, cx); return false; }
        if cx.try_global::<Contacts>().is_none() { cx.set_global(Contacts::default()); }
        let target = self.target.borrow().clone();
        let contacts = cx.global_mut::<Contacts>();
        contacts.next = contacts.next.checked_add(1).expect("editor contact generation exhausted");
        let generation = contacts.next;
        contacts.active.insert(id, Contact { owner: self.owner, generation, window: window.window_handle(),
            input: self.clone(), pointer: pointer(pointer_id), revision: target.revision, lease,
            cancel: target.cancel, hitbox: None, touch_start: None, capture: None });
        true
    }
    pub(crate) fn sample(&self, pointer_id: PointerId, x: f64, y: f64, window: &mut Window, cx: &mut App) -> bool {
        if !self.owns(window, cx) { return false; }
        if !self.valid(cx) { self.cancel(window, cx); return false; }
        self.call("sample", vec![pointer(pointer_id).into(), x.into(), y.into()], window, cx)
    }
    pub(crate) fn finish(&self, pointer_id: PointerId, x: f64, y: f64, window: &mut Window, cx: &mut App) -> bool {
        if !self.owns(window, cx) { return false; }
        if !self.valid(cx) { self.cancel(window, cx); return false; }
        let accepted = self.call("finish", vec![pointer(pointer_id).into(), x.into(), y.into()], window, cx);
        if accepted { self.clear(window, cx); }
        accepted
    }
    pub(crate) fn cancel_pointer(&self, pointer_id: PointerId, window: &mut Window, cx: &mut App) {
        if !self.contact(window, cx).is_some_and(|contact| contact.pointer == pointer(pointer_id)) { return; }
        self.call("cancelPointer", vec![pointer(pointer_id).into()], window, cx);
    }
    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) { self.call("cancel", vec![], window, cx); }
    pub(crate) fn double_activate(&self, clicks:usize, window:&mut Window, cx:&mut App)->bool {
        self.valid(cx)&&self.call("doubleActivate",vec![(clicks as u64).into()],window,cx)
    }
    pub(crate) fn activate(&self, window: &mut Window, cx: &mut App) -> bool {
        self.valid(cx) && self.call("activate", vec![], window, cx)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Cancellation { pointer: String, revision: String, lease: f64 }

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wheel { pub kind:String,pub x:f64,pub y:f64,pub amount:f64 }
impl Input {
    pub(crate) fn wheel(&self,delta:gpui::ScrollDelta,mods:gpui::Modifiers,arrange:bool,window:&mut Window,cx:&mut App)->Option<Wheel> {
        if !self.valid(cx) { return None; }
        let (x,y)=match delta {
            gpui::ScrollDelta::Pixels(p)=>(f64::from(p.x),f64::from(p.y)),
            gpui::ScrollDelta::Lines(p)=>(f64::from(p.x)*40.,f64::from(p.y)*40.),
        };
        let result=self.handle.call("wheel",vec![x.into(),y.into(),(mods.control||mods.platform).into(),mods.shift.into(),mods.alt.into(),arrange.into()],window,cx).ok()?;
        self.drain(window,cx);
        let value:Wheel=serde_json::from_value(result).ok()?;
        if !value.x.is_finite()||!value.y.is_finite()||!value.amount.is_finite()||!matches!(value.kind.as_str(),"pan"|"zoom-x"|"zoom-y") {return None;}
        Some(value)
    }
    pub(crate) fn pinch(&self,scale:f64,window:&mut Window,cx:&mut App)->bool {
        self.valid(cx)&&self.call("pinch",vec![scale.into()],window,cx)
    }
}

#[derive(IntoElement)]
pub(crate) struct Surface { editor: Entity<Editor>, key: String, label: String, threshold: f64, draw: Draw, cancel: Cancel, audio_keys: bool }
pub(crate) fn surface(editor: Entity<Editor>, key: impl Into<String>, label: impl Into<String>, threshold: f64,
    draw: impl Fn(Input) -> AnyElement + 'static,
    cancel: impl Fn(&str, &mut Window, &mut App) + 'static) -> Surface {
    Surface { editor, key: key.into(), label: label.into(), threshold, draw: Rc::new(draw), cancel: Rc::new(cancel), audio_keys: false }
}
impl Surface { pub(crate) fn audio_keys(mut self, enabled: bool) -> Self { self.audio_keys = enabled; self } }

struct Drawing { input: Input, draw: Draw }
impl Render for Drawing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::proto_parent::slot(&self.input.handle.owner, (self.draw)(self.input.clone()))
    }
}
struct Mount { root: Entity<EditorSurface>, drawing: Entity<Drawing>, input: Input, props: Value, _changes: Subscription,
    app: gpui::AsyncApp, window: gpui::AnyWindowHandle, audio_key: Option<(EntityId, String)> }
impl Drop for Mount {
    fn drop(&mut self) {
        let input=self.input.clone(); let handle=self.window; let audio_key = self.audio_key.clone();
        self.app.spawn(async move |cx| { let _=cx.update(|cx| {
            if let Some(key) = audio_key && let Some(registry) = cx.try_global::<AudioSurfaces>()
                && registry.inputs.get(&key).is_some_and(|(_, current)| current.owner == input.owner) {
                cx.global_mut::<AudioSurfaces>().inputs.remove(&key);
            }
            let _=handle.update(cx,|_,window,cx| {
                if input.owns(window,cx) {
                    if let Some(contact) = input.clear(window,cx) { (contact.cancel)(&contact.pointer,window,cx); }
                }
            });
        }); }).detach();
    }
}

/// Route a physical key while Editor is already borrowed. Source decides its
/// meaning and retires the contact; the caller clears its domain previews.
pub(crate) fn audio_key(editor: &Editor, editor_id: EntityId, event: &gpui::KeyDownEvent,
    window: &mut Window, cx: &mut App) -> Option<(String, bool)> {
    let registry = cx.try_global::<AudioSurfaces>()?;
    let focused = registry.inputs.iter().find(|((id, _), (host, input))| *id == editor_id
        && *host == window.window_handle() && input.handle.state("focused").ok() == Some(Value::Bool(true)));
    let input = focused.or_else(|| editor.focus_arrange.is_focused(window).then(||
        registry.inputs.iter().find(|((id, key), (host, _))| *id == editor_id && key == "arrangement"
            && *host == window.window_handle())).flatten()).map(|(_, (_, input))| input.clone())?;
    let revision = authority(editor);
    if input.target.borrow().revision != revision { return None; }
    let key = verdant_editor_surface::native::gpui_native_key::portable_key(&event.keystroke)?;
    let mods = event.keystroke.modifiers;
    let result = input.handle.call("audioKey", vec![key.into(), mods.shift.into(),
        (mods.control || mods.platform).into(), mods.alt.into(), event.is_held.into(), revision.into()], window, cx);
    let retired = input.handle.state("phase").ok().as_ref().and_then(Value::as_str) == Some("idle")
        && input.clear(window, cx).is_some();
    let command = result.ok().and_then(|value| value.as_str().map(str::to_owned)).unwrap_or_default();
    if !matches!(command.as_str(), "" | "delete" | "ripple-delete" | "split" | "select-all" | "join" | "range-tool" | "select-tool" | "escape") { return None; }
    (!command.is_empty() || retired).then_some((command, retired))
}

/// Retire the compiled contact while Editor is already borrowed by a domain
/// cancellation. The caller owns clearing its previews; no reentrant callback.
pub(crate) fn cancel_from_editor(editor: EntityId, window: &mut Window, cx: &mut App) {
    let input=cx.try_global::<Contacts>().and_then(|c|c.active.get(&editor))
        .filter(|contact|contact.window==window.window_handle()).map(|contact|contact.input.clone());
    if let Some(input)=input {
        input.clear(window,cx);
        let _=input.handle.call("cancel",vec![],window,cx);
        input.handle.take_events(); input.handle.take_failures();
    }
}

pub(crate) fn cancel_from_editor_deferred(editor: EntityId, cx: &mut App) {
    let input=cx.try_global::<Contacts>().and_then(|c|c.active.get(&editor)).map(|c|(c.owner,c.generation,c.window,c.input.clone()));
    if let Some((owner,generation,handle,input))=input {
        cx.defer(move |cx| {
            let current=cx.try_global::<Contacts>().and_then(|c|c.active.get(&editor)).is_some_and(|c|c.owner==owner && c.generation==generation);
            if !current { return; }
            let _=handle.update(cx,|_,window,cx| { input.clear(window,cx); let _=input.handle.call("cancel",vec![],window,cx);input.handle.take_events();input.handle.take_failures(); });
        });
    }
}

impl RenderOnce for Surface {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current();
        let initial_parent = parent.clone();
        let revision = authority(self.editor.read(cx));
        let target = Target { editor: self.editor.downgrade(), revision: revision.clone(), cancel: self.cancel };
        let editor = self.editor.read(cx);
        let disabled = editor.menu.is_some() || editor.text_prompt.is_some() || editor.modal_dialog.is_some();
        let props = json!({ "revision": revision, "label": self.label, "threshold": self.threshold, "disabled": disabled,
            "commands": if self.audio_keys { "audio" } else { "none" } });
        let audio_key = self.audio_keys.then(|| (self.editor.entity_id(), self.key.clone()));
        let initial_audio_key = audio_key.clone();
        let initial_props = props.clone();
        let initial_target = target.clone();
        let draw = self.draw.clone();
        let key = SharedString::from(format!("proto-editor-{:?}-{}", self.editor.entity_id(), self.key));
        let mount = window.use_keyed_state(key, cx, move |window, cx| {
            let root = EditorSurface::create(initial_props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|e| panic!("compiled editing surface setup failed: {e:?}"));
            let input = Input { handle: root.read(cx).handle(), owner: root.entity_id(), target: Rc::new(RefCell::new(initial_target)) };
            let drawing = cx.new(|_| Drawing { input: input.clone(), draw });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|e| panic!("compiled editing surface drawing slot failed: {e:?}"));
            let window_handle = window.window_handle();
            let changes = cx.observe(&root, move |mount: &mut Mount, _, cx| {
                let input = mount.input.clone();
                cx.defer(move |cx| { let _ = window_handle.update(cx, |_, window, cx| input.drain(window, cx)); });
            });
            Mount { root, drawing, input, props: initial_props, _changes: changes, app: cx.to_async(), window: window_handle, audio_key: initial_audio_key }
        });
        mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx))
                    .expect("compiled editing surface parent");
            }
            // Retire old authority while its old cancellation target is still
            // available; a new document must never receive the old callback.
            if mount.props != props {
                let _ = mount.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx));
                mount.input.drain(window, cx);
                mount.props = props;
            }
            *mount.input.target.borrow_mut() = target;
            if let Some(key) = audio_key {
                if cx.try_global::<AudioSurfaces>().is_none() { cx.set_global(AudioSurfaces::default()); }
                cx.global_mut::<AudioSurfaces>().inputs.insert(key.clone(), (window.window_handle(), mount.input.clone()));
                mount.audio_key = Some(key);
            }
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; cx.notify(); });
            mount.root.clone()
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerPhase { Begin, Sample, Finish, Context }
#[derive(Clone, Copy)]
pub(crate) struct Sample {
    pub phase: PointerPhase,
    pub pointer: PointerId,
    pub x: f64,
    pub y: f64,
    pub position: gpui::Point<gpui::Pixels>,
    pub modifiers: gpui::Modifiers,
    pub clicks: usize,
}

/// Physical input transport. Every domain callback follows admission by the
/// generated component; final coordinates are sampled before the terminal call.
/// The return value is meaningful for Begin only (domain hit-test acceptance).
pub(crate) fn install_pointer_input(input: &Input, bounds: gpui::Bounds<gpui::Pixels>, hitbox: &gpui::Hitbox,
    window: &mut Window, callback: impl Fn(Sample, &mut Window, &mut App) -> bool + 'static) {
    use gpui::{DispatchPhase, MouseButton, MouseDownEvent, MouseExitEvent, MouseMoveEvent, MouseUpEvent, TouchDragEvent, TouchPhase};
    let callback = Rc::new(callback);
    let (gate, hit, send) = (input.clone(), hitbox.clone(), callback.clone());
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !hit.is_hovered(window) { return; }
        if !crate::host_pointer::contains_window_point(window, (f64::from(event.position.x), f64::from(event.position.y))) { return; }
        let p = event.position - bounds.origin;
        let mut sample = Sample { phase: PointerPhase::Begin, pointer: PointerId::Mouse,
            x: f64::from(p.x), y: f64::from(p.y), position: event.position, modifiers: event.modifiers, clicks: event.click_count };
        if event.button == MouseButton::Right {
            if gate.activate(window, cx) { sample.phase = PointerPhase::Context; send(sample, window, cx); cx.stop_propagation(); }
        } else if event.button == MouseButton::Left && window.captured_hitbox().is_none()
            && gate.begin(sample.pointer, sample.x, sample.y, window, cx) {
            if send(sample, window, cx) { gate.capture(hit.id, window, cx); }
            else { gate.cancel(window, cx); }
            cx.stop_propagation();
        }
    });
    let (gate, send) = (input.clone(), callback.clone());
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Capture { return; }
        if !gate.contact(window, cx).is_some_and(|contact| contact.pointer == pointer(PointerId::Mouse)) { return; }
        if event.pressed_button != Some(MouseButton::Left) || !gate.owns_mouse_capture(window, cx) {
            gate.cancel_pointer(PointerId::Mouse, window, cx); return;
        }
        if !crate::host_pointer::contains_window_point(window, (f64::from(event.position.x), f64::from(event.position.y))) {
            gate.cancel_pointer(PointerId::Mouse, window, cx); return;
        }
        let p = event.position - bounds.origin;
        let sample = Sample { phase: PointerPhase::Sample, pointer: PointerId::Mouse,
            x: f64::from(p.x), y: f64::from(p.y), position: event.position, modifiers: event.modifiers, clicks: 1 };
        if gate.sample(sample.pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
    });
    let (gate, send) = (input.clone(), callback.clone());
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Capture || event.button != MouseButton::Left { return; }
        if !gate.contact(window, cx).is_some_and(|contact| contact.pointer == pointer(PointerId::Mouse)) { return; }
        if !gate.owns_mouse_capture(window, cx) { gate.cancel_pointer(PointerId::Mouse, window, cx); return; }
        if !crate::host_pointer::contains_window_point(window, (f64::from(event.position.x), f64::from(event.position.y))) {
            gate.cancel_pointer(PointerId::Mouse, window, cx); return;
        }
        let p = event.position - bounds.origin;
        let mut sample = Sample { phase: PointerPhase::Sample, pointer: PointerId::Mouse,
            x: f64::from(p.x), y: f64::from(p.y), position: event.position, modifiers: event.modifiers, clicks: 1 };
        if gate.sample(sample.pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
        if gate.finish(sample.pointer, sample.x, sample.y, window, cx) {
            sample.phase = PointerPhase::Finish; send(sample, window, cx);
        }
    });
    let gate = input.clone();
    window.on_mouse_event(move |_: &MouseExitEvent, phase, window, cx| {
        if phase == DispatchPhase::Capture { gate.cancel_pointer(PointerId::Mouse, window, cx); }
    });
    let (gate, hit, send) = (input.clone(), hitbox.clone(), callback);
    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
        let expected = if event.phase == TouchPhase::Started { DispatchPhase::Bubble } else { DispatchPhase::Capture };
        if phase != expected { return; }
        let pointer = PointerId::Touch(0);
        if event.phase == TouchPhase::Started {
            if window.default_prevented() || window.captured_hitbox().is_some() { return; }
        } else if !gate.owns_touch(event.start_position, window, cx) { return; }
        if !crate::host_pointer::contains_window_point(window, (f64::from(event.position.x), f64::from(event.position.y))) {
            gate.cancel_pointer(pointer, window, cx); return;
        }
        let p = event.position - bounds.origin;
        let mut sample = Sample { phase: PointerPhase::Sample, pointer,
            x: f64::from(p.x), y: f64::from(p.y), position: event.position, modifiers: gpui::Modifiers::default(), clicks: 1 };
        match event.phase {
            TouchPhase::Started if hit.is_hovered_at(event.start_position, window) => {
                let start = event.start_position - bounds.origin;
                if !gate.begin(pointer, f64::from(start.x), f64::from(start.y), window, cx) { return; }
                gate.set_touch_start(event.start_position, window, cx);
                let begin = Sample { phase: PointerPhase::Begin, x: f64::from(start.x), y: f64::from(start.y), position: event.start_position, ..sample };
                if send(begin, window, cx) {
                    gate.capture(hit.id, window, cx); window.prevent_default(); cx.stop_propagation();
                    if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
                } else { gate.cancel(window, cx); }
            }
            TouchPhase::Moved => {
                window.prevent_default(); cx.stop_propagation();
                if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
            }
            TouchPhase::Ended => {
                window.prevent_default(); cx.stop_propagation();
                if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
                if gate.finish(pointer, sample.x, sample.y, window, cx) { sample.phase = PointerPhase::Finish; send(sample, window, cx); }
            }
            TouchPhase::Cancelled => { gate.cancel_pointer(pointer, window, cx); cx.stop_propagation(); }, _ => {}
        }
    });
}

#[cfg(test)]
mod contact_tests {
    use gpui::ParentElement;
    use super::*;
    use std::cell::Cell;
    use gpui::{canvas, div, point, px, size, HitboxBehavior, MouseButton, MouseDownEvent,
        MouseMoveEvent, MouseUpEvent, PlatformInput, Styled, TestAppContext, WindowHandle};

    struct Harness {
        editor: Entity<Editor>, visible: bool,
        input: Rc<RefCell<Option<Input>>>, phases: Rc<RefCell<Vec<&'static str>>>,
        cancels: Rc<Cell<usize>>, hitbox: Rc<Cell<Option<gpui::HitboxId>>>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut root = div().size_full();
            if self.visible {
                let (observed, phases, hitbox, cancels) =
                    (self.input.clone(), self.phases.clone(), self.hitbox.clone(), self.cancels.clone());
                root = root.child(div().w(px(120.)).h(px(90.)).child(surface(self.editor.clone(), "native-contact", "编辑接触", 0.,
                    move |input| {
                        *observed.borrow_mut() = Some(input.clone());
                        let phases = phases.clone(); let hitbox = hitbox.clone();
                        canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                            move |bounds, hit, window, _| {
                                hitbox.set(Some(hit.id)); let phases = phases.clone();
                                install_pointer_input(&input, bounds, &hit, window, move |sample, _, _| {
                                    phases.borrow_mut().push(match sample.phase {
                                        PointerPhase::Begin => "begin", PointerPhase::Sample => "sample",
                                        PointerPhase::Finish => "finish", PointerPhase::Context => "context",
                                    }); true
                                });
                            }).size_full().into_any_element()
                    }, move |_, _, _| cancels.set(cancels.get() + 1))));
            }
            root
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let host = cx.open_window(size(px(260.), px(160.)), |window, cx| Harness {
            editor: cx.new(|cx| Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx)),
            visible: true, input: Rc::new(RefCell::new(None)), phases: Rc::default(),
            cancels: Rc::new(Cell::new(0)), hitbox: Rc::new(Cell::new(None)),
        });
        host.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(host, cx); host
    }
    fn down(window: &mut Window, cx: &mut App) {
        window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent {
            position: point(px(20.), px(20.)), button: MouseButton::Left, ..Default::default()
        }), cx);
    }
    fn up(window: &mut Window, cx: &mut App) {
        window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent {
            position: point(px(160.), px(110.)), button: MouseButton::Left, ..Default::default()
        }), cx);
    }
    #[gpui::test]
    fn native_editing_contact_lost_button_cancels_but_a_valid_terminal_sample_commits(cx: &mut TestAppContext) {
        let host = open(cx);
        host.update(cx, |host, window, cx| {
            down(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
            assert!(window.captured_hitbox().is_some());
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(40.), px(30.)), pressed_button: None, ..Default::default()
            }), cx);
            assert_eq!(host.cancels.get(), 1);
            assert!(window.captured_hitbox().is_none());
            up(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
            down(window, cx); up(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin", "begin", "sample", "finish"]);
        }).unwrap();
    }
    #[gpui::test]
    fn native_editing_contact_cannot_commit_or_release_a_same_hitbox_replacement(cx: &mut TestAppContext) {
        let host = open(cx);
        host.update(cx, |host, window, cx| {
            down(window, cx);
            let hitbox = host.hitbox.get().unwrap();
            let replacement = crate::host_pointer::OwnedPointerCapture::acquire(hitbox, window, cx);
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(40.), px(30.)), pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            assert_eq!(host.cancels.get(), 1);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
            assert!(replacement.is_current(window, cx), "old cancellation preserves the newer physical lease");
            replacement.release(window, cx); up(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
        }).unwrap();
    }
    #[gpui::test]
    fn native_touch_contact_does_not_borrow_mouse_capture_or_accept_mouse_release(cx: &mut TestAppContext) {
        let host = open(cx);
        host.update(cx, |host, window, cx| {
            let touch = |phase, position, window: &mut Window, cx: &mut App| {
                window.dispatch_event(PlatformInput::Touch(gpui::TouchEvent {
                    id: gpui::TouchId(51), phase, position, ..Default::default()
                }), cx);
            };
            touch(gpui::TouchPhase::Started, point(px(20.), px(20.)), window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
            assert!(window.captured_hitbox().is_none());
            up(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
            touch(gpui::TouchPhase::Moved, point(px(80.), px(30.)), window, cx);
            touch(gpui::TouchPhase::Ended, point(px(100.), px(30.)), window, cx);
            assert_eq!(host.phases.borrow().iter().filter(|phase| **phase == "finish").count(), 1);
            assert_eq!(host.cancels.get(), 0);
        }).unwrap();
    }
    #[gpui::test]
    fn late_generated_cancel_cannot_retire_the_next_contact_with_the_same_pointer(cx: &mut TestAppContext) {
        let host = open(cx);
        host.update(cx, |host, window, cx| {
            down(window, cx);
            let input = host.input.borrow().as_ref().unwrap().clone();
            // Delay a real generated cancellation batch while its old physical
            // cleanup settles, then model a newer native acquisition.
            input.handle.call("cancel", vec![], window, cx).unwrap();
            let delayed = input.handle.take_events();
            assert_eq!(delayed.len(), 1);
            input.clear(window, cx);
            assert!(input.begin(PointerId::Mouse, 20., 20., window, cx));
            input.capture(host.hitbox.get().unwrap(), window, cx);
            input.handle.owner.borrow_mut().emitted.extend(delayed);
            input.drain(window, cx);
            assert!(input.owns_mouse_capture(window, cx));
            assert_eq!(host.cancels.get(), 0);
            assert!(input.finish(PointerId::Mouse, 30., 30., window, cx));
        }).unwrap();
    }
    #[gpui::test]
    fn removed_editing_surface_retires_its_contact_before_a_late_terminal_input(cx: &mut TestAppContext) {
        let host = open(cx);
        host.update(cx, |host, window, cx| { down(window, cx); host.visible = false; cx.notify(); }).unwrap();
        crate::native_test_support::frames(host, cx);
        host.update(cx, |host, window, cx| {
            assert!(!has_contact(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_none());
            assert_eq!(host.cancels.get(), 1);
            up(window, cx);
            assert_eq!(&*host.phases.borrow(), &["begin"]);
        }).unwrap();
    }
}
