//! Post-render processing that is explicitly *not* synthesis (SPEC §8):
//! the DYN curve becomes a per-sample linear gain applied to verified PCM.

use verdant_domain::{Note, TempoMap};

use crate::wire::PhonemeTimingTrace;

/// Verified map from actual rendered time to the author's score clock.
/// Duplicate render anchors represent a gap rounded to zero frames.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeMap {
    pub render_seconds: Vec<f64>,
    pub authored_seconds: Vec<f64>,
}

impl TimeMap {
    pub fn new(render_seconds: Vec<f64>, authored_seconds: Vec<f64>) -> Result<Self, String> {
        let n = render_seconds.len();
        if n < 2 || n != authored_seconds.len() {
            return Err("time map needs equal arrays of at least two boundaries".into());
        }
        for (name, values) in [("render", &render_seconds), ("authored", &authored_seconds)] {
            if values[0] != 0.0
                || values.iter().any(|v| !v.is_finite() || *v < 0.0)
                || values.windows(2).any(|w| w[1] < w[0])
                || values[n - 1] <= 0.0
            {
                return Err(format!("{name} time-map boundaries must start at zero, be finite and nondecreasing, and end after zero"));
            }
        }
        Ok(Self { render_seconds, authored_seconds })
    }

    /// Right-continuous mapping on [0, render_end); no endpoint extension.
    pub fn authored_at(&self, rendered: f64) -> Option<f64> {
        if !rendered.is_finite() || rendered < 0.0 || rendered >= *self.render_seconds.last()? {
            return None;
        }
        let i = self.render_seconds.partition_point(|t| *t <= rendered) - 1;
        let a = self.render_seconds[i];
        let b = self.render_seconds[i + 1];
        let f = (rendered - a) / (b - a);
        Some(self.authored_seconds[i] + f * (self.authored_seconds[i + 1] - self.authored_seconds[i]))
    }
}

/// Apply DYN on the same render-to-score map as C1. Old artifacts without a
/// time map retain the original API behavior. This runs off the audio thread.
pub fn dyn_envelope_with_time_map(
    notes: &[&Note],
    tempo: &TempoMap,
    start_seconds: f64,
    sample_rate: u32,
    len: usize,
    time_map: Option<&TimeMap>,
) -> Option<Vec<f32>> {
    let Some(time_map) = time_map else { return dyn_envelope(notes, tempo, start_seconds, sample_rate, len) };
    let no_dyn = notes.iter().all(|n| n.curves.gain_millidecibel_points.is_empty());
    if no_dyn && (len == 0 || time_map.authored_at((len - 1) as f64 / sample_rate as f64).is_some()) {
        return None;
    }
    // The project domain rejects overlapping notes.
    let mut ranges: Vec<_> = notes
        .iter()
        .map(|n| {
            let points: Vec<_> = n.curves.gain_millidecibel_points.iter().map(|p| (p.tick, p.millidecibels as f64 / 1000.0)).collect();
            (
                tempo.seconds_at(n.start_tick as f64) - start_seconds,
                tempo.seconds_at(n.end_tick() as f64) - start_seconds,
                n.start_tick,
                points, n.curves.interpolation("DYN"),
            )
        })
        .collect();
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut env = vec![1.0f32; len];
    for (i, gain) in env.iter_mut().enumerate() {
        let Some(score_time) = time_map.authored_at(i as f64 / sample_rate as f64) else {
            // Resampling can add a tail sample at/past the acoustic endpoint.
            // It must not escape DYN or extend the valid authored interval.
            *gain = 0.0;
            continue;
        };
        let absolute = start_seconds + score_time;
        let k = ranges.partition_point(|r| r.0 <= score_time);
        if k == 0 {
            continue;
        }
        let (_, end, tick, points, mode) = &ranges[k - 1];
        if score_time < *end && !points.is_empty() {
            *gain = 10f64.powf(verdant_domain::editing::interpolate(points,tempo.tick_at(absolute)-*tick as f64,0.,*mode,|p|p.0,|p|p.1) / 20.0) as f32;
        }
    }
    Some(env)
}

/// DYN for a materialized phrase. A scheduled phone belongs to the note in
/// the timing trace, including its anticipatory consonants. Prefix samples
/// hold that note's first DYN value; from the score onset onward the existing
/// score clock samples its curve. Worker phrase boundaries cannot change this
/// ownership. Missing timing retains the legacy path without any arithmetic
/// changes. Invalid explicit timing must not silently fall back to that path.
pub fn dyn_envelope_for_phrase(
    notes: &[&Note],
    tempo: &TempoMap,
    start_seconds: f64,
    sample_rate: u32,
    len: usize,
    time_map: Option<&TimeMap>,
    timing: Option<&PhonemeTimingTrace>,
) -> Result<Option<Vec<f32>>, String> {
    let Some(timing) = timing else {
        return Ok(dyn_envelope_with_time_map(notes, tempo, start_seconds, sample_rate, len, time_map));
    };
    const EPSILON: f64 = 1e-9;
    if timing.method != "vowel-anchor-v1" || timing.notes.is_empty()
        || !start_seconds.is_finite() || start_seconds < 0.0 || sample_rate == 0 {
        return Err("invalid explicit phoneme timing for DYN".into());
    }
    let map = time_map.ok_or("phoneme-timed DYN requires the verified render-to-score time map")?;
    let mut ids = std::collections::BTreeSet::new();
    let mut ranges = Vec::with_capacity(timing.notes.len());
    for anchor in &timing.notes {
        let note = notes.iter().copied().find(|note| note.id.as_str() == anchor.note_id)
            .ok_or_else(|| format!("phoneme-timed DYN note {} is missing", anchor.note_id))?;
        let onset = anchor.note_onset_seconds;
        let end = anchor.note_end_seconds;
        let prefix = anchor.prefix_seconds;
        let trim = anchor.tail_trim_seconds;
        let from = onset - prefix;
        let to = end - trim;
        if !ids.insert(anchor.note_id.as_str())
            || [onset, end, prefix, trim, from, to].iter().any(|v| !v.is_finite())
            || onset < 0.0 || end <= onset || prefix < 0.0 || trim < 0.0
            || from < -EPSILON || to <= from || to <= onset
            || (tempo.seconds_at(note.start_tick as f64) - start_seconds - onset).abs() > EPSILON
            || (tempo.seconds_at(note.end_tick() as f64) - start_seconds - end).abs() > EPSILON {
            return Err(format!("invalid scheduled DYN interval for note {}", anchor.note_id));
        }
        let points: Vec<_> = note.curves.gain_millidecibel_points.iter()
            .map(|point| (point.tick, point.millidecibels as f64 / 1000.0)).collect();
        ranges.push((from.max(0.0), to, onset, note.start_tick, points, note.curves.interpolation("DYN")));
    }
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0 + EPSILON)
        || ranges.last().is_some_and(|range| range.1 > *map.authored_seconds.last().unwrap() + EPSILON) {
        return Err("overlapping or out-of-map phoneme-timed DYN intervals".into());
    }
    let no_dyn = ranges.iter().all(|range| range.4.is_empty());
    if no_dyn && (len == 0 || map.authored_at((len - 1) as f64 / sample_rate as f64).is_some()) {
        return Ok(None);
    }
    let mut envelope = vec![1.0f32; len];
    for (i, gain) in envelope.iter_mut().enumerate() {
        let Some(score_time) = map.authored_at(i as f64 / sample_rate as f64) else {
            *gain = 0.0; // Never extend the verified endpoint after resampling.
            continue;
        };
        // Only floating residue within 1 ns of an authored boundary is
        // canonicalized. This is far below a sample, not a timing policy.
        let k = ranges.partition_point(|range| range.0 <= score_time + EPSILON);
        if k == 0 { continue; }
        let (_, end, onset, tick, points, mode) = &ranges[k - 1];
        if score_time >= *end - EPSILON || points.is_empty() { continue; }
        let db = if score_time < *onset {
            points[0].1
        } else {
            verdant_domain::editing::interpolate(points,tempo.tick_at(start_seconds+score_time)-*tick as f64,0.,*mode,|p|p.0,|p|p.1)
        };
        *gain = 10f64.powf(db / 20.0) as f32;
    }
    Ok(Some(envelope))
}

/// Gain envelope for a phrase at `sample_rate`, or `None` when no note in the
/// phrase carries a DYN curve. Outside notes the gain is unity.
pub fn dyn_envelope(notes: &[&Note], tempo: &TempoMap, start_seconds: f64, sample_rate: u32, len: usize) -> Option<Vec<f32>> {
    if notes.iter().all(|n| n.curves.gain_millidecibel_points.is_empty()) {
        return None;
    }
    let mut env = vec![1.0f32; len];
    for n in notes.iter().filter(|n| !n.curves.gain_millidecibel_points.is_empty()) {
        let pts: Vec<(i64, f64)> = n.curves.gain_millidecibel_points.iter().map(|p| (p.tick, p.millidecibels as f64 / 1000.0)).collect();
        let s = tempo.seconds_at(n.start_tick as f64) - start_seconds;
        let e = tempo.seconds_at(n.end_tick() as f64) - start_seconds;
        let a = ((s * sample_rate as f64).round().max(0.0) as usize).min(len);
        let b = ((e * sample_rate as f64).round().max(0.0) as usize).min(len);
        for (i, g) in env.iter_mut().enumerate().take(b).skip(a) {
            let t = start_seconds + i as f64 / sample_rate as f64;
            let db = verdant_domain::editing::interpolate(&pts,tempo.tick_at(t)-n.start_tick as f64,0.,n.curves.interpolation("DYN"),|p|p.0,|p|p.1);
            *g = 10f64.powf(db / 20.0) as f32;
        }
    }
    Some(env)
}
