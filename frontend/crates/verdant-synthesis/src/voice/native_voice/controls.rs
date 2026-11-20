//! Frozen context/local control blend with the original float32 arithmetic.
//!
//! `research/verdant-engine-parent-repair-20261119/r13b/source/verdant_dual_runtime.py`
//! and `verdant_history_gate.py` are pinned at Git object
//! a3b0a1a3f14020f349d575f2bc1a66b05d3035ba. The earlier technical sources were
//! `neural/cfinal/r12_local_runtime.py` and `neural/cfinal/r12_research_runtime.py`.
//! The native voice loader admits graph identity and neighbor encoding.
use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde_json::Value;

use crate::{Arrays, graph::{GraphSession, GraphTensor}};
use super::super::{CONTROLS, CORE, slice, validate_conditions};

const CONTROLLER_FIELDS: [&str; 5] = ["latent25", "phone_ids", "languages", "states", "prosody"];
const ACCEPTED_CONTEXT: usize = 31;
const HISTORY: usize = 512;
const FADE_BEGIN: usize = 448;
const SEMANTIC_SMOOTH: u32 = 4;

pub(super) fn require_recurrent_abi(graph: &GraphSession) -> Result<()> {
    // The frozen Loop/Slice export declares every output axis symbolic,
    // including hidden_out. Each executed tensor is checked exactly below.
    graph.require_interface(&[
        ("latent25", 1, &[1, -1, 25]), ("phone_ids", 7, &[1, -1, 3]),
        ("languages", 7, &[1, -1]), ("states", 7, &[1, -1]),
        ("prosody", 1, &[1, -1, 8]), ("hidden_in", 1, &[2, 1, 128]),
    ], &[
        ("harmonic_logits", 1, &[-1, -1, -1]),
        ("noise_log_gains", 1, &[-1, -1, -1]),
        ("log_amplitude", 1, &[-1, -1, -1]),
        ("hidden_out", 1, &[-1, -1, -1]),
    ])
}

pub(super) fn require_accepted_abi(graph: &GraphSession) -> Result<()> {
    graph.require_interface(&[
        ("latent25", 1, &[1, -1, 25]), ("phone_ids", 7, &[1, -1, 3]),
        ("languages", 7, &[1, -1]), ("states", 7, &[1, -1]),
        ("prosody", 1, &[1, -1, 8]),
    ], &[
        ("harmonic_logits", 1, &[1, -1, 129]),
        ("noise_log_gains", 1, &[1, -1, 65]),
        ("log_amplitude", 1, &[1, -1, 1]),
    ])
}

fn controller_inputs(arrays: &Arrays, first: usize, last: usize) -> Result<BTreeMap<String, GraphTensor>> {
    CONTROLLER_FIELDS.into_iter().map(|key| {
        Ok((key.into(), slice(&arrays[key], first, last)?))
    }).collect()
}

fn friction_table(value: &Value) -> Result<([bool; 256], [bool; 256])> {
    let rows = value.as_array().context("native voice phone table must be an array")?;
    let mut known = [false; 256];
    let mut friction = [false; 256];
    for row in rows {
        let id = row["id"].as_u64().filter(|&id| id < 256)
            .context("native voice phone ID must be an integer in 0..255")? as usize;
        let fricative = row["fricative"].as_bool().context("native voice fricative flag must be boolean")?;
        let affricate = row["affricate"].as_bool().context("native voice affricate flag must be boolean")?;
        known[id] = true;
        friction[id] = fricative || affricate;
    }
    Ok((known, friction))
}

fn gate_block(arrays: &Arrays, first: usize, last: usize, friction: &[bool; 256], alpha: &mut [f32]) {
    let active = &arrays["active"].values;
    let voiced = &arrays["voiced"].values;
    let ids = &arrays["phone_ids"].values;
    let prosody = &arrays["prosody"].values;
    let begin = first.saturating_sub(HISTORY - 1);
    let mut age = 0;
    let mut entered = false;
    // Four binary FIR slots have an exact float32 sum. A bit mask keeps
    // the reference's reset/shift semantics without allocating per frame.
    let mut semantic_history = 0_u8;
    for frame in begin..last {
        if active[frame] == 0. {
            age = 0;
            entered = false;
            semantic_history = 0;
            if frame >= first { alpha[frame - first] = 0.; }
            continue;
        }
        age = (age + 1).min(HISTORY);
        let prior = frame > begin && active[frame - 1] == 1.;
        let onset = !prior || ids[frame * 3 + 1] != ids[(frame - 1) * 3 + 1]
            || (prosody[frame * 8 + 2] as f32) < (prosody[(frame - 1) * 8 + 2] as f32);
        if onset { entered = prior; }
        let raw = entered && friction[ids[frame * 3 + 1] as usize] && voiced[frame] == 0.;
        semantic_history = ((semantic_history << 1) | u8::from(raw)) & ((1 << SEMANTIC_SMOOTH) - 1);
        if frame >= first {
            let position = ((age as f32 - FADE_BEGIN as f32) / (HISTORY - FADE_BEGIN) as f32).clamp(0., 1.);
            let square = position * position;
            let slope = 3.0_f32 - 2.0_f32 * position;
            let long = square * slope;
            let semantic = semantic_history.count_ones() as f32 / SEMANTIC_SMOOTH as f32;
            alpha[frame - first] = long.max(semantic);
        }
    }
}

fn control_values<'a>(outputs: &'a BTreeMap<String, GraphTensor>, key: &str,
    frames: usize, width: usize) -> Result<&'a [f32]> {
    let tensor = outputs.get(key).with_context(|| format!("native voice controller lacks {key}"))?;
    ensure!(tensor.shape == [1, frames as i64, width as i64],
        "native voice controller {key} output frame shape changed");
    // GraphSession::run validates the complete shape/data length and every
    // value's finiteness when it takes ownership of the native outputs.
    tensor.f32_values()
}

pub(super) fn predict(arrays: &Arrays, frames: usize, recurrent: &mut GraphSession,
    accepted: &mut GraphSession, native_phone_table: &Value) -> Result<BTreeMap<String, Vec<f32>>> {
    ensure!(validate_conditions(arrays)? == frames, "native voice condition and requested frame clocks differ");
    let (known, friction) = friction_table(native_phone_table)?;
    for frame in 0..frames {
        ensure!(arrays["active"].values[frame] == 0. || known[arrays["phone_ids"].values[frame * 3 + 1] as usize],
            "native voice gate encountered an unknown active phone ID");
    }
    let mut controls: BTreeMap<String, Vec<f32>> = CONTROLS.into_iter()
        .map(|(key, width)| (key.into(), Vec::with_capacity(frames * width))).collect();
    let mut hidden = GraphTensor::f32(vec![2, 1, 128], vec![0.; 256])?;
    let mut alpha = [0.0_f32; CORE];
    for first in (0..frames).step_by(CORE) {
        let last = (first + CORE).min(frames);
        let count = last - first;
        let begin = first.saturating_sub(ACCEPTED_CONTEXT);
        gate_block(arrays, first, last, &friction, &mut alpha[..count]);
        // R12B receives its full 31-frame left context on every call. Only
        // emitted rows, starting at first-begin, belong to this core block.
        let local = accepted.run(controller_inputs(arrays, begin, last)?)?;
        ensure!(local.len() == CONTROLS.len(), "native voice local controller output fields changed");
        let mut inputs = controller_inputs(arrays, first, last)?;
        inputs.insert("hidden_in".into(), hidden);
        let mut wide = recurrent.run(inputs)?;
        ensure!(wide.len() == CONTROLS.len() + 1, "native voice context controller output fields changed");
        hidden = wide.remove("hidden_out").context("native voice context controller lacks hidden_out")?;
        ensure!(hidden.shape == [2, 1, 128], "native voice context hidden output shape changed");
        hidden.f32_values()?;
        // D runs continuously even where alpha is one. Its hidden output
        // moves into the next call; rest resets remain inside the graph.
        for (key, width) in CONTROLS {
            let accepted_values = control_values(&local, key, last - begin, width)?;
            let recurrent_values = control_values(&wide, key, count, width)?;
            let target = controls.get_mut(key).expect("declared native voice control");
            let rest = if key == "harmonic_logits" { 0.0_f32 } else { -12.0_f32 };
            for (offset, &a) in alpha[..count].iter().enumerate() {
                let accepted_row = &accepted_values[(first - begin + offset) * width..(first - begin + offset + 1) * width];
                let recurrent_row = &recurrent_values[offset * width..(offset + 1) * width];
                if arrays["active"].values[first + offset] == 0. {
                    ensure!(accepted_row.iter().all(|&value| value == rest),
                        "native voice local controller exact rest gate failed for {key}");
                    target.resize(target.len() + width, rest);
                } else if a == 0. {
                    target.extend_from_slice(recurrent_row);
                } else if a == 1. {
                    target.extend_from_slice(accepted_row);
                } else {
                    let recurrent_weight = 1.0_f32 - a;
                    for (&d, &b) in recurrent_row.iter().zip(accepted_row) {
                        // Keep the two float32 products separate, as in
                        // NumPy; no fused multiply-add or float64 blend.
                        let d_term = recurrent_weight * d;
                        let b_term = a * b;
                        let value = d_term + b_term;
                        ensure!(value.is_finite(), "native voice blended {key} is nonfinite");
                        target.push(value);
                    }
                }
            }
        }
    }
    for (key, width) in CONTROLS {
        ensure!(controls[key].len() == frames * width, "native voice blended {key} frame clock changed");
    }
    Ok(controls)
}
