//! Native boundary for authored continuous controls.
//!
//! A compiled `ContinuousControl` owns phase, preview, keyboard, focus and
//! accessibility. This module only maps native coordinates/physical units,
//! routes owned capture, paints the Adwaita slot and checks the project token
//! before passing a proposal to the existing domain command boundary.

use std::{cell::{Cell, RefCell}, rc::Rc};
use gpui::{App, AppContext, Bounds, Context, Entity, IntoElement, MouseButton,
    MouseDownEvent, Pixels, Render, RenderOnce, SharedString, Subscription,
    WeakEntity, Window, div, prelude::*};
use serde::Deserialize;
use serde_json::{Value, json};
use verdant_continuous_control::{ContinuousControl, ContinuousControlHandle,
    native::{gpui_native_move::{MoveGestureBinding, MoveGestureCancelContext,
        MoveGestureCancelReason, MoveGestureInput, MoveGestureLease,
        MoveGestureSample, attach_move_gesture}, gpui_native_owner::NativeSnapshot}};
use crate::{editor::Editor, proto_knob::Contact};

#[derive(Clone, Debug)]
pub(crate) struct Spec {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub fine_step: f64,
    pub default_value: f64,
    pub label: String,
    pub role: &'static str,
    pub drag_mode: &'static str,
    pub double_action: &'static str,
    pub tap_action: &'static str,
    /// The host has a transient draft/viewport that echoes emitted previews.
    /// Such an echo is distinct from a new project revision or external value.
    pub preview_echo: bool,
    pub disabled: bool,
    pub mixer_preview: bool,
}
impl Spec {
    pub fn slider(value: f64, min: f64, max: f64, label: impl Into<String>) -> Self {
        Self { value, min, max, step: (max-min)/100.0, fine_step: (max-min)/1000.0,
            default_value: value, label: label.into(), role: "slider", drag_mode: "absolute",
            double_action: "reset", tap_action: "none", preview_echo: false, disabled: false, mixer_preview: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventKind { Start, Preview, Commit, Cancel, Activate, Edit }
#[derive(Clone, Copy, Debug)]
pub(crate) struct Event { pub kind: EventKind, pub value: f64 }
#[derive(Clone, Copy, Debug)]
pub(crate) struct Display { pub value: f64, pub initial: f64, pub active: bool, pub focus_visible: bool }
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample { pub value: f64, pub fine: f64, pub jump: bool }

type FocusBinding = Rc<dyn Fn(&gpui::FocusHandle, &mut Window, &mut App)>;
type Geometry = Rc<dyn Fn(&MoveGestureSample, Bounds<Pixels>, f64) -> Sample>;
type Callback = Rc<dyn Fn(&mut Editor, Event, &mut Window, &mut Context<Editor>)>;
type Validator = Rc<dyn Fn(&Editor) -> bool>;
type Visual = Rc<dyn Fn(Display, Actions, &mut Window, &mut App) -> gpui::AnyElement>;

#[derive(Clone)]
struct Target {
    editor: WeakEntity<Editor>, generation: u64, revision: u64, token: String,
    window: gpui::AnyWindowHandle, callback: Callback, valid: Validator,
    dialog: Option<std::mem::Discriminant<crate::dialogs::ModalDialog>>, mixer_preview: bool,
}
impl Target {
    fn valid(&self, editor: &Editor) -> bool {
        editor.document_generation == self.generation && editor.revision().get() == self.revision
            && editor.modal_dialog.as_ref().map(std::mem::discriminant) == self.dialog
            && editor.menu.is_none() && editor.text_prompt.is_none()
            && editor.session.pointer_owner().is_none() && editor.note_creation.is_none()
            && editor.lane_stroke.is_none() && editor.device_drag.is_none()
            && (self.mixer_preview || (editor.mixer_drag.is_none() && editor.master_drag.is_none()))
            && (self.valid)(editor)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload { value: f64, revision: String }

pub(crate) fn state<T: serde::de::DeserializeOwned>(handle: &ContinuousControlHandle, key: &str) -> Result<T, String> {
    let value = handle.state(key).map_err(|error| format!("连续控件状态 {key}：{error:?}"))?;
    serde_json::from_value(value).map_err(|error| format!("连续控件状态 {key}：{error}"))
}
fn report(target: &Target, message: String, cx: &mut App) {
    let _ = target.editor.update(cx, |editor, cx| { editor.notice(message); cx.notify(); });
}
fn decode(events: Vec<(String, Value)>) -> Result<Vec<(Event, String)>, String> {
    events.into_iter().map(|(name, payload)| {
        let kind = match name.as_str() {
            "interactionStart" => EventKind::Start, "valuePreview" => EventKind::Preview,
            "valueCommit" => EventKind::Commit, "interactionCancel" => EventKind::Cancel,
            "activate" => EventKind::Activate, "editRequest" => EventKind::Edit,
            _ => return Err(format!("连续控件返回未知事件：{name}")),
        };
        let payload: Payload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
        if !payload.value.is_finite() { return Err("连续控件返回非有限值".into()); }
        Ok((Event { kind, value: payload.value }, payload.revision))
    }).collect()
}
fn drain(handle: &ContinuousControlHandle, target: &Target, window: &mut Window, cx: &mut App) -> bool {
    let events = handle.take_events();
    let failures = handle.take_failures();
    if !failures.is_empty() { report(target, format!("连续控件未提交修改：{failures:?}"), cx); return false; }
    let outputs = match decode(events) { Ok(outputs) => outputs, Err(error) => { report(target, error, cx); return false; } };
    for (event, revision) in outputs {
        if revision != target.token || window.window_handle() != target.window { continue; }
        let callback = target.callback.clone();
        let _ = target.editor.update(cx, |editor, cx| {
            if target.valid(editor) { callback(editor, event, window, cx); }
        });
    }
    true
}
fn call(handle: &ContinuousControlHandle, method: &str, arguments: Vec<Value>, window: &mut Window, cx: &mut App) -> Result<bool, String> {
    handle.call(method, arguments, window, cx).map_err(|error| format!("连续控件 {method}：{error:?}"))?
        .as_bool().ok_or_else(|| format!("连续控件 {method} 没有返回布尔值"))
}
fn cancel_source(handle: &ContinuousControlHandle, target: &Target, window: &mut Window, cx: &mut App) {
    // onUnmounted may already have emitted the rollback before the native
    // lease is released. Drain it even if before-dispose retired the source.
    if !handle.owner().upgrade().is_some_and(|owner| owner.borrow().disposed) {
        if let Err(error) = call(handle, "cancel", vec![], window, cx) { report(target, error, cx); }
    }
    drain(handle, target, window, cx);
}

#[derive(Clone)]
pub(crate) struct Actions { handle: ContinuousControlHandle, target: Rc<RefCell<Target>> }
impl Actions {
    pub fn invoke(&self, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) {
        let target = self.target.borrow().clone();
        match call(&self.handle, method, args, window, cx) {
            Ok(_) => { drain(&self.handle, &target, window, cx); }
            Err(error) => { self.handle.take_events(); report(&target, error, cx); }
        }
    }
}

struct Binding {
    handle: ContinuousControlHandle, target: Rc<RefCell<Target>>, geometry: Rc<RefCell<Geometry>>,
    double_click: Rc<Cell<bool>>, contact: Rc<Contact>, capture: RefCell<Option<crate::host_pointer::OwnedPointerCapture>>, window: gpui::AnyWindowHandle, app: gpui::AsyncApp,
}
impl Binding {
    fn sample(&self, sample: &MoveGestureSample, window: &Window) -> Option<Sample> {
        let bounds = self.handle.owner.borrow().native_hitbox.as_ref()?.bounds;
        if !sample.position.x.is_finite() || !sample.position.y.is_finite()
            || !sample.total_delta.x.is_finite() || !sample.total_delta.y.is_finite()
            || !crate::host_pointer::contains_window_point(window, (f64::from(bounds.origin.x) + sample.position.x,
                f64::from(bounds.origin.y) + sample.position.y)) { return None; }
        let initial = state::<f64>(&self.handle, if state::<String>(&self.handle, "phase").ok()? == "idle" { "value" } else { "initialValue" }).ok()?;
        let mapped = (self.geometry.borrow())(sample, bounds, initial);
        (mapped.value.is_finite() && mapped.fine.is_finite()).then_some(mapped)
    }
    fn invoke(&self, method: &str, args: Vec<Value>, window: &mut Window, cx: &mut App) -> bool {
        let target = self.target.borrow().clone();
        match call(&self.handle, method, args, window, cx) {
            Ok(consumed) => { drain(&self.handle, &target, window, cx); consumed }
            Err(error) => {
                self.contact.input.set(None); self.handle.take_events();
                report(&target, error, cx); cancel_source(&self.handle, &target, window, cx); false
            }
        }
    }
    fn cancel(&self, window: &mut Window, cx: &mut App) {
        self.contact.input.set(None);
        if let Some(capture) = self.capture.borrow_mut().take() { capture.release(window, cx); }
        cancel_source(&self.handle, &self.target.borrow(), window, cx);
        let lease = self.contact.lease.borrow().as_ref().and_then(std::rc::Weak::upgrade);
        if let Some(lease) = lease { lease.cancel(MoveGestureCancelReason::HostCancel, window, cx); }
    }
}
impl MoveGestureBinding for Binding {
    fn should_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) -> bool {
        if !matches!(sample.input, MoveGestureInput::Mouse | MoveGestureInput::Touch) || self.sample(sample, window).is_none() { return false; }
        let target = self.target.borrow();
        if crate::proto_editor_surface::has_contact(target.editor.entity_id(), cx) { return false; }
        let available = target.editor.upgrade().is_some_and(|entity| {
            let editor = entity.read(cx);
            let competing = editor.proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade)
                .is_some_and(|contact| contact.is_active() && !Rc::ptr_eq(&contact, &self.contact));
            target.valid(editor) && !competing && editor.session.pointer_owner().is_none()
                && editor.note_creation.is_none() && editor.lane_stroke.is_none() && editor.device_drag.is_none()
                && editor.mixer_drag.is_none() && editor.master_drag.is_none()
        });
        available && state::<bool>(&self.handle, "disabled").is_ok_and(|disabled| !disabled)
    }
    fn on_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        let target = self.target.borrow().clone();
        self.contact.begin(sample.input);
        let _ = target.editor.update(cx, |editor, _| editor.proto_pointer_contact = Some(Rc::downgrade(&self.contact)));
        if state::<String>(&self.handle, "phase").is_ok_and(|phase| phase != "idle") { cancel_source(&self.handle, &target, window, cx); }
        let Some(mapped) = self.sample(sample, window) else { self.cancel(window, cx); return; };
        let double = sample.input == MoveGestureInput::Mouse && self.double_click.replace(false);
        let consumed = self.invoke("begin", vec![mapped.value.into(), mapped.jump.into(), sample.modifiers.alt.into(), double.into()], window, cx);
        if !consumed || state::<String>(&self.handle, "phase").is_ok_and(|phase| phase == "idle") {
            self.contact.input.set(None);
        } else if sample.input == MoveGestureInput::Mouse && let Some(hitbox) = self.handle.owner.borrow().native_hitbox.as_ref() {
            *self.capture.borrow_mut() = Some(crate::host_pointer::OwnedPointerCapture::acquire(hitbox.id, window, cx));
        }
    }
    fn on_move(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        if self.contact.input.get() != Some(sample.input) { return; }
        if sample.input == MoveGestureInput::Mouse && !self.capture.borrow().as_ref().is_some_and(|capture| capture.is_current(window, cx)) {
            self.cancel(window, cx); return;
        }
        let Some(mapped) = self.sample(sample, window) else { self.cancel(window, cx); return; };
        self.invoke("sample", vec![mapped.value.into(), mapped.fine.into(),
            (sample.total_delta.x.hypot(sample.total_delta.y) >= 3.0).into(), sample.modifiers.shift.into()], window, cx);
    }
    fn on_end(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        if self.contact.input.get() != Some(sample.input) { return; }
        if sample.input == MoveGestureInput::Mouse && !self.capture.borrow().as_ref().is_some_and(|capture| capture.is_current(window, cx)) {
            self.cancel(window, cx); return;
        }
        self.contact.input.set(None);
        if let Some(capture) = self.capture.borrow_mut().take() { capture.release(window, cx); }
        let Some(mapped) = self.sample(sample, window) else { self.cancel(window, cx); return; };
        self.invoke("finish", vec![mapped.value.into(), mapped.fine.into(),
            (sample.total_delta.x.hypot(sample.total_delta.y) >= 3.0).into(), sample.modifiers.shift.into()], window, cx);
    }
    fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
        let active = self.contact.input.replace(None).is_some();
        let capture = self.capture.borrow_mut().take();
        let target = self.target.borrow().clone();
        if let Some(context) = context {
            if let Some(capture) = capture { capture.release(context.window, context.cx); }
            if active { cancel_source(&self.handle, &target, context.window, context.cx); }
        } else {
            drop(capture);
            if !active { return; }
            let (handle, window, contact) = (self.handle.clone(), self.window, self.contact.clone());
            let expected = contact.generation.get();
            self.app.spawn(async move |cx| {
                if contact.is_current(expected) { let _ = window.update(cx, |_, window, cx| cancel_source(&handle, &target, window, cx)); }
            }).detach();
        }
    }
}

struct Drawing { handle: ContinuousControlHandle, visual: Visual, actions: Actions, double_click: Rc<Cell<bool>> }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display = Display { value: state(&self.handle, "value").unwrap_or(0.0),
            initial: state(&self.handle, "initialValue").unwrap_or(0.0),
            active: state::<String>(&self.handle, "phase").is_ok_and(|phase| phase != "idle"),
            focus_visible: state(&self.handle, "focusVisible").unwrap_or(false) };
        let visual = (self.visual)(display, self.actions.clone(), window, cx);
        let double = self.double_click.clone();
        crate::proto_parent::slot(&self.handle.owner, div().id("scalar-native-visual").size_full().on_mouse_down(MouseButton::Left,
            move |event: &MouseDownEvent, _, _| double.set(event.click_count >= 2)).child(visual).into_any_element())
    }
}
struct Mount { root: Entity<ContinuousControl>, drawing: Entity<Drawing>, target: Rc<RefCell<Target>>,
    geometry: Rc<RefCell<Geometry>>, lease: Rc<MoveGestureLease>, contact: Rc<Contact>, props: Value, _changes: Subscription }

#[derive(IntoElement)]
pub(crate) struct Scalar { editor: Entity<Editor>, key: SharedString, spec: Spec, geometry: Geometry, visual: Visual, callback: Callback, valid: Validator, focus_binding: Option<FocusBinding> }
pub(crate) fn scalar(editor: Entity<Editor>, key: impl Into<SharedString>, spec: Spec,
    geometry: impl Fn(&MoveGestureSample, Bounds<Pixels>, f64) -> Sample + 'static,
    visual: impl Fn(Display, Actions, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    callback: impl Fn(&mut Editor, Event, &mut Window, &mut Context<Editor>) + 'static) -> Scalar {
    Scalar { editor, key: key.into(), spec, geometry: Rc::new(geometry), visual: Rc::new(visual),
        callback: Rc::new(callback), valid: Rc::new(|_| true), focus_binding: None }
}
impl Scalar {
    /// Publish the compiler-owned focus target so virtualized owners can retain
    /// a stable item identity without installing a second native focus owner.
    pub fn on_focus_handle(mut self, bind: impl Fn(&gpui::FocusHandle, &mut Window, &mut App) + 'static) -> Self {
        self.focus_binding = Some(Rc::new(bind)); self
    }
    pub fn valid_when(mut self, valid: impl Fn(&Editor) -> bool + 'static) -> Self { self.valid = Rc::new(valid); self }
}
impl RenderOnce for Scalar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let editor = self.editor.read(cx);
        let token = json!({ "project": editor.project().project_id, "generation": editor.document_generation,
            "revision": editor.revision().get(), "control": self.key.to_string() }).to_string();
        let target = Target { editor: self.editor.downgrade(), generation: editor.document_generation,
            revision: editor.revision().get(), token: token.clone(), window: window.window_handle(), callback: self.callback, valid: self.valid,
            dialog: editor.modal_dialog.as_ref().map(std::mem::discriminant), mixer_preview: self.spec.mixer_preview };
        let spec = self.spec;
        let props = json!({ "value": spec.value, "min": spec.min, "max": spec.max, "step": spec.step,
            "fineStep": spec.fine_step, "defaultValue": spec.default_value, "label": spec.label,
            "disabled": spec.disabled, "revision": token, "role": spec.role, "dragMode": spec.drag_mode,
            "doubleAction": spec.double_action, "tapAction": spec.tap_action, "previewEcho": spec.preview_echo });
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let initial_props = props.clone();
        let initial_target = target.clone();
        let initial_visual = self.visual.clone();
        let initial_geometry = self.geometry.clone();
        let key = SharedString::from(format!("proto-scalar-{}", self.key));
        let mount = window.use_keyed_state(key, cx, move |window, cx| {
            let root = ContinuousControl::create(initial_props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled continuous control setup: {error:?}"));
            let handle = root.read(cx).handle();
            let target = Rc::new(RefCell::new(initial_target));
            let geometry = Rc::new(RefCell::new(initial_geometry));
            let contact = Rc::new(Contact::default());
            let double_click = Rc::new(Cell::new(false));
            let drawing = cx.new(|_| Drawing { handle: handle.clone(), visual: initial_visual,
                actions: Actions { handle: handle.clone(), target: target.clone() }, double_click: double_click.clone() });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled continuous control slot: {error:?}"));
            let lease = Rc::new(attach_move_gesture(handle.owner(), Box::new(Binding { handle,
                target: target.clone(), geometry: geometry.clone(), double_click, contact: contact.clone(), capture: RefCell::new(None),
                window: window.window_handle(), app: cx.to_async() })).expect("continuous native capture"));
            contact.window.set(Some(window.window_handle()));
            *contact.lease.borrow_mut() = Some(Rc::downgrade(&lease));
            let window_handle = window.window_handle();
            let changes = cx.observe(&root, move |mount: &mut Mount, _, cx| {
                mount.drawing.update(cx, |_, cx| cx.notify());
                let (handle, target, lease, contact) = (mount.root.read(cx).handle(), mount.target.borrow().clone(), mount.lease.clone(), mount.contact.clone());
                let expected = contact.generation.get();
                cx.defer(move |cx| { let _ = window_handle.update(cx, |_, window, cx| {
                    let good = drain(&handle, &target, window, cx);
                    if contact.is_current(expected) && (!good || state::<String>(&handle, "phase").is_ok_and(|phase| phase == "idle")) {
                        lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                    }
                }); });
            });
            Mount { root, drawing, target, geometry, lease, contact, props: initial_props, _changes: changes }
        });
        let root = mount.update(cx, |mount, cx| {
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx)).expect("compiled continuous parent");
            }
            *mount.geometry.borrow_mut() = self.geometry;
            if mount.props != props {
                match mount.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx)) {
                    Ok(()) => mount.props = props,
                    Err(error) => { report(&mount.target.borrow(), format!("连续控件属性更新失败：{error:?}"), cx); mount.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx); }
                }
                let handle = mount.root.read(cx).handle();
                drain(&handle, &mount.target.borrow(), window, cx);
                if mount.lease.is_active() && state::<String>(&handle, "phase").is_ok_and(|phase| phase == "idle") {
                    mount.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                }
            }
            *mount.target.borrow_mut() = target;
            mount.drawing.update(cx, |drawing, cx| { drawing.visual = self.visual; cx.notify(); });
            mount.root.clone()
        });
        if let Some(bind) = self.focus_binding {
            let focus = gpui::Focusable::focus_handle(root.read(cx), cx);
            bind(&focus, window, cx);
        }
        div().size_full().child(root)
    }
}
