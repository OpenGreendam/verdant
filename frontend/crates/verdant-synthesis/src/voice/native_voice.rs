//! Native execution of the admitted verdant-voice graphs.
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use verdant_voice_format::{CONTEXT_CONTROLLER as RECURRENT, LOCAL_CONTROLLER as ACCEPTED, DECODER, PHONE_TABLE as PHONES};

use crate::{Arrays, AudioResult, NativePlan, NumericArray, model::VoicePackage, graph::GraphSession};
use super::{CONTROLS, RESOLVED_NEIGHBOR_ENCODING, neighbor_encoding_from_contracts,
    require_condition_neighbor_encoding, validate_conditions};

mod controls;
mod decode;
mod semantics;

pub fn features(conditions: &Arrays, plan: &NativePlan) -> Result<Vec<f32>> {
    semantics::features(conditions, plan)
}

const RELEASE_SOURCE: &str = "a3b0a1a3f14020f349d575f2bc1a66b05d3035ba";

struct Engine<'a> {
    recurrent: GraphSession,
    accepted: GraphSession,
    decoder: GraphSession,
    phones: &'a Value,
}

impl<'a> Engine<'a> {
    fn open(directory: &Path, source: &'a VoicePackage, options: &Value) -> Result<Self> {
        const THREADS: usize = 2;
        ensure!(directory.canonicalize()? == source.directory, "native graph directory differs from the admitted package");
        let recurrent = GraphSession::new(&source.context_graph, options, THREADS)?;
        controls::require_recurrent_abi(&recurrent)?;
        let accepted = GraphSession::new(&source.local_graph, options, THREADS)?;
        controls::require_accepted_abi(&accepted)?;
        let decoder = GraphSession::new(&source.decoder_graph, options, THREADS)?;
        decode::require_decoder_abi(&decoder)?;
        let phones = &source.phone_table;
        Ok(Self { recurrent, accepted, decoder, phones })
    }
}

pub fn validate_package(directory: &Path, source: &VoicePackage, options: &Value) -> Result<()> {
    ensure!(neighbor_encoding_from_contracts(&source.contracts)? == RESOLVED_NEIGHBOR_ENCODING,
        "native voice requires the resolved-context source bank contract");
    Engine::open(directory, source, options)?;
    Ok(())
}

/// The native reference prepares controller-only conditions. The decoder
/// receives the original authored target verbatim.
pub(super) fn render_with_controller(directory: &Path, source: &VoicePackage, conditions: &Arrays, controller: &Arrays,
    semantic: Vec<f32>, options: &Value, seed: u64) -> Result<AudioResult> {
    require_condition_neighbor_encoding(conditions, RESOLVED_NEIGHBOR_ENCODING)?;
    let frames = validate_conditions(conditions)?;
    require_condition_neighbor_encoding(controller, RESOLVED_NEIGHBOR_ENCODING)?;
    ensure!(validate_conditions(controller)? == frames, "controller and decoder frame clocks differ");
    ensure!(conditions["f0_hz"].values.iter().all(|&hz| hz == 0. || (35. ..=2000.).contains(&hz)),
        "native voice voiced F0 outside 35..2000 Hz");
    let mut engine = Engine::open(directory, source, options)?;
    let controls = controls::predict(controller, frames, &mut engine.recurrent, &mut engine.accepted, engine.phones)?;
    let mut decoded = decode::decode(conditions, &semantic, &controls, &mut engine.decoder, frames, seed)?;
    let wave = decoded.waves.remove("waveform").context("native voice decoder waveform missing")?;
    let samples: Vec<_> = wave.into_iter().map(f64::from).collect();
    let mut arrays = conditions.clone();
    arrays.insert("semantic".into(), NumericArray::new(vec![frames, 16], "float32", semantic.into_iter().map(f64::from).collect())?);
    for (key, width) in CONTROLS {
        arrays.insert(key.into(), NumericArray::new(vec![frames, width], "float32", controls[key].iter().copied().map(f64::from).collect())?);
    }
    for (key, values) in decoded.waves {
        arrays.insert(key, NumericArray::new(vec![values.len()], "float32", values.into_iter().map(f64::from).collect())?);
    }
    arrays.insert("waveform".into(), NumericArray::new(vec![samples.len()], "float32", samples.clone())?);
    for (name, value) in [("phase16_out", decoded.phase16), ("phase48_out", decoded.phase48)] {
        arrays.insert(name.into(), NumericArray::new(vec![1], "float64", vec![value])?);
    }
    Ok(AudioResult {
        sample_rate: 48000,
        samples,
        source_dtype: "float32".into(),
        arrays,
        metadata: json!({
            "engine_name":"verdant-voice",
            "candidate":"verdant-voice",
            "kind":"verdant-voice-native-onnx",
            "format":"verdant-voice-render-v1", "route":"verdant-voice", "sample_rate":48000,
            "hop_samples":240, "frame_period_seconds":0.005, "frames":frames,
            "release_source_commit":RELEASE_SOURCE, "training_updates":0, "quality_admission":false,
            "model_sha256":{
                (RECURRENT.0):source.hashes["verdant_voice_context_controller"],
                (ACCEPTED.0):source.hashes["verdant_voice_local_controller"],
                (DECODER.0):source.hashes["verdant_voice_decoder"],
            },
            "base_context_controller_sha256":RECURRENT.2,
            "bank_phone_table_sha256":PHONES.2,
            "voice_bank_sha256":source.hashes["bank_tensors"],
            "neighbor_encoding":RESOLVED_NEIGHBOR_ENCODING, "periodic_filter_phase":"zero-phase-v1",
            "exact_leaf_bank_used":true, "original_statistics_and_mlpg_required":true,
            "authored_pitch":super::authored_pitch_report(conditions),
            "linked_decoder_sample_rates":[16000,48000], "normalization":"none",
            "old_waveform_generated":false, "fallback_used":false, "reference_audio_opened":false,
        }),
    })
}

