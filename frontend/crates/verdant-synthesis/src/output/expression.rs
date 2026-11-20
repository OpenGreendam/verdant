use std::sync::Arc;
use anyhow::{ensure, Context, Result};
use rustfft::{Fft, FftPlanner};
use serde_json::{json, Value};
use crate::core::AcousticControlTrack;
use crate::dsp::rng::NumpyPcg64;
use crate::signal::{self, BorderMode, Complex64};
use super::{cell_at, exact_fields, finite_range, peak, safe_cell_at,
    seconds_to_samples, SampleDtype, BLOCK, EXPRESSION_METHOD};

const KEYS: [&str; 6] = ["BRE", "BRI", "CLE", "GEN", "OPE", "GWL"];
const DEFAULTS: [f64; 6] = [0.0, 64.0, 0.0, 64.0, 127.0, 0.0];
const BRE: usize = 0;
const BRI: usize = 1;
const CLE: usize = 2;
const GEN: usize = 3;
const OPE: usize = 4;
const GWL: usize = 5;

struct Segment {
    start: f64,
    end: f64,
    onset: f64,
    curves: [Option<Vec<[f64; 2]>>; 6],
}
struct Track { segments: Vec<Segment> }

pub(super) fn validate(value: &Value) -> Result<()> { Track::parse(value).map(|_| ()) }

impl Track {
    fn parse(value: &Value) -> Result<Self> {
        let object = exact_fields(value, &["method", "segments"], "expression")?;
        ensure!(object["method"].as_str() == Some(EXPRESSION_METHOD),
            "expression requires vocaloid-controls-v1 method");
        let segments = object["segments"].as_array().context("expression segments must be an array")?;
        ensure!((1..=2048).contains(&segments.len()),
            "expression needs 1..2048 explicitly owned note intervals");
        let mut prepared = Vec::with_capacity(segments.len());
        let mut last_end = 0.0;
        for value in segments {
            let object = exact_fields(value,
                &["start_seconds", "end_seconds", "note_onset_seconds", "curves"],
                "expression segment")?;
            let start = finite_range(&object["start_seconds"], 0.0, 120.0 + 1e-10, "expression start")?;
            let end = finite_range(&object["end_seconds"], 0.0, 120.0 + 1e-10, "expression end")?;
            let onset = finite_range(&object["note_onset_seconds"], 0.0, 120.0 + 1e-10, "expression onset")?;
            ensure!(start >= last_end - 1e-10 && start <= onset && onset < end,
                "expression intervals must be ordered, nonoverlapping and own their onset");
            last_end = end;
            let curves = object["curves"].as_object().context("expression curves must be an object")?;
            ensure!(!curves.is_empty() && curves.keys().all(|key| KEYS.contains(&key.as_str())),
                "expression contains an unsupported controller");
            let mut compiled: [Option<Vec<[f64; 2]>>; 6] = std::array::from_fn(|_| None);
            for (key, points) in curves {
                let which = KEYS.iter().position(|&name| name == key).context("unsupported controller")?;
                let points = points.as_array().context("controller points must be an array")?;
                ensure!((1..=32768).contains(&points.len()), "controller needs 1..32768 compiled points");
                let mut previous = -1.0;
                let mut curve = Vec::with_capacity(points.len());
                for point in points {
                    let point = point.as_array().context("controller point must be [time,value]")?;
                    ensure!(point.len() == 2, "controller point must be [time,value]");
                    let time = finite_range(&point[0], 0.0, 120.0, "controller time")?;
                    let value = finite_range(&point[1], 0.0, 127.0, "controller value")?;
                    ensure!(time > previous, "controller point times must strictly increase");
                    previous = time;
                    curve.push([time, value]);
                }
                compiled[which] = Some(curve);
            }
            prepared.push(Segment { start, end, onset, curves: compiled });
        }
        Ok(Self { segments: prepared })
    }

    fn values_at(&self, times: &[f64], mapping: Option<&TimeMap<'_>>) -> Vec<[f64; 6]> {
        let mut values = vec![DEFAULTS; times.len()];
        let score: Vec<f64> = times.iter().map(|&t| match mapping {
            Some(map) => map.score(t), None => t,
        }).collect();
        // Queries are ordered, and unsupported map endpoints form a suffix.
        // Do interval searches once per segment rather than scanning every
        // sample against every note. Tiny admitted overlaps retain last-wins.
        let supported = score.partition_point(|v| v.is_finite());
        let score = &score[..supported];
        for segment in &self.segments {
            let first = score.partition_point(|&t| t < segment.start - 1e-12);
            let last = score.partition_point(|&t| t < segment.end - 1e-12);
            if last <= first { continue; }
            for (key, curve) in segment.curves.iter().enumerate() {
                if let Some(curve) = curve {
                    for index in first..last {
                        values[index][key] = interpolate_curve(curve, score[index] - segment.onset);
                    }
                }
            }
        }
        values
    }
}

struct TimeMap<'a> { rendered: &'a [f64], authored: &'a [f64], collapsed: bool }
impl<'a> TimeMap<'a> {
    fn new((rendered, authored): (&'a [f64], &'a [f64])) -> Result<Self> {
        ensure!(rendered.len() >= 2 && rendered.len() == authored.len()
            && rendered.iter().chain(authored).all(|v| v.is_finite())
            && rendered[0] == 0.0 && authored[0] == 0.0
            && rendered[rendered.len() - 1] > 0.0 && authored[authored.len() - 1] > 0.0
            && rendered.windows(2).all(|v| v[1] >= v[0])
            && authored.windows(2).all(|v| v[1] >= v[0]),
            "expression time map must start at zero, be finite and nondecreasing, and end after zero");
        Ok(Self { rendered, authored,
            collapsed: rendered.windows(2).any(|v| v[0] == v[1]) })
    }
    fn score(&self, time: f64) -> f64 {
        if time < self.rendered[0] || time >= self.rendered[self.rendered.len() - 1] {
            return f64::NAN;
        }
        let index = self.rendered.partition_point(|&t| t <= time) - 1;
        if self.collapsed {
            let fraction = (time - self.rendered[index]) /
                (self.rendered[index + 1] - self.rendered[index]);
            self.authored[index] + fraction * (self.authored[index + 1] - self.authored[index])
        } else {
            if time == self.rendered[index] { return self.authored[index]; }
            let slope = (self.authored[index + 1] - self.authored[index]) /
                (self.rendered[index + 1] - self.rendered[index]);
            slope * (time - self.rendered[index]) + self.authored[index]
        }
    }
}

fn interpolate_curve(points: &[[f64; 2]], time: f64) -> f64 {
    if time <= points[0][0] { return points[0][1]; }
    if time >= points[points.len() - 1][0] { return points[points.len() - 1][1]; }
    let index = points.partition_point(|point| point[0] <= time) - 1;
    if time == points[index][0] { return points[index][1]; }
    let slope = (points[index + 1][1] - points[index][1]) /
        (points[index + 1][0] - points[index][0]);
    slope * (time - points[index][0]) + points[index][1]
}
fn lerp(time: f64, left_time: f64, right_time: f64, left: f64, right: f64) -> f64 {
    if time <= left_time || left_time == right_time { left }
    else if time >= right_time { right }
    else { (right - left) / (right_time - left_time) * (time - left_time) + left }
}

// The frozen frame_input copies every source into np.zeros(dtype=float64).
// Active STFT, envelopes, breath mixing, IRFFT and OLA therefore stay f64 even
// for a C1 float32 source. Plans and scratch are reused across the whole run.
struct FftPair {
    forward: Arc<dyn Fft<f64>>, inverse: Arc<dyn Fft<f64>>,
    data: Vec<Complex64>, scratch: Vec<Complex64>,
}
impl FftPair {
    fn new(size: usize) -> Self {
        let mut planner = FftPlanner::<f64>::new();
        let forward = planner.plan_fft_forward(size);
        let inverse = planner.plan_fft_inverse(size);
        let zero = Complex64::new(0.0, 0.0);
        let scratch = vec![zero; forward.get_inplace_scratch_len().max(inverse.get_inplace_scratch_len())];
        Self { forward, inverse, data: vec![zero; size], scratch }
    }
    fn analyze(&mut self, wave: &[f64], origin: i64, window: &[f64], sum: f64, bins: &mut [Complex64]) {
        for (offset, value) in self.data.iter_mut().enumerate() {
            let index = origin + offset as i64;
            let sample = if index >= 0 && (index as usize) < wave.len() { wave[index as usize] } else { 0.0 };
            value.re = sample * window[offset];
            value.im = 0.0;
        }
        self.forward.process_with_scratch(&mut self.data, &mut self.scratch);
        let scale = 1.0 / sum;
        for (out, value) in bins.iter_mut().zip(&self.data) { *out = *value * scale; }
        // Real transforms have exactly real DC and Nyquist cells.
        bins[0].im = 0.0;
        if let Some(last) = bins.last_mut() { last.im = 0.0; }
    }
    fn overlap_add(&mut self, bins: &[Complex64], window: &[f64], sum: f64, output: &mut [f64]) {
        let size = self.data.len();
        self.data[..bins.len()].copy_from_slice(bins);
        self.data[0].im = 0.0;
        self.data[size / 2].im = 0.0;
        for index in 1..size / 2 { self.data[size - index] = self.data[index].conj(); }
        self.inverse.process_with_scratch(&mut self.data, &mut self.scratch);
        let inverse_scale = 1.0 / size as f64;
        for index in 0..size {
            let value = (self.data[index].re * inverse_scale) * sum;
            output[index] += value * window[index];
        }
    }
}

pub(super) struct ExpressionResult {
    pub samples: Vec<f64>, pub dtype: SampleDtype, pub report: Value,
    pub active: Vec<bool>, pub blend: Option<Vec<f64>>,
}

pub(super) fn apply(
    source: &[f64], dtype: SampleDtype, sample_rate: u32, controls: &AcousticControlTrack,
    expression: &Value, seed: u64, time_map: Option<(&[f64], &[f64])>,
) -> Result<ExpressionResult> {
    ensure!((8000..=192000).contains(&sample_rate), "expression sample rate must be 8000..192000");
    let track = Track::parse(expression)?;
    ensure!(seed <= u32::MAX as u64, "expression seed must be an unsigned 32-bit integer");
    let mapping = time_map.map(TimeMap::new).transpose()?;
    let count = source.len();
    let rate = sample_rate as f64;
    let mut voiced = vec![false; count];
    let mut active = vec![false; count];
    let mut ranges = DEFAULTS.map(|value| [value, value]);
    let mut growl_enabled = false;
    for begin in (0..count).step_by(BLOCK) {
        let end = (begin + BLOCK).min(count);
        let times: Vec<f64> = (begin..end).map(|i| i as f64 / rate).collect();
        let values = track.values_at(&times, mapping.as_ref());
        for (offset, (time, value)) in times.iter().zip(&values).enumerate() {
            let local_voiced = cell_at(controls, *time).is_some_and(|cell|
                controls.voiced[cell] && controls.continuity_identity[cell] >= 0);
            voiced[begin + offset] = local_voiced;
            active[begin + offset] = local_voiced && value.iter().zip(DEFAULTS).any(|(&v, default)| v != default);
            for key in 0..6 {
                ranges[key][0] = ranges[key][0].min(value[key]);
                ranges[key][1] = ranges[key][1].max(value[key]);
            }
            growl_enabled |= local_voiced && value[GWL] > 0.0;
        }
    }
    let enabled = active.iter().any(|&v| v);
    let defaults: serde_json::Map<String, Value> = KEYS.iter().zip(DEFAULTS)
        .map(|(&key, value)| (key.into(), json!(value))).collect();
    let range_report: serde_json::Map<String, Value> = KEYS.iter().zip(ranges)
        .map(|(&key, value)| (key.into(), json!(value))).collect();
    let mut report = json!({"enabled": enabled, "method": EXPRESSION_METHOD,
        "implementation": "Verdant functional reconstruction, not Yamaha DSP equivalence",
        "defaults": defaults, "ranges": range_report,
        "scope": "explicitly voiced cells only; unvoiced samples exact bypass",
        "sample_rate": sample_rate, "samples": count,
        "growl_method": "F0-coupled 1/2 and 1/3 subharmonic sidebands; no recorded growl reference"});
    if !enabled {
        report["exact_bypass"] = json!(true);
        return Ok(ExpressionResult { samples: source.to_vec(), dtype, report, active, blend: None });
    }
    let mut blend = vec![0.0; count];
    let mut index = 0;
    while index < count {
        if !active[index] { index += 1; continue; }
        let begin = index;
        while index < count && active[index] { index += 1; }
        blend[begin..index].fill(1.0);
        let width = seconds_to_samples(0.005, rate).min((index - begin) / 2);
        if width > 1 {
            for step in 0..width {
                let ramp = 0.5 - 0.5 * (step as f64 * (std::f64::consts::PI / (width - 1) as f64)).cos();
                blend[begin + step] = ramp;
                blend[index - 1 - step] = ramp;
            }
        }
    }
    let size = if sample_rate >= 32000 { 2048 } else { 1024 };
    let hop = size / 4;
    let padded_count = count.max(size);
    let frame_count = 1 + padded_count.div_ceil(hop);
    let output_count = size.checked_add((frame_count - 1).checked_mul(hop)
        .context("expression output size overflow")?).context("expression output size overflow")?;
    let centers: Vec<f64> = (0..frame_count)
        .map(|frame| ((size / 2 + frame * hop) as f64 / rate) - (size as f64 / 2.0) / rate).collect();
    let mut left = Vec::with_capacity(frame_count);
    let mut sparse = Vec::with_capacity(frame_count * 2);
    for &center in &centers {
        let mut index = (center * rate).floor().clamp(0.0, (count - 1) as f64) as usize;
        if index > 0 && center < index as f64 / rate { index -= 1; }
        left.push(index);
        sparse.push(index);
        sparse.push((index + 1).min(count - 1));
    }
    sparse.sort_unstable();
    sparse.dedup();
    let sparse_times: Vec<f64> = sparse.iter().map(|&i| i as f64 / rate).collect();
    let sparse_values = track.values_at(&sparse_times, mapping.as_ref());
    let mut frames: Vec<[f64; 6]> = Vec::with_capacity(frame_count);
    let mut frame_voiced = Vec::with_capacity(frame_count);
    let mut frame_f0 = Vec::with_capacity(frame_count);
    let mut sparse_index = 0;
    for (&center, &index) in centers.iter().zip(&left) {
        while sparse_index + 1 < sparse.len() && sparse[sparse_index] < index { sparse_index += 1; }
        let right_index = (sparse_index + 1).min(sparse.len() - 1);
        let (t0, t1) = (sparse_times[sparse_index], sparse_times[right_index]);
        let values = std::array::from_fn(|key| lerp(center, t0, t1,
            sparse_values[sparse_index][key], sparse_values[right_index][key]));
        frames.push(values);
        let c0 = cell_at(controls, t0);
        let c1 = cell_at(controls, t1);
        let uv = |cell: Option<usize>| if cell.is_some_and(|c|
            controls.voiced[c] && controls.continuity_identity[c] >= 0) { 1.0 } else { 0.0 };
        frame_voiced.push(lerp(center, t0, t1, uv(c0), uv(c1)) >= 0.5);
        frame_f0.push(lerp(center, t0, t1, controls.f0_hz[safe_cell_at(controls, t0)],
            controls.f0_hz[safe_cell_at(controls, t1)]));
    }
    let breath_enabled = frames.iter().zip(&frame_voiced).any(|(v, &uv)| uv && v[BRE] > 0.0);
    let noise = breath_enabled.then(|| NumpyPcg64::new(seed ^ 0x5658_4334).standard_normal_vec(padded_count));
    let window = signal::hann(size, true);
    let window_sum = window.iter().sum::<f64>();
    let frequency_step = 1.0 / (size as f64 * (1.0 / rate));
    let bins = size / 2 + 1;
    let frequencies: Vec<f64> = (0..bins).map(|i| i as f64 * frequency_step).collect();
    let high_shelf: Vec<f64> = frequencies.iter().map(|&f| {
        let squared = (f / 1800.0).powi(2); squared / (1.0 + squared)
    }).collect();
    let mut accumulated = vec![0.0; output_count];
    let mut normalization = vec![0.0; output_count];
    let mut audio_fft = FftPair::new(size);
    let mut spectral = vec![Complex64::new(0.0, 0.0); bins];
    let mut noise_spectral = vec![Complex64::new(0.0, 0.0); if breath_enabled { bins } else { 0 }];
    let mut log_magnitude = vec![0.0; bins];
    let mut warped = vec![0.0; bins];
    let mut gains = vec![0.0; bins];
    for frame_index in 0..frame_count {
        let origin = (frame_index * hop) as i64 - (size / 2) as i64;
        audio_fft.analyze(source, origin, &window, window_sum, &mut spectral);
        let value = frames[frame_index];
        if frame_voiced[frame_index] && value[..5] != DEFAULTS[..5] {
            for (log, z) in log_magnitude.iter_mut().zip(&spectral) {
                *log = z.norm().max(1e-10).ln();
            }
            let sigma = (0.65 * frame_f0[frame_index].max(60.0) / (rate / size as f64)).max(2.0);
            let envelope = signal::gaussian_filter1d(&log_magnitude, sigma, BorderMode::Nearest)?;
            let ratio = 2.0_f64.powf(-(value[GEN] - 64.0) / 64.0 * 0.28);
            let closed = 1.0 - value[OPE] / 127.0;
            let mut cursor = 0;
            for (index, &frequency) in frequencies.iter().enumerate() {
                let local_ratio = ratio * (1.0 - 0.18 * closed * (-(frequency / 1700.0).powi(2)).exp());
                let query = frequency / local_ratio;
                while cursor + 1 < bins && frequencies[cursor + 1] <= query { cursor += 1; }
                warped[index] = if query <= frequencies[0] { envelope[0] }
                    else if cursor + 1 == bins { envelope[bins - 1] }
                    else { lerp(query, frequencies[cursor], frequencies[cursor + 1], envelope[cursor], envelope[cursor + 1]) };
            }
            let broad = signal::gaussian_filter1d(&warped, (900.0 / (rate / size as f64)).max(4.0), BorderMode::Nearest)?;
            let tilt = (value[BRI] - 64.0) / if value[BRI] >= 64.0 { 63.0 } else { 64.0 } * 12.0 - 6.0 * closed;
            for index in 0..bins {
                let contrast = (warped[index] - broad[index]) * (0.45 * value[CLE] / 127.0);
                let tilt_db = tilt * high_shelf[index];
                let log_gain = warped[index] - envelope[index] + contrast + tilt_db * std::f64::consts::LN_10 / 20.0;
                gains[index] = log_gain.clamp(-2.3, 2.3).exp();
                spectral[index] *= gains[index];
            }
            if value[BRE] > 0.0 {
                let noise = noise.as_deref().context("active breath requires its full-run random stream")?;
                audio_fft.analyze(noise, origin, &window, window_sum, &mut noise_spectral);
                for index in 0..bins {
                    // The gain already contains GEN/OPE. Starting from the
                    // warped envelope here would warp breath a second time.
                    let magnitude = envelope[index].exp() * gains[index];
                    noise_spectral[index] *= magnitude;
                }
                let norm = noise_spectral.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
                if norm > 0.0 {
                    let tone_norm_squared = spectral.iter().map(|z| z.norm_sqr()).sum::<f64>();
                    let tone_norm = tone_norm_squared.sqrt();
                    let scale = tone_norm / norm;
                    let amount = 0.50 * value[BRE] / 127.0;
                    let divisor = (1.0 + amount * amount).sqrt();
                    for index in 0..bins {
                        let shaped = noise_spectral[index] * scale;
                        spectral[index] = (spectral[index] + amount * shaped) / divisor;
                    }
                }
            }
        }
        let begin = frame_index * hop;
        audio_fft.overlap_add(&spectral, &window, window_sum, &mut accumulated[begin..begin + size]);
        for offset in 0..size { normalization[begin + offset] += window[offset] * window[offset]; }
    }
    let offset = size / 2;
    // Reuse the OLA buffer for final samples, preserving the full-run grid.
    accumulated.copy_within(offset..offset + count, 0);
    accumulated.truncate(count);
    for (index, value) in accumulated.iter_mut().enumerate() {
        let divisor = normalization[offset + index];
        *value /= if divisor > 1e-10 { divisor } else { 1.0 };
    }
    if growl_enabled {
        // Sequential cumulative F0 over the entire waveform, without a reset
        // at an analysis tile, phone boundary or inactive interval.
        let mut phase_sum = 0.0;
        let phase_scale = 2.0 * std::f64::consts::PI / rate;
        for begin in (0..count).step_by(BLOCK) {
            let end = (begin + BLOCK).min(count);
            let times: Vec<f64> = (begin..end).map(|i| i as f64 / rate).collect();
            let values = track.values_at(&times, mapping.as_ref());
            for (local, (&time, value)) in times.iter().zip(&values).enumerate() {
                let index = begin + local;
                phase_sum += controls.f0_hz[safe_cell_at(controls, time)];
                let phase = phase_sum * phase_scale;
                let amount = value[GWL] / 127.0 * if voiced[index] { 1.0 } else { 0.0 };
                let half = 0.55 * amount;
                let third = 0.20 * amount;
                let modulation = (1.0 + half * (phase / 2.0).sin() + third * (phase / 3.0).sin()) /
                    (1.0 + (half * half + third * third) / 2.0).sqrt();
                accumulated[index] *= modulation;
            }
        }
    }
    for index in 0..count {
        accumulated[index] = if active[index] {
            source[index] + blend[index] * (accumulated[index] - source[index])
        } else { source[index] };
    }
    ensure!(accumulated.iter().all(|v| v.is_finite()), "expression produced nonfinite samples");
    ensure!(accumulated.iter().zip(source).zip(&voiced).all(|((&a, &b), &uv)| uv || a.to_bits() == b.to_bits()),
        "expression modified an unvoiced sample");
    report["exact_bypass"] = json!(false);
    report["changed_samples"] = json!(accumulated.iter().zip(source).filter(|(a, b)| a != b).count());
    report["peak_before"] = json!(peak(source));
    report["peak_after"] = json!(peak(&accumulated));
    report["fft_size"] = json!(size);
    report["hop_samples"] = json!(hop);
    report["unvoiced_exact"] = json!(true);
    Ok(ExpressionResult { samples: accumulated, dtype: SampleDtype::Float64, report, active, blend: Some(blend) })
}
