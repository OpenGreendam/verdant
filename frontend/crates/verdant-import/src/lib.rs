//! In-process UST/USTX import. No source tool, resampler, plugin or Python is
//! invoked. Unsupported source controls survive in the embedded import report.
mod controls;
mod strict_yaml;
mod ust;
mod ustx;

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use verdant_domain::{Note, NoteId, Project, Pronunciation, PronunciationSource, Track, TrackId, PPQ};
use verdant_domain::model::{TempoPoint, TimeSignature};

pub type Result<T> = std::result::Result<T, ImportError>;
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{location}: {message}")]
    Invalid { location: String, message: String },
    #[error("invalid USTX YAML: {0}")]
    Yaml(#[source] serde_yaml::Error),
    #[error("import JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
impl ImportError {
    pub(crate) fn at(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid { location: location.into(), message: message.into() }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat { Ust, Ustx }
impl ImportFormat {
    /// Resolve only the formats handled by this importer. Other score formats
    /// belong to verdant-exchange, with their own timing and metadata rules.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "ust" => Some(Self::Ust), "ustx" => Some(Self::Ustx), _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UstEncoding { #[default] Auto, Cp932, Utf8 }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LyricMode {
    /// Original text with an unset reading. No pronunciation is guessed.
    #[default]
    Preserve,
    /// Caller identifies source lyrics as pinyin. No lexical tone is invented;
    /// numbered and unnumbered spellings are both retained exactly.
    Pinyin,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReadingOverrides {
    /// Event IDs from the report, e.g. t0/p0/n12 (UST uses p0).
    pub by_source_id: BTreeMap<String, Pronunciation>,
    pub by_lyric: BTreeMap<String, Pronunciation>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImportOptions {
    pub title: Option<String>,
    /// Display name; the library reads only the supplied bytes.
    pub source_name: Option<String>,
    /// Empty selects every vocal track, including empty tracks.
    pub tracks: Vec<usize>,
    pub ust_encoding: UstEncoding,
    pub lyric_mode: LyricMode,
    pub readings: ReadingOverrides,
    pub extra_rest_lyrics: BTreeSet<String>,
    /// SHA-256 of original bytes, checked before decoding.
    pub expected_source_sha256: Option<String>,
    pub credits: Vec<String>,
}
#[derive(Debug)]
pub struct ImportResult { pub project: Project, pub report: ImportReport }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub format: ImportFormat,
    pub name: Option<String>,
    pub sha256: String,
    pub byte_length: usize,
    pub encoding: String,
    pub ppq: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceTempo { pub tick: i64, pub bpm: f64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMeter { pub tick: i64, pub numerator: u32, pub denominator: u32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldDisposition {
    /// Actively represented by a domain field or curve, possibly sampled.
    Translated,
    /// Original value retained, without executable semantics.
    RetainedUnsupported,
    /// Source metadata or superseded compatibility data.
    Metadata,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldReport {
    pub location: String,
    pub disposition: FieldDisposition,
    pub source_value: Value,
    pub target: Option<String>,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportNotice { pub code: String, pub location: String, pub message: String }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind { Note, Rest, Gap, Continuation }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingDisposition {
    Unresolved, DeclaredSourcePinyin, InspectedSourceIdOverride, InspectedLyricOverride, Continuation, Silence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceReadingEvidence { LyricOnly, VoicebankPhonemeAlias, NotApplicable }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventMapping {
    pub source_id: String,
    pub source_track_index: usize,
    pub source_part_index: usize,
    pub source_note_index: usize,
    pub kind: EventKind,
    pub source_start_tick: i64,
    pub source_duration_tick: i64,
    pub source_midi_note: Option<i32>,
    pub source_lyric: String,
    /// Source aliases are evidence, not portable IPA or an inferred reading.
    pub source_reading_evidence: SourceReadingEvidence,
    pub reading: ReadingDisposition,
    pub project_start_tick: i64,
    pub project_duration_tick: i64,
    pub project_note_id: Option<NoteId>,
    /// The previous adjacent project note whose nucleus this event sustains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_of_project_note_id: Option<NoteId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackReport {
    pub source_track_index: usize,
    pub project_track_id: TrackId,
    pub name: String,
    pub source_event_count: usize,
    pub project_note_count: usize,
    pub rest_count: usize,
    pub continuation_count: usize,
    pub unresolved_reading_count: usize,
    pub source_end_tick: i64,
    pub midi_range: Option<[i32; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub schema: String,
    pub source: SourceIdentity,
    pub title: String,
    pub source_tempos: Vec<SourceTempo>,
    pub source_time_signatures: Vec<SourceMeter>,
    pub lyric_mode: LyricMode,
    pub tracks: Vec<TrackReport>,
    pub events: Vec<EventMapping>,
    pub fields: Vec<FieldReport>,
    pub notices: Vec<ImportNotice>,
    pub duration_seconds: f64,
    pub credits: Vec<String>,
    pub source_file_copied: bool,
}
impl ImportReport {
    /// Exact original source IDs/lyrics still needing an authored reading.
    pub fn unresolved_readings(&self) -> impl Iterator<Item = &EventMapping> {
        self.events.iter().filter(|e| e.reading == ReadingDisposition::Unresolved)
    }
    pub fn unsupported_fields(&self) -> impl Iterator<Item = &FieldReport> {
        self.fields.iter().filter(|f| f.disposition == FieldDisposition::RetainedUnsupported)
    }
}
/// The manifest consumed by verdant-app --audition-dir. No audio is generated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditionCatalog { pub schema: String, pub projects: Vec<AuditionEntry> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditionEntry { pub title: String, pub path: String }
impl ImportResult {
    pub fn audition_catalog(&self, project_path: impl Into<String>) -> AuditionCatalog {
        AuditionCatalog { schema: "verdant-audition-catalog/1".into(), projects: vec![AuditionEntry {
            title: self.report.title.clone(), path: project_path.into(),
        }] }
    }
}

pub(crate) struct Source {
    ppq: i64,
    encoding: String,
    title: String,
    tempos: Vec<SourceTempo>,
    meters: Vec<SourceMeter>,
    tracks: Vec<SourceTrack>,
    fields: Vec<FieldReport>,
    notices: Vec<ImportNotice>,
}
pub(crate) struct SourceTrack {
    index: usize,
    name: String,
    notes: Vec<SourceNote>,
    end_tick: i64,
    gain_db: f64,
    pan: i32,
    muted: bool,
    solo: bool,
    curves: Vec<controls::PartCurve>,
}
pub(crate) struct SourceNote {
    part: usize,
    index: usize,
    start: i64,
    duration: i64,
    midi: i32,
    lyric: String,
    source_reading: SourceReadingEvidence,
    control: controls::NoteControl,
}
impl SourceNote {
    fn source_id(&self, track: usize) -> String { format!("t{track}/p{}/n{}", self.part, self.index) }
}

/// Import original bytes into a 960-PPQ editable domain project. Rejects
/// ambiguous syntax, invalid tempo maps, unrepresentable note boundaries and
/// overlaps. Track selection, source identity and reading decisions are explicit.
pub fn import(format: ImportFormat, bytes: &[u8], options: &ImportOptions) -> Result<ImportResult> {
    let sha256 = hex::encode(Sha256::digest(bytes));
    if let Some(expected) = &options.expected_source_sha256 {
        if expected.len() != 64 || !expected.bytes().all(|c| c.is_ascii_hexdigit()) || !expected.eq_ignore_ascii_case(&sha256) {
            return Err(ImportError::at("/source_sha256", "source bytes do not match the expected SHA-256"));
        }
    }
    let source = match format {
        ImportFormat::Ust => ust::parse(bytes, options.ust_encoding)?,
        ImportFormat::Ustx => ustx::parse(bytes)?,
    };
    let title = options.title.clone().unwrap_or_else(|| source.title.clone());
    let selected: BTreeSet<_> = options.tracks.iter().copied().collect();
    if selected.len() != options.tracks.len() { return Err(ImportError::at("/options/tracks", "duplicate track selection")); }
    for index in &selected {
        if !source.tracks.iter().any(|t| t.index == *index) { return Err(ImportError::at("/options/tracks", format!("source track {index} does not exist"))); }
    }
    let mut project = Project::new_empty();
    let mut project_hash = Sha256::new();
    project_hash.update(sha256.as_bytes());
    project_hash.update(serde_json::to_vec(options)?);
    project.project_id = format!("import-{}", hex::encode(project_hash.finalize()));
    project.tracks.clear();
    project.tempo_map = source.tempos.iter().map(|t| Ok(TempoPoint {
        tick: ticks(t.tick, source.ppq, "/tempos")?, micro_bpm: micro_bpm(t.bpm, "/tempos")?, unknown: BTreeMap::new(),
    })).collect::<Result<_>>()?;
    project.time_signatures = source.meters.iter().map(|s| Ok(TimeSignature {
        tick: ticks(s.tick, source.ppq, "/time_signatures")?, numerator: s.numerator, denominator: s.denominator, unknown: BTreeMap::new(),
    })).collect::<Result<_>>()?;
    let tempo = verdant_domain::TempoMap::new(&project.tempo_map, PPQ).map_err(|e| ImportError::at("/tempos", e.to_string()))?;
    let mut report = ImportReport {
        schema: "verdant-score-import/1".into(),
        source: SourceIdentity { format, name: options.source_name.clone(), sha256: sha256.clone(), byte_length: bytes.len(), encoding: source.encoding, ppq: source.ppq },
        title, source_tempos: source.tempos, source_time_signatures: source.meters, lyric_mode: options.lyric_mode,
        tracks: Vec::new(), events: Vec::new(), fields: source.fields, notices: source.notices,
        duration_seconds: 0.0, credits: options.credits.clone(), source_file_copied: false,
    };
    let mut used_readings = BTreeSet::new();
    let mut rest_ids = BTreeSet::new();
    for mut raw in source.tracks {
        if !selected.is_empty() && !selected.contains(&raw.index) { continue; }
        let ti = raw.index;
        raw.notes.sort_by_key(|n| (n.start, n.part, n.index));
        for pair in raw.notes.windows(2) {
            if checked_add(pair[0].start, pair[0].duration, "/notes")? > pair[1].start {
                return Err(ImportError::at(pair[1].source_id(ti), "overlapping source notes cannot become a monophonic Verdant track"));
            }
        }
        let mut track = Track::new(raw.name.clone(), verdant_domain::TRACK_COLORS[ti % verdant_domain::TRACK_COLORS.len()]);
        track.id = TrackId(format!("import-{sha256}-t{ti}"));
        track.gain_millidecibels = controls::track_gain(raw.gain_db, &format!("/tracks/{ti}/volume"))?;
        track.pan_permille = raw.pan;
        track.muted = raw.muted;
        track.solo = raw.solo;
        let mut tr = TrackReport { source_track_index: ti, project_track_id: track.id.clone(), name: raw.name,
            source_event_count: raw.notes.len(), project_note_count: 0, rest_count: 0, continuation_count: 0,
            unresolved_reading_count: 0, source_end_tick: raw.end_tick, midi_range: None };
        let mut previous_end = 0;
        let mut anchor: Option<usize> = None;
        let mut previous_pitch: Option<(i64, i32, f64)> = None;
        for n in &raw.notes {
            let id = n.source_id(ti);
            let start = ticks(n.start, source.ppq, &id)?;
            let duration = ticks(n.duration, source.ppq, &id)?;
            if n.start > previous_end {
                add_gap(&mut report, ti, previous_end, n.start - previous_end, source.ppq)?;
                tr.rest_count += 1;
                anchor = None;
            }
            let rest = rest_lyric(&n.lyric) || options.extra_rest_lyrics.contains(&n.lyric);
            let mut event = EventMapping { source_id: id.clone(), source_track_index: ti, source_part_index: n.part,
                source_note_index: n.index, kind: EventKind::Note, source_start_tick: n.start,
                source_duration_tick: n.duration, source_midi_note: Some(n.midi), source_lyric: n.lyric.clone(),
                source_reading_evidence: n.source_reading, reading: ReadingDisposition::Unresolved,
                project_start_tick: start, project_duration_tick: duration, project_note_id: None, continuation_of_project_note_id: None };
            if rest {
                rest_ids.insert(id.clone());
                event.kind = EventKind::Rest;
                event.reading = ReadingDisposition::Silence;
                anchor = None;
                tr.rest_count += 1;
                report.notices.push(ImportNotice { code: "rest_marker".into(), location: id.clone(), message: "Source silence/breath marker keeps its timing; no breath audio is invented.".into() });
            } else {
                let mut note = Note {
                    id: NoteId(format!("import-{sha256}-{id}")), start_tick: start, duration_tick: duration,
                    midi_note: n.midi, detune_millicents: 0, lyric: n.lyric.clone(),
                    pronunciation: Pronunciation::default(), curves: Default::default(), vibrato: None,
                    portamento_ms: Some(0), unknown: Map::new(),
                };
                controls::apply(&mut note, &n.control, &raw.curves, n.start, n.part, source.ppq, &tempo, previous_pitch.filter(|p| p.0 == n.start), &id, &mut report)?;
                if n.lyric == "+" {
                    let index = anchor.ok_or_else(|| ImportError::at(&id, "+ continuation needs an adjacent sounding note"))?;
                    let preceding = &track.notes[index];
                    if preceding.end_tick() != start {
                        return Err(ImportError::at(&id, "+ continuation must be adjacent to the preceding sounding note"));
                    }
                    note.set_continuation(true);
                    note.unknown.insert("source_note_ids".into(), json!([id]));
                    event.kind = EventKind::Continuation;
                    event.reading = ReadingDisposition::Continuation;
                    event.project_note_id = Some(note.id.clone());
                    event.continuation_of_project_note_id = Some(preceding.id.clone());
                    anchor = Some(track.notes.len());
                    track.notes.push(note);
                    tr.continuation_count += 1;
                } else {
                    note.pronunciation = reading(n, &id, options, &mut used_readings)?;
                    event.reading = if options.readings.by_source_id.contains_key(&id) { ReadingDisposition::InspectedSourceIdOverride }
                        else if options.readings.by_lyric.contains_key(&n.lyric) { ReadingDisposition::InspectedLyricOverride }
                        else if options.lyric_mode == LyricMode::Pinyin { ReadingDisposition::DeclaredSourcePinyin }
                        else { ReadingDisposition::Unresolved };
                    if note.pronunciation.pinyin.is_empty() && note.pronunciation.phonemes.is_empty() {
                        tr.unresolved_reading_count += 1;
                        report.notices.push(ImportNotice { code: "reading_required".into(), location: id.clone(), message: "Source lyric retained; supply an inspected reading before synthesis.".into() });
                    }
                    note.unknown.insert("source_note_ids".into(), json!([id]));
                    event.project_note_id = Some(note.id.clone());
                    anchor = Some(track.notes.len());
                    track.notes.push(note);
                }
                tr.midi_range = Some(tr.midi_range.map_or([n.midi, n.midi], |[lo, hi]| [lo.min(n.midi), hi.max(n.midi)]));
            }
            previous_end = checked_add(n.start, n.duration, &id)?;
            previous_pitch = (!rest).then_some((previous_end, n.midi, n.control.tuning_cents));
            report.events.push(event);
        }
        if raw.end_tick > previous_end {
            add_gap(&mut report, ti, previous_end, raw.end_tick - previous_end, source.ppq)?;
            tr.rest_count += 1;
        }
        tr.project_note_count = track.notes.len();
        report.duration_seconds = report.duration_seconds.max(tempo.seconds_at(ticks(raw.end_tick, source.ppq, "/tracks/end")? as f64));
        track.refresh_index();
        project.tracks.push(track);
        report.tracks.push(tr);
    }
    for id in options.readings.by_source_id.keys() {
        if !used_readings.contains(id) { return Err(ImportError::at("/options/readings/by_source_id", format!("unused source reading ID {id:?}"))); }
    }
    if project.tracks.is_empty() { return Err(ImportError::at("/tracks", "source has no vocal tracks")); }
    for entry in &mut report.fields {
        if let Some((end, _)) = entry.location.match_indices('/').nth(2) {
            if rest_ids.contains(&entry.location[..end]) && entry.disposition == FieldDisposition::Translated {
                entry.disposition = FieldDisposition::RetainedUnsupported;
                entry.target = None;
                entry.detail = "Control belongs to a silence marker; original value retained without execution.".into();
            }
        }
    }
    project.extensions.insert("score_import".into(), serde_json::to_value(&report)?);
    Ok(ImportResult { project, report })
}

fn reading(n: &SourceNote, id: &str, options: &ImportOptions, used: &mut BTreeSet<String>) -> Result<Pronunciation> {
    let supplied = options.readings.by_source_id.get(id).or_else(|| options.readings.by_lyric.get(&n.lyric));
    let mut reading = if let Some(value) = supplied {
        if options.readings.by_source_id.contains_key(id) { used.insert(id.to_string()); }
        if value.pinyin.is_empty() && value.phonemes.is_empty() { return Err(ImportError::at(id, "explicit reading has no pinyin or phonemes")); }
        if !value.phonemes.is_empty() && (value.language.is_none() || value.alphabet.is_none()) { return Err(ImportError::at(id, "explicit phonemes need language and alphabet")); }
        value.clone()
    } else if options.lyric_mode == LyricMode::Pinyin {
        let body = n.lyric.trim_end_matches(['1', '2', '3', '4', '5']);
        let tone_chars = n.lyric.len() - body.len();
        if tone_chars > 1 || body.is_empty() || !body.chars().all(|c| c.is_ascii_alphabetic() || matches!(c, 'ü' | 'Ü' | ':')) {
            return Err(ImportError::at(id, "source lyric is not a single explicit pinyin spelling; supply a reading override"));
        }
        Pronunciation { pinyin: n.lyric.clone(), frontend_profile: "imported-explicit-pinyin/1".into(), ..Default::default() }
    } else { Pronunciation::default() };
    reading.input_text = Some(n.lyric.clone());
    if !reading.pinyin.is_empty() || !reading.phonemes.is_empty() {
        reading.source = PronunciationSource::Imported;
        if reading.frontend_profile.is_empty() { reading.frontend_profile = "imported-inspected-reading/1".into(); }
    }
    Ok(reading)
}
/// Decode the CLI's JSON reading sidecar with recursive duplicate-key rejection.
pub fn parse_readings_json(bytes: &[u8]) -> Result<ReadingOverrides> {
    Ok(serde_json::from_value(strict_yaml::parse_json(bytes)?)?)
}

fn rest_lyric(lyric: &str) -> bool { matches!(lyric, "R" | "r" | "br" | "BR" | "SP" | "AP") }
fn add_gap(report: &mut ImportReport, track: usize, start: i64, duration: i64, ppq: i64) -> Result<()> {
    report.events.push(EventMapping { source_id: format!("t{track}/gap-{start}"), source_track_index: track,
        source_part_index: 0, source_note_index: 0, kind: EventKind::Gap, source_start_tick: start,
        source_duration_tick: duration, source_midi_note: None, source_lyric: String::new(),
        source_reading_evidence: SourceReadingEvidence::NotApplicable, reading: ReadingDisposition::Silence,
        project_start_tick: ticks(start, ppq, "/gap")?, project_duration_tick: ticks(duration, ppq, "/gap")?, project_note_id: None, continuation_of_project_note_id: None });
    Ok(())
}
pub(crate) fn ticks(value: i64, ppq: i64, path: &str) -> Result<i64> {
    let scaled = i128::from(value) * i128::from(PPQ);
    if ppq <= 0 || scaled % i128::from(ppq) != 0 { return Err(ImportError::at(path, format!("{value}/{ppq} has no exact {PPQ}-PPQ tick representation"))); }
    i64::try_from(scaled / i128::from(ppq)).map_err(|_| ImportError::at(path, "tick overflow"))
}
pub(crate) fn checked_add(a: i64, b: i64, path: &str) -> Result<i64> { a.checked_add(b).ok_or_else(|| ImportError::at(path, "tick overflow")) }
pub(crate) fn micro_bpm(bpm: f64, path: &str) -> Result<u64> {
    let scaled = (bpm * 1_000_000.0).round();
    if !bpm.is_finite() || scaled < 1.0 || scaled > 1_000_000_000.0 { return Err(ImportError::at(path, "tempo must be finite, positive, and at most 1000 BPM")); }
    Ok(scaled as u64)
}
pub(crate) fn object<'a>(v: &'a Value, path: &str) -> Result<&'a Map<String, Value>> { v.as_object().ok_or_else(|| ImportError::at(path, "expected mapping")) }
pub(crate) fn array<'a>(v: &'a Value, path: &str) -> Result<&'a [Value]> { v.as_array().map(Vec::as_slice).ok_or_else(|| ImportError::at(path, "expected sequence")) }
pub(crate) fn text<'a>(v: &'a Value, path: &str) -> Result<&'a str> { v.as_str().ok_or_else(|| ImportError::at(path, "expected text")) }
pub(crate) fn integer(v: &Value, minimum: i64, path: &str) -> Result<i64> {
    v.as_i64().filter(|v| *v >= minimum).ok_or_else(|| ImportError::at(path, format!("expected integer >= {minimum}")))
}
pub(crate) fn number(v: &Value, path: &str) -> Result<f64> { v.as_f64().filter(|v| v.is_finite()).ok_or_else(|| ImportError::at(path, "expected finite number")) }
pub(crate) fn required<'a>(map: &'a Map<String, Value>, key: &str, path: &str) -> Result<&'a Value> { map.get(key).ok_or_else(|| ImportError::at(path, format!("missing {key}"))) }
pub(crate) fn field(fields: &mut Vec<FieldReport>, path: impl Into<String>, value: Value, disposition: FieldDisposition, target: Option<&str>, detail: &str) {
    fields.push(FieldReport { location: path.into(), disposition, source_value: value, target: target.map(str::to_string), detail: detail.into() });
}
pub(crate) fn retain_except(map: &Map<String, Value>, supported: &[&str], path: &str, fields: &mut Vec<FieldReport>) {
    for (key, value) in map {
        if !supported.contains(&key.as_str()) {
            field(fields, format!("{path}/{key}"), value.clone(), FieldDisposition::RetainedUnsupported, None, "Source field has no admitted Verdant execution semantics; original value retained.");
        }
    }
}

/// Shared source bar-to-tick admission for UST and USTX meter maps.
pub(crate) fn push_meter_from_bar(meters: &mut Vec<SourceMeter>, previous_bar: &mut i64,
    ppq: i64, bar: i64, numerator: i64, denominator: i64, path: &str) -> Result<()> {
    if ppq <= 0 { return Err(ImportError::at(path, "meter PPQ must be positive")); }
    if bar < 0 || (meters.is_empty() && bar != 0) || (!meters.is_empty() && bar <= *previous_bar) {
        return Err(ImportError::at(path, "meters must begin at bar 0 and increase strictly"));
    }
    let numerator = u32::try_from(numerator).ok().filter(|n| *n > 0)
        .ok_or_else(|| ImportError::at(path, "meter numerator must be a positive u32"))?;
    let denominator = u32::try_from(denominator).ok().filter(|d| d.is_power_of_two())
        .ok_or_else(|| ImportError::at(path, "meter denominator must be a power of two within u32"))?;
    let tick = if let Some(previous) = meters.last() {
        let span = i128::from(bar - *previous_bar).checked_mul(i128::from(ppq))
            .and_then(|n| n.checked_mul(i128::from(previous.numerator) * 4))
            .ok_or_else(|| ImportError::at(path, "meter boundary overflow"))?;
        if span % i128::from(previous.denominator) != 0 {
            return Err(ImportError::at(path, "fractional source tick at meter boundary"));
        }
        let span = i64::try_from(span / i128::from(previous.denominator))
            .map_err(|_| ImportError::at(path, "meter boundary overflow"))?;
        checked_add(previous.tick, span, path)?
    } else { 0 };
    meters.push(SourceMeter { tick, numerator, denominator });
    *previous_bar = bar;
    Ok(())
}
