use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value, json};
use verdant_domain::{Note, NoteId, PointId, Project, Track};
use verdant_domain::model::{ExpressionPoint, GainPoint, PitchPoint, TempoPoint, TimeSignature};
use verdant_domain::parts::{PartContent, PartInstance, Parts};
use crate::{Disposition as D, ExchangeError as E, ExchangeReport, ExportOptions, Format, Limits, Result};

pub(crate) const MAX_TICK: i64 = (1_i64 << 53) - 1;
pub(crate) fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| E::at(path, "expected an object"))
}
pub(crate) fn array<'a>(value: &'a Value, path: &str) -> Result<&'a [Value]> {
    value.as_array().map(Vec::as_slice).ok_or_else(|| E::at(path, "expected an array"))
}
pub(crate) fn required<'a>(map: &'a Map<String, Value>, key: &str, path: &str) -> Result<&'a Value> {
    map.get(key).ok_or_else(|| E::at(format!("{path}/{key}"), "missing field"))
}
pub(crate) fn integer(v: &Value, path: &str) -> Result<i64> { v.as_i64().ok_or_else(|| E::at(path, "expected an integer")) }
pub(crate) fn number(v: &Value, path: &str) -> Result<f64> {
    v.as_f64().filter(|n| n.is_finite()).ok_or_else(|| E::at(path, "expected a finite number"))
}
pub(crate) fn text<'a>(v: &'a Value, path: &str) -> Result<&'a str> { v.as_str().ok_or_else(|| E::at(path, "expected a string")) }
pub(crate) fn int(map: &Map<String, Value>, key: &str, default: i64, path: &str) -> Result<i64> {
    map.get(key).map(|v| integer(v, &format!("{path}/{key}"))).unwrap_or(Ok(default))
}
pub(crate) fn num(map: &Map<String, Value>, key: &str, default: f64, path: &str) -> Result<f64> {
    map.get(key).map(|v| number(v, &format!("{path}/{key}"))).unwrap_or(Ok(default))
}
pub(crate) fn string(map: &Map<String, Value>, key: &str, default: &str, path: &str) -> Result<String> {
    map.get(key).map(|v| text(v, &format!("{path}/{key}"))).unwrap_or(Ok(default)).map(str::to_string)
}
pub(crate) fn boolean(map: &Map<String, Value>, key: &str, default: bool, path: &str) -> Result<bool> {
    map.get(key).map(|v| v.as_bool().ok_or_else(|| E::at(format!("{path}/{key}"), "expected boolean"))).unwrap_or(Ok(default))
}
pub(crate) fn bounded(value: i64, min: i64, max: i64, path: &str) -> Result<i64> {
    if (min..=max).contains(&value) { Ok(value) } else { Err(E::at(path, format!("integer outside {min}..={max}"))) }
}
pub(crate) fn end(start: i64, duration: i64, path: &str) -> Result<i64> {
    bounded(start, 0, MAX_TICK, path)?; bounded(duration, 1, MAX_TICK, path)?;
    start.checked_add(duration).filter(|v| *v <= MAX_TICK).ok_or_else(|| E::at(path, "note/part end exceeds exact tick range"))
}
pub(crate) fn scale(value: i64, from: i64, to: i64, path: &str) -> Result<i64> {
    if from <= 0 || to <= 0 { return Err(E::at(path, "PPQ must be positive")); }
    let n = i128::from(value) * i128::from(to);
    if n % i128::from(from) != 0 { return Err(E::at(path, format!("tick {value} at PPQ {from} has no exact representation at PPQ {to}"))); }
    i64::try_from(n / i128::from(from)).ok().filter(|v| (-MAX_TICK..=MAX_TICK).contains(v))
        .ok_or_else(|| E::at(path, "tick scaling overflow"))
}
pub(crate) fn micro_bpm(bpm: f64, path: &str, report: &mut ExchangeReport) -> Result<u64> {
    if !bpm.is_finite() || !(0.000001..=1000.0).contains(&bpm) { return Err(E::at(path, "tempo must be positive and at most 1000 BPM")); }
    let result = (bpm * 1_000_000.0).round() as u64;
    if result as f64 / 1_000_000.0 != bpm { report.issue(path, D::Quantized, bpm, "Domain tempo is represented in micro-BPM."); }
    Ok(result)
}
pub(crate) fn source_project(ppq: i64) -> Project {
    let mut project = Project::new_empty(); project.ppq = ppq;
    project.tracks.clear(); project.voice_refs.clear(); project
}
pub(crate) fn source_track(name: impl Into<String>) -> Track {
    let mut track = Track::new(name, "teal"); track.voice_ref = None; track.monophonic = false; track
}
pub(crate) fn default_maps(project: &mut Project) {
    if project.tempo_map.is_empty() { project.tempo_map.push(TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: BTreeMap::new() }); }
    if project.time_signatures.is_empty() { project.time_signatures.push(TimeSignature { tick: 0, numerator: 4, denominator: 4, unknown: BTreeMap::new() }); }
}
pub(crate) fn note(start: i64, duration: i64, key: i64, lyric: String, path: &str) -> Result<Note> {
    end(start, duration, path)?; bounded(key, 0, 127, path)?;
    let mut n = Note::new(start, duration, key as i32); n.lyric = lyric; n.portamento_ms = Some(0); Ok(n)
}
pub(crate) fn velocity(note: &Note) -> Result<u8> {
    match note.unknown.get("midi_velocity") {
        None => Ok(64),
        Some(v) => Ok(bounded(integer(v, "/notes/midi_velocity")?, 0, 127, "/notes/midi_velocity")? as u8),
    }
}
pub(crate) fn set_velocity(n: &mut Note, v: i64, path: &str) -> Result<()> {
    bounded(v, 0, 127, path)?; n.unknown.insert("midi_velocity".into(), v.into()); Ok(())
}
pub(crate) fn phonemes(note: &mut Note, value: &str, language: Option<&str>, alphabet: &str) {
    if value.trim().is_empty() { return; }
    note.pronunciation.phonemes = value.split_whitespace().map(str::to_string).collect();
    note.pronunciation.language = language.map(str::to_string); note.pronunciation.alphabet = Some(alphabet.into());
    note.pronunciation.input_text = Some(note.lyric.clone()); note.pronunciation.source = verdant_domain::PronunciationSource::Imported;
    note.pronunciation.frontend_profile = "exchange-explicit-source-phonemes/1".into();
}
pub(crate) fn retain(map: &Map<String, Value>, known: &[&str], path: &str, report: &mut ExchangeReport) -> Map<String, Value> {
    map.iter().filter(|(k, _)| !known.contains(&k.as_str())).map(|(k, v)| {
        report.issue(format!("{path}/{}", pointer(k)), D::RetainedUnsupported, v.clone(), "Source field retained without executable semantics."); (k.clone(), v.clone())
    }).collect()
}
fn pointer(s: &str) -> String { s.replace('~', "~0").replace('/', "~1") }
pub(crate) fn saved(map: &Map<String, Value>, key: &str, preserve: bool) -> Map<String, Value> {
    if preserve { map.get(key).and_then(Value::as_object).cloned().unwrap_or_default() } else { Map::new() }
}
pub(crate) fn save_unknown(target: &mut Map<String, Value>, key: &str, value: Map<String, Value>, preserve: bool) {
    if preserve && !value.is_empty() { target.insert(key.into(), Value::Object(value)); }
}

// A recursive visitor rejects duplicate JSON keys before serde's Value map can
// overwrite them. serde_json's ordinary depth limit remains enabled.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<T: Deserializer<'de>>(d: T) -> std::result::Result<Self, T::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("a JSON value with unique object keys") }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Strict, E> { Ok(Strict(v.into())) }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Strict, E> { Ok(Strict(v.into())) }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Strict, E> { Ok(Strict(v.into())) }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v).map(|v| Strict(Value::Number(v))).ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Strict, E> { Ok(Strict(v.into())) }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Strict, E> { Ok(Strict(v.into())) }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Strict, E> { Ok(Strict(Value::Null)) }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Strict, E> { Ok(Strict(Value::Null)) }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> std::result::Result<Strict, A::Error> {
                let mut out = Vec::new(); while let Some(Strict(v)) = a.next_element()? { out.push(v); } Ok(Strict(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> std::result::Result<Strict, A::Error> {
                let mut out = Map::new(); while let Some(key) = a.next_key::<String>()? {
                    if out.contains_key(&key) { return Err(de::Error::custom(format!("duplicate JSON key {key:?}"))); }
                    out.insert(key, a.next_value::<Strict>()?.0);
                } Ok(Strict(Value::Object(out)))
            }
        }
        d.deserialize_any(V)
    }
}
pub(crate) fn parse_json(bytes: &[u8], allow_terminal_nul: bool) -> Result<Value> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let bytes = if allow_terminal_nul { bytes.strip_suffix(&[0]).unwrap_or(bytes) } else { bytes };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Strict::deserialize(&mut deserializer)?.0; deserializer.end()?; Ok(value)
}

pub(crate) fn validate(project: &Project, limits: &Limits, export_format: Option<Format>) -> Result<()> {
    if project.ppq <= 0 || project.ppq > MAX_TICK / 4 { return Err(E::at("/ppq", "PPQ outside supported exact range")); }
    if project.tempo_map.len().saturating_add(project.time_signatures.len()) > limits.max_events { return Err(E::at("/time", "tempo/meter event limit exceeded")); }
    if project.tracks.len() > limits.max_tracks || project.note_count() > limits.max_notes { return Err(E::at("/tracks", "track/note limit exceeded")); }
    verdant_domain::TempoMap::new(&project.tempo_map, project.ppq).map_err(|e| E::at("/tempo_map", e.to_string()))?;
    verdant_domain::MeterMap::new(&project.time_signatures, project.ppq).map_err(|e| E::at("/time_signatures", e))?;
    for pair in project.tempo_map.windows(2) { if pair[0].tick >= pair[1].tick { return Err(E::at("/tempo_map", "duplicate or unsorted tempo ticks")); } }
    for point in &project.tempo_map { bounded(point.tick, 0, MAX_TICK, "/tempo_map")?; }
    let mut tracks = BTreeSet::new(); let mut notes = BTreeSet::new(); let mut events = 0usize;
    for t in &project.tracks {
        if !tracks.insert(&t.id) { return Err(E::at("/tracks", "duplicate track identity")); }
        if !(-1000..=1000).contains(&t.pan_permille) { return Err(E::at("/tracks/pan", "pan outside -1000..1000")); }
        for n in &t.notes {
            if !notes.insert(&n.id) { return Err(E::at("/notes", "duplicate note identity")); }
            end(n.start_tick, n.duration_tick, "/notes")?; bounded(i64::from(n.midi_note), 0, 127, "/notes/key")?;
            velocity(n)?;
            for ticks in [n.curves.pitch_millicent_points.iter().map(|p| p.tick).collect::<Vec<_>>(),
                n.curves.gain_millidecibel_points.iter().map(|p| p.tick).collect()] {
                validate_curve(&ticks, n.duration_tick)?; events = events.checked_add(ticks.len()).ok_or_else(|| E::at("/curves", "event count overflow"))?;
            }
            for (lane, points) in &n.curves.expression_points {
                validate_curve(&points.iter().map(|p| p.tick).collect::<Vec<_>>(), n.duration_tick)?;
                let reports_clamping = matches!(export_format, Some(Format::Vpr | Format::Vsqx))
                    && verdant_domain::model::expression_default(lane).is_some();
                if !reports_clamping && points.iter().any(|p| p.value > 127) { return Err(E::at("/curves", "expression outside 0..127")); }
                events = events.checked_add(points.len()).ok_or_else(|| E::at("/curves", "event count overflow"))?;
            }
            if n.vibrato.as_ref().is_some_and(|v| !v.is_valid()) { return Err(E::at("/vibrato", "invalid vibrato")); }
            verdant_domain::editing::validate_note(n).map_err(|e| E::at("/notes", e))?;
        }
    }
    if events > limits.max_events { return Err(E::at("/curves", "event limit exceeded")); }
    let document = Parts::load(project).map_err(|e| E::at("/parts", e))?;
    document.validate(project).map_err(|e| E::at("/parts", e))?;
    if document.instances.len() > limits.max_events || document.contents.values().map(|c| c.notes.len()).sum::<usize>() > limits.max_notes { return Err(E::at("/parts", "part/content note limit exceeded")); }
    for part in &document.instances { end(part.start_tick, part.duration_tick, "/parts")?; end(part.source_offset_tick, part.duration_tick, "/parts/source")?; }
    for content in document.contents.values() { for n in &content.notes { end(n.start_tick, n.duration_tick, "/parts/content")?; } }
    Ok(())
}
fn validate_curve(ticks: &[i64], duration: i64) -> Result<()> {
    if ticks.iter().any(|t| *t < 0 || *t > duration) || ticks.windows(2).any(|w| w[0] >= w[1]) {
        return Err(E::at("/curves", "curve points must have strictly increasing ticks inside note boundaries"));
    } Ok(())
}

pub(crate) fn add_part(project: &mut Project, track_index: usize, id: String, name: String, start: i64, duration: i64,
    mut relative: Vec<Note>, metadata: Map<String, Value>) -> Result<()> {
    end(start, duration, "/parts")?;
    if track_index >= project.tracks.len() { return Err(E::at("/parts", "missing destination track")); }
    let mut parts = Parts::load(project).map_err(|e| E::at("/parts", e))?;
    if parts.contents.contains_key(&id) || parts.instances.iter().any(|p| p.id == id) { return Err(E::at("/parts", "duplicate part identity")); }
    let track = &mut project.tracks[track_index];
    let mut bindings = BTreeMap::new();
    for (i, n) in relative.iter_mut().enumerate() {
        if end(n.start_tick, n.duration_tick, "/parts/notes")? > duration { return Err(E::at("/parts/notes", "note extends outside declared part")); }
        n.id = NoteId(format!("{id}:source:{i}"));
        let mut visible = n.clone(); visible.id = NoteId(format!("{id}:note:{i}")); visible.start_tick += start;
        bindings.insert(n.id.clone(), visible.id.clone()); track.notes.push(visible);
    }
    parts.contents.insert(id.clone(), PartContent { id: id.clone(), notes: relative });
    parts.instances.push(PartInstance { id: id.clone(), content_id: id.clone(), name, track: track.id.clone(), start_tick: start,
        duration_tick: duration, source_offset_tick: 0, muted: false, bindings });
    project.extensions.insert(verdant_domain::parts::KEY.into(), serde_json::to_value(parts)?);
    if !metadata.is_empty() {
        let map = project.extensions.entry("exchange_parts").or_insert_with(|| json!({}));
        map.as_object_mut().ok_or_else(|| E::at("/exchange_parts", "expected object"))?.insert(id, metadata.into());
    }
    Ok(())
}

pub(crate) struct ExportPart<'a> {
    pub id: String, pub content_id: String, pub source_offset: i64, pub muted: bool, pub name: String, pub start: i64, pub duration: i64, pub notes: Vec<&'a Note>, pub metadata: Map<String, Value>,
}
pub(crate) fn parts<'a>(project: &'a Project, track: &'a Track) -> Result<Vec<ExportPart<'a>>> {
    let document = Parts::load(project).map_err(|e| E::at("/parts", e))?;
    let mut owned = BTreeSet::new(); let mut result = Vec::new();
    for part in document.instances.iter().filter(|p| p.track == track.id) {
        let mut notes: Vec<_> = part.bindings.values().map(|id| track.note(id).ok_or_else(|| E::at("/parts", "binding references a missing concrete note"))).collect::<Result<_>>()?;
        notes.sort_by_key(|n| (n.start_tick, n.midi_note, n.duration_tick));
        for n in &notes {
            if n.start_tick < part.start_tick || n.end_tick() > part.start_tick + part.duration_tick { return Err(E::at("/parts", "materialized note exceeds part bounds")); }
            if !owned.insert(n.id.clone()) { return Err(E::at("/parts", "duplicate note binding")); }
        }
        let metadata = project.extensions.get("exchange_parts").and_then(|m| m.get(&part.id)).and_then(Value::as_object).cloned().unwrap_or_default();
        result.push(ExportPart { id: part.id.clone(), content_id: part.content_id.clone(), source_offset: part.source_offset_tick, muted: part.muted, name: part.name.clone(), start: part.start_tick, duration: part.duration_tick, notes, metadata });
    }
    let mut free: Vec<_> = track.notes.iter().filter(|n| !owned.contains(&n.id)).collect();
    free.sort_by_key(|n| (n.start_tick, n.midi_note, n.duration_tick));
    if !free.is_empty() || result.is_empty() {
        let duration = free.iter().map(|n| n.end_tick()).max().unwrap_or(project.ppq * 4).max(1);
        result.push(ExportPart { id: format!("{}:ungrouped", track.id), content_id: format!("{}:ungrouped", track.id), source_offset: 0, muted: false, name: track.name.clone(), start: 0, duration, notes: free, metadata: Map::new() });
    }
    result.sort_by_key(|p| p.start); Ok(result)
}
pub(crate) fn selected<'a>(p: &'a Project, o: &ExportOptions) -> impl Iterator<Item = (usize, &'a Track)> {
    p.tracks.iter().enumerate().filter(move |(i, _)| o.track.is_none_or(|selected| selected == *i))
}

pub(crate) fn report_project_fields(p: &Project, o: &ExportOptions, report: &mut ExchangeReport) -> Result<()> {
    if p.render_preferences != verdant_domain::model::RenderPreferences::default() {
        report.issue("/render_preferences", D::Unsupported, serde_json::to_value(&p.render_preferences)?,
            "This score format has no Verdant render-route, seed or native synthesis-settings fields; these authored preferences are not exported.");
    }
    if !p.voice_refs.is_empty() {
        report.issue("/voice_refs", D::Unsupported, serde_json::to_value(&p.voice_refs)?, "Voice references do not license or install a destination synthesis engine or voice database.");
    }
    for key in ["automation", "master_inserts", "media", "plugins", "master_gain_millidecibels"] {
        if let Some(value) = p.extensions.get(key) { report.issue(format!("/extensions/{key}"), D::Unsupported, value.clone(), "This interchange does not map host plugins, mixer automation or audio assets."); }
    }
    let compatible = match report.format { Format::Midi => "exchange_midi", Format::Svp => "exchange_svp", Format::Vsqx => "exchange_vsqx", Format::Vpr => "exchange_vpr", Format::Ufdata => "exchange_ufdata", Format::Ust | Format::Ustx => "score_import" };
    if let Some(source)=p.extensions.get("score_import") {
        let expected=match report.format {Format::Ust=>Some("ust"),Format::Ustx=>Some("ustx"),_=>None};
        let same=o.preserve_extensions && source.get("source").and_then(|s|s.get("format")).and_then(Value::as_str)==expected;
        if let Some(fields)=source.get("fields").and_then(Value::as_array) { for field in fields {
            if field.get("disposition").and_then(Value::as_str)!=Some("retained_unsupported"){continue;}
            let path=field.get("location").and_then(Value::as_str).unwrap_or("/source");
            let retained=same && !matches!(path.rsplit('/').next(),Some("mode2_pitch"|"VBR"|"pitch"|"vibrato"|"Intensity"|"tuning")) && (path.starts_with("/#SETTING/") || path.starts_with("/tracks/")&&path[8..].matches('/').count()==1 || path.starts_with("/expressions/") || selected(p,o).any(|(_,t)|t.notes.iter().any(|n|n.unknown.get("source_note_ids").and_then(Value::as_array).is_some_and(|ids|ids.iter().filter_map(Value::as_str).any(|id|path.strip_prefix(id).is_some_and(|suffix|suffix.starts_with('/')&&!suffix[1..].contains('/')))))));
            report.issue(path,if retained{D::RetainedUnsupported}else{D::Unsupported},field.get("source_value").cloned().unwrap_or(Value::Null),if retained{"Source sample-control/extension field retained without executing its engine semantics."}else{"Retained source field has no destination event or compatible representation; original value is recorded here."});
        }}
    }
    for (key,value) in &p.extensions {
        if key.starts_with("exchange_") && key != "exchange_report" && key != "exchange_parts" && (!o.preserve_extensions || key != compatible) {
            report.issue(format!("/extensions/{key}"), D::Unsupported, value.clone(), "Source-specific extension fields have no compatible destination representation.");
        }
    }
    for (ti, t) in selected(p, o) {
        for (ni, n) in t.notes.iter().enumerate() {
            let path = format!("/tracks/{ti}/notes/{ni}");
            for (key,value) in &n.unknown { if key.starts_with("exchange_") && (!o.preserve_extensions || key != compatible) { report.issue(format!("{path}/{key}"),D::Unsupported,value.clone(),"Source-specific note extension omitted from this destination."); } }
            for (key,value) in [("stress",json!(n.pronunciation.stress)),("mora_weights",json!(n.pronunciation.mora_weights)),("reading_provenance",json!(n.pronunciation.reading_provenance))] { if !value.is_null() { report.issue(format!("{path}/pronunciation/{key}"),D::Unsupported,value,"Destination has no equivalent authored reading/prosody metadata field."); } }
            if !n.pronunciation.pinyin.is_empty() { report.issue(format!("{path}/pronunciation/pinyin"), D::Unsupported, n.pronunciation.pinyin.clone(), "Reading is distinct from lyric; no destination phoneme spelling is guessed."); }
            if n.pronunciation.phoneme_durations_permille.is_some() { report.issue(format!("{path}/pronunciation/phoneme_durations_permille"), D::Unsupported, json!(n.pronunciation.phoneme_durations_permille), "Native phoneme duration ratios have no admitted equivalent."); }
            if let Some(v) = &n.vibrato { report.issue(format!("{path}/vibrato"), D::Unsupported, serde_json::to_value(v)?, "Automatic vibrato models differ; explicit pitch points export separately."); }
            if n.portamento_ms != Some(0) { report.issue(format!("{path}/portamento_ms"), D::Unsupported, json!(n.portamento_ms), "Native portamento time has no equivalent transition model."); }
            if n.is_muted() && !matches!(report.format, Format::Svp) { report.issue(format!("{path}/muted"), D::Unsupported, true, "Destination has no admitted per-note mute field; score note is retained."); }
        }
    }
    Ok(())
}

/// Bar indices in SVP/VOCALOID are zero-based; changes must be on a bar edge.
pub(crate) fn meters_from_bars(mut bars: Vec<(i64, u32, u32)>, ppq: i64) -> Result<Vec<TimeSignature>> {
    if bars.is_empty() { bars.push((0, 4, 4)); }
    bars.sort_by_key(|p| p.0);
    if bars[0].0 != 0 || bars.windows(2).any(|w| w[0].0 == w[1].0) { return Err(E::at("/meter", "meter bars must start at zero without duplicates")); }
    let mut result = Vec::new(); let mut last_bar = 0; let mut tick = 0i64; let mut length = ppq.checked_mul(4).ok_or_else(|| E::at("/meter", "bar overflow"))?;
    for (bar, numerator, denominator) in bars {
        bounded(bar, 0, MAX_TICK, "/meter/bar")?;
        if numerator == 0 || denominator == 0 || !denominator.is_power_of_two() { return Err(E::at("/meter", "invalid meter")); }
        tick = tick.checked_add((bar - last_bar).checked_mul(length).ok_or_else(|| E::at("/meter", "bar overflow"))?).ok_or_else(|| E::at("/meter", "tick overflow"))?;
        let next = i128::from(ppq) * 4 * i128::from(numerator);
        if next % i128::from(denominator) != 0 { return Err(E::at("/meter", "meter bar has fractional ticks at this resolution")); }
        length = i64::try_from(next / i128::from(denominator)).map_err(|_| E::at("/meter", "bar overflow"))?;
        result.push(TimeSignature { tick, numerator, denominator, unknown: BTreeMap::new() }); last_bar = bar;
    } Ok(result)
}
pub(crate) fn meters_to_bars(p: &Project) -> Result<Vec<(i64, u32, u32)>> {
    let mut result = Vec::new(); let mut previous = 0; let mut bar = 0; let mut length = p.ppq * 4;
    for m in &p.time_signatures {
        if (m.tick - previous) % length != 0 { return Err(E::at("/time_signatures", "destination requires meter changes at bar boundaries")); }
        bar += (m.tick - previous) / length; result.push((bar, m.numerator, m.denominator));
        length = i64::try_from(i128::from(p.ppq) * 4 * i128::from(m.numerator) / i128::from(m.denominator)).map_err(|_| E::at("/meter", "bar overflow"))?;
        if length <= 0 { return Err(E::at("/meter", "bar length must be positive")); } previous = m.tick;
    } Ok(result)
}

pub(crate) enum Curve { Pitch, Gain, Expression(String) }
/// Map already admitted absolute controller nodes onto editable note curves.
/// Hold or linear interpolation is explicit; boundary values are evaluated so
/// a controller node before a note still supplies its starting value.
pub(crate) fn apply_curve(notes: &mut [Note], nodes: &[(i64, f64)], kind: Curve, linear: bool, path: &str) -> Result<()> {
    if nodes.windows(2).any(|w| w[0].0 >= w[1].0) || nodes.iter().any(|p| !p.1.is_finite()) { return Err(E::at(path, "controller ticks must strictly increase and values be finite")); }
    if nodes.is_empty() { return Ok(()); }
    if nodes.iter().any(|p| p.1.abs() >= i64::MAX as f64) { return Err(E::at(path, "curve value exceeds signed integer range")); }
    for n in notes {
        let mut points = Vec::new();
        let evaluate = |at: i64| {
            let i = nodes.partition_point(|p| p.0 <= at);
            if i == 0 { return 0.0; } let left = nodes[i - 1];
            if !linear || i == nodes.len() { left.1 } else { let right = nodes[i]; left.1 + (right.1 - left.1) * (at - left.0) as f64 / (right.0 - left.0) as f64 }
        };
        points.push((0, evaluate(n.start_tick)));
        points.extend(nodes.iter().filter(|(t, _)| *t > n.start_tick && *t < n.end_tick()).map(|(t, v)| (*t - n.start_tick, *v)));
        points.push((n.duration_tick, evaluate(n.end_tick())));
        let interpolation = if linear { verdant_domain::editing::Interpolation::Linear } else { verdant_domain::editing::Interpolation::Hold };
        match &kind {
            Curve::Pitch => { n.curves.pitch_millicent_points = points.into_iter().map(|(tick, v)| PitchPoint { id: PointId::generate(), tick, millicents: v.round() as i64, unknown: BTreeMap::new() }).collect(); n.curves.set_interpolation("PIT", interpolation); }
            Curve::Gain => { n.curves.gain_millidecibel_points = points.into_iter().map(|(tick, v)| GainPoint { id: PointId::generate(), tick, millidecibels: v.round() as i64, unknown: BTreeMap::new() }).collect(); n.curves.set_interpolation("DYN", interpolation); }
            Curve::Expression(key) => { n.curves.expression_points.insert(key.clone(), points.into_iter().map(|(tick, v)| ExpressionPoint { id: PointId::generate(), tick, value: v.round().clamp(0.0, 127.0) as u8, unknown: BTreeMap::new() }).collect()); n.curves.set_interpolation(key, interpolation); }
        }
    } Ok(())
}

/// Controller snapshots omit ephemeral point identities. A compatible export
/// keeps source nodes outside notes and source controller metadata until an
/// authored boundary or curve actually changes.
pub(crate) fn curve_snapshot<'a>(notes: impl Iterator<Item = &'a Note>, offset: i64) -> Value {
    let mut notes: Vec<_> = notes.collect();
    notes.sort_by_key(|n| (n.start_tick, n.midi_note, n.duration_tick));
    json!(notes.into_iter().map(|n| json!({"start": n.start_tick-offset,"duration":n.duration_tick,"key":n.midi_note,"detune":n.detune_millicents,
        "pitch":n.curves.pitch_millicent_points.iter().map(|p|(p.tick,p.millicents)).collect::<Vec<_>>(),
        "gain":n.curves.gain_millidecibel_points.iter().map(|p|(p.tick,p.millidecibels)).collect::<Vec<_>>(),
        "expression":n.curves.expression_points.iter().map(|(k,v)|(k.clone(),v.iter().map(|p|(p.tick,p.value)).collect::<Vec<_>>())).collect::<BTreeMap<_,_>>(),
        "interpolation": n.curves.unknown
    })).collect::<Vec<_>>())
}

/// Phonetic alphabets are explicit contracts; spelling in another engine's
/// dictionary is not guessed from a portable IPA or a voicebank sample alias.
pub(crate) fn destination_phonemes(note:&Note, accepted:&[&str], report:&mut ExchangeReport, path:&str)->String {
    if note.pronunciation.phonemes.is_empty() { return String::new(); }
    if note.pronunciation.alphabet.as_deref().is_some_and(|a|accepted.contains(&a)) { return note.pronunciation.phonemes.join(" "); }
    report.issue(format!("{path}/pronunciation/phonemes"),D::Unsupported,json!({"alphabet":note.pronunciation.alphabet,"phonemes":note.pronunciation.phonemes}),"The authored phonetic alphabet has no admitted mapping to the destination engine's dictionary; lyric text is preserved without guessing symbols.");
    String::new()
}
