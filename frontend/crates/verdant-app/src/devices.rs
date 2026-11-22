//! Device view (bottom pane): the selected track's effect chain as device
//! cards from left to right, as in a DAW's device chain. Each built-in effect
//! has a graphical editor in the manner of dedicated mixing plug-ins: the
//! EQ's response curve with band handles (right-click for shape, slope and
//! stereo placement; double-click to add a band), transfer curves for the
//! dynamics with threshold handles, a decay-over-frequency graph for the
//! reverb and an echo graph for the delay. Other plug-ins get a knob card.
//! Displayed values glide to their new values after any external change.
//!
//! Stateful compiled GraphPoint and NormalizedKnob roots own interaction,
//! focus, keyboard and accessible actions. Native code retains the Adwaita
//! drawing, graph geometry, parameter conversion and atomic domain commit.

use gpui::{
    Context, DispatchPhase, Entity, Focusable, FontWeight, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Role, SharedString, Window, canvas, div, fill, prelude::*, px, quad, svg,
};
use verdant_domain::{Revision, TrackId};
use verdant_ui_protocol::GesturePhase;
#[cfg(test)]
use verdant_ui_protocol::{ControlEffect, GestureEvent, PlannerError, drag_normalized};
use std::rc::Rc;
use std::time::{Duration, Instant};
use verdant_plugins::builtin_dsp::{self as dsp, BandKind, EqParams, Placement};
use verdant_plugins::{InsertKey, ParamDesc};

use crate::assets::icon;
use crate::editor::Editor;
use crate::generated::controls::{self, CommandArg, CommandId};
use crate::paint::{Texts, b, flush_texts, p, polyline};
use crate::theme::{Theme, with_alpha};

mod suite;
mod value_entry;
mod signal;
pub(crate) mod workflow;
mod modulation;
mod analysis_tools;
mod graph_control;
mod presentation;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) enum DeviceTarget { Track(TrackId), Master }

impl From<TrackId> for DeviceTarget {
    fn from(track: TrackId) -> Self { Self::Track(track) }
}

impl DeviceTarget {
    pub(crate) fn track_id(&self) -> Option<&TrackId> {
        match self { Self::Track(track) => Some(track), Self::Master => None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeviceScope { SelectedTrack, NativeWindow }

impl DeviceScope {
    pub(crate) fn for_window(window: &Window) -> Self {
        if window.root::<crate::plugin_editor::NativePluginWindow>().flatten().is_some() { Self::NativeWindow }
        else { Self::SelectedTrack }
    }

    pub(crate) fn allows(self, owner: &DeviceTarget, editor: &Editor) -> bool {
        self == Self::NativeWindow || matches!(owner, DeviceTarget::Track(track) if *track == editor.session.track)
    }
}

/// Which control a drag edits.
#[derive(Clone, Debug, PartialEq)]
pub enum Control { Knob(String), Band(usize), Handle(&'static str), ParameterHandle(String) }

/// A pressed or dragged control. Values preview locally and are committed
/// to the plug-in once, on release.
pub struct DeviceDrag {
    track: DeviceTarget,
    slot: usize,
    key: InsertKey,
    control: Control,
    phase: GesturePhase,
    capture: Option<crate::host_pointer::OwnedPointerCapture>,
    surface: crate::host_pointer::SurfaceToken,
    basis: DeviceBasis,
    scope: DeviceScope,
    source: Option<graph_control::Retirement>,
    preview: Vec<(String, f64)>,
}

/// Keep modulation sources visible while the parameter area scrolls.
pub(crate) fn modulation_source_bar(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: Option<InsertKey>) -> gpui::AnyElement {
    key.map_or_else(|| div().into_any_element(), |key| modulation::docked_sources(ed, editor, &Target { track: owner.clone(), slot, key }))
}
pub(crate) fn toggle_modulation_panel(ed: &mut Editor, owner: DeviceTarget, slot: usize, key: InsertKey, cx: &mut Context<Editor>) {
    if !ed.device_target_matches(&owner, slot, key) { return; }
    let target = Target { track: owner, slot, key };
    let mut state = workflow::read_workspace(ed, &target); state.matrix_open = !state.matrix_open;
    workflow::write_workspace(ed, &target, &state); cx.notify();
}

// ---------------------------------------------------------------------------
// Value mapping and formatting
// ---------------------------------------------------------------------------

/// Wide positive ranges (Hz, Q, attack/release) are swept logarithmically.
pub fn is_log(d: &ParamDesc) -> bool { d.min > 0.0 && d.max / d.min >= 50.0 }

pub fn to_norm(d: &ParamDesc, v: f64) -> f64 {
    if d.max <= d.min { return 0.0; }
    let n = if is_log(d) { (v.max(d.min) / d.min).ln() / (d.max / d.min).ln() } else { (v - d.min) / (d.max - d.min) };
    n.clamp(0.0, 1.0)
}

pub fn from_norm(d: &ParamDesc, n: f64) -> f64 {
    let n = n.clamp(0.0, 1.0);
    let v = if is_log(d) { d.min * (d.max / d.min).powf(n) } else { d.min + n * (d.max - d.min) };
    if d.step_count > 0 && d.max > d.min {
        let step = (d.max - d.min) / d.step_count as f64;
        (d.min + ((v - d.min) / step).round() * step).clamp(d.min, d.max)
    } else {
        v.clamp(d.min, d.max)
    }
}

pub fn format_value(d: &ParamDesc, v: f64) -> String {
    // Filters at the end of their range are off, as FabFilter labels them.
    let id = d.id.as_str();
    if (matches!(id, "sc_hpf" | "low_cut") && v <= 20.5) || (matches!(id, "sc_lpf" | "high_cut") && v >= 19_999.0) { return "关".into(); }
    if d.unit == "dB" && matches!(id, "range" | "knee") { return format!("{v:.1} dB"); }
    match d.unit.as_str() {
        "Hz" if v >= 1000.0 => format!("{:.2} kHz", v / 1000.0),
        "Hz" if v < 10.0 => format!("{v:.2} Hz"),
        "Hz" => format!("{v:.0} Hz"),
        "dB" => format!("{v:+.1} dB"),
        "ms" if v < 1.0 => format!("{v:.2} ms"),
        "ms" if v < 10.0 => format!("{v:.1} ms"),
        "ms" => format!("{v:.0} ms"),
        "%" => format!("{v:.0} %"),
        ":1" => format!("{v:.1}:1"),
        "" if d.id.ends_with("_q") || d.id == "q" => format!("{v:.2}"),
        "s" => format!("{v:.2} s"),
        "st" if d.step_count == 0 => format!("{v:+.1} 半音"),
        "st" => format!("{v:+.0} 半音"),
        "ct" => format!("{v:.0} 音分"),
        "" if d.step_count == 1 => (if v >= 0.5 { "开" } else { "关" }).to_string(),
        "" if d.step_count > 1 => format!("{v:.0}"),
        unit => format!("{v:.2} {unit}"),
    }
}

/// Parameters of a live insert with the drag preview applied.
fn device_params(ed: &Editor, key: InsertKey) -> Option<Vec<ParamDesc>> {
    let mut params = ed.plugin_params.get(&key)?.clone();
    if let Some(drag) = ed.device_drag.as_ref().filter(|d| d.key == key) {
        for (id, value) in &drag.preview {
            if let Some(p) = params.iter_mut().find(|p| &p.id == id) { p.value = *value; }
        }
    }
    Some(params)
}

#[cfg(test)]
fn bounded_graph_value(d: &ParamDesc, axis: Axis, start: f64, delta: f64, travel: f64, fine: bool) -> Result<f64, PlannerError> {
    let value = axis.value(drag_normalized(start, delta, travel, fine)?).clamp(d.min, d.max);
    // Discrete graph handles obey the same native steps as knobs.
    Ok(from_norm(d, to_norm(d, value)))
}

fn param<'a>(params: &'a [ParamDesc], id: &str) -> Option<&'a ParamDesc> { params.iter().find(|p| p.id == id) }

fn current_params(ed: &Editor, target: &Target) -> Option<Vec<ParamDesc>> {
    if !ed.device_target_matches(&target.track, target.slot, target.key) { return None; }
    if target.slot == crate::instrument::INSTRUMENT_SLOT {
        let DeviceTarget::Track(track) = &target.track else { return None };
        let params = crate::instrument::synth_params(ed.project().track(track)?)?;
        let id = ed.project().track(track)?.instrument()?.get("id")?.as_str()?;
        return Some(crate::instrument::synth_descs_for(id, &params));
    }
    let handle = ed.live_device_insert(&target.track, target.slot)?;
    if handle.key() != target.key { return None; }
    ed.plugin_params.get(&target.key).cloned()
}

fn write_if_current(ed: &mut Editor, target: &Target, values: Vec<(String, f64)>, cx: &mut Context<Editor>) {
    if current_params(ed, target).is_some() {
        ed.set_device_params(target.track.clone(), target.slot, values, cx);
    }
}

fn target_for(ed: &Editor, owner: &DeviceTarget, slot: usize) -> Option<Target> {
    let key = if slot == crate::instrument::INSTRUMENT_SLOT {
        let DeviceTarget::Track(track) = owner else { return None };
        let id = ed.project().track(track)?.instrument()?.get("id")?.as_str()?;
        crate::instrument::instrument_key_for_generation(track, id, ed.document_generation)
    } else { ed.live_device_insert(owner, slot)?.key() };
    Some(Target { track: owner.clone(), slot, key })
}

// ---------------------------------------------------------------------------
// Gesture plumbing shared by knobs and graphs
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Target { track: DeviceTarget, slot: usize, key: InsertKey }

#[derive(Clone, PartialEq)]
struct DeviceBasis { project: String, generation: u64, revision: Revision, params: Vec<ParamDesc> }

impl Target {
    fn basis(&self, editor: &Editor) -> Option<DeviceBasis> {
        if editor.plugin_busy.contains(&self.key) || (self.slot != crate::instrument::INSTRUMENT_SLOT && editor.inserts_loading) { return None; }
        let params = editor.current_device_parameters(&self.track, self.slot, self.key)?;
        Some(DeviceBasis { project: editor.project().project_id.clone(), generation: editor.document_generation, revision: editor.revision(), params })
    }
}

impl Editor {
    /// Resolve the authoritative device slot, never an animated/preview value
    /// or a cache entry whose live insert was removed or replaced.
    pub(crate) fn current_device_parameters(&self, target: &DeviceTarget, slot: usize, key: InsertKey) -> Option<Vec<ParamDesc>> {
        current_params(self, &Target { track: target.clone(), slot, key })
    }

    /// Window notifications cannot cancel a capture acquired by another editor
    /// window. Project changes still use the unconditional cancellation path.
    pub(crate) fn cancel_device_drag_in(&mut self, window: gpui::AnyWindowHandle, cx: &mut Context<Self>) {
        if self.device_drag.as_ref().and_then(|drag| drag.capture.as_ref()).is_some_and(|capture| capture.window() == window) {
            self.cancel_device_drag(cx);
        }
    }

    pub(crate) fn has_proto_pointer_contact(&self) -> bool {
        self.proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade).is_some_and(|contact| contact.is_active())
    }

    /// Device previews never reach the plug-in until release. All cancellation
    /// paths retire the same owned capture, including failures and detachment.
    pub(crate) fn cancel_device_drag(&mut self, cx: &mut Context<Self>) {
        let Some(mut drag) = self.device_drag.take() else { return };
        if let Some(source)=drag.source.take(){source.cancel(cx);}
        if let Some(capture) = drag.capture.take() { capture.release_deferred(cx); }
        cx.notify();
    }

    fn cancel_device_drag_in_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let capture = self.device_drag.as_mut().and_then(|drag| drag.capture.take());
        self.cancel_device_drag(cx);
        if let Some(capture) = capture { capture.release(window, cx); }
    }
}

fn device_input_available(ed: &Editor) -> bool {
    ed.session.pointer_owner().is_none() && ed.lane_stroke.is_none()
        && ed.note_creation.is_none() && ed.mixer_drag.is_none() && ed.master_drag.is_none()
        && ed.menu.is_none() && ed.text_prompt.is_none() && ed.modal_dialog.is_none()
}

fn may_begin_device_input(ed: &Editor) -> bool {
    device_input_available(ed) && ed.device_drag.is_none() && !ed.has_proto_pointer_contact()
}

// ---------------------------------------------------------------------------
// Knob
// ---------------------------------------------------------------------------

/// A source-owned Proto-UI slider interaction with an Adwaita drawing slot.
/// The host retains plugin unit conversion and the domain commit boundary.
fn knob(editor: Entity<Editor>, target: Target, desc: ParamDesc, label: &str, th: Theme, size: f32) -> impl IntoElement {
    let (drop_editor, drop_target, drop_id) = (editor.clone(), target.clone(), desc.id.clone());
    div().id(SharedString::from(format!("knob-{}-{}", target.slot, desc.id)))
        .on_drop(move |source: &modulation::SourceDrag, window, cx| {
            drop_editor.update(cx, |ed, cx| {
                if DeviceScope::for_window(window).allows(&drop_target.track, ed) {
                    modulation::drop_source(ed, &drop_target, &drop_id, source, window, cx);
                }
            });
        })
        .child(crate::proto_knob::knob(editor, target.track, target.slot, target.key, desc, label, th, size))
}

pub(crate) fn open_parameter_value(editor: &Entity<Editor>, owner: DeviceTarget, slot: usize,
    key: InsertKey, desc: ParamDesc, cx: &mut gpui::App) {
    value_entry::open(editor, Target { track: owner, slot, key }, desc, cx);
}

pub(crate) type SemanticCommand = Rc<dyn Fn(Option<&crate::proto_controls::Activation>, &mut Window, &mut gpui::App)>;

/// Semantic activation must not synthesize pointer input. GPUI's default
/// accessible Click creates a MouseUp which can otherwise commit an unrelated
/// held preview before a toolbar command hides or changes its input surface.
pub(crate) fn semantic_command<T: 'static>(editor: gpui::WeakEntity<Editor>, actor: gpui::WeakEntity<T>,
    command: impl Fn(&mut T, Option<&crate::proto_controls::Activation>, &mut Window, &mut Context<T>) + 'static) -> SemanticCommand {
    Rc::new(move |event, window, cx| {
        // Native cancellation calls the authored prototype and can report an
        // error through Editor. Release the read borrow before invoking it.
        let contact = editor.upgrade().and_then(|editor| editor.read(cx).proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade));
        if let Some(contact) = contact { contact.cancel_in_window(window, cx); }
        let _ = editor.update(cx, |ed, cx| ed.cancel_device_drag_in(window.window_handle(), cx));
        let _ = actor.update(cx, |actor, cx| command(actor, event, window, cx));
    })
}

pub(crate) fn semantic_click(button: crate::proto_controls::Action, command: SemanticCommand) -> crate::proto_controls::Action {
    // The accepted event retains its keyboard/assistive origin and semantic
    // anchor; the compiled action never invents a pointer release for either.
    button.on_click(move |event, window, cx| command(Some(event), window, cx))
}

// ---------------------------------------------------------------------------
// Parameter animation
// ---------------------------------------------------------------------------

/// A device editor's displayed values easing toward the plug-in's values
/// after an external change (undo, a menu choice, a knob reset, automation
/// reads). Drags and their previews are shown directly.
pub struct DeviceAnim {
    from: Vec<f64>,
    to: Vec<f64>,
    start: Instant,
    selected_band: usize,
    signal_waveform: bool,
    comparison: Comparison,
    workspace_identity: Option<String>,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct Comparison {
    active: usize,
    slots: [Option<Vec<(String, f64)>>; 2],
    #[serde(skip)]
    pending: Option<usize>,
}

impl Comparison {
    /// A label follows the committed host snapshot, never a requested write.
    /// A rejected/rolled-back transaction keeps the previous active group.
    fn reconcile(&mut self, current: &[(String, f64)], busy: bool) {
        let Some(index) = self.pending else { return };
        if busy { return; }
        if self.slots[index].as_deref() == Some(current) { self.active = index; }
        self.pending = None;
    }
}

impl DeviceAnim {
    fn settled(target: Vec<f64>, now: Instant) -> Self {
        Self { from: target.clone(), to: target, start: now - ANIM, selected_band: 0, signal_waveform: false, comparison: Comparison::default(), workspace_identity: None }
    }
}

fn selected_band(ed: &Editor, key: InsertKey, count: usize) -> usize {
    ed.device_anim.get(&key).map_or(0, |state| state.selected_band).min(count.saturating_sub(1))
}

fn select_band(ed: &mut Editor, key: InsertKey, band: usize) {
    ed.device_anim.entry(key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now())).selected_band = band;
}

const ANIM: Duration = Duration::from_millis(240);

/// Ease-out cubic: quick to respond, gentle to settle (GNOME's default feel).
pub fn ease_out(t: f64) -> f64 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }

fn unstepped(d: &ParamDesc, n: f64) -> f64 {
    let n = n.clamp(0.0, 1.0);
    if is_log(d) { d.min * (d.max / d.min).powf(n) } else { d.min + n * (d.max - d.min) }
}

/// The displayed value `t` (0…1) of the way from `from` to `to`, eased in
/// the knob's own scale; stepped parameters switch at once.
pub fn interpolate(d: &ParamDesc, from: f64, to: f64, t: f64) -> f64 {
    if d.step_count > 0 || d.max <= d.min || t >= 1.0 { return to; }
    let (a, b) = (to_norm(d, from), to_norm(d, to));
    unstepped(d, a + (b - a) * ease_out(t))
}

fn animate(ed: &mut Editor, key: InsertKey, params: &mut [ParamDesc], snap: bool, window: &mut Window) {
    workflow::restore_and_reconcile(ed, key, params, snap);
    let target: Vec<f64> = params.iter().map(|p| p.value).collect();
    let now = Instant::now();
    let anim = ed.device_anim.entry(key).or_insert_with(|| DeviceAnim::settled(target.clone(), now));
    if snap || anim.to.len() != target.len() {
        anim.from = target.clone(); anim.to = target; anim.start = now - ANIM;
        return;
    }
    let mut t = now.duration_since(anim.start).as_secs_f64() / ANIM.as_secs_f64();
    if anim.to != target {
        let shown: Vec<f64> = params.iter().enumerate().map(|(i, d)| interpolate(d, anim.from[i], anim.to[i], t)).collect();
        anim.from = shown; anim.to = target; anim.start = now;
        t = 0.0;
    }
    if t < 1.0 {
        for (i, p) in params.iter_mut().enumerate() { p.value = interpolate(p, anim.from[i], anim.to[i], t); }
        window.request_animation_frame();
    }
}

// ---------------------------------------------------------------------------
// Graphs with draggable handles
// ---------------------------------------------------------------------------

/// A graph axis: value range and scale, independent of the parameter's own
/// range (a threshold handle can sit on a level axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis { pub min: f64, pub max: f64, pub log: bool, start: f64, end: f64 }

impl Axis {
    pub const fn lin(min: f64, max: f64) -> Self { Self { min, max, log: false, start: 0.0, end: 1.0 } }
    pub const fn log(min: f64, max: f64) -> Self { Self { min, max, log: true, start: 0.0, end: 1.0 } }
    pub const fn span(mut self, start: f64, end: f64) -> Self { self.start = start; self.end = end; self }
    pub fn norm(&self, v: f64) -> f64 {
        let n = if self.log { (v.max(self.min) / self.min).ln() / (self.max / self.min).ln() } else { (v - self.min) / (self.max - self.min) };
        self.start + n.clamp(0.0, 1.0) * (self.end - self.start)
    }
    pub fn value(&self, n: f64) -> f64 {
        let n = ((n - self.start) / (self.end - self.start)).clamp(0.0, 1.0);
        if self.log { self.min * (self.max / self.min).powf(n) } else { self.min + n * (self.max - self.min) }
    }
}

const GRAPH_PAD: f64 = 12.0;

/// Graph geometry: normalized coordinates inside the padded plot area.
#[derive(Clone, Copy)]
struct Plot { o: gpui::Point<Pixels>, w: f64, h: f64 }

impl Plot {
    fn x(&self, n: f64) -> f64 { GRAPH_PAD + n * (self.w - 2.0 * GRAPH_PAD) }
    fn y(&self, n: f64) -> f64 { self.h - GRAPH_PAD - n * (self.h - 2.0 * GRAPH_PAD) }
    fn inner_w(&self) -> f64 { self.w - 2.0 * GRAPH_PAD }
    fn inner_h(&self) -> f64 { self.h - 2.0 * GRAPH_PAD }
}

/// One draggable point. `at` is its normalized position; each axis that
/// can be dragged names the parameter it writes.
#[derive(Clone)]
struct Handle {
    control: Control,
    at: (f64, f64),
    x: Option<(String, Axis)>,
    y: Option<(String, Axis)>,
    color: gpui::Rgba,
    label: String,
    hollow: bool,
    selected: bool,
    /// Extra writes made by the drag (turning a band on).
    also: Vec<(String, f64)>,
    defaults: Vec<(String, f64)>,
}

type Painter = Rc<dyn Fn(&mut Window, Plot, &[ParamDesc], &mut Texts)>;
type HandleMenu = Rc<dyn Fn(&Editor, usize) -> Vec<crate::menu::MenuItem>>;
type DoubleClick = Rc<dyn Fn((f64, f64)) -> Option<(Vec<(String, f64)>, Option<usize>)>>;

fn paint_grid(window: &mut Window, plot: Plot, th: Theme, xs: &[(f64, String)], ys: &[(f64, String)], zero: Option<f64>, texts: &mut Texts) {
    let grid = with_alpha(th.text, 0.06);
    for (n, label) in xs {
        let x = plot.x(*n);
        window.paint_quad(fill(b(x, 0.0, 1.0, plot.h, plot.o), grid));
        let label_x = (x + 3.).min(plot.w - label.chars().count() as f64 * 6.5 - 5.).max(5.);
        texts.push((label.clone(), p(label_x, plot.h - 16.0, plot.o), 11.0, th.text_secondary));
    }
    for (n, label) in ys {
        let y = plot.y(*n);
        let strong = zero.is_some_and(|z| (z - n).abs() < 1e-9);
        window.paint_quad(fill(b(0.0, y, plot.w, 1.0, plot.o), if strong { with_alpha(th.text, 0.16) } else { grid }));
        texts.push((label.clone(), p(5.0, y - 14.0, plot.o), 11.0, th.text_secondary));
    }
}

fn fill_under(window: &mut Window, plot: Plot, pts: &[(f64, f64)], base: f64, color: gpui::Rgba) {
    if pts.len() < 2 { return; }
    let mut area = gpui::PathBuilder::fill();
    area.move_to(p(pts[0].0, base, plot.o));
    for &(x, y) in pts { area.line_to(p(x, y, plot.o)); }
    area.line_to(p(pts[pts.len() - 1].0, base, plot.o));
    area.close();
    if let Ok(path) = area.build() { window.paint_path(path, color); }
}

#[allow(clippy::too_many_arguments)]
fn graph(editor: Entity<Editor>, target: Target, params: Vec<ParamDesc>, name: &str, label: &str, th: Theme,
    painter: Painter, handles: Vec<Handle>, menu: Option<HandleMenu>, double: Option<DoubleClick>) -> gpui::Stateful<gpui::Div> {
    graph_control::graph(editor,target,params,name,label,th,painter,handles,menu,double)
}

// ---------------------------------------------------------------------------
// Shared widgets
// ---------------------------------------------------------------------------

/// A linked toggle group writing one stepped parameter.
fn choice(ed: &Editor, track: &DeviceTarget, slot: usize, params: &[ParamDesc], id: &str, labels: &[&str], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let current = param(params, id).map_or(0, |d| d.value.round() as usize);
    let target = target_for(ed, track, slot);
    if labels.len() > 8 {
        let rows: Vec<_> = labels.iter().map(|s| (*s).to_string()).collect();
        let label = rows.get(current).cloned().unwrap_or_default();
        let id = id.to_string();
        return dropdown_button(format!("choice-menu-{slot}-{id}").into(), label, th, move |_| {
            use crate::editor::MenuAction as M; use crate::menu::MenuItem as I;
            let Some(target) = &target else { return Vec::new() };
            rows.iter().enumerate().map(|(i, name)| I::action(name.clone(), M::SetDeviceParams {
                target: target.track.clone(), slot: target.slot, expected_key: Some(target.key), values: vec![(id.clone(), i as f64)] }).checked(i == current)).collect()
        }, cx).into_any_element();
    }
    div().flex().flex_wrap().p(gpui::rems(2.0/16.)).gap(gpui::rems(2.0/16.)).rounded(gpui::rems(8.0/16.)).bg(th.control).children(labels.iter().enumerate().map(|(i, label)| {
        let (target, id_owned) = (target.clone(), id.to_string());
        let on = i == current;
        let navigate: Vec<_> = (0..labels.len()).map(|index| parameter_command(target.clone(), vec![(id_owned.clone(), index as f64)], cx)).collect();
        let command = parameter_command(target, vec![(id_owned, i as f64)], cx);
        semantic_click(crate::proto_controls::action(SharedString::from(format!("choice-{slot}-{id}-{i}")), label.to_string()).role(Role::RadioButton).aria_selected(on).aria_label(label.to_string()).tab_index(0)
            .h(gpui::rems(26.0/16.)).px(gpui::rems(10.0/16.)).flex().items_center().rounded(gpui::rems(6.0/16.)).text_sm().cursor_pointer()
            .when(on, |d| d.bg(th.checked).font_weight(FontWeight::SEMIBOLD))
            .hover(move |s| s.bg(if on { th.checked } else { th.hover }))
            .active(move |s| s.bg(th.control_pressed))
            .child(label.to_string()), command)
            .navigation(i, labels.len(), move |index, window, cx| { if let Some(command) = navigate.get(index) { command(None, window, cx); } })
    })).into_any_element()
}

/// A switch row writing a 0/1 parameter.
fn toggle(ed: &Editor, track: &DeviceTarget, slot: usize, params: &[ParamDesc], id: &str, label: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let on = param(params, id).is_some_and(|d| d.value >= 0.5);
    let (target, id_owned) = (target_for(ed, track, slot), id.to_string());
    let command = parameter_command(target, vec![(id_owned, if on { 0.0 } else { 1.0 })], cx);
    semantic_click(crate::proto_controls::action(SharedString::from(format!("toggle-{slot}-{id}")), label.to_string()).role(Role::Switch).aria_label(label.to_string()).aria_selected(on).tab_index(0)
        .flex().items_center().gap(gpui::rems(8.0/16.)).cursor_pointer().text_sm()
        .child(label.to_string()).child(crate::view::switch(on, th)), command)
        .into_any_element()
}

fn parameter_command(target: Option<Target>, values: Vec<(String, f64)>, cx: &Context<Editor>) -> SemanticCommand {
    semantic_command(cx.weak_entity(), cx.weak_entity(), move |ed, _, window, cx| {
        let Some(target) = &target else { return };
        if may_begin_device_input(ed) && DeviceScope::for_window(window).allows(&target.track, ed) {
            write_if_current(ed, target, values.clone(), cx);
        }
    })
}

/// A dropdown-looking button that opens a menu (AdwComboRow's button).
pub(crate) fn dropdown_button(id: SharedString, label: String, th: Theme, items: impl Fn(&Editor) -> Vec<crate::menu::MenuItem> + 'static, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let command = semantic_command(cx.weak_entity(), cx.weak_entity(), move |ed, event, window, cx| {
        let position = event.map_or_else(|| window.mouse_position(), crate::proto_controls::Activation::position);
        ed.show_menu(position, items(ed), window, cx);
    });
    semantic_click(crate::proto_controls::action(id, label.clone()).role(Role::ComboBox).aria_label(label.clone()).tab_index(0).h(gpui::rems(30.0/16.)).px(gpui::rems(10.0/16.)).flex().items_center().gap(gpui::rems(6.0/16.))
        .rounded(gpui::rems(6.0/16.)).bg(th.control).hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(th.checked)).cursor_pointer().text_sm()
        .child(div().max_w(gpui::rems(120.0/16.)).truncate().child(label))
        .child(svg().path(icon("chevron-down")).size(gpui::rems(12.0/16.)).text_color(th.text_secondary)), command)
        .into_any_element()
}

/// Sidechain source picker: any other track's pre-insert signal.
fn sidechain_picker(ed: &Editor, target: &DeviceTarget, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let spec = ed.device_insert_specs(target).get(slot).cloned();
    let source = spec.as_ref().and_then(|s| verdant_service::project_audio::sidechain_source(s).map(str::to_string));
    let source_name = source.as_ref().and_then(|id| ed.project().tracks.iter().find(|t| t.id.as_str() == id).map(|t| t.name.clone()));
    let label = match (&source, &source_name) { (None, _) => "无".to_string(), (Some(_), Some(name)) => name.clone(), (Some(_), None) => "已移除的轨道".into() };
    let me = target.clone();
    let expected_key = ed.live_device_insert(target, slot).map(|handle| handle.key());
    let items = move |ed: &Editor| {
        use crate::editor::MenuAction as M;
        use crate::menu::MenuItem as I;
        let mut items = vec![I::header("侧链来源"), I::action("无（检测输入本身）", M::SetSidechain { target: me.clone(), slot, expected_key, source: None }).checked(source.is_none())];
        for t in ed.project().tracks.iter().filter(|t| !matches!(&me, DeviceTarget::Track(track) if t.id == *track)) {
            let on = source.as_deref() == Some(t.id.as_str());
            items.push(I::action(t.name.clone(), M::SetSidechain { target: me.clone(), slot, expected_key, source: Some(t.id.clone()) }).checked(on));
        }
        items
    };
    div().flex().items_center().gap(gpui::rems(8.0/16.))
        .child(svg().path(icon("git-merge")).size(gpui::rems(14.0/16.)).text_color(th.text_secondary))
        .child(div().text_sm().text_color(th.text_secondary).child("侧链"))
        .child(dropdown_button(SharedString::from(format!("sidechain-{slot}")), label, th, items, cx))
        .into_any_element()
}

fn knob_row(editor: &Entity<Editor>, target: impl Fn() -> Target, params: &[ParamDesc], ids: &[&str], th: Theme, size: f32) -> gpui::Div {
    let mut row = div().flex().flex_wrap().gap(gpui::rems(4.0/16.));
    for id in ids {
        if let Some(d) = param(params, id) {
            let label = d.name.clone();
            row = row.child(knob(editor.clone(), target(), d.clone(), &label, th, size));
        }
    }
    row
}

// ---------------------------------------------------------------------------
// EQ
// ---------------------------------------------------------------------------

/// GNOME palette colours for the six EQ bands.
const BAND_COLORS: [u32; 6] = [0x3584e4, 0x33d17a, 0xf6d32d, 0xff7800, 0xe66100, 0x9141ac];
fn band_color(i: usize) -> gpui::Rgba { gpui::rgb(BAND_COLORS[i % BAND_COLORS.len()]) }

const FREQ_AXIS: Axis = Axis::log(dsp::FREQ_MIN, dsp::FREQ_MAX);
const GAIN_AXIS: Axis = Axis::lin(-dsp::GAIN_RANGE, dsp::GAIN_RANGE);

fn freq_grid() -> Vec<(f64, String)> {
    [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0].iter()
        .map(|&f| (FREQ_AXIS.norm(f), if f >= 1000.0 { format!("{}k", f / 1000.0) } else { format!("{f}") })).collect()
}

fn eq_menu(track: DeviceTarget, slot: usize, key: InsertKey, band: usize, eq: &EqParams) -> Vec<crate::menu::MenuItem> {
    use crate::editor::MenuAction as M;
    use crate::menu::MenuItem as I;
    let b = eq.bands[band];
    let set = |field: &str, v: f64| M::SetDeviceParams { target: track.clone(), slot, expected_key: Some(key), values: vec![(dsp::eq_param_id(band, field), v)] };
    let mut items = vec![I::header(format!("频段 {}", band + 1))];
    for kind in BandKind::ALL { items.push(I::action(kind.label(), set("shape", kind.index() as f64)).checked(b.kind == kind)); }
    if b.kind.has_slope() && !eq.slope_continuous[band] {
        items.push(I::separator());
        for (i, slope) in dsp::SLOPES.iter().enumerate() { items.push(I::action(format!("{slope} dB/oct"), set("slope", i as f64)).checked(b.slope == i)); }
    }
    items.push(I::action("连续陡度（FIR）", set("slope_continuous", if eq.slope_continuous[band] { 0. } else { 1. })).checked(eq.slope_continuous[band]));
    items.push(I::separator());
    let places = Placement::ALL.iter().map(|&pl| I::action(pl.label(), set("place", pl.index() as f64)).checked(b.place == pl)).collect();
    items.push(I::submenu("立体声位置", places));
    items.push(I::separator());
    items.push(I::action(if b.on { "关闭频段" } else { "开启频段" }, set("on", if b.on { 0.0 } else { 1.0 })));
    let prefix = format!("b{}_", band + 1);
    let values = EqParams::default().descs().into_iter().filter(|p| p.id.starts_with(&prefix)).map(|p| (p.id, p.value)).collect();
    items.push(I::action("重置频段", M::SetDeviceParams { target: track.clone(), slot, expected_key: Some(key), values }));
    items
}

fn eq_band_tab(ed: &Editor, target: Target, index: usize, band: dsp::Band, selected: bool, cx: &Context<Editor>) -> crate::proto_controls::Action {
    let th = ed.theme;
    let command = suite::command(ed, target.clone(), suite::SuiteAction::SelectBand(index), cx);
    let navigate: Vec<_> = (0..dsp::EQ_BANDS).map(|index| suite::command(ed, target.clone(), suite::SuiteAction::SelectBand(index), cx)).collect();
    suite::command_button(crate::proto_controls::action(SharedString::from(format!("eq-band-{}-{index}", target.slot)), format!("频段 {}", index + 1))
        .role(Role::Tab).aria_selected(selected).tab_index(0).aria_label(format!("频段 {}：{}", index + 1, band.kind.label()))
        .h(gpui::rems(30.0/16.)).px(gpui::rems(8.0/16.)).flex().items_center().gap(gpui::rems(5.0/16.)).rounded(gpui::rems(6.0/16.)).text_sm().cursor_pointer()
        .border_1().border_color(if selected { with_alpha(band_color(index), 0.6) } else { th.shade }).bg(th.card)
        .when(selected, |d| d.bg(th.checked)).hover(move |s| s.bg(if selected { th.checked } else { th.hover })).active(move |s| s.bg(th.control_pressed))
        .child(div().size(gpui::rems(9.0/16.)).rounded(gpui::rems(4.5/16.)).bg(if band.on { band_color(index) } else { with_alpha(band_color(index), 0.3) }))
        .child(format!("{}", index + 1)), command)
        .navigation(index, dsp::EQ_BANDS, move |index, window, cx| { if let Some(command) = navigate.get(index) { command(None, window, cx); } })
}

fn eq_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let selected = selected_band(ed, key, dsp::EQ_BANDS);
    let eq = EqParams::from_descs(params);
    let target = || Target { track: track.clone(), slot, key };
    let band = eq.bands[selected];
    let measured = signal::snapshot(ed, track, slot, key);
    let all_bands = workflow::read_workspace(ed, &target()).expanded_sections.contains("eq-all-bands");

    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let eq = EqParams::from_descs(params);
        let ys: Vec<(f64, String)> = [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0].iter().map(|&db| (GAIN_AXIS.norm(db), format!("{db:+.0}"))).collect();
        signal::paint_spectrum(window, plot, measured.as_deref(), th, texts);
        paint_grid(window, plot, th, &freq_grid(), &ys, Some(0.5), texts);
        let samples = 240;
        let freq_at = |i: usize| FREQ_AXIS.value(i as f64 / samples as f64);
        let rate = verdant_plugins::SAMPLE_RATE;
        let band_rate = rate * if eq.phase == 1 && !eq.uses_fir() { 2. } else { 1. };
        // Each band's own curve, tinted, then the stereo sum on top.
        for (i, b) in eq.bands.iter().enumerate().filter(|(_, b)| b.on) {
            let pts: Vec<(f64, f64)> = (0..=samples).map(|k| { let f = freq_at(k); (plot.x(k as f64 / samples as f64), plot.y(GAIN_AXIS.norm(eq.band_response_db(i, f, rate)))) }).collect();
            if i == selected { fill_under(window, plot, &pts, plot.y(0.5), with_alpha(band_color(i), 0.16)); }
            polyline(window, plot.o, pts.iter().map(|&q| Some(q)), if i == selected { 1.5 } else { 1.0 }, with_alpha(band_color(i), if i == selected { 0.85 } else { 0.4 }));
            if eq.dynamics[i].enabled && b.kind.has_gain() {
                let mut limit = *b;
                limit.gain_db += eq.dynamics[i].range_db;
                let edge: Vec<_> = (0..=samples).map(|k| (plot.x(k as f64 / samples as f64), plot.y(GAIN_AXIS.norm(dsp::band_response_db_with_slope(&limit, freq_at(k), band_rate, eq.slope_continuous[i].then_some(eq.effective_slope(i))))))).collect();
                polyline(window, plot.o, edge.iter().enumerate().map(|(k, &point)| if k % 8 < 5 { Some(point) } else { None }), 1.25, with_alpha(band_color(i), 0.65));
            }
        }
        let auto_gain = eq.auto_gain_db(rate);
        let curve: Vec<(f64, f64)> = (0..=samples).map(|k| { let f = freq_at(k); (plot.x(k as f64 / samples as f64), plot.y(GAIN_AXIS.norm(eq.response_db(f, rate) + auto_gain))) }).collect();
        fill_under(window, plot, &curve, plot.y(0.5), with_alpha(th.accent, 0.10));
        polyline(window, plot.o, curve.iter().map(|&q| Some(q)), 2.25, th.accent);
    });
    let handle_bands: Vec<usize> = eq.bands.iter().enumerate().filter(|(i, band)| band.on || *i == selected).map(|(i, _)| i).collect();
    let handles: Vec<Handle> = handle_bands.iter().map(|&i| {
        let b = &eq.bands[i];
        let gain = b.kind.has_gain().then(|| (dsp::eq_param_id(i, "gain"), GAIN_AXIS));
        let (kind, f0, q0, on0) = dsp::EQ_LAYOUT[i];
        Handle {
            control: Control::Band(i),
            at: (FREQ_AXIS.norm(b.freq), GAIN_AXIS.norm(if b.kind.has_gain() { b.gain_db } else { 0.0 })),
            x: Some((dsp::eq_param_id(i, "freq"), FREQ_AXIS)), y: gain,
            color: band_color(i), label: format!("{}{}", i + 1, b.place.short()), hollow: !b.on, selected: i == selected,
            // Dragging a band that is off turns it on, as in parametric EQ editors.
            also: if b.on { vec![] } else { vec![(dsp::eq_param_id(i, "on"), 1.0)] },
            defaults: vec![(dsp::eq_param_id(i, "freq"), f0), (dsp::eq_param_id(i, "gain"), 0.0), (dsp::eq_param_id(i, "q"), q0),
                (dsp::eq_param_id(i, "on"), if on0 { 1.0 } else { 0.0 }), (dsp::eq_param_id(i, "shape"), kind.index() as f64)],
        }
    }).collect();
    let menu_track = track.clone();
    let menu: HandleMenu = Rc::new(move |ed, i| {
        let params = ed.live_device_insert(&menu_track, slot).and_then(|h| ed.plugin_params.get(&h.key()).cloned()).unwrap_or_default();
        eq_menu(menu_track.clone(), slot, key, handle_bands[i], &EqParams::from_descs(&params))
    });
    // Double-click empty space: bring in the first band that is off, there.
    let bands_now = eq.bands;
    let double: DoubleClick = Rc::new(move |(nx, ny)| {
        let i = bands_now.iter().position(|b| !b.on)?;
        let freq = FREQ_AXIS.value(nx);
        let kind = if freq < 60.0 { BandKind::LowCut } else if freq > 12_000.0 { BandKind::HighCut } else { BandKind::Bell };
        let mut writes = vec![(dsp::eq_param_id(i, "on"), 1.0), (dsp::eq_param_id(i, "freq"), freq), (dsp::eq_param_id(i, "shape"), kind.index() as f64)];
        if kind.has_gain() { writes.push((dsp::eq_param_id(i, "gain"), GAIN_AXIS.value(ny))); }
        Some((writes, Some(i)))
    });

    let mut bands = div().id(format!("eq-band-tabs-{slot}")).role(Role::TabList).aria_label("均衡器频段").flex().flex_wrap().flex_none().gap(gpui::rems(4.0/16.)).children((0..dsp::EQ_BANDS).filter(|&i| all_bands || eq.bands[i].on || i == selected).map(|i| {
        eq_band_tab(ed, target(), i, eq.bands[i], i == selected, cx)
    }));
    if let Some(next) = eq.bands.iter().position(|band| !band.on) {
        let add_target = target();
        let command = semantic_command(cx.weak_entity(), cx.weak_entity(), move |ed, _, window, cx| {
            if !may_begin_device_input(ed) || !DeviceScope::for_window(window).allows(&add_target.track, ed)
                || !ed.device_target_matches(&add_target.track, add_target.slot, add_target.key) { return; }
            select_band(ed, key, next);
            write_if_current(ed, &add_target, vec![(dsp::eq_param_id(next, "on"), 1.)], cx);
        });
        bands = bands.child(semantic_click(presentation::button(format!("eq-add-band-{slot}"), "＋ 频段", false, th)
            .aria_label("启用下一个空闲均衡频段"), command));
    }
    bands = bands.child(div().flex_1()).child(semantic_click(
        presentation::button(format!("eq-all-bands-{slot}"), if all_bands { "仅显示当前" } else { "全部 24 段" }, all_bands, th),
        presentation::section_command(target(), "eq-all-bands".into(), cx)));
    let shape_track = track.clone();
    let shape = dropdown_button(SharedString::from(format!("eq-shape-{slot}")), band.kind.label().into(), th, move |ed| {
        let params = ed.live_device_insert(&shape_track, slot).and_then(|h| ed.plugin_params.get(&h.key()).cloned()).unwrap_or_default();
        eq_menu(shape_track.clone(), slot, key, selected, &EqParams::from_descs(&params))
    }, cx);
    let band_switch = toggle(ed, track, slot, params, &dsp::eq_param_id(selected, "on"), "启用频段", cx);
    let place_labels: Vec<&str> = Placement::ALL.iter().map(|p| match p { Placement::Stereo => "立体", other => other.short() }).collect();
    let mut knobs = div().flex().items_start().gap(gpui::rems(4.0/16.));
    for (field, label) in [("freq", "频率"), ("gain", "增益"), ("q", "Q")] {
        if field == "gain" && !band.kind.has_gain() { continue; }
        if let Some(d) = param(params, &dsp::eq_param_id(selected, field)) { knobs = knobs.child(knob(editor.clone(), target(), d.clone(), label, th, 52.0)); }
    }
    if let Some(d) = param(params, "output") { knobs = knobs.child(div().w(gpui::rems(12.0/16.))).child(knob(editor.clone(), target(), d.clone(), "输出", th, 52.0)); }
    let mut controls_row = div().flex().flex_wrap().items_center().gap(gpui::rems(10.0/16.)).child(shape);
    if band.kind.has_slope() && !eq.slope_continuous[selected] {
        let slope_labels: Vec<String> = dsp::SLOPES.iter().map(|s| format!("{s}")).collect();
        let refs: Vec<&str> = slope_labels.iter().map(String::as_str).collect();
        controls_row = controls_row.child(choice(ed, track, slot, params, &dsp::eq_param_id(selected, "slope"), &refs, cx));
    }
    controls_row = controls_row.child(band_switch)
        .child(toggle(ed, track, slot, params, &dsp::eq_param_id(selected, "slope_continuous"), "连续陡度", cx));
    if eq.slope_continuous[selected] {
        if let Some(desc) = param(params, &dsp::eq_param_id(selected, "slope_db")) {
            let mut desc = desc.clone(); desc.min = band.kind.min_slope();
            desc.value = eq.effective_slope(selected);
            desc.default_value = desc.default_value.map(|value| value.clamp(desc.min, desc.max));
            knobs = knobs.child(knob(editor.clone(), target(), desc, "陡度 dB/oct", th, 52.));
        }
        controls_row = controls_row.child(div().text_xs().text_color(th.text_secondary)
            .child(format!("固定延迟 · 最低 {:.0} dB/oct", band.kind.min_slope())));
    }
    div().flex().flex_col().gap(gpui::rems(10.0/16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "eq-graph", "均衡器频率响应：拖动频段点调频率和增益，右键换形状，双击空白处添加频段", th, painter, handles, Some(menu), Some(double)).h(gpui::rems(320.0/16.)).w_full())
        .child(bands)
        .child(div().flex().flex_col().gap(gpui::rems(12./16.)).p(gpui::rems(12./16.)).rounded(gpui::rems(10./16.)).bg(th.card).border_1().border_color(th.shade)
            .child(controls_row)
            .child(div().flex().flex_wrap().items_center().gap(gpui::rems(12.0/16.)).child(knobs).child(div().flex_1())
            .child(div().flex().flex_col().gap(gpui::rems(4.0/16.)).items_end()
                .child(div().text_xs().text_color(th.text_secondary).child("立体声位置"))
                .child(choice(ed, track, slot, params, &dsp::eq_param_id(selected, "place"), &place_labels, cx)))))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Dynamics
// ---------------------------------------------------------------------------

const LEVEL_FLOOR: f64 = -60.0;

/// A static level transfer graph (input → output dBFS) with a threshold handle.
fn transfer_graph(editor: &Entity<Editor>, target: Target, params: &[ParamDesc], name: &str, label: &str, th: Theme,
    curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64>, floor: f64) -> gpui::AnyElement {
    let axis = Axis::lin(floor, 12.0);
    let threshold = param(params, "threshold").map_or(-18.0, |d| d.value);
    let c = curve.clone();
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        texts.push(("静态输入 → 输出 · dBFS".into(), p(12., 9., plot.o), 11., th.text_secondary));
        let grid: Vec<(f64, String)> = (1..4).map(|k| { let db = floor * k as f64 / 4.0; (axis.norm(db), format!("{db:.0}")) }).collect();
        paint_grid(window, plot, th, &grid, &grid, None, texts);
        polyline(window, plot.o, [Some((plot.x(0.0), plot.y(0.0))), Some((plot.x(1.0), plot.y(1.0)))], 1.0, with_alpha(th.text, 0.18));
        let t = param(params, "threshold").map_or(-18.0, |d| d.value);
        let knee = param(params, "knee").map_or(0.0, |d| d.value);
        if knee > 0.0 {
            let (k0, k1) = (plot.x(axis.norm(t - knee / 2.0)), plot.x(axis.norm(t + knee / 2.0)));
            window.paint_quad(fill(b(k0, 0.0, (k1 - k0).max(1.0), plot.h, plot.o), with_alpha(th.accent, 0.08)));
        }
        let pts: Vec<(f64, f64)> = (0..=160).map(|i| { let x = axis.value(i as f64 / 160.0); (plot.x(axis.norm(x)), plot.y(axis.norm(c(params, x)))) }).collect();
        fill_under(window, plot, &pts, plot.h, with_alpha(th.accent, 0.10));
        polyline(window, plot.o, pts.iter().map(|&q| Some(q)), 2.25, th.accent);
        window.paint_quad(fill(b(plot.x(axis.norm(t)), 0.0, 1.0, plot.h, plot.o), with_alpha(th.accent, 0.4)));
    });
    let default = param(params, "threshold").and_then(|d| d.default_value).unwrap_or(threshold);
    let handle = Handle {
        control: Control::Handle("threshold"), at: (axis.norm(threshold), axis.norm(curve(params, threshold))),
        x: Some(("threshold".into(), axis)), y: None, color: gpui::rgb(0xffffff), label: String::new(), hollow: false, selected: false,
        also: vec![], defaults: vec![("threshold".into(), default)],
    };
    graph(editor.clone(), target, params.to_vec(), name, label, th, painter, vec![handle], None, None).size(gpui::rems(260.0/16.)).flex_none().into_any_element()
}

fn comp_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64> = Rc::new(|ps, x| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        let curve = |input| dsp::compressor_style_curve_db(v("style", 0.).round() as usize, v("threshold", -18.), v("ratio", 3.), v("knee", 6.), v("range", 60.), input);
        let input = x + v("input", 0.);
        let auto = if v("auto_gain", 0.) >= 0.5 { (-curve(-12.) - 12.).clamp(-24., 24.) } else { 0. };
        let wet = v("mix", 100.) / 100.;
        dsp::lin_to_db(dsp::db_to_lin(input) * (1. - wet) + dsp::db_to_lin(curve(input) + v("makeup", 0.) + auto) * wet) + v("output", 0.)
    });
    div().flex().flex_col().gap(gpui::rems(14.0/16.))
        .child(div().flex().flex_wrap().gap(gpui::rems(12./16.))
            .child(transfer_graph(editor, target(), params, "comp-graph", "压缩器传输曲线：左右拖动阈值", th, curve, LEVEL_FLOOR))
            .child(div().flex_1().min_w(gpui::rems(240./16.)).child(signal::history(ed, editor, target(), cx))))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(gpui::rems(10.0/16.))
            .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10.0/16.)).child(choice(ed, track, slot, params, "style", &dsp::COMP_STYLES, cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
            .child(knob_row(editor, target, params, &["threshold", "ratio", "knee", "range", "makeup"], th, 48.0))
            .child(knob_row(editor, target, params, &["attack", "release", "lookahead", "mix", "sc_hpf", "sc_lpf"], th, 48.0))
            .child(toggle(ed, track, slot, params, "sc_listen", "监听侧链", cx)))
        .into_any_element()
}

fn gate_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64> = Rc::new(|ps, x| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        let processed = dsp::gate_style_curve_db(v("style", 0.).round() as usize, v("threshold", -40.), v("ratio", 50.), v("range", 40.), v("knee", 0.), x);
        let wet = v("mix", 100.) / 100.;
        dsp::lin_to_db(dsp::db_to_lin(x) * (1. - wet) + dsp::db_to_lin(processed) * wet) + v("output", 0.)
    });
    div().flex().flex_col().gap(gpui::rems(14.0/16.))
        .child(div().flex().flex_wrap().gap(gpui::rems(12./16.))
            .child(transfer_graph(editor, target(), params, "gate-graph", "门限传输曲线：左右拖动阈值", th, curve, -80.0))
            .child(div().flex_1().min_w(gpui::rems(240./16.)).child(signal::history(ed, editor, target(), cx))))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(gpui::rems(10.0/16.))
            .child(div().flex().items_center().gap(gpui::rems(10.0/16.)).child(toggle(ed, track, slot, params, "sc_listen", "监听侧链", cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
            .child(knob_row(editor, target, params, &["threshold", "range", "ratio", "mix"], th, 48.0))
            .child(knob_row(editor, target, params, &["attack", "hold", "release", "lookahead", "sc_hpf", "sc_lpf"], th, 48.0)))
        .into_any_element()
}

fn deesser_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let level = Axis::lin(-60.0, 0.0);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let ys: Vec<(f64, String)> = [-48.0, -36.0, -24.0, -12.0].iter().map(|&db| (level.norm(db), format!("{db:.0}"))).collect();
        paint_grid(window, plot, th, &freq_grid(), &ys, None, texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let (freq, threshold, range) = (v("freq", 6000.0), v("threshold", -24.0), v("range", 12.0));
        // The detected band: shaded from the crossover up.
        let x = plot.x(FREQ_AXIS.norm(freq));
        window.paint_quad(fill(b(x, 0.0, plot.w - x, plot.h, plot.o), with_alpha(th.accent, 0.10)));
        window.paint_quad(fill(b(x, 0.0, 1.0, plot.h, plot.o), with_alpha(th.accent, 0.45)));
        let y = plot.y(level.norm(threshold));
        polyline(window, plot.o, [Some((x, y)), Some((plot.w, y))], 1.5, th.accent);
        // The deepest reduction, dashed below the threshold.
        let floor = plot.y(level.norm(threshold - range));
        let mut sx = x;
        while sx < plot.w { polyline(window, plot.o, [Some((sx, floor)), Some(((sx + 6.0).min(plot.w), floor))], 1.0, with_alpha(th.accent, 0.5)); sx += 10.0; }
    });
    let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
    let handle = Handle {
        control: Control::Handle("freq-threshold"), at: (FREQ_AXIS.norm(v("freq", 6000.0)), level.norm(v("threshold", -24.0))),
        x: Some(("freq".into(), FREQ_AXIS)), y: Some(("threshold".into(), level)), color: gpui::rgb(0xffffff), label: String::new(),
        hollow: false, selected: false, also: vec![], defaults: vec![("freq".into(), 6000.0), ("threshold".into(), -24.0)],
    };
    div().flex().flex_col().gap(gpui::rems(10.0/16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "deesser-graph", "齿音检测：左右拖动频率，上下拖动阈值", th, painter, vec![handle], None, None).h(gpui::rems(290.0/16.)).w_full())
        .child(div().flex().items_center().gap(gpui::rems(10.0/16.)).child(choice(ed, track, slot, params, "mode", &dsp::DEESSER_MODES, cx))
            .child(toggle(ed, track, slot, params, "sc_listen", "监听", cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
        .child(knob_row(editor, target, params, &["threshold", "range", "freq", "release"], th, 42.0))
        .into_any_element()
}

fn limiter_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let axis = Axis::lin(-36.0, 0.0);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let grid: Vec<(f64, String)> = [-24.0, -12.0].iter().map(|&db| (axis.norm(db), format!("{db:.0}"))).collect();
        paint_grid(window, plot, th, &grid, &grid, None, texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let (gain, ceiling) = (v("gain", 0.0), v("ceiling", -0.3));
        polyline(window, plot.o, [Some((plot.x(0.0), plot.y(0.0))), Some((plot.x(1.0), plot.y(1.0)))], 1.0, with_alpha(th.text, 0.18));
        let pts: Vec<(f64, f64)> = (0..=120).map(|i| { let x = -36.0 + i as f64 * 0.3; (plot.x(axis.norm(x)), plot.y(axis.norm((x + gain).min(ceiling)))) }).collect();
        fill_under(window, plot, &pts, plot.h, with_alpha(th.accent, 0.10));
        polyline(window, plot.o, pts.iter().map(|&q| Some(q)), 2.25, th.accent);
        let y = plot.y(axis.norm(ceiling));
        polyline(window, plot.o, [Some((0.0, y)), Some((plot.w, y))], 1.0, with_alpha(th.error, 0.7));
        texts.push(("上限".into(), p(plot.w - 34.0, y + 2.0, plot.o), 10.0, th.text_secondary));
    });
    let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
    let handle = Handle {
        control: Control::Handle("ceiling"), at: (1.0, axis.norm(v("ceiling", -0.3))), x: None, y: Some(("ceiling".into(), axis)),
        color: gpui::rgb(0xffffff), label: String::new(), hollow: false, selected: false, also: vec![], defaults: vec![("ceiling".into(), -0.3)],
    };
    let mut handle = handle;
    // The ceiling axis covers −12…0 dB; keep the handle on the drawn line.
    handle.at.0 = 0.92;
    div().flex().flex_col().gap(gpui::rems(14.0/16.))
        .child(div().flex().flex_wrap().gap(gpui::rems(12./16.))
            .child(graph(editor.clone(), target(), params.to_vec(), "limiter-graph", "限制器传输曲线：上下拖动输出上限", th, painter, vec![handle], None, None).size(gpui::rems(260.0/16.)).flex_none())
            .child(div().flex_1().min_w(gpui::rems(240./16.)).child(signal::history(ed, editor, target(), cx))))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(gpui::rems(10.0/16.))
            .child(knob_row(editor, target, params, &["gain", "ceiling", "release"], th, 54.0))
            .child(knob_row(editor, target, params, &["lookahead", "link"], th, 54.0)))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Space and color
// ---------------------------------------------------------------------------

fn reverb_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], advanced: bool, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let decay_axis = Axis::log(0.02, 320.0);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let ys: Vec<_> = [0.1, 1.0, 10.0, 100.0].iter().map(|&s| (decay_axis.norm(s), format!("{s} s"))).collect();
        paint_grid(window, plot, th, &freq_grid(), &ys, None, texts);
        let values = suite::values(dsp::REVERB_SPECS, params);
        let freeze = param(params, "freeze").is_some_and(|p| p.value >= 0.5);
        let points: Vec<_> = (0..=240).map(|i| {
            let n = i as f64 / 240.0;
            let seconds = dsp::reverb_decay_at_frequency(&values, FREQ_AXIS.value(n), verdant_plugins::SAMPLE_RATE);
            (plot.x(n), plot.y(decay_axis.norm(seconds)))
        }).collect();
        fill_under(window, plot, &points, plot.y(0.), with_alpha(th.accent, 0.12));
        polyline(window, plot.o, points.into_iter().map(Some), 2.25, th.accent);
        if !freeze {
            let reference = plot.y(decay_axis.norm(dsp::reverb_effective_decay(&values)));
            polyline(window, plot.o, (0..=80).map(|i| (i % 3 != 0).then_some((plot.x(i as f64 / 80.), reference))), 1., with_alpha(th.text_secondary, 0.6));
        }
        texts.push(((if freeze { "冻结 · 无限延续" } else { "反馈网络衰减估计（RT60）" }).into(), p(10., 7., plot.o), 11., th.text_secondary));
    });
    let values = suite::values(dsp::REVERB_SPECS, params);
    let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
    let rate = v("decay_rate", 100.) / 100.;
    let freeze = v("freeze", 0.) >= 0.5;
    let mut handles = Vec::new();
    if !freeze {
        handles.push(Handle { control: Control::Handle("decay"), at: (FREQ_AXIS.norm(400.), decay_axis.norm(v("decay", 2.) * rate)),
            x: None, y: Some(("decay".into(), Axis::log(decay_axis.min / rate, decay_axis.max / rate))),
            color: th.text, label: "基准".into(), hollow: false, selected: false, also: vec![], defaults: vec![("decay".into(), 2.)] });
    }
    let damping = v("damping", 8000.);
    handles.push(Handle { control: Control::Handle("damping"), at: (FREQ_AXIS.norm(damping), decay_axis.norm(dsp::reverb_decay_at_frequency(&values, damping, verdant_plugins::SAMPLE_RATE))),
        x: Some(("damping".into(), FREQ_AXIS)), y: None, color: th.text, label: "阻尼".into(), hollow: false,
        selected: false, also: vec![], defaults: vec![("damping".into(), 8000.)] });
    let node = selected_band(ed, key, 8);
    for index in 0..8 {
        let prefix = format!("decay{}", index + 1);
        let on = v(&format!("{prefix}_on"), 0.) >= 0.5;
        if !on && index != node { continue; }
        let freq = v(&format!("{prefix}_freq"), 1000.);
        handles.push(Handle { control: Control::Band(index), at: (FREQ_AXIS.norm(freq), decay_axis.norm(dsp::reverb_decay_at_frequency(&values, freq, verdant_plugins::SAMPLE_RATE))),
            x: Some((format!("{prefix}_freq"), FREQ_AXIS)), y: None, color: band_color(index), label: (index+1).to_string(),
            hollow: !on, selected: index == node, also: vec![(format!("{prefix}_on"), 1.)], defaults: vec![(format!("{prefix}_freq"), 1000.)] });
    }
    let post_axis = Axis::lin(-36., 24.);
    let post_painter: Painter = Rc::new(move |window, plot, params, texts| {
        let grid = [-24., -12., 0., 12.].map(|db| (post_axis.norm(db), format!("{db:+.0} dB")));
        paint_grid(window, plot, th, &freq_grid(), &grid, Some(post_axis.norm(0.)), texts);
        let values = suite::values(dsp::REVERB_SPECS, params);
        let points = (0..=240).map(|i| { let n = i as f64 / 240.; Some((plot.x(n), plot.y(post_axis.norm(dsp::reverb_post_response_db(&values, FREQ_AXIS.value(n), verdant_plugins::SAMPLE_RATE))))) });
        polyline(window, plot.o, points, 2.25, th.accent);
        texts.push(("输出均衡".into(), p(10., 7., plot.o), 11., th.text_secondary));
    });
    let mut post_handles = Vec::new();
    for (control, freq, gain, label) in [
        ("post-low", "decay_low_freq", "post_low_gain", "低架"),
        ("post-mid", "post_mid_freq", "post_mid_gain", "峰值"),
        ("post-high", "decay_high_freq", "post_high_gain", "高架"),
    ] {
        let defaults = [freq, gain].iter().filter_map(|id| param(params, id).and_then(|p| p.default_value.map(|v| ((*id).into(), v)))).collect();
        post_handles.push(Handle { control: Control::Handle(control), at: (FREQ_AXIS.norm(v(freq, 1000.)), post_axis.norm(v(gain, 0.))),
            x: Some((freq.into(), FREQ_AXIS)), y: Some((gain.into(), post_axis)), color: th.accent, label: label.into(), hollow: false,
            selected: false, also: vec![], defaults });
    }
    for (id, default) in [("low_cut", 120.), ("high_cut", 12000.)] {
        post_handles.push(Handle { control: Control::Handle(id), at: (FREQ_AXIS.norm(v(id, default)), post_axis.norm(-30.)),
            x: Some((id.into(), FREQ_AXIS)), y: None, color: th.text_secondary, label: String::new(), hollow: true,
            selected: false, also: vec![], defaults: vec![(id.into(), default)] });
    }
    for index in 0..8 {
        let prefix = format!("post{}", index + 1);
        let on = v(&format!("{prefix}_on"), 0.) >= 0.5;
        if !on && index != node { continue; }
        let freq = v(&format!("{prefix}_freq"), 1000.); let gain = v(&format!("{prefix}_gain"), 0.);
        let has_gain = v(&format!("{prefix}_shape"), 0.) < 3.;
        post_handles.push(Handle { control: Control::Band(index), at: (FREQ_AXIS.norm(freq), post_axis.norm(if has_gain { gain } else { -12. })),
            x: Some((format!("{prefix}_freq"), FREQ_AXIS)), y: has_gain.then(|| (format!("{prefix}_gain"), post_axis)), color: band_color(index),
            label: (index+1).to_string(), hollow: !on, selected: index == node, also: vec![(format!("{prefix}_on"), 1.)],
            defaults: vec![(format!("{prefix}_freq"), 1000.), (format!("{prefix}_gain"), 0.)] });
    }
    if advanced {
        return div().flex().flex_col().gap(gpui::rems(12./16.))
            .child(presentation::caption("输出均衡与衰减节点", "拖动曲线上的节点编辑当前频段", th))
            .child(graph(editor.clone(), target(), params.to_vec(), "reverb-post", "混响输出均衡：拖动频率和增益", th, post_painter, post_handles, None, None).h(gpui::rems(260./16.)).w_full())
            .child(knob_row(editor, target, params, &["post_low_gain", "post_mid_freq", "post_mid_gain", "post_mid_q", "post_high_gain"], th, 46.))
            .child(suite::reverb_nodes(ed, editor, track, slot, key, params, cx))
            .child(sidechain_picker(ed, track, slot, cx)).into_any_element();
    }
    div().flex().flex_col().gap(gpui::rems(12./16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "reverb-graph", "混响衰减随频率：拖动基准衰减与阻尼", th, painter, handles, None, None).h(gpui::rems(290./16.)).w_full())
        .child(knob_row(editor, target, params, &["predelay", "decay", "size", "damping", "diffusion"], th, 50.))
        .child(knob_row(editor, target, params, &["width", "low_cut", "high_cut", "modulation", "mix"], th, 46.))
        .into_any_element()
}

fn delay_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    suite::delay(ed, editor, track, slot, key, params, cx)
}

fn saturation_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let count = param(params, "band_count").map_or(1, |p| p.value.round().clamp(1., 6.) as usize);
    let selected = selected_band(ed, key, count);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let grid = vec![(0.25, String::new()), (0.5, "0".to_string()), (0.75, String::new())];
        paint_grid(window, plot, th, &grid, &grid, Some(0.5), texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let values = suite::values(dsp::SATURATION_SPECS, params);
        polyline(window, plot.o, [Some((plot.x(0.0), plot.y(0.0))), Some((plot.x(1.0), plot.y(1.0)))], 1.0, with_alpha(th.text, 0.18));
        let pts = (0..=160).map(|i| { let x = -1.0 + i as f64 / 80.0; Some((plot.x((x + 1.0) / 2.0), plot.y((((x * (1. - v("mix", 100.) / 100.) + dsp::saturation_band_curve(&values, selected, x) * v("mix", 100.) / 100.) + 1.0) / 2.0).clamp(-0.05, 1.05)))) });
        polyline(window, plot.o, pts, 2.25, th.accent);
        texts.push((format!("频段 {} 静态特性", selected + 1), p(10., 8., plot.o), 10., th.text_secondary));
    });
    div().flex().gap(gpui::rems(16.0/16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "saturation-graph", "饱和传输曲线", th, painter, vec![], None, None).size(gpui::rems(170.0/16.)).flex_none())
        .child(div().flex_1().min_w_0().flex().flex_col().gap(gpui::rems(10.0/16.))
            .child(choice(ed, track, slot, params, "style", &dsp::SATURATION_STYLES, cx))
            .child(knob_row(editor, target, params, &["drive", "tone", "mix", "output"], th, 46.0))
            .child(sidechain_picker(ed, track, slot, cx)))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Instrument (built-in synth)
// ---------------------------------------------------------------------------

/// Apply the live drag preview for `key`, then ease toward new values.
pub(crate) fn preview_and_animate(ed: &mut Editor, key: InsertKey, params: &mut [ParamDesc], window: &mut Window) {
    let dragging = ed.device_drag.as_ref().filter(|d| d.key == key).map(|d| d.preview.clone());
    if let Some(preview) = &dragging {
        for (id, value) in preview {
            if let Some(p) = params.iter_mut().find(|p| &p.id == id) { p.value = *value; }
        }
    }
    animate(ed, key, params, dragging.is_some(), window);
}

pub(crate) fn synth_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let slot = crate::instrument::INSTRUMENT_SLOT;
    let instrument_id = ed.project().track(track).and_then(|track| track.instrument()).and_then(|spec| spec.get("id")).and_then(serde_json::Value::as_str).unwrap_or(dsp::SYNTH_ID);
    let owner = DeviceTarget::Track(track.clone());
    let track = &owner;
    let target = || Target { track: track.clone(), slot, key };
    let (attack, decay, release) = (Axis::log(1.0, 2000.0), Axis::log(1.0, 4000.0), Axis::log(1.0, 4000.0));
    let sustain = Axis::lin(0.0, 100.0);
    // ADSR: four equal columns for attack, decay, hold and release.
    let points = move |ps: &[ParamDesc]| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        let (a, d, s, r) = (attack.norm(v("attack", 5.0)), decay.norm(v("decay", 300.0)), sustain.norm(v("sustain", 70.0)), release.norm(v("release", 250.0)));
        let xa = 0.02 + 0.23 * a;
        let xd = xa + 0.02 + 0.23 * d;
        let xs = 0.75;
        let xr = xs + 0.02 + 0.23 * r;
        [(0.0, 0.0), (xa, 1.0), (xd, s), (xs, s), (xr.min(1.0), 0.0)]
    };
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        paint_grid(window, plot, th, &[(0.25, "A".into()), (0.5, "D".into()), (0.75, "R".into())], &[(0.5, String::new())], None, texts);
        let pts: Vec<(f64, f64)> = points(params).iter().map(|&(x, y)| (plot.x(x), plot.y(y))).collect();
        fill_under(window, plot, &pts, plot.y(0.0), with_alpha(th.accent, 0.14));
        polyline(window, plot.o, pts.iter().map(|&q| Some(q)), 2.25, th.accent);
    });
    let pts = points(params);
    let mk = |control: &'static str, at: (f64, f64), x: Option<(String, Axis)>, y: Option<(String, Axis)>, defaults: Vec<(String, f64)>| Handle {
        control: Control::Handle(control), at, x, y, color: gpui::rgb(0xffffff), label: String::new(), hollow: false, selected: false, also: vec![], defaults };
    let handles = vec![
        mk("attack", pts[1], Some(("attack".into(), attack.span(0.02, 0.25))), None, vec![("attack".into(), 5.0)]),
        mk("decay", pts[2], Some(("decay".into(), decay.span(pts[1].0 + 0.02, pts[1].0 + 0.25))), Some(("sustain".into(), sustain)), vec![("decay".into(), 300.0), ("sustain".into(), 70.0)]),
        mk("release", pts[4], Some(("release".into(), release.span(0.77, 1.0))), None, vec![("release".into(), 250.0)]),
    ];
    let mut waves = |id: &str, label: &str| div().flex().items_center().gap(gpui::rems(8.0/16.))
        .child(div().w(gpui::rems(64.0/16.)).text_sm().text_color(th.text_secondary).child(label.to_string()))
        .child(choice(ed, track, slot, params, id, &dsp::WAVES, cx));
    let osc1 = waves("osc1", "振荡器 1");
    let osc2 = param(params, "osc2").map(|_| waves("osc2", "振荡器 2"));
    let page = presentation::page(ed, &target());
    let content = if page == 2 { signal::panel(ed, editor, track, slot, key, cx) }
    else if page == 1 {
        div().flex().flex_col().gap(gpui::rems(12./16.))
            .child(presentation::caption("音色设计", "展开振荡器、滤波器、效果或演奏设置", th))
            .child(suite::remaining(ed, editor, track, slot, key, instrument_id, params, cx))
            .child(modulation::panel(ed, editor, track, slot, key, instrument_id, params, cx)).into_any_element()
    } else {
        div().flex().flex_col().gap(gpui::rems(14./16.))
            .child(signal::compact(ed, track, slot, key))
            .child(div().flex().flex_wrap().items_center().gap(gpui::rems(16./16.))
                .child(div().flex_1().min_w(gpui::rems(280./16.)).flex().flex_col().gap(gpui::rems(10./16.))
                    .child(presentation::caption("振荡器", "波形与调谐", th)).child(osc1).children(osc2))
                .child(knob_row(editor, target, params, &["osc2_semi", "detune", "osc2_mix", "width", "gain"], th, 48.)))
            .child(graph(editor.clone(), target(), params.to_vec(), "synth-adsr", "音量包络：拖动起音、衰减/延音与释音点", th, painter, handles, None, None).h(gpui::rems(200./16.)).w_full())
            .child(knob_row(editor, target, params, &["attack", "decay", "sustain", "release", "cutoff", "resonance", "env_amount"], th, 46.))
            .when(workflow::read_workspace(ed, &target()).matrix_open, |d| d.child(modulation::panel(ed, editor, track, slot, key, instrument_id, params, cx)))
            .into_any_element()
    };
    div().font(crate::theme::plugin_ui_font(ed.ui_font.clone())).flex().flex_col().gap(gpui::rems(10./16.))
        .child(suite::toolbar(ed, track, slot, key, instrument_id, params, cx)).child(content).into_any_element()
}

// ---------------------------------------------------------------------------
// Device cards and the view
// ---------------------------------------------------------------------------

pub(crate) fn generic_editor(ed: &Editor, editor: &Entity<Editor>, track: &DeviceTarget, slot: usize, key: InsertKey, params: &[ParamDesc]) -> gpui::AnyElement {
    let th = ed.theme;
    let shown: Vec<&ParamDesc> = params.iter().filter(|p| p.max > p.min).collect();
    let mut grid = div().flex().flex_wrap().gap(gpui::rems(8.0/16.)).w_full();
    for d in &shown {
        grid = grid.child(knob(editor.clone(), Target { track: track.clone(), slot, key }, (*d).clone(), &d.name, th, 40.0));
    }
    div().flex().flex_col().gap(gpui::rems(8.0/16.)).child(grid)
        .when(params.is_empty(), |d| d.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数")))
        .into_any_element()
}

/// Preferred window size; the contents scroll vertically at smaller sizes.
pub(crate) fn native_editor_size(id: &str) -> (f32, f32) {
    match id {
        dsp::EQ_ID => (900., 760.),
        dsp::SYNTH_ID => (940., 780.),
        dsp::ONE_SYNTH_ID => (800., 720.),
        dsp::COMPRESSOR_ID | dsp::GATE_ID => (880., 740.),
        dsp::MULTIBAND_ID | dsp::FILTER_ID | dsp::SIMPLON_ID | dsp::SATURATION_ID => (900., 780.),
        dsp::MICRO_ID => (720., 670.),
        _ => (800., 720.),
    }
}

fn card_width(id: &str) -> Option<f32> {
    dsp::EFFECT_IDS.contains(&id).then(|| match id {
        dsp::EQ_ID | dsp::MULTIBAND_ID | dsp::FILTER_ID | dsp::SATURATION_ID => 760.,
        dsp::COMPRESSOR_ID | dsp::GATE_ID => 740.,
        _ => 660.,
    })
}

/// Both the bottom device chain and standalone native windows use this exact
/// editor. There is no second parameter implementation for master inserts.
pub(crate) fn render_builtin_editor(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget,
    slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let specs = ed.device_insert_specs(owner);
    let id = specs.get(slot).map(|spec| spec.id.as_str()).unwrap_or("");
    let target = Target { track: owner.clone(), slot, key };
    let page = presentation::page(ed, &target);
    let content = if page == 2 {
        signal::panel(ed, editor, owner, slot, key, cx)
    } else if page == 1 {
        div().flex().flex_col().gap(gpui::rems(14./16.))
            .child(presentation::caption("高级处理", "按用途展开参数组", ed.theme))
            .when(matches!(id, dsp::EQ_ID | dsp::REVERB_ID), |d| d.child(analysis_tools::tools(ed, editor, target.clone(), id, cx)))
            .when(id == dsp::MULTIBAND_ID, |d| d.child(suite::multiband_advanced(ed, editor, owner, slot, key, params, cx)))
            .when(matches!(id, dsp::FILTER_ID | dsp::SIMPLON_ID | dsp::MICRO_ID), |d| d.child(suite::filter_advanced(ed, editor, owner, slot, key, id, params, cx)))
            .when(id == dsp::COMPRESSOR_ID, |d| d.child(suite::compressor_sidechain(ed, editor, owner, slot, key, params, cx)))
            .when(id == dsp::EQ_ID, |d| d.child(suite::eq_dynamic(ed, editor, owner, slot, key, params, cx)))
            .when(id == dsp::REVERB_ID, |d| d.child(reverb_editor(ed, editor, owner, slot, key, params, true, cx)))
            .when(id == dsp::SATURATION_ID, |d| d.child(saturation_editor(ed, editor, owner, slot, key, params, cx)))
            .when(id == dsp::LIMITER_ID, |d| d.child(sidechain_picker(ed, owner, slot, cx)))
            .child(suite::remaining(ed, editor, owner, slot, key, id, params, cx))
            .child(modulation::panel(ed, editor, owner, slot, key, id, params, cx)).into_any_element()
    } else {
        let body = match id {
            dsp::EQ_ID => eq_editor(ed, editor, owner, slot, key, params, cx),
            dsp::COMPRESSOR_ID => comp_editor(ed, editor, owner, slot, key, params, cx),
            dsp::GATE_ID => gate_editor(ed, editor, owner, slot, key, params, cx),
            dsp::DEESSER_ID => deesser_editor(ed, editor, owner, slot, key, params, cx),
            dsp::LIMITER_ID => limiter_editor(ed, editor, owner, slot, key, params, cx),
            dsp::REVERB_ID => reverb_editor(ed, editor, owner, slot, key, params, false, cx),
            dsp::DELAY_ID => delay_editor(ed, editor, owner, slot, key, params, cx),
            dsp::SATURATION_ID => suite::saturation_bands(ed, editor, owner, slot, key, params, cx),
            dsp::MULTIBAND_ID => suite::multiband(ed, editor, owner, slot, key, params, cx),
            dsp::FILTER_ID | dsp::SIMPLON_ID | dsp::MICRO_ID => suite::filters(ed, editor, owner, slot, key, id, params, cx),
            _ => generic_editor(ed, editor, owner, slot, key, params),
        };
        div().flex().flex_col().gap(gpui::rems(10./16.))
            .child(signal::compact(ed, owner, slot, key)).child(body)
            .when(id == dsp::SATURATION_ID, |d| d.child(knob_row(editor, || target.clone(), params, &["band_count", "mix", "output"], ed.theme, 46.)))
            .when(workflow::read_workspace(ed, &target).matrix_open, |d| d.child(modulation::panel(ed, editor, owner, slot, key, id, params, cx)))
            .into_any_element()
    };
    div().font(crate::theme::plugin_ui_font(ed.ui_font.clone())).flex().flex_col().gap(gpui::rems(10./16.)).w_full().min_w_0()
        .child(suite::toolbar(ed, owner, slot, key, id, params, cx))
        .child(content).into_any_element()
}

#[derive(Clone, Copy)]
enum ChainAction {
    Bypass { slot: usize, expected_key: Option<InsertKey> },
    Menu { slot: usize, expected_key: Option<InsertKey> },
    AddBuiltin(&'static str),
    Browse(usize),
}

fn chain_command(ed: &Editor, track: TrackId, action: ChainAction, cx: &Context<Editor>) -> SemanticCommand {
    let editor = cx.weak_entity();
    let project = ed.project().project_id.clone();
    let generation = ed.document_generation;
    // A project revision binds the persisted slot and chain structure without
    // retaining or cloning any potentially large plug-in state blobs. A live
    // host can change independently, so its optional key is checked as well.
    let revision = ed.revision();
    let owner = DeviceTarget::Track(track.clone());
    let guard_track = track.clone();
    let allowed = Rc::new(move |ed: &Editor, window: &Window| {
        ed.project().project_id == project && ed.document_generation == generation && ed.revision() == revision
            && ed.project().track(&guard_track).is_some() && DeviceScope::for_window(window).allows(&owner, ed)
            && device_input_available(ed) && !ed.inserts_loading
            && match action {
                ChainAction::Bypass { slot, expected_key } | ChainAction::Menu { slot, expected_key } =>
                    ed.live_insert(&guard_track, slot).map(|handle| handle.key()) == expected_key,
                ChainAction::AddBuiltin(_) | ChainAction::Browse(_) => true,
            }
    });
    let before_cancel = allowed.clone();
    let activate = semantic_command(editor.clone(), editor.clone(), move |ed, event, window, cx| {
        if !allowed(ed, window) || !may_begin_device_input(ed) { return; }
        match action {
            ChainAction::Bypass { slot, .. } => {
                controls::dispatch(ed, CommandId::PluginBypass, CommandArg::TrackSlot(track.clone(), slot as u32), window, cx);
            }
            ChainAction::Menu { slot, expected_key } => {
                use crate::editor::MenuAction as M;
                use crate::menu::MenuItem as I;
                let specs = ed.insert_specs(&track);
                let Some(spec) = specs.get(slot) else { return };
                let items = vec![
                    I::action("打开原生插件窗口", M::OpenInsertEditor { track: track.clone(), slot, expected_key })
                        .enabled(ed.live_insert(&track, slot).is_some_and(|h| h.has_editor() || h.spec().format == "builtin")),
                    I::action(if spec.bypass { "启用" } else { "旁通" }, M::BypassInsert { track: track.clone(), slot, expected_key }),
                    I::separator(),
                    I::action("移除效果器", M::RemoveInsert { track: track.clone(), slot, expected_key }),
                ];
                let position = event.map_or_else(|| window.mouse_position(), crate::proto_controls::Activation::position);
                ed.show_menu(position, items, window, cx);
            }
            ChainAction::AddBuiltin(id) => ed.add_insert(&track, "builtin", id, cx),
            ChainAction::Browse(slot) => ed.open_plugin_browser(track.clone(), slot, cx),
        }
    });
    Rc::new(move |event, window, cx| {
        let current = editor.upgrade().is_some_and(|editor| before_cancel(editor.read(cx), window));
        if current { activate(event, window, cx); }
    })
}

#[allow(clippy::too_many_arguments)]
fn device_card(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, params: Option<(InsertKey, Vec<ParamDesc>)>, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let owner = DeviceTarget::Track(track.clone());
    let specs = ed.insert_specs(track);
    let spec = &specs[slot];
    let handle = ed.live_insert(track, slot);
    let expected_key = handle.as_ref().map(|handle| handle.key());
    let builtin = (spec.format == "builtin").then_some(spec.id.as_str());
    let on = !spec.bypass;
    let keyed = verdant_service::project_audio::sidechain_source(spec).is_some();
    let title = builtin.and_then(dsp::short_label).map_or(spec.name.clone(), str::to_string);
    let header = div().flex().items_center().gap(gpui::rems(10.0/16.)).h(gpui::rems(32.0/16.))
        .child(suite::command_button(crate::proto_controls::action(SharedString::from(format!("device-power-{slot}")), "效果器旁通").role(Role::Switch)
            .aria_label("启用效果器").aria_selected(on).tab_index(0).cursor_pointer().child(crate::view::switch(on, th)),
            chain_command(ed, track.clone(), ChainAction::Bypass { slot, expected_key }, cx)))
        .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).text_color(if on { th.text } else { th.text_secondary }).child(title))
        .when(keyed, |d| d.child(div().px(gpui::rems(8.0/16.)).h(gpui::rems(22.0/16.)).flex().items_center().rounded(gpui::rems(11.0/16.)).bg(with_alpha(th.accent, 0.18)).text_xs().text_color(th.accent).child("侧链")))
        .child(suite::command_button(crate::proto_controls::action(SharedString::from(format!("device-menu-{slot}")), "效果器菜单").role(Role::Button).aria_label("效果器菜单").tab_index(0).size(gpui::rems(28.0/16.)).rounded(gpui::rems(6.0/16.))
            .flex().items_center().justify_center().hover(move |s| s.bg(th.hover)).active(move |s| s.bg(th.control_pressed)).cursor_pointer()
            .child(svg().path(icon("menu")).size(gpui::rems(14.0/16.)).text_color(th.text)),
            chain_command(ed, track.clone(), ChainAction::Menu { slot, expected_key }, cx)));
    let body = match params {
        Some((key, params)) if builtin.is_some() => render_builtin_editor(ed, editor, &owner, slot, key, &params, cx),
        Some((key, params)) => generic_editor(ed, editor, &owner, slot, key, &params),
        None => div().h(gpui::rems(120.0/16.)).flex().items_center().justify_center().text_sm().text_color(th.text_secondary)
            .child(if handle.is_some() { "正在读取参数…" } else { "插件未载入（标识与状态已保留）" }).into_any_element(),
    };
    let width = builtin.and_then(card_width).or(Some(480.));
    div().id(SharedString::from(format!("device-{slot}"))).flex_none().flex().flex_col().gap(gpui::rems(10.0/16.)).p(gpui::rems(14.0/16.))
        .rounded(gpui::rems(12.0/16.)).bg(th.card).shadow_sm().when(!on, |d| d.opacity(0.72))
        .when_some(width, |d, w| d.w(px(w)))
        .child(header).child(body)
        .into_any_element()
}

fn add_card(ed: &Editor, track: &TrackId, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let pill = |id: SharedString, label: &str| crate::proto_controls::action(id, label.to_string()).role(Role::Button).aria_label(label.to_string()).tab_index(0).h(gpui::rems(32.0/16.)).px(gpui::rems(12.0/16.))
        .flex().items_center().rounded(gpui::rems(6.0/16.)).bg(th.control).hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(th.checked))
        .cursor_pointer().text_sm().child(label.to_string());
    let effects = dsp::EFFECT_IDS.iter().map(|&id| {
        suite::command_button(pill(SharedString::from(format!("device-add-{id}")), dsp::short_label(id).unwrap_or(id)),
            chain_command(ed, track.clone(), ChainAction::AddBuiltin(id), cx))
    });
    div().id("device-add").flex_none().w(gpui::rems(220.0/16.)).flex().flex_col().gap(gpui::rems(8.0/16.)).p(gpui::rems(14.0/16.)).rounded(gpui::rems(12.0/16.))
        .border_1().border_color(th.shade)
        .child(div().text_sm().font_weight(FontWeight::BOLD).child("添加效果器"))
        .child(div().flex().flex_wrap().gap(gpui::rems(6.0/16.)).children(effects))
        .child(suite::command_button(pill("device-add-browse".into(), "浏览插件…"),
            chain_command(ed, track.clone(), ChainAction::Browse(slot), cx)))
        .into_any_element()
}

/// The device view for the selected track.
pub fn device_view(ed: &mut Editor, editor: &Entity<Editor>, _window: &mut Window, _cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let track = ed.session.track.clone();
    let Some(name) = ed.project().track(&track).map(|t| t.name.clone()) else {
        return div().size_full().bg(th.view).into_any_element();
    };
    let count = ed.insert_specs(&track).len();
    let empty = count == 0 && ed.project().track(&track).is_none_or(|track| track.instrument().is_none());
    let owner = editor.clone();
    div().id("device-view").role(Role::Group).aria_label(format!("{name} 的效果器链"))
        .track_focus(&ed.focus_devices)
        .size_full().flex().flex_col().bg(th.view)
        .child(div().h(gpui::rems(40.0/16.)).flex_none().px(gpui::rems(16.0/16.)).flex().items_center().gap(gpui::rems(8.0/16.))
            .child(svg().path(icon("sliders-horizontal")).size(gpui::rems(15.0/16.)).text_color(th.text_secondary))
            .child(div().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(div().text_sm().text_color(th.text_secondary).child(if empty { "· 还没有效果器".to_string() } else { format!("· {count} 个效果器，信号从左到右") })))
        .child(div().flex_1().min_h_0().child(crate::proto_scroll::content("device-chain", "效果器链", move |window, cx| {
            owner.update(cx, |ed, cx| device_chain(ed, &owner, window, cx))
        }).axes("both")))
        .into_any_element()
}

fn device_chain(ed: &mut Editor, editor: &Entity<Editor>, window: &mut Window, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let track = ed.session.track.clone();
    let count = ed.insert_specs(&track).len();
    for slot in 0..count { ed.fetch_track_insert_params(&track, slot, cx); }
    let mut params = Vec::with_capacity(count);
    for slot in 0..count {
        let live = ed.live_insert(&track, slot).map(|h| h.key()).and_then(|key| device_params(ed, key).map(|p| (key, p)));
        params.push(live.map(|(key, mut values)| {
            let dragging = ed.device_drag.as_ref().is_some_and(|d| d.key == key);
            animate(ed, key, &mut values, dragging, window);
            (key, values)
        }));
    }
    let instrument = crate::instrument::instrument_card(ed, editor, &track, window, cx);
    let ed = &*ed;
    let cards: Vec<gpui::AnyElement> = instrument.into_iter().chain(params.into_iter().enumerate().map(|(slot, p)| device_card(ed, editor, &track, slot, p, cx))).collect();
    div().flex().items_start().gap(gpui::rems(12.0/16.)).px(gpui::rems(12.0/16.)).pb(gpui::rems(12.0/16.))
        .children(cards).child(add_card(ed, &track, count, cx)).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(phase: GesturePhase, event: GestureEvent, hit: bool, crossed: bool, reset: bool, double: bool) -> Result<verdant_ui_protocol::ControlPlan, PlannerError> {
        verdant_ui_protocol::plan_control(phase, event, verdant_ui_protocol::ControlGuards {
            hit_control: hit, threshold_crossed: crossed, reset_modifier: reset, double_click: double,
        })
    }

    fn d(min: f64, max: f64, unit: &str) -> ParamDesc {
        ParamDesc { id: "x".into(), name: "x".into(), value: min, min, max, unit: unit.into(), step_count: 0, default_value: None }
    }

    #[test]
    fn knob_mapping_is_logarithmic_for_wide_ranges_and_round_trips() {
        let hz = d(20.0, 20_000.0, "Hz");
        assert!(is_log(&hz));
        assert!((from_norm(&hz, 0.5) - 632.455).abs() < 0.01, "half a turn is the geometric middle");
        for v in [20.0, 100.0, 1_000.0, 20_000.0] { assert!((from_norm(&hz, to_norm(&hz, v)) - v).abs() < 1e-6); }
        let db = d(-24.0, 24.0, "dB");
        assert!(!is_log(&db));
        assert_eq!(from_norm(&db, 0.5), 0.0);
        let mut stepped = d(0.0, 1.0, "");
        stepped.step_count = 1;
        assert_eq!(from_norm(&stepped, 0.4), 0.0);
        assert_eq!(from_norm(&stepped, 0.6), 1.0);
        assert_eq!(format_value(&hz, 1500.0), "1.50 kHz");
        assert_eq!(format_value(&d(0.01, 20., "Hz"), 0.25), "0.25 Hz");
        assert_eq!(format_value(&d(0.01, 500., "ms"), 0.05), "0.05 ms");
        assert_eq!(format_value(&db, 3.0), "+3.0 dB");
    }

    #[test]
    fn compiled_control_protocol_selects_drags_commits_and_resets() {
        use ControlEffect::*;
        use GestureEvent::*;
        use GesturePhase::*;
        let down = plan(Idle, Down, true, false, false, false).unwrap();
        assert_eq!(down.next_phase, Armed);
        assert!(down.consumed);
        assert_eq!(down.effects, [Select, Begin, Capture]);
        assert!(plan(Armed, Move, true, false, false, false).unwrap().effects.is_empty());
        assert_eq!(plan(Armed, Move, true, true, false, false).unwrap().next_phase, Dragging);
        assert_eq!(plan(Dragging, Up, true, false, false, false).unwrap().effects, [Commit, Release]);
        assert_eq!(plan(Armed, Up, true, false, false, false).unwrap().effects, [Discard, Release]);
        assert_eq!(plan(Idle, Down, true, false, false, true).unwrap().effects, [Select, Reset]);
        assert_eq!(plan(Idle, Down, true, false, true, false).unwrap().effects, [Select, Reset]);
        assert!(!plan(Idle, Down, false, false, false, false).unwrap().consumed);
        assert_eq!(drag_normalized(0.5, 90.0, 180.0, false).unwrap(), 1.0);
        assert!((drag_normalized(0.5, 90.0, 180.0, true).unwrap() - 0.55).abs() < 1e-9);
        assert_eq!(drag_normalized(0.2, -500.0, 180.0, false).unwrap(), 0.0);
    }

    #[test]
    fn graph_axes_map_both_ways_and_animation_eases_in_the_knob_scale() {
        let hz = Axis::log(20.0, 20_000.0);
        assert!((hz.value(hz.norm(1_000.0)) - 1_000.0).abs() < 1e-9);
        assert_eq!(Axis::lin(-60.0, 0.0).norm(-30.0), 0.5);
        assert_eq!(Axis::lin(-60.0, 0.0).norm(10.0), 1.0, "clamped");
        let freq = d(20.0, 20_000.0, "Hz");
        // Halfway in time is past halfway in value (ease-out), on the log scale.
        let mid = interpolate(&freq, 20.0, 20_000.0, 0.5);
        assert!(mid > 632.5 && mid < 20_000.0, "{mid}");
        assert_eq!(interpolate(&freq, 20.0, 20_000.0, 1.0), 20_000.0);
        let mut stepped = d(0.0, 7.0, "");
        stepped.step_count = 7;
        assert_eq!(interpolate(&stepped, 0.0, 5.0, 0.1), 5.0, "stepped values switch at once");
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
    }

    #[test]
    fn compiled_control_cancel_discards_armed_and_previewed_edits() {
        for phase in [GesturePhase::Armed, GesturePhase::Dragging] {
            let cancel = plan(phase, GestureEvent::Cancel, false, false, false, false).unwrap();
            assert_eq!(cancel.next_phase, GesturePhase::Idle);
            assert_eq!(cancel.effects, [ControlEffect::Discard, ControlEffect::Release]);
            assert!(plan(cancel.next_phase, GestureEvent::Up, true, false, false, false).unwrap().effects.is_empty(),
                "a late release must not commit a cancelled preview");
            let next = plan(cancel.next_phase, GestureEvent::Down, true, false, false, false).unwrap();
            assert_eq!(next.next_phase, GesturePhase::Armed);
            assert!(next.effects.contains(&ControlEffect::Begin));
        }
    }

    #[test]
    fn graph_drags_use_the_display_axis_then_clamp_to_the_parameter() {
        let ceiling = d(-12., 0., "dB");
        let level_axis = Axis::lin(-36., 0.);
        assert!((bounded_graph_value(&ceiling, level_axis, level_axis.norm(-6.), 10., 100., false).unwrap() + 2.4).abs() < 1e-9);
        assert_eq!(bounded_graph_value(&ceiling, level_axis, level_axis.norm(-6.), -100., 100., false).unwrap(), -12.);
        let cutoff = d(20., 1000., "Hz");
        assert!((bounded_graph_value(&cutoff, FREQ_AXIS, FREQ_AXIS.norm(100.), 10., 100., false).unwrap() - 199.5262315).abs() < 1e-6);
        assert!((bounded_graph_value(&cutoff, FREQ_AXIS, FREQ_AXIS.norm(100.), 100., 100., false).unwrap() - 1000.).abs() < 1e-9);
        let delay = d(1., 2000., "ms");
        let timeline = Axis::lin(0., 2000.);
        assert!((bounded_graph_value(&delay, timeline, timeline.norm(500.), 10., 100., false).unwrap() - 700.).abs() < 1e-9);
        let attack = d(1., 2000., "ms");
        let envelope = Axis::log(1., 2000.).span(0.02, 0.25);
        let before = envelope.norm(100.);
        let value = bounded_graph_value(&attack, envelope, before, 3., 100., false).unwrap();
        assert!((envelope.norm(value) - before - 0.03).abs() < 1e-9, "a handle follows the pointer within its own envelope segment");
    }

    #[test]
    fn comparison_waits_for_the_committed_snapshot_and_survives_rejection() {
        let a = vec![("gain".to_string(), -3.)];
        let b = vec![("gain".to_string(), 2.)];
        let mut comparison = Comparison { active: 0, slots: [Some(a.clone()), Some(b.clone())], pending: Some(1) };
        comparison.reconcile(&b, true);
        assert_eq!(comparison.active, 0, "busy host has not confirmed the request");
        assert_eq!(comparison.pending, Some(1));
        comparison.reconcile(&a, false);
        assert_eq!(comparison.active, 0, "rollback leaves A active");
        assert_eq!(comparison.pending, None);
        comparison.pending = Some(1);
        comparison.reconcile(&b, false);
        assert_eq!(comparison.active, 1);
        assert_eq!(comparison.pending, None);
    }


}

#[cfg(test)]
mod native_tests {
    use super::*;
    use std::cell::Cell;
    use gpui::{App, AppContext, FocusHandle, Render, TestAppContext, WindowHandle, point, size};
    use verdant_domain::{Command, CommandBatch};

    const WIDTH: f32 = 240.0;
    const HEIGHT: f32 = 140.0;

    struct Harness {
        editor: Entity<Editor>,
        target: Target,
        desc: ParamDesc,
        show: bool,
        outside: FocusHandle,
        block_up: Rc<Cell<bool>>,
        with_knob: bool,
        double: Option<DoubleClick>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let editor = self.editor.read(cx);
            let theme = editor.theme;
            let focus = editor.focus_devices.clone();
            let desc = self.desc.clone();
            let handle = Handle { control: Control::Handle("gain"), at: (0.5, to_norm(&desc, desc.value)), x: None,
                y: Some((desc.id.clone(), Axis::lin(desc.min, desc.max))), color: theme.accent, label: String::new(),
                hollow: false, selected: false, also: Vec::new(), defaults: vec![(desc.id.clone(), desc.default_value.unwrap())] };
            let block_up = self.block_up.clone();
            div().id("native-device-harness").relative().size_full().track_focus(&focus)
                .when(self.show, |view| view.child(div().w(px(WIDTH)).h(px(HEIGHT)).child(graph(
                    self.editor.clone(), self.target.clone(), vec![desc.clone()], "test-graph", "Gain graph", theme,
                    Rc::new(|_, _, _, _| {}), vec![handle], None, self.double.clone()).size_full())))
                .when(self.with_knob, |view| view.child(div().absolute().left(px(250.0)).top_0().child(crate::proto_knob::knob(
                    self.editor.clone(), self.target.track.clone(), self.target.slot, self.target.key, desc, "Gain", theme, 64.0))))
                .child(div().id("outside-device").track_focus(&self.outside).absolute().left(px(400.0)).top(px(160.0)).size(px(20.0)))
                .child(canvas(|_, _, _| {}, move |_, _, window, _| {
                    window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Bubble && block_up.get() { cx.stop_propagation(); }
                    });
                }).size(px(0.0)))
        }
    }

    fn open(cx: &mut TestAppContext, with_knob: bool) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(520.0), px(220.0)), |window, cx| {
            let editor = cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx));
            let track = editor.read(cx).session.track.clone();
            let key = crate::instrument::instrument_key_for_generation(&track, dsp::SYNTH_ID, editor.read(cx).document_generation);
            let target = Target { key, track: DeviceTarget::Track(track), slot: crate::instrument::INSTRUMENT_SLOT };
            let desc = target.basis(editor.read(cx)).unwrap().params.into_iter().find(|desc| desc.id == "gain").unwrap();
            Harness { editor, target, desc, show: true, outside: cx.focus_handle(), block_up: Rc::new(Cell::new(false)), with_knob, double: None }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }

    fn start(host: &Harness) -> gpui::Point<Pixels> {
        // The graph has a one-pixel border. The pointer is at the actual
        // production handle, using the same descriptor/axis that was rendered.
        point(px(WIDTH / 2.0), px(1.0 + GRAPH_PAD as f32 + (HEIGHT - 2.0 - 2.0 * GRAPH_PAD as f32)
            * (1.0 - to_norm(&host.desc, host.desc.value) as f32)))
    }

    fn down(host: &Harness, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: start(host), button: MouseButton::Left, ..Default::default()
        }), cx);
        assert!(host.editor.read(cx).device_drag.is_some(), "production graph accepted the handle");
        assert!(window.captured_hitbox().is_some());
    }

    fn up(window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
            position: point(px(300.0), px(0.0)), button: MouseButton::Left, ..Default::default()
        }), cx);
    }

    fn gain(host: &Harness, cx: &App) -> f64 {
        let editor = host.editor.read(cx);
        let params = crate::instrument::synth_params(editor.project().track(host.target.track.track_id().unwrap()).unwrap()).unwrap();
        crate::instrument::synth_descs(&params).into_iter().find(|desc| desc.id == "gain").unwrap().value
    }

    fn hold_suite_knob(window: &mut Window, cx: &mut App) -> (verdant_normalized_knob::NormalizedKnobHandle, gpui::Point<Pixels>, serde_json::Value) {
        let owner = verdant_normalized_knob::native::gpui_native_owner::all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id()) && owner.declared_events.contains("valueCommit")
        }).expect("the production compiled knob is mounted in this window");
        let knob = verdant_normalized_knob::NormalizedKnobHandle { owner };
        let accepted = knob.state("value").unwrap();
        let start = knob.owner.borrow().native_hitbox.as_ref().unwrap().bounds.center();
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: start, button: MouseButton::Left, ..Default::default()
        }), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
            position: start + point(px(0.), px(-36.)), pressed_button: Some(MouseButton::Left), ..Default::default()
        }), cx);
        assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
        assert!(knob.state("value").unwrap().as_f64().unwrap() > accepted.as_f64().unwrap());
        (knob, start, accepted)
    }

    fn release_suite_knob(start: gpui::Point<Pixels>, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
            position: start + point(px(0.), px(-36.)), button: MouseButton::Left, ..Default::default()
        }), cx);
    }

    // GPUI's ActionRequest dispatcher is crate-private. These tests exercise
    // the production command shared by pointer/keyboard/accessible activation
    // with real window input and the durable store; OS action transport is a
    // separate test lane, not simulated by directly invoking this callback.
    #[gpui::test]
    fn suite_commands_cancel_preview_before_comparison_copy_and_reset(cx: &mut TestAppContext) {
        for action in [suite::SuiteAction::Compare(1), suite::SuiteAction::CopyComparison, suite::SuiteAction::Reset, suite::SuiteAction::SelectBand(2)] {
            let window = open(cx, true);
            if matches!(action, suite::SuiteAction::Reset) {
                window.update(cx, |host, _, cx| {
                    host.editor.update(cx, |ed, cx| {
                        assert!(ed.set_device_params(host.target.track.clone(), host.target.slot, vec![("gain".into(), -18.)], cx));
                    });
                    host.desc = current_params(host.editor.read(cx), &host.target).unwrap().into_iter().find(|d| d.id == "gain").unwrap();
                    cx.notify();
                }).unwrap();
                crate::native_test_support::frames(window, cx);
            }
            window.update(cx, |host, window, cx| {
                let before = host.editor.read(cx).revision();
                let accepted_gain = gain(host, cx);
                if matches!(action, suite::SuiteAction::Compare(_)) {
                    host.editor.update(cx, |ed, _| {
                        let saved = current_params(ed, &host.target).unwrap().into_iter()
                            .map(|p| (p.id.clone(), if p.id == "gain" { -14. } else { p.value })).collect();
                        ed.device_anim.entry(host.target.key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now()))
                            .comparison.slots[1] = Some(saved);
                    });
                }
                let command = host.editor.update(cx, |ed, cx| suite::command(ed, host.target.clone(), action.clone(), cx));
                let (knob, start, accepted) = hold_suite_knob(window, cx);
                command(None, window, cx);
                assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!(knob.state("value").unwrap(), accepted);
                assert!(!host.editor.read(cx).has_proto_pointer_contact());
                let (expected_gain, writes) = match action {
                    suite::SuiteAction::Compare(_) => (-14., 1),
                    suite::SuiteAction::Reset => (host.desc.default_value.unwrap(), 1),
                    suite::SuiteAction::CopyComparison => {
                        let ed = host.editor.read(cx);
                        for values in &ed.device_anim[&host.target.key].comparison.slots {
                            assert_eq!(values.as_ref().unwrap().iter().find(|(id, _)| id == "gain").unwrap().1, accepted_gain,
                                "copy saves the accepted value, never the held preview");
                        }
                        (accepted_gain, 0)
                    }
                    suite::SuiteAction::SelectBand(index) => {
                        assert_eq!(selected_band(host.editor.read(cx), host.target.key, dsp::EQ_BANDS), index);
                        (accepted_gain, 0)
                    }
                    _ => unreachable!(),
                };
                assert_eq!(gain(host, cx), expected_gain);
                assert_eq!(host.editor.read(cx).revision().get(), before.get() + writes);
                release_suite_knob(start, window, cx);
                assert_eq!(gain(host, cx), expected_gain);
                assert_eq!(host.editor.read(cx).revision().get(), before.get() + writes, "late release cannot add a second edit");
            }).unwrap();
        }
    }

    #[gpui::test]
    fn suite_old_target_cannot_cancel_a_replacement_knob(cx: &mut TestAppContext) {
        for changed in ["generation", "instrument", "selected-track"] {
            let window = open(cx, true);
            let old_command = window.update(cx, |host, _, cx| {
                let command = host.editor.update(cx, |ed, cx| suite::command(ed, host.target.clone(), suite::SuiteAction::CopyComparison, cx));
                host.editor.update(cx, |ed, _| {
                    match changed {
                        "generation" => ed.document_generation += 1,
                        "instrument" => {
                            ed.persistence.commit(ed.revision(), CommandBatch::single("Replace instrument", Command::SetTrackInstrument {
                                track: ed.session.track.clone(), instrument: Some(crate::instrument::builtin_instrument_spec(
                                    dsp::ONE_SYNTH_ID, &dsp::SynthParams::one_defaults(), "test")),
                            })).unwrap();
                        }
                        "selected-track" => {
                            let track = ed.project().tracks[1].id.clone();
                            ed.persistence.commit(ed.revision(), CommandBatch::single("Instrument on another track", Command::SetTrackInstrument {
                                track: track.clone(), instrument: Some(crate::instrument::synth_spec(&dsp::SynthParams::default(), "test")),
                            })).unwrap();
                            ed.session.track = track;
                        }
                        _ => unreachable!(),
                    }
                    host.target = target_for(ed, &DeviceTarget::Track(ed.session.track.clone()), crate::instrument::INSTRUMENT_SLOT).unwrap();
                    host.desc = current_params(ed, &host.target).unwrap().into_iter().find(|d| d.id == "gain").unwrap();
                });
                cx.notify(); command
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                let before = host.editor.read(cx).revision();
                let (knob, start, _) = hold_suite_knob(window, cx);
                old_command(None, window, cx);
                assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"), "old {changed} command cannot retire the replacement owner");
                assert!(host.editor.read(cx).has_proto_pointer_contact());
                assert_eq!(host.editor.read(cx).revision(), before);
                release_suite_knob(start, window, cx);
                assert_eq!(host.editor.read(cx).revision().get(), before.get() + 1);
            }).unwrap();
        }
    }

    #[gpui::test]
    fn suite_commands_preserve_another_window_and_a_new_lease(cx: &mut TestAppContext) {
        let first = open(cx, true);
        let (editor, target, desc) = first.update(cx, |host, _, _| (host.editor.clone(), host.target.clone(), host.desc.clone())).unwrap();
        let second = cx.open_window(size(px(520.), px(220.)), |_, cx| Harness {
            editor, target, desc, show: true, outside: cx.focus_handle(), block_up: Rc::new(Cell::new(false)), with_knob: true, double: None,
        });
        crate::native_test_support::frames(second, cx);
        let (knob, old_start, accepted) = second.update(cx, |_, window, cx| hold_suite_knob(window, cx)).unwrap();
        first.update(cx, |host, window, cx| {
            let command = host.editor.update(cx, |ed, cx| suite::command(ed, host.target.clone(), suite::SuiteAction::CopyComparison, cx));
            command(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
            assert!(host.editor.read(cx).has_proto_pointer_contact());
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            assert!(host.editor.read(cx).device_anim.get(&host.target.key).is_none_or(|state| state.comparison.slots[0].is_none()),
                "a rival window cannot copy the other contact's preview");
        }).unwrap();
        let (_, replacement_start, _) = second.update(cx, |host, window, cx| {
            let command = host.editor.update(cx, |ed, cx| suite::command(ed, host.target.clone(), suite::SuiteAction::CopyComparison, cx));
            command(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            release_suite_knob(old_start, window, cx);
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            hold_suite_knob(window, cx)
        }).unwrap();
        crate::native_test_support::frames(second, cx);
        second.update(cx, |host, window, cx| {
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"), "queued cleanup cannot retire the replacement generation");
            release_suite_knob(replacement_start, window, cx);
            assert_eq!(host.editor.read(cx).revision().get(), 1);
        }).unwrap();
    }

    struct EqTabHarness {
        editor: Entity<Editor>, target: Target, desc: ParamDesc, tab_focus: FocusHandle,
    }

    impl Render for EqTabHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let tab = self.editor.update(cx, |ed, cx| eq_band_tab(ed, self.target.clone(), 2, EqParams::default().bands[2],
                selected_band(ed, self.target.key, dsp::EQ_BANDS) == 2, cx).track_focus(&self.tab_focus));
            div().size_full().flex().items_start().gap(px(20.))
                .child(crate::proto_knob::knob(self.editor.clone(), self.target.track.clone(), self.target.slot,
                    self.target.key, self.desc.clone(), "Gain", self.editor.read(cx).theme, 64.))
                .child(tab)
        }
    }

    #[gpui::test]
    fn eq_tab_element_routes_keyboard_selection_after_retiring_focus_preview(cx: &mut TestAppContext) {
        let main = open(cx, false);
        let (editor, target, desc) = main.update(cx, |host, _, _| (host.editor.clone(), host.target.clone(), host.desc.clone())).unwrap();
        let window = cx.open_window(size(px(520.), px(220.)), |_, cx| EqTabHarness { editor, target, desc, tab_focus: cx.focus_handle() });
        crate::native_test_support::frames(window, cx);
        let (knob, start, accepted) = window.update(cx, |_, window, cx| hold_suite_knob(window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| window.focus(&host.tab_focus, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "space");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(selected_band(host.editor.read(cx), host.target.key, dsp::EQ_BANDS), 2,
                "the exact EQ tab element used by eq_editor accepts keyboard activation");
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            release_suite_knob(start, window, cx);
            assert_eq!(host.editor.read(cx).revision().get(), 0);
        }).unwrap();
    }

    fn persist_missing_insert(host: &Harness, cx: &mut App) {
        host.editor.update(cx, |ed, _| {
            let spec = verdant_plugins::InsertSpec::new("clap", "org.verdant.test-missing", "Missing plug-in", "/not-installed/test.clap");
            ed.persistence.commit(ed.revision(), CommandBatch::single("Persist missing insert", Command::SetTrackInserts {
                track: host.target.track.track_id().unwrap().clone(), inserts: vec![serde_json::to_value(spec).unwrap()],
            })).unwrap();
        });
    }

    #[gpui::test]
    fn unloaded_device_menu_cancels_its_preview_and_preserves_another_window(cx: &mut TestAppContext) {
        let first = open(cx, true);
        first.update(cx, |host, _, cx| { persist_missing_insert(host, cx); cx.notify(); }).unwrap();
        crate::native_test_support::frames(first, cx);
        first.update(cx, |host, window, cx| {
            let track = host.target.track.track_id().unwrap().clone();
            let command = host.editor.update(cx, |ed, cx| chain_command(ed, track, ChainAction::Menu { slot: 0, expected_key: None }, cx));
            let before = host.editor.read(cx).revision();
            let (knob, start, accepted) = hold_suite_knob(window, cx);
            command(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            {
                use crate::menu::MenuCommand;
                use crate::editor::MenuAction;
                let ed = host.editor.read(cx);
                assert!(ed.plugins.is_none(), "the missing plug-in path needs no fabricated live host");
                let items = &ed.menu.as_ref().expect("the real menu command remains available for an unloaded insert").items;
                assert!(items.iter().any(|item| item.enabled && matches!(item.command, Some(MenuCommand::Editor(MenuAction::RemoveInsert { expected_key: None, .. })))));
                assert!(items.iter().any(|item| item.enabled && matches!(item.command, Some(MenuCommand::Editor(MenuAction::BypassInsert { expected_key: None, .. })))));
                assert!(items.iter().any(|item| !item.enabled && matches!(item.command, Some(MenuCommand::Editor(MenuAction::OpenInsertEditor { .. })))));
            }
            release_suite_knob(start, window, cx);
            assert_eq!(host.editor.read(cx).revision(), before, "opening a menu and late release cannot commit the held value");
            host.editor.update(cx, |ed, cx| ed.close_menu(window, cx));
        }).unwrap();
        let (editor, target, desc) = first.update(cx, |host, _, _| (host.editor.clone(), host.target.clone(), host.desc.clone())).unwrap();
        let second = cx.open_window(size(px(520.), px(220.)), |_, cx| Harness {
            editor, target, desc, show: true, outside: cx.focus_handle(), block_up: Rc::new(Cell::new(false)), with_knob: true, double: None,
        });
        crate::native_test_support::frames(second, cx);
        let (knob, start, _) = second.update(cx, |_, window, cx| hold_suite_knob(window, cx)).unwrap();
        first.update(cx, |host, window, cx| {
            let track = host.target.track.track_id().unwrap().clone();
            let command = host.editor.update(cx, |ed, cx| chain_command(ed, track, ChainAction::Menu { slot: 0, expected_key: None }, cx));
            command(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
            assert!(host.editor.read(cx).menu.is_none(), "a rival window cannot take over an active contact");
        }).unwrap();
        second.update(cx, |host, window, cx| {
            let before = host.editor.read(cx).revision();
            release_suite_knob(start, window, cx);
            assert_eq!(host.editor.read(cx).revision().get(), before.get() + 1);
        }).unwrap();
    }

    #[gpui::test]
    fn stale_chain_buttons_cannot_cancel_a_current_knob_or_change_the_replaced_slots(cx: &mut TestAppContext) {
        for action in [ChainAction::Bypass { slot: 0, expected_key: None }, ChainAction::Menu { slot: 0, expected_key: None },
            ChainAction::AddBuiltin(dsp::EQ_ID), ChainAction::Browse(1)] {
            let window = open(cx, true);
            let command = window.update(cx, |host, _, cx| {
                persist_missing_insert(host, cx);
                let track = host.target.track.track_id().unwrap().clone();
                let command = host.editor.update(cx, |ed, cx| chain_command(ed, track.clone(), action, cx));
                host.editor.update(cx, |ed, _| {
                    let replacement = verdant_plugins::InsertSpec::new("clap", "org.verdant.replacement", "Replacement", "/not-installed/replacement.clap");
                    let appended = verdant_plugins::InsertSpec::new("clap", "org.verdant.appended", "Appended", "/not-installed/appended.clap");
                    ed.persistence.commit(ed.revision(), CommandBatch::single("Replace chain", Command::SetTrackInserts {
                        track, inserts: vec![serde_json::to_value(replacement).unwrap(), serde_json::to_value(appended).unwrap()],
                    })).unwrap();
                });
                cx.notify(); command
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                let before = host.editor.read(cx).revision();
                let slots = host.editor.read(cx).insert_specs(host.target.track.track_id().unwrap());
                let (knob, start, _) = hold_suite_knob(window, cx);
                command(None, window, cx);
                assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
                assert!(host.editor.read(cx).menu.is_none());
                assert_eq!(host.editor.read(cx).revision(), before);
                assert_eq!(host.editor.read(cx).insert_specs(host.target.track.track_id().unwrap()), slots);
                release_suite_knob(start, window, cx);
                assert_eq!(host.editor.read(cx).revision().get(), before.get() + 1);
            }).unwrap();
        }
    }

    #[gpui::test]
    fn graph_up_routes_before_stopped_bubble_and_commits_its_final_sample(cx: &mut TestAppContext) {
        let window = open(cx, false);
        window.update(cx, |host, window, cx| {
            down(host, window, cx);
            host.block_up.set(true);
            let revision = host.editor.read(cx).revision();
            // No intervening move: only the final up crosses the threshold.
            up(window, cx);
            assert_eq!(gain(host, cx), host.desc.max);
            assert_eq!(host.editor.read(cx).revision().get(), revision.get() + 1);
            assert!(host.editor.read(cx).device_drag.is_none());
            assert!(window.captured_hitbox().is_none());
        }).unwrap();
    }

    #[gpui::test]
    fn graph_window_bounds_cancel_without_mouse_exit_and_reject_late_up(cx: &mut TestAppContext) {
        for terminal_only in [false, true] {
            let window = open(cx, false);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: start(host) + point(px(30.0), px(-10.0)),
                    pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                assert_eq!(host.editor.read(cx).device_drag.as_ref().unwrap().phase, GesturePhase::Dragging);
                let outside = if terminal_only { point(px(540.0), px(20.0)) } else { point(px(-20.0), px(60.0)) };
                // An implicit window grab need not emit MouseExited before up.
                if terminal_only {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                        position: outside, button: MouseButton::Left, ..Default::default()
                    }), cx);
                } else {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                        position: outside, pressed_button: Some(MouseButton::Left), ..Default::default()
                    }), cx);
                }
                assert!(host.editor.read(cx).device_drag.is_none());
                assert!(window.captured_hitbox().is_none());
                if !terminal_only { up(window, cx); }
                assert_eq!(host.editor.read(cx).revision().get(), 0);
                assert_eq!(gain(host, cx), host.desc.value);
            }).unwrap();
        }
    }

    #[gpui::test]
    fn graph_stale_track_slot_and_value_cannot_reach_the_store(cx: &mut TestAppContext) {
        for change in ["track", "slot", "value", "generation"] {
            let window = open(cx, false);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                let track = host.target.track.track_id().unwrap().clone();
                host.editor.update(cx, |editor, _| {
                    match change {
                        "track" => editor.session.track = editor.project().tracks[1].id.clone(),
                        "generation" => editor.document_generation += 1,
                        "slot" => {
                            editor.persistence.commit(editor.revision(), CommandBatch::single("Remove instrument", Command::SetTrackInstrument {
                                track: track.clone(), instrument: None,
                            })).unwrap();
                        }
                        "value" => {
                            let mut params = crate::instrument::synth_params(editor.project().track(&track).unwrap()).unwrap();
                            params.set("gain", -18.0).unwrap();
                            editor.persistence.commit(editor.revision(), CommandBatch::single("External value", Command::SetTrackInstrument {
                                track: track.clone(), instrument: Some(crate::instrument::synth_spec(&params, "test")),
                            })).unwrap();
                        }
                        _ => unreachable!(),
                    }
                });
                let revision = host.editor.read(cx).revision();
                up(window, cx);
                assert_eq!(host.editor.read(cx).revision(), revision, "old {change} drag must not commit");
                assert!(host.editor.read(cx).device_drag.is_none());
                assert!(window.captured_hitbox().is_none());
                if change != "slot" { assert_eq!(gain(host, cx), if change == "value" { -18.0 } else { host.desc.value }); }
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert!(host.editor.read(cx).device_drag.is_none());
                assert!(window.captured_hitbox().is_none());
            }).unwrap();
        }
    }

    #[gpui::test]
    fn graph_rejects_an_unmatched_key_even_when_cached_parameters_remain(cx: &mut TestAppContext) {
        let window = open(cx, false);
        window.update(cx, |host, _, cx| {
            host.target.key = InsertKey(host.target.key.0 ^ 1);
            host.editor.update(cx, |editor, _| { editor.plugin_params.insert(host.target.key, vec![host.desc.clone()]); });
            cx.notify();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position: start(host), button: MouseButton::Left, ..Default::default()
            }), cx);
            up(window, cx);
            assert!(host.editor.read(cx).device_drag.is_none());
            assert!(window.captured_hitbox().is_none());
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            assert_eq!(gain(host, cx), host.desc.value);
        }).unwrap();
    }

    #[gpui::test]
    fn graph_exit_blur_and_subtree_detach_cancel_without_waiting_for_up(cx: &mut TestAppContext) {
        for reason in ["exit", "blur", "detach"] {
            let window = open(cx, false);
            window.update(cx, |host, window, cx| down(host, window, cx)).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| match reason {
                "exit" => { window.dispatch_event(gpui::PlatformInput::MouseExited(gpui::MouseExitEvent::default()), cx); }
                "blur" => window.focus(&host.outside, cx),
                "detach" => { host.show = false; cx.notify(); }
                _ => unreachable!(),
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert!(host.editor.read(cx).device_drag.is_none(), "{reason} removed the preview");
                assert!(window.captured_hitbox().is_none(), "{reason} released native capture");
                up(window, cx);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
                assert_eq!(gain(host, cx), host.desc.value);
            }).unwrap();
        }
    }

    #[gpui::test]
    fn production_knob_touch_and_native_graph_mouse_cannot_steal_each_other(cx: &mut TestAppContext) {
        let window = open(cx, true);
        window.update(cx, |host, window, cx| {
            let touch_start = point(px(282.0), px(50.0));
            let touch = |phase, position, window: &mut Window, cx: &mut App| {
                window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
                    id: gpui::TouchId(42), phase, position, ..Default::default()
                }), cx);
            };
            touch(gpui::TouchPhase::Started, touch_start, window, cx);
            assert!(host.editor.read(cx).has_proto_pointer_contact());
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position: start(host), button: MouseButton::Left, ..Default::default()
            }), cx);
            up(window, cx);
            window.dispatch_event(gpui::PlatformInput::MouseExited(gpui::MouseExitEvent::default()), cx);
            assert!(host.editor.read(cx).has_proto_pointer_contact());
            assert!(host.editor.read(cx).device_drag.is_none());
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            touch(gpui::TouchPhase::Cancelled, touch_start, window, cx);
            assert!(!host.editor.read(cx).has_proto_pointer_contact());

            down(host, window, cx);
            touch(gpui::TouchPhase::Started, touch_start, window, cx);
            assert!(!host.editor.read(cx).has_proto_pointer_contact());
            assert!(host.editor.read(cx).device_drag.is_some());
            touch(gpui::TouchPhase::Cancelled, touch_start, window, cx);
            up(window, cx);
            assert_eq!(gain(host, cx), host.desc.max);
            assert_eq!(host.editor.read(cx).revision().get(), 1);
        }).unwrap();
    }

    #[gpui::test]
    fn graph_cancellation_follows_its_window_when_another_window_loses_focus(cx: &mut TestAppContext) {
        let first = open(cx, false);
        let (editor, target, desc) = first.update(cx, |host, window, cx| {
            let focus = host.editor.read(cx).focus_devices.clone();
            window.focus(&focus, cx);
            (host.editor.clone(), host.target.clone(), host.desc.clone())
        }).unwrap();
        crate::native_test_support::frames(first, cx);
        let second = cx.open_window(size(px(520.0), px(220.0)), |_, cx| {
            Harness { editor, target, desc, show: true, outside: cx.focus_handle(),
                block_up: Rc::new(Cell::new(false)), with_knob: false, double: None }
        });
        crate::native_test_support::frames(second, cx);
        second.update(cx, |host, window, cx| down(host, window, cx)).unwrap();
        crate::native_test_support::frames(second, cx);
        first.update(cx, |host, window, cx| {
            // Deliver the old window's blur after a different window acquired
            // the shared Editor's graph preview. This must not take its model.
            window.focus(&host.outside, cx);
        }).unwrap();
        crate::native_test_support::frames(first, cx);
        second.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).device_drag.is_some());
            assert!(window.captured_hitbox().is_some());
            host.editor.update(cx, |ed, cx| ed.cancel_edits(verdant_session::CancelReason::FocusLost, window, cx));
            assert!(host.editor.read(cx).device_drag.is_none());
        }).unwrap();
        crate::native_test_support::frames(second, cx);
        second.update(cx, |host, window, cx| {
            assert!(window.captured_hitbox().is_none());
            up(window, cx);
            assert_eq!(host.editor.read(cx).revision().get(), 0);
        }).unwrap();
    }

    struct RollWindow { editor: Entity<Editor> }

    impl Render for RollWindow {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().track_focus(&self.editor.read(cx).focus_roll)
                .child(crate::roll::piano_roll(self.editor.clone()))
        }
    }

    #[gpui::test]
    fn blank_graph_double_click_respects_a_live_touch_session_and_then_can_commit(cx: &mut TestAppContext) {
        let graph_window = cx.open_window(size(px(520.0), px(220.0)), |window, cx| {
            let mut project = crate::native_test_support::instrument_project();
            project.tracks[0].notes = vec![verdant_domain::Note::new(100, 100, 60)];
            project.tracks[0].refresh_index();
            let editor = cx.new(|cx| {
                let mut editor = Editor::for_native_test(project, window, cx);
                editor.needs_fit = false;
                editor.session.snap.grid_ticks = 0;
                editor.session.viewport.start_tick = 0.0;
                editor.session.viewport.px_per_tick = 1.0;
                editor.session.viewport.top_pitch = 65.0;
                editor.session.viewport.row_height = 20.0;
                editor
            });
            let track = editor.read(cx).session.track.clone();
            let key = crate::instrument::instrument_key_for_generation(&track, dsp::SYNTH_ID, editor.read(cx).document_generation);
            let target = Target { track: track.into(), slot: crate::instrument::INSTRUMENT_SLOT, key };
            let desc = target.basis(editor.read(cx)).unwrap().params.into_iter().find(|desc| desc.id == "gain").unwrap();
            Harness { editor, target, desc, show: true, outside: cx.focus_handle(),
                block_up: Rc::new(Cell::new(false)), with_knob: false,
                double: Some(Rc::new(|_| Some((vec![("gain".into(), -2.0)], None)))) }
        });
        let editor = graph_window.update(cx, |host, _, _| host.editor.clone()).unwrap();
        let roll_window = cx.open_window(size(px(480.0), px(180.0)), |_, _| RollWindow { editor });
        crate::native_test_support::frames(graph_window, cx);
        crate::native_test_support::frames(roll_window, cx);
        roll_window.update(cx, |host, window, cx| {
            for (phase, x) in [(gpui::TouchPhase::Started, 150.0), (gpui::TouchPhase::Moved, 180.0)] {
                window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
                    id: gpui::TouchId(83), phase, position: point(px(x), px(90.0)), ..Default::default()
                }), cx);
            }
            assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(verdant_session::PointerId::Touch(0)));
        }).unwrap();
        graph_window.update(cx, |host, window, cx| {
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left, click_count: 2, position: point(px(20.0), px(70.0)), ..Default::default()
            }), cx);
            assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(verdant_session::PointerId::Touch(0)), "the rival input cannot retire its owner");
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            assert_eq!(gain(host, cx), host.desc.value);
            up(window, cx);
        }).unwrap();
        roll_window.update(cx, |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
                id: gpui::TouchId(83), phase: gpui::TouchPhase::Cancelled,
                position: point(px(180.0), px(90.0)), ..Default::default()
            }), cx);
        }).unwrap();
        graph_window.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).session.pointer_owner().is_none());
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left, click_count: 2, position: point(px(20.0), px(70.0)), ..Default::default()
            }), cx);
            assert_eq!(host.editor.read(cx).revision().get(), 1, "normal blank double-click remains a domain command");
            assert_eq!(gain(host, cx), -2.0);
        }).unwrap();
    }
}
