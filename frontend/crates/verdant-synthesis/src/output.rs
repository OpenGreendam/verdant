//! Final audible processing: voice expression, expression-local headroom, then
//! the phoneme-local consonant guard. Upstream arrays and metadata are retained.
//!
//! The numerical oracle is `native/verdant_acoustics/{voice_expression,
//! output_pipeline,output_transients}.py` at the admitted production baseline.
//! Expression never changes source pitch/time; only explicitly voiced, owned
//! samples can change. The final guard only attenuates explicitly eligible UV
//! samples. Neutral and protected samples retain their original floating bits.
//! File publication and immutable-operation admission belong to the worker.
//! Its PCM publisher must use `quantize_preserving_pcm16` with the original
//! decoded PCM codes, rather than quantizing the entire final floating vector.

mod expression;
mod filters;
mod transients;

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Map, Value};
use crate::core::{AcousticControlTrack, AudioResult, NumericArray};

pub const EXPRESSION_METHOD: &str = "vocaloid-controls-v1";
pub const TRANSIENT_METHOD: &str = "phoneme-output-envelope-v1";
const BLOCK: usize = 65_536;

/// Validate the complete compiled controller contract without processing audio.
pub fn validate_expression(value: &Value) -> Result<()> { expression::validate(value) }

/// Validate the complete final-guard contract; JSON null is an explicit bypass.
pub fn validate_output_transients(value: &Value) -> Result<()> {
    if value.is_null() { return Ok(()); }
    transients::Configuration::parse(value).map(|_| ())
}

pub fn output_transients_capabilities() -> Value {
    json!({"methods": [TRANSIENT_METHOD], "minimum_strength": 0.0, "maximum_strength": 1.0,
        "minimum_attenuation_db": 0.0, "maximum_attenuation_db": 12.0,
        "stage": "final waveform after reconstruction and voice expression",
        "eligible_classes": ["affricate", "aspiration", "fricative", "sibilant", "stop_aspirated", "stop_unaspirated", "stop_voiced"],
        "sample_peak_monotone": true, "voiced_samples_bit_exact": true})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SampleDtype { Float32, Float64 }

impl SampleDtype {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "float32" => Ok(Self::Float32),
            "float64" => Ok(Self::Float64),
            _ => bail!("final output requires float32 or float64 source dtype"),
        }
    }
    fn name(self) -> &'static str {
        match self { Self::Float32 => "float32", Self::Float64 => "float64" }
    }
    pub(super) fn store(self, value: f64) -> f64 {
        match self { Self::Float32 => f64::from(value as f32), Self::Float64 => value }
    }
    fn tolerance(self) -> f64 {
        let epsilon = match self {
            Self::Float32 => f64::from(f32::EPSILON),
            Self::Float64 => f64::EPSILON,
        };
        (4.0 * epsilon).max(1e-12)
    }
}

pub(super) fn peak(samples: &[f64]) -> f64 {
    samples.iter().fold(0.0_f64, |p, v| p.max(v.abs()))
}
fn rms(samples: &[f64]) -> f64 {
    (samples.iter().map(|v| v * v).sum::<f64>() / samples.len() as f64).sqrt()
}
pub(super) fn seconds_to_samples(seconds: f64, rate: f64) -> usize {
    (seconds * rate).round_ties_even() as usize
}

pub(super) fn validate_controls(controls: &AcousticControlTrack) -> Result<()> {
    let count = controls.f0_hz.len();
    ensure!(count > 0 && [controls.voiced.len(), controls.phonetic_classes.len(),
        controls.phone_identity.len(), controls.segment_identity.len(),
        controls.continuity_identity.len()].iter().all(|&n| n == count),
        "acoustic controls require aligned nonempty one-dimensional cells");
    ensure!(controls.f0_hz.iter().zip(&controls.voiced).all(|(&f, &uv)|
        f.is_finite() && if uv { f > 0.0 } else { f == 0.0 }),
        "control F0 must be finite, positive for voiced cells and zero for UV");
    ensure!(controls.phonetic_classes.iter().all(|s| matches!(s.as_str(),
        "vowel" | "nasal" | "sonorant" | "voiced_fricative" | "sibilant" |
        "fricative" | "aspiration" | "stop_unaspirated" | "stop_aspirated" |
        "stop_voiced" | "affricate" | "closure" | "silence" | "unknown")),
        "explicit supported phonetic classes are required");
    let period = controls.period_seconds;
    let origin = controls.origin_seconds;
    ensure!(period.is_finite() && period > 0.0 && origin.is_finite(),
        "control clock needs a finite positive period and finite origin");
    ensure!((origin + count as f64 * period).is_finite() && origin + period != origin,
        "control clock is not representable at the requested precision");
    Ok(())
}

pub(super) fn cell_at(controls: &AcousticControlTrack, time: f64) -> Option<usize> {
    let index = ((time - controls.origin_seconds) / controls.period_seconds).floor();
    (index >= 0.0 && index < controls.f0_hz.len() as f64).then_some(index as usize)
}

pub(super) fn safe_cell_at(controls: &AcousticControlTrack, time: f64) -> usize {
    let index = ((time - controls.origin_seconds) / controls.period_seconds).floor();
    if !index.is_finite() || index < 0.0 { 0 }
    else { (index as usize).min(controls.f0_hz.len() - 1) }
}

pub(super) fn exact_fields<'a>(value: &'a Value, fields: &[&str], kind: &str)
    -> Result<&'a Map<String, Value>>
{
    let object = value.as_object().with_context(|| format!("{kind} must be an object"))?;
    ensure!(object.len() == fields.len() && fields.iter().all(|f| object.contains_key(*f)),
        "{kind} has invalid fields");
    Ok(object)
}
pub(super) fn finite_range(value: &Value, lo: f64, hi: f64, field: &str) -> Result<f64> {
    let number = value.as_f64().with_context(|| format!("{field} must be numeric"))?;
    ensure!(number.is_finite() && number >= lo && number <= hi,
        "{field} must be finite and within {lo}..{hi}");
    Ok(number)
}
fn trace_array(values: Vec<f64>, dtype: &str) -> Result<NumericArray> {
    NumericArray::new(vec![values.len()], dtype, values)
}
fn boolean_array(values: &[bool]) -> Result<NumericArray> {
    trace_array(values.iter().map(|&v| if v { 1.0 } else { 0.0 }).collect(), "bool")
}

/// Preserve the original PCM conversion at every unchanged floating sample.
/// With no enabled expression, the final guard also cannot increase an
/// upstream integer magnitude, including C1's different prior rounding rule.
pub fn quantize_preserving_pcm16(
    upstream_pcm: &[i16], source_float: &[f64], final_float: &[f64],
    expression_enabled: bool,
) -> Result<Vec<i16>> {
    ensure!(!upstream_pcm.is_empty() && upstream_pcm.len() == source_float.len()
        && final_float.len() == source_float.len(), "final PCM sources must be aligned and nonempty");
    ensure!(source_float.iter().chain(final_float).all(|v| v.is_finite()),
        "final PCM sources must be finite");
    let mut pcm = upstream_pcm.to_vec();
    for (index, (&prior, &candidate)) in source_float.iter().zip(final_float).enumerate() {
        if prior == candidate { continue; }
        let rounded = (candidate * 32768.0).round_ties_even().clamp(-32768.0, 32767.0);
        let mut code = rounded as i32;
        if !expression_enabled {
            let magnitude = code.abs().min(i32::from(upstream_pcm[index]).abs());
            code = code.signum() * magnitude;
        }
        pcm[index] = code as i16;
    }
    Ok(pcm)
}

/// Finalize a reconstructed floating waveform and retain all upstream evidence.
/// `time_map` is (rendered seconds, authored seconds); duplicate rendered
/// anchors retain zero-frame gaps and use the rightmost anchor at a boundary.
/// `seed` is the admitted route seed; expression's NumPy stream admits u32.
pub fn finalize(
    audio: AudioResult,
    controls: &AcousticControlTrack,
    expression: Option<&Value>,
    output_transients: Option<&Value>,
    time_map: Option<(&[f64], &[f64])>,
    seed: u64,
) -> Result<AudioResult> {
    let expression = expression.filter(|v| !v.is_null());
    let output_transients = output_transients.filter(|v| !v.is_null());
    if expression.is_none() && output_transients.is_none() { return Ok(audio); }
    let AudioResult { sample_rate, samples: source, source_dtype, mut arrays, metadata } = audio;
    let original_dtype = SampleDtype::parse(&source_dtype)?;
    ensure!(!source.is_empty() && source.iter().all(|v| v.is_finite()),
        "final output requires a nonempty finite floating mono vector");
    if original_dtype == SampleDtype::Float32 {
        ensure!(source.iter().all(|&v| f64::from(v as f32) == v),
            "float32 source samples must retain their original precision");
    }
    validate_controls(controls)?;
    let transient_config = output_transients.map(transients::Configuration::parse).transpose()?;
    let mut reports = Map::new();
    let (mut current, mut dtype, expression_enabled) = if let Some(track) = expression {
        let result = expression::apply(&source, original_dtype, sample_rate, controls, track, seed, time_map)?;
        let enabled = result.report["enabled"].as_bool().unwrap_or(false);
        arrays.insert("expression_active".into(), boolean_array(&result.active)?);
        if let Some(blend) = &result.blend {
            arrays.insert("expression_blend".into(), trace_array(blend.clone(), "float64")?);
        }
        reports.insert("voice_expression".into(), result.report);
        let mut current = result.samples;
        if enabled {
            let blend = result.blend.as_deref().context("active expression requires its ownership blend")?;
            let (report, gain) = apply_headroom(&mut current, result.dtype, &source,
                original_dtype, &result.active, blend, sample_rate)?;
            reports.insert("expression_headroom".into(), report);
            if let Some(gain) = gain {
                arrays.insert("expression_headroom_gain".into(), trace_array(gain, "float64")?);
            }
        }
        (current, result.dtype, enabled)
    } else { (source.clone(), original_dtype, false) };
    if let Some(configuration) = transient_config {
        let (report, gain, eligible) = transients::apply(&mut current, dtype, sample_rate,
            controls, &configuration, 0.0)?;
        reports.insert("output_transients".into(), report);
        arrays.insert("transients_sample_gain".into(), trace_array(gain, "float64")?);
        arrays.insert("transients_sample_eligible".into(), boolean_array(&eligible)?);
    }
    ensure!(current.len() == source.len() && current.iter().all(|v| v.is_finite()),
        "final output changed dimensions or produced nonfinite samples");
    // A neutral expression leaves source storage unchanged; active expression
    // is an f64 OLA result even when its upstream C1 waveform was float32.
    if expression_enabled { dtype = SampleDtype::Float64; }
    let changed: Vec<bool> = current.iter().zip(&source).map(|(a, b)| a != b).collect();
    let changed_count = changed.iter().filter(|&&v| v).count();
    arrays.insert("waveform_final".into(), trace_array(current.clone(), dtype.name())?);
    arrays.insert("sample_rate".into(), NumericArray::new(vec![], "int32", vec![sample_rate as f64])?);
    arrays.insert("changed_samples".into(), boolean_array(&changed)?);
    let report = json!({
        "format": "verdant-output-pipeline-v1",
        "stage_order": ["voice_expression", "expression_headroom", "output_transients"],
        "sample_rate": sample_rate, "samples": current.len(), "seed": seed,
        "changed_float_samples": changed_count,
        "peak_before": peak(&source), "peak_after": peak(&current),
        "rms_before": rms(&source), "rms_after": rms(&current),
        "samples_outside_pcm_full_scale": current.iter().filter(|v| v.abs() >= 1.0).count(),
        "changed_samples_outside_pcm_full_scale": current.iter().zip(&changed)
            .filter(|(v, changed)| **changed && v.abs() >= 1.0).count(),
        "quantization": "PCM16 nearest integer at changed samples only; original codes elsewhere",
        "normalization": "none", "upstream_archives_modified": false,
        "upstream_pitch_measurement_scope": "before final voice expression and transient control; not a final-output measurement",
        "float_input_dtype": source_dtype, "float_output_dtype": dtype.name(),
        "implementation": "Rust production DSP port; no Python subprocess",
        "numerical_oracles": ["verdant_acoustics.voice_expression", "verdant_acoustics.output_pipeline",
            "verdant_acoustics.output_transients"],
        "processors": reports,
    });
    let mut metadata = match metadata {
        Value::Object(object) => object,
        other => { let mut object = Map::new(); object.insert("upstream_metadata".into(), other); object }
    };
    metadata.insert("output_pipeline".into(), report);
    Ok(AudioResult { sample_rate, samples: current, source_dtype: dtype.name().into(),
        arrays, metadata: Value::Object(metadata) })
}

fn apply_headroom(
    current: &mut [f64], current_dtype: SampleDtype,
    source: &[f64], source_dtype: SampleDtype,
    active: &[bool], blend: &[f64], sample_rate: u32,
) -> Result<(Value, Option<Vec<f64>>)> {
    ensure!(current.len() == source.len() && active.len() == source.len() && blend.len() == source.len(),
        "expression headroom requires aligned samples and ownership");
    let ceiling = 0.98_f64;
    let mut active_peak = 0.0_f64;
    let mut upstream_peak = 0.0_f64;
    let mut outside_peak = 0.0_f64;
    for (index, &sample) in current.iter().enumerate() {
        if active[index] {
            active_peak = active_peak.max(sample.abs());
            upstream_peak = upstream_peak.max(source[index].abs());
        } else { outside_peak = outside_peak.max(sample.abs()); }
    }
    let mut required = if active_peak > ceiling { Some(vec![0.0; current.len()]) } else { None };
    if let Some(required) = &mut required {
        for (index, value) in required.iter_mut().enumerate() {
            if active[index] {
                // NumPy maximum with a float32 source evaluates its target in
                // float32. Promote THAT target before adding f64 slack. Never
                // cast target+tolerance back to float32 (PR15 regression).
                let target = source_dtype.store(ceiling.max(source[index].abs()));
                *value = (20.0 * (current[index].abs().max(1e-12) / target).log10()).max(0.0);
            }
        }
    }
    let applied = required.as_ref().is_some_and(|v| v.iter().any(|&x| x > 0.0));
    let mut report = json!({
        "applied": applied, "ceiling": ceiling,
        "method": "lookahead-scalar-envelope", "normalization": "none",
        "scope": "expression-active samples only; existing upstream peaks are permitted",
        "target": "max(ceiling, absolute upstream sample)",
        "peak_before": peak(current), "active_peak_before": active_peak,
        "outside_active_peak_before": outside_peak, "active_upstream_peak": upstream_peak,
        "lookahead_seconds": 0.0015,
        "boundary_taper": "existing expression blend; necessary peak attenuation is retained",
    });
    let gain = if applied {
        let required = required.context("headroom attenuation requires a detector track")?;
        let radius = seconds_to_samples(0.0015, sample_rate as f64).max(1);
        let mut gain = vec![1.0; current.len()];
        let mut index = 0;
        let mut maximum = 0.0_f64;
        let mut changed = 0;
        while index < active.len() {
            if !active[index] { index += 1; continue; }
            let begin = index;
            while index < active.len() && active[index] { index += 1; }
            let local = &required[begin..index];
            if !local.iter().any(|&v| v > 0.0) { continue; }
            let shaped = filters::smooth_majorant(local, radius)?;
            for offset in 0..local.len() {
                let at = begin + offset;
                let attenuation = local[offset].max(shaped[offset] * blend[at]);
                maximum = maximum.max(attenuation);
                gain[at] = (-attenuation * (std::f64::consts::LN_10 / 20.0)).exp();
                if gain[at] < 1.0 {
                    current[at] = current_dtype.store(current[at] * gain[at]);
                    changed += 1;
                }
            }
        }
        report["changed_samples"] = json!(changed);
        report["maximum_attenuation_db"] = json!(maximum);
        Some(gain)
    } else { None };
    let tolerance = current_dtype.tolerance();
    let mut active_after = 0.0_f64;
    let mut outside_after = 0.0_f64;
    for begin in (0..current.len()).step_by(BLOCK) {
        let end = (begin + BLOCK).min(current.len());
        for index in begin..end {
            if active[index] {
                let target: f64 = source_dtype.store(ceiling.max(source[index].abs()));
                let observed: f64 = current[index].abs();
                ensure!(observed <= target + tolerance,
                    "expression headroom failed to bound expression-created peaks");
                active_after = active_after.max(observed);
            } else {
                ensure!(current[index].to_bits() == source[index].to_bits(),
                    "expression headroom modified samples outside expression ownership");
                outside_after = outside_after.max(current[index].abs());
            }
        }
    }
    report["peak_after"] = json!(peak(current));
    report["active_peak_after"] = json!(active_after);
    report["outside_active_peak_after"] = json!(outside_after);
    report["outside_active_bit_exact"] = json!(true);
    Ok((report, gain))
}

#[cfg(test)]
mod tests;
