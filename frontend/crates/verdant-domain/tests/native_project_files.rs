use std::fs;
use std::path::PathBuf;
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectPersistence};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-native-files-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

fn score() -> Project {
    let mut project = Project::new_empty();
    project.tracks[0].notes.push(Note::new(0, 480, 60));
    project.extensions.insert("fixture_credit".into(), serde_json::json!({"author":"synthetic test", "future":17}));
    project
}

#[test]
fn declined_version_admission_does_not_create_or_repair_recovery_files() {
    let temp = Temp::new();
    let path = temp.0.join("newer.verp");
    let journal = verdant_domain::persistence::journal_path(&path);
    let mut project = score();
    project.extensions.insert("verdant.created_with".into(), "999.0.0".into());
    verdant_domain::io::save_project_atomic(&path, &project).unwrap();
    let original = fs::read(&path).unwrap();
    let mut deny = |project: &Project| project.extensions["verdant.created_with"] != "999.0.0";
    assert!(ProjectPersistence::open_if(&path, &mut deny).unwrap().is_none());
    assert!(!journal.exists());
    fs::write(&journal, b"existing recovery bytes").unwrap();
    assert!(ProjectPersistence::open_if(&path, &mut deny).unwrap().is_none());
    assert_eq!(fs::read(&journal).unwrap(), b"existing recovery bytes");
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn native_file_recovers_saves_exports_and_keeps_history() {
    let temp = Temp::new();
    let original = temp.0.join("song.verp");
    let mut store = ProjectPersistence::create(&original, score()).unwrap();
    let id = store.project().tracks[0].notes[0].id.clone();
    let track = store.project().tracks[0].id.clone();
    let saved_bytes = fs::read(&original).unwrap();
    store.commit(store.revision(), CommandBatch::single("move", Command::MoveNotes {
        track, note_ids:vec![id], delta_tick:120, delta_semitones:2,
    })).unwrap();
    store.autosave().unwrap();
    let edited = store.project_for_save();
    drop(store);

    let (mut recovered, report) = ProjectPersistence::open(&original).unwrap();
    assert!(report.recovered_anything());
    assert!(recovered.accept_recovery());
    assert_eq!(recovered.project_for_save(), edited);
    assert_eq!(fs::read(&original).unwrap(), saved_bytes);
    let destination = verdant_domain::io::project_output_path(&temp.0.join("saved")).unwrap();
    assert_eq!(destination.extension().unwrap(), "verp");
    recovered.save_as(&destination).unwrap();
    let active = recovered.project_path().to_path_buf();
    let dirty = recovered.is_dirty();
    recovered.export_project(&temp.0.join("copy")).unwrap();
    assert_eq!(recovered.project_path(), active);
    assert_eq!(recovered.is_dirty(), dirty);
    assert_eq!(ProjectPersistence::read_saved(&temp.0.join("copy.verp")).unwrap(), edited);
    assert!(!verdant_domain::persistence::journal_path(&temp.0.join("copy.verp")).exists());
    drop(recovered);

    let (mut reopened, report) = ProjectPersistence::open(&destination).unwrap();
    assert!(!report.recovered_anything());
    assert_eq!(reopened.project_for_save(), edited);
    assert_eq!(reopened.project().format, "greendam-project");
    assert_eq!(reopened.project().version, "0.1");
    reopened.undo().unwrap();
    assert_eq!(reopened.project().tracks[0].notes[0].start_tick, 0);
    assert_eq!(reopened.project().tracks[0].notes[0].midi_note, 60);
    assert_eq!(reopened.project().extensions["fixture_credit"]["future"], 17);
    assert_eq!(fs::read(&original).unwrap(), saved_bytes);
}

#[test]
fn native_path_still_requires_a_valid_document_header() {
    let temp = Temp::new();
    let path = temp.0.join("ordinary.verp");
    let mut document = score();
    for (format, version) in [("unrelated-json", "0.1"), ("greendam-project", "1.0")] {
        document.format = format.into();
        document.version = version.into();
        let bytes = serde_json::to_vec(&document).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(ProjectPersistence::open(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!verdant_domain::persistence::journal_path(&path).exists());
    }
}

#[test]
fn retired_paths_are_rejected_before_reads_or_sidecar_creation() {
    use verdant_domain::io::{load_project, project_output_path, RETIRED_PROJECT_FORMAT_MESSAGE};
    use verdant_domain::persistence::{journal_path, lock_path};

    let temp = Temp::new();
    for name in ["song.json", "song.verdant.JSON", "song.gdmproj", "song.GDMProj"] {
        let path = temp.0.join(name);
        assert!(!verdant_domain::io::has_project_extension(&path));
        for error in [
            project_output_path(&path).unwrap_err().to_string(),
            load_project(&path).unwrap_err().to_string(),
            ProjectPersistence::open(&path).unwrap_err().to_string(),
            ProjectPersistence::inspect(&path).unwrap_err().to_string(),
            ProjectPersistence::read_saved(&path).unwrap_err().to_string(),
            ProjectPersistence::create(&path, score()).unwrap_err().to_string(),
            ProjectPersistence::export_snapshot(&path, &score()).unwrap_err().to_string(),
        ] {
            assert_eq!(error, RETIRED_PROJECT_FORMAT_MESSAGE);
        }
        assert!(!path.exists());
        assert!(!journal_path(&path).exists());
        assert!(!lock_path(&path).exists());
    }
    assert!(project_output_path(&temp.0.join("audio.wav")).is_err());
}

#[test]
fn native_parser_rejects_duplicate_document_fields() {
    let encoded = serde_json::to_string(&score()).unwrap();
    let duplicate = format!(r#"{{"format":"greendam-project",{}"#, &encoded[1..]);
    assert!(verdant_domain::io::parse_project(&duplicate).unwrap_err().to_string().contains("duplicate field"));
}
