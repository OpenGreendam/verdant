//! Pure note-relative brush geometry shared by every numeric curve lane.
//!
//! PIT freehand strokes use unsnapped project ticks. They retain pointer
//! vertices and sample intermediate values on the renderer's 5 ms clock,
//! converted through the project's tempo map and PPQ. A whole drag, including
//! every crossed note, becomes one batch. Smooth mode filters values only;
//! point selection and dragging retain their existing separate path.
use std::collections::BTreeMap;

use verdant_domain::{Command, CommandBatch, Note, PointId, TempoMap, Track};
use verdant_domain::editing::{Interpolation, interpolate as curve_value};
use verdant_domain::model::PitchPoint;

pub(crate) struct CurveSpan {
    pub from: i64,
    pub to: i64,
    pub fresh: BTreeMap<i64, f64>,
}

impl CurveSpan {
    pub fn keeps(&self, tick: i64) -> bool {
        !self.fresh.contains_key(&tick) && (tick < self.from || tick > self.to)
    }
}

fn interpolate(points: &[(f64, f64)], at: f64, default: f64) -> f64 {
    let index = points.partition_point(|p| p.0 <= at);
    match (index.checked_sub(1).map(|i| points[i]), points.get(index)) {
        (Some(a), Some(b)) if b.0 > a.0 => a.1 + (b.1-a.1)*(at-a.0)/(b.0-a.0),
        (Some(a), _) => a.1,
        (None, Some(b)) => b.1,
        _ => default,
    }
}

/// A note-level controller has one effective value regardless of the pointer's
/// horizontal location within the note. Keep the last drawn value in that
/// note; a line crossing it with no interior pointer sample uses its onset.
pub(crate) fn note_value(samples: &[(f64, f64)], note_start: i64, duration: i64,
    default: f64, erase: bool) -> Option<f64> {
    if samples.is_empty() || duration <= 0 || samples.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return None;
    }
    let start = note_start as f64;
    let end = (note_start+duration) as f64;
    if let Some(sample) = samples.iter().rev().find(|sample| sample.0 >= start && sample.0 < end) {
        return Some(if erase { default } else { sample.1 });
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    if sorted[0].0 < end && sorted.last()?.0 > start {
        Some(if erase { default } else { interpolate(&sorted, start, default) })
    } else {
        None
    }
}

pub(crate) fn prepare_stroke(original: &[(i64, f64)], samples: &[(f64, f64)],
    note_start: i64, duration: i64, default: f64, erase: bool) -> Option<CurveSpan> {
    if samples.is_empty() || duration <= 0 || samples.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let lo = sorted.first()?.0-note_start as f64;
    let hi = sorted.last()?.0-note_start as f64;
    if hi < 0. || lo > duration as f64 { return None; }
    let from = lo.round().clamp(0., duration as f64) as i64;
    let to = hi.round().clamp(0., duration as f64) as i64;
    let mut fresh = BTreeMap::new();
    let prior: Vec<_> = original.iter().map(|(t, v)| (*t as f64, *v)).collect();
    // Protect the existing interpolation outside the brush. An empty original
    // curve has its controller's real neutral value, not an assumed zero.
    for guard in [from.checked_sub(1).filter(|t| *t >= 0),
                  to.checked_add(1).filter(|t| *t <= duration)].into_iter().flatten() {
        if !original.iter().any(|p| p.0 == guard) {
            fresh.insert(guard, interpolate(&prior, guard as f64, default));
        }
    }
    let value = |tick: i64| if erase { default } else {
        interpolate(&sorted, (note_start+tick) as f64, default)
    };
    fresh.insert(from, value(from));
    if !erase {
        // Retain original brush vertices as well as the display's 15-tick
        // sampling grid; a short final interval must not lose its endpoint.
        for &(tick, _) in &sorted {
            let local = (tick-note_start as f64).round() as i64;
            if local > from && local < to { fresh.insert(local, value(local)); }
        }
        let mut tick = from.saturating_add(15);
        while tick < to {
            fresh.insert(tick, value(tick));
            let next = tick.saturating_add(15);
            if next == tick { break; }
            tick = next;
        }
    }
    fresh.insert(to, value(to));
    Some(CurveSpan { from, to, fresh })
}

/// Visit integer project ticks on the existing nonlinear/control-track clock.
/// Endpoints and tempo changes also survive when they fall between frames.
fn pitch_ticks(tempo: &TempoMap, from: i64, to: i64, mut visit: impl FnMut(i64)) {
    visit(from);
    let hop = verdant_backend::f0::HOP_SECONDS;
    let mut frame = (tempo.seconds_at(from as f64) / hop).floor() as i64 + 1;
    let end = tempo.seconds_at(to as f64);
    while (frame as f64) * hop < end {
        let tick = tempo.tick_at(frame as f64 * hop).round() as i64;
        if tick > from && tick < to { visit(tick); }
        frame += 1;
    }
    for point in tempo.points() {
        if point.tick > from && point.tick < to { visit(point.tick); }
    }
    visit(to);
}

/// Build the exact batch consumed by pitch-lane release, without mutating the
/// document during preview. Later passes over the same tick overwrite earlier
/// passes, so backtracking with the pencil does not leave duplicate vertices.
pub(crate) fn pitch_stroke_batch(track: &Track, tempo: &TempoMap, samples: &[(f64, f64)],
    smooth: bool, erase: bool) -> Option<CommandBatch> {
    pitch_batch(track, tempo, samples, smooth, erase, |_note, drawn| drawn)
}

/// Build the batch consumed by the piano-roll pitch pen. The drawn line is
/// absolute pitch: every note it crosses bends by the distance between the
/// drawn pitch and that note's own base, so one continuous drag reads as one
/// contour instead of one offset per note.
pub(crate) fn pitch_line_batch(track: &Track, tempo: &TempoMap, samples: &[(f64, f64)],
    smooth: bool) -> Option<CommandBatch> {
    pitch_batch(track, tempo, samples, smooth, false, |note, drawn| {
        (drawn - (note.midi_note as f64 + 0.5)) * 100.0
    })
}

/// Shared body of both pitch batches. `cents_of` turns a drawn sample value
/// into the relative cents the note stores.
fn pitch_batch(track: &Track, tempo: &TempoMap, samples: &[(f64, f64)], smooth: bool, erase: bool,
    cents_of: impl Fn(&Note, f64) -> f64) -> Option<CommandBatch> {
    if samples.is_empty() || samples.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) { return None; }
    let filtered;
    let samples = if smooth && samples.len() > 4 {
        let mut out = samples.to_vec();
        for i in 2..samples.len() - 2 {
            out[i].1 = (samples[i-2].1 + 4.*samples[i-1].1 + 6.*samples[i].1 + 4.*samples[i+1].1 + samples[i+2].1)/16.;
        }
        filtered = out;
        filtered.as_slice()
    } else { samples };
    let (lo, hi) = samples.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
    let mut commands = Vec::new();
    for note in track.notes_in_range(lo.floor() as i64, (hi.ceil() as i64).saturating_add(1)) {
        let mut drawn = BTreeMap::new();
        let mut segment = |a: (f64, f64), b: (f64, f64)| {
            let left = a.0.min(b.0).max(note.start_tick as f64);
            let right = a.0.max(b.0).min(note.end_tick() as f64);
            if left > right { return; }
            let from = left.round() as i64;
            let to = right.round() as i64;
            let value = |tick| {
                let at = if from == to { b.0.clamp(left, right) }
                    else if tick == from { left } else if tick == to { right } else { tick as f64 };
                let value = if erase { 0. } else if a.0 == b.0 { b.1 } else {
                    a.1 + (b.1-a.1)*((at-a.0)/(b.0-a.0)).clamp(0., 1.)
                };
                cents_of(note, value).clamp(-1200., 1200.)
            };
            for (&tick, prior) in drawn.range_mut((from-note.start_tick)..=(to-note.start_tick)) {
                *prior = value(tick+note.start_tick);
            }
            pitch_ticks(tempo, from, to, |tick| {
                drawn.insert(tick-note.start_tick, value(tick));
            });
        };
        if samples.len() == 1 { segment(samples[0], samples[0]); }
        else { for pair in samples.windows(2) { segment(pair[0], pair[1]); } }
        let (Some((&from, _)), Some((&to, _))) = (drawn.first_key_value(), drawn.last_key_value()) else { continue; };
        let mode = note.curves.interpolation("PIT");
        let value_at = |tick| curve_value(&note.curves.pitch_millicent_points, tick as f64, 0., mode,
            |p| p.tick, |p| p.millicents as f64 / 1000.);
        // Baking a nonlinear source to the same 5 ms convention allows the
        // newly drawn section to be linear without changing its neighbours to
        // a different interpolation law. Original vertices/metadata survive.
        let mut fresh = BTreeMap::new();
        if mode != Interpolation::Linear {
            pitch_ticks(tempo, note.start_tick, note.end_tick(), |tick| {
                let local = tick-note.start_tick;
                if local < from || local > to { fresh.insert(local, value_at(local)); }
            });
            if mode == Interpolation::Hold {
                for point in &note.curves.pitch_millicent_points {
                    let tick = point.tick-1;
                    if tick >= 0 && (tick < from || tick > to) { fresh.insert(tick, value_at(tick)); }
                }
            }
        }
        for tick in [from.checked_sub(1).filter(|t| *t >= 0), to.checked_add(1).filter(|t| *t <= note.duration_tick)].into_iter().flatten() {
            fresh.insert(tick, value_at(tick));
        }
        fresh.extend(drawn);
        let mut points: Vec<_> = note.curves.pitch_millicent_points.iter()
            .filter(|p| (p.tick < from || p.tick > to) && !fresh.contains_key(&p.tick)).cloned().collect();
        // Reuse an existing point's identity and extension data at shared ticks.
        for (tick, cents) in fresh {
            let mut point = note.curves.pitch_millicent_points.binary_search_by_key(&tick, |p| p.tick).ok()
                .map(|index| note.curves.pitch_millicent_points[index].clone())
                .unwrap_or_else(|| PitchPoint { id: PointId::generate(), tick, millicents: 0, unknown: Default::default() });
            point.millicents = (cents*1000.).round() as i64;
            points.push(point);
        }
        points.sort_by_key(|p| p.tick);
        if mode == Interpolation::Linear {
            commands.push(Command::SetPitchCurve { track: track.id.clone(), note_id: note.id.clone(), points });
        } else {
            let mut edited = note.clone();
            edited.curves.pitch_millicent_points = points;
            edited.curves.set_interpolation("PIT", Interpolation::Linear);
            commands.push(Command::ReplaceNotes { track: track.id.clone(), notes: vec![edited] });
        }
    }
    (!commands.is_empty()).then(|| CommandBatch { label: if erase { "擦除 PIT" } else if smooth { "平滑绘制 PIT" } else { "绘制 PIT" }.into(), commands })
}

#[cfg(test)]
#[path = "pitch_draw_tests.rs"]
mod pitch_draw_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasing_a_constant_nondefault_curve_resets_only_the_brush() {
        let edited = prepare_stroke(&[(0, 127.), (960, 127.)], &[(400., 10.), (500., 20.)],
                                    0, 960, 64., true).unwrap();
        assert_eq!(edited.fresh[&399], 127.);
        assert_eq!(edited.fresh[&400], 64.);
        assert_eq!(edited.fresh[&500], 64.);
        assert_eq!(edited.fresh[&501], 127.);
        assert!(edited.keeps(0) && edited.keeps(960));
        assert!(!edited.keeps(450));
    }

    #[test]
    fn short_and_reversed_strokes_keep_the_actual_endpoint_and_vertices() {
        let edited = prepare_stroke(&[], &[(126., 90.), (113., 0.), (110., 127.)],
                                    100, 120, 64., false).unwrap();
        assert_eq!(edited.fresh[&10], 127.);
        assert_eq!(edited.fresh[&13], 0.);
        assert_eq!(edited.fresh[&26], 90.);
        assert_eq!(edited.fresh[&9], 64.);
        assert_eq!(edited.fresh[&27], 64.);
    }

    #[test]
    fn fractional_clicks_reach_the_nearest_integer_tick_and_cross_note_clips() {
        let click = prepare_stroke(&[], &[(100.4, 12.)], 100, 60, 127., false).unwrap();
        assert_eq!(click.fresh[&0], 12.);
        let clipped = prepare_stroke(&[], &[(50., 0.), (200., 120.)], 100, 60, 0., false).unwrap();
        assert_eq!((clipped.from, clipped.to), (0, 60));
        assert_eq!(clipped.fresh[&0], 40.);
        assert_eq!(clipped.fresh[&60], 88.);
    }

    #[test]
    fn note_controller_uses_interior_clicks_and_erase_resets_the_effective_default() {
        assert_eq!(note_value(&[(900.5, 127.)], 0, 960, 64., false), Some(127.));
        assert_eq!(note_value(&[(900.5, 127.)], 0, 960, 64., true), Some(64.));
        assert_eq!(note_value(&[(960., 127.)], 0, 960, 64., false), None);
        assert_eq!(note_value(&[(960., 127.)], 960, 960, 64., false), Some(127.));
    }

    #[test]
    fn note_controller_applies_lines_to_every_crossed_note_and_respects_draw_order() {
        let line = [(100., 0.), (3000., 127.)];
        assert_eq!(note_value(&line, 0, 960, 64., false), Some(0.));
        assert!((note_value(&line, 960, 960, 64., false).unwrap()-127.*860./2900.).abs() < 1e-12);
        assert!((note_value(&line, 1920, 960, 64., false).unwrap()-127.*1820./2900.).abs() < 1e-12);
        assert_eq!(note_value(&line, 2880, 960, 64., false), Some(127.));
        assert_eq!(note_value(&line, 3840, 960, 64., false), None);
        assert_eq!(note_value(&[(800., 12.), (200., 100.)], 0, 960, 64., false), Some(100.));
        assert_eq!(note_value(&line, 960, 960, 64., true), Some(64.));
    }
}
