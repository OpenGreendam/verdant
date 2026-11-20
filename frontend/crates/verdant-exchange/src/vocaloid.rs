//! VPR is a ZIP container containing Project/sequence.json. Its musical
//! resolution is 480 ticks/quarter; no external member is ever extracted.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use serde_json::{Map, Value, json};
use verdant_domain::{Note, Project};
use verdant_domain::model::TempoPoint;
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::SimpleFileOptions};
use crate::{ExchangeError as E, ExchangeReport, ImportOptions, ExportOptions, Disposition as D, Result};
use crate::common::*;
const KEY: &str = "exchange_vpr";
const SEQUENCE: &str = "Project/sequence.json";
const CONTROLLER_MAX: i64 = 127;
const DYN_NEUTRAL: i64 = 64;
const DYN_GAIN_MIN: i64 = -60_000;
const DYN_GAIN_MAX: i64 = 12_000;
const PIT_SCALE: f64 = 8192.0;
const MILLICENTS_PER_SEMITONE: f64 = 100_000.0;
const EXPRESSION: [(&str, &str); 7] = [
    ("breathiness", "BRE"), ("brightness", "BRI"), ("clearness", "CLE"),
    ("gender", "GEN"), ("portamento", "POR"), ("opening", "OPE"), ("growl", "GWL"),
];

fn expression_default(lane: &str) -> i64 {
    match lane { "BRE" | "GWL" => 0, "OPE" => CONTROLLER_MAX, _ => 64 }
}

fn controller_name(name: &str) -> &str {
    match name { "DYN" | "dynamics" => "dyn", _ => name }
}

// A two-segment linear score conversion, anchored at 0, 64 and 127.
fn dynamics_gain(value: f64) -> f64 {
    let delta = value - DYN_NEUTRAL as f64;
    if delta < 0.0 { delta * (-DYN_GAIN_MIN) as f64 / DYN_NEUTRAL as f64 }
    else { delta * DYN_GAIN_MAX as f64 / (CONTROLLER_MAX - DYN_NEUTRAL) as f64 }
}

fn gain_dynamics(gain: i64) -> f64 {
    let delta = if gain < 0 { gain as f64 * DYN_NEUTRAL as f64 / (-DYN_GAIN_MIN) as f64 }
    else { gain as f64 * (CONTROLLER_MAX - DYN_NEUTRAL) as f64 / DYN_GAIN_MAX as f64 };
    DYN_NEUTRAL as f64 + delta
}

fn pitch_millicents(pit: f64, sensitivity: f64) -> f64 {
    pit / PIT_SCALE * sensitivity * MILLICENTS_PER_SEMITONE
}

fn member_name(name: &str, directory: bool) -> Result<()> {
    if name.is_empty() || name.starts_with('/') || name.contains(['\\', '\0', ':']) || name.bytes().any(|c| c < 32)
        || name.trim_end_matches('/').split('/').any(|p| p.is_empty() || p == "." || p == "..")
        || (name.ends_with('/') && !directory) { return Err(E::at("/archive", format!("unsafe archive member {name:?}"))); }
    Ok(())
}
fn read_archive(bytes: &[u8], options: &ImportOptions) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    if archive.len() > options.limits.max_archive_members { return Err(E::at("/archive", "member limit exceeded")); }
    let mut total = 0u64; let mut names = BTreeSet::new(); let mut members = BTreeMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?; let name = file.name().to_string(); member_name(&name, file.is_dir())?;
        if std::str::from_utf8(file.name_raw()).is_err() || file.encrypted() || file.is_symlink() || file.enclosed_name().is_none()
            || !matches!(file.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            return Err(E::at(format!("/archive/{name}"), "encrypted, symlink, unsupported compression or non-UTF-8 member"));
        }
        if !names.insert(name.trim_end_matches('/').to_ascii_lowercase()) { return Err(E::at("/archive", "duplicate/case-colliding member name")); }
        total = total.checked_add(file.size()).ok_or_else(|| E::at("/archive", "uncompressed length overflow"))?;
        if total > options.limits.max_archive_bytes || file.size() > options.limits.max_file_bytes as u64 { return Err(E::at("/archive", "uncompressed byte limit exceeded")); }
        let expected = file.size(); let mut data = Vec::new();
        (&mut file).take(expected.checked_add(1).ok_or_else(||E::at("/archive","member length overflow"))?).read_to_end(&mut data)?;
        if data.len() as u64 != expected { return Err(E::at(format!("/archive/{name}"), "member actual length differs from declared size")); }
        if !file.is_dir() { members.insert(name, data); }
    }
    Ok(members)
}
pub(crate) fn import_vpr(bytes: &[u8], options: &ImportOptions, report: &mut ExchangeReport) -> Result<Project> {
    let mut members = read_archive(bytes, options)?;
    let bytes = members.remove(SEQUENCE).ok_or_else(|| E::at("/archive", "missing exact Project/sequence.json member"))?;
    let value = parse_json(&bytes, false)?; let root = object(&value, "/")?;
    let mut project = source_project(480); project.tempo_map.clear(); report.source_ppq = 480;
    let master = object(required(root, "masterTrack", "")?, "/masterTrack")?;
    let tempo = object(required(master, "tempo", "/masterTrack")?, "/masterTrack/tempo")?;
    let mut global = None;
    if let Some(v) = tempo.get("global") {
        let v = object(v, "/masterTrack/tempo/global")?;
        if boolean(v, "isEnabled", true, "/masterTrack/tempo/global")? { global = Some(number(required(v, "value", "/masterTrack/tempo/global")?, "/masterTrack/tempo/global/value")?); }
    }
    if let Some(v) = global {
        project.tempo_map.push(TempoPoint { tick: 0, micro_bpm: micro_bpm(v / 100.0, "/masterTrack/tempo/global", report)?, unknown: BTreeMap::new() });
        if let Some(events) = tempo.get("events") { if !array(events, "/masterTrack/tempo/events")?.is_empty() { report.issue("/masterTrack/tempo/events", D::RetainedUnsupported, events.clone(), "Enabled global tempo overrides stored tempo events."); } }
    } else if let Some(events) = tempo.get("events") {
        for (i, t) in array(events, "/masterTrack/tempo/events")?.iter().enumerate() {
            let path = format!("/masterTrack/tempo/events/{i}"); let t = object(t, &path)?;
            project.tempo_map.push(TempoPoint { tick: int(t, "pos", 0, &path)?, micro_bpm: micro_bpm(number(required(t, "value", &path)?, &path)? / 100.0, &path, report)?,
                unknown: retain(t, &["pos", "value"], &path, report).into_iter().collect() });
        }
    }
    let signature = object(required(master, "timeSig", "/masterTrack")?, "/masterTrack/timeSig")?; let mut bars = Vec::new(); let mut meter_extras=BTreeMap::new();
    for (i, raw) in array(required(signature, "events", "/masterTrack/timeSig")?, "/masterTrack/timeSig/events")?.iter().enumerate() {
        let path = format!("/masterTrack/timeSig/events/{i}"); let m = object(raw, &path)?;
        bars.push((int(m, "bar", 0, &path)?, bounded(int(m, "numer", 4, &path)?, 1, 65535, &path)? as u32,
            bounded(int(m, "denom", 4, &path)?, 1, 65536, &path)? as u32));
        meter_extras.insert(int(m,"bar",0,&path)?,retain(m, &["bar", "numer", "denom"], &path, report));
    }
    project.time_signatures = meters_from_bars(bars.clone(), 480)?;
    bars.sort_by_key(|m|m.0);for(m,(bar,_,_))in project.time_signatures.iter_mut().zip(bars){if let Some(extra)=meter_extras.remove(&bar){m.unknown=extra.into_iter().collect();}}
    default_maps(&mut project);
    let tracks = array(required(root, "tracks", "")?, "/tracks")?;
    if tracks.len() > options.limits.max_tracks { return Err(E::at("/tracks", "track limit exceeded")); }
    let mut retained_tracks = Vec::new(); let mut total = 0usize;
    for (source_ti, raw_track) in tracks.iter().enumerate() {
        let path = format!("/tracks/{source_ti}"); let t = object(raw_track, &path)?;
        let kind = int(t, "type", 0, &path)?;
        if !matches!(kind, 0 | 2) {
            report.issue(&path, D::RetainedUnsupported, raw_track.clone(), "Audio/unknown VPR track retained without reading audio or creating a fake singing track.");
            retained_tracks.push(json!({"index":source_ti,"value":raw_track})); continue;
        }
        let ti = project.tracks.len(); let mut track = source_track(string(t, "name", "Track", &path)?);
        track.unknown.insert("vpr_source_index".into(), source_ti.into());
        track.muted = boolean(t, "isMuted", false, &path)?; track.solo = boolean(t, "isSoloMode", false, &path)?;
        let unknown = retain(t, &["name", "parts", "isMuted", "isSoloMode"], &path, report);
        save_unknown(&mut track.unknown, KEY, unknown, options.preserve_extensions); project.tracks.push(track);
        for (pi, raw_part) in array(required(t, "parts", &path)?, &path)?.iter().enumerate() {
            let pp = format!("{path}/parts/{pi}"); let part = object(raw_part, &pp)?;
            let start = int(part, "pos", 0, &pp)?; let duration = integer(required(part, "duration", &pp)?, &pp)?;
            end(start, duration, &pp)?;
            let language = part.get("voice").or_else(|| part.get("aiVoice")).and_then(|v| v.get("langID")).and_then(Value::as_i64).and_then(language);
            let mut notes = Vec::new();
            for (ni, raw_note) in array(required(part, "notes", &pp)?, &pp)?.iter().enumerate() {
                total += 1; if total > options.limits.max_notes { return Err(E::at(&pp, "note limit exceeded")); }
                let np = format!("{pp}/notes/{ni}"); let map = object(raw_note, &np)?;
                let mut n = note(int(map, "pos", 0, &np)?, integer(required(map, "duration", &np)?, &np)?,
                    integer(required(map, "number", &np)?, &np)?, string(map, "lyric", "", &np)?, &np)?;
                let raw_velocity = map.get("velocity").cloned().unwrap_or(json!(64));
                let v = if let Some(v) = raw_velocity.as_i64() { v } else {
                    let v = number(&raw_velocity, &np)?; if !(0.0..=1.0).contains(&v) { return Err(E::at(&np, "fractional VPR velocity must be normalized 0..1")); }
                    report.issue(format!("{np}/velocity"), D::Quantized, raw_velocity.clone(), "Normalized VPR velocity mapped to the editable integer 0..127 field."); (v * 127.0).round() as i64
                };
                set_velocity(&mut n, v, &np)?;
                let lang = map.get("langID").and_then(Value::as_i64).and_then(self::language).or(language);
                phonemes(&mut n, &string(map, "phoneme", "", &np)?, lang, "vocaloid-xsampa");
                let mut unknown = retain(map, &["pos", "duration", "number", "lyric", "phoneme", "velocity"], &np, report);
                unknown.insert("source_velocity".into(), json!({"value":raw_velocity,"editable":v}));
                save_unknown(&mut n.unknown, KEY, unknown, options.preserve_extensions); notes.push(n);
            }
            let controllers = part.get("controllers").cloned().unwrap_or_else(|| json!([]));
            let curves = parse_controllers(&controllers, &format!("{pp}/controllers"), options, report)?;
            if array(&controllers,&pp)?.iter().filter_map(|c|c.get("events").and_then(Value::as_array)).flatten().any(|e|e.get("pos").and_then(Value::as_i64).is_some_and(|t|t>duration)) { return Err(E::at(&pp,"controller event extends beyond declared part")); }
            apply_controllers(&mut notes, &curves, &pp, report)?;
            let extra = retain(part, &["pos", "duration", "name", "notes", "controllers"], &pp, report);
            let metadata = if options.preserve_extensions { json!({"format":"vpr","extra":extra,"controllers":controllers,"controller_ppq":480,"curve_snapshot":curve_snapshot(notes.iter(),0)}).as_object().expect("metadata").clone() } else { Map::new() };
            add_part(&mut project, ti, format!("vpr:t{source_ti}:p{pi}"), string(part, "name", "Part", &pp)?, start, duration, notes, metadata)?;
        }
    }
    let root_extra = retain(root, &["masterTrack", "tracks"], "", report);
    let master_extra = retain(master, &["tempo", "timeSig"], "/masterTrack", report);
    let tempo_extra = retain(tempo, &["events", "global"], "/masterTrack/tempo", report);
    let meter_extra = retain(signature, &["events"], "/masterTrack/timeSig", report);
    if options.preserve_extensions {
        let member_values: Vec<_> = members.into_iter().map(|(name, data)| {
            report.issue(format!("/archive/{name}"), D::RetainedUnsupported, data.len(), "Archive member retained byte-for-byte without extraction or execution."); json!({"name":name,"hex":hex::encode(data)})
        }).collect();
        project.extensions.insert(KEY.into(), json!({"root":root_extra,"master":master_extra,"tempo":tempo_extra,"meter":meter_extra,"tracks":retained_tracks,"members":member_values,"tempo_source":tempo,"tempo_snapshot":project.tempo_map.iter().map(|t|(t.tick,t.micro_bpm)).collect::<Vec<_>>()}));
    } else { for (name, data) in members { report.issue(format!("/archive/{name}"), D::Unsupported, data.len(), "Caller disabled archive extension retention."); } }
    report.issue("/synthesis", D::Metadata, "VOCALOID", "VPR score data does not install, license or emulate VOCALOID or its voices. Select an installed destination voice before rendering.");
    Ok(project)
}

pub(crate) fn language(id: i64) -> Option<&'static str> {
    match id { 0 => Some("ja"), 1 => Some("en"), 2 => Some("ko"), 3 => Some("es"), 4 => Some("zh"), _ => None }
}
pub(crate) fn language_id(lang: Option<&str>) -> Option<i64> {
    match lang { Some("ja" | "japanese") => Some(0), Some("en" | "english") => Some(1), Some("ko" | "korean") => Some(2), Some("es" | "spanish") => Some(3), Some("zh" | "mandarin") => Some(4), _ => None }
}
pub(crate) type Controllers = BTreeMap<String, Vec<(i64, f64)>>;
pub(crate) fn parse_controllers(value: &Value, path: &str, options: &ImportOptions, report: &mut ExchangeReport) -> Result<Controllers> {
    let mut result = BTreeMap::new(); let mut total = 0usize;
    for (ci, value) in array(value, path)?.iter().enumerate() {
        let path = format!("{path}/{ci}"); let map = object(value, &path)?; let key = string(map, "name", "", &path)?;
        let key = controller_name(&key);
        if key.trim().is_empty() { return Err(E::at(&path, "controller name must not be empty")); }
        if result.contains_key(key) { return Err(E::at(&path, "duplicate controller name")); }
        let mut points = Vec::new(); let mut previous_tick=None;
        for (ei, e) in array(required(map, "events", &path)?, &path)?.iter().enumerate() {
            total += 1; if total > options.limits.max_events { return Err(E::at(&path, "controller event limit")); }
            let e = object(e, &path)?; let tick = int(e, "pos", 0, &path)?;
            bounded(tick, 0, MAX_TICK, &path)?;
            if previous_tick.is_some_and(|previous|previous>=tick) { return Err(E::at(&path,"duplicate or decreasing controller tick")); } previous_tick=Some(tick);
            let Some(value) = e.get("value").and_then(Value::as_f64) else {
                if e.get("value").is_some_and(|v|!v.is_null()&&v.as_str()!=Some("ZeroPitch")) { return Err(E::at(&path,"controller value must be numeric, null or ZeroPitch")); }
                report.issue(format!("{path}/events/{ei}"), D::RetainedUnsupported, Value::Object(e.clone()), "Non-numeric controller sentinel has no admitted editable value."); continue;
            };
            bounded(tick, 0, MAX_TICK, &path)?;
            if !value.is_finite() || points.last().is_some_and(|p: &(i64, f64)| p.0 >= tick) { return Err(E::at(&path, "controller values must be finite with strictly increasing ticks")); }
            points.push((tick, value)); retain(e, &["pos", "value"], &path, report);
        }
        retain(map, &["name", "events"], &path, report); result.insert(key.to_owned(), points);
    } Ok(result)
}
pub(crate) fn apply_controllers(notes: &mut [Note], controllers: &Controllers, path: &str, report: &mut ExchangeReport) -> Result<()> {
    let mut pit = vec![(0, 0.0)]; let mut sensitivity = vec![(0, 2.0)];
    if let Some(v) = controllers.get("pitchBend") { pit = v.clone(); }
    if let Some(v) = controllers.get("pitchBendSens") { sensitivity = v.clone(); }
    if controllers.contains_key("pitchBend") || controllers.contains_key("pitchBendSens") {
        if pit.iter().any(|(_, value)| !(-PIT_SCALE..=PIT_SCALE - 1.0).contains(value))
            || sensitivity.iter().any(|(_, value)| !(0.0..=CONTROLLER_MAX as f64).contains(value)) {
            return Err(E::at(path, "PIT/PBS controller out of range"));
        }
        let times: BTreeSet<_> = pit.iter().chain(&sensitivity).map(|p| p.0).collect(); let mut points = Vec::new();
        let mut pit = pit.into_iter().peekable();
        let mut sensitivity = sensitivity.into_iter().peekable();
        let (mut p, mut s) = (0.0, 2.0);
        for t in times {
            if let Some((_, value)) = pit.next_if(|(tick, _)| *tick == t) { p = value; }
            if let Some((_, value)) = sensitivity.next_if(|(tick, _)| *tick == t) { s = value; }
            let value = pitch_millicents(p, s);
            if value != value.round() { report.issue(format!("{path}/pitch/{t}"), D::Quantized, value, "Pitch bend rounded to integer millicents."); }
            points.push((t, value));
        } apply_curve(notes, &points, Curve::Pitch, false, path)?;
    }
    for (name, points) in controllers {
        let expression = EXPRESSION.iter().find(|(controller, _)| *controller == name.as_str()).map(|(_, lane)| *lane);
        if name == "dyn" || expression.is_some() {
            let controller_path = format!("{path}/controllers/{name}");
            if points.iter().any(|(_, value)| !(0.0..=CONTROLLER_MAX as f64).contains(value) || value.fract() != 0.0) {
                return Err(E::at(&controller_path, "VOCALOID controller must be an integer in 0..127"));
            }
            let (kind, default) = if let Some(lane) = expression {
                (Curve::Expression(lane.into()), expression_default(lane) as f64)
            } else { (Curve::Gain, 0.0) };
            let mut nodes = Vec::with_capacity(points.len() + 1);
            if points.first().is_some_and(|p| p.0 > 0) { nodes.push((0, default)); }
            for &(tick, value) in points {
                let mapped = if name == "dyn" { dynamics_gain(value) } else { value };
                if mapped != mapped.round() {
                    report.issue(format!("{controller_path}/{tick}"), D::Quantized, value, "DYN converted to the nearest whole millidecibel.");
                }
                nodes.push((tick, mapped));
            }
            apply_curve(notes, &nodes, kind, false, &controller_path)?;
        } else if !matches!(name.as_str(), "pitchBend" | "pitchBendSens") {
            let detail = if matches!(name.as_str(), "vel" | "velocity" | "acc" | "accent") {
                "Part-level velocity/accent is retained as source data without assigning it to note velocities or gain."
            } else { "Controller retained without a proprietary synthesis or timbre mapping." };
            report.issue(format!("{path}/controllers/{name}"), D::RetainedUnsupported, json!(points), detail);
        }
    } Ok(())
}

fn retained_controller(value: &Value, from: i64, to: i64, path: &str) -> Result<Value> {
    let mut value = value.clone();
    if let Some(events) = value.get_mut("events").and_then(Value::as_array_mut) {
        for event in events {
            let map = event.as_object_mut().ok_or_else(|| E::at(path, "invalid source controller event"))?;
            let tick = integer(required(map, "pos", path)?, path)?;
            map.insert("pos".into(), scale(tick, from, to, path)?.into());
        }
    }
    Ok(value)
}

fn controller_unchanged(source: &Value, current: &Value, key: &str) -> bool {
    let (Some(source), Some(current)) = (source.as_array(), current.as_array()) else { return false; };
    if source.len() != current.len() { return false; }
    source.iter().zip(current).all(|(old, new)| {
        if ["start", "duration", "key"].iter().any(|field| old[*field] != new[*field]) { return false; }
        let same_curve = match key {
            "PIT" => old["pitch"] == new["pitch"] && old["detune"] == new["detune"],
            "DYN" => old["gain"] == new["gain"],
            _ => old["expression"][key] == new["expression"][key],
        };
        same_curve && old["interpolation"]["interpolation"][key] == new["interpolation"]["interpolation"][key]
    })
}

pub(crate) fn exported_controllers(part: &ExportPart<'_>, ppq: i64, target_ppq: i64, preserve_extensions: bool, report: &mut ExchangeReport, path: &str) -> Result<Vec<Value>> {
    let compatible = preserve_extensions && matches!((report.format, part.metadata.get("format").and_then(Value::as_str)),
        (crate::Format::Vpr, Some("vpr")) | (crate::Format::Vsqx, Some("vsqx")));
    let mut result = Vec::new();
    let mut retained = BTreeSet::new();
    if let Some(old) = part.metadata.get("controllers").and_then(Value::as_array) {
        let from = part.metadata.get("controller_ppq").and_then(Value::as_i64).unwrap_or(ppq);
        let current = compatible.then(|| curve_snapshot(part.notes.iter().copied(), part.start));
        let source = part.metadata.get("curve_snapshot");
        if current.as_ref().is_some_and(|current| source == Some(current)) {
            return old.iter().map(|value| retained_controller(value, from, target_ppq, path)).collect();
        }
        for old in old {
            let name = old.get("name").and_then(Value::as_str).unwrap_or("");
            let lane = match controller_name(name) {
                "pitchBend" | "pitchBendSens" => Some("PIT"),
                "dyn" => Some("DYN"),
                _ => EXPRESSION.iter().find(|(controller, _)| *controller == name).map(|(_, lane)| *lane),
            };
            if compatible && lane.is_none_or(|lane| source.zip(current.as_ref()).is_some_and(|(source, current)| controller_unchanged(source, current, lane))) {
                result.push(retained_controller(old, from, target_ppq, path)?);
                if let Some(lane) = lane { retained.insert(lane); }
            } else if lane.is_none() {
                report.issue(format!("{path}/controllers/{name}"), D::Unsupported, old.clone(), "Source-only controller cannot be written with the selected destination or retention options.");
            } else if preserve_extensions {
                report.issue(format!("{path}/controllers/{name}/source"), D::Unsupported, old.clone(), "Original controller data is replaced by the editable score curve. Off-note nodes and source event metadata may be lost.");
            }
        }
    }
    let edit_dynamics = !retained.contains("DYN") && part.notes.iter().any(|note| !note.curves.gain_millidecibel_points.is_empty());
    let edited_expression = EXPRESSION.iter().filter(|(_, lane)| !retained.contains(lane)
        && part.notes.iter().any(|note| !note.curves.expression(lane).is_empty())).copied().collect::<Vec<_>>();
    let starts: BTreeSet<_> = part.notes.iter().map(|note| note.start_tick).collect();
    let mut controls = BTreeMap::<String, BTreeMap<i64, i64>>::new(); let mut pitch = BTreeMap::new();
    for n in &part.notes {
        let start = scale(n.start_tick - part.start, ppq, target_ppq, path)?;
        let local_tick = |tick| scale(n.start_tick - part.start + tick, ppq, target_ppq, path);
        let shared_end = starts.contains(&n.end_tick());
        if !retained.contains("PIT") {
            let offset = i64::from(n.detune_millicents);
            let points = &n.curves.pitch_millicent_points;
            let with_detune = |value: i64| value.checked_add(offset).ok_or_else(|| E::at(path, "pitch overflow"));
            if points.first().is_none_or(|point| point.tick != 0) {
                insert(&mut pitch, start, with_detune(points.first().map_or(0, |point| point.millicents))?, path)?;
            }
            for (i, p) in points.iter().enumerate() {
                if shared_end && p.tick == n.duration_tick {
                    if i > 0 && points[i-1].millicents != p.millicents { report.issue(format!("{path}/pitch/end"), D::Unsupported, p.millicents, "The following note owns the shared boundary tick."); }
                    continue;
                }
                insert(&mut pitch, local_tick(p.tick)?, with_detune(p.millicents)?, path)?;
            }
            if n.curves.interpolation("PIT") != verdant_domain::editing::Interpolation::Hold && n.curves.pitch_millicent_points.len() > 1 { report.issue(format!("{path}/pitch/interpolation"), D::Unsupported, json!(n.curves.interpolation("PIT")), "VOCALOID pitch export retains authored nodes with hold semantics."); }
        }
        for (key, points) in &n.curves.expression_points {
            if !EXPRESSION.iter().any(|(_, lane)| *lane == key.as_str()) {
                report.issue(format!("{path}/curves/{key}"), D::Unsupported, serde_json::to_value(points)?, "Unsupported destination expression.");
            }
        }
        for &(name, key) in &edited_expression {
            let points = n.curves.expression(key);
            let values = controls.entry(name.into()).or_default();
            let default = verdant_domain::model::expression_default(key).unwrap_or(0);
            let leading = points.first().filter(|point| point.tick != 0).map(|point| point.value);
            let start_node = if points.is_empty() { Some(default) } else { leading };
            for (tick, raw) in start_node.into_iter().map(|value| (0, value)).chain(points.iter().map(|p| (p.tick, p.value))) {
                if shared_end && tick == n.duration_tick {
                    if points.len() > 1 && points[points.len()-2].value != raw { report.issue(format!("{path}/curves/{key}/end"), D::Unsupported, raw, "The following note owns the shared boundary tick."); }
                    continue;
                }
                let tick = local_tick(tick)?;
                let value = i64::from(raw).min(CONTROLLER_MAX);
                if value != i64::from(raw) {
                    report.issue(format!("{path}/curves/{key}/{tick}"), D::Quantized, raw, "VOCALOID expression clamped to the integer range 0..127.");
                }
                insert(values, tick, value, path)?;
            }
            if n.curves.interpolation(key) != verdant_domain::editing::Interpolation::Hold && points.len() > 1 { report.issue(format!("{path}/curves/{key}/interpolation"), D::Unsupported, json!(n.curves.interpolation(key)), "VOCALOID controller export retains nodes with hold semantics."); }
        }
        if edit_dynamics {
            let points = &n.curves.gain_millidecibel_points;
            let values = controls.entry("dyn".into()).or_default();
            let leading = points.first().filter(|point| point.tick != 0).map(|point| point.millidecibels);
            let start_node = if points.is_empty() { Some(0) } else { leading };
            for (tick, gain) in start_node.into_iter().map(|value| (0, value)).chain(points.iter().map(|p| (p.tick, p.millidecibels))) {
                if shared_end && tick == n.duration_tick {
                    if points.len() > 1 && points[points.len()-2].millidecibels != gain { report.issue(format!("{path}/gain/end"), D::Unsupported, gain, "The following note owns the shared boundary tick."); }
                    continue;
                }
                let tick = local_tick(tick)?;
                let value = gain_dynamics(gain).round().clamp(0.0, CONTROLLER_MAX as f64);
                if dynamics_gain(value).round() as i64 != gain {
                    report.issue(format!("{path}/gain/{tick}"), D::Quantized, gain, "Gain rounded to the nearest DYN value and bounded to 0..127.");
                }
                insert(values, tick, value as i64, path)?;
            }
            if n.curves.interpolation("DYN") != verdant_domain::editing::Interpolation::Hold && points.len() > 1 {
                report.issue(format!("{path}/gain/interpolation"), D::Unsupported, json!(n.curves.interpolation("DYN")), "VOCALOID DYN export retains authored nodes with hold semantics.");
            }
        }
    }
    if !pitch.is_empty() {
        let range = pitch.values().map(|&value| {
            let limit = if value < 0 { PIT_SCALE } else { PIT_SCALE - 1.0 };
            (value.unsigned_abs() as f64 * PIT_SCALE / (limit * MILLICENTS_PER_SEMITONE)).ceil() as i64
        }).max().unwrap_or(0).clamp(2, CONTROLLER_MAX);
        let mut events = Vec::new();
        for (tick, value) in pitch { let raw = (value as f64 / (range as f64 * MILLICENTS_PER_SEMITONE) * PIT_SCALE).round();
            let bounded = raw.clamp(-PIT_SCALE, PIT_SCALE - 1.0);
            if bounded != raw { return Err(E::at(format!("{path}/pitch/{tick}"), "pitch bend exceeds PBS sensitivity")); }
            if pitch_millicents(raw, range as f64).round() as i64 != value { report.issue(format!("{path}/pitch/{tick}"), D::Quantized, value, "VOCALOID pitch bend has 14-bit resolution."); }
            let raw = raw as i64;
            events.push(json!({"pos":tick,"value":raw})); }
        result.push(json!({"name":"pitchBendSens","events":[{"pos":0,"value":range}]})); result.push(json!({"name":"pitchBend","events":events}));
    }
    for (name, nodes) in controls { result.push(json!({"name":name,"events":nodes.into_iter().map(|(pos,value)|json!({"pos":pos,"value":value})).collect::<Vec<_>>()})); }
    Ok(result)
}
fn insert(map: &mut BTreeMap<i64, i64>, tick: i64, value: i64, path: &str) -> Result<()> {
    if let Some(old) = map.insert(tick, value) { if old != value { return Err(E::at(path, "overlapping notes require incompatible part-level controls")); } } Ok(())
}

pub(crate) fn export_vpr(project: &Project, options: &ExportOptions, report: &mut ExchangeReport) -> Result<Vec<u8>> {
    let saved = if options.preserve_extensions { project.extensions.get(KEY).and_then(Value::as_object) } else { None };
    let mut root = saved.and_then(|s| s.get("root")).and_then(Value::as_object).cloned().unwrap_or_default();
    root.entry("version").or_insert_with(|| json!({"major":5,"minor":0,"revision":0})); root.entry("vender").or_insert_with(|| "Yamaha Corporation".into());
    let mut master = saved.and_then(|s| s.get("master")).and_then(Value::as_object).cloned().unwrap_or_default();
    let mut tempo = saved.and_then(|s| s.get("tempo")).and_then(Value::as_object).cloned().unwrap_or_default();
    tempo.insert("global".into(), json!({"isEnabled":false,"value":12000}));
    let mut tempo_events = Vec::new();
    for t in &project.tempo_map { let value = t.micro_bpm as f64 / 10_000.0; if value != value.round() { report.issue(format!("/tempo_map/{}", t.tick), D::Quantized, t.micro_bpm, "VPR BPM resolution is 0.01 BPM."); }
        if value.round()<1.0 { return Err(E::at("/tempo","VPR tempo rounded to zero")); }
        let mut event=if options.preserve_extensions && saved.is_some(){t.unknown.clone().into_iter().collect::<Map<_,_>>()}else{Map::new()};
        event.insert("pos".into(),scale(t.tick,project.ppq,480,"/tempo")?.into());event.insert("value".into(),json!(value.round() as u64));tempo_events.push(Value::Object(event)); }
    tempo.insert("events".into(), tempo_events.into());
    if saved.and_then(|s| s.get("tempo_snapshot")) == Some(&json!(project.tempo_map.iter().map(|t| (t.tick, t.micro_bpm)).collect::<Vec<_>>())) {
        if let Some(old) = saved.and_then(|s| s.get("tempo_source")).and_then(Value::as_object) { tempo = old.clone(); }
    }
    master.insert("tempo".into(), tempo.into());
    let mut meter = saved.and_then(|s| s.get("meter")).and_then(Value::as_object).cloned().unwrap_or_default();
    meter.insert("events".into(), project.time_signatures.iter().zip(meters_to_bars(project)?).map(|(m,(bar,numer,denom))|{let mut map=if options.preserve_extensions && saved.is_some(){m.unknown.clone().into_iter().collect::<Map<_,_>>()}else{Map::new()};for(k,v)in[("bar",json!(bar)),("numer",json!(numer)),("denom",json!(denom))]{map.insert(k.into(),v);}Value::Object(map)}).collect::<Vec<_>>().into()); master.insert("timeSig".into(), meter.into());
    master.entry("samplingRate").or_insert(json!(44100)); root.insert("masterTrack".into(), master.into());
    let mut tracks = Vec::new();
    for (ti, track) in selected(project, options) {
        let mut map = crate::common::saved(&track.unknown, KEY, options.preserve_extensions);
        map.entry("type").or_insert(json!(0)); map.insert("name".into(), track.name.clone().into()); map.insert("isMuted".into(), track.muted.into()); map.insert("isSoloMode".into(), track.solo.into());
        if track.gain_millidecibels != 0 || track.pan_permille != 0 { report.issue(format!("/tracks/{ti}/mixer"), D::Unsupported, json!({"gain":track.gain_millidecibels,"pan":track.pan_permille}), "VPR mixer automation units are retained source-only; native fader/pan mapping is not guessed."); }
        let mut out_parts = Vec::new();
        for (pi, part) in parts(project, track)?.into_iter().enumerate() {
            let path = format!("/tracks/{ti}/parts/{pi}");
            let mut map = if options.preserve_extensions && part.metadata.get("format").and_then(Value::as_str) == Some("vpr") { part.metadata.get("extra").and_then(Value::as_object).cloned().unwrap_or_default() } else { Map::new() };
            map.insert("pos".into(), scale(part.start,project.ppq,480,&path)?.into()); map.insert("duration".into(), scale(part.duration,project.ppq,480,&path)?.into()); map.insert("name".into(), part.name.clone().into());
            let mut notes = Vec::new();
            for n in &part.notes {
                let mut map = crate::common::saved(&n.unknown, KEY, options.preserve_extensions); let original_velocity = map.remove("source_velocity");
                let v = velocity(n)?; let v = original_velocity.as_ref().filter(|x| x.get("editable").and_then(Value::as_u64) == Some(u64::from(v))).and_then(|x| x.get("value")).cloned().unwrap_or(json!(v));
                map.insert("pos".into(), scale(n.start_tick-part.start,project.ppq,480,&path)?.into()); map.insert("duration".into(), scale(n.duration_tick,project.ppq,480,&path)?.into());
                map.insert("number".into(), n.midi_note.into()); map.insert("lyric".into(), n.lyric.clone().into()); map.insert("velocity".into(), v);
                map.insert("phoneme".into(), destination_phonemes(n,&["vocaloid-xsampa","xsampa","x-sampa"],report,&path).into());
                if let Some(lang) = language_id(n.pronunciation.language.as_deref()) { map.insert("langID".into(), lang.into()); }
                if !n.pronunciation.phonemes.is_empty() { map.insert("isProtected".into(), true.into()); }
                notes.push(Value::Object(map));
            }
            map.insert("notes".into(), notes.into()); map.insert("controllers".into(), exported_controllers(&part,project.ppq,480,options.preserve_extensions,report,&path)?.into());
            if let Some(voice) = &options.vocaloid_voice { crate::xml::validate_voice(voice)?; map.remove("aiVoice"); map.insert("voice".into(), json!({"compID":voice.comp_id,"langID":voice.language_id})); }
            if !map.contains_key("voice") && !map.contains_key("aiVoice") { report.issue(format!("{path}/voice"), D::Metadata, Value::Null, "Unassigned destination voice; select an installed licensed VOCALOID voice before rendering."); }
            out_parts.push(Value::Object(map));
        }
        map.insert("parts".into(), out_parts.into()); tracks.push(Value::Object(map));
    }
    // Track selection filters editable vocal tracks. Retained audio/unknown
    // tracks remain source extensions, including on a scoped export.
    if let Some(extra) = saved.and_then(|s| s.get("tracks")).and_then(Value::as_array) {
        let selected_indices: Vec<_> = selected(project, options).map(|(_, t)| t.unknown.get("vpr_source_index").and_then(Value::as_u64)).collect();
        let mut ordered: Vec<(u64, Value)> = tracks.into_iter().enumerate().map(|(i,t)| (selected_indices[i].unwrap_or(u64::MAX),t)).collect();
        for item in extra { if let (Some(index), Some(value)) = (item.get("index").and_then(Value::as_u64),item.get("value")) { ordered.push((index,value.clone())); } }
        ordered.sort_by_key(|(index,_)|*index); tracks=ordered.into_iter().map(|(_,t)|t).collect();
    }
    root.insert("tracks".into(), tracks.into());
    if let Some(voice) = &options.vocaloid_voice { root.insert("voices".into(),json!([{"compID":voice.comp_id,"name":voice.name}])); }
    root.entry("voices").or_insert(json!([]));
    let mut members = BTreeMap::new(); members.insert(SEQUENCE.to_string(), serde_json::to_vec_pretty(&root)?);
    if let Some(old) = saved.and_then(|s| s.get("members")).and_then(Value::as_array) {
        if old.len() + 1 > options.limits.max_archive_members { return Err(E::at("/archive", "member limit exceeded")); }
        for m in old {
            let map = object(m, "/archive/members")?; let name = text(required(map,"name","/archive/members")?,"/archive/members/name")?;
            member_name(name, false)?;
            let encoded = text(required(map,"hex","/archive/members")?,"/archive/members/hex")?;
            if encoded.len() > options.limits.max_file_bytes.saturating_mul(2) { return Err(E::at("/archive", "retained member limit")); }
            let bytes = hex::decode(encoded).map_err(|_| E::at("/archive", "invalid retained member hex"))?;
            if members.insert(name.into(), bytes).is_some() { return Err(E::at("/archive", "duplicate retained sequence/member")); }
        }
    }
    let mut archive = ZipWriter::new(Cursor::new(Vec::new())); let mut total = 0u64; let mut names = BTreeSet::new();
    for (name, bytes) in members {
        if !names.insert(name.to_ascii_lowercase()) { return Err(E::at("/archive", "case-colliding output member")); }
        total = total.checked_add(bytes.len() as u64).ok_or_else(|| E::at("/archive", "length overflow"))?;
        if total > options.limits.max_archive_bytes { return Err(E::at("/archive", "uncompressed output limit")); }
        archive.start_file(name, SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).unix_permissions(0o600))?; archive.write_all(&bytes)?;
    }
    Ok(archive.finish()?.into_inner())
}
