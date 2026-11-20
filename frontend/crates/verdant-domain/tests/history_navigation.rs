//! History navigation is document editing: batches remain single undo steps,
//! redo branches disappear on edits, and every jump step survives recovery.

use std::path::PathBuf;

use serde_json::json;
use verdant_domain::{Command, CommandBatch, DomainError, Note, Project, ProjectPersistence, ProjectStore};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-history-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn document(&self) -> PathBuf { self.0.join("song.verp") }
}

impl Drop for Sandbox {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn content(store: &ProjectStore) -> Project {
    let mut project = store.project_for_save();
    // Revisions identify commits, not historical content: undo/redo deliberately
    // keep increasing them. Every other document field must match exactly.
    project.revision = 0;
    project
}

#[test]
fn history_jump_restores_exact_documents_and_labels_and_truncates_redo_after_branching() {
    let sandbox = Sandbox::new();
    let mut project = Project::new_empty();
    let track = project.tracks[0].id.clone();
    let mut note = Note::new(960, 480, 60);
    note.id = "history-note".into();
    project.tracks[0].notes.push(note.clone());
    let mut persistent = ProjectPersistence::create(&sandbox.document(), project).unwrap();
    let mut states = vec![content(persistent.store())];

    let batches = [
        CommandBatch {
            label: "移动并修改音高".into(),
            commands: vec![
                Command::MoveNotes { track: track.clone(), note_ids: vec![note.id.clone()], delta_tick: 240, delta_semitones: 2 },
                Command::SetProjectExtension { key: "history-test".into(), value: Some(json!({"values": [1, 2], "text": "保留"})) },
            ],
        },
        CommandBatch::single("歌词", Command::SetLyric { track: track.clone(), note_id: note.id.clone(), text: "绿".into() }),
        CommandBatch::single("速度", Command::SetTempo { micro_bpm: 91_500_000 }),
    ];
    let mut revisions = Vec::new();
    for batch in batches {
        revisions.push(persistent.commit(persistent.revision(), batch).unwrap().unwrap().revision);
        states.push(content(persistent.store()));
    }
    let history = persistent.history();
    assert_eq!(history.position, 3);
    assert_eq!(history.entries.iter().map(|entry| entry.label).collect::<Vec<_>>(), ["移动并修改音高", "歌词", "速度"]);
    assert_eq!(history.entries.iter().map(|entry| entry.revision).collect::<Vec<_>>(), revisions);

    let commits = persistent.jump_to_history(persistent.revision(), 1).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(content(persistent.store()), states[1]);
    assert_eq!((persistent.undo_depth(), persistent.redo_depth()), (1, 2));
    assert_eq!(persistent.history().position, 1);
    assert!(persistent.jump_to_history(persistent.revision(), 1).unwrap().is_empty());
    persistent.autosave().unwrap();
    drop(persistent);

    // Compaction and process restart retain the same branch and its redo tail.
    let (mut persistent, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(content(persistent.store()), states[1]);
    assert_eq!(persistent.history().position, 1);
    assert_eq!(persistent.history().entries.iter().map(|entry| entry.label).collect::<Vec<_>>(), ["移动并修改音高", "歌词", "速度"]);
    for position in [3, 0, 2, 1] {
        let before_revision = persistent.revision().get();
        let distance = persistent.undo_depth().abs_diff(position);
        let commits = persistent.jump_to_history(persistent.revision(), position).unwrap();
        assert_eq!(commits.len(), distance);
        assert_eq!(persistent.revision().get(), before_revision + distance as u64);
        assert_eq!(content(persistent.store()), states[position]);
    }
    let stale = persistent.revision();
    persistent.commit(persistent.revision(), CommandBatch::single("新分支", Command::SetLyric {
        track, note_id: note.id, text: "翠绿".into(),
    })).unwrap().unwrap();
    let branch = content(persistent.store());
    assert_eq!(persistent.redo_depth(), 0);
    assert_eq!(persistent.history().position, 2);
    assert_eq!(persistent.history().entries.iter().map(|entry| entry.label).collect::<Vec<_>>(), ["移动并修改音高", "新分支"]);
    assert!(persistent.redo().unwrap().is_none());
    assert!(matches!(persistent.jump_to_history(stale, 0), Err(verdant_domain::PersistenceError::Domain(DomainError::StaleBase { .. }))));
    assert!(persistent.jump_to_history(persistent.revision(), 3).is_err());
    assert_eq!(content(persistent.store()), branch);
    drop(persistent);

    // The incremental journal replays jumps and branch truncation too.
    let (mut persistent, _) = ProjectPersistence::open(&sandbox.document()).unwrap();
    assert_eq!(content(persistent.store()), branch);
    assert_eq!(persistent.history().entries.iter().map(|entry| entry.label).collect::<Vec<_>>(), ["移动并修改音高", "新分支"]);
    assert_eq!(persistent.redo_depth(), 0);
    persistent.jump_to_history(persistent.revision(), 0).unwrap();
    assert_eq!(content(persistent.store()), states[0]);
    persistent.jump_to_history(persistent.revision(), 2).unwrap();
    assert_eq!(content(persistent.store()), branch);
}

#[test]
fn in_memory_history_keeps_duplicate_labels_as_separate_transaction_steps() {
    let mut store = ProjectStore::new(Project::new_empty());
    let initial = content(&store);
    for tempo in [90_000_000, 100_000_000] {
        store.apply(store.revision(), CommandBatch::single("速度", Command::SetTempo { micro_bpm: tempo })).unwrap();
    }
    // A no-op is not a committed history entry.
    store.apply(store.revision(), CommandBatch::single("不应出现", Command::SetTempo { micro_bpm: 100_000_000 })).unwrap();
    assert_eq!(store.history().entries.len(), 2);
    assert_ne!(store.history().entries[0].revision, store.history().entries[1].revision);
    let final_content = content(&store);
    assert_eq!(store.jump_to_history(store.revision(), 0).unwrap().len(), 2);
    assert_eq!(content(&store), initial);
    assert_eq!(store.jump_to_history(store.revision(), 2).unwrap().len(), 2);
    assert_eq!(content(&store), final_content);
    assert_eq!(store.undo_depth(), 2);
}
