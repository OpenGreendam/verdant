//! Native run synthesis: source → filter compensation → one lattice state.
//!
//! Production order is intentionally split at the same boundaries as Python:
//! apply_spectral_options → synthesize_run (raw) → apply_phonetic_waveform
//! → calibrate_run (relative gain, one 0.65 reference calibration)
//! → apply_waveform_options (post-calibration model dynamics).
//! `calibration_reference_waveform` is private scratch, consumed by calibration.
//! Numeric/artifact names and storage dtypes retain the frozen PR15 contract.

pub mod excitation;
pub mod harmonic_balance;
pub mod lpc;
pub mod phonetic_source;
pub mod relative_gain;
pub mod rng;
pub(crate) mod numpy_math;

#[cfg(test)]
#[path = "dsp/tests.rs"]
mod tests;

use crate::core::{AudioResult, Arrays, NumericArray, RunParameters};
use anyhow::{ensure, Context, Result};
use num_complex::Complex64;
use serde_json::{json, Value};
use std::f64::consts::PI;

pub const CALIBRATION_REFERENCE: &str = "calibration_reference_waveform";

fn array<'a>(run: &'a RunParameters, key: &str, shape: &[usize]) -> Result<&'a [f64]> {
    let input = run.arrays.get(key).with_context(|| format!("missing synthesis array {key}"))?;
    let count = shape.iter().try_fold(1_usize, |a,&b| a.checked_mul(b))
        .context("synthesis array shape overflow")?;
    ensure!(input.shape == shape && input.values.len() == count
        && input.values.iter().all(|v| v.is_finite()),
        "synthesis array {key} must be finite with shape {shape:?}");
    Ok(&input.values)
}

fn option_number(options: &Value, key: &str, default: f64) -> Result<f64> {
    match options.get(key) {
        None => Ok(default),
        Some(value) => value.as_f64().filter(|v| v.is_finite())
            .with_context(|| format!("{key} must be a finite real number")),
    }
}

fn option_string<'a>(options: &'a Value, key: &str, default: &'a str) -> Result<&'a str> {
    match options.get(key) {
        None => Ok(default),
        Some(value) => value.as_str().with_context(|| format!("{key} must be a string")),
    }
}

fn native_grid(run: &RunParameters) -> Result<(u32, usize)> {
    let sample_rate = run.metadata.get("sample_rate").and_then(Value::as_u64)
        .filter(|&v| v > 0 && v <= u32::MAX as u64).context("run requires positive sample_rate metadata")? as u32;
    let hop = run.metadata.get("hop_samples").and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok()).filter(|&v| v > 0)
        .context("run requires positive hop_samples metadata")?;
    ensure!(run.frames > 0 && run.frames.checked_mul(hop).is_some(), "native run duration is invalid");
    let period = hop as f64/f64::from(sample_rate);
    ensure!(run.controls.period_seconds.is_finite() && run.controls.origin_seconds.is_finite()
        && (run.controls.period_seconds-period).abs() <= 4.0*f64::EPSILON*period.max(1.0),
        "run control period must match sample_rate/hop_samples");
    Ok((sample_rate,hop))
}

fn frame_center_value(values: &[f64], sample: usize, hop: usize) -> f64 {
    let x = sample as f64/hop as f64;
    if x <= 0.5 { return values[0]; }
    let last = values.len()-1;
    if x >= last as f64+0.5 { return values[last]; }
    let left = (x-0.5).floor() as usize;
    values[left]+(values[left+1]-values[left])*(x-(left as f64+0.5))
}

fn inverse_rms_response(lpc: &[f64], frequency_grid: &[Complex64; 1024]) -> Result<f64> {
    let mut powers = [0.0_f64; 1024];
    for (power, &z) in powers.iter_mut().zip(frequency_grid) {
        let mut denominator = Complex64::new(0.0,0.0);
        for &coefficient in lpc.iter().rev() { denominator = denominator*z+coefficient; }
        let response = (Complex64::new(1.0,0.0)/denominator).norm();
        *power = response*response;
    }
    let gain = 1.0/(numpy_math::pairwise_sum(&powers)/1024.0).sqrt();
    ensure!(gain.is_finite() && gain > 0.0, "nonfinite filter-energy equalization gain");
    Ok(gain)
}

fn outer_ramp(output: &mut [f64], ramp: usize) {
    if ramp == 0 { return; }
    let step = if ramp > 1 { 1.0/(ramp-1) as f64 } else { 0.0 };
    let count = output.len();
    for index in 0..ramp {
        let rising = if index+1 == ramp && ramp > 1 {1.0} else {index as f64*step};
        // np.linspace(1,0,n) multiplies by a NEGATIVE step, then adds 1;
        // it is not the reversed rising array at rounding-sensitive samples.
        let falling = if index+1 == ramp && ramp > 1 {0.0} else {1.0-index as f64*step};
        output[index] *= rising*rising;
        output[count-ramp+index] *= falling*falling;
    }
}

// Authored segment identities remain untouched. Only an explicit validated
// continuation marker may widen source-envelope/harmonic admission across a
// note boundary; equal phone IDs alone never authorize that grouping.
fn source_segment_groups<'a>(run: &RunParameters, ids: &[i64], segments: &'a [i64])
    -> Result<std::borrow::Cow<'a, [i64]>> {
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    let Some(markers) = run.metadata.get("source_continuation_links") else {
        return Ok(Cow::Borrowed(segments));
    };
    let markers = markers.as_array().context("source_continuation_links must be an array")?;
    if markers.is_empty() {return Ok(Cow::Borrowed(segments));}
    let mut links = BTreeMap::new();
    for marker in markers {
        let object = marker.as_object().context("source continuation link must be an object")?;
        ensure!(object.len() == 2 && object.contains_key("segment_index") && object.contains_key("continuation_of"),
            "source continuation link requires exactly segment_index and continuation_of");
        let segment = object["segment_index"].as_i64().filter(|&id|id >= 0)
            .context("continuation segment_index must be a nonnegative integer")?;
        let anchor = object["continuation_of"].as_i64().filter(|&id|id >= 0)
            .context("continuation_of must be a nonnegative original anchor index")?;
        ensure!(segment != anchor && links.insert(segment,anchor).is_none(),
            "continuation link must have a unique target and a distinct original anchor");
    }
    let lengths = array(run,"phonemic_length",&[run.frames])?;
    let mut blocks = Vec::new();
    let mut positions = BTreeMap::new();
    let mut first = 0;
    while first < segments.len() {
        let mut last = first+1;
        while last < segments.len() && segments[last] == segments[first] {last += 1;}
        let index = blocks.len();
        ensure!(positions.insert(segments[first],index).is_none(),
            "continuation requires each authored segment to occupy one contiguous block");
        blocks.push((segments[first],first,last));
        first = last;
    }
    ensure!(links.keys().all(|id|positions.contains_key(id)), "continuation target is outside this run");
    let mut grouped = segments.to_vec();
    for (index,&(segment,first,last)) in blocks.iter().enumerate() {
        let Some(&anchor) = links.get(&segment) else {continue;};
        let anchor_position = *positions.get(&anchor).context("continuation anchor is outside this run")?;
        ensure!(anchor_position < index && !links.contains_key(&anchor) && index > 0,
            "continuation must refer directly to an earlier original nucleus anchor");
        let (_,anchor_first,anchor_last) = blocks[anchor_position];
        let (_,previous_first,previous_last) = blocks[index-1];
        ensure!(previous_last == first && grouped[previous_first] == anchor,
            "continuation chain must be adjacent with no intervening source group");
        let phone = ids[anchor_first];
        ensure!(phonetic_source::phone_kind(phone)? == "vowel"
            && ids[anchor_first..anchor_last].iter().chain(&ids[first..last]).all(|&id|id == phone),
            "source continuation requires the same audited voiced-nucleus phone identity");
        let length = lengths[anchor_first];
        ensure!(length.is_finite() && length.fract() == 0.0 && length >= 1.0
            && lengths[anchor_first..anchor_last].iter().chain(&lengths[first..last]).all(|&v|v == length),
            "source continuation must preserve original phonemic length");
        let continuity = run.controls.continuity_identity[anchor_first];
        ensure!(continuity >= 0 && run.controls.continuity_identity[anchor_first..anchor_last].iter()
            .chain(&run.controls.continuity_identity[first..last]).all(|&id|id == continuity),
            "source continuation cannot cross an acoustic run or gap");
        grouped[first..last].fill(anchor);
    }
    Ok(Cow::Owned(grouped))
}

pub fn synthesize_run(run: &RunParameters, options: &Value, seed: u64) -> Result<AudioResult> {
    ensure!(options.is_object(), "synthesis options must be an object");
    let (sample_rate,hop) = native_grid(run)?;
    let frames = run.frames;
    let order = 24;
    let lsf = array(run,"lsf_radians",&[frames,order])?;
    let f0 = array(run,"f0_hz",&[frames])?;
    let voiced_array = array(run,"voiced",&[frames])?;
    let ids_array = array(run,"native_phone_id",&[frames])?;
    let segment_array = array(run,"segment_index",&[frames])?;
    ensure!(run.controls.f0_hz == f0 && run.controls.voiced.len() == frames
        && run.controls.phone_identity.len() == frames && run.controls.segment_identity.len() == frames
        && run.controls.phonetic_classes.len() == frames && run.controls.continuity_identity.len() == frames,
        "native control track must align with every run frame");
    let voiced = &run.controls.voiced;
    let ids = &run.controls.phone_identity;
    let segments = &run.controls.segment_identity;
    for i in 0..frames {
        ensure!(voiced_array[i] == (if voiced[i] {1.0} else {0.0})
            && ids_array[i] == ids[i] as f64 && segment_array[i] == segments[i] as f64,
            "control identities and native artifact arrays disagree at frame {i}");
        ensure!(f0[i].is_finite() && (!voiced[i] || f0[i] > 0.0), "voiced F0 must be finite and positive");
    }
    let source_groups = source_segment_groups(run,ids,segments)?;
    let source = option_string(options,"source","rosenberg")?;
    let profile = option_string(options,"source_profile","phonetic")?;
    ensure!(matches!(source,"rosenberg"|"impulse"), "source must be rosenberg or impulse");
    ensure!(matches!(profile,"phonetic"|"legacy"), "source_profile must be legacy or phonetic");
    let configured_gamma = option_number(options,"bandwidth_gamma",0.994)?;
    let gamma = run.metadata.get("output_bandwidth_gamma").or_else(||
        run.metadata.get("vocal_tract").and_then(|report|report.get("output_bandwidth_gamma")))
        .map(|value|value.as_f64().context("output_bandwidth_gamma must be a real number"))
        .transpose()?.unwrap_or(configured_gamma);
    ensure!(gamma.is_finite() && gamma > 0.0 && gamma <= 1.0,
        "bandwidth_gamma must be finite in (0,1]");
    let noise_level = option_number(options,"noise_level",0.012)?;
    ensure!(noise_level >= 0.0, "noise_level must be nonnegative");
    let brightness = option_number(options,"vowel_brightness_db",4.0)?;
    let balance_db = option_number(options,"vowel_balance_db",0.0)?;
    ensure!((0.0..=18.0).contains(&balance_db), "vowel_balance_db must be finite and within 0..18");
    ensure!(balance_db == 0.0 || profile == "phonetic", "vowel balance requires phonetic source");
    let samples = frames*hop;
    let mut lpc = Vec::with_capacity(frames*(order+1));
    let mut reflection = Vec::with_capacity(frames*order);
    let mut filter_gain = Vec::with_capacity(frames);
    let gamma_powers:Vec<f64> = (0..=order).map(|power|gamma.powf(power as f64)).collect();
    let mut max_radius = 0.0_f64;
    let frequency_grid = std::array::from_fn(|index|
        Complex64::from_polar(1.0,-(index as f64*(PI/1024.0))));
    for frame in 0..frames {
        let mut row = lpc::lsf_to_lpc(&lsf[frame*order..(frame+1)*order])?;
        for (coefficient,&expansion) in row.iter_mut().zip(&gamma_powers) {*coefficient *= expansion;}
        if frame > 0 && row == lpc[(frame-1)*(order+1)..frame*(order+1)] {
            reflection.extend_from_within((frame-1)*order..frame*order);
            filter_gain.push(filter_gain[frame-1]);
        } else {
            let radius = lpc::maximum_pole_radius(&row)?;
            ensure!(radius < 1.0, "unstable all-pole filter after LSF conversion");
            max_radius = max_radius.max(radius);
            filter_gain.push(inverse_rms_response(&row,&frequency_grid)?);
            reflection.extend(lpc::lpc_to_reflection(&row)?);
        }
        lpc.extend(row);
    }
    let controls = if profile == "phonetic" {
        let lengths = array(run,"phonemic_length",&[frames])?;
        ensure!(lengths.iter().all(|v| v.fract() == 0.0 && *v >= 1.0 && *v <= i64::MAX as f64),
            "phone lengths must be aligned positive integer values");
        let lengths:Vec<i64> = lengths.iter().map(|&v|v as i64).collect();
        Some(phonetic_source::source_controls(ids,&source_groups,voiced,sample_rate,hop,brightness,Some(&lengths))?)
    } else {None};
    let mut balance_info = None;
    let harmonic_gains = if balance_db > 0.0 {
        let model_f0 = array(run,"model_f0_hz_assuming_natural_log",&[frames])?;
        let source_controls = controls.as_ref().unwrap();
        let (gains,metadata) = harmonic_balance::vowel_harmonic_gains(
            &lpc,lsf,order,f0,model_f0,voiced,ids,&source_groups,sample_rate,hop,source,
            &source_controls.brightness_db,balance_db)?;
        balance_info = Some(metadata);
        Some(gains)
    } else {None};
    let source_brightness = controls.as_ref().map_or(&[0.0][..],|controls|controls.brightness_db.as_slice());
    let (mut periodic,excitation_info) = excitation::bandlimited_excitation(
        f0,voiced,sample_rate,hop,source,excitation::OVERSAMPLE,source_brightness,harmonic_gains.as_deref())?;
    let scale = excitation_info.get("periodic_rms_before_harmonic_balance").and_then(Value::as_f64)
        .unwrap_or_else(||numpy_math::voiced_rms(&periodic,voiced,hop));
    ensure!(scale.is_finite() && scale >= 0.0, "invalid periodic source RMS");
    let scale = scale.max(1e-12);
    for value in &mut periodic {*value /= scale;}
    let mut random = rng::NumpyPcg64::new(seed);
    let noise = random.standard_normal_vec(samples);
    // Detailed helper is the single-pass implementation of the canonical
    // options::consonant_noise_gain hook; retain its report without rerunning it.
    let mut dynamics = crate::options::consonant_noise_dynamics(run,options)?;
    let active = if let Some(dynamics) = &dynamics {
        ensure!(dynamics.sample_gain.len() == samples && dynamics.sample_eligible.len() == samples
            && dynamics.sample_gain.iter().all(|v| v.is_finite() && *v > 0.0 && *v <= 1.0),
            "consonant noise multiplier must match the complete sample grid and remain in (0,1]");
        dynamics.sample_gain.iter().any(|v| *v != 1.0)
    } else {false};
    let mut reference_excitation = active.then(||Vec::with_capacity(samples));
    let mut excitation = Vec::with_capacity(samples);
    for i in 0..samples {
        let (periodic_part,old_noise_gain,old) = if let Some(controls) = &controls {
            let periodic_part = periodic[i]*controls.periodic_gain[i];
            let gain = controls.noise_gain[i];
            (periodic_part,gain,periodic_part+noise[i]*gain)
        } else if voiced[i/hop] {
            (periodic[i],noise_level,periodic[i]+noise_level*noise[i])
        } else {(0.0,1.0,noise[i])};
        let gain = frame_center_value(&filter_gain,i,hop);
        if let Some(reference) = &mut reference_excitation {
            reference.push(old*gain);
            let multiplier = dynamics.as_ref().unwrap().sample_gain[i];
            let new_noise_gain = if multiplier == 1.0 {old_noise_gain} else {old_noise_gain*multiplier};
            // Protect the unchanged source branch exactly; active UV changes
            // share the same random sample, source phase and compensation.
            excitation.push(if multiplier == 1.0 {old*gain}
                else {(periodic_part+noise[i]*new_noise_gain)*gain});
        } else {excitation.push(old*gain);}
    }
    let mut output = lpc::lattice_filter(&excitation,&reflection,frames,order,hop)?;
    let mut reference = reference_excitation.as_ref().map(|input|
        lpc::lattice_filter(input,&reflection,frames,order,hop)).transpose()?;
    let ramp = (0.012*f64::from(sample_rate)).round_ties_even() as usize;
    let ramp = ramp.min(samples/2);
    outer_ramp(&mut output,ramp);
    if let Some(reference) = &mut reference {outer_ramp(reference,ramp);}
    ensure!(output.iter().all(|v|v.is_finite()) && reference.as_ref()
        .is_none_or(|waveform|waveform.iter().all(|v|v.is_finite())), "nonfinite synthesis waveform");
    let max_reflection = reflection.iter().map(|v|v.abs()).fold(0.0,f64::max);
    let mut dsp = json!({
        "source":source,"original_pulse_table_used":false,"source_profile":profile,
        "phonetic_source_controls":controls.as_ref().map(|controls|&controls.metadata),
        "source_status":"traditional approximate excitation, original excitation not decoded",
        "excitation":excitation_info,"seed":seed,"bandwidth_expansion_gamma":gamma,
        "maximum_frame_filter_pole_radius":max_radius,
        "filter_energy_equalization":"inverse RMS response over 1024 frequency bins",
        "unidentified_scalar_gain_used":false,"voiced_noise_level":if controls.is_none(){Some(noise_level)}else{None},
        "noise_gain_policy":if controls.is_none(){"uniform legacy V/UV switch"}
            else{"phone-conditioned per-sample gains recorded in phonetic_source_controls"},
        "rosenberg_open_fraction":if source == "rosenberg"{Some(0.45)}else{None},
        "rosenberg_close_fraction":if source == "rosenberg"{Some(0.20)}else{None},
        "sample_filter_interpolation":"linear PARCOR at frame centers, held at endpoints; all-pole synthesis lattice",
        "control_frame_time":"(frame_index + .5) * hop_samples / sample_rate",
        "sample_f0_policy":"frame-held; continuous phase accumulation without note-step smoothing",
        "state_scope":"one call per contiguous run; RNG, phase and lattice state persist across internal segment boundaries",
        "maximum_absolute_reflection_coefficient":max_reflection,
        "interpolated_filter_stability":"each interpolated PARCOR remains strictly within (-1,1); actual waveform checked finite",
        "endpoint_ramp_seconds":ramp as f64/f64::from(sample_rate)
    });
    if let Some(links) = run.metadata.get("source_continuation_links").filter(|links|links.as_array().is_some_and(|links|!links.is_empty())) {
        dsp["source_continuation_links"] = links.clone();
        dsp["source_continuation_policy"] = json!("explicit original nucleus anchor groups only source envelopes and harmonic-boundary gates; authored segment indices, note F0/DYN ownership, run-wide RNG/phase/lattice remain intact");
    }
    if let Some(mut report) = balance_info {
        report["source_rms_policy"] = json!("retain the unmodified periodic source's voiced-sample RMS; no cross-vowel renormalization from added partials");
        dsp["vowel_harmonic_balance"] = report;
    }
    if let Some(dynamics) = &mut dynamics {
        let report = &mut dynamics.report;
        report["adapter_position"] = json!("noise envelope before inverse-RMS filter compensation and the time-varying synthesis lattice");
        report["calibration_reference_available"] = json!(reference.is_some());
        report["extra_lattice_passes"] = json!(usize::from(reference.is_some()));
        report["noise_samples_and_periodic_source_reused"] = json!(true);
        report["reference_raw_peak"] = json!(peak(reference.as_deref().unwrap_or(&output)));
        report["candidate_raw_peak"] = json!(peak(&output));
        report["filter_memory_scope"] = json!("continuous within the run; candidate noise can ring into neighboring phones");
        report["release_and_closure_times_modified"] = json!(false);
        dsp["consonant_dynamics"] = report.clone();
    }
    for key in ["phonetic_correction","vocal_tract"] {
        if let Some(report) = run.metadata.get(key) {dsp[key] = report.clone();}
    }
    let mut metadata = run.metadata.clone();
    ensure!(metadata.is_object(), "run metadata must be an object");
    metadata["dsp"] = dsp;
    metadata["seed"] = json!(seed);
    metadata["frame_count"] = json!(frames);
    metadata["actual_seconds"] = json!(samples as f64/f64::from(sample_rate));
    let mut arrays = Arrays::new();
    arrays.insert("lpc_coefficients".into(),NumericArray::new(vec![frames,order+1],"float64",lpc)?);
    arrays.insert("parcor_coefficients".into(),NumericArray::new(vec![frames,order],"float64",reflection)?);
    if let Some(reference) = reference {
        arrays.insert(CALIBRATION_REFERENCE.into(),NumericArray::new(vec![samples],"float64",reference)?);
    }
    Ok(AudioResult {sample_rate,samples:output,source_dtype:"float64".into(),arrays,metadata})
}

fn peak(values: &[f64]) -> f64 {values.iter().map(|v|v.abs()).fold(0.0,f64::max)}

/// Run exactly once, after contact correction and before model dynamics.
/// Candidate and unchanged-source reference receive the same legacy relative
/// gain. Only the reference determines normalization when consonants changed.
pub fn calibrate_run(audio: &mut AudioResult, run: &RunParameters, options: &Value) -> Result<()> {
    let (sample_rate,hop) = native_grid(run)?;
    ensure!(audio.sample_rate == sample_rate && run.frames.checked_mul(hop) == Some(audio.samples.len())
        && audio.samples.iter().all(|v|v.is_finite()), "calibration requires one finite complete native run");
    ensure!(audio.metadata.get("run_peak_calibration_gain").is_none(), "run has already been calibrated");
    let log_gain = array(run,"generated_log_gain",&[run.frames])?;
    let mode = option_string(options,"gain_mode","off")?;
    if mode == "relative" {
        ensure!(options.get("model_dynamics").is_none_or(Value::is_null)
            && options.get("consonant_dynamics").is_none_or(Value::is_null),
            "model_dynamics/consonant_dynamics cannot be combined with legacy gain_mode='relative'");
    }
    let gain_info = relative_gain::apply_relative_model_gain(&mut audio.samples,log_gain,hop,mode,0.25,6.0)?;
    let candidate_peak = peak(&audio.samples);
    let reference_peak = if let Some(reference) = audio.arrays.get_mut(CALIBRATION_REFERENCE) {
        ensure!(reference.shape == [audio.samples.len()] && reference.values.len() == audio.samples.len()
            && reference.dtype == "float64", "invalid unchanged-source calibration reference");
        relative_gain::apply_relative_model_gain(&mut reference.values,log_gain,hop,mode,0.25,6.0)?;
        peak(&reference.values)
    } else {candidate_peak};
    let gain = 0.65/reference_peak.max(1e-12);
    for value in &mut audio.samples {*value *= gain;}
    ensure!(audio.samples.iter().all(|v|v.is_finite()), "nonfinite calibrated waveform");
    audio.metadata["relative_model_gain"] = gain_info;
    audio.metadata["run_peak_calibration_gain"] = json!(gain);
    audio.metadata["gain_policy"] = json!("single peak calibration to 0.65 over the whole contiguous run; original exp/scale identified; original excitation/filter absolute calibration unresolved");
    let candidate_after = peak(&audio.samples);
    if let Some(report) = audio.metadata.get_mut("dsp").and_then(|dsp|dsp.get_mut("consonant_dynamics")) {
        report["level_calibration"] = json!("0.65 divided by the unchanged-source reference peak after identical phonetic correction; candidate never renormalized");
        report["calibration_reference_peak"] = json!(reference_peak);
        report["candidate_peak_before_calibration"] = json!(candidate_peak);
        report["applied_calibration_gain"] = json!(gain);
        report["candidate_peak_after_calibration"] = json!(candidate_after);
        report["phonetic_contact_decisions"] = json!("unchanged-source reference, once; identical gain applied to candidate");
        report["frame_parameters_modified"] = json!(false);
        report["score_pitch_modified"] = json!(false);
        report["timing_modified"] = json!(false);
        audio.metadata["consonant_dynamics"] = report.clone();
        audio.metadata["gain_policy"] = json!("single peak calibration to 0.65 over the whole contiguous run; original exp/scale identified; original excitation/filter absolute calibration unresolved; optional unvoiced noise-envelope candidate uses the unchanged-source reference calibration");
    }
    audio.arrays.remove(CALIBRATION_REFERENCE);
    Ok(())
}
