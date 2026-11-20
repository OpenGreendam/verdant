//! Audited phone-conditioned source envelopes, with no HMM-state semantics.
use anyhow::{bail, ensure, Result};
use serde_json::{json, Value};
use std::f64::consts::PI;

#[path = "phone_inventory.rs"]
mod inventory;

pub fn phone_kind(id: i64) -> Result<&'static str> {
    if matches!(id, -1 | 0 | 1 | 2) { return Ok("silence"); }
    inventory::phone_row(id).map(|row| row.2)
        .ok_or_else(|| anyhow::anyhow!("No phonetic-source classification for native phone ID {id}"))
}

pub fn native_phone_key(id: i64) -> Option<&'static str> {
    inventory::phone_row(id).map(|row| row.0)
}

pub fn native_phone_name(id: i64) -> Option<&'static str> {
    inventory::phone_row(id).map(|row| row.1)
}

pub struct SourceControls {
    pub periodic_gain: Vec<f64>,
    pub noise_gain: Vec<f64>,
    pub brightness_db: Vec<f64>,
    pub phone_kind: Vec<&'static str>,
    pub metadata: Value,
}

pub fn source_controls(ids: &[i64], segments: &[i64], voiced: &[bool],
    sample_rate: u32, hop: usize, vowel_brightness_db: f64,
    phone_lengths: Option<&[i64]>) -> Result<SourceControls> {
    ensure!(!ids.is_empty() && ids.len() == segments.len() && ids.len() == voiced.len(),
        "phonetic source requires aligned nonempty phone IDs, segment IDs and voicing");
    ensure!(sample_rate > 0 && hop > 0, "positive integer sample_rate and hop required");
    ensure!(vowel_brightness_db.is_finite() && (0.0..=12.0).contains(&vowel_brightness_db),
        "vowel brightness must be 0..12 dB");
    if let Some(lengths) = phone_lengths {
        ensure!(lengths.len() == ids.len() && lengths.iter().all(|&length| length >= 1),
            "phone lengths must be aligned positive integer values");
    }
    let sample_count = ids.len().checked_mul(hop)
        .ok_or_else(|| anyhow::anyhow!("phonetic source duration overflow"))?;
    let mut periodic = vec![0.0; sample_count];
    let mut noise = vec![0.0; sample_count];
    let mut brightness = vec![0.0; ids.len()];
    let kinds = ids.iter().map(|&id| phone_kind(id)).collect::<Result<Vec<_>>>()?;
    let mut records = Vec::new();
    let mut first = 0;
    while first < ids.len() {
        let mut last = first+1;
        while last < ids.len() && segments[last] == segments[first] { last += 1; }
        ensure!(ids[first..last].iter().all(|&id| id == ids[first]),
            "one segment must retain one native phone identity");
        let length = phone_lengths.map_or(1, |lengths| lengths[first]);
        if let Some(lengths) = phone_lengths {
            ensure!(lengths[first..last].iter().all(|&n| n == length),
                "one segment must retain one phonemic length");
        }
        let kind = kinds[first];
        let begin = first*hop;
        let end = last*hop;
        let count = end-begin;
        let duration = count as f64 / f64::from(sample_rate);
        let mut closure = 0.0_f64;
        let mut release = 0.0_f64;
        let mut closure_policy = "ordinary duration-based closure, capped at 35 ms";
        let frame_brightness = match kind {
            "vowel" => vowel_brightness_db,
            "sonorant" | "voiced_fricative" => vowel_brightness_db.min(1.0),
            _ => 0.0,
        };
        brightness[first..last].fill(frame_brightness);
        if matches!(kind, "stop_unaspirated" | "stop_aspirated" | "stop_voiced" | "affricate") {
            let ratio = if kind == "affricate" { 0.20 } else if kind == "stop_aspirated" { 0.30 } else { 0.42 };
            closure = 0.035_f64.min(0.006_f64.max(ratio*duration));
            release = 0.008_f64.min(0.003_f64.max(0.12*duration));
            closure = closure.min(duration-release);
            if length > 1 {
                let release_tail = 0.045_f64.min(0.020_f64.max(0.15*duration));
                closure = closure.max(duration-release_tail).min(duration-release);
                closure_policy = "explicit long stop/affricate: quiet hold then one final release; 20..45 ms release-plus-tail budget";
            }
        }
        for local in 0..count {
            let sample = begin+local;
            let active = voiced[sample/hop];
            let time = (local as f64+0.5)/f64::from(sample_rate);
            let (mut pg, ng) = match kind {
                "vowel" => (1.0, 0.008),
                "nasal" | "sonorant" => (1.0, 0.006),
                "voiced_fricative" => (0.65, 0.35),
                "sibilant" | "fricative" => (0.0, 1.0),
                "aspiration" => (0.0, 0.65),
                "closure" => (0.0, 0.008),
                "stop_unaspirated" | "stop_aspirated" | "stop_voiced" | "affricate" => {
                    let closed = time < closure;
                    let burst = time >= closure && time < closure+release;
                    let ng = if closed { 0.008 } else if burst {
                        let phase = ((time-closure)/release).clamp(0.0, 1.0);
                        0.12 + 1.08 * (PI*phase).sin().powi(2)
                    } else if kind == "affricate" { 0.90 }
                    else if kind == "stop_aspirated" { 0.60 }
                    else if kind == "stop_voiced" { 0.10 } else { 0.08 };
                    let pg = if kind == "affricate" { if closed { 0.10 } else { 0.35 } }
                        else if kind == "stop_voiced" { if closed { 0.12 } else { 0.70 } }
                        else { 0.0 };
                    (pg, ng)
                }
                "silence" => (0.0, 0.0),
                _ => bail!("unknown phonetic source class {kind}"),
            };
            if !active { pg = 0.0; }
            periodic[sample] = pg;
            noise[sample] = ng;
        }
        records.push(json!({"segment_index":segments[first], "phone_id":ids[first],
            "native_key":native_phone_key(ids[first]), "kind":kind,
            "first_frame":first, "end_frame":last,
            "closure_seconds":closure, "release_seconds":release,
            "phonemic_length":length, "closure_policy":closure_policy,
            "phonetic_envelope_source":"declared synthesis heuristic, not recovered HMM-state semantics"}));
        first = last;
    }
    // One ramp per voiced island, not per phone. Adjacent voiced phones retain
    // their glottal clock and never acquire an internal source fade.
    let ramp_length = (0.002*f64::from(sample_rate)).round_ties_even().max(1.0) as usize;
    first = 0;
    while first < voiced.len() {
        let mut last = first+1;
        while last < voiced.len() && voiced[last] == voiced[first] { last += 1; }
        if voiced[first] {
            let begin = first*hop;
            let end = last*hop;
            let ramp = ramp_length.min((end-begin)/2);
            let step = if ramp > 1 { (PI/2.0)/(ramp-1) as f64 } else { 0.0 };
            for index in 0..ramp {
                let window = if index+1 == ramp && ramp > 1 { (PI/2.0).sin().powi(2) }
                    else { (index as f64 * step).sin().powi(2) };
                periodic[begin+index] *= window;
                periodic[end-1-index] *= window;
            }
        }
        first = last;
    }
    Ok(SourceControls {periodic_gain:periodic, noise_gain:noise,
        brightness_db:brightness, phone_kind:kinds,
        metadata:json!({"profile":"phonetic-v1", "known_phone_ids_required":true,
            "vowel_brightness_db":vowel_brightness_db,
            "state_indices_assigned_phonetic_semantics":false,
            "spectral_model_modified":false,"segments":records})})
}
