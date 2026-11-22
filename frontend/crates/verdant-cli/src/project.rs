//! Public project operations. All edits use the domain reducer; read-only
//! snapshots replay verified recovery records without opening a writer.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use verdant_domain::{Command, CommandBatch, DomainError, Project, ProjectPersistence, ProjectStore, RecoveryReport, Revision};

use crate::CliError;

const CREATOR_KEY: &str = "verdant.created_with";

pub(crate) struct Params(Map<String, Value>);

impl Params {
    pub(crate) fn new(value: Value) -> Result<Self, CliError> {
        match value {
            Value::Object(map) => Ok(Self(map)),
            _ => Err(CliError::new("INVALID_ARGUMENT", "params must be an object")),
        }
    }

    pub(crate) fn optional<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>, CliError> {
        self.0.remove(key).map(|value| serde_json::from_value(value)
            .map_err(|error| CliError::new("INVALID_ARGUMENT", format!("/{key}: {error}")))).transpose()
    }

    pub(crate) fn required<T: DeserializeOwned>(&mut self, key: &str) -> Result<T, CliError> {
        self.optional(key)?.ok_or_else(|| CliError::new("INVALID_ARGUMENT", format!("/{key} is required")))
    }

    pub(crate) fn finish(self) -> Result<(), CliError> {
        if let Some(key) = self.0.keys().next() {
            return Err(CliError::new("INVALID_ARGUMENT", format!("unknown parameter /{key}")));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecoveryChoice { Saved, Recovered }

impl RecoveryChoice {
    fn parse(value: Option<&str>) -> Result<Option<Self>, CliError> {
        match value {
            None => Ok(None),
            Some("saved") => Ok(Some(Self::Saved)),
            Some("recovered") => Ok(Some(Self::Recovered)),
            Some(_) => Err(CliError::new("INVALID_ARGUMENT", "/recovery must be saved or recovered")),
        }
    }
}

#[derive(Default)]
pub(crate) struct Choices {
    recovery: Option<RecoveryChoice>,
    allowed_creator: Option<std::string::String>,
}

impl Choices {
    pub(crate) fn take(params: &mut Params) -> Result<Self, CliError> {
        let recovery: Option<String> = params.optional("recovery")?;
        Ok(Self { recovery: RecoveryChoice::parse(recovery.as_deref())?, allowed_creator: params.optional("allowed_creator")? })
    }

    fn admit(&self, project: &Project, report: &RecoveryReport, required: bool) -> Result<(), CliError> {
        if required && report.recovered_revision > report.saved_revision && self.recovery.is_none() {
            return Err(CliError::new("DECISION_REQUIRED", format!(
                "saved revision {} has recoverable revision {}; set recovery to saved or recovered",
                report.saved_revision, report.recovered_revision,
            )).with_details(json!({"saved_revision":report.saved_revision,"recovered_revision":report.recovered_revision,
                "required_choice":{"recovery":["saved","recovered"]}})));
        }
        if required {
            require_creator(project, self.allowed_creator.as_deref())?;
        }
        Ok(())
    }
}

fn creator_marker(project: &Project) -> Option<String> {
    project.extensions.get(CREATOR_KEY).map(|value| value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()))
}

fn creator_needs_approval(project: &Project) -> Option<String> {
    let creator = creator_marker(project)?;
    let newer = match (semver::Version::parse(&creator), semver::Version::parse(env!("CARGO_PKG_VERSION"))) {
        (Ok(creator), Ok(current)) => creator.cmp_precedence(&current).is_gt(),
        _ => true,
    };
    newer.then_some(creator)
}

fn require_creator(project: &Project, allowed: Option<&str>) -> Result<(), CliError> {
    if let Some(creator) = creator_needs_approval(project) {
        if allowed != Some(creator.as_str()) {
            return Err(CliError::new("DECISION_REQUIRED", format!(
                "project creator {creator:?} is newer or unrecognized by {}; set allowed_creator to that exact marker to continue",
                env!("CARGO_PKG_VERSION"),
            )).with_details(json!({"creator":creator,"current_version":env!("CARGO_PKG_VERSION"),
                "required_choice":{"allowed_creator":creator}})));
        }
    }
    Ok(())
}

pub(crate) fn stamp_creator(project: &mut Project) {
    project.extensions.insert(CREATOR_KEY.into(), json!(env!("CARGO_PKG_VERSION")));
}

pub(crate) fn persistence_error(error: verdant_domain::PersistenceError) -> CliError {
    use verdant_domain::{JournalError, PersistenceError};
    let code = match &error {
        PersistenceError::Busy(_) => "PROJECT_BUSY",
        PersistenceError::AlreadyExists(_) => "OUTPUT_EXISTS",
        PersistenceError::Domain(DomainError::StaleBase { .. }) => "REVISION_CONFLICT",
        PersistenceError::Domain(_) => "INVALID_PROJECT",
        PersistenceError::Io(verdant_domain::io::IoError::Io { source, .. }) if source.kind() == std::io::ErrorKind::AlreadyExists => "OUTPUT_EXISTS",
        PersistenceError::Io(verdant_domain::io::IoError::Io { .. }) => "IO_ERROR",
        PersistenceError::Io(_) => "INVALID_PROJECT",
        PersistenceError::Journal(JournalError::Io { .. } | JournalError::Storage(_)
            | JournalError::WriteUncertain { .. } | JournalError::Poisoned(_)) => "IO_ERROR",
        PersistenceError::Journal(_) => "INVALID_PROJECT",
    };
    CliError::new(code, error.to_string())
}

fn domain_error(error: DomainError) -> CliError {
    let code = if matches!(&error, DomainError::StaleBase { .. }) { "REVISION_CONFLICT" } else { "INVALID_ARGUMENT" };
    CliError::new(code, error.to_string())
}

pub(crate) fn native_output(path: PathBuf) -> Result<PathBuf, CliError> {
    verdant_domain::io::project_output_path(&path)
        .map_err(|error| CliError::new("INVALID_ARGUMENT", error.to_string()))
}

fn native_input(path: &Path) -> Result<(), CliError> {
    verdant_domain::io::require_project_path(path)
        .map_err(|error| CliError::new("INVALID_ARGUMENT", error.to_string()))
}

pub(crate) fn require_fresh(path: &Path) -> Result<(), CliError> {
    for candidate in [path.to_path_buf(), verdant_domain::persistence::journal_path(path)] {
        match candidate.symlink_metadata() {
            Ok(_) => return Err(CliError::new("OUTPUT_EXISTS", format!("output already exists: {}", candidate.display()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(CliError::new("IO_ERROR", format!("{}: {error}", candidate.display()))),
        }
    }
    Ok(())
}

pub(crate) fn read_store(path: &Path, choices: &Choices, required: bool) -> Result<(ProjectStore, RecoveryReport), CliError> {
    let (store, report) = ProjectPersistence::inspect(path).map_err(persistence_error)?;
    store.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
    choices.admit(store.project(), &report, required)?;
    if choices.recovery == Some(RecoveryChoice::Saved) && report.recovered_revision > report.saved_revision {
        let saved = ProjectPersistence::read_saved(path).map_err(persistence_error)?;
        if required { require_creator(&saved, choices.allowed_creator.as_deref())?; }
        let saved = ProjectStore::new(saved);
        saved.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
        Ok((saved, report))
    } else {
        Ok((store, report))
    }
}

/// Obtain a fully validated owned render/pack snapshot without a writer lease.
/// A pending branch requires an explicit saved/recovered choice; creator
/// approval matches exactly the marker reported by project.inspect.
pub fn read_snapshot(
    path: &Path,
    recovery: std::option::Option<&str>,
    allowed_creator: std::option::Option<&str>,
) -> Result<Project, CliError> {
    let choices = Choices { recovery: RecoveryChoice::parse(recovery)?, allowed_creator: allowed_creator.map(str::to_owned) };
    let (store, _) = read_store(path, &choices, true)?;
    Ok(store.project_for_save())
}

fn recovery_json(report: &RecoveryReport) -> Value {
    json!({
        "saved_revision": report.saved_revision,
        "recovered_revision": report.recovered_revision,
        "decision_required": report.recovered_revision > report.saved_revision,
        "used_autosave": report.used_autosave,
        "replayed_commits": report.replayed_commits,
        "torn_tail_bytes": report.torn_tail_bytes,
        "recovered_at_ms": report.recovered_at_ms,
        "completed_interrupted_save": report.completed_interrupted_save,
        "preserved_unknown_keys": report.load.preserved_unknown_keys,
    })
}

pub(crate) fn store_json(store: &ProjectStore) -> Value {
    let history = store.history();
    json!({
        "project_id": store.project().project_id,
        "revision": store.revision().get(),
        "content_digest": store.content_digest(),
        "dirty": store.is_dirty(),
        "undo_depth": store.undo_depth(),
        "redo_depth": store.redo_depth(),
        "history": {"position": history.position, "entries": history.entries.iter().map(|entry|
            json!({"revision":entry.revision.get(), "label":entry.label})).collect::<Vec<_>>()},
    })
}

pub(crate) fn copy_assets(store: &ProjectStore, source: &Path, output: &Path, dry_run: bool) -> Result<Value, CliError> {
    let report = if dry_run {
        verdant_service::media::preflight_project_assets(store, source, output)
    } else {
        verdant_service::media::copy_project_assets(store, source, output)
    }.map_err(crate::integrations::media_error)?;
    serde_json::to_value(report).map_err(|error| CliError::new("INTERNAL_ERROR", error.to_string()))
}

fn commit_json(commit: &Option<verdant_domain::Commit>) -> Value {
    match commit {
        None => Value::Null,
        Some(commit) => json!({"revision":commit.revision.get(),"kind":match commit.kind {
            verdant_domain::CommitKind::Edit=>"edit",verdant_domain::CommitKind::Undo=>"undo",verdant_domain::CommitKind::Redo=>"redo",
        },"label":commit.label,"changed_notes":commit.changed_notes,"tick_range":commit.tick_range,"meta_changed":commit.meta_changed}),
    }
}

pub(crate) struct EditParams {
    pub(crate) path: PathBuf,
    choices: Choices,
    expected: Revision,
    project_id: Option<std::string::String>,
    pub(crate) dry_run: bool,
}

impl EditParams {
    pub(crate) fn take(params: &mut Params) -> Result<Self, CliError> {
        Ok(Self {
            path: params.required("path")?,
            choices: Choices::take(params)?,
            expected: params.required("expect_revision")?,
            project_id: params.optional("expect_project_id")?,
            dry_run: params.optional("dry_run")?.unwrap_or(false),
        })
    }

    fn check(&self, store: &ProjectStore, report: &RecoveryReport) -> Result<(), CliError> {
        self.choices.admit(store.project(), report, true)?;
        // This guards the complete recoverable branch even when the explicit
        // choice will discard it and return to the saved revision.
        if self.expected.get() != report.recovered_revision {
            return Err(CliError::new("REVISION_CONFLICT", format!(
                "expected revision {} but current recoverable revision is {}", self.expected.get(), report.recovered_revision,
            )));
        }
        if self.project_id.as_ref().is_some_and(|id| id != &store.project().project_id) {
            return Err(CliError::new("REVISION_CONFLICT", "project identity differs from expect_project_id"));
        }
        Ok(())
    }

    pub(crate) fn read(&self) -> Result<(ProjectStore, RecoveryReport), CliError> {
        let (store, report) = read_store(&self.path, &self.choices, true)?;
        self.check(&store, &report)?;
        Ok((store, report))
    }

    pub(crate) fn open(&self) -> Result<(ProjectPersistence, RecoveryReport), CliError> {
        let mut rejected = None;
        let opened = ProjectPersistence::open_admitted(&self.path, &mut |store, report| {
            let decision = self.check(store, report).and_then(|()| store.validate_document()
                .map_err(|error| CliError::new("INVALID_PROJECT", error.to_string())));
            match decision { Ok(()) => true, Err(error) => { rejected = Some(error); false } }
        }).map_err(persistence_error)?;
        let Some((mut document, report)) = opened else {
            return Err(rejected.expect("a rejected admission records its reason"));
        };
        match self.choices.recovery {
            Some(RecoveryChoice::Saved) => { document.discard_recovery().map_err(persistence_error)?; },
            Some(RecoveryChoice::Recovered) => { document.accept_recovery(); },
            None => {},
        }
        Ok((document, report))
    }
}

struct OperationSpec {
    op: &'static str,
    command: &'static str,
    fields: &'static [&'static str],
}

// The public protocol is an explicit finite mapping. Adding a Rust enum
// variant cannot silently add a public operation or change accepted fields.
const OPERATIONS: &[OperationSpec] = &[
    OperationSpec { op:"move_notes", command:"MoveNotes", fields:&["track","note_ids","delta_tick","delta_semitones"] },
    OperationSpec { op:"resize_notes", command:"ResizeNotes", fields:&["track","note_ids","edge","delta_tick"] },
    OperationSpec { op:"duplicate_notes", command:"DuplicateNotes", fields:&["track","source_ids","new_ids","delta_tick","delta_semitones"] },
    OperationSpec { op:"insert_notes", command:"InsertNotes", fields:&["track","notes"] },
    OperationSpec { op:"delete_notes", command:"DeleteNotes", fields:&["track","note_ids"] },
    OperationSpec { op:"set_lyric", command:"SetLyric", fields:&["track","note_id","text"] },
    OperationSpec { op:"set_pronunciation", command:"SetPronunciation", fields:&["track","note_id","pronunciation"] },
    OperationSpec { op:"set_pitch", command:"SetPitch", fields:&["track","note_id","midi_note","detune_millicents"] },
    OperationSpec { op:"set_timing", command:"SetTiming", fields:&["track","note_id","start_tick","duration_tick"] },
    OperationSpec { op:"replace_notes", command:"ReplaceNotes", fields:&["track","notes"] },
    OperationSpec { op:"set_tempo_event", command:"SetTempoEvent", fields:&["tick","micro_bpm"] },
    OperationSpec { op:"set_time_signature_event", command:"SetTimeSignatureEvent", fields:&["tick","numerator","denominator"] },
    OperationSpec { op:"move_track", command:"MoveTrack", fields:&["track","index"] },
    OperationSpec { op:"set_track_data", command:"SetTrackData", fields:&["track","key","value"] },
    OperationSpec { op:"set_project_extension", command:"SetProjectExtension", fields:&["key","value"] },
    OperationSpec { op:"set_route", command:"SetRoute", fields:&["route"] },
    OperationSpec { op:"set_synthesis", command:"SetSynthesis", fields:&["settings"] },
    OperationSpec { op:"set_vocal_tract", command:"SetVocalTract", fields:&["settings"] },
    OperationSpec { op:"set_model_dynamics", command:"SetModelDynamics", fields:&["settings"] },
    OperationSpec { op:"set_phoneme_timing", command:"SetPhonemeTiming", fields:&["settings"] },
    OperationSpec { op:"set_consonant_dynamics", command:"SetConsonantDynamics", fields:&["settings"] },
    OperationSpec { op:"set_output_transients", command:"SetOutputTransients", fields:&["settings"] },
    OperationSpec { op:"add_track", command:"AddTrack", fields:&["index","track"] },
    OperationSpec { op:"remove_track", command:"RemoveTrack", fields:&["track"] },
    OperationSpec { op:"set_track_header", command:"SetTrackHeader", fields:&["track","header"] },
    OperationSpec { op:"set_pitch_curve", command:"SetPitchCurve", fields:&["track","note_id","points"] },
    OperationSpec { op:"set_gain_curve", command:"SetGainCurve", fields:&["track","note_id","points"] },
    OperationSpec { op:"set_expression_curve", command:"SetExpressionCurve", fields:&["track","note_id","parameter","points"] },
    OperationSpec { op:"set_vibrato", command:"SetVibrato", fields:&["track","note_ids","vibrato"] },
    OperationSpec { op:"set_portamento", command:"SetPortamento", fields:&["track","note_ids","portamento_ms"] },
    OperationSpec { op:"set_track_inserts", command:"SetTrackInserts", fields:&["track","inserts"] },
    OperationSpec { op:"set_track_instrument", command:"SetTrackInstrument", fields:&["track","instrument"] },
];

pub fn operation_catalog() -> Value {
    json!({"schema":"verdant-project-operations/1", "discriminator":"op",
        "units":{"tick":"Project ticks; read project.ppq. One quarter note is ppq ticks.",
            "micro_bpm":"BPM multiplied by 1000000; 120 BPM is 120000000. null removes a nonzero tempo event.",
            "gain_millidecibels":"dB multiplied by 1000; -6 dB is -6000.",
            "pan_permille":"-1000 left through 0 center to 1000 right.",
            "detune_millicents":"Cents multiplied by 1000; 100 cents is 100000.",
            "curve_tick":"Note-relative ticks; preserve the source note's duration and other fields."},
        "operations":OPERATIONS.iter().map(|spec|{
            let mut item=json!({"op":spec.op,"fields":spec.fields,
                "identity":"Inspect stable IDs. New track/note IDs must be unique; preserve unknown fields when replacing existing records."});
            if let Some(example)=operation_example(spec.op){item["example"]=example;}
            item
        }).collect::<Vec<_>>()})
}
fn operation_example(op:&str)->Option<Value>{
    let track="INSPECTED_TRACK_ID";let note="INSPECTED_NOTE_ID";
    Some(match op{
        "add_track"=>json!({"op":op,"index":1,"track":{"id":"NEW_UNIQUE_TRACK_ID","name":"Audio","monophonic":false,"color":"teal","notes":[]}}),
        "insert_notes"=>json!({"op":op,"track":track,"notes":[{"id":"NEW_UNIQUE_NOTE_ID","start_tick":0,"duration_tick":960,"midi_note":60,"lyric":"la"}]}),
        "set_tempo_event"=>json!({"op":op,"tick":0,"micro_bpm":120000000}),
        "set_time_signature_event"=>json!({"op":op,"tick":0,"numerator":4,"denominator":4}),
        "move_notes"=>json!({"op":op,"track":track,"note_ids":[note],"delta_tick":120,"delta_semitones":0}),
        "resize_notes"=>json!({"op":op,"track":track,"note_ids":[note],"edge":"end","delta_tick":120}),
        "duplicate_notes"=>json!({"op":op,"track":track,"source_ids":[note],"new_ids":["NEW_UNIQUE_NOTE_ID"],"delta_tick":960,"delta_semitones":0}),
        "delete_notes"=>json!({"op":op,"track":track,"note_ids":[note]}),
        "set_lyric"=>json!({"op":op,"track":track,"note_id":note,"text":"la"}),
        "set_pitch"=>json!({"op":op,"track":track,"note_id":note,"midi_note":60,"detune_millicents":0}),
        "set_timing"=>json!({"op":op,"track":track,"note_id":note,"start_tick":0,"duration_tick":960}),
        "move_track"=>json!({"op":op,"track":track,"index":0}),
        "remove_track"=>json!({"op":op,"track":track}),
        "set_track_header"=>json!({"op":op,"track":track,"header":{"name":"Audio","voice_ref":null,"monophonic":false,"color":"teal","gain_millidecibels":-6000,"pan_permille":0,"muted":false,"solo":false,"render_route":null}}),
        "set_pitch_curve"=>json!({"op":op,"track":track,"note_id":note,"points":[{"id":"NEW_UNIQUE_POINT_ID","tick":0,"millicents":0}]}),
        "set_gain_curve"=>json!({"op":op,"track":track,"note_id":note,"points":[{"id":"NEW_UNIQUE_POINT_ID","tick":0,"millidecibels":0}]}),
        "set_portamento"=>json!({"op":op,"track":track,"note_ids":[note],"portamento_ms":80}),
        "set_route"=>json!({"op":op,"route":"verdant-voice"}),
        _=>return None,
    })
}

fn batch(label: String, operations: Vec<Value>) -> Result<CommandBatch, CliError> {
    let mut commands = Vec::with_capacity(operations.len());
    for (index, value) in operations.into_iter().enumerate() {
        let Value::Object(mut object) = value else {
            return Err(CliError::new("INVALID_ARGUMENT", format!("/operations/{index} must be an object")));
        };
        let op = object.remove("op").and_then(|value| value.as_str().map(str::to_owned))
            .ok_or_else(|| CliError::new("INVALID_ARGUMENT", format!("/operations/{index}/op is required")))?;
        let spec = OPERATIONS.iter().find(|spec| spec.op == op)
            .ok_or_else(|| CliError::new("INVALID_ARGUMENT", format!("/operations/{index}/op: unsupported operation {op}")))?;
        for key in object.keys() {
            if !spec.fields.contains(&key.as_str()) {
                return Err(CliError::new("INVALID_ARGUMENT", format!("unknown field /operations/{index}/{key}")));
            }
        }
        if matches!(op.as_str(), "set_project_extension") && object.get("key").and_then(Value::as_str) == Some(CREATOR_KEY) {
            return Err(CliError::new("INVALID_ARGUMENT", "ordinary edits preserve verdant.created_with; new/import/migrate establish the creator"));
        }
        if op == "set_route" && object.get("route").and_then(Value::as_str) != Some("verdant-voice") {
            return Err(CliError::new("INVALID_ARGUMENT", "set_route only accepts verdant-voice. Select the admitted original on its track."));
        }
        if op == "set_track_data" && object.get("key").and_then(Value::as_str).is_some_and(|key| [
            "id","name","voice_ref","monophonic","notes","color","gain_millidecibels","pan_permille","muted","solo","render_route",
        ].contains(&key)) {
            return Err(CliError::new("INVALID_ARGUMENT", "set_track_data cannot shadow native track fields; use the typed track/note operations"));
        }
        object.insert("name".into(), json!(spec.command));
        let command: Command = serde_json::from_value(Value::Object(object))
            .map_err(|error| CliError::new("INVALID_ARGUMENT", format!("/operations/{index}: {error}")))?;
        commands.push(command);
    }
    Ok(CommandBatch { label, commands })
}

fn check_command_input(command: &Command) -> Result<(), CliError> {
    // IDs remain opaque strings. Empty identifiers and repeated selections
    // are refused rather than relying on commands that silently skip them.
    let check = |ids: &[verdant_domain::NoteId]| -> Result<(), CliError> {
        let mut seen = BTreeSet::new();
        for id in ids {
            if id.as_str().is_empty() || !seen.insert(id) {
                return Err(CliError::new("INVALID_ARGUMENT", "note selections require unique nonempty stable IDs"));
            }
        }
        Ok(())
    };
    match command {
        Command::MoveNotes { note_ids, .. } | Command::ResizeNotes { note_ids, .. }
        | Command::DeleteNotes { note_ids, .. } | Command::SetVibrato { note_ids, .. }
        | Command::SetPortamento { note_ids, .. } => check(note_ids)?,
        Command::DuplicateNotes { source_ids, new_ids, .. } => { check(source_ids)?; check(new_ids)?; },
        _ => {},
    }
    let tick = |value: i64| (0..=Revision::MAX as i64).contains(&value);
    let bounds = |start: i64, duration: i64| tick(start) && duration > 0
        && start.checked_add(duration).is_some_and(tick);
    let note = |note: &verdant_domain::Note| bounds(note.start_tick, note.duration_tick)
        && (0..=127).contains(&note.midi_note) && !note.id.as_str().is_empty()
        && note.curves.pitch_millicent_points.iter().all(|point| tick(point.tick) && point.millicents.unsigned_abs() <= 2_400_000)
        && note.curves.gain_millidecibel_points.iter().all(|point| tick(point.tick) && (-60_000..=12_000).contains(&point.millidecibels))
        && note.curves.expression_points.values().flatten().all(|point| tick(point.tick) && point.value <= 127);
    let valid = match command {
        Command::MoveNotes { delta_tick, .. } | Command::ResizeNotes { delta_tick, .. }
        | Command::DuplicateNotes { delta_tick, .. } => delta_tick.unsigned_abs() <= Revision::MAX,
        Command::InsertNotes { notes, .. } | Command::ReplaceNotes { notes, .. } => notes.iter().all(note),
        Command::AddTrack { track, .. } => !track.id.as_str().is_empty() && track.notes.iter().all(note),
        Command::SetTiming { start_tick, duration_tick, .. } => bounds(*start_tick, *duration_tick),
        Command::SetPitch { midi_note, .. } => (0..=127).contains(midi_note),
        Command::SetTempoEvent { tick: at, .. } | Command::SetTimeSignatureEvent { tick: at, .. } => tick(*at),
        Command::SetPitchCurve { points, .. } => points.iter().all(|point| tick(point.tick) && point.millicents.unsigned_abs() <= 2_400_000),
        Command::SetGainCurve { points, .. } => points.iter().all(|point| tick(point.tick) && (-60_000..=12_000).contains(&point.millidecibels)),
        Command::SetExpressionCurve { points, .. } => points.iter().all(|point| tick(point.tick) && point.value <= 127),
        _ => true,
    };
    if !valid {
        return Err(CliError::new("INVALID_ARGUMENT", "command identifiers, note bounds, pitch or control values exceed the exact native range"));
    }
    Ok(())
}

fn inspect(mut params: Params, history_only: bool) -> Result<Value, CliError> {
    let path: PathBuf = params.required("path")?;
    let choices = Choices::take(&mut params)?;
    params.finish()?;
    let (store, report) = read_store(&path, &choices, false)?;
    let mut result = store_json(&store);
    result["path"] = json!(path);
    result["recovery"] = recovery_json(&report);
    result["creator_approval_required"] = json!(creator_needs_approval(store.project()));
    if !history_only { result["project"] = json!(store.project_for_save()); }
    Ok(result)
}

fn validate(mut params: Params) -> Result<Value, CliError> {
    let path: PathBuf = params.required("path")?;
    let choices = Choices::take(&mut params)?;
    params.finish()?;
    let (store, report) = read_store(&path, &choices, false)?;
    let overlaps = store.project().tracks.iter().filter(|track| track.monophonic).flat_map(|track|
        track.overlaps().into_iter().map(|(left, right)| json!({"track_id":track.id,"note_ids":[left,right]}))).collect::<Vec<_>>();
    Ok(json!({"valid":true,"project_id":store.project().project_id,"revision":store.revision().get(),
        "content_digest":store.content_digest(),"recovery":recovery_json(&report),
        "creator_approval_required":creator_needs_approval(store.project()),"monophonic_overlaps":overlaps,
        "renderability_checked":false}))
}

fn new_project(mut params: Params) -> Result<Value, CliError> {
    let output = native_output(params.required("output")?)?;
    let template: Option<Project> = params.optional("project")?;
    let ppq: Option<i64> = params.optional("ppq")?;
    let allowed_creator: Option<String> = params.optional("allowed_creator")?;
    let operations = params.optional("operations")?.unwrap_or_default();
    let label = params.optional("label")?.unwrap_or_else(|| "Create project".into());
    let dry_run: bool = params.optional("dry_run")?.unwrap_or(false);
    params.finish()?;
    require_fresh(&output)?;
    if template.is_some() && ppq.is_some() { return Err(CliError::new("INVALID_ARGUMENT", "ppq belongs in project when a template is provided")); }
    let mut project = template.unwrap_or_else(Project::new_empty);
    require_creator(&project, allowed_creator.as_deref())?;
    project.project_id = uuid::Uuid::new_v4().to_string();
    project.revision = 0;
    if let Some(ppq) = ppq { project.ppq = ppq; }
    stamp_creator(&mut project);
    let mut store = ProjectStore::new(project);
    store.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
    let batch = batch(label, operations)?;
    batch.commands.iter().try_for_each(check_command_input)?;
    let commit = store.apply(store.revision(), batch).map_err(domain_error)?;
    store.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
    if dry_run {
        return Ok(json!({"output":output,"dry_run":true,"state":store_json(&store),"project":store.project_for_save(),"commit":commit_json(&commit)}));
    }
    let document = ProjectPersistence::create_from_store(&output, store).map_err(persistence_error)?;
    Ok(json!({"output":output,"dry_run":false,"state":store_json(document.store()),"project":document.project_for_save(),"commit":commit_json(&commit)}))
}

fn apply(mut params: Params) -> Result<Value, CliError> {
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    let label = params.optional("label")?.unwrap_or_else(|| "CLI edit".into());
    let operations = params.required("operations")?;
    params.finish()?;
    let batch = batch(label, operations)?;
    execute_batch(edit, batch)
}

/// Shared durable gate for semantic operations compiled by media/effect and
/// other public CLI adapters. `params` contains only the common edit fields.
pub(crate) fn apply_domain_batch(params: Value, batch: CommandBatch) -> Result<Value, CliError> {
    let mut params = Params::new(params)?;
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    params.finish()?;
    execute_batch(edit, batch)
}

fn execute_batch(edit: EditParams, batch: CommandBatch) -> Result<Value, CliError> {
    batch.commands.iter().try_for_each(check_command_input)?;
    if edit.dry_run {
        let (mut preview, report) = edit.read()?;
        let commit = preview.apply(preview.revision(), batch).map_err(domain_error)?;
        preview.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
        return Ok(json!({"path":edit.path,"dry_run":true,"state":store_json(&preview),"commit":commit_json(&commit),"recovery":recovery_json(&report)}));
    }
    let (mut document, report) = edit.open()?;
    let commit = document.commit(document.revision(), batch).map_err(persistence_error)?;
    if commit.is_some() || report.recovered_revision > report.saved_revision {
        document.save().map_err(persistence_error)?;
    }
    Ok(json!({"path":edit.path,"dry_run":false,"state":store_json(document.store()),"commit":commit_json(&commit),"recovery":recovery_json(&report)}))
}

fn navigate(mut params: Params, undo: bool) -> Result<Value, CliError> {
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    params.finish()?;
    if edit.dry_run {
        let (mut preview, report) = edit.read()?;
        let commit = if undo { preview.undo() } else { preview.redo() }.map_err(domain_error)?;
        preview.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
        return Ok(json!({"path":edit.path,"dry_run":true,"state":store_json(&preview),"commit":commit_json(&commit),"recovery":recovery_json(&report)}));
    }
    let (mut document, report) = edit.open()?;
    let commit = if undo { document.undo() } else { document.redo() }.map_err(persistence_error)?;
    if commit.is_some() || report.recovered_revision > report.saved_revision { document.save().map_err(persistence_error)?; }
    Ok(json!({"path":edit.path,"dry_run":false,"state":store_json(document.store()),"commit":commit_json(&commit),"recovery":recovery_json(&report)}))
}

fn save_as(mut params: Params) -> Result<Value, CliError> {
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    let output = native_output(params.required("output")?)?;
    params.finish()?;
    require_fresh(&output)?;
    let (store, report) = edit.read()?;
    let assets = copy_assets(&store, &edit.path, &output, edit.dry_run)?;
    if edit.dry_run {
        return Ok(json!({"output":output,"dry_run":true,"state":store_json(&store),"recovery":recovery_json(&report),"assets":assets}));
    }
    let document = ProjectPersistence::create_from_store(&output, store).map_err(persistence_error)?;
    Ok(json!({"output":output,"dry_run":false,"source_unchanged":true,"state":store_json(document.store()),"recovery":recovery_json(&report),"assets":assets}))
}

fn recover(mut params: Params) -> Result<Value, CliError> {
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    params.finish()?;
    if edit.choices.recovery.is_none() {
        return Err(CliError::new("INVALID_ARGUMENT", "project.recover requires recovery=saved or recovered"));
    }
    let (store, report) = edit.read()?;
    if edit.dry_run {
        return Ok(json!({"path":edit.path,"dry_run":true,"state":store_json(&store),"recovery":recovery_json(&report)}));
    }
    let (mut document, report) = edit.open()?;
    if edit.choices.recovery == Some(RecoveryChoice::Recovered) && report.recovered_revision > report.saved_revision {
        document.save().map_err(persistence_error)?;
    }
    Ok(json!({"path":edit.path,"dry_run":false,"state":store_json(document.store()),"recovery":recovery_json(&report)}))
}

fn migrate(mut params: Params) -> Result<Value, CliError> {
    let edit = EditParams::take(&mut params)?;
    native_input(&edit.path)?;
    let output = native_output(params.required("output")?)?;
    let loss_policy: String = params.optional("loss_policy")?.unwrap_or_else(|| "strict".into());
    params.finish()?;
    require_fresh(&output)?;
    let (store, recovery) = edit.read()?;
    let has_losses = store.undo_depth() + store.redo_depth() > 0;
    let report = json!({"schema":"verdant-native-migration/1","source_project_id":store.project().project_id,
        "source_revision":store.revision().get(),"has_losses":has_losses,"issues":if has_losses {
            vec![json!({"path":"/history","disposition":"unsupported","detail":"Migration establishes a fresh project identity; use project.save_as to preserve the existing history."})]
        } else { Vec::<Value>::new() }});
    crate::exchange::admit_losses(&loss_policy, has_losses, &report, edit.dry_run)?;
    let mut project = store.project_for_save();
    project.project_id = uuid::Uuid::new_v4().to_string();
    project.revision = 0;
    stamp_creator(&mut project);
    let store = ProjectStore::new(project);
    let assets = copy_assets(&store, &edit.path, &output, edit.dry_run)?;
    if edit.dry_run { return Ok(json!({"output":output,"dry_run":true,"project":store.project(),"report":report,"recovery":recovery_json(&recovery),"assets":assets})); }
    let document = ProjectPersistence::create_from_store(&output, store).map_err(persistence_error)?;
    Ok(json!({"output":output,"dry_run":false,"state":store_json(document.store()),"report":report,"recovery":recovery_json(&recovery),"assets":assets}))
}

pub fn dispatch(op: &str, params: Value) -> Option<Result<Value, CliError>> {
    let operation: fn(Params) -> Result<Value, CliError> = match op {
        "project.new" => new_project,
        "project.inspect" => |params| inspect(params, false),
        "project.validate" => validate,
        "project.save_as" => save_as,
        "project.recover" => recover,
        "project.migrate" => migrate,
        "project.apply" => apply,
        "history.list" => |params| inspect(params, true),
        "history.undo" => |params| navigate(params, true),
        "history.redo" => |params| navigate(params, false),
        _ => return None,
    };
    Some(Params::new(params).and_then(operation))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(op: &str, params: Value) -> Value { dispatch(op, params).unwrap().unwrap() }

    #[test]
    fn project_operations_reject_retired_paths_before_reading_or_writing() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["old.json", "old.verdant.JSON", "old.gdmproj"] {
            let path = temp.path().join(name);
            let output = temp.path().join("out.verp");
            for (operation, params) in [
                ("project.inspect", json!({"path":path})),
                ("project.validate", json!({"path":path})),
                ("project.migrate", json!({"path":path,"output":output,"expect_revision":0})),
                ("project.save_as", json!({"path":path,"output":output,"expect_revision":0})),
                ("project.new", json!({"output":path})),
            ] {
                let error = dispatch(operation, params).unwrap().unwrap_err();
                assert_eq!(error.message, verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE);
            }
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn public_batches_save_as_and_history_keep_stable_ids_and_revisions() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.verp");
        let created = call("project.new", json!({"output":source}));
        let track = created["project"]["tracks"][0]["id"].as_str().unwrap();
        let edited = call("project.apply", json!({"path":source,"expect_revision":0,"label":"one intent","operations":[
            {"op":"insert_notes","track":track,"notes":[{"id":"authored-note","start_tick":0,"duration_tick":960,"midi_note":60}]},
            {"op":"set_lyric","track":track,"note_id":"authored-note","text":"绿"},
        ]}));
        assert_eq!(edited["state"]["revision"], 1);
        assert_eq!(edited["state"]["undo_depth"], 1);
        let undone = call("history.undo", json!({"path":source,"expect_revision":1}));
        assert_eq!(undone["state"]["revision"], 2);
        assert_eq!(call("project.inspect", json!({"path":source}))["project"]["tracks"][0]["notes"], json!([]));
        let redone = call("history.redo", json!({"path":source,"expect_revision":2}));
        assert_eq!(redone["state"]["revision"], 3);
        let copy = temp.path().join("copy.verp");
        call("project.save_as", json!({"path":source,"output":copy,"expect_revision":3}));
        let history = call("history.list", json!({"path":copy}));
        assert_eq!(history["history"]["entries"], json!([{"revision":1,"label":"one intent"}]));
        call("history.undo", json!({"path":copy,"expect_revision":3}));
        let original = call("project.inspect", json!({"path":source}));
        assert_eq!(original["revision"], 3);
        assert_eq!(original["project"]["tracks"][0]["notes"][0]["id"], "authored-note");
        assert_eq!(original["project"]["tracks"][0]["notes"][0]["lyric"], "绿");
    }

    #[test]
    fn dry_run_and_refused_decisions_leave_saved_document_and_journal_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("read-only.verp");
        let mut project = Project::new_empty();
        project.extensions.insert(CREATOR_KEY.into(), json!("99.0.0"));
        verdant_domain::io::save_project_atomic(&path, &project).unwrap();
        let before = std::fs::read(&path).unwrap();
        call("project.inspect", json!({"path":path}));
        call("project.validate", json!({"path":path}));
        let error = dispatch("project.apply", json!({"path":path,"expect_revision":0,"label":"tempo","dry_run":true,
            "operations":[{"op":"set_tempo_event","tick":0,"micro_bpm":90_000_000}]})).unwrap().unwrap_err();
        assert_eq!(error.code, "DECISION_REQUIRED");
        let preview = call("project.apply", json!({"path":path,"expect_revision":0,"label":"tempo","dry_run":true,
            "allowed_creator":"99.0.0","operations":[{"op":"set_tempo_event","tick":0,"micro_bpm":90_000_000}]}));
        assert_eq!(preview["state"]["revision"], 1);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!verdant_domain::persistence::journal_path(&path).exists());
        assert!(!verdant_domain::persistence::lock_path(&path).exists());
    }
}
