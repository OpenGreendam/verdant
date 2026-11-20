use verdant_domain::io::parse_project;
use verdant_domain::{Command, CommandBatch, Project, ProjectPersistence, ProjectStore, SynthesisSettings};

#[test]
fn legacy_document_keeps_absent_settings_and_unknown_profile_keys_round_trip() {
    let legacy = Project::new_empty();
    let value = serde_json::to_value(&legacy).unwrap();
    assert!(value["render_preferences"].get("synthesis").is_none());
    let (loaded, _) = parse_project(&value.to_string()).unwrap();
    assert!(loaded.render_preferences.synthesis.is_none());
    let mut selected = value;
    selected["render_preferences"]["synthesis"] = serde_json::json!({
        "vowel_balance_db":18.0, "future_profile_note":"keep",
        "wideband":{"method":"matched-balanced", "presence_db":24.0, "future_anchor":42}
    });
    let (loaded, report) = parse_project(&selected.to_string()).unwrap();
    assert_eq!(serde_json::to_value(loaded).unwrap(), selected);
    assert_eq!(report.preserved_unknown_keys.len(), 2);
}

#[test]
fn synthesis_is_one_revision_undoable_and_recoverable_after_reopen() {
    let root = std::env::temp_dir().join(format!("verdant-synthesis-journal-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("song.verp");
    let mut document = ProjectPersistence::create(&path, Project::new_empty()).unwrap();
    let selected = SynthesisSettings::native_v10();
    let commit = document
        .commit(document.revision(), CommandBatch::single("Native v10", Command::SetSynthesis { settings: Some(selected.clone()) }))
        .unwrap()
        .unwrap();
    assert!(commit.meta_changed);
    assert_eq!(document.undo_depth(), 1);
    assert_eq!(document.project().render_preferences.synthesis, Some(selected.clone()));
    drop(document);
    let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits, 1);
    assert_eq!(document.project().render_preferences.synthesis, Some(selected.clone()));
    document.undo().unwrap();
    assert!(document.project().render_preferences.synthesis.is_none());
    document.redo().unwrap();
    assert_eq!(document.project().render_preferences.synthesis, Some(selected));
    document.save().unwrap();
    drop(document);
    let (document, _) = ProjectPersistence::open(&path).unwrap();
    assert_eq!(document.project().render_preferences.synthesis, Some(SynthesisSettings::native_v10()));
    drop(document);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_parameters_are_atomic_and_noop_batch_does_not_make_history() {
    let mut store = ProjectStore::new(Project::new_empty());
    let initial = store.revision();
    let mut bad = SynthesisSettings::native_v10();
    bad.vowel_balance_db = f64::NAN;
    assert!(store.apply(initial, CommandBatch::single("bad", Command::SetSynthesis { settings: Some(bad) })).is_err());
    assert_eq!(store.revision(), initial);
    let batch = CommandBatch {
        label: "cancel within batch".into(),
        commands: vec![Command::SetSynthesis { settings: Some(SynthesisSettings::native_v10()) }, Command::SetSynthesis { settings: None }],
    };
    assert!(store.apply(initial, batch).unwrap().is_none());
    assert_eq!(store.undo_depth(), 0);
    assert_eq!(store.revision(), initial);
}

#[test]
fn phonetic_correction_round_trips_recovers_and_undoes_without_losing_unknown_metadata() {
    let root = std::env::temp_dir().join(format!("verdant-phonetic-journal-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("song.verp");
    let mut initial = Project::new_empty();
    initial.render_preferences.synthesis = Some(SynthesisSettings::native_v10());
    let mut document = ProjectPersistence::create(&path, initial.clone()).unwrap();
    let mut settings = SynthesisSettings::native_v10();
    let mut correction = verdant_domain::PhoneticCorrectionSettings::articulatory_v1();
    correction.unknown.insert("future_recipe_note".into(), serde_json::json!("keep"));
    settings.phonetic_correction = Some(correction);
    document.commit(document.revision(), CommandBatch::single("phonetic", Command::SetSynthesis { settings: Some(settings.clone()) })).unwrap();
    drop(document);
    let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits, 1);
    assert_eq!(document.project().render_preferences.synthesis, Some(settings.clone()));
    let (round_trip, report) = parse_project(&serde_json::to_string(document.project()).unwrap()).unwrap();
    assert_eq!(round_trip.render_preferences.synthesis, Some(settings));
    assert!(report.preserved_unknown_keys.iter().any(|s| s.contains("phonetic_correction/future_recipe_note")));
    document.undo().unwrap();
    assert_eq!(document.project().render_preferences.synthesis, initial.render_preferences.synthesis);
    drop(document);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_phonetic_settings_are_rejected_atomically() {
    let mut store = ProjectStore::new(Project::new_empty());
    for strength in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
        let mut settings = SynthesisSettings::native_v10();
        let mut correction = verdant_domain::PhoneticCorrectionSettings::articulatory_v1();
        correction.strength = strength;
        settings.phonetic_correction = Some(correction);
        let revision = store.revision();
        assert!(store.apply(revision, CommandBatch::single("bad", Command::SetSynthesis { settings: Some(settings) })).is_err());
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), 0);
    }
}

#[test]
fn vocoder_highband_round_trips_recovers_and_undoes_without_changing_native_settings() {
    assert!(Project::new_empty().render_preferences.vocoder_highband.is_none());
    let v10 = SynthesisSettings::native_v10();
    assert_eq!(v10.wideband.as_ref().unwrap().method, "matched-balanced");
    assert_eq!(v10.wideband.as_ref().unwrap().presence_db, 24.0);
    assert_eq!(v10.vowel_balance_db, 18.0);
    for (index, synthesis) in [None, Some(SynthesisSettings::default()), Some(v10)].into_iter().enumerate() {
        let root = std::env::temp_dir().join(format!("verdant-vocoder-highband-journal-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("song.verp");
        let mut initial = Project::new_empty();
        initial.render_preferences.synthesis = synthesis.clone();
        let before = initial.render_preferences.clone();
        let mut document = ProjectPersistence::create(&path, initial).unwrap();
        let mut guard = verdant_domain::VocoderHighbandSettings::source_coherent_v1();
        guard.strength = 0.5;
        guard.unknown.insert("future_guard_note".into(), serde_json::json!("keep"));
        let selected = Some(guard);
        let commit = document.commit(document.revision(), CommandBatch::single("C1 高频保护", Command::SetVocoderHighband {
            settings: selected.clone(),
        })).unwrap().unwrap();
        assert!(commit.meta_changed);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        drop(document);
        let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 1);
        assert_eq!(document.project().render_preferences.vocoder_highband, selected);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        let value = serde_json::to_value(document.project()).unwrap();
        assert_eq!(value["render_preferences"]["vocoder_highband"]["strength"], 0.5);
        assert!(value["render_preferences"]["synthesis"].get("vocoder_highband").is_none());
        let (parsed, report) = parse_project(&value.to_string()).unwrap();
        assert_eq!(parsed.render_preferences.vocoder_highband, selected);
        assert!(report.preserved_unknown_keys.iter().any(|s| s.contains("/render_preferences/vocoder_highband/future_guard_note")));
        document.undo().unwrap();
        assert_eq!(document.project().render_preferences, before);
        document.redo().unwrap();
        assert_eq!(document.project().render_preferences.vocoder_highband, selected);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        document.save().unwrap();
        drop(document);
        let (document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 0);
        assert_eq!(document.project().render_preferences.vocoder_highband, selected);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        drop(document);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn invalid_vocoder_highband_parameters_and_cancelled_batches_do_not_mutate_history() {
    let mut store = ProjectStore::new(Project::new_empty());
    for strength in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
        let mut guard = verdant_domain::VocoderHighbandSettings::source_coherent_v1();
        guard.strength = strength;
        let revision = store.revision();
        assert!(store.apply(revision, CommandBatch::single("bad", Command::SetVocoderHighband { settings: Some(guard) })).is_err());
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), 0);
    }
    let mut guard = verdant_domain::VocoderHighbandSettings::source_coherent_v1();
    guard.method = "unknown".into();
    assert!(store.apply(store.revision(), CommandBatch::single("bad", Command::SetVocoderHighband { settings: Some(guard) })).is_err());
    assert_eq!(store.undo_depth(), 0);
    let revision = store.revision();
    let cancelled = CommandBatch { label: "guard on then off".into(), commands: vec![
        Command::SetVocoderHighband { settings: Some(verdant_domain::VocoderHighbandSettings::source_coherent_v1()) },
        Command::SetVocoderHighband { settings: None },
    ] };
    assert!(store.apply(revision, cancelled).unwrap().is_none());
    assert_eq!(store.revision(), revision);
    assert_eq!(store.undo_depth(), 0);
    assert!(store.project().render_preferences.synthesis.is_none());
    assert!(store.project().render_preferences.vocoder_highband.is_none());
    let mut bad = verdant_domain::VocoderHighbandSettings::source_coherent_v1();
    bad.strength = -1.0;
    let atomic = CommandBatch { label: "invalid transaction".into(), commands: vec![
        Command::SetSynthesis { settings: Some(SynthesisSettings::native_v10()) },
        Command::SetVocoderHighband { settings: Some(bad) },
    ] };
    assert!(store.apply(revision, atomic).is_err());
    assert!(store.project().render_preferences.synthesis.is_none());
    assert!(store.project().render_preferences.vocoder_highband.is_none());
    assert_eq!(store.revision(), revision);
    assert_eq!(store.undo_depth(), 0);
}
