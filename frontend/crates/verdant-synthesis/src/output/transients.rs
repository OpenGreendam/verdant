use anyhow::{ensure, Result};
use serde_json::{json, Value};
use crate::core::AcousticControlTrack;
use super::{cell_at, exact_fields, finite_range, seconds_to_samples, SampleDtype, TRANSIENT_METHOD};
use super::filters::{self, Butterworth3};

fn eligible_class(value: &str) -> bool {
    matches!(value, "stop_unaspirated" | "stop_aspirated" | "stop_voiced" |
        "affricate" | "fricative" | "sibilant" | "aspiration")
}
fn release_class(value: &str) -> bool {
    matches!(value, "stop_unaspirated" | "stop_aspirated" | "stop_voiced" | "affricate")
}
fn frication_class(value: &str) -> bool {
    matches!(value, "affricate" | "fricative" | "sibilant" | "aspiration")
}

pub(super) struct Configuration { pub strength: f64, pub cap: f64 }
impl Configuration {
    pub(super) fn parse(value: &Value) -> Result<Self> {
        let object = exact_fields(value, &["method", "strength", "max_attenuation_db"], "output_transients")?;
        ensure!(object["method"].as_str() == Some(TRANSIENT_METHOD), "unsupported output_transients method");
        Ok(Self {
            strength: finite_range(&object["strength"], 0.0, 1.0, "output_transients strength")?,
            cap: finite_range(&object["max_attenuation_db"], 0.0, 12.0, "output_transients max_attenuation_db")?,
        })
    }
}

fn same_region(controls: &AcousticControlTrack, a: usize, b: usize) -> bool {
    controls.phone_identity[a] == controls.phone_identity[b]
        && controls.segment_identity[a] == controls.segment_identity[b]
        && controls.continuity_identity[a] == controls.continuity_identity[b]
        && controls.phonetic_classes[a] == controls.phonetic_classes[b]
        && controls.voiced[a] == controls.voiced[b]
}

fn sample_boundary(controls: &AcousticControlTrack, frame: usize, count: usize,
    rate: f64, offset: f64) -> usize
{
    // Search the exact uniform-clock sample-center expression. Algebraically
    // replacing it with ceil(frame_time*rate-.5) changes floating boundaries.
    let mut low = 0;
    let mut high = count;
    while low < high {
        let middle = low + (high - low) / 2;
        let time = offset + (middle as f64 + 0.5) / rate;
        let cell = ((time - controls.origin_seconds) / controls.period_seconds).floor();
        if cell < frame as f64 { low = middle + 1; } else { high = middle; }
    }
    low
}

pub(super) fn apply(
    audio: &mut [f64], dtype: SampleDtype, sample_rate: u32,
    controls: &AcousticControlTrack, configuration: &Configuration, time_offset: f64,
) -> Result<(Value, Vec<f64>, Vec<bool>)> {
    ensure!(sample_rate >= 8000, "output_transients sample rate must be at least 8000");
    ensure!(time_offset.is_finite(), "output_transients time offset must be finite");
    let rate = sample_rate as f64;
    let frame_eligible: Vec<bool> = (0..controls.f0_hz.len()).map(|index|
        !controls.voiced[index] && controls.continuity_identity[index] >= 0 &&
            eligible_class(&controls.phonetic_classes[index])).collect();
    let eligible: Vec<bool> = (0..audio.len()).map(|index|
        cell_at(controls, time_offset + (index as f64 + 0.5) / rate)
            .is_some_and(|cell| frame_eligible[cell])).collect();
    let mut gain = vec![1.0; audio.len()];
    let mut regions = Vec::new();
    let edge = seconds_to_samples(0.002, rate).max(1);
    let lookahead = seconds_to_samples(0.0015, rate).max(1);
    let rms_width = (seconds_to_samples(0.008, rate) | 1).max(3);
    let slow_width = (seconds_to_samples(0.024, rate) | 1).max(3);
    let lowpass = Butterworth3::new(200.0, rate, false)?;
    let highpass = Butterworth3::new(3000.0_f64.min(rate * 0.375), rate, true)?;
    let epsilon = 1e-12;
    let mut first = 0;
    let mut attenuated = 0;
    while first < frame_eligible.len() {
        let mut last = first + 1;
        while last < frame_eligible.len() && same_region(controls, last - 1, last) { last += 1; }
        if !frame_eligible[first] { first = last; continue; }
        let begin = sample_boundary(controls, first, audio.len(), rate, time_offset);
        let end = sample_boundary(controls, last, audio.len(), rate, time_offset);
        if end <= begin { first = last; continue; }
        let source = audio[begin..end].to_vec();
        let kind = controls.phonetic_classes[first].as_str();
        let short = filters::rms(&source, rms_width)?;
        let reference = filters::percentile(&short, 75.0);
        let crest = if release_class(kind) { 3.0 } else { 3.5 };
        let ceiling: Vec<f64> = short.iter().map(|&v|
            crest * v.max(reference * 0.25).min(reference.max(epsilon))).collect();
        let peak_excess: Vec<f64> = source.iter().zip(&ceiling).map(|(x, &ceiling)|
            (20.0 * (x.abs().max(epsilon) / ceiling.max(epsilon)).log10()).max(0.0)).collect();
        let (low, high) = if source.len() > 1 {
            (lowpass.filtfilt(&source)?, highpass.filtfilt(&source)?)
        } else { (source.clone(), vec![0.0; 1]) };
        let low_rms = filters::rms(&low, rms_width)?;
        let low_limit = if release_class(kind) { 0.40 } else { 0.30 } * reference.max(epsilon);
        let pressure: Vec<f64> = low_rms.iter().map(|&v|
            (20.0 * (v.max(epsilon) / low_limit).log10()).max(0.0)).collect();
        let mut frication = vec![0.0; source.len()];
        if frication_class(kind) {
            let high_short = filters::rms(&high, rms_width)?;
            let high_slow = filters::rms(&high, slow_width)?;
            let high_floor = (filters::percentile(&high_short, 50.0) * 0.5).max(epsilon);
            for index in 0..source.len() {
                frication[index] = (20.0 * (high_short[index].max(epsilon) /
                    (high_slow[index].max(high_floor) * 1.5)).log10()).max(0.0);
            }
        }
        let required: Vec<f64> = (0..source.len()).map(|index|
            (peak_excess[index].max(pressure[index]).max(frication[index]) * configuration.strength)
                .min(configuration.cap)).collect();
        let shaped = filters::smooth_majorant(&required, lookahead)?;
        let mut maximum_attenuation = 0.0_f64;
        let mut peak_before = 0.0_f64;
        let mut peak_after = 0.0_f64;
        let mut sum_before = 0.0;
        let mut sum_after = 0.0;
        let mut above_target = 0;
        for index in 0..source.len() {
            let distance = index.min(source.len() - 1 - index);
            let weight = 0.5 - 0.5 * (std::f64::consts::PI * (distance as f64 / edge as f64).min(1.0)).cos();
            let gain_db = -shaped[index].min(configuration.cap) * weight;
            maximum_attenuation = maximum_attenuation.max(-gain_db);
            let at = begin + index;
            gain[at] = (gain_db * (std::f64::consts::LN_10 / 20.0)).exp();
            if gain[at] < 1.0 {
                ensure!(eligible[at], "output_transients attempted to modify a noneligible sample");
                audio[at] = dtype.store(source[index] * gain[at]);
                attenuated += 1;
            }
            let before = source[index];
            let after = audio[at];
            ensure!(after.is_finite() && after.abs() <= before.abs(),
                "output_transients violated its final-sample attenuation contract");
            peak_before = peak_before.max(before.abs());
            peak_after = peak_after.max(after.abs());
            sum_before += before * before;
            sum_after += after * after;
            above_target += usize::from(after.abs() > ceiling[index] * (1.0 + 1e-7));
        }
        regions.push(json!({
            "first_frame": first, "frame_count": last - first,
            "first_sample": begin, "sample_count": end - begin, "phonetic_class": kind,
            "reference_rms": reference, "crest_factor": crest, "pressure_limit_rms": low_limit,
            "peak_detector_max_db": peak_excess.iter().copied().fold(0.0_f64, f64::max),
            "pressure_detector_max_db": pressure.iter().copied().fold(0.0_f64, f64::max),
            "frication_detector_max_db": frication.iter().copied().fold(0.0_f64, f64::max),
            "maximum_attenuation_db": maximum_attenuation,
            "peak_before": peak_before, "peak_after": peak_after,
            "rms_before": (sum_before / source.len() as f64).sqrt(),
            "rms_after": (sum_after / source.len() as f64).sqrt(),
            "samples_above_peak_target_after": above_target,
        }));
        first = last;
    }
    let minimum_gain = gain.iter().copied().fold(1.0_f64, f64::min);
    let maximum_gain = gain.iter().copied().fold(0.0_f64, f64::max);
    let report = json!({
        "configuration": {"method": TRANSIENT_METHOD, "strength": configuration.strength,
            "max_attenuation_db": configuration.cap},
        "applied": attenuated > 0,
        "core": "verdant_synthesis::output::transients",
        "oracle_core": "verdant_acoustics.output_transients.guard_output_transients",
        "status": "declared final-output DSP policy; perceptual benefit requires listening",
        "normalization": "none", "time_offset_seconds": time_offset,
        "sample_rate": rate, "total_samples": audio.len(),
        "sample_gain_range": [minimum_gain, maximum_gain],
        "eligible_samples": eligible.iter().filter(|&&v| v).count(),
        "attenuated_samples": attenuated, "sample_peak_monotone": true,
        "noneligible_samples_bit_exact": true, "length_unchanged": true,
        "phone_boundaries_modified": false,
        "detector_policy": {"short_rms_seconds": 0.008, "frication_reference_seconds": 0.024,
            "pressure_lowpass_hz": 200.0, "frication_highpass_hz": 3000.0_f64.min(rate * 0.375),
            "lookahead_seconds": 0.0015, "boundary_taper_seconds": 0.002,
            "anchor": "75th percentile of phone-local short RMS", "targets_are_hard_limits": false,
            "target_exceptions": "strength below one, configured attenuation cap and 2 ms boundary taper"},
        "regions": regions,
    });
    Ok((report, gain, eligible))
}
