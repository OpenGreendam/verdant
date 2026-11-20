//! Synthesizer V Studio's native JSON score schema (`time`, `mainGroup`,
//! `mainRef`, `blickOffset`). Blick boundaries are exact musical ticks.
use std::collections::{BTreeMap, BTreeSet};
use serde_json::{Map, Value, json};
use verdant_domain::{Note, Project};
use verdant_domain::model::TempoPoint;
use crate::{ExchangeError as E, ExchangeReport, ImportOptions, ExportOptions, Disposition as D, Result};
use crate::common::*;
const BLICKS: i64 = 705_600_000;
const KEY: &str = "exchange_svp";

pub(crate) fn import(bytes: &[u8], options: &ImportOptions, report: &mut ExchangeReport) -> Result<Project> {
    let value = parse_json(bytes, true)?; let root = object(&value, "/")?;
    let version = integer(required(root, "version", "")?, "/version")?;
    bounded(version, 100, 1_000_000, "/version")?;
    let time = object(required(root, "time", "")?, "/time")?;
    let mut project = source_project(BLICKS); project.tempo_map.clear(); project.time_signatures.clear(); report.source_ppq = BLICKS;
    for (i, tempo) in array(required(time, "tempo", "/time")?, "/time/tempo")?.iter().enumerate() {
        let path = format!("/time/tempo/{i}"); let t = object(tempo, &path)?;
        let tick = integer(required(t, "position", &path)?, &path)?;
        let bpm = micro_bpm(number(required(t, "bpm", &path)?, &path)?, &path, report)?;
        let unknown = retain(t, &["position", "bpm"], &path, report).into_iter().collect();
        project.tempo_map.push(TempoPoint { tick, micro_bpm: bpm, unknown });
    }
    let mut bars = Vec::new();
    let mut meter_extras = BTreeMap::new();
    for (i, meter) in array(required(time, "meter", "/time")?, "/time/meter")?.iter().enumerate() {
        let path = format!("/time/meter/{i}"); let m = object(meter, &path)?;
        bars.push((integer(required(m, "index", &path)?, &path)?,
            bounded(integer(required(m, "numerator", &path)?, &path)?, 1, 65535, &path)? as u32,
            bounded(integer(required(m, "denominator", &path)?, &path)?, 1, 65536, &path)? as u32));
        meter_extras.insert(integer(required(m, "index", &path)?, &path)?,retain(m, &["index", "numerator", "denominator"], &path, report));
    }
    project.time_signatures = meters_from_bars(bars.clone(), BLICKS)?;
    bars.sort_by_key(|b| b.0);
    for (meter,(bar,_,_)) in project.time_signatures.iter_mut().zip(bars) { if let Some(extra)=meter_extras.remove(&bar) { meter.unknown=extra.into_iter().collect(); } }
    default_maps(&mut project);
    let tracks = array(required(root, "tracks", "")?, "/tracks")?;
    if tracks.len() > options.limits.max_tracks { return Err(E::at("/tracks", "track limit exceeded")); }
    let mut library = BTreeMap::<String, &Value>::new();
    if let Some(groups) = root.get("library") {
        for group in array(groups, "/library")? {
            let map = object(group, "/library")?; let id = text(required(map, "uuid", "/library")?, "/library/uuid")?;
            if id.trim().is_empty(){return Err(E::at("/library/uuid","group UUID must not be empty"));}
            if library.insert(id.into(), group).is_some() { return Err(E::at("/library", "duplicate note-group UUID")); }
        }
    }
    let mut used_library = BTreeSet::new(); let mut total_notes = 0usize;
    for (ti, raw_track) in tracks.iter().enumerate() {
        let path = format!("/tracks/{ti}"); let map = object(raw_track, &path)?;
        let mut track = source_track(string(map, "name", "Track", &path)?);
        let mut unknown = retain(map, &["name", "mainGroup", "mainRef", "groups", "mixer", "dispOrder", "dispColor", "renderEnabled"], &path, report);
        for key in ["dispOrder", "dispColor", "renderEnabled"] { if let Some(v) = map.get(key) { unknown.insert(key.into(), v.clone()); } }
        if let Some(mixer) = map.get("mixer") {
            let mixer = object(mixer, &format!("{path}/mixer"))?;
            let gain = num(mixer, "gainDecibel", 0.0, &path)?;
            if !(-96.0..=24.0).contains(&gain) { return Err(E::at(format!("{path}/mixer/gainDecibel"), "mixer gain outside editable domain range")); }
            track.gain_millidecibels = (gain * 1000.0).round() as i32;
            let pan = num(mixer, "pan", 0.0, &path)?;
            if !(-1.0..=1.0).contains(&pan) { return Err(E::at(format!("{path}/mixer/pan"), "pan outside -1..1")); }
            track.pan_permille = (pan * 1000.0).round() as i32;
            track.muted = boolean(mixer, "mute", false, &path)?; track.solo = boolean(mixer, "solo", false, &path)?;
            let rest = retain(mixer, &["gainDecibel", "pan", "mute", "solo"], &format!("{path}/mixer"), report);
            if !rest.is_empty() { unknown.insert("mixer".into(), rest.into()); }
        }
        save_unknown(&mut track.unknown, KEY, unknown, options.preserve_extensions); project.tracks.push(track);
        if let Some(group) = map.get("mainGroup") {
            let reference = map.get("mainRef").cloned().unwrap_or_else(|| json!({"blickOffset":0,"pitchOffset":0}));
            add_group(&mut project, ti, group, &reference, format!("svp:t{ti}:main"), &format!("{path}/mainGroup"), options, report, &mut total_notes)?;
        }
        if let Some(groups) = map.get("groups") {
            for (pi, reference) in array(groups, &format!("{path}/groups"))?.iter().enumerate() {
                let r = object(reference, &path)?;
                if boolean(r, "isInstrumental", false, &path)? || r.get("audio").is_some_and(|v| !v.is_null()) {
                    report.issue(format!("{path}/groups/{pi}"), D::RetainedUnsupported, reference.clone(), "Instrumental reference retained; external audio is not opened or synthesized.");
                    if options.preserve_extensions {
                        project.tracks[ti].unknown.entry("exchange_svp_audio").or_insert_with(|| json!([])).as_array_mut().expect("new array").push(reference.clone());
                    } continue;
                }
                let id = text(required(r, "groupID", &path)?, &path)?;
                let group = library.get(id).ok_or_else(|| E::at(format!("{path}/groups/{pi}/groupID"), "missing note group"))?;
                used_library.insert(id.to_string());
                add_group(&mut project, ti, group, reference, format!("svp:t{ti}:p{pi}"), &format!("{path}/groups/{pi}"), options, report, &mut total_notes)?;
            }
        }
    }
    let root_extra = retain(root, &["version", "time", "tracks", "library"], "", report);
    let time_extra = retain(time, &["tempo", "meter"], "/time", report);
    if options.preserve_extensions {
        let unused: Vec<_> = library.iter().filter(|(id, _)| !used_library.contains(*id)).map(|(_, value)| (*value).clone()).collect();
        project.extensions.insert(KEY.into(), json!({"root":root_extra,"time":time_extra,"version":version,"unused_library":unused}));
    }
    report.issue("/synthesis", D::Metadata, "Synthesizer V", "File interchange does not install, license or emulate Synthesizer V or its voice databases. Automatic pitch/voice attributes remain source-only.");
    Ok(project)
}

fn add_group(project: &mut Project, ti: usize, value: &Value, reference: &Value, id: String, path: &str,
    options: &ImportOptions, report: &mut ExchangeReport, total: &mut usize) -> Result<()> {
    let group = object(value, path)?; let r = object(reference, path)?;
    if text(required(group,"uuid",path)?,path)?.trim().is_empty(){return Err(E::at(path,"group UUID must not be empty"));}
    let start = int(r, "blickOffset", 0, path)?; bounded(start, 0, MAX_TICK, path)?;
    let pitch = int(r, "pitchOffset", 0, path)?; bounded(pitch, -127, 127, path)?;
    if pitch!=0 {report.issue(format!("{path}/reference/pitchOffset"),D::RetainedUnsupported,pitch,"Pitch offset is materialized into editable note keys and retained for compatible reference export. References with different offsets have independent domain content because domain parts have no transpose field.");}
    let db = r.get("database").and_then(Value::as_object);
    let language = db.and_then(|d| d.get("language")).and_then(Value::as_str);
    let alphabet = db.and_then(|d| d.get("phoneset")).and_then(Value::as_str).unwrap_or("synthv-source-phoneset");
    let mut notes = Vec::new(); let mut note_ids = BTreeSet::new();
    for (ni, raw) in array(required(group, "notes", path)?, path)?.iter().enumerate() {
        *total = total.checked_add(1).ok_or_else(|| E::at(path, "note count overflow"))?;
        if *total > options.limits.max_notes { return Err(E::at(path, "note limit exceeded")); }
        let np = format!("{path}/notes/{ni}"); let map = object(raw, &np)?;
        if let Some(uuid) = map.get("uuid") { if !note_ids.insert(text(uuid, &np)?) { return Err(E::at(&np, "duplicate note UUID")); } }
        let mut n = note(integer(required(map, "onset", &np)?, &np)?, integer(required(map, "duration", &np)?, &np)?,
            integer(required(map, "pitch", &np)?, &np)?.checked_add(pitch).ok_or_else(|| E::at(&np, "pitch overflow"))?, string(map, "lyrics", "", &np)?, &np)?;
        let detune = num(map, "detune", 0.0, &np)? * 1000.0;
        if detune.abs() > i32::MAX as f64 { return Err(E::at(&np, "detune overflow")); }
        n.detune_millicents = detune.round() as i32;
        if detune != detune.round() { report.issue(format!("{np}/detune"), D::Quantized, map["detune"].clone(), "Detune uses integer millicents in Verdant."); }
        phonemes(&mut n, &string(map, "phonemes", "", &np)?, language, alphabet);
        let mut extra = retain(map, &["onset", "duration", "pitch", "lyrics", "phonemes", "detune", "attributes"], &np, report);
        if let Some(attributes) = map.get("attributes") {
            let attributes = object(attributes, &format!("{np}/attributes"))?;
            n.set_muted(boolean(attributes, "muted", false, &np)?);
            let rest = retain(attributes, &["muted"], &format!("{np}/attributes"), report);
            if !rest.is_empty() { extra.insert("attributes".into(), rest.into()); }
        }
        save_unknown(&mut n.unknown, KEY, extra, options.preserve_extensions);
        notes.push(n);
    }
    notes.sort_by_key(|n| n.start_tick);
    if let Some(parameters) = group.get("parameters") {
        let parameters = object(parameters, &format!("{path}/parameters"))?;
        for (key, parameter) in parameters {
            let pp = format!("{path}/parameters/{key}");
            if !matches!(key.as_str(), "pitchDelta" | "loudness") { report.issue(&pp, D::RetainedUnsupported, parameter.clone(), "Source timbre parameter has no admitted equivalent or matching units."); continue; }
            let parameter = object(parameter, &pp)?; let mode = string(parameter, "mode", "linear", &pp)?;
            if mode != "linear" { report.issue(&pp, D::RetainedUnsupported, Value::Object(parameter.clone()), "Only linear source curve interpolation has an exact admitted counterpart; curve retained without approximate resampling."); continue; }
            let raw = array(required(parameter, "points", &pp)?, &pp)?;
            if raw.len() % 2 != 0 || raw.len() / 2 > options.limits.max_events { return Err(E::at(&pp, "curve point count must be even and bounded")); }
            let mut points = Vec::new();
            for pair in raw.chunks_exact(2) {
                let tick = integer(&pair[0], &pp)?; bounded(tick, -MAX_TICK, MAX_TICK, &pp)?;
                let value = number(&pair[1], &pp)? * 1000.0;
                if value.abs() > i64::MAX as f64 { return Err(E::at(&pp, "curve value overflow")); }
                if value != value.round() { report.issue(&pp, D::Quantized, pair[1].clone(), "Native curve values use integer millicents or millidecibels."); }
                points.push((tick, value));
            }
            apply_curve(&mut notes, &points, if key == "pitchDelta" { Curve::Pitch } else { Curve::Gain }, true, &pp)?;
            retain(parameter, &["mode", "points"], &pp, report);
        }
    }
    let group_extra = retain(group, &["name", "uuid", "notes", "parameters"], path, report);
    let ref_extra = retain(r, &["blickOffset", "pitchOffset", "groupID", "mute", "isInstrumental"], &format!("{path}/reference"), report);
    let duration = notes.iter().map(Note::end_tick).max().unwrap_or(BLICKS).max(1);
    end(start, duration, path)?;
    let muted = boolean(r, "mute", false, path)?;
    let metadata = if options.preserve_extensions { json!({"format":"svp","group":group_extra,"reference":ref_extra,
        "parameters":group.get("parameters").cloned().unwrap_or_else(||json!({})),"source_group_id":group.get("uuid"),"pitch_offset":pitch,"is_main":path.ends_with("/mainGroup"),"curve_snapshot":curve_snapshot(notes.iter(),0)})
        .as_object().expect("metadata object").clone() } else { Map::new() };
    add_part(project, ti, id.clone(), string(group, "name", "Part", path)?, start, duration, notes, metadata)?;
    let mut parts = verdant_domain::parts::Parts::load(project).map_err(|e| E::at(path, e))?;
    let index = parts.instances.iter().position(|p|p.id==id).expect("added part");
    if let Some(uuid)=group.get("uuid").and_then(Value::as_str).filter(|id|!id.is_empty()) {
        let content_id=format!("svp:group:{uuid}:pitch:{pitch}");
        let mut content=parts.contents.remove(&id).expect("new content");content.id=content_id.clone();
        let mut bindings=BTreeMap::new();
        for (i,n) in content.notes.iter_mut().enumerate(){let visible=parts.instances[index].bindings.get(&n.id).cloned().expect("new binding");n.id=verdant_domain::NoteId(format!("{content_id}:source:{i}"));bindings.insert(n.id.clone(),visible);}
        if let Some(existing)=parts.contents.get(&content_id) { if curve_snapshot(existing.notes.iter(),0)!=curve_snapshot(content.notes.iter(),0){return Err(E::at(path,"one source group identity has inconsistent note content"));} }
        else {parts.contents.insert(content_id.clone(),content);}
        parts.instances[index].content_id=content_id;parts.instances[index].bindings=bindings;
    }
    parts.instances[index].muted=muted;
    if muted { for visible in parts.instances[index].bindings.values(){if let Some(n)=project.tracks[ti].notes.iter_mut().find(|n|n.id==*visible){n.set_muted(true);}} }
    project.extensions.insert(verdant_domain::parts::KEY.into(),serde_json::to_value(parts)?);
    Ok(())
}

pub(crate) fn export(project: &Project, options: &ExportOptions, report: &mut ExchangeReport) -> Result<Vec<u8>> {
    let source = if options.preserve_extensions { project.extensions.get(KEY).and_then(Value::as_object) } else { None };
    let mut root = source.and_then(|s| s.get("root")).and_then(Value::as_object).cloned().unwrap_or_default();
    root.insert("version".into(), source.and_then(|s| s.get("version")).cloned().unwrap_or(json!(100)));
    let mut time = source.and_then(|s| s.get("time")).and_then(Value::as_object).cloned().unwrap_or_default();
    let tempos = project.tempo_map.iter().map(|t| {let mut map=if options.preserve_extensions{t.unknown.clone().into_iter().collect::<Map<_,_>>()}else{Map::new()};map.insert("position".into(),scale(t.tick,project.ppq,BLICKS,"/tempo")?.into());map.insert("bpm".into(),json!(t.micro_bpm as f64/1_000_000.0));Ok(Value::Object(map))}).collect::<Result<Vec<_>>>()?;
    time.insert("tempo".into(), tempos.into());
    time.insert("meter".into(), project.time_signatures.iter().zip(meters_to_bars(project)?).map(|(m,(index,numerator,denominator))|{let mut map=if options.preserve_extensions{m.unknown.clone().into_iter().collect::<Map<_,_>>()}else{Map::new()};for(k,v)in[("index",json!(index)),("numerator",json!(numerator)),("denominator",json!(denominator))]{map.insert(k.into(),v);}Value::Object(map)}).collect::<Vec<_>>().into());
    root.insert("time".into(), time.into());
    let mut tracks = Vec::new(); let mut library = source.and_then(|s| s.get("unused_library")).and_then(Value::as_array).cloned().unwrap_or_default();
    let mut emitted = BTreeMap::<String,Value>::new();
    let mut content_uuids = BTreeMap::<String,String>::new();
    for (ti, track) in selected(project, options) {
        let mut t = saved(&track.unknown, KEY, options.preserve_extensions);
        t.insert("name".into(), track.name.clone().into()); t.insert("dispOrder".into(), tracks.len().into());
        let mut mixer = t.get("mixer").and_then(Value::as_object).cloned().unwrap_or_default();
        mixer.insert("gainDecibel".into(), (f64::from(track.gain_millidecibels) / 1000.0).into());
        mixer.insert("pan".into(), (f64::from(track.pan_permille) / 1000.0).into()); mixer.insert("mute".into(), track.muted.into()); mixer.insert("solo".into(), track.solo.into()); t.insert("mixer".into(), mixer.into());
        let mut references = Vec::new();
        let mut has_main = false;
        for (pi, part) in parts(project, track)?.into_iter().enumerate() {
            let meta = if options.preserve_extensions && part.metadata.get("format").and_then(Value::as_str) == Some("svp") { part.metadata.clone() } else { Map::new() };
            let mut group = meta.get("group").and_then(Value::as_object).cloned().unwrap_or_default();
            let mut uuid = meta.get("source_group_id").and_then(Value::as_str).filter(|s|!s.is_empty()).map(str::to_string).unwrap_or_else(||verdant_domain::NoteId::generate().to_string());
            if let Some(previous)=content_uuids.get(&part.content_id) { uuid=previous.clone(); } else { content_uuids.insert(part.content_id.clone(),uuid.clone()); }
            let pitch_offset=meta.get("pitch_offset").and_then(Value::as_i64).unwrap_or(0);
            let unchanged_curve=project.ppq==BLICKS && meta.get("curve_snapshot")==Some(&curve_snapshot(part.notes.iter().copied(),part.start));
            if part.source_offset!=0 {report.issue(format!("/tracks/{ti}/parts/{pi}/source_offset"),D::Unsupported,part.source_offset,"SVP reference has no admitted crop window; the visible cropped notes export as an independent group.");uuid=verdant_domain::NoteId::generate().to_string();}
            group.insert("uuid".into(), uuid.clone().into()); group.insert("name".into(), part.name.clone().into());
            let mut notes = Vec::new(); let mut pitch = BTreeMap::new(); let mut gain = BTreeMap::new();
            for (ni, n) in part.notes.iter().enumerate() {
                let path = format!("/tracks/{ti}/parts/{pi}/notes/{ni}");
                let mut map = saved(&n.unknown, KEY, options.preserve_extensions);
                map.insert("onset".into(), scale(n.start_tick - part.start, project.ppq, BLICKS, &path)?.into());
                map.insert("duration".into(), scale(n.duration_tick, project.ppq, BLICKS, &path)?.into());
                map.insert("pitch".into(), bounded(i64::from(n.midi_note)-pitch_offset,0,127,&path)?.into()); map.insert("lyrics".into(), n.lyric.clone().into());
                let phoneset=meta.get("reference").and_then(|v|v.get("database")).and_then(|v|v.get("phoneset")).and_then(Value::as_str).unwrap_or("synthv-source-phoneset");
                map.insert("phonemes".into(), destination_phonemes(n,&[phoneset],report,&path).into()); map.insert("detune".into(), (f64::from(n.detune_millicents) / 1000.0).into());
                let mut attrs = map.get("attributes").and_then(Value::as_object).cloned().unwrap_or_default(); attrs.insert("muted".into(), (n.is_muted() && !part.muted).into()); map.insert("attributes".into(), attrs.into());
                if n.unknown.contains_key("midi_velocity") { report.issue(format!("{path}/midi_velocity"), D::Unsupported, json!(velocity(n)?), "SVP has no MIDI note velocity field; no consonant strength is guessed."); }
                for p in &n.curves.pitch_millicent_points {
                    insert_curve(&mut pitch, scale(n.start_tick - part.start + p.tick, project.ppq, BLICKS, &path)?, p.millicents as f64/1000.0, &path)?;
                }
                for p in &n.curves.gain_millidecibel_points {
                    insert_curve(&mut gain, scale(n.start_tick - part.start + p.tick, project.ppq, BLICKS, &path)?, p.millidecibels as f64/1000.0, &path)?;
                }
                for (key, points) in &n.curves.expression_points { if !points.is_empty() { report.issue(format!("{path}/curves/{key}"), D::Unsupported, serde_json::to_value(points)?, "Source timbre model and unit range differ; no numerical relabeling is performed."); } }
                for key in ["PIT", "DYN"] { if n.curves.interpolation(key) != verdant_domain::editing::Interpolation::Linear && !n.curves.is_empty() { report.issue(format!("{path}/curves/{key}/interpolation"), D::Unsupported, json!(n.curves.interpolation(key)), "Exported SVP nodes use linear interpolation; curve shape is not guaranteed."); } }
                notes.push(Value::Object(map));
            }
            group.insert("notes".into(), notes.into());
            let mut parameters = meta.get("parameters").and_then(Value::as_object).cloned().unwrap_or_default();
            for (name, points) in [("pitchDelta", pitch), ("loudness", gain)] {
                // An unsupported original curve may be retained when no
                // editable corresponding curve has been authored.
                if !unchanged_curve && (!points.is_empty() || !parameters.contains_key(name)) {
                    let flat: Vec<Value> = points.into_iter().flat_map(|(tick,value)| [tick.into(), json!(value)]).collect();
                    let mut parameter = parameters.get(name).and_then(Value::as_object).cloned().unwrap_or_default(); parameter.insert("mode".into(), "linear".into()); parameter.insert("points".into(), flat.into()); parameters.insert(name.into(), parameter.into());
                }
            }
            group.insert("parameters".into(), parameters.into());
            let mut reference = meta.get("reference").and_then(Value::as_object).cloned().unwrap_or_default();
            reference.insert("groupID".into(), uuid.clone().into()); reference.insert("blickOffset".into(), scale(part.start, project.ppq, BLICKS, "/parts/start")?.into()); reference.insert("pitchOffset".into(), pitch_offset.into());
            reference.insert("isInstrumental".into(), false.into()); reference.insert("mute".into(), part.muted.into());
            let is_main=meta.get("is_main").and_then(Value::as_bool)==Some(true) && !has_main;
            if is_main {t.insert("mainGroup".into(),Value::Object(group));t.insert("mainRef".into(),Value::Object(reference));has_main=true;}
            else {
                if let Some(old)=emitted.get(&uuid){
                    if old.get("name")!=group.get("name") {report.issue(format!("/tracks/{ti}/parts/{pi}/name"),D::Unsupported,part.name.clone(),"SVP linked references share one group name; the first exported group's name is retained.");if let Some(name)=old.get("name"){group.insert("name".into(),name.clone());}}
                    if old!=&Value::Object(group.clone()) {let previous=uuid;uuid=verdant_domain::NoteId::generate().to_string();group.insert("uuid".into(),uuid.clone().into());reference.insert("groupID".into(),uuid.clone().into());report.issue(format!("/tracks/{ti}/parts/{pi}/reference"),D::Metadata,previous,"Linked source references diverged in the authored score; exported an independent group with a fresh UUID.");}}
                if !emitted.contains_key(&uuid){let value=Value::Object(group);emitted.insert(uuid,value.clone());library.push(value);}references.push(Value::Object(reference));
            }
        }
        if !has_main {let main_uuid=verdant_domain::NoteId::generate().to_string();
            t.insert("mainGroup".into(),json!({"name":"main","uuid":main_uuid,"notes":[],"parameters":{}}));
            t.insert("mainRef".into(),json!({"groupID":main_uuid,"blickOffset":0,"pitchOffset":0,"isInstrumental":false}));
        }
        if options.preserve_extensions { if let Some(audio) = track.unknown.get("exchange_svp_audio").and_then(Value::as_array) { references.extend(audio.iter().cloned()); } }
        t.insert("groups".into(), references.into()); tracks.push(Value::Object(t));
    }
    root.insert("tracks".into(), tracks.into()); root.insert("library".into(), library.into());
    let mut bytes = serde_json::to_vec_pretty(&root)?; bytes.push(0); Ok(bytes)
}
fn insert_curve(points: &mut BTreeMap<i64, f64>, tick: i64, value: f64, path: &str) -> Result<()> {
    if let Some(old) = points.insert(tick, value) { if old != value { return Err(E::at(path, "overlapping notes require incompatible group-level parameter values")); } } Ok(())
}
