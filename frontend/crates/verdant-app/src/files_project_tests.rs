use super::*;
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectPersistence};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-project-entry-{}", Project::new_empty().project_id));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn score() -> Project {
    let mut project = Project::new_empty();
    project.ppq = 480;
    project.voice_refs.clear();
    project.tracks[0].voice_ref = None;
    let mut note = Note::new(240, 480, 64);
    note.lyric = "a".into();
    project.tracks[0].notes.push(note);
    project
}

fn opened(result: Result<StartupOpen, String>) -> StartupDocument {
    match result.unwrap() { StartupOpen::Opened(document) => document, StartupOpen::Newer { .. } => panic!("unexpected newer creator") }
}

#[test]
fn save_as_suggestions_use_verp() {
    assert_eq!(project_suggested_name(None), "project.verp");
    for name in ["song.verp", "song.VERP"] {
        assert_eq!(project_suggested_name(Some(Path::new(name))), "song.verp");
    }
    assert_eq!(native_save_path(Path::new("song")).unwrap(), Path::new("song.verp"));
    assert_eq!(native_save_path(Path::new("song.VERP")).unwrap(), Path::new("song.VERP"));
    assert!(native_save_path(Path::new("song.ust")).is_err());
}

#[test]
fn startup_reopens_native_untitled_recovery_and_creates_fresh_verp() {
    let temp = Temp::new();
    let working = temp.0.join("untitled/project.verp");
    let mut store = ProjectPersistence::create(&working, score()).unwrap();
    let track = &store.project().tracks[0];
    let batch = CommandBatch::single("move", Command::MoveNotes {
        track:track.id.clone(), note_ids:vec![track.notes[0].id.clone()], delta_tick:120, delta_semitones:0,
    });
    store.commit(store.revision(), batch).unwrap();
    store.autosave().unwrap();
    let original = std::fs::read(&working).unwrap();
    let expected = store.project_for_save();
    drop(store);
    let opened = opened(open_startup_document(None, &temp.0, &[], None));
    assert_eq!(opened.persistence.project_path(), working);
    assert_eq!(opened.persistence.project_for_save(), expected);
    assert!(opened.recovery.recovered_anything());
    assert!(opened.persistence.pending_recovery().is_some());
    assert!(!opened.new_demo);
    assert!(opened.project_path.is_none());
    assert_eq!(std::fs::read(&working).unwrap(), original);
    let fresh = Temp::new();
    let opened = match open_startup_document(None, &fresh.0, &[], None).unwrap() { StartupOpen::Opened(document) => document, _ => panic!("fresh document") };
    assert_eq!(opened.persistence.project_path(), fresh.0.join("untitled/project.verp"));
    assert!(opened.project_path.is_none());
    assert!(opened.new_demo);
    assert_eq!(creator_version(opened.persistence.project()).as_deref(), Some(CREATOR_VERSION));
}

#[test]
fn native_startup_checks_content_and_preserves_builtin_and_external_destinations() {
    let temp = Temp::new();
    let ordinary = temp.0.join("ordinary.json");
    std::fs::write(&ordinary, br#"{"version":1,"project":{"tracks":[]}}"#).unwrap();
    assert!(open_startup_document(Some(&ordinary), &temp.0, &[], None).is_err());
    assert!(!verdant_domain::persistence::journal_path(&ordinary).exists());
    assert!(!temp.0.join("untitled").exists());
    let documents = temp.0.join("documents");
    for extension in ["verp", "VERP"] {
        let project = crate::demos::project(crate::demos::HEXIE_EXAMPLE_ID).unwrap();
        let path = documents.join(format!("example-{}-{}.{extension}", crate::demos::HEXIE_EXAMPLE_ID, project.project_id));
        let store = ProjectPersistence::create(&path, project).unwrap();
        drop(store);
        let mut document = opened(open_startup_document(Some(&path), &temp.0, &[], None));
        assert!(!document.new_demo);
        if extension == "verp" { assert!(document.project_path.is_none()); }
        let external = temp.0.join("external").join(path.file_name().unwrap());
        document.persistence.save_as(&external).unwrap();
        let expected = document.persistence.project_for_save();
        drop(document);
        let reopened = opened(open_startup_document(Some(&external), &temp.0, &[], None));
        assert_eq!(reopened.project_path.as_deref(), Some(external.as_path()));
        assert_eq!(reopened.persistence.project_for_save(), expected);
    }
}

#[test]
fn ufdata_menu_and_cli_entry_save_reopen_and_export_native_verp() {
    let temp = Temp::new();
    let input = temp.0.join("source.ufdata");
    let source = verdant_exchange::export(verdant_exchange::Format::Ufdata, &score(), &Default::default()).unwrap();
    std::fs::write(&input, &source.bytes).unwrap();
    let format = crate::menu::file_format(&input).unwrap().unwrap();
    let (mut imported, report, working) = import_project_document(&input, &temp.0.join("documents")).unwrap();
    let (second, _, second_working) = import_project_document(&input, &temp.0.join("documents")).unwrap();
    assert_ne!(working, second_working);
    assert_eq!(second.project().tracks[0].notes[0].midi_note, 64);
    assert_eq!(working.extension().unwrap(), "verp");
    assert_eq!(report["format"], "ufdata");
    let saved = temp.0.join("edited.verp");
    imported.save_as(&saved).unwrap();
    let expected = imported.project_for_save();
    drop(imported);
    let (reopened, _) = ProjectPersistence::open(&saved).unwrap();
    assert_eq!(reopened.project_for_save(), expected);
    reopened.export_project(&temp.0.join("native-copy.verp")).unwrap();
    assert_eq!(ProjectPersistence::read_saved(&temp.0.join("native-copy.verp")).unwrap(), expected);
    let exported = verdant_exchange::export(format, reopened.project(), &Default::default()).unwrap();
    let again = verdant_exchange::import(format, &exported.bytes, &Default::default()).unwrap();
    let note = &again.project.tracks[0].notes[0];
    assert_eq!((note.start_tick, note.duration_tick, note.midi_note), (240, 480, 64));
    assert!(open_startup_document(Some(&input), &temp.0.join("cli-state"), &[], None).is_err());
    assert_eq!(creator_version(reopened.project()).as_deref(), Some(CREATOR_VERSION));
    assert_eq!(std::fs::read(input).unwrap(), source.bytes);
}

#[test]
fn creator_gate_precedes_journal_creation_and_preserves_creator_on_save() {
    let temp = Temp::new();
    for creator in [None, Some("0.0.1"), Some(CREATOR_VERSION), Some("99999.0.0"), Some("future-version")] {
        let path = temp.0.join(format!("{}.VERP", Project::new_empty().project_id));
        let mut project = score();
        if let Some(creator) = creator { project.extensions.insert(CREATOR_EXTENSION.into(), serde_json::json!(creator)); }
        verdant_domain::io::save_project_atomic(&path, &project).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let needs_confirmation = creator.is_some_and(|value| creator_is_newer(value, CREATOR_VERSION));
        match open_native_project(&path, None).unwrap() {
            NativeOpen::Newer(value) => {
                assert!(needs_confirmation);
                assert_eq!(Some(value.as_str()), creator);
                assert!(!verdant_domain::persistence::journal_path(&path).exists());
                assert!(matches!(open_startup_document(Some(&path), &temp.0, &[], None).unwrap(), StartupOpen::Newer { .. }));
            }
            NativeOpen::Opened(_, _) => assert!(!needs_confirmation),
        }
        let NativeOpen::Opened(mut document, _) = open_native_project(&path, creator).unwrap() else { panic!("explicit creator approval"); };
        let destination = temp.0.join(format!("copy-{}.verp", project.project_id));
        document.save_as(&destination).unwrap();
        assert_eq!(creator_version(document.project()).as_deref(), creator);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    assert!(!creator_is_newer("0.1.0-alpha.1", "0.1.0"));
    assert!(!creator_is_newer("0.1.0+build.2", "0.1.0+build.1"));
    assert!(creator_is_newer("0.10.0", "0.9.0"));
}

#[test]
fn retired_projects_are_rejected_by_open_and_import_without_touching_source() {
    use verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE;
    use verdant_domain::persistence::{journal_path, lock_path};

    let temp = Temp::new();
    let documents = temp.0.join("documents");
    for extension in ["json", "verdant.JSON", "gdmproj", "GDMProj"] {
        let path = temp.0.join(format!("old.{extension}"));
        let bytes = serde_json::to_vec(&score()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let journal = journal_path(&path);
        std::fs::write(&journal, b"existing recovery bytes").unwrap();
        assert_eq!(open_native_project(&path, None).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(open_startup_document(Some(&path), &temp.0, &[], None).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(import_project_document(&path, &documents).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(native_save_path(&path).unwrap_err(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(&journal).unwrap(), b"existing recovery bytes");
        assert!(!lock_path(&path).exists());
        assert!(!documents.exists());
    }
}

#[test]
fn recovery_prompt_uses_relative_time_and_file_name() {
    let now = 1_800_000_000_000;
    assert_eq!(relative_time_text(now - 20_000, now), "刚刚");
    assert_eq!(relative_time_text(now + 5_000, now), "刚刚");
    assert!(relative_time_text(now - 5 * 60_000, now).starts_with("5 分钟前（"));
    assert!(relative_time_text(now - 3 * 3_600_000, now).starts_with("3 小时前（"));
    assert!(relative_time_text(now - 30 * 3_600_000, now).starts_with("昨天 "));
    assert_eq!(relative_time_text(now - 3 * 86_400_000, now).len(), "2027-01-12 08:00".len());
    let report = verdant_domain::RecoveryReport { saved_revision: 4, recovered_revision: 9, recovered_at_ms: now - 120_000, ..Default::default() };
    let detail = recovery_prompt_detail(Path::new("/very/long/folder/that/would/overflow/歌.verp"), &report, now);
    assert!(detail.starts_with("「歌.verp」"), "{detail}");
    assert!(detail.contains("2 分钟前") && detail.contains("5 步编辑"), "{detail}");
    assert!(!detail.contains("/very/long") && !detail.contains("Unix"), "{detail}");
}
