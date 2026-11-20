use std::collections::BTreeSet;
use serde_json::{Map, Value};
use crate::controls::{self, BendPoint, CurveKind, NoteControl, PartCurve, Shape, SourceVibrato};
use crate::{FieldDisposition as D, ImportError, ImportNotice, Result, Source, SourceMeter, SourceNote, SourceTempo, SourceTrack, array, checked_add, field, integer, micro_bpm, number, object, required, retain_except, text};

pub(crate) fn parse(bytes: &[u8]) -> Result<Source> {
    let decoded = std::str::from_utf8(bytes).map_err(|_| ImportError::at("/encoding", "USTX must be valid UTF-8"))?;
    let value = crate::strict_yaml::parse(decoded.strip_prefix('\u{feff}').unwrap_or(decoded))?;
    let root = object(&value, "/")?;
    text(required(root, "ustx_version", "/")?, "/ustx_version")?;
    let ppq = get_int(root, "resolution", 480, 1, "/resolution")?;
    let tempos = parse_tempos(root)?;
    let meters = parse_meters(root, ppq)?;
    let title = root.get("name").map(|v| text(v, "/name")).transpose()?.unwrap_or("Imported USTX").to_string();
    let mut source = Source { ppq, encoding: "utf-8".into(), title, tempos, meters, tracks: Vec::new(), fields: Vec::new(), notices: Vec::new() };
    retain_except(root, &["ustx_version", "resolution", "bpm", "beat_per_bar", "beat_unit", "tempos", "time_signatures", "tracks", "voice_parts", "expressions", "name"], "", &mut source.fields);
    field(&mut source.fields, "/ustx_version", root["ustx_version"].clone(), D::Metadata, None, "Source USTX version.");
    for key in ["name", "resolution", "beat_per_bar", "beat_unit"] {
        if let Some(v) = root.get(key) { field(&mut source.fields, format!("/{key}"), v.clone(), D::Metadata, None, "Source project metadata; explicit time_signatures take precedence over compatibility meter fields."); }
    }
    for (key, admitted) in [("tempos", &["position", "bpm"][..]), ("time_signatures", &["bar_position", "beat_per_bar", "beat_unit"][..])] {
        for (i, value) in optional_array(root, key, key)?.iter().enumerate() {
            let path = format!("/{key}/{i}");
            retain_except(object(value, &path)?, admitted, &path, &mut source.fields);
        }
    }
    if let Some(bpm) = root.get("bpm") {
        field(&mut source.fields, "/bpm", bpm.clone(), D::Metadata, Some("project.tempo_map"), "A nonempty tempos map takes precedence over this compatibility BPM.");
        if root.get("tempos").and_then(Value::as_array).is_some_and(|t| !t.is_empty()) && bpm.as_f64() != Some(source.tempos[0].bpm) {
            source.notices.push(ImportNotice { code: "obsolete_top_level_bpm".into(), location: "/bpm".into(), message: "Nonempty source tempos map is authoritative; the conflicting top-level BPM is retained as metadata.".into() });
        }
    }
    for t in &source.tempos {
        if t.bpm != micro_bpm(t.bpm, "/tempos")? as f64 / 1_000_000.0 {
            source.notices.push(ImportNotice { code: "tempo_quantization".into(), location: "/tempos".into(), message: "Tempo rounded to Verdant micro-BPM; original BPM retained in source_tempos.".into() });
        }
    }
    if let Some(expressions) = root.get("expressions") {
        let expressions = object(expressions, "/expressions")?;
        for (abbr, descriptor) in expressions {
            field(&mut source.fields, format!("/expressions/{abbr}"), descriptor.clone(),
                if matches!(abbr.as_str(), "dyn" | "pitd") { D::Metadata } else { D::RetainedUnsupported }, None,
                "Source expression descriptor retained. Only dyn (0.1 dB) and pitd (cents) curves have admitted source units.");
        }
    }
    let tracks = array(required(root, "tracks", "/")?, "/tracks")?;
    for (index, raw) in tracks.iter().enumerate() {
        let path = format!("/tracks/{index}");
        let map = object(raw, &path)?;
        let name = map.get("track_name").map(|v| text(v, &path)).transpose()?.map(str::to_string).unwrap_or_else(|| format!("Track {}", index + 1));
        let gain_db = get_number(map, "volume", 0.0, &path)?;
        controls::track_gain(gain_db, &path)?;
        let pan = get_number(map, "pan", 0.0, &path)?;
        if !(-100.0..=100.0).contains(&pan) { return Err(ImportError::at(&path, "source track pan must be -100..100 percent")); }
        let muted = get_bool(map, "mute", false, &path)?;
        let solo = get_bool(map, "solo", false, &path)?;
        retain_except(map, &["track_name", "volume", "pan", "mute", "solo"], &path, &mut source.fields);
        for key in ["volume", "pan", "mute", "solo"] {
            if let Some(v) = map.get(key) { field(&mut source.fields, format!("{path}/{key}"), v.clone(), D::Translated, Some("project.tracks"), "Track mixer value translated to domain units (dB to millidecibels, percent pan to permille)."); }
        }
        if map.contains_key("singer") || map.contains_key("phonemizer") || map.contains_key("renderer_settings") {
            source.notices.push(ImportNotice { code: "source_voicebank_not_executed".into(), location: path,
                message: "Source singer, phonemizer and renderer settings are retained; imported tracks use the Verdant project's voice-sn reference. No source voicebank mapping or pronunciation is inferred.".into() });
        }
        source.tracks.push(SourceTrack { index, name, notes: Vec::new(), end_tick: 0, gain_db, pan: (pan * 10.0).round() as i32, muted, solo, curves: Vec::new() });
    }
    for (pi, raw_part) in optional_array(root, "voice_parts", "/voice_parts")?.iter().enumerate() {
        let path = format!("/voice_parts/{pi}");
        let part = object(raw_part, &path)?;
        let ti = usize::try_from(get_int(part, "track_no", 0, 0, &path)?).map_err(|_| ImportError::at(&path, "track index overflow"))?;
        if ti >= source.tracks.len() { return Err(ImportError::at(&path, "voice part references a missing track")); }
        let position = integer(required(part, "position", &path)?, 0, &path)?;
        let duration = get_int(part, "duration", 0, 0, &path)?;
        source.tracks[ti].end_tick = source.tracks[ti].end_tick.max(checked_add(position, duration, &path)?);
        retain_except(part, &["track_no", "position", "duration", "notes", "curves"], &path, &mut source.fields);
        for key in ["track_no", "position", "duration"] {
            if let Some(v) = part.get(key) { field(&mut source.fields, format!("{path}/{key}"), v.clone(), D::Metadata, None, "Original part layout retained while notes use absolute project ticks."); }
        }
        let mut seen_curves = BTreeSet::new();
        for (ci, raw_curve) in optional_array(part, "curves", &path)?.iter().enumerate() {
            let cp = format!("{path}/curves/{ci}");
            let curve = object(raw_curve, &cp)?;
            let abbr = text(required(curve, "abbr", &cp)?, &cp)?;
            if !seen_curves.insert(abbr) { return Err(ImportError::at(&cp, "duplicate part curve abbreviation")); }
            let kind = match abbr { "pitd" => Some(CurveKind::Pitch), "dyn" => Some(CurveKind::Dynamics), _ => None };
            let xs = array(required(curve, "xs", &cp)?, &cp)?.iter().map(|v| integer(v, i64::MIN, &cp)).collect::<Result<Vec<_>>>()?;
            let ys = array(required(curve, "ys", &cp)?, &cp)?.iter().map(|v| integer(v, i64::MIN, &cp).map(|i| i as f64)).collect::<Result<Vec<_>>>()?;
            if xs.len() != ys.len() || xs.windows(2).any(|x| x[0] >= x[1]) { return Err(ImportError::at(&cp, "curve xs/ys must have equal lengths and strictly increasing positions")); }
            let descriptor = root.get("expressions").and_then(|v| v.get(abbr));
            let compatible = descriptor.map_or(true, |d| d.get("type").and_then(Value::as_str) == Some("Curve")
                && d.get("abbr").and_then(Value::as_str).is_none_or(|a| a == abbr));
            let admitted = kind.is_some() && compatible;
            controls::record_control(&mut source.fields, &cp, raw_curve.clone(), admitted,
                if kind == Some(CurveKind::Pitch) { "note.curves.pitch_millicent_points" } else { "note.curves.gain_millidecibel_points" });
            retain_except(curve, &["abbr", "xs", "ys"], &cp, &mut source.fields);
            if admitted && !xs.is_empty() && ys.iter().any(|y| *y != 0.0) {
                let default = descriptor.and_then(|d| d.get("default_value")).map(|v| number(v, &cp)).transpose()?.unwrap_or(0.0);
                source.tracks[ti].curves.push(PartCurve { part: pi, start: position, kind: kind.unwrap(), xs, ys, default });
            }
        }
        for (ni, raw_note) in optional_array(part, "notes", &path)?.iter().enumerate() {
            let np = format!("t{ti}/p{pi}/n{ni}");
            let n = object(raw_note, &np)?;
            let start = checked_add(position, integer(required(n, "position", &np)?, 0, &np)?, &np)?;
            let duration = integer(required(n, "duration", &np)?, 1, &np)?;
            let midi = integer(required(n, "tone", &np)?, 0, &np)?;
            if midi > 127 { return Err(ImportError::at(&np, "MIDI note exceeds 127")); }
            let lyric = text(required(n, "lyric", &np)?, &np)?.to_string();
            let mut control = NoteControl::default();
            if let Some(tuning) = n.get("tuning") {
                control.tuning_cents = integer(tuning, i64::MIN, &np)? as f64;
                controls::record_control(&mut source.fields, &format!("{np}/tuning"), tuning.clone(), true, "note.curves.pitch_millicent_points");
            }
            if let Some(pitch) = n.get("pitch") {
                let pp = format!("{np}/pitch");
                let pitch_map = object(pitch, &pp)?;
                control.snap_first = get_bool(pitch_map, "snap_first", true, &pp)?;
                let mut supported = true;
                let mut points: Vec<BendPoint> = Vec::new();
                for (i, raw_point) in optional_array(pitch_map, "data", &pp)?.iter().enumerate() {
                    let point_path = format!("{pp}/data/{i}");
                    let p = object(raw_point, &point_path)?;
                    let ms = number(required(p, "x", &point_path)?, &point_path)?;
                    let cents = number(required(p, "y", &point_path)?, &point_path)? * 10.0;
                    let shape = p.get("shape").map(|v| text(v, &point_path)).transpose()?.unwrap_or("io");
                    let shape = Shape::parse(shape);
                    supported &= shape.is_some();
                    if points.last().is_some_and(|p| p.ms > ms) { return Err(ImportError::at(&point_path, "pitch point times must not decrease")); }
                    points.push(BendPoint { ms, cents, shape: shape.unwrap_or(Shape::InOut) });
                    retain_except(p, &["x", "y", "shape"], &point_path, &mut source.fields);
                }
                if supported && !points.is_empty() { control.pitch = Some(points); }
                controls::record_control(&mut source.fields, &pp, pitch.clone(), supported, "note.curves.pitch_millicent_points");
                retain_except(pitch_map, &["data", "snap_first"], &pp, &mut source.fields);
            }
            if let Some(raw_vibrato) = n.get("vibrato") {
                let vp = format!("{np}/vibrato");
                let v = object(raw_vibrato, &vp)?;
                control.vibrato = Some(SourceVibrato { length: get_number(v, "length", 0.0, &vp)?, period_ms: get_number(v, "period", 175.0, &vp)?,
                    depth_cents: get_number(v, "depth", 25.0, &vp)?, fade_in: get_number(v, "in", 10.0, &vp)?, fade_out: get_number(v, "out", 10.0, &vp)?,
                    shift: get_number(v, "shift", 0.0, &vp)?, drift: get_number(v, "drift", 0.0, &vp)?, volume_link: get_number(v, "vol_link", 0.0, &vp)? }.validate(&vp)?);
                controls::record_control(&mut source.fields, &vp, raw_vibrato.clone(), true, "note.curves.pitch_millicent_points + gain_millidecibel_points");
                retain_except(v, &["length", "period", "depth", "in", "out", "shift", "drift", "vol_link"], &vp, &mut source.fields);
            }
            retain_except(n, &["position", "duration", "tone", "lyric", "tuning", "pitch", "vibrato"], &np, &mut source.fields);
            let track = &mut source.tracks[ti];
            track.end_tick = track.end_tick.max(checked_add(start, duration, &np)?);
            let has_alias = n.get("phoneme_overrides").and_then(Value::as_array).is_some_and(|overrides|
                overrides.iter().any(|o| o.get("phoneme").and_then(Value::as_str).is_some_and(|p| !p.is_empty())));
            let source_reading = if has_alias || lyric.starts_with('?') || (lyric.contains('[') && lyric.contains(']')) {
                crate::SourceReadingEvidence::VoicebankPhonemeAlias
            } else { crate::SourceReadingEvidence::LyricOnly };
            track.notes.push(SourceNote { part: pi, index: ni, start, duration, midi: midi as i32, lyric, source_reading, control });
        }
    }
    Ok(source)
}

fn parse_tempos(root: &Map<String, Value>) -> Result<Vec<SourceTempo>> {
    let raw = optional_array(root, "tempos", "/tempos")?;
    let mut tempos = Vec::new();
    if raw.is_empty() {
        let bpm = number(required(root, "bpm", "/bpm")?, "/bpm")?;
        micro_bpm(bpm, "/bpm")?;
        return Ok(vec![SourceTempo { tick: 0, bpm }]);
    }
    for (i, item) in raw.iter().enumerate() {
        let path = format!("/tempos/{i}");
        let map = object(item, &path)?;
        let tick = integer(required(map, "position", &path)?, 0, &path)?;
        let bpm = number(required(map, "bpm", &path)?, &path)?;
        micro_bpm(bpm, &path)?;
        if (i == 0 && tick != 0) || tempos.last().is_some_and(|t: &SourceTempo| t.tick >= tick) {
            return Err(ImportError::at(&path, "tempo map must start at 0 and increase strictly"));
        }
        tempos.push(SourceTempo { tick, bpm });
    }
    Ok(tempos)
}
fn parse_meters(root: &Map<String, Value>, ppq: i64) -> Result<Vec<SourceMeter>> {
    let raw = optional_array(root, "time_signatures", "/time_signatures")?;
    let mut meters = Vec::with_capacity(raw.len().max(1));
    let mut previous_bar = 0;
    if raw.is_empty() {
        crate::push_meter_from_bar(&mut meters, &mut previous_bar, ppq, 0,
            get_int(root, "beat_per_bar", 4, 1, "/beat_per_bar")?,
            get_int(root, "beat_unit", 4, 1, "/beat_unit")?, "/time_signatures")?;
    } else {
        for (i, item) in raw.iter().enumerate() {
            let path = format!("/time_signatures/{i}");
            let map = object(item, &path)?;
            crate::push_meter_from_bar(&mut meters, &mut previous_bar, ppq,
                integer(required(map, "bar_position", &path)?, 0, &path)?,
                integer(required(map, "beat_per_bar", &path)?, 1, &path)?,
                integer(required(map, "beat_unit", &path)?, 1, &path)?, &path)?;
        }
    }
    Ok(meters)
}
fn optional_array<'a>(map: &'a Map<String, Value>, key: &str, path: &str) -> Result<&'a [Value]> { map.get(key).map(|v| array(v, path)).unwrap_or(Ok(&[])) }
fn get_number(map: &Map<String, Value>, key: &str, default: f64, path: &str) -> Result<f64> { map.get(key).map(|v| number(v, path)).unwrap_or(Ok(default)) }
fn get_int(map: &Map<String, Value>, key: &str, default: i64, minimum: i64, path: &str) -> Result<i64> { map.get(key).map(|v| integer(v, minimum, path)).unwrap_or(Ok(default)) }
fn get_bool(map: &Map<String, Value>, key: &str, default: bool, path: &str) -> Result<bool> {
    map.get(key).map(|v| v.as_bool().ok_or_else(|| ImportError::at(path, format!("{key} must be boolean")))).unwrap_or(Ok(default))
}
