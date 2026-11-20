//! User decisions about a newer autosave must never rewrite the manual save.

use std::fs;
use std::path::PathBuf;

use verdant_domain::persistence::journal_path;
use verdant_domain::{Command, CommandBatch, JournalError, Note, PersistenceError, Project, ProjectPersistence};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("verdant-autosave-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn edit(document: &mut ProjectPersistence, delta_tick: i64) {
    let track = &document.project().tracks[0];
    let batch = CommandBatch::single("移动音符", Command::MoveNotes {
        track: track.id.clone(), note_ids: vec![track.notes[0].id.clone()], delta_tick, delta_semitones: 0,
    });
    document.commit(document.revision(), batch).unwrap().unwrap();
}

fn create(path: &std::path::Path) -> ProjectPersistence {
    let mut project = Project::new_empty();
    project.tracks[0].notes.push(Note::new(960, 480, 60));
    // Extended project data must survive the full recovery path too.
    project.extensions.insert("recovery-fixture".into(), serde_json::json!({"lyrics":"热异常", "values":[1,2,3]}));
    ProjectPersistence::create(path, project).unwrap()
}

#[test]
fn newer_autosave_offers_accept_or_discard_without_changing_manual_save() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("song.verp");
    let mut document = create(&path);
    let saved_bytes = fs::read(&path).unwrap();
    let saved_project = document.project_for_save();
    let saved_revision = document.revision();
    assert!(document.pending_recovery().is_none());
    assert!(!document.autosave().unwrap());
    edit(&mut document, 240);
    edit(&mut document, 120);
    document.undo().unwrap();
    let autosaved_project = document.project_for_save();
    let autosaved_revision = document.revision();
    let undo_depth = document.undo_depth();
    let redo_depth = document.redo_depth();
    assert!(document.autosave().unwrap());
    assert!(!document.autosave().unwrap());
    assert!(document.is_dirty());
    assert_eq!(fs::read(&path).unwrap(), saved_bytes);
    drop(document); // A crash leaves only the durable files.

    let (mut recovered, report) = ProjectPersistence::open(&path).unwrap();
    assert!(report.used_autosave);
    assert_eq!(report.saved_revision, saved_revision.get());
    assert_eq!(report.recovered_revision, autosaved_revision.get());
    assert!(report.recovered_revision > report.saved_revision);
    assert!(recovered.pending_recovery().is_some());
    assert!(recovered.accept_recovery());
    assert!(recovered.pending_recovery().is_none());
    assert_eq!(recovered.project_for_save(), autosaved_project);
    assert_eq!(recovered.revision(), autosaved_revision);
    assert_eq!(recovered.undo_depth(), undo_depth);
    assert_eq!(recovered.redo_depth(), redo_depth);
    assert!(recovered.is_dirty());
    assert_eq!(fs::read(&path).unwrap(), saved_bytes);
    drop(recovered);

    // Accepting a recovery is not a manual save. Another crash still
    // offers the unsaved work, and now the user explicitly discards it.
    let (mut recovered, _) = ProjectPersistence::open(&path).unwrap();
    assert!(recovered.pending_recovery().is_some());
    assert!(recovered.discard_recovery().unwrap());
    assert_eq!(recovered.project_for_save(), saved_project);
    assert_eq!(recovered.revision(), saved_revision);
    assert!(!recovered.is_dirty());
    assert!(recovered.pending_recovery().is_none());
    assert_eq!(fs::read(&path).unwrap(), saved_bytes);
    drop(recovered);
    let (mut reopened, report) = ProjectPersistence::open(&path).unwrap();
    assert!(!report.recovered_anything());
    assert!(reopened.pending_recovery().is_none());
    assert_eq!(reopened.project_for_save(), saved_project);
    assert_eq!(fs::read(&path).unwrap(), saved_bytes);
    // The new saved-file branch remains usable after the old recovery
    // branch has been explicitly abandoned.
    edit(&mut reopened, 480);
    assert_eq!(reopened.revision(), saved_revision.next().unwrap());
}

#[test]
fn torn_autosave_checkpoint_is_rejected_without_touching_saved_file_or_evidence() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("song.verp");
    let mut document = create(&path);
    let saved_bytes = fs::read(&path).unwrap();
    edit(&mut document, 240);
    document.autosave().unwrap();
    drop(document);
    let sidecar = journal_path(&path);
    let full = fs::read(&sidecar).unwrap();
    // A truncated checkpoint is not a valid recoverable transaction prefix.
    // Reject it, including the missing-newline case, without repairing it.
    for cut in [0, 1, full.len() / 2, full.len() - 1] {
        fs::write(&sidecar, &full[..cut]).unwrap();
        assert!(matches!(ProjectPersistence::open(&path), Err(PersistenceError::Journal(JournalError::Corrupt { .. }))));
        assert_eq!(fs::read(&path).unwrap(), saved_bytes);
        assert_eq!(fs::read(&sidecar).unwrap(), full[..cut]);
        assert_eq!(ProjectPersistence::read_saved(&path).unwrap().tracks[0].notes[0].start_tick, 960);
    }
}

#[test]
fn recovery_offer_also_covers_commits_before_the_periodic_checkpoint() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("song.verp");
    let mut document = create(&path);
    edit(&mut document, 240);
    let expected = document.project_for_save();
    drop(document);
    let (mut recovered, report) = ProjectPersistence::open(&path).unwrap();
    assert!(!report.used_autosave);
    assert_eq!(report.replayed_commits, 1);
    assert!(recovered.pending_recovery().is_some());
    recovered.accept_recovery();
    assert_eq!(recovered.project_for_save(), expected);
}

#[test]
fn delayed_discard_cannot_erase_edits_or_an_externally_changed_manual_save() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("song.verp");
    let mut document = create(&path);
    edit(&mut document, 240);
    document.autosave().unwrap();
    drop(document);
    let (mut recovered, _) = ProjectPersistence::open(&path).unwrap();
    edit(&mut recovered, 120);
    let expected = recovered.project_for_save();
    let saved_bytes = fs::read(&path).unwrap();
    let sidecar = journal_path(&path);
    let journal_bytes = fs::read(&sidecar).unwrap();
    assert!(recovered.discard_recovery().is_err());
    assert_eq!(recovered.project_for_save(), expected);
    assert_eq!(fs::read(&path).unwrap(), saved_bytes);
    assert_eq!(fs::read(&sidecar).unwrap(), journal_bytes);
    drop(recovered);

    let (mut recovered, _) = ProjectPersistence::open(&path).unwrap();
    let mut changed = ProjectPersistence::read_saved(&path).unwrap();
    changed.extensions.insert("external-edit".into(), serde_json::json!(true));
    verdant_domain::io::save_project_atomic(&path, &changed).unwrap();
    let external_bytes = fs::read(&path).unwrap();
    assert!(recovered.discard_recovery().is_err());
    assert_eq!(fs::read(&path).unwrap(), external_bytes);
    assert_eq!(fs::read(&sidecar).unwrap(), journal_bytes);
}
