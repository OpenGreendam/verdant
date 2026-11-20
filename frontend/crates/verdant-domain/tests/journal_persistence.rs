//! Native project transaction durability and recovery.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use verdant_domain::digest::sha256_hex;
use verdant_domain::io::{load_project, save_project_atomic};
use verdant_domain::persistence::journal_path;
use verdant_domain::{Command, CommandBatch, CommitKind, JournalError, Note, NoteId, PersistenceError, Project, ProjectPersistence, ProjectStore, Revision, Track, TrackId};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-domain-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn document(&self) -> PathBuf {
        self.0.join("song.verp")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rev(value: u64) -> Revision {
    Revision::new(value).unwrap()
}

fn project() -> (Project, TrackId) {
    let mut project = Project::new_empty();
    let track = project.tracks[0].id.clone();
    for (id, tick, pitch) in [("n1", 960, 60), ("n2", 1440, 64)] {
        let mut note = Note::new(tick, 480, pitch);
        note.id = id.into();
        project.tracks[0].notes.push(note);
    }
    project.tracks[0].refresh_index();
    (project, track)
}

fn fixture() -> (Sandbox, ProjectPersistence, TrackId) {
    let sandbox = Sandbox::new();
    let (project, track) = project();
    let persistent = ProjectPersistence::create(&sandbox.document(), project).unwrap();
    (sandbox, persistent, track)
}

fn move_notes(track: &TrackId, delta_tick: i64) -> CommandBatch {
    CommandBatch::single("移动", Command::MoveNotes {
        track: track.clone(), note_ids: vec!["n1".into(), "n2".into()], delta_tick, delta_semitones: 0,
    })
}

fn first_tick(project: &Project) -> i64 {
    project.tracks[0].note(&"n1".into()).unwrap().start_tick
}

fn frames(path: &Path) -> Vec<Value> {
    fs::read_to_string(path).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

fn write_frames(path: &Path, frames: &[Value]) {
    let mut bytes = Vec::new();
    for frame in frames {
        serde_json::to_writer(&mut bytes, frame).unwrap();
        bytes.push(b'\n');
    }
    fs::write(path, bytes).unwrap();
}

fn modify_payload(frame: &mut Value, mutate: impl FnOnce(&mut Value), update_checksum: bool) {
    let mut payload: Value = serde_json::from_str(frame["payload"].as_str().unwrap()).unwrap();
    mutate(&mut payload);
    let payload = serde_json::to_string(&payload).unwrap();
    if update_checksum {
        frame["sha256"] = sha256_hex(payload.as_bytes()).into();
    }
    frame["payload"] = payload.into();
}

#[test]
fn committed_edits_undo_and_redo_recover_with_monotonic_revisions() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap().unwrap();
    assert_eq!(persistent.undo().unwrap().unwrap().revision, rev(2));
    assert_eq!(persistent.redo().unwrap().unwrap().revision, rev(3));
    persistent.undo().unwrap().unwrap();
    drop(persistent);

    let (mut recovered, report) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(report.saved_revision, 0);
    assert_eq!(report.recovered_revision, 4);
    assert_eq!(report.replayed_commits, 4);
    assert!(report.recovered_anything());
    assert!(report.recovered_at_ms > 0);
    assert_eq!(recovered.revision(), rev(4));
    assert_eq!(first_tick(recovered.project()), 960);
    assert_eq!(recovered.undo_depth(), 0);
    assert_eq!(recovered.redo_depth(), 1);
    assert!(!recovered.is_dirty()); // undo restored the exact saved content
    assert_eq!(recovered.redo().unwrap().unwrap().kind, CommitKind::Redo);
    assert_eq!(recovered.revision(), rev(5));
    assert_eq!(first_tick(recovered.project()), 1200);
    assert!(recovered.is_dirty());
}

#[test]
fn incomplete_final_frame_recovers_prior_complete_commit_at_multiple_boundaries() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    persistent.commit(rev(1), move_notes(&track, 240)).unwrap();
    drop(persistent);
    let path = journal_path(&sandbox.document());
    let original = fs::read(&path).unwrap();
    let last_start = original[..original.len() - 1].iter().rposition(|byte| *byte == b'\n').unwrap() + 1;
    for cut in [last_start, last_start + 1, (last_start + original.len()) / 2, original.len() - 1] {
        fs::write(&path, &original[..cut]).unwrap();
        let (mut recovered, report) = ProjectPersistence::open(&sandbox.document()).unwrap();
        assert_eq!(recovered.revision(), rev(1));
        assert_eq!(first_tick(recovered.project()), 1200);
        assert_eq!(report.torn_tail_bytes, (cut - last_start) as u64);
        assert_eq!(fs::metadata(&path).unwrap().len(), last_start as u64);
        // A repaired prefix remains writable; the new complete record recovers.
        recovered.commit(rev(1), move_notes(&track, 480)).unwrap();
        drop(recovered);
        let (reopened, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
        assert_eq!(reopened.revision(), rev(2));
        assert_eq!(first_tick(reopened.project()), 1680);
        drop(reopened);
    }
}

#[test]
fn corruption_in_any_complete_record_is_reported_and_not_repaired() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    persistent.commit(rev(1), move_notes(&track, 240)).unwrap();
    drop(persistent);
    let path = journal_path(&sandbox.document());
    let original = frames(&path);
    for index in [0, 1, 2] { // checkpoint, earlier transaction, complete final transaction
        let mut damaged = original.clone();
        damaged[index]["sha256"] = Value::from("0".repeat(64));
        write_frames(&path, &damaged);
        let before = fs::read(&path).unwrap();
        assert!(matches!(ProjectPersistence::open(&sandbox.document()), Err(PersistenceError::Journal(JournalError::Corrupt { .. }))));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn foreign_checkpoint_or_transaction_is_never_applied() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    drop(persistent);
    let path = journal_path(&sandbox.document());
    let original = frames(&path);
    for index in [0, 1] {
        let mut damaged = original.clone();
        modify_payload(&mut damaged[index], |payload| {
            if index == 0 {
                payload["store"]["project"]["project_id"] = "another-project".into();
                payload["saved"]["project_id"] = "another-project".into();
            } else {
                payload["project_id"] = "another-project".into();
            }
        }, true);
        write_frames(&path, &damaged);
        let before = fs::read(&path).unwrap();
        assert!(matches!(ProjectPersistence::open(&sandbox.document()), Err(PersistenceError::Journal(JournalError::IdentityMismatch { .. }))));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn same_project_id_with_divergent_saved_content_is_reported() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    let mut divergent = load_project(&sandbox.document()).unwrap().0;
    divergent.unknown.insert("future_title".into(), "a different branch at the same revision".into());
    save_project_atomic(&sandbox.document(), &divergent).unwrap();
    drop(persistent);
    let path = journal_path(&sandbox.document());
    let before = fs::read(&path).unwrap();
    assert!(matches!(ProjectPersistence::open(&sandbox.document()), Err(PersistenceError::Journal(JournalError::StaleIdentity(_)))));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(load_project(&sandbox.document()).unwrap().0, divergent);
}

#[test]
fn revision_gaps_reordering_and_patch_divergence_are_rejected() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    drop(persistent);
    let path = journal_path(&sandbox.document());
    let original = frames(&path);
    for mode in 0..4 {
        let mut damaged = original.clone();
        modify_payload(&mut damaged[1], |payload| match mode {
            0 => payload["commit"]["revision"] = 9.into(),
            1 => payload["base_revision"] = 5.into(),
            2 => payload["previous"] = "wrong-preceding-checksum".into(),
            _ => payload["entry"]["notes"][0]["after"]["lyric"] = "different patch".into(),
        }, true);
        write_frames(&path, &damaged);
        let before = fs::read(&path).unwrap();
        assert!(matches!(ProjectPersistence::open(&sandbox.document()), Err(PersistenceError::Journal(JournalError::Corrupt { .. }))));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn noops_rejected_batches_and_reads_do_not_journal() {
    let (sandbox, mut persistent, track) = fixture();
    let path = journal_path(&sandbox.document());
    let bytes = fs::read(&path).unwrap();
    assert!(persistent.commit(rev(0), move_notes(&track, 0)).unwrap().is_none());
    assert!(persistent.commit(rev(7), move_notes(&track, 240)).is_err());
    assert!(persistent.commit(rev(0), CommandBatch::single("bad", Command::SetTiming {
        track: track.clone(), note_id: "n1".into(), start_tick: -1, duration_tick: 480,
    })).is_err());
    // Two metadata changes which cancel inside one batch are also a no-op.
    assert!(persistent.commit(rev(0), CommandBatch { label: "cancelled tempo change".into(), commands: vec![
        Command::SetTempo { micro_bpm: 90_000_000 }, Command::SetTempo { micro_bpm: 120_000_000 },
    ] }).unwrap().is_none());
    assert!(persistent.undo().unwrap().is_none());
    assert!(persistent.redo().unwrap().is_none());
    for _ in 0..100 {
        let _snapshot = persistent.snapshot();
        let _dirty = persistent.is_dirty();
    }
    assert_eq!(persistent.revision(), rev(0));
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn journal_writer_failure_does_not_publish_candidate_or_history() {
    let (sandbox, mut persistent, track) = fixture();
    let snapshot = persistent.snapshot();
    let path = journal_path(&sandbox.document());
    // Another writer changes the file: this writer must refuse to append.
    fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"interference").unwrap();
    assert!(persistent.commit(rev(0), move_notes(&track, 240)).is_err());
    assert!(Arc::ptr_eq(&snapshot, &persistent.snapshot()));
    assert_eq!(persistent.revision(), rev(0));
    assert_eq!(persistent.undo_depth(), 0);
    assert_eq!(persistent.log().len(), 0);
}

#[test]
fn autosave_compaction_preserves_saved_document_dirty_state_and_redo() {
    let (sandbox, mut persistent, track) = fixture();
    let document_before = fs::read(sandbox.document()).unwrap();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    persistent.commit(rev(1), move_notes(&track, 240)).unwrap();
    persistent.undo().unwrap();
    assert!(persistent.is_dirty());
    assert!(persistent.autosave().unwrap());
    let journal_before = fs::read(journal_path(&sandbox.document())).unwrap();
    assert!(!persistent.autosave().unwrap());
    assert_eq!(fs::read(journal_path(&sandbox.document())).unwrap(), journal_before);
    assert_eq!(fs::read(sandbox.document()).unwrap(), document_before);
    assert!(persistent.is_dirty());
    drop(persistent);

    let (mut recovered, report) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert!(report.used_autosave);
    assert_eq!(report.replayed_commits, 0);
    assert_eq!(recovered.revision(), rev(3));
    assert_eq!(recovered.undo_depth(), 1);
    assert_eq!(recovered.redo_depth(), 1);
    assert!(recovered.is_dirty());
    recovered.redo().unwrap();
    assert_eq!(first_tick(recovered.project()), 1440);
}

#[test]
fn save_and_post_save_edits_preserve_the_last_state_and_all_undo() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    persistent.save().unwrap();
    assert!(!persistent.is_dirty());
    persistent.commit(rev(1), move_notes(&track, 240)).unwrap();
    drop(persistent);
    let (mut recovered, report) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(report.replayed_commits, 1);
    assert_eq!(recovered.revision(), rev(2));
    assert_eq!(recovered.undo_depth(), 2);
    assert_eq!(first_tick(recovered.project()), 1440);
    recovered.undo().unwrap();
    assert!(!recovered.is_dirty()); // back to last saved content despite new revision
    recovered.undo().unwrap();
    assert_eq!(first_tick(recovered.project()), 960);
    assert!(recovered.is_dirty());
}

#[test]
fn crash_between_durable_document_save_and_checkpoint_loses_nothing() {
    let (sandbox, mut persistent, track) = fixture();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    // The exact intermediate phase of save(): document durable, old recovery
    // checkpoint/log still present. No fake corrupt record or zero-fill.
    save_project_atomic(&sandbox.document(), &persistent.project_for_save()).unwrap();
    drop(persistent);
    let (mut recovered, report) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert!(report.completed_interrupted_save);
    assert_eq!(recovered.revision(), rev(1));
    assert!(!recovered.is_dirty());
    assert_eq!(recovered.undo_depth(), 1);
    recovered.commit(rev(1), move_notes(&track, 240)).unwrap();
    drop(recovered);
    let (again, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(again.revision(), rev(2));
    assert_eq!(first_tick(again.project()), 1440);
    assert!(again.is_dirty());
}

#[test]
fn save_as_rebinds_only_after_writing_document_and_recovery() {
    let (sandbox, mut persistent, track) = fixture();
    let original_document = fs::read(sandbox.document()).unwrap();
    persistent.commit(rev(0), move_notes(&track, 240)).unwrap();
    let invalid_parent = sandbox.0.join("a-file-not-a-directory");
    fs::write(&invalid_parent, b"keep this file").unwrap();
    let snapshot = persistent.snapshot();
    assert!(persistent.save_as(&invalid_parent.join("score.verp")).is_err());
    assert_eq!(persistent.project_path(), sandbox.document());
    assert!(Arc::ptr_eq(&snapshot, &persistent.snapshot()));
    assert!(persistent.is_dirty());
    let target = sandbox.0.join("another.verp");
    persistent.save_as(&target).unwrap();
    assert_eq!(persistent.project_path(), target);
    assert!(!persistent.is_dirty());
    assert_eq!(fs::read(sandbox.document()).unwrap(), original_document);
    drop(persistent);
    let (mut recovered, _) = ProjectPersistence::open(&target).unwrap();
    assert_eq!(recovered.undo_depth(), 1);
    recovered.undo().unwrap();
    assert_eq!(first_tick(recovered.project()), 960);
}

#[test]
fn creating_a_project_cannot_erase_existing_or_orphaned_recovery_data() {
    let (sandbox, persistent, _) = fixture();
    let path = journal_path(&sandbox.document());
    let before = fs::read(&path).unwrap();
    assert!(matches!(ProjectPersistence::create(&sandbox.document(), Project::new_empty()), Err(PersistenceError::AlreadyExists(_))));
    drop(persistent);
    fs::remove_file(sandbox.document()).unwrap();
    assert!(matches!(ProjectPersistence::create(&sandbox.document(), Project::new_empty()), Err(PersistenceError::AlreadyExists(_))));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(!sandbox.document().exists());
}

#[test]
fn one_note_edit_journal_size_is_independent_of_100k_note_project() {
    let sandbox = Sandbox::new();
    let mut document = Project::new_empty();
    let track = document.tracks[0].id.clone();
    for index in 0..100_000 {
        let mut note = Note::new(index * 480, 480, 60);
        note.id = NoteId(format!("note-{index}"));
        document.tracks[0].notes.push(note);
    }
    let mut persistent = ProjectPersistence::create(&sandbox.document(), document).unwrap();
    let path = journal_path(&sandbox.document());
    let before = fs::metadata(&path).unwrap().len();
    persistent.commit(rev(0), CommandBatch::single("lyric", Command::SetLyric {
        track, note_id: "note-50000".into(), text: "啊".into(),
    })).unwrap();
    let delta = fs::metadata(&path).unwrap().len() - before;
    assert!(delta < 16 * 1024, "one note transaction wrote {delta} bytes instead of a small delta");
}

#[test]
fn inserts_and_track_existence_in_one_batch_undo_and_recover_exactly() {
    let (sandbox, mut persistent, track) = fixture();
    let before = persistent.project_for_save();
    let added = Track::new("extra", "blue");
    let added_id = added.id.clone();
    let mut note = Note::new(0, 480, 60);
    note.id = "new-note".into();
    let inserts = vec![json!({"format":"clap","id":"eq","name":"EQ","version":"1","state_b64":"AA==","bypass":false,"future":7})];
    persistent.commit(rev(0), CommandBatch { label: "track + note + insert".into(), commands: vec![
        Command::AddTrack { index: 1, track: added },
        Command::InsertNotes { track: added_id.clone(), notes: vec![note] },
        Command::SetTrackInserts { track: added_id.clone(), inserts: inserts.clone() },
    ] }).unwrap();
    persistent.undo().unwrap();
    assert_eq!(serde_json::to_value(persistent.project()).unwrap(), serde_json::to_value(&before).unwrap());
    persistent.redo().unwrap();
    assert_eq!(persistent.project().track(&added_id).unwrap().unknown["inserts"], Value::Array(inserts));
    // Edit a note then remove that track in one batch: undo restores the
    // pre-batch track, not an intermediate edited/deleted-note state.
    persistent.commit(rev(3), CommandBatch { label: "edit then remove".into(), commands: vec![
        Command::SetLyric { track: track.clone(), note_id: "n1".into(), text: "temporary".into() },
        Command::RemoveTrack { track: track.clone() },
    ] }).unwrap();
    persistent.undo().unwrap();
    drop(persistent);
    let (recovered, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(recovered.revision(), rev(5));
    assert_eq!(recovered.project().track(&track).unwrap().note(&"n1".into()).unwrap().lyric, "");
    assert_eq!(recovered.project().track(&added_id).unwrap().notes.len(), 1);
}

#[test]
fn track_inserts_replace_remove_and_undo_preserve_unknown_plugin_fields() {
    let (project, track) = project();
    let mut store = ProjectStore::new(project);
    let inserts = vec![json!({"format":"clap","id":"eq","name":"EQ","version":"1","state_b64":"AA==","bypass":false,"future":7})];
    store.apply(rev(0), CommandBatch::single("add", Command::SetTrackInserts { track: track.clone(), inserts: inserts.clone() })).unwrap();
    assert_eq!(store.undo_depth(), 1);
    store.undo().unwrap();
    assert!(!store.project().tracks[0].unknown.contains_key("inserts"));
    store.redo().unwrap();
    assert_eq!(store.project().tracks[0].unknown["inserts"], Value::Array(inserts.clone()));
    store.apply(rev(3), CommandBatch::single("remove", Command::SetTrackInserts { track, inserts: vec![] })).unwrap();
    store.undo().unwrap();
    assert_eq!(store.project().tracks[0].unknown["inserts"], Value::Array(inserts));
}

#[test]
fn interval_index_rebuilds_after_timing_pitch_undo_redo_and_recovery() {
    let sandbox = Sandbox::new();
    let (mut project, track) = project();
    let mut long = Note::new(2000, 50_000, 60);
    long.id = "long".into();
    project.tracks[0].notes.push(long);
    let mut persistent = ProjectPersistence::create(&sandbox.document(), project).unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_range(49_000, 49_500).count(), 1);
    assert_eq!(persistent.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 60, 60).count(), 1);
    persistent.commit(rev(0), CommandBatch::single("shorten", Command::SetTiming {
        track: track.clone(), note_id: "long".into(), start_tick: 2000, duration_tick: 480,
    })).unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_range(49_000, 49_500).count(), 0);
    persistent.undo().unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_range(49_000, 49_500).count(), 1);
    persistent.redo().unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_range(49_000, 49_500).count(), 0);
    persistent.undo().unwrap();
    persistent.commit(rev(4), CommandBatch::single("repitch", Command::SetPitch {
        track, note_id: "long".into(), midi_note: 64, detune_millicents: 0,
    })).unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 60, 60).count(), 0);
    assert_eq!(persistent.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 64, 64).count(), 1);
    persistent.undo().unwrap();
    assert_eq!(persistent.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 60, 60).count(), 1);
    persistent.redo().unwrap();
    persistent.autosave().unwrap();
    drop(persistent);
    let (mut recovered, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(recovered.project().tracks[0].notes_in_range(49_000, 49_500).count(), 1);
    assert_eq!(recovered.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 60, 60).count(), 0);
    assert_eq!(recovered.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 64, 64).count(), 1);
    assert_eq!(recovered.project().tracks[0].notes_in_range(i64::MIN, -1).count(), 0);
    recovered.undo().unwrap();
    assert_eq!(recovered.project().tracks[0].notes_in_pitch_range(49_000, 49_500, 60, 60).count(), 1);
}

#[test]
fn project_revision_bounds_and_journal_suffix_are_preserved() {
    assert!(serde_json::from_value::<Revision>(json!(Revision::MAX + 1)).is_err());
    let (mut document, _) = project();
    document.revision = Revision::MAX + 1;
    assert!(verdant_domain::io::parse_project(&serde_json::to_string(&document).unwrap()).is_err());
    assert_eq!(journal_path(Path::new("song.verp")), Path::new("song.verp.journal"));
}

#[test]
fn native_export_preserves_unknown_fields() {
    let sandbox = Sandbox::new();
    let (mut document, _) = project();
    document.unknown.insert("future_root".into(), json!({"x":1}));
    document.extensions.insert("future_extension".into(), json!([1,2,3]));
    document.tracks[0].unknown.insert("future_track".into(), true.into());
    document.tracks[0].notes[0].unknown.insert("future_note".into(), json!({"x":"value"}));
    document.tracks[0].notes[0].pronunciation.unknown.insert("future_pronunciation".into(), 7.into());
    document.tracks[0].notes[0].curves.unknown.insert("future_curve".into(), json!([]));
    document.render_preferences.unknown.insert("future_route_option".into(), "x".into());
    document.tempo_map[0].unknown.insert("future_tempo".into(), true.into());
    document.time_signatures[0].unknown.insert("future_signature".into(), 9.into());
    document.voice_refs.get_mut("voice-sn").unwrap().unknown.insert("future_voice".into(), "metadata".into());
    let mut vibrato = verdant_domain::Vibrato::DEFAULT;
    vibrato.unknown.insert("future_vibrato".into(), json!({"shape":"triangle"}));
    document.tracks[0].notes[0].vibrato = Some(vibrato);
    document.tracks[0].notes[0].curves.pitch_millicent_points.push(serde_json::from_value(json!({
        "id":"future-pitch", "tick":0, "millicents":0, "future_point":"pitch metadata",
    })).unwrap());
    document.tracks[0].notes[0].curves.gain_millidecibel_points.push(serde_json::from_value(json!({
        "id":"future-gain", "tick":0, "millidecibels":0, "future_point":"gain metadata",
    })).unwrap());
    let track = document.tracks[0].id.clone();
    let mut expected = serde_json::to_value(&document).unwrap();
    expected["revision"] = 1.into();
    expected["tracks"][0]["notes"][0]["lyric"] = "updated lyric".into();
    let mut persistent = ProjectPersistence::create(&sandbox.document(), document).unwrap();
    persistent.commit(rev(0), CommandBatch::single("lyric", Command::SetLyric {
        track, note_id:"n1".into(), text:"updated lyric".into(),
    })).unwrap();
    persistent.autosave().unwrap();
    drop(persistent);
    let (persistent, recovery) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert!(recovery.used_autosave);
    let copy = sandbox.0.join("copy.verp");
    persistent.export_project(&copy).unwrap();
    let (imported, report) = load_project(&copy).unwrap();
    assert_eq!(serde_json::to_value(imported).unwrap(), expected);
    assert!(report.preserved_unknown_keys.iter().any(|key| key == "/future_root"));
    assert!(report.preserved_unknown_keys.iter().any(|key| key == "/tempo_map/0/future_tempo"));
    assert!(report.preserved_unknown_keys.iter().any(|key| key.ends_with("/vibrato/future_vibrato")));
}
