//! Bounded admission for the synthetic voice protocol fixture.

use serde_json::{Map, Value};

use crate::{MAX_PHRASE_SECONDS, PROTOCOL};

pub(crate) fn request(value: &Value) -> Result<&Map<String, Value>, String> {
    let object = value.as_object().filter(|object| object.get("protocol").and_then(Value::as_str) == Some(PROTOCOL))
        .ok_or("Expected protocol ogd-worker/1")?;
    if unknown_fields(object, &["protocol", "request_id", "op", "job_id", "revision", "route", "voice_ref", "payload"]).next().is_some() {
        return Err("Unknown request fields".into());
    }
    if !object.get("request_id").and_then(Value::as_str).is_some_and(|id| !id.is_empty() && id.chars().count() <= 128) {
        return Err("request_id required, at most 128 chars".into());
    }
    Ok(object)
}

pub(crate) fn job_id(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 64 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
}

pub(crate) fn render(request: &Map<String, Value>) -> Result<&str, String> {
    let job = job_id(request.get("job_id")).ok_or("job_id must be 1..64 ASCII letters/digits/_/-")?;
    if request.get("revision").and_then(Value::as_u64).is_none() {
        return Err("revision must be a nonnegative integer".into());
    }
    match request.get("route").and_then(Value::as_str) {
        Some("verdant-voice") => {}
        Some("C0" | "C1" | "A") => return Err("retired or restricted render route".into()),
        _ => return Err("test double supports verdant-voice native_score only".into()),
    }
    let payload = request.get("payload").and_then(Value::as_object).ok_or("payload must be an object")?;
    let mut unknown: Vec<_> = unknown_fields(payload, &[
        "kind", "score", "native_seed", "control_track", "expression", "output_transients", "voice_music",
    ]).collect();
    if !unknown.is_empty() {
        unknown.sort_unstable();
        return Err(format!("Unknown payload fields: {}", unknown.join(", ")));
    }
    if payload.get("kind").and_then(Value::as_str) != Some("native_score") {
        return Err("test double accepts native_score payloads only".into());
    }
    if let Some(profile) = payload.get("voice_music") {
        let profile: verdant_backend::wire::VoiceMusic = serde_json::from_value(profile.clone()).map_err(|error| error.to_string())?;
        profile.validate()?;
    }
    if let Some(seed) = payload.get("native_seed") {
        if !seed.as_u64().is_some_and(|value| value < 1 << 31) {
            return Err("native_seed must be an integer in [0,2^31)".into());
        }
    }
    let score = payload.get("score").and_then(Value::as_object);
    let segments = score.and_then(|score| score.get("segments")).and_then(Value::as_array)
        .filter(|segments| (1..=2048).contains(&segments.len())).ok_or("score requires 1..2048 explicit segments")?;
    let maximum = if score.is_some_and(|score| score.contains_key("continuity")) { 120.0 } else { MAX_PHRASE_SECONDS };
    let mut seconds = 0.0;
    for segment in segments {
        let segment = segment.as_object().ok_or("segment must be an object")?;
        if let Some(f0) = segment.get("f0_hz") {
            finite(Some(f0), 35.0, 2000.0, "segment F0")?;
        }
        seconds += finite(segment.get("duration_seconds"), 0.005 - 1e-12, 30.0, "duration_seconds")?;
        seconds += finite(Some(segment.get("gap_after_seconds").unwrap_or(&Value::from(0))), 0.0, 30.0, "gap_after_seconds")?;
    }
    if seconds > maximum + 1e-12 {
        return Err(format!("Split runs longer than {maximum} seconds before rendering"));
    }
    if let Some(track) = payload.get("control_track").filter(|value| !value.is_null()) {
        let track = track.as_object().ok_or("control_track must be an object")?;
        let arrays = ["frame_times_seconds", "f0_hz", "uv"].map(|key| track.get(key).and_then(Value::as_array));
        let [Some(times), Some(f0), Some(uv)] = arrays else {
            return Err("control_track requires equally sized frame_times_seconds, f0_hz and uv arrays".into());
        };
        if !(2..=32_768).contains(&times.len()) || times.len() != f0.len() || times.len() != uv.len() {
            return Err("control_track requires equally sized frame_times_seconds, f0_hz and uv arrays".into());
        }
        let mut previous = -1.0;
        for ((time, hz), unvoiced) in times.iter().zip(f0).zip(uv) {
            let time = finite(Some(time), 0.0, maximum + 1e-12, "frame time")?;
            let hz = finite(Some(hz), 0.0, 2000.0, "F0")?;
            if time <= previous || !matches!(unvoiced.as_bool(), Some(true) if hz == 0.0) && !matches!(unvoiced.as_bool(), Some(false) if hz >= 35.0) {
                return Err("Track times must increase; UV must be bool; UV/F0 must agree".into());
            }
            previous = time;
        }
    }
    Ok(job)
}

fn unknown_fields<'a>(object: &'a Map<String, Value>, allowed: &'a [&str]) -> impl Iterator<Item = &'a str> {
    object.keys().map(String::as_str).filter(move |key| !allowed.contains(key))
}

fn finite(value: Option<&Value>, low: f64, high: f64, name: &str) -> Result<f64, String> {
    value.and_then(Value::as_f64).filter(|value| value.is_finite() && (low..=high).contains(value))
        .ok_or_else(|| format!("{name} must be a finite number in [{low}, {high}]"))
}
