//! Frozen `Native48Session.render` host for the admitted 48 kHz decoder graph.
//!
//! Both oscillator clocks interpolate in float32 before integrating in
//! float64. Context is discarded after graph execution; noise keeps its global
//! sample address, and neither clipping nor waveform normalization is applied.
use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use crate::{
    Arrays,
    dsp::numpy_math::pairwise_sum,
    graph::{GraphSession, GraphTensor, TensorData},
};
use super::super::{CONTROLS, slice, validate_conditions, white_noise};

const CORE: usize = 64;
const CONTEXT: usize = 16;
const HOP16: usize = 80;
const HOP48: usize = 240;
const RATE16: f64 = 16_000.;
const RATE48: f64 = 48_000.;
const SEMANTIC_WIDTH: usize = 16;
const NOISE48_SEED_XOR: u64 = 0x48000;
const WAVEFORMS: [(&str, usize); 5] = [
    ("waveform", HOP48),
    ("lowband", HOP48),
    ("periodic_high", HOP48),
    ("noise_high", HOP48),
    ("waveform16", HOP16),
];

/// The complete accepted R12B combined graph interface, without opening a model.
pub(super) fn decoder_abi() -> (
    &'static [(&'static str, i32, &'static [i64])],
    &'static [(&'static str, i32, &'static [i64])],
) {
    (
        &[
            ("harmonic_logits", 1, &[1, -1, 129]),
            ("noise_log_gains", 1, &[1, -1, 65]),
            ("log_amplitude", 1, &[1, -1, 1]),
            ("semantic", 1, &[1, -1, 16]),
            ("f0_hz", 1, &[1, -1]),
            ("voiced", 9, &[1, -1]),
            ("active", 9, &[1, -1]),
            ("white_noise16", 1, &[1, -1]),
            ("white_noise48", 1, &[1, -1]),
            ("phase16_in", 11, &[1]),
            ("phase48_in", 11, &[1]),
        ],
        &[
            ("waveform", 1, &[1, -1]),
            ("lowband", 1, &[1, -1]),
            ("periodic_high", 1, &[1, -1]),
            ("noise_high", 1, &[1, -1]),
            ("waveform16", 1, &[1, -1]),
            ("phase16_out", 11, &[1]),
            ("phase48_out", 11, &[1]),
        ],
    )
}

pub(super) fn require_decoder_abi(graph: &GraphSession) -> Result<()> {
    let (inputs, outputs) = decoder_abi();
    graph.require_interface(inputs, outputs)
}

pub(super) struct Decoded {
    pub(super) waves: BTreeMap<String, Vec<f32>>,
    pub(super) phase16: f64,
    pub(super) phase48: f64,
}

/// Integer-grid interpolation matches both frozen sample clocks. Keeping the
/// operations in float32 also preserves their rounding at late block offsets.
fn clock<'a>(f0: &'a [f32], voiced: &'a [bool], hop: usize) -> impl Iterator<Item = f32> + 'a {
    (0..f0.len() * hop).map(move |sample| {
        let shifted = sample + hop / 2;
        let left = (shifted / hop).saturating_sub(1).min(f0.len() - 1);
        let right = (left + 1).min(f0.len() - 1);
        let fraction = if sample < hop / 2 { 0. } else { (shifted % hop) as f32 / hop as f32 };
        let a = if voiced[left] { f0[left] } else { 300. };
        let b = if voiced[right] { f0[right] } else { 300. };
        a * (1. - fraction) + b * fraction
    })
}

/// Retain only the prefix positions consumed by the fixed core budget. The
/// accumulation still visits every sample in NumPy cumsum's sequential order.
fn core_phase_prefix(f0: &[f32], voiced: &[bool], hop: usize, rate: f64) -> Vec<f64> {
    let mut prefix = Vec::with_capacity(f0.len().div_ceil(CORE) + 1);
    let mut samples = clock(f0, voiced, hop);
    let mut phase = 0.;
    prefix.push(phase);
    for _ in (0..f0.len()).step_by(CORE) {
        for hz in samples.by_ref().take(CORE * hop) {
            phase += f64::from(hz) / rate;
        }
        prefix.push(phase);
    }
    prefix
}

fn block_phases(f0: &[f32], voiced: &[bool], hop: usize, rate: f64,
    offset: usize, prefix: f64) -> (f64, f64) {
    let local: Vec<f64> = clock(f0, voiced, hop).map(|hz| f64::from(hz) / rate).collect();
    // np.sum uses a pairwise reduction, unlike the global np.cumsum above.
    // Reusing the pinned reduction keeps phase offsets independent of rounding
    // changes from a sequential local sum or dividing after the reduction.
    let phase = (prefix - pairwise_sum(&local[..offset])).rem_euclid(1.);
    let expected = (phase + pairwise_sum(&local)).rem_euclid(1.);
    (phase, expected)
}

fn require_phase(output: &BTreeMap<String, GraphTensor>, key: &str, expected: f64) -> Result<()> {
    let tensor = output.get(key).with_context(|| format!("native voice decoder is missing {key}"))?;
    ensure!(tensor.shape == [1], "native voice decoder {key} phase shape changed");
    let actual = tensor.f64_values()?[0];
    ensure!(actual.is_finite() && (0. ..1.).contains(&actual),
        "native voice decoder {key} is not a valid phase state");
    let difference = (actual - expected).abs();
    ensure!(difference.min(1. - difference) <= 1e-7,
        "native voice decoder {key} did not integrate its sample clock");
    Ok(())
}

/// Execute an already admitted combined graph with the frozen 64-frame core
/// and 16-frame context. Phase outputs remain float64 in the caller's result.
pub(super) fn decode(arrays: &Arrays, semantic: &[f32], controls: &BTreeMap<String, Vec<f32>>,
    graph: &mut GraphSession, frames: usize, seed: u64) -> Result<Decoded> {
    ensure!(validate_conditions(arrays)? == frames, "native voice decoder condition frame count differs");
    ensure!(semantic.len() == frames * SEMANTIC_WIDTH && semantic.iter().all(|v| v.is_finite()),
        "native voice decoder semantic input must be finite float32[T,16]");
    ensure!(controls.len() == CONTROLS.len(), "native voice decoder requires exactly three control fields");
    for (key, width) in CONTROLS {
        let values = controls.get(key).with_context(|| format!("native voice decoder is missing {key}"))?;
        ensure!(values.len() == frames * width && values.iter().all(|v| v.is_finite()),
            "native voice decoder {key} control shape or finite contract failed");
    }

    let f0: Vec<f32> = arrays["f0_hz"].values.iter().map(|&v| v as f32).collect();
    let voiced: Vec<bool> = arrays["voiced"].values.iter().map(|&v| v == 1.).collect();
    let prefix16 = core_phase_prefix(&f0, &voiced, HOP16, RATE16);
    let prefix48 = core_phase_prefix(&f0, &voiced, HOP48, RATE48);
    let mut waves: BTreeMap<String, Vec<f32>> = WAVEFORMS.into_iter()
        .map(|(key, hop)| (key.into(), Vec::with_capacity(frames * hop))).collect();

    for (block, first) in (0..frames).step_by(CORE).enumerate() {
        let last = (first + CORE).min(frames);
        let begin = first.saturating_sub(CONTEXT);
        let end = (last + CONTEXT).min(frames);
        let count = end - begin;
        let local_f0 = &f0[begin..end];
        let local_voiced = &voiced[begin..end];
        let (phase16, expected16) = block_phases(local_f0, local_voiced, HOP16, RATE16,
            (first - begin) * HOP16, prefix16[block]);
        let (phase48, expected48) = block_phases(local_f0, local_voiced, HOP48, RATE48,
            (first - begin) * HOP48, prefix48[block]);

        let mut inputs = BTreeMap::new();
        for (key, width) in CONTROLS {
            inputs.insert(key.into(), GraphTensor::f32(vec![1, count as i64, width as i64],
                controls[key][begin * width..end * width].to_vec())?);
        }
        inputs.insert("semantic".into(), GraphTensor::f32(vec![1, count as i64, SEMANTIC_WIDTH as i64],
            semantic[begin * SEMANTIC_WIDTH..end * SEMANTIC_WIDTH].to_vec())?);
        for key in ["f0_hz", "voiced", "active"] {
            inputs.insert(key.into(), slice(&arrays[key], begin, end)?);
        }
        inputs.insert("white_noise16".into(), GraphTensor::f32(vec![1, (count * HOP16) as i64],
            white_noise(seed, count * HOP16, begin * HOP16))?);
        inputs.insert("white_noise48".into(), GraphTensor::f32(vec![1, (count * HOP48) as i64],
            white_noise(seed ^ NOISE48_SEED_XOR, count * HOP48, begin * HOP48))?);
        for (key, phase) in [("phase16_in", phase16), ("phase48_in", phase48)] {
            inputs.insert(key.into(), GraphTensor::new(vec![1], TensorData::F64(vec![phase]))?);
        }

        let output = graph.run(inputs)?;
        require_phase(&output, "phase16_out", expected16)?;
        require_phase(&output, "phase48_out", expected48)?;
        for (key, hop) in WAVEFORMS {
            let tensor = output.get(key).with_context(|| format!("native voice decoder is missing {key}"))?;
            ensure!(tensor.shape == [1, (count * hop) as i64], "native voice decoder {key} sample clock changed");
            // GraphSession validates finiteness and storage before returning.
            let values = tensor.f32_values()?;
            waves.get_mut(key).expect("declared native voice waveform branch")
                .extend_from_slice(&values[(first - begin) * hop..(last - begin) * hop]);
        }
    }

    for (frame, &is_voiced) in voiced.iter().enumerate() {
        if !is_voiced {
            ensure!(waves["periodic_high"][frame * HOP48..(frame + 1) * HOP48].iter().all(|&v| v == 0.),
                "native voice decoder periodic high band leaked into UV");
        }
        if arrays["active"].values[frame] == 0. {
            ensure!(waves["waveform"][frame * HOP48..(frame + 1) * HOP48].iter().all(|&v| v == 0.),
                "native voice decoder waveform leaked into an authored rest");
        }
    }

    Ok(Decoded {
        waves,
        phase16: prefix16[prefix16.len() - 1].rem_euclid(1.),
        phase48: prefix48[prefix48.len() - 1].rem_euclid(1.),
    })
}
