//! Stateful Proto-UI knob with an Adwaita native drawing slot.
//!
//! Value, phase, cancellation, keyboard input, focus and accessibility belong
//! to the authored prototype. This host maps pointer displacement and plugin
//! units and submits a checked commit to the existing project/store boundary.

use std::{cell::{Cell, RefCell}, rc::Rc};

use gpui::{
    App, AppContext, Context, Entity, IntoElement, MouseButton, MouseDownEvent,
    Render, RenderOnce, SharedString, Subscription, WeakEntity, Window,
    canvas, div, prelude::*, px, quad,
};
use serde::Deserialize;
use serde_json::{Value, json};
use verdant_domain::TrackId;
use verdant_normalized_knob::{
    NormalizedKnob, NormalizedKnobHandle,
    native::{
        gpui_native_move::{
            MoveGestureBinding, MoveGestureCancelContext, MoveGestureCancelReason,
            MoveGestureInput, MoveGestureLease, MoveGestureSample, attach_move_gesture,
        },
        gpui_native_owner::NativeSnapshot,
    },
};
use verdant_plugins::{InsertKey, ParamDesc};
use verdant_ui_protocol::GesturePhase;

use crate::{
    devices::{format_value, from_norm, to_norm},
    editor::Editor,
    paint::{b, polyline},
    theme::Theme,
};

#[derive(Clone)]
struct Target {
    editor: WeakEntity<Editor>,
    track: TrackId,
    slot: usize,
    key: InsertKey,
    desc: ParamDesc,
}

impl Target {
    fn current_desc(&self, editor: &Editor) -> Option<ParamDesc> {
        // The device page belongs to the selected track. Selection can change
        // before GPUI paints and detaches this old subtree.
        if editor.session.track != self.track { return None; }
        let track = editor.project().track(&self.track)?;
        if self.slot == crate::instrument::INSTRUMENT_SLOT {
            if self.key != crate::instrument::instrument_key(&self.track) { return None; }
            // Instrument descriptors are derived from the project, not the
            // asynchronous effect-parameter cache, which has no instrument key.
            let params = crate::instrument::synth_params(track)?;
            return crate::instrument::synth_descs(&params).into_iter().find(|desc| desc.id == self.desc.id);
        }
        if editor.live_insert(&self.track, self.slot)?.key() != self.key { return None; }
        editor.plugin_params.get(&self.key)?.iter().find(|desc| desc.id == self.desc.id).cloned()
    }

    fn revision(&self, editor: &Editor, desc: &ParamDesc) -> String {
        // Source treats this as an opaque authority token. Include the live
        // parameter basis because a host parameter refresh can precede the
        // store revision, and a new document can reuse a revision number.
        json!({
            "project": editor.project().project_id,
            "revision": editor.revision().to_string(),
            "track": self.track, "slot": self.slot, "key": self.key.0,
            "parameter": desc.id, "value": desc.value,
            "min": desc.min, "max": desc.max, "steps": desc.step_count,
            "default": desc.default_value,
        }).to_string()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit { value: f64, revision: String }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cancellation { revision: String }

enum Output { Commit(Commit), Cancel(Cancellation) }

fn decode_events(events: Vec<(String, Value)>) -> Result<Vec<Output>, String> {
    events.into_iter().map(|(name, payload)| match name.as_str() {
        "valueCommit" => {
            let commit: Commit = serde_json::from_value(payload).map_err(|error| error.to_string())?;
            if !commit.value.is_finite() || !(0.0..=1.0).contains(&commit.value) {
                return Err("normalized knob returned an invalid value".into());
            }
            Ok(Output::Commit(commit))
        }
        "interactionCancel" => serde_json::from_value(payload).map(Output::Cancel).map_err(|error| error.to_string()),
        _ => Err(format!("normalized knob returned an unknown event or payload: {name}")),
    }).collect()
}

fn state<T: serde::de::DeserializeOwned>(handle: &NormalizedKnobHandle, name: &str) -> Result<T, String> {
    let value = handle.state(name).map_err(|error| format!("读取旋钮状态 {name} 失败：{error:?}"))?;
    serde_json::from_value(value).map_err(|error| format!("旋钮状态 {name} 不符合协议：{error}"))
}

fn report(target: &Target, message: impl Into<String>, cx: &mut App) {
    let message = message.into();
    let _ = target.editor.update(cx, |editor, cx| {
        editor.notice(message);
        cx.notify();
    });
}

/// Fully decode the event batch before any project mutation. The native
/// callback queue can outlive a props update; commits retain their source
/// revision so a late result cannot write into a different project state.
#[derive(Default)]
struct Drained { cancel: bool, failed: bool }

fn drain(handle: &NormalizedKnobHandle, target: &Target, cx: &mut App) -> Drained {
    let events = handle.take_events();
    let failures = handle.take_failures();
    if !failures.is_empty() {
        report(target, format!("旋钮交互失败，未提交修改：{failures:?}"), cx);
        return Drained { cancel: true, failed: true };
    }
    let outputs = match decode_events(events) {
        Ok(outputs) => outputs,
        Err(error) => {
            report(target, format!("旋钮协议失败，未提交修改：{error}"), cx);
            return Drained { cancel: true, failed: true };
        }
    };
    let mut cancel = false;
    for output in outputs {
        match output {
            Output::Cancel(Cancellation { revision }) => {
                // The source phase/generation is checked again before native
                // cancellation; an old event cannot retire a newer contact.
                let _basis = revision;
                cancel = true;
            }
            Output::Commit(commit) => {
                let accepted = target.editor.update(cx, |editor, cx| {
                    let current = target.current_desc(editor);
                    if current.as_ref().is_none_or(|desc| target.revision(editor, desc) != commit.revision)
                        || editor.plugin_busy.contains(&target.key) {
                        editor.notice("工程或参数已改变；已取消这次旋钮修改");
                        cx.notify();
                        return false;
                    }
                    let desc = current.expect("validated live parameter and slot identity");
                    if desc.min != target.desc.min || desc.max != target.desc.max
                        || desc.step_count != target.desc.step_count {
                        editor.notice("参数范围已改变；已取消这次旋钮修改");
                        cx.notify();
                        return false;
                    }
                    let value = from_norm(&desc, commit.value);
                    editor.set_track_insert_params(target.track.clone(), target.slot,
                        vec![(target.desc.id.clone(), value)], cx);
                    true
                });
                if !matches!(accepted, Ok(true)) { cancel = true; }
            }
        }
    }
    Drained { cancel, failed: false }
}

fn call(handle: &NormalizedKnobHandle, method: &str, arguments: Vec<Value>,
    window: &mut Window, cx: &mut App) -> Result<bool, String> {
    let value = handle.call(method, arguments, window, cx)
        .map_err(|error| format!("旋钮交互失败：{method}: {error:?}"))?;
    value.as_bool().ok_or_else(|| format!("旋钮协议失败：{method} 未返回布尔结果"))
}

#[derive(Default)]
struct Contact {
    generation: Cell<u64>,
    input: Cell<Option<MoveGestureInput>>,
}

impl Contact {
    fn begin(&self, input: MoveGestureInput) -> u64 {
        let next = self.generation.get().checked_add(1).expect("knob contact generation exhausted");
        self.generation.set(next);
        self.input.set(Some(input));
        next
    }

    fn is_current(&self, generation: u64) -> bool { self.generation.get() == generation }
}

struct Binding {
    handle: NormalizedKnobHandle,
    target: Rc<RefCell<Target>>,
    double_click: Rc<Cell<bool>>,
    contact: Rc<Contact>,
    window: gpui::AnyWindowHandle,
    app: gpui::AsyncApp,
}

fn sample_arguments(sample: &MoveGestureSample) -> Result<Vec<Value>, String> {
    if !sample.total_delta.x.is_finite() || !sample.total_delta.y.is_finite() {
        return Err("旋钮终止或移动事件包含无效坐标".into());
    }
    Ok(vec![(-sample.total_delta.y).into(),
        (sample.total_delta.x.hypot(sample.total_delta.y) >= 3.0).into(),
        sample.modifiers.shift.into()])
}

fn cancel_source(handle: &NormalizedKnobHandle, target: &Target, window: &mut Window, cx: &mut App) {
    match call(handle, "cancel", vec![], window, cx) {
        Ok(_) => { drain(handle, target, cx); }
        Err(error) => {
            // No partially emitted commit from a failed callback may escape.
            handle.take_events();
            handle.take_failures();
            report(target, error, cx);
        }
    }
}

impl Binding {
    fn invoke(&self, target: &Target, method: &str, arguments: Vec<Value>, window: &mut Window, cx: &mut App) -> bool {
        match call(&self.handle, method, arguments, window, cx) {
            Ok(consumed) => consumed,
            Err(error) => {
                self.contact.input.set(None);
                self.handle.take_events();
                self.handle.take_failures();
                report(target, error, cx);
                cancel_source(&self.handle, target, window, cx);
                false
            }
        }
    }

    fn drain(&self, target: &Target, window: &mut Window, cx: &mut App) {
        if drain(&self.handle, target, cx).failed {
            self.contact.input.set(None);
            cancel_source(&self.handle, target, window, cx);
        }
    }
}

fn defer_lease_cancel(lease: Rc<MoveGestureLease>, handle: NormalizedKnobHandle, target: Target,
    contact: Rc<Contact>, window: gpui::AnyWindowHandle, failed: bool, cx: &mut App) {
    let expected = contact.generation.get();
    cx.defer(move |cx| {
        if !contact.is_current(expected) { return; }
        let _ = window.update(cx, |_, window, cx| {
            if failed {
                contact.input.set(None);
                cancel_source(&handle, &target, window, cx);
                lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                return;
            }
            match state::<GesturePhase>(&handle, "phase") {
                Ok(GesturePhase::Idle) => lease.cancel(MoveGestureCancelReason::HostCancel, window, cx),
                Ok(_) => {}, // A new authored interaction already owns the phase.
                Err(error) => {
                    report(&target, error, cx);
                    lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                }
            }
        });
    });
}

impl MoveGestureBinding for Binding {
    fn should_start(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) -> bool {
        if !matches!(sample.input, MoveGestureInput::Mouse | MoveGestureInput::Touch) { return false; }
        let target = self.target.borrow();
        let available = target.editor.upgrade().is_some_and(|entity| {
            let editor = entity.read(cx);
            editor.device_drag.is_none() && editor.session.pointer_owner().is_none()
                && editor.lane_stroke.is_none() && editor.note_creation.is_none()
                && editor.mixer_drag.is_none() && editor.menu.is_none()
                && editor.text_prompt.is_none() && editor.modal_dialog.is_none()
                && !editor.plugin_busy.contains(&target.key) && target.current_desc(editor).is_some()
        });
        if !available { return false; }
        match state::<bool>(&self.handle, "disabled") {
            Ok(disabled) => !disabled,
            Err(error) => { report(&target, error, cx); false }
        }
    }

    fn on_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        let target = self.target.borrow().clone();
        self.contact.begin(sample.input);
        // A no-context cancellation may still be queued when a replacement
        // native contact starts. Retire its old preview synchronously before
        // beginning this generation; the queued callback must then be stale.
        match state::<GesturePhase>(&self.handle, "phase") {
            Ok(GesturePhase::Idle) => {},
            Ok(_) => cancel_source(&self.handle, &target, window, cx),
            Err(error) => {
                self.contact.input.set(None);
                report(&target, error, cx);
                cancel_source(&self.handle, &target, window, cx);
                return;
            }
        }
        let double = sample.input == MoveGestureInput::Mouse && self.double_click.replace(false);
        let consumed = self.invoke(&target, "begin",
            vec![sample.modifiers.alt.into(), double.into()], window, cx);
        match state::<GesturePhase>(&self.handle, "phase") {
            Ok(phase) => if !consumed || phase == GesturePhase::Idle { self.contact.input.set(None); },
            Err(error) => {
                self.contact.input.set(None);
                report(&target, error, cx);
                cancel_source(&self.handle, &target, window, cx);
            }
        }
        self.drain(&target, window, cx);
    }

    fn on_move(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        if self.contact.input.get() != Some(sample.input) { return; }
        let target = self.target.borrow().clone();
        match sample_arguments(sample) {
            Ok(arguments) => { self.invoke(&target, "sample", arguments, window, cx); }
            Err(error) => {
                self.contact.input.set(None);
                report(&target, error, cx);
                cancel_source(&self.handle, &target, window, cx);
            }
        }
        self.drain(&target, window, cx);
    }

    fn on_end(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        if self.contact.input.get() != Some(sample.input) { return; }
        self.contact.input.set(None);
        let target = self.target.borrow().clone();
        match sample_arguments(sample) {
            Ok(arguments) => { self.invoke(&target, "finish", arguments, window, cx); }
            Err(error) => {
                report(&target, error, cx);
                cancel_source(&self.handle, &target, window, cx);
            }
        }
        self.drain(&target, window, cx);
    }

    fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
        if self.contact.input.replace(None).is_none() { return; }
        let target = self.target.borrow().clone();
        if let Some(context) = context {
            cancel_source(&self.handle, &target, context.window, context.cx);
        } else {
            let (handle, window, contact) = (self.handle.clone(), self.window, self.contact.clone());
            let expected = contact.generation.get();
            self.app.spawn(async move |cx| {
                if !contact.is_current(expected) { return; }
                let _ = window.update(cx, |_, window, cx| {
                    if handle.owner().upgrade().is_some_and(|owner| !owner.borrow().disposed) {
                        cancel_source(&handle, &target, window, cx);
                    }
                });
            }).detach();
        }
    }
}

struct Mount {
    root: Entity<NormalizedKnob>,
    drawing: Entity<Drawing>,
    target: Rc<RefCell<Target>>,
    lease: Rc<MoveGestureLease>,
    contact: Rc<Contact>,
    props: Value,
    _changes: Subscription,
}

#[derive(IntoElement)]
pub(crate) struct Knob {
    editor: Entity<Editor>,
    track: TrackId,
    slot: usize,
    key: InsertKey,
    desc: ParamDesc,
    label: String,
    theme: Theme,
    size: f32,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn knob(editor: Entity<Editor>, track: TrackId, slot: usize, key: InsertKey,
    desc: ParamDesc, label: &str, theme: Theme, size: f32) -> Knob {
    Knob { editor, track, slot, key, desc, label: label.into(), theme, size }
}

impl RenderOnce for Knob {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let editor = self.editor.read(cx);
        let mut target = Target { editor: self.editor.downgrade(), track: self.track,
            slot: self.slot, key: self.key, desc: self.desc.clone() };
        let current = target.current_desc(editor);
        let disabled = current.is_none() || editor.plugin_busy.contains(&self.key) || editor.device_drag.is_some()
            || editor.menu.is_some() || editor.text_prompt.is_some() || editor.modal_dialog.is_some()
            || editor.session.pointer_owner().is_some() || editor.note_creation.is_some()
            || editor.lane_stroke.is_some() || editor.mixer_drag.is_some();
        let desc = current.unwrap_or(self.desc);
        let revision = target.revision(editor, &desc);
        target.desc = desc.clone();
        let props = json!({
            "value": to_norm(&desc, desc.value),
            "defaultValue": to_norm(&desc, desc.default_value.unwrap_or(desc.value)),
            "step": if desc.step_count > 0 { 1.0 / desc.step_count as f64 } else { 0.01 },
            "label": self.label,
            "disabled": disabled,
            "revision": revision,
        });
        let key = SharedString::from(format!("proto-knob-{}-{}-{}-{}", target.track.as_str(), self.key.0, self.slot, desc.id));
        let initial_target = target.clone();
        let initial_props = props.clone();
        let initial_label = self.label.clone();
        let initial_desc = desc.clone();
        let mount = window.use_keyed_state(key, cx, move |window, cx| {
            let root = NormalizedKnob::create(initial_props.clone(), None, window, cx)
                .unwrap_or_else(|error| panic!("compiled knob setup failed: {error:?}"));
            let handle = root.read(cx).handle();
            let double_click = Rc::new(Cell::new(false));
            let contact = Rc::new(Contact::default());
            let target = Rc::new(RefCell::new(initial_target));
            let drawing = cx.new(|_| Drawing { handle: handle.clone(), desc: initial_desc,
                label: initial_label, theme: self.theme, size: self.size, double_click: double_click.clone(),
                target: target.clone(), state_error: None });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled knob slot failed: {error:?}"));
            let lease = Rc::new(attach_move_gesture(handle.owner(), Box::new(Binding {
                handle, target: target.clone(), double_click, contact: contact.clone(),
                window: window.window_handle(), app: cx.to_async(),
            })).unwrap_or_else(|error| panic!("compiled knob pointer binding failed: {error}")));
            let window_handle = window.window_handle();
            let changes = cx.observe(&root, move |mount: &mut Mount, _, cx| {
                let handle = mount.root.read(cx).handle();
                let target = mount.target.borrow().clone();
                mount.drawing.update(cx, |_, cx| cx.notify());
                let output = drain(&handle, &target, cx);
                // A method can settle to idle while its event was drained in
                // the native callback. Inspect the source phase even then;
                // waiting for another event would leave capture until MouseUp.
                if output.failed || output.cancel || mount.lease.is_active() {
                    defer_lease_cancel(mount.lease.clone(), handle, target, mount.contact.clone(), window_handle, output.failed, cx);
                }
            });
            Mount { root, drawing, target, lease, contact, props: initial_props, _changes: changes }
        });
        mount.update(cx, |mount, cx| {
            *mount.target.borrow_mut() = target.clone();
            if mount.props != props {
                match mount.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx)) {
                    Ok(()) => mount.props = props,
                    Err(error) => {
                        report(&target, format!("旋钮属性更新失败：{error:?}"), cx);
                        mount.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                    }
                }
                if mount.lease.is_active() {
                    let handle = mount.root.read(cx).handle();
                    match state::<GesturePhase>(&handle, "phase") {
                        Ok(GesturePhase::Idle) => mount.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx),
                        Ok(_) => {},
                        Err(error) => {
                            report(&target, error, cx);
                            mount.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
                        }
                    }
                }
            }
            mount.drawing.update(cx, |drawing, cx| {
                if drawing.desc != desc || drawing.label != self.label || drawing.theme != self.theme || drawing.size != self.size {
                    drawing.desc = desc;
                    drawing.label = self.label;
                    drawing.theme = self.theme;
                    drawing.size = self.size;
                    cx.notify();
                }
            });
            mount.root.clone()
        })
    }
}

struct Drawing {
    handle: NormalizedKnobHandle,
    desc: ParamDesc,
    label: String,
    theme: Theme,
    size: f32,
    double_click: Rc<Cell<bool>>,
    target: Rc<RefCell<Target>>,
    state_error: Option<String>,
}

fn arc(cx: f64, cy: f64, radius: f64, from: f64, to: f64) -> Vec<Option<(f64, f64)>> {
    let steps = ((to - from).abs() / 4.0).ceil().max(1.0) as usize;
    (0..=steps).map(|index| {
        let angle = (from + (to - from) * index as f64 / steps as f64).to_radians();
        Some((cx + radius * angle.cos(), cy + radius * angle.sin()))
    }).collect()
}

impl Render for Drawing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display = (|| {
            let value: f64 = state(&self.handle, "value")?;
            let focused: bool = state(&self.handle, "focusVisible")?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("旋钮状态 value 不在有效范围内".to_string());
            }
            Ok((value, focused))
        })();
        let (n, focused) = match display {
            Ok(display) => { self.state_error = None; display }
            Err(error) => {
                if self.state_error.as_ref() != Some(&error) {
                    self.state_error = Some(error.clone());
                    let target = self.target.borrow().clone();
                    cx.defer(move |cx| report(&target, error, cx));
                }
                return div().w(px(self.size + 26.0)).text_xs().text_color(self.theme.text_secondary)
                    .child("参数暂不可用").into_any_element();
            }
        };
        let (desc, th, size, double_click) = (self.desc.clone(), self.theme, self.size, self.double_click.clone());
        let value_text = format_value(&desc, from_norm(&desc, n));
        div().flex().flex_col().items_center().gap(px(2.0)).w(px(size + 26.0))
            .child(canvas(|bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal), move |bounds, hitbox, window, _| {
                let o = bounds.origin;
                let side = f64::from(bounds.size.width.min(bounds.size.height));
                let (c, r, start, sweep) = (side / 2.0, side / 2.0 - 3.0, 135.0, 270.0);
                let zero = if desc.min < 0.0 && desc.max > 0.0 { to_norm(&desc, 0.0) } else { 0.0 };
                polyline(window, o, arc(c, c, r, start, start + sweep), 3.5, th.control_pressed);
                let (a, z) = if n < zero { (n, zero) } else { (zero, n) };
                if z - a > 0.002 { polyline(window, o, arc(c, c, r, start + sweep * a, start + sweep * z), 3.5, th.accent); }
                let body = r - 7.0;
                window.paint_quad(quad(b(c - body, c - body, body * 2.0, body * 2.0, o), px(body as f32), th.control,
                    px(if focused { 2.0 } else { 1.0 }), if focused { th.accent } else { th.shade }, gpui::BorderStyle::Solid));
                let angle = (start + sweep * n).to_radians();
                let (ix, iy) = (c + (body - 4.0) * angle.cos(), c + (body - 4.0) * angle.sin());
                window.paint_quad(quad(b(ix - 2.5, iy - 2.5, 5.0, 5.0, o), px(2.5), th.text, px(0.0), th.text, gpui::BorderStyle::Solid));
                if hitbox.is_hovered(window) { window.set_cursor_style(gpui::CursorStyle::ResizeUpDown, &hitbox); }
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _| {
                    if phase.capture() && event.button == MouseButton::Left && hitbox.is_hovered(window) {
                        double_click.set(event.click_count >= 2);
                    }
                });
            }).size(px(size)))
            .child(div().text_xs().text_color(th.text_secondary).child(self.label.clone()))
            .child(div().text_sm().text_color(th.text).whitespace_nowrap().child(value_text))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{FocusHandle, Focusable, TestAppContext, WindowHandle, size};
    use verdant_normalized_knob::native::gpui_native_owner::resolved;

    #[test]
    fn event_batch_rejects_unknown_fields_and_nonportable_values_before_commit() {
        assert!(decode_events(vec![("valueCommit".into(), json!({"value":0.5,"revision":"r1"})),
            ("unexpected".into(), json!({}))]).is_err());
        for payload in [json!({"value":1.2,"revision":"r1"}), json!({"value":0.5}),
            json!({"value":0.5,"revision":"r1","extra":true})] {
            assert!(decode_events(vec![("valueCommit".into(), payload)]).is_err());
        }
        for payload in [json!({}), json!({"revision":"r1","extra":true})] {
            assert!(decode_events(vec![("interactionCancel".into(), payload)]).is_err());
        }
        assert!(matches!(decode_events(vec![("interactionCancel".into(), json!({"revision":"r1"}))])
            .unwrap().as_slice(), [Output::Cancel(Cancellation { revision })] if revision == "r1"));
    }

    #[test]
    fn late_cancel_cannot_retire_a_new_contact_even_after_it_already_finished() {
        let contact = Contact::default();
        let old = contact.begin(MoveGestureInput::Mouse);
        contact.input.set(None);
        assert!(contact.is_current(old));
        let new = contact.begin(MoveGestureInput::Touch);
        contact.input.set(None);
        // A started boolean would be false in both cases and accept the stale
        // callback. The generation survives the newer terminal event.
        assert!(!contact.is_current(old));
        assert!(contact.is_current(new));
    }

    struct Slot;
    impl Render for Slot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size(px(64.0))
        }
    }

    struct Harness {
        knob: Entity<NormalizedKnob>,
        outside: FocusHandle,
        show: bool,
        lease: Option<MoveGestureLease>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().flex().flex_col().items_start()
                .when(self.show, |view| view.child(self.knob.clone()))
                .child(div().id("outside-knob").track_focus(&self.outside).size(px(20.0)))
        }
    }

    fn props(value: f64, revision: &str) -> Value {
        json!({"value":value,"defaultValue":0.25,"step":0.01,"label":"Cutoff","revision":revision})
    }

    fn frames(window: WindowHandle<Harness>, cx: &mut TestAppContext) {
        for _ in 0..3 {
            cx.update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            }).unwrap();
            cx.run_until_parked();
        }
    }

    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        open_with_pointer(cx, false)
    }

    // A native source consumer with the same sample mapper as the production
    // binding. Store/audio services are deliberately outside this harness;
    // emitted proposals remain observable for assertions before host acceptance.
    struct SourcePointer {
        handle: NormalizedKnobHandle,
        contact: Contact,
    }

    impl MoveGestureBinding for SourcePointer {
        fn should_start(&self, sample: &MoveGestureSample, _: &mut Window, _: &mut App) -> bool {
            matches!(sample.input, MoveGestureInput::Mouse | MoveGestureInput::Touch)
                && !state::<bool>(&self.handle, "disabled").unwrap()
        }
        fn on_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
            self.contact.begin(sample.input);
            assert!(call(&self.handle, "begin", vec![false.into(), false.into()], window, cx).unwrap());
        }
        fn on_move(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
            if self.contact.input.get() == Some(sample.input) {
                call(&self.handle, "sample", sample_arguments(sample).unwrap(), window, cx).unwrap();
            }
        }
        fn on_end(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
            if self.contact.input.get() == Some(sample.input) {
                self.contact.input.set(None);
                call(&self.handle, "finish", sample_arguments(sample).unwrap(), window, cx).unwrap();
            }
        }
        fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
            if self.contact.input.replace(None).is_none() { return; }
            if let Some(context) = context {
                call(&self.handle, "cancel", vec![], context.window, context.cx).unwrap();
            }
        }
    }

    fn open_with_pointer(cx: &mut TestAppContext, pointer: bool) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(240.0), px(180.0)), |window, cx| {
            let knob = NormalizedKnob::create(props(0.5, "r1"), None, window, cx).unwrap();
            let slot = cx.new(|_| Slot);
            knob.update(cx, |knob, cx| knob.set_slot(vec![NativeSnapshot::native_view(slot.into())], cx)).unwrap();
            let lease = pointer.then(|| {
                let handle = knob.read(cx).handle();
                attach_move_gesture(handle.owner(), Box::new(SourcePointer { handle, contact: Contact::default() })).unwrap()
            });
            Harness { knob, outside: cx.focus_handle(), show: true, lease }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        frames(window, cx);
        window
    }

    fn close_to(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "actual {actual}, expected {expected}");
    }

    fn only_commit(handle: &NormalizedKnobHandle, expected: f64, revision: &str) {
        assert!(handle.take_failures().is_empty());
        let outputs = decode_events(handle.take_events()).unwrap();
        assert!(matches!(outputs.as_slice(), [Output::Commit(commit)]
            if (commit.value - expected).abs() < 1e-12 && commit.revision == revision));
    }

    #[gpui::test]
    fn native_keyboard_commit_is_controlled_and_projects_slider_semantics(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| {
            let knob = host.knob.read(cx);
            let owner = knob.owner.borrow();
            assert!(owner.mounted && owner.accessible);
            assert_eq!(resolved(&owner.accessible_role).text(), "slider");
            assert!(owner.modules.a11y_node.is_some(), "the rendered GPUI semantics node is bound");
            close_to(resolved(&owner.accessible_states["valueNow"]).num(), 0.5);
            drop(owner);
            window.focus(&knob.focus_handle(cx), cx);
        }).unwrap();
        frames(window, cx);
        // Real GPUI keyboard routing exercises the compiled instance, including
        // portable key-name translation and native RunHandle semantics.
        cx.simulate_keystrokes(window.into(), "up");
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            only_commit(&handle, 0.51, "r1");
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);
            assert!(state::<bool>(&handle, "focused").unwrap());
            host.knob.update(cx, |knob, cx| knob.set_props(props(0.51, "r2"), window, cx)).unwrap();
            close_to(state::<f64>(&handle, "value").unwrap(), 0.51);
        }).unwrap();
        frames(window, cx);
        cx.simulate_keystrokes(window.into(), "shift-down");
        window.update(cx, |host, _, cx| {
            let handle = host.knob.read(cx).handle();
            only_commit(&handle, 0.509, "r2");
            close_to(state::<f64>(&handle, "value").unwrap(), 0.51);
            let owner = handle.owner.borrow();
            close_to(resolved(&owner.accessible_states["valueNow"]).num(), 0.51);
        }).unwrap();
    }

    #[gpui::test]
    fn native_final_sample_blur_and_subtree_detach_obey_the_source_lifecycle(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            assert!(call(&handle, "begin", vec![false.into(), false.into()], window, cx).unwrap());
            assert!(call(&handle, "sample", vec![18.0.into(), true.into(), false.into()], window, cx).unwrap());
            close_to(state::<f64>(&handle, "value").unwrap(), 0.6);
            assert!(call(&handle, "finish", vec![36.0.into(), true.into(), false.into()], window, cx).unwrap());
            only_commit(&handle, 0.7, "r1");
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);
            assert_eq!(state::<GesturePhase>(&handle, "phase").unwrap(), GesturePhase::Idle);
            assert!(call(&handle, "begin", vec![false.into(), false.into()], window, cx).unwrap());
            call(&handle, "sample", vec![36.0.into(), true.into(), false.into()], window, cx).unwrap();
        }).unwrap();
        // GPUI coalesces focus notifications within an app update. Let the
        // generated target receive its actual focus before moving it away.
        frames(window, cx);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            assert!(state::<bool>(&handle, "focused").unwrap());
            window.focus(&host.outside, cx);
        }).unwrap();
        frames(window, cx);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            assert_eq!(state::<GesturePhase>(&handle, "phase").unwrap(), GesturePhase::Idle);
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);
            assert!(matches!(decode_events(handle.take_events()).unwrap().as_slice(), [Output::Cancel(_)]));
            assert!(call(&handle, "begin", vec![false.into(), false.into()], window, cx).unwrap());
            call(&handle, "sample", vec![36.0.into(), true.into(), false.into()], window, cx).unwrap();
            host.show = false;
            cx.notify();
        }).unwrap();
        frames(window, cx);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            assert!(!handle.owner.borrow().mounted);
            assert_eq!(state::<GesturePhase>(&handle, "phase").unwrap(), GesturePhase::Idle);
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);
            assert!(matches!(decode_events(handle.take_events()).unwrap().as_slice(), [Output::Cancel(_)]));
            assert!(!call(&handle, "begin", vec![false.into(), false.into()], window, cx).unwrap());
            assert!(handle.take_failures().is_empty());
        }).unwrap();
    }

    #[gpui::test]
    fn native_drag_leaves_the_knob_and_uses_the_up_sample_but_window_exit_cancels(cx: &mut TestAppContext) {
        let window = open_with_pointer(cx, true);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            let bounds = handle.owner.borrow().bounds.unwrap();
            let start = bounds.origin + gpui::point(px(32.0), px(50.0));
            let moved = start + gpui::point(px(100.0), px(-18.0));
            let final_position = start + gpui::point(px(100.0), px(-36.0));
            assert!(bounds.contains(&start));
            assert!(!bounds.contains(&moved));
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position: start, button: MouseButton::Left, ..Default::default()
            }), cx);
            assert!(host.lease.as_ref().unwrap().is_active());
            window.dispatch_event(gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position: moved, pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            close_to(state::<f64>(&handle, "value").unwrap(), 0.6);
            assert_eq!(state::<GesturePhase>(&handle, "phase").unwrap(), GesturePhase::Dragging);
            assert!(handle.take_events().is_empty(), "leaving the element is not a cancellation");
            window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position: final_position, button: MouseButton::Left, ..Default::default()
            }), cx);
            only_commit(&handle, 0.7, "r1");
            assert!(!host.lease.as_ref().unwrap().is_active());
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);

            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position: start, button: MouseButton::Left, ..Default::default()
            }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position: moved, pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseExited(gpui::MouseExitEvent::default()), cx);
            assert!(!host.lease.as_ref().unwrap().is_active());
            assert_eq!(state::<GesturePhase>(&handle, "phase").unwrap(), GesturePhase::Idle);
            close_to(state::<f64>(&handle, "value").unwrap(), 0.5);
            assert!(matches!(decode_events(handle.take_events()).unwrap().as_slice(), [Output::Cancel(_)]));
            window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position: final_position, button: MouseButton::Left, ..Default::default()
            }), cx);
            assert!(handle.take_events().is_empty(), "late release after exit cannot commit");
        }).unwrap();
    }

    #[gpui::test]
    fn native_touch_owns_the_contact_until_its_terminal_sample(cx: &mut TestAppContext) {
        let window = open_with_pointer(cx, true);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            let start = handle.owner.borrow().bounds.unwrap().origin + gpui::point(px(32.0), px(50.0));
            let final_position = start + gpui::point(px(100.0), px(-36.0));
            let touch = |phase, position, window: &mut Window, cx: &mut App| {
                window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
                    id: gpui::TouchId(7), phase, position, ..Default::default()
                }), cx);
            };
            touch(gpui::TouchPhase::Started, start, window, cx);
            assert!(host.lease.as_ref().unwrap().is_active());
            window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position: final_position, button: MouseButton::Left, ..Default::default()
            }), cx);
            assert!(host.lease.as_ref().unwrap().is_active());
            assert!(handle.take_events().is_empty());
            touch(gpui::TouchPhase::Ended, final_position, window, cx);
            only_commit(&handle, 0.7, "r1");
            assert!(!host.lease.as_ref().unwrap().is_active());
        }).unwrap();
    }

    #[gpui::test]
    fn native_owner_disposal_cancels_active_lease_without_callback_failures(cx: &mut TestAppContext) {
        let window = open_with_pointer(cx, true);
        window.update(cx, |host, window, cx| {
            let handle = host.knob.read(cx).handle();
            let start = handle.owner.borrow().bounds.unwrap().origin + gpui::point(px(32.0), px(50.0));
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position: start, button: MouseButton::Left, ..Default::default()
            }), cx);
            assert!(host.lease.as_ref().unwrap().is_active());
            handle.dispose(window, cx).expect("native lifecycle disposal must remain legal");
            let failures = handle.take_failures();
            assert!(failures.is_empty(), "native disposal failures: {failures:?}");
            assert!(handle.owner.borrow().disposed);
            assert!(!host.lease.as_ref().unwrap().is_active());
            assert!(!handle.take_events().iter().any(|(event, _)| event == "valueCommit"));
        }).unwrap();
    }
}
