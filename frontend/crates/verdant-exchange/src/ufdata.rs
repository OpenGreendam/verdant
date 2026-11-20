//! UtaFormatix Data v1. The upstream data model and converter are authoritative:
//! https://github.com/sdercolin/utaformatix-data/tree/609287a0ac8661a1f9c92ee3dc5906820c9a26eb
//! https://github.com/sdercolin/utaformatix3/blob/91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae/core/src/main/kotlin/core/io/UfData.kt
//! Ticks are 1/480 quarter note; pitch values are semitones, held until the
//! next point. Absolute `null` ends an override. `measurePrefix` describes
//! source-editor padding already removed from these ticks; it is never added
//! to or subtracted from the editable score here.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use verdant_domain::editing::{Interpolation, interpolate};
use verdant_domain::model::{PitchPoint, TempoPoint};
use verdant_domain::{Note, PointId, Project, PronunciationSource, Track};

use crate::common::*;
use crate::{Disposition as D, ExchangeError as E, ExchangeReport, ExportOptions, ImportOptions, Result};

const KEY: &str = "exchange_ufdata";
const PPQ: i64 = 480;
const PHONEME_ALPHABET: &str = "ufdata-source-phoneme";
const MILLICENTS_PER_SEMITONE: f64 = 100_000.0;

struct Budget {
    used: usize,
    limit: usize,
}
impl Budget {
    fn add(&mut self, count: usize, path: &str) -> Result<()> {
        self.used = self.used.checked_add(count).ok_or_else(|| E::at(path, "event count overflow"))?;
        if self.used > self.limit {
            return Err(E::at(path, "ufdata event/sampling limit exceeded"));
        }
        Ok(())
    }
}

struct Pitch {
    points: Vec<(i64, Option<f64>)>,
    absolute: bool,
}

fn read_pitch(value: Option<&Value>, path: &str, budget: &mut Budget, report: &mut ExchangeReport) -> Result<Pitch> {
    // The published README permits nullable pitch; the current Kotlin model
    // uses an empty Pitch() instead. Admit both and emit the canonical object.
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return Ok(Pitch { points: Vec::new(), absolute: false });
    };
    let map = object(value, path)?;
    let ticks = map.get("ticks").map(|v| array(v, &format!("{path}/ticks"))).transpose()?.unwrap_or(&[]);
    let values = map.get("values").map(|v| array(v, &format!("{path}/values"))).transpose()?.unwrap_or(&[]);
    if ticks.len() != values.len() {
        return Err(E::at(path, "pitch ticks and values must have equal lengths"));
    }
    budget.add(ticks.len(), path)?;
    let absolute = boolean(map, "isAbsolute", false, path)?;
    let mut points = Vec::with_capacity(ticks.len());
    for (index, (tick, value)) in ticks.iter().zip(values).enumerate() {
        let tp = format!("{path}/ticks/{index}");
        let tick = bounded(integer(tick, &tp)?, -MAX_TICK, MAX_TICK, &tp)?;
        if points.last().is_some_and(|p: &(i64, Option<f64>)| p.0 >= tick) {
            return Err(E::at(&tp, "pitch ticks must strictly increase"));
        }
        let vp = format!("{path}/values/{index}");
        let value = if value.is_null() {
            if !absolute {
                return Err(E::at(&vp, "null pitch is only defined for isAbsolute=true"));
            }
            None
        } else {
            let value = number(value, &vp)?;
            // Reject impossible native conversion even for a point in a gap.
            if value.abs() * MILLICENTS_PER_SEMITONE >= i64::MAX as f64 - 127.0 * MILLICENTS_PER_SEMITONE {
                return Err(E::at(&vp, "pitch exceeds the native millicent range"));
            }
            Some(value)
        };
        points.push((tick, value));
    }
    retain(map, &["ticks", "values", "isAbsolute"], path, report);
    Ok(Pitch { points, absolute })
}

fn pitch_snapshot(track: &Track, ppq: i64) -> Value {
    let mut notes: Vec<_> = track.notes.iter().collect();
    notes.sort_by_key(|n| (n.start_tick, n.duration_tick, n.midi_note));
    json!({"ppq":ppq,"notes":notes.into_iter().map(|n| json!({
        "start":n.start_tick,"duration":n.duration_tick,"key":n.midi_note,"detune":n.detune_millicents,
        "pitch":n.curves.pitch_millicent_points.iter().map(|p|(p.tick,p.millicents)).collect::<Vec<_>>(),
        "interpolation":n.curves.interpolation("PIT")
    })).collect::<Vec<_>>()})
}

fn phoneme_snapshot(note: &Note) -> Value {
    json!({"phonemes":note.pronunciation.phonemes,"alphabet":note.pronunciation.alphabet,
        "language":note.pronunciation.language,"pinyin":note.pronunciation.pinyin})
}

fn materialize_pitch(notes: &mut [Note], pitch: &Pitch, path: &str, budget: &mut Budget, report: &mut ExchangeReport) -> Result<()> {
    if pitch.points.is_empty() {
        return Ok(());
    }
    for (ni, note) in notes.iter_mut().enumerate() {
        let at_start = pitch.points.partition_point(|p| p.0 <= note.start_tick);
        let at_end = pitch.points.partition_point(|p| p.0 < note.end_tick());
        let mut nodes = Vec::new();
        let held = at_start.checked_sub(1).and_then(|i| pitch.points[i].1);
        nodes.push((0, held));
        nodes.extend(pitch.points[at_start..at_end].iter().map(|(tick, value)| (*tick - note.start_tick, *value)));
        // A terminal anchor is not audible beyond the note. Use the value
        // immediately before its end, so the following note cannot bend it.
        nodes.push((note.duration_tick, nodes.last().expect("start anchor").1));
        budget.add(nodes.len(), path)?;
        let mut converted = Vec::with_capacity(nodes.len());
        for (tick, value) in nodes {
            let semitones = match value {
                Some(v) if pitch.absolute => v - f64::from(note.midi_note),
                Some(v) => v,
                None => 0.0,
            };
            let millicents = semitones * MILLICENTS_PER_SEMITONE;
            if !millicents.is_finite() || millicents.abs() >= i64::MAX as f64 {
                return Err(E::at(path, "pitch conversion overflow"));
            }
            if (millicents - millicents.round()).abs() > 1e-7 {
                report.issue(
                    format!("{path}/notes/{ni}/pitch/{tick}"),
                    D::Quantized,
                    json!(value),
                    "Editable pitch uses integer millicents; when extension retention is enabled, the original source curve is retained for unchanged compatible export.",
                );
            }
            converted.push(PitchPoint { id: PointId::generate(), tick, millicents: millicents.round() as i64, unknown: BTreeMap::new() });
        }
        note.curves.pitch_millicent_points = converted;
        note.curves.set_interpolation("PIT", Interpolation::Hold);
    }
    Ok(())
}

fn ensure_monophonic(notes: &[Note], path: &str) -> Result<()> {
    for pair in notes.windows(2) {
        if pair[0].end_tick() > pair[1].start_tick {
            return Err(E::at(path, "overlapping notes require separate ufdata tracks; no note is silently shortened"));
        }
    }
    Ok(())
}

pub(crate) fn import(bytes: &[u8], options: &ImportOptions, report: &mut ExchangeReport) -> Result<Project> {
    let value = parse_json(bytes, false)?;
    let root = object(&value, "")?;
    let version = integer(required(root, "formatVersion", "")?, "/formatVersion")?;
    if version != 1 {
        return Err(E::at("/formatVersion", format!("unsupported ufdata formatVersion {version}; supported version is 1")));
    }
    let source = object(required(root, "project", "")?, "/project")?;
    let title = text(required(source, "name", "/project")?, "/project/name")?;
    let prefix = bounded(int(source, "measurePrefix", 0, "/project")?, 0, i64::from(i32::MAX), "/project/measurePrefix")?;
    let mut project = source_project(PPQ);
    project.unknown.insert("title".into(), title.into());
    report.source_ppq = PPQ;
    let mut budget = Budget { used: 0, limit: options.limits.max_events };

    let raw_tempos = array(required(source, "tempos", "/project")?, "/project/tempos")?;
    budget.add(raw_tempos.len(), "/project/tempos")?;
    project.tempo_map.clear();
    for (index, raw) in raw_tempos.iter().enumerate() {
        let path = format!("/project/tempos/{index}");
        let map = object(raw, &path)?;
        let tick = bounded(integer(required(map, "tickPosition", &path)?, &format!("{path}/tickPosition"))?, 0, MAX_TICK, &path)?;
        if index == 0 && tick != 0 || project.tempo_map.last().is_some_and(|p| p.tick >= tick) {
            return Err(E::at(&path, "tempo map must start at zero and strictly increase"));
        }
        let bpm = number(required(map, "bpm", &path)?, &format!("{path}/bpm"))?;
        let mut point = TempoPoint { tick, micro_bpm: micro_bpm(bpm, &format!("{path}/bpm"), report)?, unknown: BTreeMap::new() };
        let extras = retain(map, &["tickPosition", "bpm"], &path, report);
        if options.preserve_extensions && !extras.is_empty() {
            point.unknown.insert(KEY.into(), extras.into());
        }
        project.tempo_map.push(point);
    }

    let raw_meters = array(required(source, "timeSignatures", "/project")?, "/project/timeSignatures")?;
    budget.add(raw_meters.len(), "/project/timeSignatures")?;
    let mut bars = Vec::new();
    let mut meter_extras = Vec::new();
    for (index, raw) in raw_meters.iter().enumerate() {
        let path = format!("/project/timeSignatures/{index}");
        let map = object(raw, &path)?;
        let bar = bounded(integer(required(map, "measurePosition", &path)?, &path)?, 0, i64::from(i32::MAX), &path)?;
        if index == 0 && bar != 0 || bars.last().is_some_and(|p: &(i64, u32, u32)| p.0 >= bar) {
            return Err(E::at(&path, "meter measures must start at zero and strictly increase"));
        }
        let numerator = bounded(integer(required(map, "numerator", &path)?, &path)?, 1, 64, &path)? as u32;
        let denominator = bounded(integer(required(map, "denominator", &path)?, &path)?, 1, 64, &path)? as u32;
        bars.push((bar, numerator, denominator));
        meter_extras.push(retain(map, &["measurePosition", "numerator", "denominator"], &path, report));
    }
    project.time_signatures = meters_from_bars(bars, PPQ)?;
    for (point, extras) in project.time_signatures.iter_mut().zip(meter_extras) {
        bounded(point.tick, 0, MAX_TICK, "/project/timeSignatures")?;
        if options.preserve_extensions && !extras.is_empty() {
            point.unknown.insert(KEY.into(), extras.into());
        }
    }
    if raw_tempos.is_empty() {
        report.issue("/project/tempos", D::Metadata, json!([]), "Empty tempo map uses 120 BPM at tick zero.");
    }
    if raw_meters.is_empty() {
        report.issue("/project/timeSignatures", D::Metadata, json!([]), "Empty meter map uses 4/4 at measure zero.");
    }
    default_maps(&mut project);

    let raw_tracks = source.get("tracks").map(|v| array(v, "/project/tracks")).transpose()?.unwrap_or(&[]);
    if raw_tracks.len() > options.limits.max_tracks {
        return Err(E::at("/project/tracks", "track limit exceeded"));
    }
    let mut note_count = 0usize;
    for (ti, raw) in raw_tracks.iter().enumerate() {
        let path = format!("/project/tracks/{ti}");
        let map = object(raw, &path)?;
        let name = text(required(map, "name", &path)?, &format!("{path}/name"))?;
        let mut track = source_track(name);
        track.monophonic = true;
        let notes = map.get("notes").map(|v| array(v, &format!("{path}/notes"))).transpose()?.unwrap_or(&[]);
        note_count = note_count.checked_add(notes.len()).ok_or_else(|| E::at(&path, "note count overflow"))?;
        if note_count > options.limits.max_notes {
            return Err(E::at(&path, "note limit exceeded"));
        }
        for (ni, raw) in notes.iter().enumerate() {
            let np = format!("{path}/notes/{ni}");
            let note_map = object(raw, &np)?;
            let start = bounded(integer(required(note_map, "tickOn", &np)?, &format!("{np}/tickOn"))?, 0, MAX_TICK, &np)?;
            let end_tick = bounded(integer(required(note_map, "tickOff", &np)?, &format!("{np}/tickOff"))?, 0, MAX_TICK, &np)?;
            let mut n = note(
                start,
                end_tick - start,
                integer(required(note_map, "key", &np)?, &format!("{np}/key"))?,
                string(note_map, "lyric", "", &np)?,
                &np,
            )?;
            let phoneme = note_map.get("phoneme").cloned().unwrap_or(Value::Null);
            if !phoneme.is_null() {
                let value = text(&phoneme, &format!("{np}/phoneme"))?;
                if !value.is_empty() {
                    // No dictionary/alphabet is declared by ufdata. Keep the
                    // entire string (including spaces) as an opaque edit.
                    n.pronunciation.phonemes = vec![value.into()];
                    n.pronunciation.alphabet = Some(PHONEME_ALPHABET.into());
                    n.pronunciation.input_text = Some(n.lyric.clone());
                    n.pronunciation.source = PronunciationSource::Imported;
                    n.pronunciation.frontend_profile = "exchange-ufdata-opaque-phoneme/1".into();
                    report.issue(format!("{np}/phoneme"), D::Unsupported, value,
                        "Phoneme text is editable and preserved exactly. Ufdata declares no alphabet or language; inspect or replace the reading before synthesis.");
                }
            }
            let extra = retain(note_map, &["key", "tickOn", "tickOff", "lyric", "phoneme"], &np, report);
            if options.preserve_extensions {
                n.unknown.insert(KEY.into(), json!({"fields":extra,"phoneme":phoneme,"phoneme_snapshot":phoneme_snapshot(&n)}));
            }
            track.notes.push(n);
        }
        if track.notes.windows(2).any(|w| w[0].start_tick > w[1].start_tick) {
            report.issue(
                format!("{path}/notes"),
                D::Metadata,
                "sorted",
                "Notes were ordered by tick without changing their timing or content.",
            );
        }
        track.notes.sort_by_key(|n| n.start_tick);
        ensure_monophonic(&track.notes, &format!("{path}/notes"))?;
        let pitch = read_pitch(map.get("pitch"), &format!("{path}/pitch"), &mut budget, report)?;
        let outside = pitch
            .points
            .iter()
            .filter(|(tick, _)| {
                let i = track.notes.partition_point(|n| n.start_tick <= *tick);
                i == 0 || track.notes[i - 1].end_tick() <= *tick
            })
            .count();
        if outside > 0 {
            report.issue(format!("{path}/pitch/off_note_points"), D::RetainedUnsupported, outside,
                "Track pitch outside note intervals has no editable note owner. It is retained for unchanged compatible export when extension retention is enabled.");
        }
        materialize_pitch(&mut track.notes, &pitch, &path, &mut budget, report)?;
        let extra = retain(map, &["name", "notes", "pitch"], &path, report);
        if options.preserve_extensions {
            track.unknown.insert(KEY.into(), json!({"fields":extra,"pitch":map.get("pitch").cloned().unwrap_or(Value::Null),"pitch_snapshot":pitch_snapshot(&track, PPQ)}));
        }
        project.tracks.push(track);
    }

    let root_extra = retain(root, &["formatVersion", "project"], "", report);
    let project_extra = retain(source, &["name", "tracks", "timeSignatures", "tempos", "measurePrefix"], "/project", report);
    if prefix != 0 {
        report.issue("/project/measurePrefix", if options.preserve_extensions { D::Metadata } else { D::Unsupported }, prefix,
            "Source-editor leading-measure preference; ufdata ticks already exclude that padding. Editable note/tempo/meter positions are not shifted.");
    }
    if options.preserve_extensions {
        project.extensions.insert(KEY.into(), json!({"root":root_extra,"project":project_extra,"measurePrefix":prefix}));
    }
    Ok(project)
}

fn retained_fields(value: Option<&Value>, path: &str, report: &mut ExchangeReport) -> Map<String, Value> {
    let map = value.and_then(Value::as_object).cloned().unwrap_or_default();
    for (key, value) in &map {
        report.issue(
            format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
            D::RetainedUnsupported,
            value.clone(),
            "Source extension retained in the compatible ufdata object without executable semantics.",
        );
    }
    map
}

fn destination_tick(tick: i64, source_ppq: i64, path: &str, report: &mut ExchangeReport) -> Result<i64> {
    let numerator = i128::from(tick) * i128::from(PPQ);
    let denominator = i128::from(source_ppq);
    let rounded =
        if numerator >= 0 { (numerator + denominator / 2) / denominator } else { -((-numerator + denominator / 2) / denominator) };
    let result = i64::try_from(rounded)
        .ok()
        .filter(|v| (-MAX_TICK..=MAX_TICK).contains(v))
        .ok_or_else(|| E::at(path, "ufdata tick scaling overflow"))?;
    if numerator % denominator != 0 {
        report.issue(path, D::Quantized, tick, format!("Rounded a PPQ-{source_ppq} position to tick {result} at ufdata's fixed 480 PPQ."));
    }
    Ok(result)
}

fn pitch_value(note: &Note, relative_tick: f64) -> Result<Option<f64>> {
    let delta = interpolate(
        &note.curves.pitch_millicent_points,
        relative_tick,
        0.0,
        note.curves.interpolation("PIT"),
        |p| p.tick,
        |p| p.millicents as f64,
    ) + f64::from(note.detune_millicents);
    let value = f64::from(note.midi_note) + delta / MILLICENTS_PER_SEMITONE;
    if !value.is_finite() || value.abs() * MILLICENTS_PER_SEMITONE >= i64::MAX as f64 - 127.0 * MILLICENTS_PER_SEMITONE {
        return Err(E::at("/pitch", "native pitch interpolation exceeds the supported millicent range"));
    }
    // In the upstream converter relative zero means the destination's
    // unmodified note pitch, represented by absolute null.
    Ok((delta != 0.0).then_some(value))
}

fn generated_pitch(track: &Track, ppq: i64, path: &str, budget: &mut Budget, report: &mut ExchangeReport) -> Result<Value> {
    let mut notes: Vec<_> = track.notes.iter().collect();
    notes.sort_by_key(|n| n.start_tick);
    let mut points = BTreeMap::<i64, Option<f64>>::new();
    for (ni, note) in notes.iter().enumerate() {
        let np = format!("{path}/notes/{ni}/pitch");
        // Timing diagnostics are emitted for the actual note fields below.
        let start = rounded_tick(note.start_tick, ppq)?;
        let end_tick = rounded_tick(note.end_tick(), ppq)?;
        let curve = &note.curves.pitch_millicent_points;
        let changing = curve.windows(2).any(|p| p[0].millicents != p[1].millicents);
        if changing && note.curves.interpolation("PIT") != Interpolation::Hold {
            let samples = usize::try_from(end_tick - start).map_err(|_| E::at(&np, "pitch sampling length overflow"))?;
            budget.add(samples.saturating_add(1), &np)?;
            report.issue(&np, D::Quantized, json!({"interpolation":note.curves.interpolation("PIT"),"samples":samples}),
                "A continuous native pitch curve is sampled at every 1/480-quarter-note tick and held between ticks; automatic vibrato and portamento are not baked into it.");
            for tick in start..end_tick {
                let relative_numerator = i128::from(tick) * i128::from(ppq) - i128::from(note.start_tick) * i128::from(PPQ);
                let relative = (relative_numerator as f64 / PPQ as f64).clamp(0.0, note.duration_tick as f64);
                points.insert(tick, pitch_value(note, relative)?);
            }
        } else {
            budget.add(curve.len().saturating_add(2), &np)?;
            points.insert(start, pitch_value(note, 0.0)?);
            for point in curve {
                if point.tick == note.duration_tick {
                    continue;
                }
                let absolute = note.start_tick.checked_add(point.tick).ok_or_else(|| E::at(&np, "pitch tick overflow"))?;
                let tick = destination_tick(absolute, ppq, &format!("{np}/{}", point.tick), report)?;
                if tick >= start && tick < end_tick {
                    points.insert(tick, pitch_value(note, point.tick as f64)?);
                }
            }
        }
        points.insert(end_tick, None);
    }
    let mut ticks = Vec::new();
    let mut values = Vec::new();
    let mut previous = None;
    for (tick, value) in points {
        // Missing initial data already means default pitch. Null resets at
        // gaps/adjacent notes remain whenever the preceding value was active.
        if value == previous {
            continue;
        }
        ticks.push(tick);
        values.push(value);
        previous = value;
    }
    Ok(json!({"ticks":ticks,"values":values,"isAbsolute":true}))
}

fn rounded_tick(tick: i64, ppq: i64) -> Result<i64> {
    let value = (i128::from(tick) * i128::from(PPQ) + i128::from(ppq) / 2) / i128::from(ppq);
    i64::try_from(value).ok().filter(|v| (0..=MAX_TICK).contains(v)).ok_or_else(|| E::at("/ticks", "ufdata tick scaling overflow"))
}

fn report_native_track(track: &Track, options: &ExportOptions, path: &str, report: &mut ExchangeReport) {
    for (key, value, active) in [
        ("gain_millidecibels", json!(track.gain_millidecibels), track.gain_millidecibels != 0),
        ("pan_permille", json!(track.pan_permille), track.pan_permille != 0),
        ("muted", json!(track.muted), track.muted),
        ("solo", json!(track.solo), track.solo),
        ("voice_ref", json!(track.voice_ref), track.voice_ref.is_some()),
        ("render_route", json!(track.render_route), track.render_route.is_some()),
    ] {
        if active {
            report.issue(
                format!("{path}/{key}"),
                D::Unsupported,
                value,
                "Ufdata carries a score and track pitch, without mixer or synthesis-engine settings.",
            );
        }
    }
    for (key, value) in &track.unknown {
        if key != KEY || !options.preserve_extensions {
            report.issue(
                format!("{path}/{key}"),
                D::Unsupported,
                value.clone(),
                "Native/source track metadata is not retained by this export.",
            );
        }
    }
}

fn time_fields(unknown: &BTreeMap<String, Value>, options: &ExportOptions, path: &str, report: &mut ExchangeReport) -> Map<String, Value> {
    for (key, value) in unknown {
        if key != KEY || !options.preserve_extensions {
            report.issue(
                format!("{path}/{key}"),
                D::Unsupported,
                value.clone(),
                "Source tempo/meter extension is not retained by this export.",
            );
        }
    }
    retained_fields(options.preserve_extensions.then(|| unknown.get(KEY)).flatten(), path, report)
}

pub(crate) fn export(project: &Project, options: &ExportOptions, report: &mut ExchangeReport) -> Result<Vec<u8>> {
    let source = if options.preserve_extensions { project.extensions.get(KEY) } else { None };
    let mut root = retained_fields(source.and_then(|s| s.get("root")), "", report);
    let mut output = retained_fields(source.and_then(|s| s.get("project")), "/project", report);
    let title = project.unknown.get("title").map(|v| text(v, "/title")).transpose()?.unwrap_or(&project.project_id);
    output.insert("name".into(), title.into());
    let prefix = source.and_then(|s| s.get("measurePrefix")).map(|v| integer(v, "/project/measurePrefix")).transpose()?.unwrap_or(0);
    bounded(prefix, 0, i64::from(i32::MAX), "/project/measurePrefix")?;
    output.insert("measurePrefix".into(), prefix.into());
    if prefix != 0 {
        report.issue(
            "/project/measurePrefix",
            D::Metadata,
            prefix,
            "Retained source-editor padding preference; exported tick positions remain unchanged.",
        );
    }
    let mut budget = Budget { used: 0, limit: options.limits.max_events };
    budget.add(project.tempo_map.len().saturating_add(project.time_signatures.len()), "/project/time")?;
    let mut tempos = Vec::new();
    let mut last_tick = None;
    for (index, tempo) in project.tempo_map.iter().enumerate() {
        let path = format!("/project/tempos/{index}");
        let tick = destination_tick(tempo.tick, project.ppq, &format!("{path}/tickPosition"), report)?;
        if last_tick.is_some_and(|last| last >= tick) {
            return Err(E::at(&path, "tempo events collide at ufdata's 480-PPQ resolution"));
        }
        last_tick = Some(tick);
        let bpm = tempo.micro_bpm as f64 / 1_000_000.0;
        if !(0.000001..=1000.0).contains(&bpm) {
            return Err(E::at(&path, "tempo must be positive and at most 1000 BPM"));
        }
        let mut map = time_fields(&tempo.unknown, options, &path, report);
        map.insert("tickPosition".into(), tick.into());
        map.insert("bpm".into(), json!(bpm));
        tempos.push(Value::Object(map));
    }
    output.insert("tempos".into(), tempos.into());
    let bars = meters_to_bars(project)?;
    let mut meters = Vec::new();
    for (index, (source_meter, (bar, numerator, denominator))) in project.time_signatures.iter().zip(bars).enumerate() {
        let path = format!("/project/timeSignatures/{index}");
        bounded(bar, 0, i64::from(i32::MAX), &path)?;
        let mut map = time_fields(&source_meter.unknown, options, &path, report);
        map.insert("measurePosition".into(), bar.into());
        map.insert("numerator".into(), numerator.into());
        map.insert("denominator".into(), denominator.into());
        meters.push(Value::Object(map));
    }
    output.insert("timeSignatures".into(), meters.into());
    let mut tracks = Vec::new();
    for (ti, track) in selected(project, options) {
        let path = format!("/project/tracks/{ti}");
        report_native_track(track, options, &path, report);
        let metadata = if options.preserve_extensions { track.unknown.get(KEY) } else { None };
        let mut map = retained_fields(metadata.and_then(|s| s.get("fields")), &path, report);
        map.insert("name".into(), track.name.clone().into());
        let mut notes: Vec<_> = track.notes.iter().collect();
        notes.sort_by_key(|n| n.start_tick);
        for pair in notes.windows(2) {
            if pair[0].end_tick() > pair[1].start_tick {
                return Err(E::at(&path, "overlapping notes require separate ufdata tracks; no note is silently shortened"));
            }
        }
        let mut encoded_notes = Vec::new();
        for (ni, note) in notes.into_iter().enumerate() {
            let np = format!("{path}/notes/{ni}");
            let start = destination_tick(note.start_tick, project.ppq, &format!("{np}/tickOn"), report)?;
            let end_tick = destination_tick(note.end_tick(), project.ppq, &format!("{np}/tickOff"), report)?;
            if end_tick <= start {
                return Err(E::at(&np, "note collapses at ufdata's 480-PPQ resolution; no note was dropped"));
            }
            let saved = if options.preserve_extensions { note.unknown.get(KEY) } else { None };
            let mut n = retained_fields(saved.and_then(|s| s.get("fields")), &np, report);
            n.insert("key".into(), note.midi_note.into());
            n.insert("tickOn".into(), start.into());
            n.insert("tickOff".into(), end_tick.into());
            n.insert("lyric".into(), note.lyric.clone().into());
            let phoneme = if saved.and_then(|s| s.get("phoneme_snapshot")) == Some(&phoneme_snapshot(note)) {
                let value = saved.and_then(|s| s.get("phoneme")).cloned().unwrap_or(Value::Null);
                if !value.is_null() {
                    text(&value, &format!("{np}/phoneme"))?;
                }
                value
            } else if note.pronunciation.phonemes.is_empty() {
                Value::Null
            } else {
                note.pronunciation.phonemes.join(" ").into()
            };
            n.insert("phoneme".into(), phoneme);
            if !note.pronunciation.phonemes.is_empty()
                && (note.pronunciation.alphabet.as_deref() != Some(PHONEME_ALPHABET) || note.pronunciation.language.is_some())
            {
                report.issue(format!("{np}/pronunciation"), D::Unsupported, json!({"alphabet":note.pronunciation.alphabet,"language":note.pronunciation.language}),
                    "Phoneme text is exported verbatim, but ufdata cannot declare its alphabet or language. The destination must interpret or replace that reading.");
            }
            if !note.curves.gain_millidecibel_points.is_empty() {
                report.issue(
                    format!("{np}/curves/DYN"),
                    D::Unsupported,
                    serde_json::to_value(&note.curves.gain_millidecibel_points)?,
                    "Ufdata has no dynamics curve.",
                );
            }
            for (key, points) in &note.curves.expression_points {
                if !points.is_empty() {
                    report.issue(
                        format!("{np}/curves/{key}"),
                        D::Unsupported,
                        serde_json::to_value(points)?,
                        "Ufdata has no matching expression/timbre controller.",
                    );
                }
            }
            for (key, value) in &note.unknown {
                if key != KEY && !key.starts_with("exchange_") && !matches!(key.as_str(), "muted" | "source_note_ids") {
                    report.issue(
                        format!("{np}/{key}"),
                        D::Unsupported,
                        value.clone(),
                        "Native/source note extension has no ufdata field; the lyric and note remain editable.",
                    );
                }
            }
            encoded_notes.push(Value::Object(n));
        }
        map.insert("notes".into(), encoded_notes.into());
        let pitch = if metadata.and_then(|s| s.get("pitch_snapshot")) == Some(&pitch_snapshot(track, project.ppq)) {
            let value = metadata.and_then(|s| s.get("pitch"));
            read_pitch(value, &format!("{path}/pitch"), &mut budget, report)?;
            value.filter(|v| !v.is_null()).cloned().unwrap_or_else(|| json!({"ticks":[],"values":[],"isAbsolute":false}))
        } else {
            if metadata
                .and_then(|s| s.get("pitch"))
                .is_some_and(|v| v.get("ticks").and_then(Value::as_array).is_some_and(|v| !v.is_empty()))
            {
                report.issue(format!("{path}/pitch"), D::Unsupported, metadata.and_then(|s| s.get("pitch")).cloned().unwrap_or(Value::Null),
                    "The note timing/key or editable pitch changed. Pitch is regenerated from current notes; original off-note data and source precision are not replayed over those edits.");
            }
            generated_pitch(track, project.ppq, &path, &mut budget, report)?
        };
        map.insert("pitch".into(), pitch);
        tracks.push(Value::Object(map));
    }
    output.insert("tracks".into(), tracks.into());
    for (key, value) in &project.extensions {
        if key != KEY
            && key != "score_import"
            && !key.starts_with("exchange_")
            && !matches!(key.as_str(), "automation" | "master_inserts" | "media" | "plugins" | "master_gain_millidecibels")
        {
            report.issue(
                format!("/extensions/{key}"),
                D::Unsupported,
                value.clone(),
                "Ufdata does not carry this native document extension; parts are flattened to the visible notes.",
            );
        }
    }
    for (key, value) in &project.unknown {
        if key != "title" {
            report.issue(format!("/{key}"), D::Unsupported, value.clone(), "Native document metadata has no ufdata field.");
        }
    }
    root.insert("formatVersion".into(), 1.into());
    root.insert("project".into(), output.into());
    Ok(serde_json::to_vec_pretty(&root)?)
}
