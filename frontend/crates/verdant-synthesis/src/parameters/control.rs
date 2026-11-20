//! Authored control cells and score-note prosody on the shared acoustic clock.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest,Sha256};
use super::timing::{field,finite,TimingPlan};

pub const NATIVE_F0_MIN_HZ: f64 = 35.0;
pub const NATIVE_F0_MAX_HZ: f64 = 2000.0;
pub const NATIVE_F0_RANGE: [f64; 2] = [NATIVE_F0_MIN_HZ, NATIVE_F0_MAX_HZ];
pub const DEFAULT_PORTAMENTO_MS: f64 = 80.0;
const AUTO_GLIDE_NOTE_SHARE: f64 = 0.20;
const VIBRATO_RAMP_SECONDS: f64 = 0.12;

pub(super) fn map_authored_times(times: &[f64], timing: &TimingPlan) -> Result<Vec<f64>> {
    ensure!(timing.render_seconds.len() == timing.authored_seconds.len()
        && timing.render_seconds.len() >= 2, "invalid render-to-authored control map");
    let render = &timing.render_seconds;
    let authored = &timing.authored_seconds;
    ensure!(render[0] == 0.0 && authored[0] == 0.0 && *render.last().context("missing render map")? > 0.0
        && render.iter().chain(authored).all(|v| v.is_finite())
        && render.windows(2).all(|v| v[1] >= v[0]) && authored.windows(2).all(|v| v[1] >= v[0]),
        "invalid render-to-authored control map");
    let mut result = Vec::with_capacity(times.len());
    for &time in times {
        ensure!(time.is_finite(), "control query must be finite");
        let right = render.partition_point(|&v| v <= time);
        ensure!(right > 0 && right < render.len(), "control queries must lie inside the acoustic time map");
        let left = right-1;
        let width = render[right]-render[left];
        ensure!(width > 0.0, "zero-width control map interval");
        result.push(authored[left]+(time-render[left])*(authored[right]-authored[left])/width);
    }
    Ok(result)
}
fn numbers(value: &Value, key: &str) -> Result<Vec<f64>> {
    value.get(key).and_then(Value::as_array).with_context(|| format!("control_track requires {key}"))?
        .iter().map(|v| finite(v,key)).collect()
}
fn interpolation(positions: &[f64], values: &[f64], query: f64) -> f64 {
    if query <= positions[0] { return values[0]; }
    if query >= positions[positions.len()-1] { return values[values.len()-1]; }
    let right = positions.partition_point(|&v| v <= query);
    let left = right-1;
    if positions[left] == query { return values[left]; }
    // np.interp computes the slope first, retaining that rounding order.
    let slope = (values[right]-values[left])/(positions[right]-positions[left]);
    slope*(query-positions[left])+values[left]
}

pub fn sample_authored_control(track: &Value, render_times: &[f64], timing: &TimingPlan,
    max_points: usize, max_seconds: f64) -> Result<(Vec<f64>,Vec<bool>,Value)>
{
    sample_authored_control_in_range(track,render_times,timing,max_points,max_seconds,NATIVE_F0_RANGE)
}

pub(super) fn sample_authored_control_in_range(track: &Value, render_times: &[f64], timing: &TimingPlan,
    max_points: usize, max_seconds: f64, range: [f64; 2]) -> Result<(Vec<f64>,Vec<bool>,Value)>
{
    ensure!(track.is_object(), "control_track must be an object");
    let times = numbers(track,"frame_times_seconds")?;
    let values = numbers(track,"f0_hz")?;
    let uv: Vec<bool> = track.get("uv").and_then(Value::as_array).context("control_track requires uv")?
        .iter().map(|v| v.as_bool().context("control_track UV must be boolean")).collect::<Result<_>>()?;
    ensure!((2..=max_points).contains(&times.len()) && times.len() == values.len() && times.len() == uv.len(),
        "control_track requires aligned arrays within the admitted point limit");
    ensure!(times.iter().all(|&t| t >= 0.0 && t <= max_seconds+1e-12)
        && times.windows(2).all(|v| v[1] > v[0]),
        "control_track requires increasing finite nonnegative times");
    for (&value,&unvoiced) in values.iter().zip(&uv) {
        ensure!(if unvoiced { value == 0.0 } else { (range[0]..=range[1]).contains(&value) },
            "authored control F0 must be {}..{} Hz when voiced and zero when UV",range[0],range[1]);
    }
    let authored = map_authored_times(render_times,timing)?;
    let mut active = vec![false;authored.len()];
    let mut result = vec![0.0;authored.len()];
    let mut island_start = vec![0;times.len()];
    let mut island_end = vec![0;times.len()];
    let mut point = 0;
    while point+1 < times.len() {
        if uv[point] { point += 1; continue; }
        let first = point;
        while point+1 < times.len() && !uv[point] { point += 1; }
        island_start[first..point].fill(first);
        island_end[first..point].fill(point);
    }
    for (index,&time) in authored.iter().enumerate() {
        let right = times.partition_point(|&t| t <= time);
        if right == 0 || right == times.len() || uv[right-1] { continue; }
        active[index] = true;
        let cell = right-1;
        let first = island_start[cell];
        let end = island_end[cell];
        // The island's UV/end sentinel is excluded: no pitch bridge across UV.
        result[index] = interpolation(&times[first..end],&values[first..end],time);
    }
    let canonical = canonical_json(track)?;
    let hash = hex::encode(Sha256::digest(canonical.as_bytes()));
    let metadata = json!({"kind":"authored_control_applied_in_native_synthesis","control_sha256":hash,
        "source_points":times.len(),"control_frame_count":result.len(),
        "time_mapping":"render to authored boundaries, right-continuous across collapsed gaps",
        "interpolation":"linear Hz between authored anchors within each voiced region; no interpolation across UV",
        "voicing_policy":"intersect this validity mask with selected native phoneme/model UV before DSP",
        "native_uv_curve_policy":"native unvoiced consonants gate the authored curve to zero without refitting its neighboring voiced values; authored UV gaps separately prevent interpolation",
        "authored_valid_voiced_frames":active.iter().filter(|&&v| v).count(),
        "native_control_f0_range_hz":range});
    Ok((result,active,metadata))
}

// Match hashlib.sha256(json.dumps(track, sort_keys=True,
// separators=(',', ':')).encode()) including Python float exponent spelling
// and ensure_ascii. This is provenance; serde's different exponent/Unicode
// formatting must not silently change the control digest.
pub(super) fn canonical_json(value: &Value) -> Result<String> {
    fn string(text: &str, out: &mut String) {
        out.push('"');
        for ch in text.chars() {
            match ch {
                '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"),
                '\u{08}' => out.push_str("\\b"), '\u{0c}' => out.push_str("\\f"),
                '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
                '\u{20}'..='\u{7e}' => out.push(ch),
                _ => {
                    let mut buffer = [0u16;2];
                    for &unit in ch.encode_utf16(&mut buffer).iter() { use std::fmt::Write; let _ = write!(out,"\\u{unit:04x}"); }
                }
            }
        }
        out.push('"');
    }
    fn float(value: f64) -> String {
        if value == 0.0 { return if value.is_sign_negative() { "-0.0".into() } else { "0.0".into() }; }
        let negative = value.is_sign_negative();
        let text = value.abs().to_string();
        let (mantissa, explicit) = if let Some(index) = text.find(['e','E']) {
            (&text[..index],text[index+1..].parse::<i32>().expect("valid f64 exponent"))
        } else { (text.as_str(),0) };
        let before = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
        let mut digits: String = mantissa.chars().filter(|&c| c != '.').collect();
        let leading = digits.bytes().take_while(|&c| c == b'0').count();
        digits.drain(..leading);
        let exponent = before-leading as i32-1+explicit;
        while digits.len() > 1 && digits.ends_with('0') { digits.pop(); }
        let mut result = if negative { "-".to_owned() } else { String::new() };
        if !(-4..16).contains(&exponent) {
            result.push(digits.as_bytes()[0] as char);
            if digits.len() > 1 { result.push('.'); result.push_str(&digits[1..]); }
            result.push('e');
            result.push(if exponent >= 0 { '+' } else { '-' });
            result.push_str(&format!("{:02}",exponent.abs()));
        } else {
            let place = exponent+1;
            if place <= 0 {
                result.push_str("0.");
                result.extend(std::iter::repeat('0').take((-place) as usize));
                result.push_str(&digits);
            } else if place as usize >= digits.len() {
                result.push_str(&digits);
                result.extend(std::iter::repeat('0').take(place as usize-digits.len()));
                result.push_str(".0");
            } else {
                result.push_str(&digits[..place as usize]);
                result.push('.');
                result.push_str(&digits[place as usize..]);
            }
        }
        result
    }
    fn append(value: &Value, out: &mut String) -> Result<()> {
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            Value::Number(v) => {
                if v.is_f64() { out.push_str(&float(v.as_f64().context("invalid JSON float")?)); }
                else { out.push_str(&v.to_string()); }
            }
            Value::String(v) => string(v,out),
            Value::Array(values) => {
                out.push('[');
                for (index,value) in values.iter().enumerate() { if index > 0 { out.push(','); } append(value,out)?; }
                out.push(']');
            }
            Value::Object(values) => {
                let mut keys: Vec<&String> = values.keys().collect();
                keys.sort_unstable();
                out.push('{');
                for (index,key) in keys.iter().enumerate() {
                    if index > 0 { out.push(','); }
                    string(key,out); out.push(':'); append(&values[*key],out)?;
                }
                out.push('}');
            }
        }
        Ok(())
    }
    let mut output = String::new();
    append(value,&mut output)?;
    Ok(output)
}

#[derive(Clone)]
struct Vibrato { depth: f64, rate: f64, delay: f64 }
#[derive(Clone)]
struct Note {
    identity: Value, start: f64, end: f64, duration: f64, cents: f64,
    portamento: Option<f64>, positions: Vec<f64>, offsets: Vec<f64>, vibrato: Option<Vibrato>,
}
fn bounded(value: f64, low: f64, high: f64, name: &str) -> Result<f64> {
    ensure!(value.is_finite() && value >= low && value <= high, "{name} outside the supported range");
    Ok(value)
}
fn normalize_notes(notes: &Value) -> Result<Vec<Note>> {
    let mut result = Vec::new();
    let mut last_end = f64::NEG_INFINITY;
    for (index,source) in notes.as_array().context("pitch_notes must be an array")?.iter().enumerate() {
        ensure!(source.is_object(), "pitch note must be an object");
        if source.get("kind").and_then(Value::as_str) == Some("rest") || source.get("is_rest").and_then(Value::as_bool) == Some(true) { continue; }
        let start = bounded(field(source,"start_seconds",None)?,0.0,f64::MAX,"note start")?;
        let duration = bounded(field(source,"duration_seconds",None)?,1e-9,f64::MAX,"note duration")?;
        ensure!(start >= last_end-1e-10, "singing control requires nonoverlapping notes");
        let end = start+duration;
        ensure!(end.is_finite(), "note interval overflow");
        let midi = bounded(field(source,"midi_note",None)?,0.0,127.0,"MIDI pitch")?;
        let detune = match source.get("detune_cents") {
            Some(value) => finite(value,"detune cents")?,
            None => field(source,"detune_millicents",Some(0.0))?/1000.0,
        };
        let cents = midi*100.0+bounded(detune,-2400.0,2400.0,"detune cents")?;
        let portamento = source.get("portamento_ms").filter(|v| !v.is_null())
            .map(|v| bounded(finite(v,"portamento milliseconds")?,0.0,2000.0,"portamento milliseconds")).transpose()?;
        let mut positions = Vec::new();
        let mut offsets = Vec::new();
        if let Some(points) = source.get("pitch_points") {
            for point in points.as_array().context("pitch_points must be an array")? {
                let offset = bounded(field(point,"offset_seconds",None)?,0.0,duration,"PIT offset seconds")?;
                let value = bounded(field(point,"cents",None)?,-2400.0,2400.0,"PIT cents")?;
                ensure!(positions.last().is_none_or(|&previous| offset > previous), "PIT point times must be strictly increasing");
                positions.push(offset); offsets.push(value);
            }
        }
        let vibrato = match source.get("vibrato") {
            None|Some(Value::Null)|Some(Value::Bool(false)) => None,
            Some(value) => {
                let object = value.as_object().context("vibrato must be an object")?;
                if object.is_empty() { None } else {
                    Some(Vibrato { depth:bounded(field(value,"depth_cents",Some(0.0))?,0.0,200.0,"vibrato depth")?,
                        rate:bounded(field(value,"rate_millihertz",Some(5500.0))?,1000.0,12000.0,"vibrato rate")?,
                        delay:bounded(field(value,"delay_permille",Some(350.0))?,0.0,1000.0,"vibrato onset")? })
                }
            }
        };
        result.push(Note { identity:source.get("event_index").or_else(|| source.get("index")).cloned().unwrap_or(json!(index)),
            start,end,duration,cents,portamento,positions,offsets,vibrato });
        last_end = end;
    }
    ensure!(!result.is_empty(), "at least one sounding note is required");
    Ok(result)
}
fn smootherstep(value: f64) -> f64 {
    let x = value.clamp(0.0,1.0);
    x*x*x*(x*(x*6.0-15.0)+10.0)
}
fn manual_curve(note: &Note) -> bool { !note.positions.is_empty() || note.vibrato.as_ref().is_some_and(|v| v.depth > 0.0) }

pub fn plan_note_pitch(notes: &Value, times: &[f64], native_voiced: Option<&[bool]>,
    automatic: bool, default_portamento_ms: f64, manual_f0_hz: Option<&[f64]>) -> Result<(Vec<f64>,Value)>
{
    plan_note_pitch_with_maximum(notes,times,native_voiced,automatic,default_portamento_ms,manual_f0_hz,NATIVE_F0_MAX_HZ)
}

pub(super) fn plan_note_pitch_with_maximum(notes: &Value, times: &[f64], native_voiced: Option<&[bool]>,
    automatic: bool, default_portamento_ms: f64, manual_f0_hz: Option<&[f64]>, maximum_f0_hz: f64) -> Result<(Vec<f64>,Value)>
{
    ensure!(maximum_f0_hz.is_finite() && maximum_f0_hz > 0.0 && maximum_f0_hz <= NATIVE_F0_RANGE[1],
        "unsupported authored pitch ceiling");
    let max_ms = bounded(default_portamento_ms,0.0,2000.0,"automatic portamento limit")?;
    ensure!(!times.is_empty() && times.iter().all(|v| v.is_finite() && *v >= 0.0)
        && times.windows(2).all(|v| v[1] > v[0]),
        "frame times must be a finite strictly increasing nonnegative vector");
    if let Some(mask) = native_voiced { ensure!(mask.len() == times.len(), "voiced must be aligned to frame times"); }
    let normalized = normalize_notes(notes)?;
    let mut active = vec![false;times.len()];
    let mut cents = vec![0.0;times.len()];
    let mut explicit = Vec::new();
    for (index,note) in normalized.iter().enumerate() {
        let previous = index.checked_sub(1).map(|i| &normalized[i]);
        let transition = note.portamento.filter(|&v| v > 0.0).zip(previous)
            .filter(|(_,p)| (p.end-note.start).abs() <= 1e-10);
        if let Some((ms,previous)) = transition {
            let seconds = (ms/1000.0).min(note.duration);
            explicit.push(json!({"from_event":previous.identity,"to_event":note.identity,
                "start_seconds":note.start,"end_seconds":note.start+seconds,"curve":"authored incoming cubic smoothstep"}));
        }
        let first_frame = times.partition_point(|&t| t < note.start);
        let end_frame = times.partition_point(|&t| t < note.end);
        for frame in first_frame..end_frame {
            let time = times[frame];
            active[frame] = true;
            let elapsed = time-note.start;
            let mut value = note.cents;
            if !note.positions.is_empty() { value += interpolation(&note.positions,&note.offsets,elapsed); }
            if let Some((ms,previous)) = transition {
                let seconds = (ms/1000.0).min(note.duration);
                let x = (elapsed/seconds).clamp(0.0,1.0);
                value += (previous.cents-note.cents)*(1.0-x*x*(3.0-2.0*x));
            }
            if let Some(vibrato) = &note.vibrato {
                if vibrato.depth > 0.0 {
                    let onset = note.duration*vibrato.delay/1000.0;
                    let after = (elapsed-onset).max(0.0);
                    let ramp = (after/VIBRATO_RAMP_SECONDS).min(1.0);
                    value += vibrato.depth*ramp*(std::f64::consts::TAU*vibrato.rate/1000.0*after).sin();
                }
            }
            cents[frame] = value;
        }
    }
    let mut transitions = Vec::new();
    if automatic && max_ms > 0.0 && manual_f0_hz.is_none() {
        for pair in normalized.windows(2) {
            let (previous,note) = (&pair[0],&pair[1]);
            if (previous.end-note.start).abs() > 1e-10 || note.portamento.is_some()
                || previous.portamento.is_some_and(|v| v > 0.0) || manual_curve(previous) || manual_curve(note)
                || (previous.cents-note.cents).abs() < 1e-9 { continue; }
            let left = (max_ms/2000.0).min(AUTO_GLIDE_NOTE_SHARE*previous.duration);
            let right = (max_ms/2000.0).min(AUTO_GLIDE_NOTE_SHARE*note.duration);
            let start = note.start-left;
            let end = note.start+right;
            let first_frame = times.partition_point(|&t| t < start);
            let end_frame = times.partition_point(|&t| t <= end);
            for frame in first_frame..end_frame {
                let time = times[frame];
                if active[frame] {
                    cents[frame] = previous.cents+(note.cents-previous.cents)*smootherstep((time-start)/(end-start));
                }
            }
            transitions.push(json!({"from_event":previous.identity,"to_event":note.identity,
                "start_seconds":start,"boundary_seconds":note.start,"end_seconds":end,
                "from_cents":previous.cents,"to_cents":note.cents}));
        }
    }
    if let Some(manual) = manual_f0_hz {
        ensure!(manual.len() == times.len() && manual.iter().all(|&v| v.is_finite() && (0.0..=maximum_f0_hz).contains(&v)),
            "manual F0 must be aligned, finite and in 0..{} Hz",maximum_f0_hz);
        explicit.clear();
    }
    let mut f0 = vec![0.0;times.len()];
    for frame in 0..times.len() {
        if active[frame] && native_voiced.is_none_or(|mask| mask[frame]) {
            f0[frame] = match manual_f0_hz {
                Some(manual) => manual[frame],
                None => 440.0*((cents[frame]/100.0-69.0)/12.0).exp2(),
            };
            ensure!(f0[frame].is_finite() && f0[frame] <= maximum_f0_hz, "planned F0 exceeds the {} Hz control-track limit",maximum_f0_hz);
        }
    }
    let report = json!({"schema":"verdant-singing-prosody/1","mode":if manual_f0_hz.is_some() { "manual-track" } else { "note-plan" },
        "origin":"authored singing policy; not recovered original Founder prosody",
        "automatic_enabled":automatic && manual_f0_hz.is_none(),"automatic_limit_ms":max_ms,
        "automatic_note_share_each_side":AUTO_GLIDE_NOTE_SHARE,"automatic_curve":"quintic smootherstep in cents",
        "automatic_transitions":transitions,"explicit_transitions":explicit,
        "manual_pit_or_vibrato_suppresses_automatic":true,"uv_intersection":"native voiced AND note active AND planned F0 > 0",
        "note_count":normalized.len(),"frames":times.len(),"voiced_frames":f0.iter().filter(|&&v| v != 0.0).count(),
        "jitter_added":false,"default_vibrato_added":false});
    Ok((f0,report))
}

pub(super) fn midi_notes_from_segments(segments: &[Value]) -> Result<Option<Value>> {
    if segments.iter().any(|s| s.get("midi_note").is_none() || s.get("f0_hz").is_some()) { return Ok(None); }
    let mut notes: Vec<Value> = Vec::new();
    let mut cursor = 0.0;
    for segment in segments {
        let duration = field(segment,"duration_seconds",None)?;
        let gap = field(segment,"gap_after_seconds",Some(0.0))?;
        let midi = field(segment,"midi_note",None)?;
        let merge = match notes.last() {
            Some(last) => (field(last,"start_seconds",None)?+field(last,"duration_seconds",None)?-cursor).abs() < 1e-10
                && field(last,"midi_note",None)? == midi,
            None => false,
        };
        if merge {
            let last = notes.last_mut().context("missing inferred MIDI note")?;
            last["duration_seconds"] = json!(field(last,"duration_seconds",None)?+duration);
        } else { notes.push(json!({"start_seconds":cursor,"duration_seconds":duration,"midi_note":midi})); }
        cursor += duration+gap;
    }
    Ok(Some(Value::Array(notes)))
}
