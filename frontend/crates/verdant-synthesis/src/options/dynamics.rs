use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::f64::consts::{LN_10, PI};
use std::sync::LazyLock;
use crate::core::AcousticControlTrack;
use super::common::{bounded_number, exact_object, median_bounds, range, validate_controls};

const MODEL_METHOD: &str = "voiced-envelope-v1";
const CONSONANT_METHOD: &str = "unvoiced-model-envelope-v1";
static CORE_SOURCE_SHA256: LazyLock<String> = LazyLock::new(||
    hex::encode(sha2::Sha256::digest(include_bytes!("dynamics.rs"))));

fn validate(value: &Value, name: &str, method: &str) -> Result<Option<Value>> {
    if value.is_null() { return Ok(None); }
    let object = exact_object(value, &["method", "strength", "max_attenuation_db"], name)?;
    ensure!(object["method"] == method, "unsupported {name} method");
    let strength = bounded_number(&object["strength"], 0., 1., &format!("{name} strength"))?;
    let cap = bounded_number(&object["max_attenuation_db"], 0., 12., &format!("{name} max_attenuation_db"))?;
    Ok(Some(json!({"method": method, "strength": strength, "max_attenuation_db": cap})))
}
pub fn validate_model_dynamics(value: &Value) -> Result<Option<Value>> {
    validate(value, "model_dynamics", MODEL_METHOD)
}
pub fn validate_consonant_dynamics(value: &Value) -> Result<Option<Value>> {
    validate(value, "consonant_dynamics", CONSONANT_METHOD)
}
fn capabilities(method: &str) -> Value {
    json!({"methods": [method], "minimum_strength": 0., "maximum_strength": 1.,
        "minimum_attenuation_db": 0., "maximum_attenuation_db": 12.})
}
pub fn model_dynamics_capabilities() -> Value { capabilities(MODEL_METHOD) }
pub fn consonant_dynamics_capabilities() -> Value { capabilities(CONSONANT_METHOD) }

/// Samples and all diagnostic traces use the source's exact complete clock.
/// Neutral and protected samples are copied without performing arithmetic.
pub struct DynamicsResult {
    pub samples: Vec<f64>,
    pub sample_gain: Vec<f64>,
    pub sample_eligible: Vec<bool>,
    pub report: Value,
}

pub struct NoiseDynamics {
    pub sample_gain: Vec<f64>,
    pub sample_eligible: Vec<bool>,
    pub report: Value,
}

#[derive(Clone, Copy)]
enum Kind { Model, Consonant }
impl Kind {
    fn name(self) -> &'static str {
        match self { Self::Model => "model_dynamics", Self::Consonant => "consonant_dynamics" }
    }
    fn edge_seconds(self) -> f64 {
        match self { Self::Model => 0.005, Self::Consonant => 0.002 }
    }
    fn eligible(self, controls: &AcousticControlTrack, index: usize) -> bool {
        controls.continuity_identity[index] >= 0 && match self {
            Self::Model => controls.voiced[index] && controls.phonetic_classes[index] == "vowel",
            Self::Consonant => !controls.voiced[index] && matches!(controls.phonetic_classes[index].as_str(),
                "stop_unaspirated" | "stop_aspirated" | "stop_voiced" | "affricate"),
        }
    }
}

fn sample_lower_bound(length: usize, frame: usize, period_samples: f64) -> usize {
    let mut lower = 0;
    let mut upper = length;
    while lower < upper {
        let middle = lower+(upper-lower)/2;
        let cell = ((middle as f64+0.5)/period_samples).floor();
        if cell < frame as f64 { lower = middle+1; } else { upper = middle; }
    }
    lower
}
fn boundary(controls: &AcousticControlTrack, index: usize) -> bool {
    controls.voiced[index] != controls.voiced[index-1] ||
        controls.phonetic_classes[index] != controls.phonetic_classes[index-1] ||
        controls.phone_identity[index] != controls.phone_identity[index-1] ||
        controls.segment_identity[index] != controls.segment_identity[index-1] ||
        controls.continuity_identity[index] != controls.continuity_identity[index-1]
}

// Compute a clipped difference without overflowing finite extreme log values.
// Scaling each operand first is only needed when subtraction itself overflows.
fn frame_attenuation(log: f64, lower: f64, upper: f64, strength: f64, cap: f64) -> f64 {
    if log >= upper || strength == 0. || cap == 0. { return 0.; }
    let difference = |anchor: f64| {
        let residual = log-anchor;
        if residual.is_finite() { residual*strength }
        else { log*strength-anchor*strength }
    };
    // Keep both middle observations until after subtraction. Rounding their
    // midpoint to f64 first loses the entire residual for adjacent huge logs.
    let scaled = difference(lower)/2.+difference(upper)/2.;
    if !scaled.is_finite() { -cap }
    else { (scaled*(20./LN_10)).clamp(-cap, 0.) }
}

fn gains(length: usize, logs: &[f64], controls: &AcousticControlTrack,
    value: &Value, sample_rate: f64, kind: Kind) -> Result<Option<NoiseDynamics>> {
    let config = match kind {
        Kind::Model => validate_model_dynamics(value)?,
        Kind::Consonant => validate_consonant_dynamics(value)?,
    };
    let Some(config) = config else { return Ok(None); };
    ensure!(sample_rate.is_finite() && sample_rate > 0., "{} sample_rate must be finite and positive", kind.name());
    let frames = validate_controls(controls)?;
    ensure!(logs.len() == frames && logs.iter().all(|v| v.is_finite()),
        "{} log gain must be finite and align with every control cell", kind.name());
    let period_samples = controls.period_seconds*sample_rate;
    let expected = (frames as f64*period_samples).round_ties_even();
    ensure!(period_samples.is_finite() && expected.is_finite() && length > 0 && expected == length as f64,
        "{} source duration must match the complete control grid", kind.name());
    let strength = config["strength"].as_f64().unwrap();
    let cap = config["max_attenuation_db"].as_f64().unwrap();
    let edge_samples = (kind.edge_seconds()*sample_rate).round_ties_even().max(1.) as usize;
    let mut sample_eligible = vec![false; length];
    let mut eligible_frames = 0usize;
    for frame in 0..frames {
        if kind.eligible(controls, frame) {
            eligible_frames += 1;
            let begin = sample_lower_bound(length, frame, period_samples);
            let end = sample_lower_bound(length, frame+1, period_samples);
            sample_eligible[begin..end].fill(true);
        }
    }
    let mut gain_db = vec![0.; length];
    let mut regions = Vec::new();
    let mut first = 0;
    while first < frames {
        let mut last = first+1;
        while last < frames && !boundary(controls, last) { last += 1; }
        if kind.eligible(controls, first) {
            let begin = sample_lower_bound(length, first, period_samples);
            let end = sample_lower_bound(length, last, period_samples);
            let local_logs = &logs[first..last];
            let (lower_anchor, upper_anchor) = match kind {
                Kind::Model => median_bounds(local_logs),
                Kind::Consonant => {
                    let anchor = local_logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    (anchor, anchor)
                },
            };
            let center = lower_anchor/2.+upper_anchor/2.;
            if strength != 0. && cap != 0. && end > begin {
                let frame_db: Vec<f64> = local_logs.iter().map(|&log|
                    frame_attenuation(log, lower_anchor, upper_anchor, strength, cap)).collect();
                for sample in begin..end {
                    let query = (sample as f64+0.5)-first as f64*period_samples;
                    let cell = query/period_samples-0.5;
                    let value = if cell <= 0. { frame_db[0] }
                        else if cell >= (frame_db.len()-1) as f64 { frame_db[frame_db.len()-1] }
                        else {
                            let i = cell.floor() as usize;
                            let left = (i as f64+0.5)*period_samples;
                            let right = (i as f64+1.5)*period_samples;
                            (frame_db[i+1]-frame_db[i])/(right-left)*(query-left)+frame_db[i]
                        };
                    let inside = (sample-begin).min(end-1-sample);
                    let phase = (inside as f64/edge_samples as f64).min(1.);
                    gain_db[sample] = value*(0.5-0.5*(PI*phase).cos());
                }
            }
            regions.push(json!({"first_frame": first, "frame_count": last-first,
                "first_sample": begin, "sample_count": end-begin, "anchor_log_gain": center,
                "sample_gain_db_range": if end > begin { Some(range(&gain_db[begin..end])) } else { None }}));
        }
        first = last;
    }
    let sample_gain: Vec<f64> = gain_db.iter().map(|&value| (value*(LN_10/20.)).exp()).collect();
    ensure!(sample_gain.iter().all(|gain| gain.is_finite() && *gain >= 0. && *gain <= 1.),
        "{} violated its attenuation-only gain contract", kind.name());
    let mut report = json!({"configuration": config,
        "core": match kind { Kind::Model => "verdant_synthesis::options::apply_model_dynamics",
            Kind::Consonant => "verdant_synthesis::options::condition_noise_envelope" },
        "applied": sample_gain.iter().any(|&gain| gain != 1.), "normalization": "none",
        "anchor": match kind { Kind::Model => "median natural-log amplitude within each eligible contiguous region",
            Kind::Consonant => "maximum natural-log amplitude within each eligible contiguous region; no event-time inference" },
        "eligibility": match kind { Kind::Model => "explicit voiced vowel with nonnegative continuity identity",
            Kind::Consonant => "explicit unvoiced stop/affricate with nonnegative continuity identity" },
        "boundaries": ["phone_identity", "segment_identity", "continuity_identity", "phonetic_classes", "voiced"],
        "edge_seconds": kind.edge_seconds(), "edge_samples": edge_samples,
        "control_period_seconds": controls.period_seconds, "control_origin_seconds": controls.origin_seconds,
        "sample_rate": sample_rate, "total_samples": length, "eligible_frames": eligible_frames,
        "eligible_samples": sample_eligible.iter().filter(|&&value| value).count(),
        "attenuated_samples": sample_gain.iter().filter(|&&value| value != 1.).count(),
        "sample_gain_range": range(&sample_gain), "sample_gain_db_range": range(&gain_db), "regions": regions,
        "core_source_sha256": &*CORE_SOURCE_SHA256,
        "numerical_oracle_source_sha256": match kind {
            Kind::Model => "d1906b13c5dce07e72aed82ab329842204f7e05189235a17ae5ddd3e9315e708",
            Kind::Consonant => "0d8629738df0a57d524949d3371dc9856c23bbfa04b4f3325d997e1da4b764a2" }
    });
    if matches!(kind, Kind::Consonant) {
        report["eligible_classes"] = json!(["affricate", "stop_aspirated", "stop_unaspirated", "stop_voiced"]);
        report["periodic_component_modified"] = json!(false);
        report["phone_boundaries_modified"] = json!(false);
        report["post_filter_waveform_isolation_claimed"] = json!(false);
        report["noneligible_envelope_samples_bit_exact"] = json!(true);
    }
    Ok(Some(NoiseDynamics { sample_gain, sample_eligible, report }))
}

pub(super) fn noise_dynamics(length: usize, logs: &[f64], controls: &AcousticControlTrack,
    config: &Value, sample_rate: f64) -> Result<Option<NoiseDynamics>> {
    gains(length, logs, controls, config, sample_rate, Kind::Consonant)
}

fn apply(source: &[f64], logs: &[f64], controls: &AcousticControlTrack,
    config: &Value, sample_rate: f64, kind: Kind) -> Result<Option<DynamicsResult>> {
    let Some(gain) = gains(source.len(), logs, controls, config, sample_rate, kind)? else {
        return Ok(None);
    };
    ensure!(!source.is_empty() && source.iter().all(|v| v.is_finite()),
        "{} source must be a nonempty finite real floating vector", kind.name());
    if matches!(kind, Kind::Consonant) {
        ensure!(source.iter().all(|&value| value >= 0.), "noise envelope must be nonnegative");
    }
    let mut result = DynamicsResult { samples: source.to_vec(), sample_gain: gain.sample_gain,
        sample_eligible: gain.sample_eligible, report: gain.report };
    for (sample, &gain) in result.samples.iter_mut().zip(&result.sample_gain) {
        if gain != 1. { *sample *= gain; }
    }
    ensure!(result.samples.iter().all(|v| v.is_finite()), "nonfinite dynamics result");
    let protected_exact = source.iter().zip(&result.samples).zip(&result.sample_eligible)
        .all(|((&before, &after), &eligible)| eligible || before.to_bits() == after.to_bits());
    ensure!(protected_exact, "{} modified a protected sample", kind.name());
    if matches!(kind, Kind::Model) {
        let before = source.iter().map(|value| value.abs()).fold(0., f64::max);
        let after = result.samples.iter().map(|value| value.abs()).fold(0., f64::max);
        ensure!(after <= before, "model dynamics increased the input peak");
        result.report["input_peak"] = json!(before);
        result.report["output_peak"] = json!(after);
        result.report["peak_did_not_increase"] = json!(true);
        result.report["noneligible_samples_bit_exact"] = json!(protected_exact);
    } else {
        ensure!(source.iter().zip(&result.samples).all(|(&before, &after)| after <= before),
            "consonant dynamics increased its noise envelope");
        result.report["noneligible_envelope_samples_bit_exact"] = json!(protected_exact);
    }
    Ok(Some(result))
}

pub fn apply_model_dynamics(source: &[f64], logs: &[f64], controls: &AcousticControlTrack,
    config: &Value, sample_rate: f64) -> Result<Option<DynamicsResult>> {
    apply(source, logs, controls, config, sample_rate, Kind::Model)
}
pub fn condition_noise_envelope(source: &[f64], logs: &[f64], controls: &AcousticControlTrack,
    config: &Value, sample_rate: f64) -> Result<Option<DynamicsResult>> {
    apply(source, logs, controls, config, sample_rate, Kind::Consonant)
}

use sha2::Digest;
