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
type Command = Rc<dyn Fn(&str, &mut Window, &mut App)>;

#[derive(Default)]
struct Contacts { next: u64, active: HashMap<EntityId, Contact> }
struct Contact {
    owner: EntityId, generation: u64, window: gpui::AnyWindowHandle, input: Input,
    pointer: String, capture: Option<crate::host_pointer::OwnedPointerCapture>,
}
impl Global for Contacts {}

pub(crate) fn has_contact(editor:EntityId,cx:&App)->bool {
    cx.try_global::<Contacts>().is_some_and(|contacts|contacts.active.contains_key(&editor))
}

#[derive(Clone)]
struct Target { editor: WeakEntity<Editor>, editor_id:EntityId, revision: String, cancel: Cancel, command: Option<Command> }

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
    fn editor_id(&self) -> Option<EntityId> { Some(self.target.borrow().editor_id) }
    fn owns(&self, window: &Window, cx: &App) -> bool {
        self.editor_id().is_some_and(|id| cx.try_global::<Contacts>()
            .and_then(|c| c.active.get(&id)).is_some_and(|entry| entry.owner == self.owner && entry.window == window.window_handle()))
    }
    fn clear(&self, window: &mut Window, cx: &mut App) {
        if !self.owns(window, cx) { return; }
        if let Some(id) = self.editor_id() {
            if let Some(contact) = cx.global_mut::<Contacts>().active.remove(&id) {
                if let Some(capture) = contact.capture { capture.release(window, cx); }
            }
        }
    }
    fn capture(&self, hitbox: gpui::HitboxId, window: &mut Window, cx: &mut App) {
        if !self.owns(window, cx) { return; }
        let Some(id) = self.editor_id() else { return; };
        let capture = crate::host_pointer::OwnedPointerCapture::acquire(hitbox, window, cx);
        if let Some(contact) = cx.global_mut::<Contacts>().active.get_mut(&id) { contact.capture = Some(capture); }
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
        for (name, payload) in self.handle.take_events() {
            if name == "command" {
                let target = self.target.borrow().clone();
                if payload.get("revision").and_then(Value::as_str) == Some(target.revision.as_str()) && self.valid(cx) {
                    if let (Some(callback), Some(command)) = (target.command, payload.get("command").and_then(Value::as_str)) {
                        callback(command, window, cx);
                    }
                }
                continue;
            }
            if name != "interactionCancel" { continue; }
            let Some(pointer) = payload.get("pointer").and_then(Value::as_str) else { continue };
            if self.owns(window, cx) {
                let callback = self.target.borrow().cancel.clone();
                self.clear(window, cx);
                callback(pointer, window, cx);
            }
        }
        let settled = self.handle.state("phase").ok().as_ref().and_then(Value::as_str) == Some("idle");
        if (!failures.is_empty() || settled) && self.owns(window, cx) {
            let pointer = self.editor_id().and_then(|id| cx.try_global::<Contacts>()
                .and_then(|contacts| contacts.active.get(&id)).map(|contact|contact.pointer.clone())).unwrap_or_default();
            let callback = self.target.borrow().cancel.clone();
            self.clear(window, cx);
            callback(&pointer, window, cx);
        }
    }
    pub(crate) fn begin(&self, pointer_id: PointerId, x: f64, y: f64, window: &mut Window, cx: &mut App) -> bool {
        if !self.valid(cx) { self.cancel(window, cx); return false; }
        let Some(id) = self.editor_id() else { return false };
        if cx.try_global::<Contacts>().and_then(|contacts| contacts.active.get(&id)).is_some() { return false; }
        if !self.call("begin", vec![pointer(pointer_id).into(), x.into(), y.into()], window, cx) { return false; }
        if cx.try_global::<Contacts>().is_none() { cx.set_global(Contacts::default()); }
        let contacts = cx.global_mut::<Contacts>();
        contacts.next = contacts.next.checked_add(1).expect("editor contact generation exhausted");
        let generation = contacts.next;
        contacts.active.insert(id, Contact { owner: self.owner, generation, window: window.window_handle(),
            input: self.clone(), pointer: pointer(pointer_id), capture: None });
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

/// Restore an application canvas focus after a compiled toolbar command.
/// The mounted source still decides whether cancellation/focus is admitted.
pub(crate) fn focus_commands(mode: &str, window: &mut Window, cx: &mut App) -> bool {
    use verdant_editor_surface::native::gpui_native_owner::all_owners;
    let owner = all_owners().into_iter().find(|owner| {
        let source = owner.borrow();
        source.mounted && source.window_id == Some(window.window_handle().window_id())
            && source.declared_events.contains("command")
            && source.props.get("commandMode").and_then(Value::as_str) == Some(mode)
    });
    owner.is_some_and(|owner| EditorSurfaceHandle { owner }.call("activate", vec![], window, cx)
        .ok().and_then(|value| value.as_bool()).unwrap_or(false))
}

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
pub(crate) struct Surface { editor: Entity<Editor>, key: String, label: String, threshold: f64, draw: Draw, cancel: Cancel, command_mode: String, command: Option<Command> }
pub(crate) fn surface(editor: Entity<Editor>, key: impl Into<String>, label: impl Into<String>, threshold: f64,
    draw: impl Fn(Input) -> AnyElement + 'static,
    cancel: impl Fn(&str, &mut Window, &mut App) + 'static) -> Surface {
    Surface { editor, key: key.into(), label: label.into(), threshold, draw: Rc::new(draw), cancel: Rc::new(cancel), command_mode:String::new(), command:None }
}

impl Surface {
    pub(crate) fn commands(mut self, mode: &str, callback: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.command_mode = mode.into(); self.command = Some(Rc::new(callback)); self
    }
}

struct Drawing { input: Input, draw: Draw }
impl Render for Drawing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.input.handle.owner,(self.draw)(self.input.clone())) }
}
struct Mount { root: Entity<EditorSurface>, drawing: Entity<Drawing>, input: Input, props: Value, _changes: Subscription,
    app: gpui::AsyncApp, window: gpui::AnyWindowHandle }
impl Drop for Mount {
    fn drop(&mut self) {
        let input=self.input.clone();let window_handle=self.window;
        self.app.spawn(async move |cx| {let _=cx.update(|cx|{
            let Some(editor)=input.editor_id()else{return;};
            let owned=cx.try_global::<Contacts>().and_then(|contacts|contacts.active.get(&editor))
                .is_some_and(|contact|contact.owner==input.owner);
            if !owned{return;}
            let mut contact=cx.global_mut::<Contacts>().active.remove(&editor).expect("owned editor contact");
            if let Some(capture)=contact.capture.take(){capture.release_deferred(cx);}
            let callback=input.target.borrow().cancel.clone();
            // Remove native ownership even after the window or Editor died.
            // Only the domain callback requires the original live window.
            let _=window_handle.update(cx,|_,window,cx|callback(&contact.pointer,window,cx));
        });}).detach();
    }
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
            if handle.update(cx,|_,window,cx| { input.clear(window,cx); let _=input.handle.call("cancel",vec![],window,cx);input.handle.take_events();input.handle.take_failures(); }).is_err() {
                let current=cx.try_global::<Contacts>().and_then(|contacts|contacts.active.get(&editor))
                    .is_some_and(|contact|contact.owner==owner&&contact.generation==generation);
                if current {cx.global_mut::<Contacts>().active.remove(&editor);}
            }
        });
    }
}

impl RenderOnce for Surface {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let revision = authority(self.editor.read(cx));
        let target = Target { editor: self.editor.downgrade(), editor_id:self.editor.entity_id(), revision: revision.clone(), cancel: self.cancel, command:self.command };
        let editor = self.editor.read(cx);
        let disabled = editor.menu.is_some() || editor.text_prompt.is_some() || editor.modal_dialog.is_some();
        let props = json!({ "revision": revision, "label": self.label, "threshold": self.threshold, "disabled": disabled, "commandMode":self.command_mode });
        let parent=crate::proto_parent::current();let initial_parent=parent.clone();
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
            Mount { root, drawing, input, props: initial_props, _changes: changes, app: cx.to_async(), window: window_handle }
        });
        mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.input.handle.owner,&parent) {
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled editor parent");
            }
            // Retire old authority while its old cancellation target is still
            // available; a new document must never receive the old callback.
            if mount.props != props {
                let _ = mount.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx));
                mount.input.drain(window, cx);
                mount.props = props;
            }
            *mount.input.target.borrow_mut() = target;
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
    use gpui::{DispatchPhase, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TouchDragEvent, TouchPhase};
    let callback = Rc::new(callback);
    let (gate, hit, send) = (input.clone(), hitbox.clone(), callback.clone());
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !hit.is_hovered(window) { return; }
        let p = event.position - bounds.origin;
        let mut sample = Sample { phase: PointerPhase::Begin, pointer: PointerId::Mouse,
            x: f64::from(p.x), y: f64::from(p.y), position: event.position, modifiers: event.modifiers, clicks: event.click_count };
        if event.button == MouseButton::Right {
            if gate.activate(window, cx) { sample.phase = PointerPhase::Context; send(sample, window, cx); cx.stop_propagation(); }
        } else if event.button == MouseButton::Left && gate.begin(sample.pointer, sample.x, sample.y, window, cx) {
            if send(sample, window, cx) { gate.capture(hit.id, window, cx); }
            else { gate.cancel(window, cx); }
            cx.stop_propagation();
        }
    });
    let exited = input.clone();
    window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, window, cx| {
        if phase == DispatchPhase::Capture { exited.cancel_pointer(PointerId::Mouse, window, cx); }
    });
    let (gate, send) = (input.clone(), callback.clone());
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Capture { return; }
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
    let (gate, hit, send) = (input.clone(), hitbox.clone(), callback);
    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
        let expected = if event.phase == TouchPhase::Started { DispatchPhase::Bubble } else { DispatchPhase::Capture };
        if phase != expected { return; }
        let pointer = PointerId::Touch(0);
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
                let begin = Sample { phase: PointerPhase::Begin, x: f64::from(start.x), y: f64::from(start.y), position: event.start_position, ..sample };
                if send(begin, window, cx) {
                    gate.capture(hit.id, window, cx); window.prevent_default(); cx.stop_propagation();
                    if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
                } else { gate.cancel(window, cx); }
            }
            TouchPhase::Moved => { if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); } }
            TouchPhase::Ended => {
                if gate.sample(pointer, sample.x, sample.y, window, cx) { send(sample, window, cx); }
                if gate.finish(pointer, sample.x, sample.y, window, cx) { sample.phase = PointerPhase::Finish; send(sample, window, cx); }
            }
            TouchPhase::Cancelled => gate.cancel_pointer(pointer, window, cx), _ => {}
        }
    });
}
