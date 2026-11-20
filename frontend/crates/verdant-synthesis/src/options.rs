//! Production acoustic options. The worker calls these stages in this order:
//! spectral options, raw DSP, phonetic waveform contact, run calibration,
//! then waveform options. Controls and generated model observations are read-only.

mod common;
mod dynamics;
mod phonetic;
mod tract;

pub use dynamics::{
    apply_model_dynamics, condition_noise_envelope, consonant_dynamics_capabilities,
    model_dynamics_capabilities, validate_consonant_dynamics, validate_model_dynamics,
    DynamicsResult, NoiseDynamics,
};
pub use phonetic::{
    correct_spectrum, correct_waveform, validate_phonetic_correction,
};
pub use tract::{
    transform_lsf, validate_vocal_tract, vocal_tract_capabilities, warp_frequencies,
    warp_log_envelope, warp_lpc_envelope, VocalTractConfiguration,
};

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use crate::core::{AudioResult, NumericArray, RunParameters};
use common::{array, object_mut, sample_grid};

/// Validate all four options before any expensive numerical work. Explicit zero
/// dynamics still conflicts with the old relative-gain mode, as in production.
pub fn validate_options(options: &Value) -> Result<()> {
    ensure!(options.is_object(), "synthesis options must be an object");
    validate_phonetic_correction(&options["phonetic_correction"])?;
    validate_vocal_tract(&options["vocal_tract"])?;
    let model = validate_model_dynamics(&options["model_dynamics"])?;
    let consonant = validate_consonant_dynamics(&options["consonant_dynamics"])?;
    ensure!(options["gain_mode"] != "relative" || (model.is_none() && consonant.is_none()),
        "model_dynamics and consonant_dynamics cannot be combined with legacy gain_mode='relative'");
    Ok(())
}

/// Correct explicitly traced source phones, then warp the actual LPC envelope.
/// The effective filter bandwidth is in metadata["output_bandwidth_gamma"].
pub fn apply_spectral_options(run: &mut RunParameters, options: &Value) -> Result<()> {
    validate_options(options)?;
    let correction = validate_phonetic_correction(&options["phonetic_correction"])?;
    let tract = validate_vocal_tract(&options["vocal_tract"])?;
    let correction_active = correction.as_ref().is_some_and(|v| v["strength"].as_f64().unwrap_or(0.) != 0.);
    if !correction_active && tract.is_none() {
        return Ok(());
    }
    let (sample_rate, hop) = sample_grid(run)?;
    let gamma = match options.get("bandwidth_gamma") {
        None => 0.994,
        Some(value) => common::bounded_number(value, 0., 1., "bandwidth_gamma")?,
    };
    ensure!(gamma > 0., "bandwidth_gamma must be finite in (0,1]");
    let source = array(run, "lsf_radians")?;
    ensure!(source.shape.len() == 2 && source.shape[0] == run.frames,
        "lsf_radians must align with the complete run");
    let mut output = source.clone();
    let mut correction_report = None;
    if correction_active {
        let segments = run.metadata["segments"].as_array().context("phonetic correction requires original segments")?;
        let counts = run.metadata["frame_counts"].as_array().context("phonetic correction requires frame_counts")?;
        let counts: Vec<usize> = counts.iter().map(|value| {
            let count = value.as_u64().context("frame_counts must be positive integers")?;
            ensure!(count > 0, "frame_counts must be positive integers");
            usize::try_from(count).context("frame count exceeds address space")
        }).collect::<Result<_>>()?;
        let keys = run.metadata["native_keys"].as_array().context("phonetic correction requires actual native_keys")?;
        let (corrected, report) = correct_spectrum(&output, segments, &counts, keys,
            sample_rate, hop, &options["phonetic_correction"], gamma)?;
        if let Some(report) = report {
            run.arrays.insert("lsf_radians_before_phonetic_correction".into(), output);
            output = corrected;
            correction_report = Some(report);
        }
    }
    let mut tract_report = None;
    if tract.is_some() {
        let (warped, report) = transform_lsf(&output, &options["vocal_tract"], sample_rate as f64, gamma)?;
        if let Some(mut report) = report {
            report["adapter_position"] = json!("after phonetic spectral correction, before LPC/excitation synthesis");
            report["gain_policy"] = json!("existing inverse-RMS filter energy equalization and contiguous-run peak calibration retained; AR prediction-error scalar gain intentionally not added; absolute envelope amplitude is not preserved");
            run.arrays.insert("lsf_radians_before_vocal_tract".into(), output);
            output = warped;
            tract_report = Some(report);
        }
    }
    let effective_gamma = tract_report.as_ref().and_then(|r| r["output_bandwidth_gamma"].as_f64()).unwrap_or(gamma);
    run.arrays.insert("lsf_radians".into(), output);
    let metadata = object_mut(&mut run.metadata, "run metadata")?;
    metadata.insert("output_bandwidth_gamma".into(), json!(effective_gamma));
    if let Some(report) = correction_report { metadata.insert("phonetic_correction".into(), report); }
    if let Some(report) = tract_report { metadata.insert("vocal_tract".into(), report); }
    Ok(())
}

/// A single prefilter conditioning pass, with the report available to DSP.
/// The gains are multiplicative; the original source envelope stays in DSP.
pub fn consonant_noise_dynamics(run: &RunParameters, options: &Value) -> Result<Option<NoiseDynamics>> {
    validate_options(options)?;
    if options["consonant_dynamics"].is_null() { return Ok(None); }
    let (sample_rate, hop) = sample_grid(run)?;
    let samples = run.frames.checked_mul(hop).context("noise sample count overflow")?;
    let logs = array(run, "generated_log_gain")?;
    ensure!(logs.shape == [run.frames], "generated_log_gain must be a frame vector");
    dynamics::noise_dynamics(samples, &logs.values, &run.controls,
        &options["consonant_dynamics"], sample_rate as f64)
}

pub fn consonant_noise_gain(run: &RunParameters, options: &Value) -> Result<Option<Vec<f64>>> {
    Ok(consonant_noise_dynamics(run, options)?.map(|result| result.sample_gain))
}

/// Apply the contact detector once to the unchanged-source reference if one
/// exists, otherwise to the candidate. Both paths receive the same gain.
/// This stage MUST precede dsp::calibrate_run.
pub fn apply_phonetic_waveform(audio: &mut AudioResult, run: &RunParameters) -> Result<()> {
    let Some(report) = run.metadata.get("phonetic_correction").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let mut report = report.clone();
    let (sample_rate, hop) = sample_grid(run)?;
    ensure!(audio.sample_rate == sample_rate, "phonetic waveform sample rate differs from run");
    let reference = audio.arrays.get("calibration_reference_waveform");
    if let Some(reference) = reference {
        ensure!(reference.shape == [audio.samples.len()], "invalid calibration reference shape");
    }
    let observed = reference.map(|r| r.values.as_slice()).unwrap_or(&audio.samples);
    let gain = correct_waveform(observed, &run.controls.f0_hz, &run.controls.voiced,
        &mut report, sample_rate, hop)?;
    ensure!(gain.len() == audio.samples.len(), "contact gain duration differs from audio");
    for (sample, &gain) in audio.samples.iter_mut().zip(&gain) {
        if gain != 1. { *sample *= gain; }
    }
    if let Some(reference) = audio.arrays.get_mut("calibration_reference_waveform") {
        for (sample, &gain) in reference.values.iter_mut().zip(&gain) {
            if gain != 1. { *sample *= gain; }
        }
    }
    let means = gain.chunks_exact(hop).map(|row| row.iter().sum::<f64>() / hop as f64).collect();
    let minima = gain.chunks_exact(hop).map(|row| row.iter().copied().fold(f64::INFINITY, f64::min)).collect();
    audio.arrays.insert("phonetic_correction_gain_frame_mean".into(), NumericArray::new(vec![run.frames], "float64", means)?);
    audio.arrays.insert("phonetic_correction_gain_frame_min".into(), NumericArray::new(vec![run.frames], "float64", minima)?);
    let metadata = object_mut(&mut audio.metadata, "audio metadata")?;
    let dsp = metadata.entry("dsp").or_insert_with(|| json!({}));
    object_mut(dsp, "DSP metadata")?.insert("phonetic_correction".into(), report);
    Ok(())
}

/// Post-calibration attenuation only. This never calibrates or normalizes audio.
pub fn apply_waveform_options(audio: &mut AudioResult, run: &RunParameters, options: &Value) -> Result<()> {
    validate_options(options)?;
    if options["model_dynamics"].is_null() { return Ok(()); }
    let logs = array(run, "generated_log_gain")?;
    ensure!(logs.shape == [run.frames], "generated_log_gain must be a frame vector");
    if let Some(mut result) = apply_model_dynamics(&audio.samples, &logs.values, &run.controls,
        &options["model_dynamics"], audio.sample_rate as f64)? {
        common::round_storage(&mut result.samples, &result.sample_gain, &audio.source_dtype)?;
        let output_peak = result.samples.iter().map(|value| value.abs()).fold(0., f64::max);
        let input_peak = result.report["input_peak"].as_f64().context("missing model dynamics input peak")?;
        ensure!(result.samples.iter().all(|value| value.is_finite()) && output_peak <= input_peak,
            "model dynamics storage conversion violated its attenuation-only contract");
        result.report["output_peak"] = json!(output_peak);
        result.report["peak_did_not_increase"] = json!(true);
        audio.samples = result.samples;
        result.report["adapter_position"] = json!("after existing contiguous-run peak calibration; no subsequent normalization");
        result.report["phonetic_class_source"] = json!("Native inventory via explicit adapter controls; no inferred within-phone acoustic boundaries");
        result.report["compound_final_scope"] = json!("compound finals remain one inventory class; no separate nasal-tail or HMM-state phonetic interpretation");
        result.report["sample_gain_array"] = json!("model_dynamics_sample_gain");
        result.report["sample_eligible_array"] = json!("model_dynamics_sample_eligible");
        result.report["score_pitch_modified"] = json!(false);
        result.report["timing_modified"] = json!(false);
        let n = result.sample_gain.len();
        audio.arrays.insert("model_dynamics_sample_gain".into(), NumericArray::new(vec![n], "float64", result.sample_gain)?);
        audio.arrays.insert("model_dynamics_sample_eligible".into(), NumericArray::new(vec![n], "bool", result.sample_eligible.into_iter().map(|v| if v { 1. } else { 0. }).collect())?);
        object_mut(&mut audio.metadata, "audio metadata")?.insert("model_dynamics".into(), result.report);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
