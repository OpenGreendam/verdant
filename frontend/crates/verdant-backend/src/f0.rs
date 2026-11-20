//! Frontend F0 synthesis for routes that accept `payload.control_track`
//! (capability-gated Verdant ONNX/C0/C1/A in `ogd-worker/1`). The contour is: note base pitch + detune +
//! PIT curve (note-relative, linear between points, ends held) + vibrato +
//! explicit portamento, or a bounded automatic connection across adjacent notes.
//! Automatic connections use the same function for preview and synthesis.
//! Structural output uses equal lengths 2..=20000, strictly increasing
//! times in [0, 30] s, `uv ⇒ f0 = 0`, and voiced `f0 > 0`. Pitch is not
//! silently clamped: adapter preflight checks the actual route's F0 range,
//! while generic structural validation admits the maximum supported 5000 Hz.

use serde::Serialize;
use verdant_domain::{Note, TempoMap};

pub const MAX_CONTROL_F0_HZ: f64 = 5000.0;
pub const HOP_SECONDS: f64 = 0.005;
pub const VIBRATO_RAMP_SECONDS: f64 = 0.12;
pub const DEFAULT_PORTAMENTO_MS: u32 = 80;
pub const AUTO_GLIDE_NOTE_SHARE: f64 = 0.20;

/// Canonicalize only floating-point residue at a declared time boundary.
/// For example, subtracting two absolute times can turn exactly 30 score
/// seconds into 30.000000000000004. This is not a duration quantizer.
pub(crate) fn snap_seconds_bound(seconds: f64, bound: f64) -> f64 {
    if (seconds - bound).abs() <= 1e-12 { bound } else { seconds }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ControlTrack {
    pub frame_times_seconds: Vec<f64>,
    pub f0_hz: Vec<f64>,
    pub uv: Vec<bool>,
}

/// Linear interpolation over note-relative `(tick, value)` points; values
/// outside the first/last point hold the nearest end (SPEC §8).
pub fn interp(points: &[(i64, f64)], tick: f64) -> f64 {
    match points {
        [] => 0.0,
        [only] => only.1,
        _ => {
            if tick <= points[0].0 as f64 {
                return points[0].1;
            }
            if tick >= points[points.len() - 1].0 as f64 {
                return points[points.len() - 1].1;
            }
            let i = points.partition_point(|p| (p.0 as f64) <= tick);
            let (a, b) = (points[i - 1], points[i]);
            let f = (tick - a.0 as f64) / (b.0 - a.0) as f64;
            a.1 + (b.1 - a.1) * f
        }
    }
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A C2-continuous monotone connection with no pitch overshoot.
pub fn smootherstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

/// POR is the destination note's value at note-on. Later curve points belong
/// to later events; they do not pull an already-running transition around.
/// 64 retains the historical connection exactly; 0/127 move it earlier/later.
pub fn portamento_position(note: &Note) -> f64 {
    note.curves.expression("POR").first().map_or(64.,|p|f64::from(p.value))
}

fn positioned_window(a: &Note, b: &Note, tempo: &TempoMap, start: f64, end: f64) -> (f64, f64) {
    let value = portamento_position(b);
    if value == 64.0 { return (start, end); }
    let width = end-start;
    let normalized = if value < 64.0 { (value-64.0)/64.0 } else { (value-64.0)/63.0 };
    let lower = tempo.seconds_at(a.start_tick as f64);
    let upper = tempo.seconds_at(b.end_tick() as f64)-width;
    let shifted = (start + normalized*width/2.0).clamp(lower, upper.max(lower));
    (shifted, shifted+width)
}

/// Automatic transition window, in absolute score seconds. Manual PIT or
/// vibrato on either side owns that note's contour. A declared duration on
/// the destination owns this boundary: zero disables it, nonzero is explicit.
pub fn automatic_transition(a: &Note, b: &Note, tempo: &TempoMap) -> Option<(f64, f64)> {
    if a.is_muted() || b.is_muted() || a.end_tick() != b.start_tick || b.portamento_ms.is_some()
        || a.portamento_ms.is_some_and(|ms| ms > 0)
        || !a.curves.pitch_millicent_points.is_empty() || !b.curves.pitch_millicent_points.is_empty()
        || a.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0)
        || b.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0)
        || (base_cents(a) - base_cents(b)).abs() < 1e-9 {
        return None;
    }
    let boundary = tempo.seconds_at(b.start_tick as f64);
    let before = boundary - tempo.seconds_at(a.start_tick as f64);
    let after = tempo.seconds_at(b.end_tick() as f64) - boundary;
    let half = DEFAULT_PORTAMENTO_MS as f64 / 2000.0;
    let left = half.min(AUTO_GLIDE_NOTE_SHARE * before);
    let right = half.min(AUTO_GLIDE_NOTE_SHARE * after);
    (left > 0.0 && right > 0.0).then(|| positioned_window(a, b, tempo, boundary-left, boundary+right))
}

pub fn has_automatic_portamento(notes: &[&Note], tempo: &TempoMap) -> bool {
    notes.windows(2).any(|pair| automatic_transition(pair[0], pair[1], tempo).is_some())
}

fn automatic_cents(a: &Note, b: &Note, tempo: &TempoMap, t: f64) -> Option<f64> {
    let (start, end) = automatic_transition(a, b, tempo)?;
    let boundary = tempo.seconds_at(b.start_tick as f64);
    // A fully early/late window needs a held target/source up to/from the
    // score boundary, otherwise its pitch would jump back between the two.
    let (active_start, active_end) = if portamento_position(b) == 64.0 { (start, end) }
        else { (start.min(boundary), end.max(boundary)) };
    (t >= active_start && t <= active_end).then(|| {
        base_cents(a) + (base_cents(b) - base_cents(a)) * smootherstep(b.portamento_curve_position((t - start) / (end - start)))
    })
}

fn positioned_explicit_cents(a: &Note, b: &Note, tempo: &TempoMap, t: f64) -> Option<f64> {
    let ms = b.portamento_ms.filter(|ms| *ms > 0)?;
    if a.end_tick() < b.start_tick || portamento_position(b) == 64.0 { return None; }
    let boundary = tempo.seconds_at(b.start_tick as f64);
    let duration = (f64::from(ms)/1000.0).min(tempo.seconds_at(b.end_tick() as f64)-boundary);
    let (start, end) = positioned_window(a, b, tempo, boundary, boundary+duration);
    (t >= start.min(boundary) && t <= end.max(boundary)).then(||
        base_cents(a)+(base_cents(b)-base_cents(a))*smoothstep(b.portamento_curve_position((t-start)/(end-start))))
}

/// Absolute pitch of a note's base in cents (MIDI·100 + detune).
pub fn base_cents(n: &Note) -> f64 {
    n.midi_note as f64 * 100.0 + n.detune_millicents as f64 / 1000.0
}

/// Pitch in absolute cents for note `n` at absolute time `t` seconds, or
/// `None` outside the note. `prev` is the preceding note in time order.
pub fn note_cents_at(n: &Note, prev: Option<&Note>, tempo: &TempoMap, t: f64) -> Option<f64> {
    note_cents_at_with_next(n, prev, None, tempo, t)
}

/// Includes anticipation in the current note's tail, unlike the historical
/// predecessor-only helper. The piano roll and phrase control use this rule.
pub fn note_cents_at_with_next(n: &Note, prev: Option<&Note>, next: Option<&Note>, tempo: &TempoMap, t: f64) -> Option<f64> {
    let s = tempo.seconds_at(n.start_tick as f64);
    let e = tempo.seconds_at(n.end_tick() as f64);
    if n.is_muted() || t < s || t >= e {
        return None;
    }
    Some(note_cents_inside(n, prev, next, tempo, t, t - s, e - s))
}

/// [`note_cents_at_with_next`] with the note's score bounds precomputed.
/// Paint loops evaluating many samples of one note hoist the two tempo
/// lookups instead of repeating them per sample.
pub fn note_cents_in_span(n: &Note, prev: Option<&Note>, next: Option<&Note>, tempo: &TempoMap, t: f64, s: f64, e: f64) -> Option<f64> {
    (t >= s && t < e).then(|| note_cents_inside(n, prev, next, tempo, t, t - s, e - s))
}

// Membership is decided by the caller on its own clock. A phrase-relative
// boundary must not be converted back to absolute seconds for that decision:
// start + (end - start) can round just below end and delay a whole control cell.
fn note_cents_inside(n: &Note, prev: Option<&Note>, next: Option<&Note>, tempo: &TempoMap, t: f64, elapsed: f64, duration: f64) -> f64 {
    if let Some(cents) = prev.and_then(|p| automatic_cents(p, n, tempo, t))
        .or_else(|| next.and_then(|p| automatic_cents(n, p, tempo, t))) {
        return cents;
    }
    let mut cents = prev.and_then(|p| positioned_explicit_cents(p, n, tempo, t))
        .or_else(|| next.and_then(|p| positioned_explicit_cents(n, p, tempo, t)))
        .unwrap_or_else(|| base_cents(n));
    if !n.curves.pitch_millicent_points.is_empty() {
        cents += verdant_domain::editing::interpolate(&n.curves.pitch_millicent_points, tempo.tick_at(t)-n.start_tick as f64,
            0., n.curves.interpolation("PIT"), |p|p.tick, |p|p.millicents as f64/1000.);
    }
    if let (Some(ms), Some(p)) = (n.portamento_ms.filter(|m| *m > 0), prev) {
        let legato = p.end_tick() >= n.start_tick;
        let dur = (ms as f64 / 1000.0).min(duration);
        if legato && elapsed < dur && portamento_position(n) == 64.0 {
            let from = base_cents(p);
            cents += (from - base_cents(n)) * (1.0 - smoothstep(n.portamento_curve_position(elapsed / dur)));
        }
    }
    if let Some(v) = n.vibrato.as_ref().filter(|v| v.depth_cents > 0) {
        let onset = duration * v.delay_permille as f64 / 1000.0;
        if elapsed >= onset {
            let active=(duration-onset).max(0.);let elapsed=elapsed-onset;
            let fade_in=if v.unknown.contains_key("fade_in_permille"){active*v.fade_in_permille()as f64/1000.}else{VIBRATO_RAMP_SECONDS};
            let fade_out=active*v.fade_out_permille()as f64/1000.;
            let ramp=if fade_in>0.{(elapsed/fade_in).clamp(0.,1.)}else{1.};
            let tail=if fade_out>0.{((active-elapsed)/fade_out).clamp(0.,1.)}else{1.};
            let rate=v.rate_millihertz as f64/1000.;let phase=v.phase_millidegrees()as f64/360_000.;
            cents+=v.depth_cents as f64*ramp*tail*(std::f64::consts::TAU*(rate*elapsed+phase)).sin();
        }
    }
    cents
}

pub fn cents_to_hz(cents: f64) -> f64 {
    440.0 * ((cents / 100.0 - 69.0) / 12.0).exp2()
}

/// Control track for a phrase whose first sample is at `start_seconds`.
/// Times are right-continuous control boundaries; the exact final time is an
/// unvoiced sentinel. Include note/gap boundaries between regular 5 ms samples.
pub fn control_track(notes: &[&Note], tempo: &TempoMap, start_seconds: f64, seconds: f64) -> ControlTrack {
    control_track_impl(notes, tempo, start_seconds, seconds, &[], 30.0)
}

/// A materialized consonant lead. The note clock itself is never moved.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Preutterance {
    pub note_index: usize,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// Use all track notes for melodic context across artificial phrase splits.
/// Only a scheduled prefix in otherwise silent score time receives the
/// destination note's onset pitch; Native's explicit phone UV still gates it.
pub(crate) fn anchored_control_track(notes: &[&Note], tempo: &TempoMap, start_seconds: f64, seconds: f64, leads: &[Preutterance]) -> ControlTrack {
    control_track_impl(notes, tempo, start_seconds, seconds, leads, 30.0)
}

pub(crate) fn continuous_control_track(notes: &[&Note], tempo: &TempoMap, start_seconds: f64, seconds: f64, leads: &[Preutterance], max_seconds: f64) -> ControlTrack {
    control_track_impl(notes, tempo, start_seconds, seconds, leads, max_seconds)
}

fn control_track_impl(notes: &[&Note], tempo: &TempoMap, start_seconds: f64, seconds: f64, leads: &[Preutterance], max_seconds: f64) -> ControlTrack {
    let seconds = snap_seconds_bound(seconds, max_seconds);
    if !seconds.is_finite() || seconds <= 0.0 || seconds > max_seconds || !start_seconds.is_finite() {
        return ControlTrack { frame_times_seconds: Vec::new(), f0_hz: Vec::new(), uv: Vec::new() };
    }
    let mut times: Vec<f64> = (0..=(seconds / HOP_SECONDS).floor() as usize)
        .map(|i| (i as f64 * HOP_SECONDS * 1e6).round() / 1e6)
        .filter(|t| *t < seconds)
        .collect();
    let bounds: Vec<_> = notes
        .iter()
        .map(|n| (tempo.seconds_at(n.start_tick as f64) - start_seconds, tempo.seconds_at(n.end_tick() as f64) - start_seconds))
        .collect();
    for &(start, end) in &bounds {
        for rel in [start, end] {
            if rel > 0.0 && rel < seconds {
                times.push(rel);
            }
        }
    }
    for lead in leads {
        for rel in [lead.start_seconds - start_seconds, lead.end_seconds - start_seconds] {
            if rel > 0.0 && rel < seconds { times.push(rel); }
        }
    }
    times.push(seconds);
    times.sort_by(f64::total_cmp);
    times.dedup();
    let frames = times.len();
    let mut out =
        ControlTrack { frame_times_seconds: Vec::with_capacity(frames), f0_hz: Vec::with_capacity(frames), uv: Vec::with_capacity(frames) };
    let mut k = 0usize;
    for rel in times {
        let t = start_seconds + rel;
        while k + 1 < notes.len() && bounds[k].1 <= rel {
            k += 1;
        }
        let prev = if k > 0 { Some(notes[k - 1]) } else { None };
        let next = notes.get(k + 1).copied();
        let hz = if rel == seconds {
            None
        } else {
            notes
                .get(k)
                .and_then(|n| {
                    let (start, end) = bounds[k];
                    (!n.is_muted() && rel >= start && rel < end).then(|| note_cents_inside(n, prev, next, tempo, t, rel - start, end - start))
                })
                .or_else(|| {
                    let lead = leads.iter().find(|lead| {
                        rel >= lead.start_seconds - start_seconds && rel < lead.end_seconds - start_seconds
                    })?;
                    let n = notes[lead.note_index];
                    let prev = lead.note_index.checked_sub(1).map(|index| notes[index]);
                    let next = notes.get(lead.note_index+1).copied();
                    let onset = tempo.seconds_at(n.start_tick as f64);
                    let duration = tempo.seconds_at(n.end_tick() as f64) - onset;
                    Some(note_cents_inside(n, prev, next, tempo, onset, 0.0, duration))
                })
                .map(cents_to_hz)
                .filter(|h| h.is_finite() && *h > 0.0)
        };
        out.frame_times_seconds.push(rel);
        match hz {
            Some(h) => {
                out.f0_hz.push((h * 1000.0).round() / 1000.0);
                out.uv.push(false);
            }
            None => {
                out.f0_hz.push(0.0);
                out.uv.push(true);
            }
        }
    }
    out
}

/// Structural checks shared by the worker routes; adapter preflight checks the route's narrower range.
pub fn validate(track: &ControlTrack) -> Result<(), String> {
    validate_with_limits(track, 30.0, 20_000)
}

/// Extended limits are used only after continuous-run capability negotiation.
pub(crate) fn validate_with_limits(track: &ControlTrack, max_seconds: f64, max_points: usize) -> Result<(), String> {
    let n = track.frame_times_seconds.len();
    if !(2..=max_points).contains(&n) || track.f0_hz.len() != n || track.uv.len() != n {
        return Err(format!("control_track arrays must be equal length in 2..={max_points}"));
    }
    let mut prev = -1.0;
    for i in 0..n {
        let (t, f, uv) = (track.frame_times_seconds[i], track.f0_hz[i], track.uv[i]);
        if !(0.0..=max_seconds).contains(&t) || !(0.0..=MAX_CONTROL_F0_HZ).contains(&f) || !t.is_finite() || !f.is_finite() {
            return Err(format!("frame {i} out of range"));
        }
        if t <= prev || (uv && f != 0.0) || (!uv && f <= 0.0) {
            return Err(format!("frame {i}: times must increase; uv/F0 must agree"));
        }
        prev = t;
    }
    Ok(())
}
