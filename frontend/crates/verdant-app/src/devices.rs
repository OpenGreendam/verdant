//! Device view (bottom pane): the selected track's effect chain as device
//! cards from left to right, as in a DAW's device chain. The built-in EQ and
//! compressor get graphical editors (a response curve with band handles, a
//! transfer curve with a threshold handle, rotary knobs); other plug-ins get
//! a generic knob card.
//!
//! Interaction decisions come from the compiled Proto UI control protocol
//! (`planControlGesture`, `dragNormalized` in the pitch-gesture unit): a
//! press selects, only a drag past the threshold edits, the edit previews
//! locally and is committed once on release as a single undo step; double
//! click or Alt-click restores the default. Cancellation drops only the local
//! preview and releases its pointer capture, without writing plug-in state.

use gpui::{
    App, Bounds, Context, DispatchPhase, Entity, FontWeight, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Role, SharedString, Window, canvas, div, fill, prelude::*, px, quad, svg,
};
use verdant_domain::TrackId;
use verdant_pitch_gesture_surface::PitchGestureSurfaceHandle;
use verdant_plugins::builtin_dsp::{self as dsp, BandKind, CompParams, EqParams};
use verdant_plugins::{InsertKey, ParamDesc};

use crate::assets::icon;
use crate::editor::Editor;
use crate::generated::controls::{self, CommandArg, CommandId};
use crate::paint::{Texts, b, flush_texts, p, polyline};
use crate::theme::{Theme, with_alpha};

/// Which control a drag edits.
#[derive(Clone, Debug, PartialEq)]
pub enum Control { Knob(String), Band(usize), Threshold }

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
    match d.unit.as_str() {
        "Hz" if v >= 1000.0 => format!("{:.2} kHz", v / 1000.0),
        "Hz" => format!("{v:.0} Hz"),
        "dB" => format!("{v:+.1} dB"),
        "ms" if v < 10.0 => format!("{v:.1} ms"),
        "ms" => format!("{v:.0} ms"),
        "%" => format!("{v:.0} %"),
        ":1" => format!("{v:.1}:1"),
        "" if d.id.ends_with("_q") => format!("{v:.2}"),
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

fn eq_from(params: &[ParamDesc]) -> EqParams {
    let mut eq = EqParams::default();
    for p in params { let _ = eq.set(&p.id, p.value); }
    eq
}

fn comp_from(params: &[ParamDesc]) -> CompParams {
    let mut c = CompParams::default();
    for p in params { let _ = c.set(&p.id, p.value); }
    c
}

/// GNOME palette colours for the six EQ bands.
const BAND_COLORS: [u32; 6] = [0x3584e4, 0x33d17a, 0xf6d32d, 0xff7800, 0xe66100, 0x9141ac];
fn band_color(i: usize) -> gpui::Rgba { gpui::rgb(BAND_COLORS[i % BAND_COLORS.len()]) }

fn band_label(kind: BandKind) -> &'static str {
    match kind { BandKind::LowCut => "低切", BandKind::LowShelf => "低架", BandKind::Bell => "峰值", BandKind::HighShelf => "高架", BandKind::HighCut => "高切" }
}

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
// EQ editor
// ---------------------------------------------------------------------------

const EQ_DB: f64 = 24.0;

fn eq_x(freq: f64, w: f64) -> f64 { w * (freq / dsp::FREQ_MIN).ln() / (dsp::FREQ_MAX / dsp::FREQ_MIN).ln() }
fn eq_y(db: f64, h: f64) -> f64 { h / 2.0 - db / EQ_DB * (h / 2.0 - 10.0) }

fn eq_graph(editor: Entity<Editor>, target: Target, params: Vec<ParamDesc>, selected: usize, th: Theme) -> impl IntoElement {
    let key = target.key;
    div().id(SharedString::from(format!("eq-graph-{}", target.slot))).role(Role::Image).aria_label("均衡器频率响应曲线")
        .h(px(210.0)).w_full().rounded(px(10.0)).bg(th.view).overflow_hidden()
        .child(canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
            let o = bounds.origin;
            let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
            let eq = eq_from(&params);
            let grid = with_alpha(th.text, 0.07);
            let mut texts = Texts::new();
            for f in [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0] {
                let x = eq_x(f, w);
                window.paint_quad(fill(b(x, 0.0, 1.0, h, o), grid));
                let label = if f >= 1000.0 { format!("{}k", f / 1000.0) } else { format!("{f}") };
                texts.push((label, p(x + 3.0, h - 15.0, o), 10.0, th.text_tertiary));
            }
            for db in [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0] {
                let y = eq_y(db, h);
                window.paint_quad(fill(b(0.0, y, w, 1.0, o), if db == 0.0 { with_alpha(th.text, 0.18) } else { grid }));
                texts.push((format!("{db:+.0}"), p(4.0, y - 13.0, o), 10.0, th.text_tertiary));
            }
            // Selected band's own contribution, faint, then the sum.
            let band = eq.bands[selected];
            let samples = 220;
            let freq_at = |i: usize| dsp::FREQ_MIN * (dsp::FREQ_MAX / dsp::FREQ_MIN).powf(i as f64 / samples as f64);
            if band.on {
                let pts = (0..=samples).map(|i| { let f = freq_at(i); Some((eq_x(f, w), eq_y(dsp::band_response_db(&band, f, verdant_plugins::SAMPLE_RATE), h))) });
                polyline(window, o, pts, 1.5, with_alpha(band_color(selected), 0.6));
            }
            let curve: Vec<(f64, f64)> = (0..=samples).map(|i| { let f = freq_at(i); (eq_x(f, w), eq_y(eq.response_db(f, verdant_plugins::SAMPLE_RATE), h).clamp(-2.0, h + 2.0)) }).collect();
            let mut area = gpui::PathBuilder::fill();
            area.move_to(p(curve[0].0, eq_y(0.0, h), o));
            for &(x, y) in &curve { area.line_to(p(x, y, o)); }
            area.line_to(p(curve[curve.len() - 1].0, eq_y(0.0, h), o));
            area.close();
            if let Ok(path) = area.build() { window.paint_path(path, with_alpha(th.accent, 0.16)); }
            polyline(window, o, curve.iter().map(|&q| Some(q)), 2.0, th.accent);
            // Band handles: filled when on, ringed when selected, hollow when off.
            let handles: Vec<(f64, f64)> = eq.bands.iter().map(|b| (eq_x(b.freq, w), eq_y(if b.kind.has_gain() { b.gain_db } else { 0.0 }, h))).collect();
            for (i, (bd, &(x, y))) in eq.bands.iter().zip(&handles).enumerate() {
                let color = band_color(i);
                let r = if i == selected { 8.0 } else { 6.5 };
                if i == selected { window.paint_quad(quad(b(x - r - 2.0, y - r - 2.0, (r + 2.0) * 2.0, (r + 2.0) * 2.0, o), px((r + 2.0) as f32), gpui::rgb(0xffffff), px(0.0), gpui::rgb(0xffffff), gpui::BorderStyle::Solid)); }
                if bd.on {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, o), px(r as f32), color, px(0.0), color, gpui::BorderStyle::Solid));
                } else {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, o), px(r as f32), th.view, px(2.0), with_alpha(color, 0.7), gpui::BorderStyle::Solid));
                }
                texts.push(((i + 1).to_string(), p(x - 3.0, y - 7.5, o), 10.0, if bd.on { gpui::rgb(0x000000) } else { th.text_secondary }));
            }
            flush_texts(window, cx, bounds, texts);
            let hit = move |pos: gpui::Point<Pixels>| -> Option<usize> {
                let (x, y) = (f64::from(pos.x - o.x), f64::from(pos.y - o.y));
                handles.iter().enumerate().map(|(i, &(hx, hy))| (i, (hx - x).hypot(hy - y))).filter(|(_, d)| *d <= 12.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
            };
            let (e, hb, t, ps) = (editor.clone(), hitbox.clone(), (target.track.clone(), target.slot), params.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !hb.is_hovered(window) { return; }
                let Some(i) = hit(ev.position) else { return };
                let eq = eq_from(&ps);
                let band = eq.bands[i];
                let (freq_id, gain_id, q_id, on_id) = (dsp::eq_param_id(i, "freq"), dsp::eq_param_id(i, "gain"), dsp::eq_param_id(i, "q"), dsp::eq_param_id(i, "on"));
                let (kind, f0, q0, on0) = dsp::EQ_LAYOUT[i];
                let defaults = vec![(freq_id.clone(), f0), (gain_id.clone(), 0.0), (q_id, q0), (on_id.clone(), if on0 { 1.0 } else { 0.0 })];
                let start = vec![eq_x(band.freq, w) / w, 1.0 - eq_y(band.gain_db, h) / h];
                // Dragging a band that is off turns it on, as in parametric EQ editors.
                let mut initial = vec![(freq_id, band.freq)];
                if kind.has_gain() { initial.push((gain_id, band.gain_db)); }
                if !band.on { initial.push((on_id, 1.0)); }
                e.update(cx, |ed, cx| press(ed, &Target { track: t.0.clone(), slot: t.1, key }, Control::Band(i), start, initial, defaults, ev, hb.id, window, cx));
                cx.stop_propagation();
            });
            let e = editor.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble { return; }
                e.update(cx, |ed, cx| {
                    let Some(Control::Band(i)) = ed.device_drag.as_ref().filter(|d| d.key == key).map(|d| d.control.clone()) else { return };
                    let has_gain = dsp::EQ_LAYOUT[i].0.has_gain();
                    let turned_on = ed.device_drag.as_ref().is_some_and(|d| d.preview.iter().any(|(id, _)| id.ends_with("_on")));
                    moved(ed, &Control::Band(i), key, ev, cx, move |start, (dx, dy), fine| {
                        let nx = drag_normalized(start[0], dx, w, fine);
                        let freq = dsp::FREQ_MIN * (dsp::FREQ_MAX / dsp::FREQ_MIN).powf(nx);
                        let mut out = vec![(dsp::eq_param_id(i, "freq"), freq.clamp(dsp::FREQ_MIN, dsp::FREQ_MAX))];
                        if has_gain {
                            let ny = drag_normalized(start[1], -dy, h, fine);
                            let y = (1.0 - ny) * h;
                            let db = (h / 2.0 - y) / (h / 2.0 - 10.0) * EQ_DB;
                            out.push((dsp::eq_param_id(i, "gain"), db.clamp(-dsp::GAIN_RANGE, dsp::GAIN_RANGE)));
                        }
                        if turned_on { out.push((dsp::eq_param_id(i, "on"), 1.0)); }
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

fn eq_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let selected = ed.device_band.min(dsp::EQ_BANDS - 1);
    let eq = eq_from(params);
    let target = || Target { track: track.clone(), slot, key };
    let band = eq.bands[selected];
    let bands = div().flex().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control).children((0..dsp::EQ_BANDS).map(|i| {
        let b = eq.bands[i];
        div().id(SharedString::from(format!("eq-band-{slot}-{i}"))).role(Role::Tab).aria_selected(i == selected)
            .aria_label(format!("频段 {}：{}", i + 1, band_label(b.kind)))
            .h(px(26.0)).px(px(8.0)).flex().items_center().gap(px(5.0)).rounded(px(6.0)).text_sm()
            .when(i == selected, |d| d.bg(th.checked)).hover(move |s| s.bg(if i == selected { th.checked } else { th.hover }))
            .child(div().size(px(9.0)).rounded(px(4.5)).bg(if b.on { band_color(i) } else { with_alpha(band_color(i), 0.35) }))
            .child(format!("{}", i + 1))
            .on_click(cx.listener(move |ed, _, _, cx| { ed.device_band = i; cx.notify(); }))
    }));
    let on_id = dsp::eq_param_id(selected, "on");
    let (toggle_track, toggle_id, on) = (track.clone(), on_id.clone(), band.on);
    let band_switch = div().id(SharedString::from(format!("eq-band-on-{slot}"))).role(Role::Switch).aria_label("频段开关").aria_selected(on)
        .flex().items_center().gap(px(8.0)).cursor_pointer()
        .child(div().text_sm().child(format!("{} · {}", selected + 1, band_label(band.kind))))
        .child(crate::view::switch(on, th))
        .on_click(cx.listener(move |ed, _, _, cx| ed.set_track_insert_params(toggle_track.clone(), slot, vec![(toggle_id.clone(), if on { 0.0 } else { 1.0 })], cx)));
    let mut knobs = div().flex().items_start().gap(px(6.0));
    for (field, label) in [("freq", "频率"), ("gain", "增益"), ("q", "Q")] {
        if field == "gain" && !band.kind.has_gain() { continue; }
        if let Some(d) = param(params, &dsp::eq_param_id(selected, field)) {
            knobs = knobs.child(knob(editor.clone(), target(), d.clone(), label, th, 46.0));
        }
    }
    if let Some(d) = param(params, "output") { knobs = knobs.child(div().w(px(10.0))).child(knob(editor.clone(), target(), d.clone(), "输出", th, 46.0)); }
    div().flex().flex_col().gap(px(10.0))
        .child(eq_graph(editor.clone(), target(), params.to_vec(), selected, th))
        .child(div().flex().items_center().gap(px(12.0)).child(bands).child(div().flex_1()).child(band_switch))
        .child(knobs)
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Compressor editor
// ---------------------------------------------------------------------------

const COMP_FLOOR: f64 = -60.0;

fn comp_graph(editor: Entity<Editor>, target: Target, params: Vec<ParamDesc>, th: Theme) -> impl IntoElement {
    let key = target.key;
    div().id(SharedString::from(format!("comp-graph-{}", target.slot))).role(Role::Image).aria_label("压缩器传输曲线")
        .size(px(196.0)).flex_none().rounded(px(10.0)).bg(th.view).overflow_hidden()
        .child(canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
            let o = bounds.origin;
            let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
            let c = comp_from(&params);
            let x_of = |db: f64| (db - COMP_FLOOR) / -COMP_FLOOR * w;
            let y_of = |db: f64| h - (db - COMP_FLOOR) / -COMP_FLOOR * h;
            let grid = with_alpha(th.text, 0.07);
            let mut texts = Texts::new();
            for db in [-48.0, -36.0, -24.0, -12.0] {
                window.paint_quad(fill(b(x_of(db), 0.0, 1.0, h, o), grid));
                window.paint_quad(fill(b(0.0, y_of(db), w, 1.0, o), grid));
                texts.push((format!("{db:.0}"), p(x_of(db) + 3.0, h - 15.0, o), 10.0, th.text_tertiary));
            }
            // Knee region and the unity reference.
            let (k0, k1) = (x_of(c.threshold_db - c.knee_db / 2.0), x_of(c.threshold_db + c.knee_db / 2.0));
            window.paint_quad(fill(b(k0, 0.0, (k1 - k0).max(1.0), h, o), with_alpha(th.accent, 0.08)));
            polyline(window, o, [Some((0.0, h)), Some((w, 0.0))], 1.0, with_alpha(th.text, 0.2));
            let pts = (0..=120).map(|i| { let x = COMP_FLOOR + i as f64 * -COMP_FLOOR / 120.0; Some((x_of(x), y_of((c.curve_db(x) + c.makeup_db).min(0.0)).clamp(0.0, h))) });
            polyline(window, o, pts, 2.0, th.accent);
            let (tx, ty) = (x_of(c.threshold_db), y_of(c.curve_db(c.threshold_db) + c.makeup_db).clamp(0.0, h));
            window.paint_quad(fill(b(tx, 0.0, 1.0, h, o), with_alpha(th.accent, 0.45)));
            window.paint_quad(quad(b(tx - 9.0, ty - 9.0, 18.0, 18.0, o), px(9.0), gpui::rgb(0xffffff), px(1.0), th.shade, gpui::BorderStyle::Solid));
            flush_texts(window, cx, bounds, texts);
            let (e, hb, t, ps) = (editor.clone(), hitbox.clone(), (target.track.clone(), target.slot), params.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !hb.is_hovered(window) { return; }
                let (x, y) = (f64::from(ev.position.x - o.x), f64::from(ev.position.y - o.y));
                // The threshold line is the handle along its whole height.
                if (x - tx).abs() > 12.0 && (x - tx).hypot(y - ty) > 14.0 { return; }
                let Some(d) = param(&ps, "threshold") else { return };
                let default = d.default_value.unwrap_or(-18.0);
                e.update(cx, |ed, cx| press(ed, &Target { track: t.0.clone(), slot: t.1, key }, Control::Threshold,
                    vec![to_norm(d, d.value)], vec![("threshold".into(), d.value)], vec![("threshold".into(), default)], ev, hb.id, window, cx));
                cx.stop_propagation();
            });
            let (e, d) = (editor.clone(), param(&params, "threshold").cloned());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble { return; }
                let Some(d) = d.clone() else { return };
                e.update(cx, |ed, cx| moved(ed, &Control::Threshold, key, ev, cx, move |start, (dx, _), fine| {
                    vec![("threshold".into(), from_norm(&d, drag_normalized(start[0], dx, w, fine)))]
                }));
            });
            let e = editor.clone();
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left { return; }
                e.update(cx, |ed, cx| released(ed, &Control::Threshold, key, window, cx));
            });
        }).size_full())
}

fn comp_editor(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, key: InsertKey, params: &[ParamDesc]) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: track.clone(), slot, key };
    let row = |ids: &[(&str, &str)]| {
        let mut r = div().flex().gap(px(4.0));
        for (id, label) in ids {
            if let Some(d) = param(params, id) { r = r.child(knob(editor.clone(), target(), d.clone(), label, th, 44.0)); }
        }
        r
    };
    div().flex().gap(px(14.0))
        .child(comp_graph(editor.clone(), target(), params.to_vec(), th))
        .child(div().flex().flex_col().gap(px(10.0))
            .child(row(&[("threshold", "阈值"), ("ratio", "压缩比"), ("knee", "拐点"), ("makeup", "补偿")]))
            .child(row(&[("attack", "启动"), ("release", "释放"), ("mix", "干湿")])))
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

fn device_card(ed: &Editor, editor: &Entity<Editor>, track: &TrackId, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let specs = ed.insert_specs(track);
    let spec = &specs[slot];
    let handle = ed.live_insert(track, slot);
    let builtin = spec.format == "builtin";
    let kind = if builtin && spec.id == dsp::EQ_ID { 1 } else if builtin && spec.id == dsp::COMPRESSOR_ID { 2 } else { 0 };
    let on = !spec.bypass;
    let arg = CommandArg::TrackSlot(track.clone(), slot as u32);
    let (bypass_arg, remove_arg, menu_track) = (arg.clone(), arg.clone(), track.clone());
    let header = div().flex().items_center().gap(px(10.0)).h(px(32.0))
        .child(div().id(SharedString::from(format!("device-power-{slot}"))).role(Role::Switch).aria_label("启用效果器").aria_selected(on).cursor_pointer()
            .child(crate::view::switch(on, th))
            .on_click(cx.listener(move |ed, _, w, cx| controls::dispatch(ed, CommandId::PluginBypass, bypass_arg.clone(), w, cx))))
        .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).text_color(if on { th.text } else { th.text_secondary }).child(spec.name.clone()))
        .child(div().id(SharedString::from(format!("device-menu-{slot}"))).role(Role::Button).aria_label("效果器菜单").size(px(28.0)).rounded(px(6.0))
            .flex().items_center().justify_center().hover(move |s| s.bg(th.hover))
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
    let body = match handle.as_ref().map(|h| h.key()).and_then(|key| device_params(ed, key).map(|p| (key, p))) {
        Some((key, params)) => match kind {
            1 => eq_editor(ed, editor, track, slot, key, &params, cx),
            2 => comp_editor(ed, editor, track, slot, key, &params),
            _ => generic_editor(ed, editor, track, slot, key, &params),
        },
        None => div().h(px(120.0)).flex().items_center().justify_center().text_sm().text_color(th.text_secondary)
            .child(if handle.is_some() { "正在读取参数…" } else { "插件未载入（标识与状态已保留）" }).into_any_element(),
    };
    let _ = remove_arg;
    div().id(SharedString::from(format!("device-{slot}"))).flex_none().flex().flex_col().gap(px(10.0)).p(px(14.0))
        .rounded(px(12.0)).bg(th.card).when(!on, |d| d.opacity(0.72))
        .when(kind == 1, |d| d.w(px(560.0))).when(kind == 2, |d| d.w(px(520.0)))
        .child(header).child(body)
        .into_any_element()
}

fn add_card(ed: &Editor, track: &TrackId, slot: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let (eq_track, comp_track, browse_track) = (track.clone(), track.clone(), track.clone());
    let pill = |id: &'static str, label: &'static str| div().id(id).role(Role::Button).aria_label(label).h(px(34.0)).px(px(14.0))
        .flex().items_center().justify_center().rounded(px(6.0)).bg(th.control).hover(move |s| s.bg(th.control_pressed)).text_sm().child(label);
    div().id("device-add").flex_none().w(px(200.0)).flex().flex_col().gap(px(8.0)).p(px(14.0)).rounded(px(12.0))
        .border_1().border_color(th.shade)
        .child(div().text_sm().font_weight(FontWeight::BOLD).child("添加效果器"))
        .child(pill("device-add-eq", "均衡器").on_click(cx.listener(move |ed, _, _, cx| ed.add_insert(&eq_track, "builtin", dsp::EQ_ID, cx))))
        .child(pill("device-add-comp", "压缩器").on_click(cx.listener(move |ed, _, _, cx| ed.add_insert(&comp_track, "builtin", dsp::COMPRESSOR_ID, cx))))
        .child(pill("device-add-browse", "浏览插件…").on_click(cx.listener(move |ed, _, _, cx| ed.open_plugin_browser(browse_track.clone(), slot, cx))))
        .into_any_element()
}

/// The device view for the selected track.
pub fn device_view(ed: &mut Editor, editor: &Entity<Editor>, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let track = ed.session.track.clone();
    let Some(name) = ed.project().track(&track).map(|t| t.name.clone()) else {
        return div().size_full().bg(th.view).into_any_element();
    };
    let count = ed.insert_specs(&track).len();
    for slot in 0..count { ed.fetch_track_insert_params(&track, slot, cx); }
    let ed = &*ed;
    let cards: Vec<gpui::AnyElement> = (0..count).map(|slot| device_card(ed, editor, &track, slot, cx)).collect();
    let empty = cards.is_empty();
    div().id("device-view").role(Role::Group).aria_label(format!("{name} 的效果器链"))
        .size_full().flex().flex_col().bg(th.view)
        .child(div().h(px(40.0)).flex_none().px(px(16.0)).flex().items_center().gap(px(8.0))
            .child(svg().path(icon("sliders-horizontal")).size(px(15.0)).text_color(th.text_secondary))
            .child(div().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(div().text_sm().text_color(th.text_secondary).child(if empty { "· 还没有效果器".to_string() } else { format!("· {count} 个效果器，信号从左到右") })))
        .child(div().id("device-chain").flex_1().min_h_0().overflow_x_scroll().overflow_y_scroll().flex().items_start().gap(px(10.0)).px(px(12.0)).pb(px(12.0))
            .children(cards)
            .child(add_card(ed, &track, count, cx)))
        .into_any_element()
}

#[allow(dead_code)]
fn _unused(_: Bounds<Pixels>, _: &App) {}

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
