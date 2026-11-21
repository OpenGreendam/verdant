//! Device view (bottom pane): the selected track's effect chain as device
//! cards from left to right, as in a DAW's device chain. Each built-in effect
//! has a graphical editor in the manner of dedicated mixing plug-ins: the
//! EQ's response curve with band handles (right-click for shape, slope and
//! stereo placement; double-click to add a band), transfer curves for the
//! dynamics with threshold handles, a decay-over-frequency graph for the
//! reverb and an echo graph for the delay. Other plug-ins get a knob card.
//! Displayed values glide to their new values after any external change.
//!
//! Interaction decisions come from the compiled Proto UI control protocol
//! (`planControlGesture`, `dragNormalized` in the pitch-gesture unit): a
//! press selects, only a drag past the threshold edits, the edit previews
//! locally and is committed once on release as a single undo step; double
//! click or Alt-click restores the default. Cancellation drops only the local
//! preview and releases its pointer capture, without writing plug-in state.

use gpui::{
    Context, DispatchPhase, Entity, FontWeight, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Role, SharedString, Window, canvas, div, fill, prelude::*, px, quad, svg,
};
use verdant_domain::TrackId;
use verdant_pitch_gesture_surface::PitchGestureSurfaceHandle;
use std::rc::Rc;
use std::time::{Duration, Instant};
use verdant_plugins::builtin_dsp::{self as dsp, BandKind, EqParams, Placement, TableEffect};
use verdant_plugins::{InsertKey, ParamDesc};

use crate::assets::icon;
use crate::editor::Editor;
use crate::generated::controls::{self, CommandArg, CommandId};
use crate::paint::{Texts, b, flush_texts, p, polyline};
use crate::theme::{Theme, with_alpha};

/// Which control a drag edits.
#[derive(Clone, Debug, PartialEq)]
pub enum Control { Knob(String), Band(usize), Handle(&'static str) }

/// A pressed or dragged control. Values preview locally and are committed
/// to the plug-in once, on release.
pub struct DeviceDrag {
    track: TrackId,
    slot: usize,
    key: InsertKey,
    control: Control,
    phase: String,
    origin: (f64, f64),
    window: gpui::AnyWindowHandle,
    hitbox: gpui::HitboxId,
    /// Normalized (0…1) start positions of the edited values.
    start: Vec<f64>,
    preview: Vec<(String, f64)>,
}

#[derive(serde::Deserialize)]
struct Decision {
    #[serde(rename = "nextPhase")]
    next_phase: String,
    effects: Vec<String>,
}

fn plan(phase: &str, event: &str, hit: bool, crossed: bool, reset: bool, double: bool) -> Decision {
    let value = PitchGestureSurfaceHandle::call_data_method("planControlGesture", vec![
        phase.into(),
        event.into(),
        serde_json::json!({ "hit_control": hit, "threshold_crossed": crossed, "reset_modifier": reset, "double_click": double }),
    ]).unwrap_or_else(|error| panic!("compiled control gesture planner failed: {error:?}"));
    serde_json::from_value(value).unwrap_or_else(|error| panic!("compiled control gesture planner returned an incompatible decision: {error}"))
}

fn drag_normalized(start: f64, delta: f64, travel: f64, fine: bool) -> f64 {
    PitchGestureSurfaceHandle::call_data_method("dragNormalized", vec![start.into(), delta.into(), travel.into(), fine.into()])
        .ok().and_then(|v| v.as_f64()).unwrap_or(start)
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
        "Hz" => format!("{v:.0} Hz"),
        "dB" => format!("{v:+.1} dB"),
        "ms" if v < 10.0 => format!("{v:.1} ms"),
        "ms" => format!("{v:.0} ms"),
        "%" => format!("{v:.0} %"),
        ":1" => format!("{v:.1}:1"),
        "" if d.id.ends_with("_q") || d.id == "q" => format!("{v:.2}"),
        "s" => format!("{v:.2} s"),
        "st" => format!("{v:+.0} 半音"),
        "ct" => format!("{v:.0} 音分"),
        "" if d.step_count == 1 => (if v >= 0.5 { "开" } else { "关" }).to_string(),
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

fn param<'a>(params: &'a [ParamDesc], id: &str) -> Option<&'a ParamDesc> { params.iter().find(|p| p.id == id) }

// ---------------------------------------------------------------------------
// Gesture plumbing shared by knobs and graphs
// ---------------------------------------------------------------------------

struct Target { track: TrackId, slot: usize, key: InsertKey }

impl Editor {
    /// Device previews never reach the plug-in until release. Discarding one
    /// restores the cached values and lets parameter refresh resume immediately.
    pub(crate) fn cancel_device_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.device_drag.take() else { return };
        let decision = plan(&drag.phase, "Cancel", false, false, false, false);
        if decision.effects.iter().any(|effect| effect == "Host.ReleasePointer") {
            // Undo and track changes do not have a Window borrow. Defer until
            // the current update returns, and release only this drag's capture.
            let (window, hitbox) = (drag.window, drag.hitbox);
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| {
                    if window.captured_hitbox() == Some(hitbox) {
                        window.release_pointer();
                    }
                });
            });
        }
        cx.notify();
    }
}

#[allow(clippy::too_many_arguments)]
fn press(ed: &mut Editor, target: &Target, control: Control, start: Vec<f64>, initial: Vec<(String, f64)>, defaults: Vec<(String, f64)>,
    ev: &MouseDownEvent, hitbox: gpui::HitboxId, window: &mut Window, cx: &mut Context<Editor>) {
    if ed.device_drag.is_some() { return; }
    let decision = plan("idle", "PointerDown", true, false, ev.modifiers.alt, ev.click_count >= 2);
    for effect in &decision.effects {
        match effect.as_str() {
            "Control.Select" => if let Control::Band(band) = control { ed.device_band = band; },
            "Control.Reset" => ed.set_track_insert_params(target.track.clone(), target.slot, defaults.clone(), cx),
            "Control.Begin" => ed.device_drag = Some(DeviceDrag {
                track: target.track.clone(), slot: target.slot, key: target.key, control: control.clone(),
                phase: decision.next_phase.clone(), origin: (f64::from(ev.position.x), f64::from(ev.position.y)),
                window: window.window_handle(), hitbox,
                start: start.clone(), preview: initial.clone(),
            }),
            "Host.CapturePointer" => window.capture_pointer(hitbox),
            _ => {}
        }
    }
    cx.notify();
}

fn moved(ed: &mut Editor, control: &Control, key: InsertKey, ev: &MouseMoveEvent, cx: &mut Context<Editor>,
    preview: impl Fn(&[f64], (f64, f64), bool) -> Vec<(String, f64)>) {
    let Some(drag) = ed.device_drag.as_mut().filter(|d| d.key == key && &d.control == control) else { return };
    let delta = (f64::from(ev.position.x) - drag.origin.0, f64::from(ev.position.y) - drag.origin.1);
    let crossed = delta.0.hypot(delta.1) >= 3.0;
    let decision = plan(&drag.phase, "PointerMove", true, crossed, false, false);
    if decision.effects.iter().any(|e| e == "Control.Preview") {
        drag.preview = preview(&drag.start, delta, ev.modifiers.shift);
    }
    drag.phase = decision.next_phase;
    cx.notify();
}

fn released(ed: &mut Editor, control: &Control, key: InsertKey, window: &mut Window, cx: &mut Context<Editor>) {
    if !ed.device_drag.as_ref().is_some_and(|d| d.key == key && &d.control == control) { return; }
    let drag = ed.device_drag.take().expect("checked drag");
    let decision = plan(&drag.phase, "PointerUp", true, false, false, false);
    for effect in decision.effects {
        match effect.as_str() {
            "Control.Commit" => ed.set_track_insert_params(drag.track.clone(), drag.slot, drag.preview.clone(), cx),
            "Host.ReleasePointer" => window.release_pointer(),
            _ => {}
        }
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Knob
// ---------------------------------------------------------------------------

const KNOB_SWEEP: f64 = 270.0;

fn arc_points(cx: f64, cy: f64, r: f64, from: f64, to: f64) -> Vec<Option<(f64, f64)>> {
    let steps = ((to - from).abs() / 4.0).ceil().max(1.0) as usize;
    (0..=steps).map(|i| {
        let a = (from + (to - from) * i as f64 / steps as f64).to_radians();
        Some((cx + r * a.cos(), cy + r * a.sin()))
    }).collect()
}

/// An Adwaita-styled rotary knob: trough arc, accent value arc (from zero
/// for bipolar ranges), round body with an indicator. Drag vertically.
fn knob(editor: Entity<Editor>, target: Target, desc: ParamDesc, label: &str, th: Theme, size: f32) -> impl IntoElement {
    let value_text = format_value(&desc, desc.value);
    let control = Control::Knob(desc.id.clone());
    let key = target.key;
    let paint_desc = desc.clone();
    div().id(SharedString::from(format!("knob-{}-{}", target.slot, desc.id))).role(Role::Slider)
        .aria_label(label.to_string()).aria_value(value_text.clone())
        .flex().flex_col().items_center().gap(px(2.0)).w(px(size + 26.0))
        .child(canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
            let o = bounds.origin;
            let s = f64::from(bounds.size.width.min(bounds.size.height));
            let (c, r) = (s / 2.0, s / 2.0 - 3.0);
            let start = 135.0;
            let n = to_norm(&paint_desc, paint_desc.value);
            let zero = if paint_desc.min < 0.0 && paint_desc.max > 0.0 { to_norm(&paint_desc, 0.0) } else { 0.0 };
            polyline(window, o, arc_points(c, c, r, start, start + KNOB_SWEEP), 3.5, th.control_pressed);
            let (a, z) = if n < zero { (n, zero) } else { (zero, n) };
            if (z - a) > 0.002 { polyline(window, o, arc_points(c, c, r, start + KNOB_SWEEP * a, start + KNOB_SWEEP * z), 3.5, th.accent); }
            let body = r - 7.0;
            window.paint_quad(quad(b(c - body, c - body, body * 2.0, body * 2.0, o), px(body as f32), th.control, px(1.0), th.shade, gpui::BorderStyle::Solid));
            let ang = (start + KNOB_SWEEP * n).to_radians();
            let (ix, iy) = (c + (body - 4.0) * ang.cos(), c + (body - 4.0) * ang.sin());
            window.paint_quad(quad(b(ix - 2.5, iy - 2.5, 5.0, 5.0, o), px(2.5), th.text, px(0.0), th.text, gpui::BorderStyle::Solid));
            if hitbox.is_hovered(window) { window.set_cursor_style(gpui::CursorStyle::ResizeUpDown, &hitbox); }

            let (e, h, t, d, ctl) = (editor.clone(), hitbox.clone(), (target.track.clone(), target.slot), paint_desc.clone(), control.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !h.is_hovered(window) { return; }
                let default = d.default_value.unwrap_or(d.value);
                e.update(cx, |ed, cx| press(ed, &Target { track: t.0.clone(), slot: t.1, key }, ctl.clone(),
                    vec![to_norm(&d, d.value)], vec![(d.id.clone(), d.value)], vec![(d.id.clone(), default)], ev, h.id, window, cx));
                cx.stop_propagation();
            });
            let (e, d, ctl) = (editor.clone(), paint_desc.clone(), control.clone());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble { return; }
                let d = d.clone();
                e.update(cx, |ed, cx| moved(ed, &ctl, key, ev, cx, move |start, delta, fine| {
                    vec![(d.id.clone(), from_norm(&d, drag_normalized(start[0], -delta.1, 180.0, fine)))]
                }));
            });
            let (e, ctl) = (editor.clone(), control.clone());
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left { return; }
                e.update(cx, |ed, cx| released(ed, &ctl, key, window, cx));
            });
            let _ = cx;
        }).size(px(size)))
        .child(div().text_xs().text_color(th.text_secondary).child(label.to_string()))
        .child(div().text_sm().text_color(th.text).whitespace_nowrap().child(value_text))
}

// ---------------------------------------------------------------------------
// Parameter animation
// ---------------------------------------------------------------------------

/// A device editor's displayed values easing toward the plug-in's values
/// after an external change (undo, a menu choice, a knob reset, automation
/// reads). Drags and their previews are shown directly.
pub struct DeviceAnim { from: Vec<f64>, to: Vec<f64>, start: Instant }

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
    let target: Vec<f64> = params.iter().map(|p| p.value).collect();
    let now = Instant::now();
    let settled = |target: Vec<f64>| DeviceAnim { from: target.clone(), to: target, start: now - ANIM };
    let anim = ed.device_anim.entry(key).or_insert_with(|| settled(target.clone()));
    if snap || anim.to.len() != target.len() { *anim = settled(target); return; }
    let mut t = now.duration_since(anim.start).as_secs_f64() / ANIM.as_secs_f64();
    if anim.to != target {
        let shown: Vec<f64> = params.iter().enumerate().map(|(i, d)| interpolate(d, anim.from[i], anim.to[i], t)).collect();
        *anim = DeviceAnim { from: shown, to: target, start: now };
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
pub struct Axis { pub min: f64, pub max: f64, pub log: bool }

impl Axis {
    pub const fn lin(min: f64, max: f64) -> Self { Self { min, max, log: false } }
    pub const fn log(min: f64, max: f64) -> Self { Self { min, max, log: true } }
    pub fn norm(&self, v: f64) -> f64 {
        let n = if self.log { (v.max(self.min) / self.min).ln() / (self.max / self.min).ln() } else { (v - self.min) / (self.max - self.min) };
        n.clamp(0.0, 1.0)
    }
    pub fn value(&self, n: f64) -> f64 {
        let n = n.clamp(0.0, 1.0);
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
        texts.push((label.clone(), p(x + 3.0, plot.h - 15.0, plot.o), 10.0, th.text_tertiary));
    }
    for (n, label) in ys {
        let y = plot.y(*n);
        let strong = zero.is_some_and(|z| (z - n).abs() < 1e-9);
        window.paint_quad(fill(b(0.0, y, plot.w, 1.0, plot.o), if strong { with_alpha(th.text, 0.16) } else { grid }));
        texts.push((label.clone(), p(4.0, y - 13.0, plot.o), 10.0, th.text_tertiary));
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
    let key = target.key;
    div().id(SharedString::from(format!("{name}-{}", target.slot))).role(Role::Image).aria_label(label.to_string())
        .rounded(px(10.0)).bg(th.view).border_1().border_color(th.shade).overflow_hidden()
        .child(canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
            let plot = Plot { o: bounds.origin, w: f64::from(bounds.size.width), h: f64::from(bounds.size.height) };
            let mut texts = Texts::new();
            painter(window, plot, &params, &mut texts);
            let points: Rc<Vec<(f64, f64)>> = Rc::new(handles.iter().map(|h| (plot.x(h.at.0), plot.y(h.at.1))).collect());
            let mouse = window.mouse_position();
            let near_points = points.clone();
            let near = move |pos: gpui::Point<Pixels>| -> Option<usize> {
                let points = &near_points;
                let (x, y) = (f64::from(pos.x - plot.o.x), f64::from(pos.y - plot.o.y));
                points.iter().enumerate().map(|(i, &(hx, hy))| (i, (hx - x).hypot(hy - y))).filter(|(_, d)| *d <= 13.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
            };
            let hovered = hitbox.is_hovered(window).then(|| near(mouse)).flatten();
            let dragging = editor.read(cx).device_drag.as_ref().filter(|d| d.key == key).map(|d| d.control.clone());
            for (i, (h, &(x, y))) in handles.iter().zip(points.iter()).enumerate() {
                let active = dragging.as_ref() == Some(&h.control);
                // State layers: hover grows the handle, a drag adds a halo.
                let r = if active { 9.0 } else if hovered == Some(i) || h.selected { 8.0 } else { 6.5 };
                if active || hovered == Some(i) {
                    let halo = r + 6.0;
                    window.paint_quad(quad(b(x - halo, y - halo, halo * 2.0, halo * 2.0, plot.o), px(halo as f32), with_alpha(h.color, 0.22), px(0.0), h.color, gpui::BorderStyle::Solid));
                }
                if h.selected {
                    window.paint_quad(quad(b(x - r - 2.0, y - r - 2.0, (r + 2.0) * 2.0, (r + 2.0) * 2.0, plot.o), px((r + 2.0) as f32), gpui::rgb(0xffffff), px(0.0), gpui::rgb(0xffffff), gpui::BorderStyle::Solid));
                }
                if h.hollow {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, plot.o), px(r as f32), th.view, px(2.0), with_alpha(h.color, 0.8), gpui::BorderStyle::Solid));
                } else {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, plot.o), px(r as f32), h.color, px(0.0), h.color, gpui::BorderStyle::Solid));
                }
                if !h.label.is_empty() {
                    let ink = if h.hollow { th.text_secondary } else { gpui::rgb(0x000000) };
                    texts.push((h.label.clone(), p(x - 3.2 * h.label.chars().count() as f64, y - 7.5, plot.o), 10.0, ink));
                }
            }
            flush_texts(window, cx, bounds, texts);
            if hovered.is_some() || dragging.is_some() { window.set_cursor_style(gpui::CursorStyle::PointingHand, &hitbox); }

            // Repaint when the hovered handle changes, so hover feedback follows the pointer.
            let hb = hitbox.clone();
            let near_move = near.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, _| {
                if phase != DispatchPhase::Bubble { return; }
                let now = hb.is_hovered(window).then(|| near_move(ev.position)).flatten();
                if now != hovered { window.refresh(); }
            });
            let (e, hb, t, ps, hs, near_down, menu, double) = (editor.clone(), hitbox.clone(), (target.track.clone(), target.slot), params.clone(), handles.clone(), near.clone(), menu.clone(), double.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || !hb.is_hovered(window) { return; }
                let target = Target { track: t.0.clone(), slot: t.1, key };
                match (ev.button, near_down(ev.position)) {
                    (MouseButton::Right, Some(i)) => {
                        if let Some(menu) = &menu {
                            e.update(cx, |ed, cx| {
                                if let Control::Band(band) = hs[i].control { ed.device_band = band; }
                                let items = menu(ed, i);
                                ed.show_menu(ev.position, items, window, cx);
                            });
                            cx.stop_propagation();
                        }
                    }
                    (MouseButton::Left, Some(i)) => {
                        let h = &hs[i];
                        let norm = |axis: &Option<(String, Axis)>| axis.as_ref().and_then(|(id, a)| param(&ps, id).map(|d| a.norm(d.value)));
                        let start = vec![norm(&h.x).unwrap_or(h.at.0), norm(&h.y).unwrap_or(h.at.1)];
                        let mut initial: Vec<(String, f64)> = [&h.x, &h.y].into_iter().flatten()
                            .filter_map(|(id, _)| param(&ps, id).map(|d| (id.clone(), d.value))).collect();
                        initial.extend(h.also.iter().cloned());
                        e.update(cx, |ed, cx| press(ed, &target, h.control.clone(), start, initial, h.defaults.clone(), ev, hb.id, window, cx));
                        cx.stop_propagation();
                    }
                    (MouseButton::Left, None) if ev.click_count >= 2 => {
                        let Some(double) = &double else { return };
                        let (x, y) = (f64::from(ev.position.x - plot.o.x), f64::from(ev.position.y - plot.o.y));
                        let n = (((x - GRAPH_PAD) / plot.inner_w()).clamp(0.0, 1.0), ((plot.h - GRAPH_PAD - y) / plot.inner_h()).clamp(0.0, 1.0));
                        if let Some((writes, select)) = double(n) {
                            e.update(cx, |ed, cx| {
                                if let Some(band) = select { ed.device_band = band; }
                                ed.set_track_insert_params(target.track.clone(), target.slot, writes, cx);
                            });
                        } else {
                            e.update(cx, |ed, cx| { ed.notice("所有频段都已在使用；右键频段可关闭它"); cx.notify(); });
                        }
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            });
            let (e, hs) = (editor.clone(), handles.clone());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble { return; }
                e.update(cx, |ed, cx| {
                    let Some(control) = ed.device_drag.as_ref().filter(|d| d.key == key).map(|d| d.control.clone()) else { return };
                    let Some(h) = hs.iter().find(|h| h.control == control).cloned() else { return };
                    moved(ed, &control, key, ev, cx, move |start, (dx, dy), fine| {
                        let mut out = Vec::new();
                        if let Some((id, axis)) = &h.x { out.push((id.clone(), axis.value(drag_normalized(start[0], dx, plot.inner_w(), fine)))); }
                        if let Some((id, axis)) = &h.y { out.push((id.clone(), axis.value(drag_normalized(start[1], -dy, plot.inner_h(), fine)))); }
                        out.extend(h.also.iter().cloned());
                        out
                    });
                });
            });
            let e = editor.clone();
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left { return; }
                e.update(cx, |ed, cx| {
                    if let Some(control) = ed.device_drag.as_ref().filter(|d| d.key == key).map(|d| d.control.clone()) {
                        released(ed, &control, key, window, cx);
                    }
                });
            });
        }).size_full())
}

// ---------------------------------------------------------------------------
// Shared widgets
// ---------------------------------------------------------------------------

/// A linked toggle group writing one stepped parameter.
fn choice(ed: &Editor, track: &TrackId, slot: usize, params: &[ParamDesc], id: &str, labels: &[&str], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let current = param(params, id).map_or(0, |d| d.value.round() as usize);
    div().flex().flex_none().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control).children(labels.iter().enumerate().map(|(i, label)| {
        let (t, id_owned) = (track.clone(), id.to_string());
        let on = i == current;
        div().id(SharedString::from(format!("choice-{slot}-{id}-{i}"))).role(Role::RadioButton).aria_selected(on).aria_label(label.to_string())
            .h(px(26.0)).px(px(10.0)).flex().items_center().rounded(px(6.0)).text_sm().cursor_pointer()
            .when(on, |d| d.bg(th.checked).font_weight(FontWeight::SEMIBOLD))
            .hover(move |s| s.bg(if on { th.checked } else { th.hover }))
            .active(move |s| s.bg(th.control_pressed))
            .child(label.to_string())
            .on_click(cx.listener(move |ed, _, _, cx| ed.set_track_insert_params(t.clone(), slot, vec![(id_owned.clone(), i as f64)], cx)))
    })).into_any_element()
}

/// A switch row writing a 0/1 parameter.
fn toggle(ed: &Editor, track: &TrackId, slot: usize, params: &[ParamDesc], id: &str, label: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let on = param(params, id).is_some_and(|d| d.value >= 0.5);
    let (t, id_owned) = (track.clone(), id.to_string());
    div().id(SharedString::from(format!("toggle-{slot}-{id}"))).role(Role::Switch).aria_label(label.to_string()).aria_selected(on)
        .flex().items_center().gap(px(8.0)).cursor_pointer().text_sm()
        .child(label.to_string()).child(crate::view::switch(on, th))
        .on_click(cx.listener(move |ed, _, _, cx| ed.set_track_insert_params(t.clone(), slot, vec![(id_owned.clone(), if on { 0.0 } else { 1.0 })], cx)))
        .into_any_element()
}

/// A dropdown-looking button that opens a menu (AdwComboRow's button).
pub(crate) fn dropdown_button(id: SharedString, label: String, th: Theme, items: impl Fn(&Editor) -> Vec<crate::menu::MenuItem> + 'static, cx: &mut Context<Editor>) -> gpui::AnyElement {
    div().id(id).role(Role::ComboBox).aria_label(label.clone()).h(px(30.0)).px(px(10.0)).flex().items_center().gap(px(6.0))
        .rounded(px(6.0)).bg(th.control).hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(th.checked)).cursor_pointer().text_sm()
        .child(div().max_w(px(120.0)).truncate().child(label))
        .child(svg().path(icon("chevron-down")).size(px(12.0)).text_color(th.text_secondary))
        .on_click(cx.listener(move |ed, ev: &gpui::ClickEvent, w, cx| { let items = items(ed); ed.show_menu(ev.position(), items, w, cx); }))
        .into_any_element()
}

/// Sidechain source picker: any other track's pre-insert signal.
fn sidechain_picker(ed: &Editor, track: &TrackId, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let spec = ed.insert_specs(track).get(slot).cloned();
    let source = spec.as_ref().and_then(|s| verdant_service::project_audio::sidechain_source(s).map(str::to_string));
    let source_name = source.as_ref().and_then(|id| ed.project().tracks.iter().find(|t| t.id.as_str() == id).map(|t| t.name.clone()));
    let label = match (&source, &source_name) { (None, _) => "无".to_string(), (Some(_), Some(name)) => name.clone(), (Some(_), None) => "已移除的轨道".into() };
    let me = track.clone();
    let items = move |ed: &Editor| {
        use crate::editor::MenuAction as M;
        use crate::menu::MenuItem as I;
        let mut items = vec![I::header("侧链来源"), I::action("无（检测输入本身）", M::SetSidechain { track: me.clone(), slot, source: None }).checked(source.is_none())];
        for t in ed.project().tracks.iter().filter(|t| t.id != me) {
            let on = source.as_deref() == Some(t.id.as_str());
            items.push(I::action(t.name.clone(), M::SetSidechain { track: me.clone(), slot, source: Some(t.id.clone()) }).checked(on));
        }
        items
    };
    div().flex().items_center().gap(px(8.0))
        .child(svg().path(icon("git-merge")).size(px(14.0)).text_color(th.text_secondary))
        .child(div().text_sm().text_color(th.text_secondary).child("侧链"))
        .child(dropdown_button(SharedString::from(format!("sidechain-{slot}")), label, th, items, cx))
        .into_any_element()
}

fn knob_row(editor: &Entity<Editor>, target: impl Fn() -> Target, params: &[ParamDesc], ids: &[&str], th: Theme, size: f32) -> gpui::Div {
    let mut row = div().flex().flex_wrap().gap(px(4.0));
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

fn eq_menu(track: TrackId, slot: usize, band: usize, eq: &EqParams) -> Vec<crate::menu::MenuItem> {
    use crate::editor::MenuAction as M;
    use crate::menu::MenuItem as I;
    let b = eq.bands[band];
    let set = |field: &str, v: f64| M::SetInsertParams { track: track.clone(), slot, values: vec![(dsp::eq_param_id(band, field), v)] };
    let mut items = vec![I::header(format!("频段 {}", band + 1))];
    for kind in BandKind::ALL { items.push(I::action(kind.label(), set("shape", kind.index() as f64)).checked(b.kind == kind)); }
    if b.kind.has_slope() {
        items.push(I::separator());
        for (i, slope) in dsp::SLOPES.iter().enumerate() { items.push(I::action(format!("{slope} dB/oct"), set("slope", i as f64)).checked(b.slope == i)); }
    }
    items.push(I::separator());
    let places = Placement::ALL.iter().map(|&pl| I::action(pl.label(), set("place", pl.index() as f64)).checked(b.place == pl)).collect();
    items.push(I::submenu("立体声位置", places));
    items.push(I::separator());
    items.push(I::action(if b.on { "关闭频段" } else { "开启频段" }, set("on", if b.on { 0.0 } else { 1.0 })));
    let (kind, f0, q0, _) = dsp::EQ_LAYOUT[band];
    items.push(I::action("重置频段", M::SetInsertParams { track: track.clone(), slot, values: vec![
        (dsp::eq_param_id(band, "freq"), f0), (dsp::eq_param_id(band, "gain"), 0.0), (dsp::eq_param_id(band, "q"), q0),
        (dsp::eq_param_id(band, "shape"), kind.index() as f64), (dsp::eq_param_id(band, "slope"), 1.0), (dsp::eq_param_id(band, "place"), 0.0)] }));
    items
}

fn eq_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let selected = ed.device_band.min(dsp::EQ_BANDS - 1);
    let eq = EqParams::from_descs(params);
    let target = || Target { track: track.clone(), slot, key };
    let band = eq.bands[selected];

    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let eq = EqParams::from_descs(params);
        let ys: Vec<(f64, String)> = [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0].iter().map(|&db| (GAIN_AXIS.norm(db), format!("{db:+.0}"))).collect();
        paint_grid(window, plot, th, &freq_grid(), &ys, Some(0.5), texts);
        let samples = 240;
        let freq_at = |i: usize| FREQ_AXIS.value(i as f64 / samples as f64);
        let rate = verdant_plugins::SAMPLE_RATE;
        // Each band's own curve, tinted, then the stereo sum on top.
        for (i, b) in eq.bands.iter().enumerate().filter(|(_, b)| b.on) {
            let pts: Vec<(f64, f64)> = (0..=samples).map(|k| { let f = freq_at(k); (plot.x(k as f64 / samples as f64), plot.y(GAIN_AXIS.norm(dsp::band_response_db(b, f, rate)))) }).collect();
            if i == selected { fill_under(window, plot, &pts, plot.y(0.5), with_alpha(band_color(i), 0.16)); }
            polyline(window, plot.o, pts.iter().map(|&q| Some(q)), if i == selected { 1.5 } else { 1.0 }, with_alpha(band_color(i), if i == selected { 0.85 } else { 0.4 }));
        }
        let curve: Vec<(f64, f64)> = (0..=samples).map(|k| { let f = freq_at(k); (plot.x(k as f64 / samples as f64), plot.y(GAIN_AXIS.norm(eq.response_db(f, rate)))) }).collect();
        fill_under(window, plot, &curve, plot.y(0.5), with_alpha(th.accent, 0.10));
        polyline(window, plot.o, curve.iter().map(|&q| Some(q)), 2.25, th.accent);
    });
    let handles: Vec<Handle> = eq.bands.iter().enumerate().map(|(i, b)| {
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
        let params = ed.live_insert(&menu_track, slot).and_then(|h| ed.plugin_params.get(&h.key()).cloned()).unwrap_or_default();
        eq_menu(menu_track.clone(), slot, i, &EqParams::from_descs(&params))
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

    let bands = div().flex().flex_none().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control).children((0..dsp::EQ_BANDS).map(|i| {
        let b = eq.bands[i];
        let on = i == selected;
        div().id(SharedString::from(format!("eq-band-{slot}-{i}"))).role(Role::Tab).aria_selected(on)
            .aria_label(format!("频段 {}：{}", i + 1, b.kind.label()))
            .h(px(26.0)).px(px(8.0)).flex().items_center().gap(px(5.0)).rounded(px(6.0)).text_sm().cursor_pointer()
            .when(on, |d| d.bg(th.checked)).hover(move |s| s.bg(if on { th.checked } else { th.hover })).active(move |s| s.bg(th.control_pressed))
            .child(div().size(px(9.0)).rounded(px(4.5)).bg(if b.on { band_color(i) } else { with_alpha(band_color(i), 0.3) }))
            .child(format!("{}", i + 1))
            .on_click(cx.listener(move |ed, _, _, cx| { ed.device_band = i; cx.notify(); }))
    }));
    let shape_track = track.clone();
    let shape = dropdown_button(SharedString::from(format!("eq-shape-{slot}")), band.kind.label().into(), th, move |ed| {
        let params = ed.live_insert(&shape_track, slot).and_then(|h| ed.plugin_params.get(&h.key()).cloned()).unwrap_or_default();
        eq_menu(shape_track.clone(), slot, selected, &EqParams::from_descs(&params))
    }, cx);
    let (toggle_track, toggle_id, on) = (track.clone(), dsp::eq_param_id(selected, "on"), band.on);
    let band_switch = div().id(SharedString::from(format!("eq-band-on-{slot}"))).role(Role::Switch).aria_label("频段开关").aria_selected(on)
        .cursor_pointer().child(crate::view::switch(on, th))
        .on_click(cx.listener(move |ed, _, _, cx| ed.set_track_insert_params(toggle_track.clone(), slot, vec![(toggle_id.clone(), if on { 0.0 } else { 1.0 })], cx)));
    let place_labels: Vec<&str> = Placement::ALL.iter().map(|p| match p { Placement::Stereo => "立体", other => other.short() }).collect();
    let mut knobs = div().flex().items_start().gap(px(4.0));
    for (field, label) in [("freq", "频率"), ("gain", "增益"), ("q", "Q")] {
        if field == "gain" && !band.kind.has_gain() { continue; }
        if let Some(d) = param(params, &dsp::eq_param_id(selected, field)) { knobs = knobs.child(knob(editor.clone(), target(), d.clone(), label, th, 44.0)); }
    }
    if let Some(d) = param(params, "output") { knobs = knobs.child(div().w(px(12.0))).child(knob(editor.clone(), target(), d.clone(), "输出", th, 44.0)); }
    let mut controls_row = div().flex().items_center().gap(px(10.0)).child(bands).child(div().flex_1()).child(shape);
    if band.kind.has_slope() {
        let slope_labels: Vec<String> = dsp::SLOPES.iter().map(|s| format!("{s}")).collect();
        let refs: Vec<&str> = slope_labels.iter().map(String::as_str).collect();
        controls_row = controls_row.child(choice(ed, track, slot, params, &dsp::eq_param_id(selected, "slope"), &refs, cx));
    }
    controls_row = controls_row.child(band_switch);
    div().flex().flex_col().gap(px(10.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "eq-graph", "均衡器频率响应：拖动频段点调频率和增益，右键换形状，双击空白处添加频段", th, painter, handles, Some(menu), Some(double)).h(px(220.0)).w_full())
        .child(controls_row)
        .child(div().flex().items_center().gap(px(12.0)).child(knobs).child(div().flex_1())
            .child(div().flex().flex_col().gap(px(4.0)).items_end()
                .child(div().text_xs().text_color(th.text_secondary).child("立体声位置"))
                .child(choice(ed, track, slot, params, &dsp::eq_param_id(selected, "place"), &place_labels, cx))))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Dynamics
// ---------------------------------------------------------------------------

const LEVEL_FLOOR: f64 = -60.0;

/// A static level transfer graph (input → output dBFS) with a threshold handle.
fn transfer_graph(editor: &Entity<Editor>, target: Target, params: &[ParamDesc], name: &str, label: &str, th: Theme,
    curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64>, floor: f64) -> gpui::AnyElement {
    let axis = Axis::lin(floor, 0.0);
    let threshold = param(params, "threshold").map_or(-18.0, |d| d.value);
    let c = curve.clone();
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let grid: Vec<(f64, String)> = (1..4).map(|k| { let db = floor * k as f64 / 4.0; (axis.norm(db), format!("{db:.0}")) }).collect();
        paint_grid(window, plot, th, &grid, &grid, None, texts);
        polyline(window, plot.o, [Some((plot.x(0.0), plot.y(0.0))), Some((plot.x(1.0), plot.y(1.0)))], 1.0, with_alpha(th.text, 0.18));
        let t = param(params, "threshold").map_or(-18.0, |d| d.value);
        let knee = param(params, "knee").map_or(0.0, |d| d.value);
        if knee > 0.0 {
            let (k0, k1) = (plot.x(axis.norm(t - knee / 2.0)), plot.x(axis.norm(t + knee / 2.0)));
            window.paint_quad(fill(b(k0, 0.0, (k1 - k0).max(1.0), plot.h, plot.o), with_alpha(th.accent, 0.08)));
        }
        let pts: Vec<(f64, f64)> = (0..=160).map(|i| { let x = floor + i as f64 * -floor / 160.0; (plot.x(axis.norm(x)), plot.y(axis.norm(c(params, x)))) }).collect();
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
    graph(editor.clone(), target, params.to_vec(), name, label, th, painter, vec![handle], None, None).size(px(200.0)).flex_none().into_any_element()
}

fn comp_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64> = Rc::new(|ps, x| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        (dsp::compressor_curve_db(v("threshold", -18.0), v("ratio", 3.0), v("knee", 6.0), x).max(x - v("range", 60.0)) + v("makeup", 0.0)).min(0.0)
    });
    div().flex().gap(px(16.0))
        .child(transfer_graph(editor, target(), params, "comp-graph", "压缩器传输曲线：左右拖动阈值", th, curve, LEVEL_FLOOR))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(10.0))
            .child(div().flex().items_center().gap(px(10.0)).child(choice(ed, track, slot, params, "style", &dsp::COMP_STYLES, cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
            .child(knob_row(editor, target, params, &["threshold", "ratio", "knee", "range", "makeup"], th, 42.0))
            .child(knob_row(editor, target, params, &["attack", "release", "lookahead", "mix", "sc_hpf"], th, 42.0))
            .child(toggle(ed, track, slot, params, "sc_listen", "监听侧链", cx)))
        .into_any_element()
}

fn gate_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let curve: Rc<dyn Fn(&[ParamDesc], f64) -> f64> = Rc::new(|ps, x| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        dsp::expander_curve_db(v("threshold", -40.0), v("ratio", 50.0), v("range", 40.0), x).max(-80.0)
    });
    div().flex().gap(px(16.0))
        .child(transfer_graph(editor, target(), params, "gate-graph", "门限传输曲线：左右拖动阈值", th, curve, -80.0))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(10.0))
            .child(div().flex().items_center().gap(px(10.0)).child(toggle(ed, track, slot, params, "sc_listen", "监听侧链", cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
            .child(knob_row(editor, target, params, &["threshold", "range", "ratio", "mix"], th, 42.0))
            .child(knob_row(editor, target, params, &["attack", "hold", "release", "lookahead", "sc_hpf", "sc_lpf"], th, 42.0)))
        .into_any_element()
}

fn deesser_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
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
    div().flex().flex_col().gap(px(10.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "deesser-graph", "齿音检测：左右拖动频率，上下拖动阈值", th, painter, vec![handle], None, None).h(px(150.0)).w_full())
        .child(div().flex().items_center().gap(px(10.0)).child(choice(ed, track, slot, params, "mode", &dsp::DEESSER_MODES, cx))
            .child(toggle(ed, track, slot, params, "sc_listen", "监听", cx)).child(div().flex_1()).child(sidechain_picker(ed, track, slot, cx)))
        .child(knob_row(editor, target, params, &["threshold", "range", "freq", "release"], th, 42.0))
        .into_any_element()
}

fn limiter_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc]) -> gpui::AnyElement {
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
        control: Control::Handle("ceiling"), at: (1.0, axis.norm(v("ceiling", -0.3))), x: None, y: Some(("ceiling".into(), Axis::lin(-12.0, 0.0))),
        color: gpui::rgb(0xffffff), label: String::new(), hollow: false, selected: false, also: vec![], defaults: vec![("ceiling".into(), -0.3)],
    };
    let mut handle = handle;
    // The ceiling axis covers −12…0 dB; keep the handle on the drawn line.
    handle.at.0 = 0.92;
    div().flex().gap(px(16.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "limiter-graph", "限制器传输曲线：上下拖动输出上限", th, painter, vec![handle], None, None).size(px(180.0)).flex_none())
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(10.0))
            .child(knob_row(editor, target, params, &["gain", "ceiling", "release"], th, 46.0))
            .child(knob_row(editor, target, params, &["lookahead", "link"], th, 46.0)))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Space and color
// ---------------------------------------------------------------------------

fn reverb_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc]) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let decay_axis = Axis::log(0.2, 20.0);
    // Decay over frequency: flat to the damping frequency, then shorter;
    // the post filters' corners bound the drawn band.
    let decay_at = |ps: &[ParamDesc], f: f64| {
        let v = |id: &str, d: f64| param(ps, id).map_or(d, |p| p.value);
        let (decay, damp) = (v("decay", 2.0), v("damping", 8000.0));
        if damp >= 19_999.0 || f <= damp { decay } else { (decay * (damp / f).sqrt()).max(0.2) }
    };
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let ys: Vec<(f64, String)> = [0.5, 1.0, 2.0, 5.0, 10.0].iter().map(|&s| (decay_axis.norm(s), format!("{s} s"))).collect();
        paint_grid(window, plot, th, &freq_grid(), &ys, None, texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let (low, high) = (v("low_cut", 120.0), v("high_cut", 12000.0));
        let pts: Vec<(f64, f64)> = (0..=200).map(|i| { let n = i as f64 / 200.0; (plot.x(n), plot.y(decay_axis.norm(decay_at(params, FREQ_AXIS.value(n))))) }).collect();
        let (xl, xh) = (plot.x(FREQ_AXIS.norm(low)), plot.x(FREQ_AXIS.norm(high)));
        let inside: Vec<(f64, f64)> = pts.iter().copied().filter(|(x, _)| *x >= xl && *x <= xh).collect();
        fill_under(window, plot, &inside, plot.h, with_alpha(th.accent, 0.14));
        polyline(window, plot.o, pts.iter().map(|&q| Some(q)), 2.25, th.accent);
        for x in [xl, xh] { window.paint_quad(fill(b(x, 0.0, 1.0, plot.h, plot.o), with_alpha(th.text, 0.25))); }
        let pre = v("predelay", 10.0);
        texts.push((format!("预延迟 {pre:.0} ms"), p(plot.w - 110.0, 6.0, plot.o), 11.0, th.text_secondary));
    });
    let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
    let decay = v("decay", 2.0);
    let mk = |control: &'static str, at: (f64, f64), x: Option<(String, Axis)>, y: Option<(String, Axis)>, default: Vec<(String, f64)>, label: &str| Handle {
        control: Control::Handle(control), at, x, y, color: gpui::rgb(0xffffff), label: label.into(), hollow: false, selected: false, also: vec![], defaults: default };
    let damping_axis = Axis::log(1000.0, 20000.0);
    let handles = vec![
        mk("decay", (FREQ_AXIS.norm(400.0), decay_axis.norm(decay)), None, Some(("decay".into(), decay_axis)), vec![("decay".into(), 2.0)], ""),
        mk("damping", (FREQ_AXIS.norm(v("damping", 8000.0)), decay_axis.norm(decay)), Some(("damping".into(), damping_axis)), None, vec![("damping".into(), 8000.0)], ""),
        mk("low_cut", (FREQ_AXIS.norm(v("low_cut", 120.0)), 0.12), Some(("low_cut".into(), Axis::log(20.0, 1000.0))), None, vec![("low_cut".into(), 120.0)], ""),
        mk("high_cut", (FREQ_AXIS.norm(v("high_cut", 12000.0)), 0.12), Some(("high_cut".into(), Axis::log(1000.0, 20000.0))), None, vec![("high_cut".into(), 12000.0)], ""),
    ];
    div().flex().flex_col().gap(px(10.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "reverb-graph", "混响衰减随频率：上下拖动衰减时间，左右拖动阻尼与高低切", th, painter, handles, None, None).h(px(160.0)).w_full())
        .child(knob_row(editor, target, params, &["predelay", "decay", "size", "damping", "diffusion"], th, 42.0))
        .child(knob_row(editor, target, params, &["width", "low_cut", "high_cut", "modulation", "mix"], th, 42.0))
        .into_any_element()
}

fn delay_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let time_axis = Axis::lin(0.0, 2000.0);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let xs: Vec<(f64, String)> = [250.0, 500.0, 1000.0, 1500.0].iter().map(|&ms| (time_axis.norm(ms), format!("{ms:.0} ms"))).collect();
        paint_grid(window, plot, th, &xs, &[], None, texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let (time, fb, pingpong) = (v("time", 375.0), v("feedback", 35.0) / 100.0, v("pingpong", 0.0) >= 0.5);
        // Dry hit, then each echo: height is its level, ping-pong alternates colour.
        window.paint_quad(fill(b(plot.x(0.0) - 2.0, plot.y(1.0), 4.0, plot.y(0.0) - plot.y(1.0), plot.o), with_alpha(th.text, 0.35)));
        let mut level = 1.0;
        for n in 1..=24 {
            let t = time * n as f64;
            if t > 2000.0 || level < 0.01 { break; }
            let color = if pingpong && n % 2 == 0 { gpui::rgb(0x33d17a) } else { th.accent };
            let (x, top) = (plot.x(time_axis.norm(t)), plot.y(level));
            window.paint_quad(quad(b(x - 3.0, top, 6.0, plot.y(0.0) - top, plot.o), px(3.0), color, px(0.0), color, gpui::BorderStyle::Solid));
            level *= fb;
        }
    });
    let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
    let (time, fb) = (v("time", 375.0), v("feedback", 35.0));
    let mk = |control: &'static str, at: (f64, f64), x: Option<(String, Axis)>, y: Option<(String, Axis)>, default: (String, f64)| Handle {
        control: Control::Handle(control), at, x, y, color: gpui::rgb(0xffffff), label: String::new(), hollow: false, selected: false, also: vec![], defaults: vec![default] };
    let mut handles = vec![mk("time", (time_axis.norm(time), 1.0), Some(("time".into(), Axis::log(1.0, 2000.0))), None, ("time".into(), 375.0))];
    if time * 2.0 <= 2000.0 {
        handles.push(mk("feedback", (time_axis.norm(time * 2.0), fb / 100.0), None, Some(("feedback".into(), Axis::lin(0.0, 100.0))), ("feedback".into(), 35.0)));
    }
    div().flex().flex_col().gap(px(10.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "delay-graph", "回声：左右拖动第一个回声改时间，上下拖动第二个回声改反馈", th, painter, handles, None, None).h(px(140.0)).w_full())
        .child(div().flex().items_center().gap(px(10.0)).child(toggle(ed, track, slot, params, "pingpong", "乒乓", cx)))
        .child(knob_row(editor, target, params, &["time", "offset", "feedback", "drive", "low_cut", "high_cut", "width", "mix"], th, 42.0))
        .into_any_element()
}

fn saturation_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let grid = vec![(0.25, String::new()), (0.5, "0".to_string()), (0.75, String::new())];
        paint_grid(window, plot, th, &grid, &grid, Some(0.5), texts);
        let v = |id: &str, d: f64| param(params, id).map_or(d, |p| p.value);
        let mut fx = dsp::SaturationProcessor::new(verdant_plugins::SAMPLE_RATE);
        for id in ["drive", "style", "output"] { let _ = fx.set(id, v(id, 0.0)); }
        polyline(window, plot.o, [Some((plot.x(0.0), plot.y(0.0))), Some((plot.x(1.0), plot.y(1.0)))], 1.0, with_alpha(th.text, 0.18));
        let pts = (0..=160).map(|i| { let x = -1.0 + i as f64 / 80.0; Some((plot.x((x + 1.0) / 2.0), plot.y(((fx.curve(x) + 1.0) / 2.0).clamp(-0.05, 1.05)))) });
        polyline(window, plot.o, pts, 2.25, th.accent);
    });
    div().flex().gap(px(16.0))
        .child(graph(editor.clone(), target(), params.to_vec(), "saturation-graph", "饱和传输曲线", th, painter, vec![], None, None).size(px(170.0)).flex_none())
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(10.0))
            .child(choice(ed, track, slot, params, "style", &dsp::SATURATION_STYLES, cx))
            .child(knob_row(editor, target, params, &["drive", "tone", "mix", "output"], th, 46.0)))
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
        mk("attack", pts[1], Some(("attack".into(), attack)), None, vec![("attack".into(), 5.0)]),
        mk("decay", pts[2], Some(("decay".into(), decay)), Some(("sustain".into(), sustain)), vec![("decay".into(), 300.0), ("sustain".into(), 70.0)]),
        mk("release", pts[4], Some(("release".into(), release)), None, vec![("release".into(), 250.0)]),
    ];
    let mut waves = |id: &str, label: &str| div().flex().items_center().gap(px(8.0))
        .child(div().w(px(64.0)).text_sm().text_color(th.text_secondary).child(label.to_string()))
        .child(choice(ed, track, slot, params, id, &dsp::WAVES, cx));
    let (osc1, osc2) = (waves("osc1", "振荡器 1"), waves("osc2", "振荡器 2"));
    div().flex().flex_col().gap(px(10.0))
        .child(div().flex().gap(px(16.0))
            .child(graph(editor.clone(), target(), params.to_vec(), "synth-adsr", "音量包络：拖动起音、衰减/延音与释音点", th, painter, handles, None, None).h(px(140.0)).flex_1())
            .child(div().flex().flex_col().gap(px(8.0)).justify_center()
                .child(osc1).child(osc2)))
        .child(knob_row(editor, target, params, &["osc2_semi", "detune", "osc2_mix", "width", "gain"], th, 42.0))
        .child(knob_row(editor, target, params, &["attack", "decay", "sustain", "release", "cutoff", "resonance", "env_amount"], th, 42.0))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Device cards and the view
// ---------------------------------------------------------------------------

fn generic_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc]) -> gpui::AnyElement {
    let th = ed.theme;
    let shown: Vec<&ParamDesc> = params.iter().filter(|p| p.max > p.min).take(8).collect();
    let mut grid = div().flex().flex_wrap().gap(px(4.0)).w(px(260.0));
    for d in &shown {
        grid = grid.child(knob(editor.clone(), Target { track: track.clone(), slot, key }, (*d).clone(), &d.name, th, 40.0));
    }
    let more = params.len().saturating_sub(shown.len());
    div().flex().flex_col().gap(px(8.0)).child(grid)
        .when(params.is_empty(), |d| d.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数")))
        .when(more > 0, |d| d.child(div().text_xs().text_color(th.text_secondary).child(format!("另有 {more} 个参数在混音器的参数面板中"))))
        .into_any_element()
}

/// Card width for each built-in editor.
fn card_width(id: &str) -> Option<f32> {
    Some(match id {
        dsp::EQ_ID => 640.0,
        dsp::COMPRESSOR_ID => 600.0,
        dsp::GATE_ID => 680.0,
        dsp::DEESSER_ID | dsp::REVERB_ID | dsp::DELAY_ID => 560.0,
        dsp::LIMITER_ID | dsp::SATURATION_ID => 480.0,
        _ => return None,
    })
}

#[allow(clippy::too_many_arguments)]
fn device_card(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, params: Option<(InsertKey, Vec<ParamDesc>)>, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let specs = ed.insert_specs(track);
    let spec = &specs[slot];
    let handle = ed.live_insert(track, slot);
    let builtin = (spec.format == "builtin").then_some(spec.id.as_str());
    let on = !spec.bypass;
    let arg = CommandArg::TrackSlot(track.clone(), slot as u32);
    let (bypass_arg, menu_track) = (arg.clone(), track.clone());
    let keyed = verdant_service::project_audio::sidechain_source(spec).is_some();
    let title = builtin.and_then(dsp::short_label).map_or(spec.name.clone(), str::to_string);
    let header = div().flex().items_center().gap(px(10.0)).h(px(32.0))
        .child(div().id(SharedString::from(format!("device-power-{slot}"))).role(Role::Switch).aria_label("启用效果器").aria_selected(on).cursor_pointer()
            .child(crate::view::switch(on, th))
            .on_click(cx.listener(move |ed, _, w, cx| controls::dispatch(ed, CommandId::PluginBypass, bypass_arg.clone(), w, cx))))
        .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).text_color(if on { th.text } else { th.text_secondary }).child(title))
        .when(keyed, |d| d.child(div().px(px(8.0)).h(px(22.0)).flex().items_center().rounded(px(11.0)).bg(with_alpha(th.accent, 0.18)).text_xs().text_color(th.accent).child("侧链")))
        .child(div().id(SharedString::from(format!("device-menu-{slot}"))).role(Role::Button).aria_label("效果器菜单").size(px(28.0)).rounded(px(6.0))
            .flex().items_center().justify_center().hover(move |s| s.bg(th.hover)).active(move |s| s.bg(th.control_pressed)).cursor_pointer()
            .child(svg().path(icon("menu")).size(px(14.0)).text_color(th.text))
            .on_click(cx.listener(move |ed, ev: &gpui::ClickEvent, w, cx| {
                use crate::editor::MenuAction as M;
                use crate::menu::MenuItem as I;
                let items = vec![
                    I::action("打开原生插件窗口", M::OpenInsertEditor { track: menu_track.clone(), slot }).enabled(ed.live_insert(&menu_track, slot).is_some_and(|h| h.has_editor())),
                    I::action(if on { "旁通" } else { "启用" }, M::BypassInsert { track: menu_track.clone(), slot }),
                    I::separator(),
                    I::action("移除效果器", M::RemoveInsert { track: menu_track.clone(), slot }),
                ];
                ed.show_menu(ev.position(), items, w, cx);
            })));
    let body = match params {
        Some((key, params)) => match builtin {
            Some(dsp::EQ_ID) => eq_editor(ed, editor, track, slot, key, &params, cx),
            Some(dsp::COMPRESSOR_ID) => comp_editor(ed, editor, track, slot, key, &params, cx),
            Some(dsp::GATE_ID) => gate_editor(ed, editor, track, slot, key, &params, cx),
            Some(dsp::DEESSER_ID) => deesser_editor(ed, editor, track, slot, key, &params, cx),
            Some(dsp::LIMITER_ID) => limiter_editor(ed, editor, track, slot, key, &params),
            Some(dsp::REVERB_ID) => reverb_editor(ed, editor, track, slot, key, &params),
            Some(dsp::DELAY_ID) => delay_editor(ed, editor, track, slot, key, &params, cx),
            Some(dsp::SATURATION_ID) => saturation_editor(ed, editor, track, slot, key, &params, cx),
            _ => generic_editor(ed, editor, track, slot, key, &params),
        },
        None => div().h(px(120.0)).flex().items_center().justify_center().text_sm().text_color(th.text_secondary)
            .child(if handle.is_some() { "正在读取参数…" } else { "插件未载入（标识与状态已保留）" }).into_any_element(),
    };
    let width = builtin.and_then(card_width);
    div().id(SharedString::from(format!("device-{slot}"))).flex_none().flex().flex_col().gap(px(10.0)).p(px(14.0))
        .rounded(px(12.0)).bg(th.card).shadow_sm().when(!on, |d| d.opacity(0.72))
        .when_some(width, |d, w| d.w(px(w)))
        .child(header).child(body)
        .into_any_element()
}

fn add_card(ed: &Editor, track: &TrackId, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let browse_track = track.clone();
    let pill = |id: SharedString, label: &str| div().id(id).role(Role::Button).aria_label(label.to_string()).h(px(32.0)).px(px(12.0))
        .flex().items_center().rounded(px(6.0)).bg(th.control).hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(th.checked))
        .cursor_pointer().text_sm().child(label.to_string());
    let effects = dsp::EFFECT_IDS.iter().map(|&id| {
        let t = track.clone();
        pill(SharedString::from(format!("device-add-{id}")), dsp::short_label(id).unwrap_or(id))
            .on_click(cx.listener(move |ed, _, _, cx| ed.add_insert(&t, "builtin", id, cx)))
    });
    div().id("device-add").flex_none().w(px(220.0)).flex().flex_col().gap(px(8.0)).p(px(14.0)).rounded(px(12.0))
        .border_1().border_color(th.shade)
        .child(div().text_sm().font_weight(FontWeight::BOLD).child("添加效果器"))
        .child(div().flex().flex_wrap().gap(px(6.0)).children(effects))
        .child(pill("device-add-browse".into(), "浏览插件…").on_click(cx.listener(move |ed, _, _, cx| ed.open_plugin_browser(browse_track.clone(), slot, cx))))
        .into_any_element()
}

/// The device view for the selected track.
pub fn device_view(ed: &mut Editor, editor: &Entity<Editor>, window: &mut Window, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let track = ed.session.track.clone();
    let Some(name) = ed.project().track(&track).map(|t| t.name.clone()) else {
        return div().size_full().bg(th.view).into_any_element();
    };
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
    let empty = cards.is_empty();
    div().id("device-view").role(Role::Group).aria_label(format!("{name} 的效果器链"))
        .size_full().flex().flex_col().bg(th.view)
        .child(div().h(px(40.0)).flex_none().px(px(16.0)).flex().items_center().gap(px(8.0))
            .child(svg().path(icon("sliders-horizontal")).size(px(15.0)).text_color(th.text_secondary))
            .child(div().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(div().text_sm().text_color(th.text_secondary).child(if empty { "· 还没有效果器".to_string() } else { format!("· {count} 个效果器，信号从左到右") })))
        .child(div().id("device-chain").flex_1().min_h_0().overflow_x_scroll().overflow_y_scroll().flex().items_start().gap(px(12.0)).px(px(12.0)).pb(px(12.0))
            .children(cards)
            .child(add_card(ed, &track, count, cx)))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(format_value(&db, 3.0), "+3.0 dB");
    }

    #[test]
    fn compiled_control_protocol_selects_drags_commits_and_resets() {
        let down = plan("idle", "PointerDown", true, false, false, false);
        assert_eq!(down.next_phase, "armed");
        assert_eq!(down.effects, ["Control.Select", "Control.Begin", "Host.CapturePointer"]);
        assert_eq!(plan("armed", "PointerMove", true, false, false, false).effects, Vec::<String>::new());
        assert_eq!(plan("armed", "PointerMove", true, true, false, false).next_phase, "dragging");
        assert_eq!(plan("dragging", "PointerUp", true, false, false, false).effects, ["Control.Commit", "Host.ReleasePointer"]);
        assert_eq!(plan("armed", "PointerUp", true, false, false, false).effects, ["Control.Discard", "Host.ReleasePointer"]);
        assert_eq!(plan("idle", "PointerDown", true, false, false, true).effects, ["Control.Select", "Control.Reset"]);
        assert_eq!(plan("idle", "PointerDown", true, false, true, false).effects, ["Control.Select", "Control.Reset"]);
        assert!(plan("idle", "PointerDown", false, false, false, false).effects.is_empty());
        assert_eq!(drag_normalized(0.5, 90.0, 180.0, false), 1.0);
        assert!((drag_normalized(0.5, 90.0, 180.0, true) - 0.55).abs() < 1e-9, "Shift is ten times finer");
        assert_eq!(drag_normalized(0.2, -500.0, 180.0, false), 0.0);
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
        for phase in ["armed", "dragging"] {
            let cancel = plan(phase, "Cancel", false, false, false, false);
            assert_eq!(cancel.next_phase, "idle");
            assert_eq!(cancel.effects, ["Control.Discard", "Host.ReleasePointer"]);
            assert!(plan(&cancel.next_phase, "PointerUp", true, false, false, false).effects.is_empty(),
                "a late release must not commit a cancelled preview");
            let next = plan(&cancel.next_phase, "PointerDown", true, false, false, false);
            assert_eq!(next.next_phase, "armed");
            assert!(next.effects.iter().any(|effect| effect == "Control.Begin"),
                "cancellation must allow the next control edit");
        }
    }
}
