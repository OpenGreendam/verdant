use verdant_domain::{Command, CommandBatch, PhonemeTimingSettings, Project, ProjectPersistence, ProjectStore, VocalTractSettings};

#[test]
fn phoneme_timing_preserves_old_projects_b_and_notes_through_journal_undo_redo_save() {
    let root = std::env::temp_dir().join(format!("verdant-phoneme-timing-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("song.verp");
    let mut initial = Project::new_empty();
    initial.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(1.125));
    assert!(serde_json::to_value(&initial).unwrap()["render_preferences"].get("phoneme_timing").is_none());
    let before = initial.clone();
    let mut document = ProjectPersistence::create(&path, initial).unwrap();
    let mut settings = PhonemeTimingSettings::vowel_anchor_v1();
    settings.unknown.insert("future_note".into(), serde_json::json!("retain"));
    document.commit(document.revision(), CommandBatch::single("元音对拍", Command::SetPhonemeTiming { settings: Some(settings.clone()) })).unwrap().unwrap();
    assert_eq!(document.project().tracks, before.tracks);
    assert_eq!(document.project().render_preferences.vocal_tract, before.render_preferences.vocal_tract);
    drop(document);
    let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits, 1);
    assert_eq!(document.project().render_preferences.phoneme_timing, Some(settings.clone()));
    document.undo().unwrap();
    assert_eq!(document.project().render_preferences, before.render_preferences);
    document.redo().unwrap();
    document.save().unwrap();
    drop(document);
    let (document, recovery) = ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits, 0);
    assert_eq!(document.project().render_preferences.phoneme_timing, Some(settings));
    let text = serde_json::to_string(document.project()).unwrap();
    let (_, report) = verdant_domain::io::parse_project(&text).unwrap();
    assert!(report.preserved_unknown_keys.iter().any(|key| key == "/render_preferences/phoneme_timing/future_note"));
    drop(document);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_and_cancelled_timing_batches_are_atomic() {
    let mut store = ProjectStore::new(Project::new_empty());
    let original = store.project().render_preferences.clone();
    let revision = store.revision();
    let mut invalid = PhonemeTimingSettings::vowel_anchor_v1();
    invalid.method = "unrecognized".into();
    let invalid_batch = CommandBatch { label: "invalid".into(), commands: vec![
        Command::SetVocalTract { settings: Some(VocalTractSettings::bilinear_v1(1.125)) },
        Command::SetPhonemeTiming { settings: Some(invalid) },
    ] };
    assert!(store.apply(revision, invalid_batch).is_err());
    assert_eq!(store.project().render_preferences, original);
    assert_eq!(store.revision(), revision);
    let cancelled = CommandBatch { label: "cancelled".into(), commands: vec![
        Command::SetPhonemeTiming { settings: Some(PhonemeTimingSettings::vowel_anchor_v1()) },
        Command::SetPhonemeTiming { settings: None },
    ] };
    assert!(store.apply(revision, cancelled).unwrap().is_none());
    assert_eq!(store.project().render_preferences, original);
    assert_eq!(store.undo_depth(), 0);
}
