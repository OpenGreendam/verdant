//! Production score-to-acoustic planning port of the pinned PR15 oracle.
//!
//! One cumulative score clock fixes phone/gap budgets; the selected unchanged
//! model records allocate five-state durations. Each real contiguous run has
//! one spectral MLPG and one compact voiced pitch MLPG. Authored curves are
//! applied after model pitch and intersected with native UV. DSP, calibration,
//! acoustic options and artifact publication are separate owning stages.
//! Explicit continuation notes share one selected five-state source phone;
//! its one duration allocation is sliced into distinct authored note segments.
//! No inferred link merges ordinary equal-phone neighbors or note ownership.
mod timing;
mod duration;
mod mlpg;
mod voicing;
mod control;
mod source;
mod native_reference;

pub use timing::{validate_score_continuity, validate_score_phoneme_timing, plan_score_timing,
    SegmentTiming, TimingPlan, CONTINUITY_MAX_SECONDS, CONTINUITY_MAX_SEGMENTS,
    CONTINUITY_MAX_CONTROL_POINTS};
pub use duration::{allocate_state_durations, allocate_singing_state_durations};
pub use mlpg::{generate_independent_trajectory, original_lsf_correction, pitch_mlpg,
    spectral_mlpg, stabilize_lsf};
pub use voicing::infer_voicing;
pub use source::{continuation_source_indices,validate_score_continuations};
pub use native_reference::POLICY_ID as TERMINAL_NUCLEUS_REFERENCE_POLICY;
pub use control::{plan_note_pitch, sample_authored_control, NATIVE_F0_MIN_HZ, NATIVE_F0_MAX_HZ,
    NATIVE_F0_RANGE};

use anyhow::{bail,ensure,Context,Result};
use serde_json::{json,Value};
use crate::core::{AcousticControlTrack,Arrays,NativePlan,NumericArray,RunParameters,SelectedPhone};
use crate::model::{compact_hierarchy_report,prepare_score_segments,VoiceModel};
use timing::{field,finite};

const SCORE_SYNTHESIS_FIELDS: &[&str] = &["duration_mode","source_profile","prosody_mode",
    "vowel_brightness_db","vowel_balance_db","gain_mode","phonetic_correction"];

struct Options {
    value: Value, hop: usize, duration: String, profile: String, prosody: String,
    acoustic: String, voicing: String, selection: String, policy: String,
    neighbor: String, first_child_truth: bool, compact_phone_states: bool, pitch_range: [f64; 2],
    native_reference: bool,
}
impl Options {
    fn new(score: &Value, explicit: &Value) -> Result<Self> {
        let mut merged = serde_json::Map::new();
        if let Some(synthesis) = score.get("synthesis") {
            let object = synthesis.as_object().context("score synthesis options must be an object")?;
            ensure!(object.keys().all(|key| SCORE_SYNTHESIS_FIELDS.contains(&key.as_str())),
                "unknown native synthesis options");
            for (key,value) in object { merged.insert(key.clone(),value.clone()); }
        }
        for (key,value) in explicit.as_object().context("synthesis options must be an object")? {
            merged.insert(key.clone(),value.clone());
        }
        let value = Value::Object(merged);
        let hop = match value.get("hop_samples") {
            Some(v) => v.as_u64().and_then(|n| usize::try_from(n).ok()).filter(|&n| n > 0)
                .context("hop_samples must be a positive integer")?,
            None => 80,
        };
        let duration = choice(&value,"duration_mode","singing",&["singing","model"])?;
        let profile = choice(&value,"source_profile","phonetic",&["phonetic","legacy"])?;
        let prosody = choice(&value,"prosody_mode","auto",&["auto","step"])?;
        let acoustic = choice(&value,"acoustic_mode","original",&["original","joint-reconstruction"])?;
        let voicing = choice(&value,"voicing_mode","authored",&["authored","original-model"])?;
        let selection = choice(&value,"selection_mode","coherent",&["coherent","independent-ablation"])?;
        let policy = choice(&value,"policy","diagnostic-medoid",&["strict","diagnostic-medoid"])?;
        let neighbor = choice(&value,"neighbor_mode","score-adjacent",&["score-adjacent","unspecified-ablation"])?;
        let gain = choice(&value,"gain_mode","off",&["off","relative"])?;
        let brightness = field(&value,"vowel_brightness_db",Some(4.0))?;
        let balance = field(&value,"vowel_balance_db",Some(0.0))?;
        ensure!((0.0..=8.0).contains(&brightness), "vowel brightness must be within 0..8 dB");
        ensure!((0.0..=18.0).contains(&balance), "vowel_balance_db must be within 0..18");
        if gain == "relative" {
            for name in ["model_dynamics","consonant_dynamics"] {
                ensure!(!value.get(name).is_some_and(|v| !v.is_null()),
                    "{name} cannot be combined with legacy gain_mode='relative'");
            }
        }
        let first_child_truth = match value.get("first_child_truth") {
            Some(v) => v.as_bool().context("first_child_truth must be boolean")?, None => false,
        };
        Ok(Self { value,hop,duration,profile,prosody,acoustic,voicing,selection,policy,neighbor,first_child_truth,compact_phone_states:false,pitch_range:NATIVE_F0_RANGE,native_reference:false })
    }
}
fn choice(value: &Value, key: &str, default: &str, choices: &[&str]) -> Result<String> {
    let name = match value.get(key) { Some(v) => v.as_str().with_context(|| format!("{key} must be a string"))?, None => default };
    ensure!(choices.contains(&name), "unsupported {key}: {name}");
    Ok(name.to_owned())
}
fn selected_array<'a>(phone: &'a SelectedPhone, name: &str, shape: &[usize]) -> Result<&'a [f64]> {
    let value = phone.arrays.get(name).with_context(|| format!("selected phone is missing {name}"))?;
    let count = shape.iter().try_fold(1usize,|n,&d| n.checked_mul(d)).context("selected model shape overflow")?;
    ensure!(value.shape == shape && value.values.len() == count && value.dtype == "float64"
        && value.values.iter().all(|v| v.is_finite()),
        "selected {name} must be finite float64 with shape {shape:?}");
    Ok(&value.values)
}
fn metadata_integer(metadata: &Value, name: &str) -> Result<Option<i64>> {
    match metadata.get(name) {
        None|Some(Value::Null) => Ok(None),
        Some(v) => Ok(Some(v.as_i64().with_context(|| format!("selected {name} must be an integer or null"))?)),
    }
}
fn metadata_key(metadata: &Value) -> Result<Option<String>> {
    match metadata.get("native_key") {
        None|Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(value.as_str().context("selected native_key must be a string or null")?.to_owned())),
    }
}
struct PreparedSource {
    frames: usize, state_frames: Vec<usize>, selected: SelectedPhone,
    phone_id: Option<i64>, language: Option<i64>, native_key: Option<String>,
    phonetic_class: String, phonemic_length: i64, authored_voiced: bool, details: Value,
}
fn prepare_phone(voice: &VoiceModel, segment: &Value, frames: usize, options: &Options,
    sample_rate: u32) -> Result<PreparedSource>
{
    if options.compact_phone_states {
        ensure!(frames >= 1, "planned segment must contain at least one 5 ms integer frame");
    } else {
        ensure!(frames >= 5, "planned segment must contain at least five integer frames");
    }
    let mut context = segment.get("context").cloned().context("prepared segment is missing explicit context")?;
    let envelope = context.as_object_mut().context("prepared phone context must be an object")?;
    envelope.insert("policy".into(),json!(options.policy));
    envelope.insert("selection_mode".into(),json!(options.selection));
    envelope.insert("first_child_truth".into(),json!(options.first_child_truth));
    let selected = voice.select_phone(&context)?;
    let spectral = selected_array(&selected,"spectral_means",&[5,3,41])?;
    let variances = selected_array(&selected,"spectral_variances",&[5,3,41])?;
    selected_array(&selected,"pitch_triplets",&[5,3,3])?;
    let duration_means = selected_array(&selected,"state_duration_means",&[5])?;
    let duration_variances = selected_array(&selected,"state_duration_variances",&[5])?;
    let phone_duration = selected_array(&selected,"phone_duration",&[2])?;
    ensure!(variances.iter().all(|&v| v > 0.0), "spectral variances must be strictly positive");
    // Pitch MLPG validates the selected voiced subset, matching Python; an
    // unused UV state's pitch variance is not silently substituted or fitted.
    let phone_id = metadata_integer(&selected.metadata,"phone_id")?;
    let language = metadata_integer(&selected.metadata,"language")?;
    let native_key = metadata_key(&selected.metadata)?;
    if options.profile == "phonetic" { ensure!(phone_id.is_some(), "phonetic source requires a known native current phone for every segment"); }
    let phonetic_class = phone_kind(phone_id.unwrap_or(-1),native_key.as_deref())?.to_owned();
    let allocate = if options.compact_phone_states { duration::allocate_voice_state_durations }
        else { allocate_singing_state_durations };
    let (state_frames,articulation) = allocate(frames,duration_means,duration_variances,
        native_key.as_deref(),phone_id,spectral,&options.duration,options.hop as f64/sample_rate as f64)?;
    let authored_voiced = match segment.get("voiced") {
        Some(value) => value.as_bool().context("segment voiced must be boolean")?, None => true,
    };
    let phonemic_length = source::phonemic_length(segment,native_key.as_deref())?;
    let mut details = selected.metadata.clone();
    let object = details.as_object_mut().context("selected phone audit must be an object")?;
    let label = segment.get("label").or_else(|| segment.get("vowel"))
        .or_else(|| selected.metadata.get("context").and_then(|c| c.get("label"))).cloned().unwrap_or(Value::Null);
    let extra = json!({"label":label,"request":segment,
        "requested_seconds":field(segment,"duration_seconds",Some(0.8))?,
        "actual_seconds":frames as f64*options.hop as f64/sample_rate as f64,
        "state_frames":state_frames,"state_duration_mean_interpretation":duration_means,
        "state_duration_variance_interpretation":duration_variances,
        "state_duration_allocation":{"mode":options.duration,"counts":state_frames,"count_sum":frames,
            "base_model_algorithm":duration::allocation_metadata(),"final_policy":articulation},
        "singing_state_duration":articulation,"phone_duration_record_unused_in_timing":phone_duration,
        "voicing_mode":"explicit segment voicing, original probability retained in NPZ"});
    for (key,value) in extra.as_object().context("invalid phone audit")? { object.insert(key.clone(),value.clone()); }
    if frames < 5 && options.compact_phone_states {
        object.get_mut("state_duration_allocation").context("missing allocation audit")?["base_model_algorithm"] = json!({
            "algorithm":"compact_monotonic_representative_states",
            "constraint":"five nonnegative integer occupancies sum exactly to the requested one-to-four-frame budget",
            "reference_only":duration::allocation_metadata()});
    }
    Ok(PreparedSource { frames,state_frames,selected,phone_id,language,native_key,
        phonetic_class,phonemic_length,authored_voiced,details })
}

struct PreparedSegment {
    frames: usize,
    source: usize,
    details: Value,
}

fn prepare_run_sources(voice: &VoiceModel, segments: &[Value], timing: &TimingPlan,
    source_indices: &[usize], first: usize, end: usize, options: &Options)
    -> Result<(Vec<PreparedSource>,Vec<PreparedSegment>,Vec<Value>)>
{
    let mut sources = Vec::new();
    let mut prepared = Vec::with_capacity(end-first);
    let mut links = Vec::new();
    let mut anchor = first;
    while anchor < end {
        ensure!(source_indices[anchor] == anchor,
            "a continuation cannot begin a different acoustic run");
        let mut after = anchor+1;
        while after < end && source_indices[after] == anchor { after += 1; }
        let total = timing.segments[after-1].end_frame-timing.segments[anchor].start_frame;
        let mut source = prepare_phone(voice,&segments[anchor],total,options,voice.sample_rate)
            .with_context(|| format!("source nucleus segment {anchor}"))?;
        if options.native_reference {
            let authored_language = match segments[anchor].pointer("/phoneme/source_language").and_then(Value::as_str) {
                Some("zh")=>1, Some("en")=>2, Some("ja")=>3, Some(_)=>0,
                None=>match source.language {Some(1)=>1,Some(3)=>2,_=>0},
            };
            native_reference::protect_terminal_nucleus(&mut source,authored_language,
                after-anchor,timing.segments[after-1].frames())?;
        }
        let source_number = sources.len();
        let source_counts = &source.state_frames;
        let source_context_frames = source_counts.iter().try_fold(0usize,|sum,&value|sum.checked_add(value))
            .context("native source context frame overflow")?;
        ensure!(source_context_frames>=total,"native source context is shorter than its emitted clock");
        let mut offset = 0;
        for index in anchor..after {
            let frames = timing.segments[index].frames();
            let mut details = if index+1 == after { std::mem::take(&mut source.details) }
                else { source.details.clone() };
            if after > anchor+1 {
                let occupancy = if options.compact_phone_states {
                    source::state_occupancy_with_compact(source_counts,offset,frames,true)?
                } else { source::state_occupancy(source_counts,offset,frames)? };
                details["label"] = segments[index].get("label").or_else(|| segments[index].get("vowel"))
                    .cloned().unwrap_or_else(|| details["label"].clone());
                details["request"] = segments[index].clone();
                details["requested_seconds"] = json!(field(&segments[index],"duration_seconds",None)?);
                details["actual_seconds"] = json!(frames as f64*options.hop as f64/voice.sample_rate as f64);
                details["state_frames"] = json!(occupancy);
                details["state_duration_allocation"]["counts"] = json!(occupancy);
                details["state_duration_allocation"]["count_sum"] = json!(frames);
                details["state_duration_allocation"]["scope"] = json!("one selected five-state acoustic phone across the explicit nucleus continuation chain; counts here are per-note state occupancy");
                details["state_duration_allocation"]["source_counts"] = json!(source_counts);
                details["state_duration_allocation"]["source_total_frames"] = json!(source_context_frames);
                details["singing_state_duration"]["scope"] = json!("entire explicit source-phone continuation chain; not a new per-note phone allocation");
                details["acoustic_source_segment_index"] = json!(anchor);
                details["acoustic_source_frame_offset"] = json!(offset);
                details["acoustic_source_frame_count"] = json!(source_context_frames);
                if source_context_frames!=total {
                    details["acoustic_source_emitted_frame_count"] = json!(total);
                }
                details["selection_scope"] = json!("original nucleus selected once; continuation preserves that source's model records and state sequence");
                if index != anchor {
                    ensure!(source.phone_id.is_some_and(duration::is_nucleus) && source.authored_voiced,
                        "continuation source must be an audited voiced native nucleus");
                    ensure!(source::phonemic_length(&segments[index],source.native_key.as_deref())? == source.phonemic_length,
                        "continuation cannot change the source phonemic length");
                    links.push(json!({"segment_index":index,"continuation_of":anchor}));
                }
            }
            prepared.push(PreparedSegment { frames,source:source_number,details });
            offset += frames;
        }
        ensure!(offset == source.frames, "continuation slices changed their source frame budget");
        sources.push(source);
        anchor = after;
    }
    Ok((sources,prepared,links))
}

/// Full explicit hierarchy admission shared with the synchronous worker.
/// The model adapter owns original phone/context/question interpretation.
pub fn validate_score_hierarchy(score: &Value) -> Result<()> {
    crate::model::validate_score_hierarchy(score)?;
    Ok(())
}

// Classify only audited model identity. Labels and user IPA never change
// native source controls or assign semantics to any of the five HMM states.
fn phone_kind(phone_id: i64, key: Option<&str>) -> Result<&'static str> {
    if matches!(phone_id,-1|0|1|2) { return Ok("silence"); }
    ensure!((3..=63).contains(&phone_id) || (174..=235).contains(&phone_id),
        "no phonetic-source classification for native phone ID {phone_id}");
    if duration::is_nucleus(phone_id) { return Ok("vowel"); }
    let key = key.context("resolved consonant is missing its audited native key")?;
    Ok(match key {
        "zh:b"|"zh:d"|"zh:g" => "stop_unaspirated",
        "zh:p"|"zh:t"|"zh:k"|"en:p"|"en:t"|"en:k" => "stop_aspirated",
        "zh:z"|"zh:zh"|"zh:j"|"zh:c"|"zh:ch"|"zh:q"|"en:ch"|"en:jh" => "affricate",
        "zh:s"|"zh:sh"|"zh:x"|"en:s"|"en:sh" => "sibilant",
        "zh:f"|"en:f"|"en:th" => "fricative",
        "zh:h"|"en:hh" => "aspiration",
        "zh:r"|"en:z"|"en:zh"|"en:v"|"en:dh" => "voiced_fricative",
        "zh:m"|"zh:n"|"en:m"|"en:n"|"en:ng"|"en:nx" => "nasal",
        "en:pau"|"en:hsharp" => "silence",
        "en:bcl"|"en:dcl"|"en:gcl"|"en:pcl"|"en:tcl"|"en:kcl" => "closure",
        "en:b"|"en:d"|"en:g" => "stop_voiced",
        "zh:l"|"en:dx"|"en:l"|"en:r"|"en:rx"|"en:w"|"en:y" => "sonorant",
        _ => bail!("unclassified audited native consonant key {key}"),
    })
}
fn put(arrays: &mut Arrays, name: &str, shape: Vec<usize>, dtype: &str, values: Vec<f64>) -> Result<()> {
    arrays.insert(name.to_owned(),NumericArray::new(shape,dtype,values)?);
    Ok(())
}
fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false, Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().is_some_and(|n| n != 0.0),
        Value::String(v) => !v.is_empty(), Value::Array(v) => !v.is_empty(), Value::Object(v) => !v.is_empty(),
    }
}
fn bool_values(values: &[bool]) -> Vec<f64> { values.iter().map(|&v| if v { 1.0 } else { 0.0 }).collect() }

fn plan_run(voice: &VoiceModel, segments: &[Value], timing: &TimingPlan, first: usize,
    end: usize, run_index: usize, source_indices: &[usize], options: &Options, requested_f0: Option<&[f64]>,
    requested_voiced: Option<&[bool]>) -> Result<(RunParameters,Vec<Value>)>
{
    let sample_rate = voice.sample_rate;
    let start_frame = timing.segments[first].start_frame;
    let frames = timing.segments[end-1].end_frame-start_frame;
    let (sources,mut prepared,continuation_links) =
        prepare_run_sources(voice,segments,timing,source_indices,first,end,options)?;
    let source_clocks=sources.iter().map(|source| {
        let context=source.state_frames.iter().try_fold(0usize,|sum,&value|sum.checked_add(value))
            .context("native source context frame overflow")?;
        Ok((source.frames,context))
    }).collect::<Result<Vec<_>>>()?;
    let (context_frames,retained)=native_reference::context_clock(&source_clocks,frames)?;
    ensure!(options.native_reference || context_frames==frames,
        "native release context requires the explicitly selected reference planner");
    let mut means = Vec::with_capacity(context_frames*123);
    let mut variances = Vec::with_capacity(context_frames*123);
    let mut pitch = Vec::with_capacity(context_frames*9);
    let mut context_authored = Vec::with_capacity(context_frames);
    let mut authored = Vec::with_capacity(frames);
    let mut phone_ids = Vec::with_capacity(frames);
    let mut segment_ids = Vec::with_capacity(frames);
    let mut kinds = Vec::with_capacity(frames);
    let mut state_indices = Vec::with_capacity(context_frames);
    let mut lengths = Vec::with_capacity(frames);
    for source in &sources {
        let spectral = selected_array(&source.selected,"spectral_means",&[5,3,41])?;
        let spectral_variance = selected_array(&source.selected,"spectral_variances",&[5,3,41])?;
        let source_pitch = selected_array(&source.selected,"pitch_triplets",&[5,3,3])?;
        for (state,&count) in source.state_frames.iter().enumerate() {
            for _ in 0..count {
                means.extend_from_slice(&spectral[state*123..(state+1)*123]);
                variances.extend_from_slice(&spectral_variance[state*123..(state+1)*123]);
                pitch.extend_from_slice(&source_pitch[state*9..(state+1)*9]);
                state_indices.push(state as f64);
                context_authored.push(source.authored_voiced);
            }
        }
    }
    for (local,prep) in prepared.iter().enumerate() {
        let source = &sources[prep.source];
        authored.extend(std::iter::repeat(source.authored_voiced).take(prep.frames));
        phone_ids.extend(std::iter::repeat(source.phone_id.unwrap_or(-1)).take(prep.frames));
        segment_ids.extend(std::iter::repeat((first+local) as i64).take(prep.frames));
        kinds.extend(std::iter::repeat(source.phonetic_class.clone()).take(prep.frames));
        lengths.extend(std::iter::repeat(source.phonemic_length as f64).take(prep.frames));
    }
    ensure!(authored.len() == frames, "run frame counts disagree with the absolute timing plan");
    ensure!(context_authored.len()==context_frames,"native release context allocation mismatch");
    let (latent,spectral_info) = spectral_mlpg(&means,&variances,context_frames,&options.acoustic)?;
    let mut before_projection = Vec::with_capacity(context_frames*24);
    let mut log_gain = Vec::with_capacity(context_frames);
    for frame in 0..context_frames {
        before_projection.extend_from_slice(&latent[frame*25..frame*25+24]);
        log_gain.push(latent[frame*25+24]);
    }
    let (lsf,stabilization) = stabilize_lsf(&before_projection,context_frames,0.01)?;
    let (context_voiced,mut voicing_info) = if options.voicing == "original-model" {
        let mut counts = Vec::with_capacity(sources.len()*5);
        let mut probabilities = Vec::with_capacity(sources.len()*5);
        let mut ids = Vec::with_capacity(sources.len()*5);
        let mut languages = Vec::with_capacity(sources.len()*5);
        for source in &sources {
            let phone = source.phone_id.context("original-model voicing requires an audited phone ID for every source")?;
            counts.extend_from_slice(&source.state_frames);
            let triplets = selected_array(&source.selected,"pitch_triplets",&[5,3,3])?;
            probabilities.extend((0..5).map(|state| triplets[state*9]));
            ids.extend(std::iter::repeat(phone).take(5));
            languages.extend(std::iter::repeat(source.language.unwrap_or(0)).take(5));
        }
        for prep in &mut prepared {
            prep.details["voicing_mode"] = json!("original-model state-probability inference; explicit CLI experiment");
        }
        voicing::infer_voicing_with_compact(&counts,&probabilities,&ids,Some(&languages),options.compact_phone_states)?
    } else { (context_authored,json!({"mode":"authored","scope":"explicit per-phone phonetic voicing; original probability trajectory retained separately"})) };
    if options.voicing == "original-model" {
        voicing_info["frames_different_from_authored"] = json!(retained.iter().zip(&authored)
            .filter(|(index,authored)|context_voiced[**index]!=**authored).count());
    }
    let (log_f0,pitch_info) = pitch_mlpg(&pitch,&context_voiced,&options.acoustic)?;
    // Both MLPG operations see every original release state. Remove only the
    // declared context indices afterward, so an interior source cannot shift
    // following notes, target pitch, active masks, or authored time maps.
    let log_f0=native_reference::retain_rows(log_f0,1,context_frames,&retained)?;
    let mut voiced=native_reference::retain_rows(context_voiced,1,context_frames,&retained)?;
    let before_projection=native_reference::retain_rows(before_projection,24,context_frames,&retained)?;
    let lsf=native_reference::retain_rows(lsf,24,context_frames,&retained)?;
    let log_gain=native_reference::retain_rows(log_gain,1,context_frames,&retained)?;
    let means=native_reference::retain_rows(means,123,context_frames,&retained)?;
    let variances=native_reference::retain_rows(variances,123,context_frames,&retained)?;
    let pitch=native_reference::retain_rows(pitch,9,context_frames,&retained)?;
    let state_indices=native_reference::retain_rows(state_indices,1,context_frames,&retained)?;
    let raw_model_f0: Vec<f64> = log_f0.iter().map(|v| v.exp()).collect();
    ensure!(raw_model_f0.iter().all(|v| v.is_finite()), "nonfinite exponentiated model pitch");
    let mut raw_min = f64::INFINITY;
    let mut raw_max = f64::NEG_INFINITY;
    for (&value,&selected) in raw_model_f0.iter().zip(&voiced) {
        if selected { raw_min = raw_min.min(value); raw_max = raw_max.max(value); }
    }
    let raw_range = if raw_min.is_finite() { json!([raw_min,raw_max]) } else { Value::Null };
    let mut f0: Vec<f64> = raw_model_f0.iter().map(|v| v.clamp(45.0,800.0)).collect();
    let mut modes = Vec::with_capacity(prepared.len());
    let mut offset = 0;
    for (prep,segment) in prepared.iter().zip(&segments[first..end]) {
        let (target,mode) = if let Some(value) = segment.get("f0_hz") {
            (Some(finite(value,"segment F0")?),"explicit_user_frequency")
        } else if let Some(value) = segment.get("midi_note") {
            let midi = finite(value,"MIDI note")?;
            ensure!((0.0..=127.0).contains(&midi), "MIDI note outside 0..127");
            (Some(440.0*2.0_f64.powf((midi-69.0)/12.0)),"explicit_equal_tempered_midi_note")
        } else { (None,"natural_log_model_interpretation_clipped_to_45_800_Hz") };
        if let Some(target) = target {
            ensure!((options.pitch_range[0]..=options.pitch_range[1]).contains(&target),
                "explicit diagnostic F0 must be {}..{} Hz",options.pitch_range[0],options.pitch_range[1]);
            f0[offset..offset+prep.frames].fill(target);
        }
        modes.push(mode.to_owned());
        offset += prep.frames;
    }
    for (value,&is_voiced) in f0.iter_mut().zip(&voiced) { if !is_voiced { *value = 0.0; } }
    let before_control = f0.clone();
    if let Some(control) = requested_f0 {
        let mask = requested_voiced.context("run control voicing is missing")?;
        ensure!(control.len() == timing.total_frames && mask.len() == timing.total_frames, "run control must align with every native frame");
        for frame in 0..frames {
            voiced[frame] &= mask[start_frame+frame];
            f0[frame] = if voiced[frame] { control[start_frame+frame] } else { 0.0 };
        }
        modes.fill("provided_or_planned_continuous_f0_intersected_with_native_uv".into());
    }
    ensure!(f0.iter().zip(&voiced).all(|(&hz,&v)| hz.is_finite() && if v { hz > 0.0 } else { hz == 0.0 }),
        "native F0/UV controls disagree");
    let mut arrays = Arrays::new();
    put(&mut arrays,"f0_hz",vec![frames],"float64",f0.clone())?;
    put(&mut arrays,"voiced",vec![frames],"bool",bool_values(&voiced))?;
    put(&mut arrays,"uv",vec![frames],"bool",voiced.iter().map(|&v| if v { 0.0 } else { 1.0 }).collect())?;
    put(&mut arrays,"f0_before_continuous_control_hz",vec![frames],"float64",before_control)?;
    put(&mut arrays,"native_phone_id",vec![frames],"int64",phone_ids.iter().map(|&v| v as f64).collect())?;
    put(&mut arrays,"phonemic_length",vec![frames],"int64",lengths)?;
    put(&mut arrays,"authored_voiced",vec![frames],"bool",bool_values(&authored))?;
    put(&mut arrays,"model_f0_hz_assuming_natural_log",vec![frames],"float64",raw_model_f0)?;
    put(&mut arrays,"model_pitch_probabilities",vec![frames,3],"float64",(0..frames*3).map(|i| pitch[i*3]).collect())?;
    put(&mut arrays,"lsf_radians_before_projection",vec![frames,24],"float64",before_projection)?;
    put(&mut arrays,"lsf_radians",vec![frames,24],"float64",lsf)?;
    put(&mut arrays,"unidentified_spectral_scalar",vec![frames],"float64",log_gain.clone())?;
    put(&mut arrays,"generated_log_gain",vec![frames],"float64",log_gain)?;
    put(&mut arrays,"spectral_observation_means",vec![frames,3,41],"float64",means)?;
    put(&mut arrays,"spectral_observation_variances",vec![frames,3,41],"float64",variances)?;
    put(&mut arrays,"pitch_probability_mean_variance",vec![frames,3,3],"float64",pitch)?;
    put(&mut arrays,"state_index",vec![frames],"int64",state_indices)?;
    put(&mut arrays,"segment_index",vec![frames],"int64",segment_ids.iter().map(|&v| v as f64).collect())?;
    put(&mut arrays,"run_index",vec![frames],"int64",vec![run_index as f64;frames])?;
    let native_keys: Vec<Option<&str>> = prepared.iter().map(|p| sources[p.source].native_key.as_deref()).collect();
    let classes: Vec<&str> = prepared.iter().map(|p| sources[p.source].phonetic_class.as_str()).collect();
    let frame_counts: Vec<usize> = prepared.iter().map(|p| p.frames).collect();
    let mut metadata = json!({"run_index":run_index,"first_segment_index":first,
        "segment_indices":(first..end).collect::<Vec<_>>(),"first_frame":start_frame,
        "sample_rate":sample_rate,"hop_samples":options.hop,
        "segments":&segments[first..end],"frame_counts":frame_counts,"native_keys":native_keys,
        "native_phone_classes":classes,
        "labels":prepared.iter().map(|p| p.details["label"].clone()).collect::<Vec<_>>(),
        "spectral_mlpg":spectral_info,"pitch_mlpg":pitch_info,"lsf_projection":stabilization,
        "trajectory_scope":"one spectral and one model-pitch MLPG over the whole contiguous run; a documented reconstruction improvement, not recovered original coarticulation",
        "explicit_f0_handling":if requested_f0.is_some() { "continuous control applied after run MLPG and intersected with native phoneme UV" }
            else { "explicit per-segment F0 pinned exactly after run MLPG" },
        "f0_modes":modes,"voicing":voicing_info,
        "gain_policy":"single peak calibration to 0.65 over the whole contiguous run; original exp/scale identified; original excitation/filter absolute calibration unresolved",
        "raw_model_f0_range_hz":raw_range,"frame_count":frames,
        "actual_seconds":frames as f64*options.hop as f64/sample_rate as f64});
    if !continuation_links.is_empty() {
        metadata["source_continuation_links"] = Value::Array(continuation_links);
    }
    if context_frames!=frames {
        metadata["native_release_context"]=json!({"policy":TERMINAL_NUCLEUS_REFERENCE_POLICY,
            "emitted_frames":frames,"native_context_frames":context_frames,
            "retained_context_frame_indices":retained,
            "mlpg_scope":"complete native context before retaining the original emitted clock",
            "original_decoder_target_plan_changed":false});
    }
    let controls = AcousticControlTrack { f0_hz:f0,voiced,phonetic_classes:kinds,phone_identity:phone_ids,
        segment_identity:segment_ids,continuity_identity:vec![run_index as i64;frames],
        period_seconds:options.hop as f64/sample_rate as f64,origin_seconds:0.0 };
    let mut reports = Vec::with_capacity(prepared.len());
    for (local,mut prep) in prepared.into_iter().enumerate() {
        let index = first+local;
        prep.details["first_frame"] = json!(timing.segments[index].start_frame);
        prep.details["frame_count"] = json!(prep.frames);
        prep.details["run_index"] = json!(run_index);
        prep.details["timing"] = serde_json::to_value(&timing.segments[index])?;
        prep.details["f0_mode"] = json!(modes[local]);
        reports.push(prep.details);
    }
    Ok((RunParameters { start_frame,frames,arrays,controls,metadata },reports))
}

/// Plan a complete worker-admitted native score without file I/O or synthesis.
/// `provided_control` is already authored (PIT/vibrato/portamento included):
/// sample it once and retain its exact anchors, then intersect native UV.
pub fn plan_score(voice: &VoiceModel, score: &Value, provided_control: Option<&Value>,
    options: &Value) -> Result<NativePlan>
{
    plan_score_internal(voice,score,provided_control,options,false,NATIVE_F0_RANGE,false)
}

/// verdant-voice's fixed 16 kHz/hop80 clock admits one frame per native phone.
/// Phones of five or more frames use the unchanged legacy allocation.
pub fn plan_voice_score(voice: &VoiceModel, score: &Value, provided_control: Option<&Value>,
    options: &Value) -> Result<NativePlan>
{
    plan_score_internal(voice,score,provided_control,options,true,NATIVE_F0_RANGE,false)
}

/// Separate controller-reference plan. The original authored target is
/// still produced by `plan_voice_score`; this never changes its default path.
pub fn plan_native_reference_score(voice: &VoiceModel, score: &Value, provided_control: Option<&Value>,
    options: &Value) -> Result<NativePlan>
{
    plan_score_internal(voice,score,provided_control,options,true,NATIVE_F0_RANGE,true)
}

fn plan_score_internal(voice: &VoiceModel, score: &Value, provided_control: Option<&Value>,
    options: &Value, compact_phone_states: bool, pitch_range: [f64; 2], native_reference: bool) -> Result<NativePlan>
{
    ensure!(score.is_object(), "score must be an object");
    let mut options = Options::new(score,options)?;
    options.compact_phone_states = compact_phone_states;
    options.pitch_range = pitch_range;
    options.native_reference = native_reference;
    if compact_phone_states {
        ensure!(voice.sample_rate == 16000 && options.hop == 80, "compact verdant-voice planning requires 16 kHz/hop80 (5 ms frames)");
    }
    ensure!(voice.sample_rate > 0, "voice sample rate must be positive");
    let trace = validate_score_phoneme_timing(score)?;
    let continuity = validate_score_continuity(score)?;
    let source_segments = score.get("segments").and_then(Value::as_array).context("score requires explicit segments")?;
    let (max_segments,max_seconds,max_points) = if continuity.is_some() {
        (CONTINUITY_MAX_SEGMENTS,CONTINUITY_MAX_SECONDS,CONTINUITY_MAX_CONTROL_POINTS)
    } else { (512,30.0,20000) };
    ensure!((1..=max_segments).contains(&source_segments.len()), "score segment count exceeds the admitted limit");
    for (index,segment) in source_segments.iter().enumerate() {
        ensure!(segment.is_object(), "segment {index} must be an object");
        let duration = field(segment,"duration_seconds",None)?;
        let gap = field(segment,"gap_after_seconds",Some(0.0))?;
        if compact_phone_states {
            ensure!(duration >= 0.005-1e-12 && gap >= 0.0,
                "segment {index} duration must be at least 5 ms and gap nonnegative");
        } else {
            ensure!(duration >= 0.025-1e-12 && gap >= 0.0,
                "segment {index} duration must be at least 25 ms and gap nonnegative");
        }
        if let Some(value) = segment.get("voiced") { ensure!(value.is_boolean(), "segment voiced must be boolean"); }
        if let Some(value) = segment.get("f0_hz") {
            ensure!((pitch_range[0]..=pitch_range[1]).contains(&finite(value,"segment F0")?),
                "segment F0 must be {}..{} Hz",pitch_range[0],pitch_range[1]);
        }
        if let Some(value) = segment.get("midi_note") {
            let midi = finite(value,"MIDI note")?;
            let hz = 440.0*2.0_f64.powf((midi-69.0)/12.0);
            ensure!((0.0..=127.0).contains(&midi) && (pitch_range[0]..=pitch_range[1]).contains(&hz),
                "segment MIDI F0 must be {}..{} Hz",pitch_range[0],pitch_range[1]);
        }
    }
    ensure!(timing::score_seconds(source_segments)? <= max_seconds+1e-12,
        "score exceeds its admitted {max_seconds}-second phrase/continuous-run limit");
    let segments = prepare_score_segments(score,&options.value)?;
    ensure!(segments.len() == source_segments.len(), "context preparation changed the score segment count");
    let source_indices = continuation_source_indices(&segments)?;
    source::validate_prepared_continuations(&segments,&source_indices)?;
    let minimum_frames = if compact_phone_states { timing::COMPACT_FLOOR_FRAMES }
        else { timing::STATE_FLOOR_FRAMES };
    let timing = timing::plan_score_timing_with_floor(&segments,voice.sample_rate,options.hop,minimum_frames)?;
    timing.total_frames.checked_mul(options.hop).context("planned sample count overflow")?;
    let frame_times: Vec<f64> = (0..timing.total_frames)
        .map(|index| (index as f64+0.5)*options.hop as f64/voice.sample_rate as f64).collect();
    let mut requested_f0 = None;
    let mut requested_voiced = None;
    let mut control_info = json!({"kind":"per_segment_or_model_pitch","automatic_portamento":false});
    if let Some(track) = provided_control.filter(|v| !v.is_null()) {
        let (f0,voiced,report) = control::sample_authored_control_in_range(track,&frame_times,&timing,max_points,max_seconds,pitch_range)?;
        requested_f0 = Some(f0); requested_voiced = Some(voiced); control_info = report;
    } else if options.prosody == "auto" || score.get("pitch_notes").is_some_and(json_truthy) {
        let inferred;
        let notes = if let Some(notes) = score.get("pitch_notes") { Some(notes) }
            else { inferred = control::midi_notes_from_segments(&segments)?; inferred.as_ref() };
        if let Some(notes) = notes.filter(|notes| json_truthy(notes)) {
            let authored_times = control::map_authored_times(&frame_times,&timing)?;
            let (f0,mut report) = control::plan_note_pitch_with_maximum(notes,&authored_times,None,options.prosody == "auto",control::DEFAULT_PORTAMENTO_MS,None,pitch_range[1])?;
            requested_voiced = Some(f0.iter().map(|&v| v > 0.0).collect());
            requested_f0 = Some(f0);
            report["kind"] = json!("score_note_pitch_applied_in_native_synthesis");
            control_info = report;
        }
    }
    if let Some(f0) = &requested_f0 {
        ensure!(f0.iter().all(|&v| v == 0.0 || (pitch_range[0]..=pitch_range[1]).contains(&v)),
            "planned control F0 must remain within {}..{} Hz, including PIT, detune and vibrato",pitch_range[0],pitch_range[1]);
    }
    let mut runs = Vec::new();
    let mut reports = Vec::with_capacity(segments.len());
    let mut first = 0;
    for index in 0..segments.len() {
        if timing.segments[index].gap_frames() == 0 && index+1 < segments.len() { continue; }
        let (run,mut segment_reports) = plan_run(voice,&segments,&timing,first,index+1,runs.len(),&source_indices,&options,
            requested_f0.as_deref(),requested_voiced.as_deref())?;
        reports.append(&mut segment_reports);
        runs.push(run);
        first = index+1;
    }
    ensure!(!runs.is_empty() && first == segments.len(), "score has no complete acoustic runs");
    let mut metadata = json!({"sample_rate":voice.sample_rate,"hop_samples":options.hop,
        "model_provenance":voice.provenance,"segments":reports,
        "runs":runs.iter().map(|r| r.metadata.clone()).collect::<Vec<_>>(),
        "timing":timing.report(minimum_frames),"time_map_render_seconds":timing.render_seconds,
        "time_map_authored_seconds":timing.authored_seconds,
        "synthesis_policy":"contiguous non-gap runs; per-segment contexts and observations, shared MLPG and DSP state",
        "run_seed_policy":"score seed + first segment index of run",
        "selection_mode":options.selection,"acoustic_mode":options.acoustic,"neighbor_mode":options.neighbor,
        "voicing_mode":options.voicing,"duration_mode":options.duration,"source_profile":options.profile,
        "prosody_mode":options.prosody,"vowel_brightness_db":field(&options.value,"vowel_brightness_db",Some(4.0))?,
        "vowel_balance_db":field(&options.value,"vowel_balance_db",Some(0.0))?,
        "gain_mode":choice(&options.value,"gain_mode","off",&["off","relative"])?,
        "continuous_pitch_control":control_info,
        "phoneme_timing":timing::realized_phoneme_timing(trace.as_ref(),&timing,voice.sample_rate,options.hop)?,
        "continuity":timing::continuity_report(continuity.as_ref(),runs.len(),&timing,voice.sample_rate,options.hop),
        "context_consistency_scope":"recovered current, explicitly selected neighbor and explicitly supplied original hierarchy predicates; shared unresolved truths and numeric intervals across all 12 trees; original neighbor selection, protected hierarchy paths and text-to-hierarchy prediction remain incomplete"});
    for name in ["phonetic_correction","vocal_tract","model_dynamics","consonant_dynamics"] {
        if let Some(value) = options.value.get(name).filter(|v| !v.is_null()) { metadata[name] = value.clone(); }
    }
    // prepare_score_segments has already validated and normalized the graph
    // on each explicit context; compact the report once without losing the
    // caller's score mapping or per-tree selection provenance.
    let normalized_graph = score.get("original_hierarchy").filter(|v| !v.is_null()).map(|supplied| {
        json!({"schema_version":supplied["schema_version"],
            "nodes":segments[0]["context"]["features"]["OriginalHierarchy"]["nodes"],
            "segment_node_indices":supplied["segment_node_indices"],"provenance":supplied["provenance"]})
    });
    metadata = compact_hierarchy_report(&metadata,normalized_graph.as_ref())?;
    Ok(NativePlan { runs,duration_frames:timing.total_frames,sample_rate:voice.sample_rate,hop_samples:options.hop,metadata })
}

#[cfg(test)]
mod tests;
