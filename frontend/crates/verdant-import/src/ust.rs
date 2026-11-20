use std::borrow::Cow;
use std::collections::BTreeSet;
use encoding_rs::SHIFT_JIS;
use serde_json::{Map, Value, json};
use crate::controls::{self, BendPoint, NoteControl, Shape, SourceVibrato};
use crate::{FieldDisposition as D, ImportError, ImportNotice, Result, Source, SourceMeter, SourceNote, SourceTempo, SourceTrack, UstEncoding, checked_add, field, micro_bpm, retain_except};

struct Section { name: String, fields: Map<String, Value> }

pub(crate) fn parse(bytes: &[u8], encoding: UstEncoding) -> Result<Source> {
    let (decoded, encoding_name) = decode(bytes, encoding)?;
    let mut sections: Vec<Section> = Vec::new();
    let mut names = BTreeSet::new();
    for (line_no, line) in decoded.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let path = format!("/line/{}", line_no + 1);
        if line.starts_with('[') {
            let name = line.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                .filter(|s| s.starts_with('#') && !s.contains(['[', ']']))
                .ok_or_else(|| ImportError::at(&path, "malformed UST section header"))?;
            if !names.insert(name.to_string()) { return Err(ImportError::at(&path, format!("duplicate UST section {name}"))); }
            sections.push(Section { name: name.into(), fields: Map::new() });
        } else if let Some((key, value)) = line.split_once('=') {
            let section = sections.last_mut().ok_or_else(|| ImportError::at(&path, "UST key outside section"))?;
            let key = key.trim();
            if key.is_empty() || section.fields.contains_key(key) { return Err(ImportError::at(&path, format!("empty or duplicate UST key {key:?}"))); }
            section.fields.insert(key.into(), value.trim().into());
        } else if sections.last().is_some_and(|s| s.name == "#VERSION") && line.starts_with("UST Version") {
            let section = sections.last_mut().unwrap();
            if section.fields.insert("version_header".into(), line.into()).is_some() { return Err(ImportError::at(&path, "duplicate UST version header")); }
        } else { return Err(ImportError::at(path, "unsupported bare UST line")); }
    }
    if sections.last().map(|s| s.name.as_str()) != Some("#TRACKEND") { return Err(ImportError::at("/", "UST must end with #TRACKEND")); }
    if !sections.last().unwrap().fields.is_empty() { return Err(ImportError::at("/#TRACKEND", "data after #TRACKEND is unsupported")); }
    let setting = sections.iter().find(|s| s.name == "#SETTING").ok_or_else(|| ImportError::at("/", "missing UST #SETTING"))?;
    let setting = &setting.fields;
    if let Some(v) = string(setting, "Tracks") {
        if int(v, 1, "/#SETTING/Tracks")? != 1 { return Err(ImportError::at("/#SETTING/Tracks", "UST contains one track")); }
    }
    for key in ["Resolution", "TicksPerQuarter"] {
        if let Some(v) = string(setting, key) {
            if int(v, 1, key)? != 480 { return Err(ImportError::at(key, "UST resolution must be 480 PPQ")); }
        }
    }
    let bpm = decimal(need(setting, "Tempo", "/#SETTING")?, "/#SETTING/Tempo")?;
    micro_bpm(bpm, "/#SETTING/Tempo")?;
    let title = string(setting, "ProjectName").unwrap_or("Imported UST").to_string();
    let mut source = Source {
        ppq: 480, encoding: encoding_name.into(), title: title.clone(),
        tempos: vec![SourceTempo { tick: 0, bpm }], meters: parse_meters(setting)?,
        tracks: Vec::new(), fields: Vec::new(), notices: Vec::new(),
    };
    field(&mut source.fields, "/#SETTING/ProjectName", json!(title), D::Metadata, Some("track.name"), "Source project name retained.");
    field(&mut source.fields, "/#SETTING/Tempo", setting["Tempo"].clone(), D::Metadata, Some("project.tempo_map"), "Initial setting retained; per-event Tempo takes effect at that event onset.");
    if let Some(raw) = setting.get("TimeSignatures") {
        field(&mut source.fields, "/#SETTING/TimeSignatures", raw.clone(), D::Translated, Some("project.time_signatures"), "Source (numerator/denominator/bar) tuples translated using preceding meter lengths; original setting retained.");
    }
    retain_except(setting, &["Tempo", "Tracks", "Resolution", "TicksPerQuarter", "ProjectName", "TimeSignatures"], "/#SETTING", &mut source.fields);
    let mut track = SourceTrack { index: 0, name: title, notes: Vec::new(), end_tick: 0, gain_db: 0.0, pan: 0, muted: false, solo: false, curves: Vec::new() };
    let mut cursor = 0;
    let mut previous_start = 0;
    for section in &sections {
        let name = section.name.as_str();
        if name == "#VERSION" {
            field(&mut source.fields, "/#VERSION", Value::Object(section.fields.clone()), D::Metadata, None, "Source version declaration.");
            continue;
        }
        if matches!(name, "#SETTING" | "#TRACKEND") { continue; }
        if !name.strip_prefix('#').is_some_and(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())) {
            return Err(ImportError::at(name, "UST plugin/control sections are unsupported"));
        }
        let index = track.notes.len();
        if name != format!("#{index:04}") { return Err(ImportError::at(name, "UST numbered sections must be sequential from #0000")); }
        let map = &section.fields;
        let length = int(need(map, "Length", name)?, 1, name)?;
        let midi = int(need(map, "NoteNum", name)?, 0, name)?;
        if midi > 127 { return Err(ImportError::at(name, "MIDI note exceeds 127")); }
        let lyric = need(map, "Lyric", name)?.to_string();
        let (start, duration) = match (string(map, "Delta"), string(map, "Duration")) {
            (None, None) => (cursor, length),
            (Some(delta), Some(duration)) => {
                let delta = int(delta, i64::MIN, name)?;
                let start = checked_add(previous_start, delta, name)?;
                if start < 0 { return Err(ImportError::at(name, "negative UST note onset")); }
                (start, int(duration, 1, name)?)
            }
            _ => return Err(ImportError::at(name, "UST 2 timing requires both Delta and Duration")),
        };
        if let Some(v) = string(map, "Tempo").filter(|v| !v.is_empty()) {
            let bpm = decimal(v, name)?;
            let mbpm = micro_bpm(bpm, name)?;
            let last = source.tempos.last_mut().unwrap();
            if start < last.tick { return Err(ImportError::at(name, "UST tempo positions are not increasing")); }
            if start == last.tick { last.bpm = bpm; }
            else if micro_bpm(last.bpm, name)? != mbpm { source.tempos.push(SourceTempo { tick: start, bpm }); }
        }
        let path = format!("t0/p0/n{index}");
        let mut control = NoteControl::default();
        if let Some(v) = string(map, "Intensity").filter(|v| !v.is_empty()) {
            let intensity = decimal(v, &path)?;
            if !(0.0..=200.0).contains(&intensity) { return Err(ImportError::at(&path, "Intensity must be 0..200 percent")); }
            control.intensity_percent = Some(intensity);
            controls::record_control(&mut source.fields, &format!("{path}/Intensity"), json!(v), true, "note.curves.gain_millidecibel_points");
        }
        if ["PBS", "PBW", "PBY", "PBM"].iter().any(|k| map.contains_key(*k)) {
            let mut raw = Map::new();
            for key in ["PBS", "PBW", "PBY", "PBM"] { if let Some(v) = map.get(key) { raw.insert(key.into(), v.clone()); } }
            let pbs: Vec<_> = string(map, "PBS").unwrap_or("0").split([';', ',']).collect();
            if pbs.len() > 2 { return Err(ImportError::at(&path, "PBS has more than two coordinates")); }
            let mut x = decimal_empty(pbs[0], &path)?;
            let y = if pbs.len() == 2 { decimal_empty(pbs[1], &path)? } else { 0.0 };
            let widths = csv(string(map, "PBW").unwrap_or(""), &path)?;
            let ys = csv(string(map, "PBY").unwrap_or(""), &path)?;
            if ys.len() > widths.len() { return Err(ImportError::at(&path, "PBY has more values than PBW segments")); }
            let shapes: Vec<_> = string(map, "PBM").unwrap_or("").split(',').collect();
            let mut supported = string(map, "PBS").is_some_and(|v| !v.is_empty());
            let mut points = vec![BendPoint { ms: x, cents: y * 10.0, shape: Shape::InOut }];
            for (i, width) in widths.into_iter().enumerate() {
                if width < 0.0 { return Err(ImportError::at(&path, "PBW segments must not have negative widths")); }
                let shape = Shape::parse(shapes.get(i).copied().unwrap_or(""));
                supported &= shape.is_some();
                points[i].shape = shape.unwrap_or(Shape::InOut);
                x += width;
                if !x.is_finite() { return Err(ImportError::at(&path, "pitch time overflow")); }
                points.push(BendPoint { ms: x, cents: ys.get(i).copied().unwrap_or(0.0) * 10.0, shape: Shape::InOut });
            }
            supported &= points.len() >= 2;
            if supported { control.pitch = Some(points); }
            controls::record_control(&mut source.fields, &format!("{path}/mode2_pitch"), Value::Object(raw), supported, "note.curves.pitch_millicent_points");
        }
        if let Some(v) = string(map, "VBR").filter(|v| !v.is_empty()) {
            let values = csv(v, &path)?;
            if values.is_empty() || values.len() > 7 { return Err(ImportError::at(&path, "VBR expects one to seven values")); }
            let get = |i, default| values.get(i).copied().unwrap_or(default);
            control.vibrato = Some(SourceVibrato { length: get(0, 0.0), period_ms: get(1, 175.0), depth_cents: get(2, 25.0),
                fade_in: get(3, 10.0), fade_out: get(4, 10.0), shift: get(5, 0.0), drift: get(6, 0.0), volume_link: 0.0 }.validate(&path)?);
            controls::record_control(&mut source.fields, &format!("{path}/VBR"), json!(v), true, "note.curves.pitch_millicent_points");
        }
        retain_except(map, &["Length", "Lyric", "NoteNum", "Tempo", "Delta", "Duration", "Intensity", "PBS", "PBW", "PBY", "PBM", "VBR"], &path, &mut source.fields);
        let source_reading = if lyric.starts_with('?') || string(map, "@alias").is_some_and(|v| !v.is_empty()) {
            crate::SourceReadingEvidence::VoicebankPhonemeAlias
        } else { crate::SourceReadingEvidence::LyricOnly };
        track.notes.push(SourceNote { part: 0, index, start, duration, midi: midi as i32, lyric, source_reading, control });
        previous_start = start;
        cursor = checked_add(start, duration, name)?;
        track.end_tick = track.end_tick.max(cursor).max(checked_add(start, length, name)?);
    }
    if track.notes.is_empty() { return Err(ImportError::at("/", "UST has no note or rest events")); }
    for t in &source.tempos {
        if t.bpm != micro_bpm(t.bpm, "/tempos")? as f64 / 1_000_000.0 {
            source.notices.push(ImportNotice { code: "tempo_quantization".into(), location: "/tempos".into(), message: "Tempo rounded to Verdant micro-BPM; original BPM retained in source_tempos.".into() });
        }
    }
    source.tracks.push(track);
    Ok(source)
}

fn decode(bytes: &[u8], encoding: UstEncoding) -> Result<(Cow<'_, str>, &'static str)> {
    let utf8 = || std::str::from_utf8(bytes).map(Cow::Borrowed).map_err(|_| ImportError::at("/encoding", "UST is not valid UTF-8"));
    match encoding {
        UstEncoding::Utf8 => Ok((utf8()?, "utf-8")),
        UstEncoding::Auto if bytes.starts_with(&[0xef, 0xbb, 0xbf]) => Ok((utf8()?, "utf-8-bom")),
        UstEncoding::Auto if std::str::from_utf8(bytes).is_ok() => Ok((utf8()?, "utf-8")),
        UstEncoding::Auto | UstEncoding::Cp932 => {
            let value = SHIFT_JIS.decode_without_bom_handling_and_without_replacement(bytes)
                .ok_or_else(|| ImportError::at("/encoding", "UST is not valid CP932 (Windows-31J); replacement decoding is forbidden"))?;
            Ok((value, "cp932"))
        }
    }
}
fn string<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> { map.get(key).and_then(Value::as_str) }
fn need<'a>(map: &'a Map<String, Value>, key: &str, path: &str) -> Result<&'a str> { string(map, key).ok_or_else(|| ImportError::at(path, format!("missing {key}"))) }
fn decimal(text: &str, path: &str) -> Result<f64> {
    text.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or_else(|| ImportError::at(path, format!("invalid finite number {text:?}")))
}
fn decimal_empty(text: &str, path: &str) -> Result<f64> { if text.is_empty() { Ok(0.0) } else { decimal(text, path) } }
fn int(text: &str, min: i64, path: &str) -> Result<i64> {
    text.parse::<i64>().ok().filter(|v| *v >= min).ok_or_else(|| ImportError::at(path, format!("expected integer >= {min}")))
}
fn csv(text: &str, path: &str) -> Result<Vec<f64>> {
    if text.is_empty() { return Ok(Vec::new()); }
    text.split(',').map(|s| decimal_empty(s.trim(), path)).collect()
}

fn parse_meters(setting: &Map<String, Value>) -> Result<Vec<SourceMeter>> {
    let Some(raw) = string(setting, "TimeSignatures") else {
        return Ok(vec![SourceMeter { tick: 0, numerator: 4, denominator: 4 }]);
    };
    let raw = raw.trim();
    // The UST grammar permits one final comma, not an empty meter entry.
    let raw = raw.strip_suffix(',').unwrap_or(raw).trim_end();
    if raw.is_empty() { return Err(ImportError::at("/#SETTING/TimeSignatures", "expected at least one (numerator/denominator/bar) tuple")); }
    let mut meters = Vec::new();
    let mut previous_bar = 0;
    for (i, tuple) in raw.split(',').enumerate() {
        let path = format!("/#SETTING/TimeSignatures/{i}");
        let body = tuple.trim().strip_prefix('(').and_then(|t| t.strip_suffix(')'))
            .ok_or_else(|| ImportError::at(&path, "expected (numerator/denominator/bar) tuple"))?;
        let mut values = body.split('/');
        let mut component = |minimum| -> Result<i64> {
            int(values.next().ok_or_else(|| ImportError::at(&path, "meter tuple needs three components"))?.trim(), minimum, &path)
        };
        let numerator = component(1)?;
        let denominator = component(1)?;
        let bar = component(0)?;
        if values.next().is_some() { return Err(ImportError::at(&path, "meter tuple has more than three components")); }
        crate::push_meter_from_bar(&mut meters, &mut previous_bar, 480, bar, numerator, denominator, &path)?;
    }
    Ok(meters)
}
