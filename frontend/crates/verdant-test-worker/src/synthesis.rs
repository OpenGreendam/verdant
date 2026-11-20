//! Synthetic formant tones only: this is not a voice or production synthesis.

use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use crate::RenderError;
use crate::random::Random;

pub(crate) struct Segment {
    pub(crate) duration: f64,
    pub(crate) gap: f64,
    pub(crate) f0: f64,
    formants: [f64; 3],
}

pub(crate) fn segments(values: &[Value]) -> Result<Vec<Segment>, RenderError> {
    values.iter().map(|value| {
        let explicit_f0 = value.get("f0_hz").and_then(Value::as_f64).unwrap_or(0.0);
        let f0 = if explicit_f0 != 0.0 {
            explicit_f0
        } else {
            let midi = match value.get("midi_note") {
                None => 69.0,
                Some(midi) => midi.as_f64().or_else(|| midi.as_bool().map(|value| u8::from(value) as f64)).ok_or("midi_note must be a number")?,
            };
            440.0 * 2.0f64.powf((midi - 69.0) / 12.0)
        };
        if !f0.is_finite() || f0 <= 0.0 {
            return Err(RenderError::Failed("midi_note produces an invalid F0".into()));
        }
        let formants = match value.get("vowel") {
            Some(Value::Array(_) | Value::Object(_)) => return Err(RenderError::Failed("vowel must be a scalar".into())),
            vowel => match vowel.and_then(Value::as_str) {
                Some("e") => [520.0, 1700.0, 2500.0],
                Some("i") => [300.0, 2300.0, 3000.0],
                Some("o") => [520.0, 900.0, 2400.0],
                Some("u") => [330.0, 800.0, 2300.0],
                _ => [800.0, 1200.0, 2600.0],
            },
        };
        Ok(Segment {
            duration: value["duration_seconds"].as_f64().expect("admitted duration"),
            gap: value.get("gap_after_seconds").and_then(Value::as_f64).unwrap_or(0.0),
            f0,
            formants,
        })
    }).collect()
}

fn resonance(frequency: f64, center: f64) -> f64 {
    let bandwidth = 80.0 + 0.06 * center;
    1.0 / ((1.0 - (frequency / center).powi(2)).powi(2) + (frequency * bandwidth / center.powi(2)).powi(2)).sqrt() * (bandwidth / center)
}

fn weight(frequency: f64, formants: &[f64; 3], harmonic: usize) -> f64 {
    formants.iter().map(|center| resonance(frequency, *center)).sum::<f64>() / (harmonic as f64).powf(0.7)
}

pub(crate) fn synth(segments: &[Segment], rate: u32, cancelled: &AtomicBool, seed: u64) -> Result<Vec<f64>, RenderError> {
    let rate = rate as f64;
    let frames = segments.iter().map(|segment| (segment.duration * rate).round_ties_even() as usize + (segment.gap * rate).round_ties_even() as usize).sum();
    let mut out = Vec::with_capacity(frames);
    let mut phase = 0.0;
    let mut previous_f0 = None;
    let mut random = Random::new(seed as u32);
    let mut weights = Vec::new();
    for segment in segments {
        let start_f0 = previous_f0.unwrap_or(segment.f0);
        let nyquist = (rate / 2.0 - 500.0).min(12_000.0);
        let harmonics = ((nyquist / segment.f0.max(1.0)) as usize).max(1);
        weights.clear();
        weights.extend((1..=harmonics).map(|harmonic| weight(harmonic as f64 * segment.f0, &segment.formants, harmonic)));
        let norm = 0.6 / (weights.iter().sum::<f64>().sqrt() * 1.6).max(1e-9);
        for index in 0..(segment.duration * rate).round_ties_even() as usize {
            if index % 4000 == 0 && cancelled.load(Ordering::Relaxed) {
                return Err(RenderError::Cancelled);
            }
            let time = index as f64 / rate;
            let glide = (time / 0.04).min(1.0);
            let vibrato = if time < 0.18 { 0.0 } else { 0.012 * (2.0 * PI * 5.5 * time).sin() * ((time - 0.18) / 0.2).min(1.0) };
            let frequency = (start_f0 + (segment.f0 - start_f0) * glide) * (1.0 + vibrato);
            phase += 2.0 * PI * frequency / rate;
            let sample = weights.iter().enumerate().map(|(index, weight)| weight * ((index + 1) as f64 * phase).sin()).sum::<f64>();
            let envelope = if segment.duration > 0.05 { 1.0f64.min(time / 0.015).min((segment.duration - time) / 0.03) } else { 1.0 };
            out.push((sample * norm * envelope.max(0.0) + random.uniform_signed() * 0.002).clamp(-1.0, 1.0));
        }
        let gap = (segment.gap * rate).round_ties_even() as usize;
        out.resize(out.len() + gap, 0.0);
        previous_f0 = if gap == 0 { Some(segment.f0) } else { None };
    }
    Ok(out)
}

pub(crate) fn synth_tracked(segments: &[Segment], track: &Value, rate: u32, cancelled: &AtomicBool) -> Result<Vec<f64>, RenderError> {
    let mut pitch = Pitch::new(track);
    let mut start = 0.0;
    let bounds: Vec<_> = segments.iter().map(|segment| {
        let bound = (start, start + segment.duration, &segment.formants);
        start += segment.duration + segment.gap;
        bound
    }).collect();
    let rate = rate as f64;
    let mut out = vec![0.0; (start * rate).round_ties_even() as usize];
    let mut phase = 0.0;
    let mut segment = 0;
    for (index, sample) in out.iter_mut().enumerate() {
        if index % 4000 == 0 && cancelled.load(Ordering::Relaxed) {
            return Err(RenderError::Cancelled);
        }
        let time = index as f64 / rate;
        let hz = pitch.at(time);
        while segment + 1 < bounds.len() && time >= bounds[segment].1 {
            segment += 1;
        }
        let (start, end, formants) = bounds[segment];
        if hz <= 0.0 || !(start <= time && time < end) {
            continue;
        }
        phase += 2.0 * PI * hz / rate;
        let harmonics = ((rate / 2.0 - 500.0).min(12_000.0) / hz) as usize;
        let mut acc = 0.0;
        for harmonic in 1..=harmonics {
            acc += weight(harmonic as f64 * hz, formants, harmonic) * (harmonic as f64 * phase).sin();
        }
        let envelope = 1.0f64.min((time - start) / 0.015).min((end - time) / 0.03);
        *sample = (acc * 0.05 * envelope.max(0.0)).clamp(-1.0, 1.0);
    }
    Ok(out)
}

/// Monotone sampling of the submitted control at either audio or pitch-frame times.
pub(crate) struct Pitch<'a> {
    times: &'a [Value],
    f0: &'a [Value],
    uv: &'a [Value],
    frame: usize,
}

impl<'a> Pitch<'a> {
    pub(crate) fn new(track: &'a Value) -> Self {
        Self { times: track["frame_times_seconds"].as_array().expect("admitted times"),
            f0: track["f0_hz"].as_array().expect("admitted F0"),
            uv: track["uv"].as_array().expect("admitted UV"), frame: 0 }
    }

    pub(crate) fn at(&mut self, time: f64) -> f64 {
        let timestamp = |index: usize| self.times[index].as_f64().expect("admitted time");
        while self.frame + 1 < self.times.len() && timestamp(self.frame + 1) <= time {
            self.frame += 1;
        }
        let frame = self.frame;
        if self.uv[frame].as_bool().expect("admitted UV") { return 0.0; }
        let current = self.f0[frame].as_f64().expect("admitted F0");
        let next = frame + 1;
        if next == self.times.len() || self.uv[next].as_bool().expect("admitted UV") { return current; }
        let alpha = ((time - timestamp(frame)) / (timestamp(next) - timestamp(frame))).clamp(0.0, 1.0);
        current + (self.f0[next].as_f64().expect("admitted F0") - current) * alpha
    }
}
