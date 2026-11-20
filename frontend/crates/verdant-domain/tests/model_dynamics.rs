use verdant_domain::io::parse_project;
use verdant_domain::{Command, CommandBatch, ModelDynamicsSettings, Project, ProjectPersistence, ProjectStore, SynthesisSettings, VocalTractSettings};

#[test]
fn optional_dynamics_recovers_undoes_and_saves_without_changing_legacy_synthesis_or_b() {
    let old = serde_json::to_value(Project::new_empty()).unwrap();
    assert!(old["render_preferences"].get("model_dynamics").is_none());
    assert!(parse_project(&old.to_string()).unwrap().0.render_preferences.model_dynamics.is_none());
    for (index, synthesis) in [None, Some(SynthesisSettings::default()), Some(SynthesisSettings::native_v10())].into_iter().enumerate() {
        let root = std::env::temp_dir().join(format!("verdant-model-dynamics-journal-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("song.verp");
        let mut initial = Project::new_empty();
        initial.render_preferences.synthesis = synthesis.clone();
        initial.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(1.125));
        initial.render_preferences.native_seed = 6789;
        let before = initial.render_preferences.clone();
        let mut document = ProjectPersistence::create(&path, initial).unwrap();
        let mut dynamics = ModelDynamicsSettings::voiced_envelope_v1();
        dynamics.strength = 0.123456789;
        dynamics.max_attenuation_db = 4.56789;
        dynamics.unknown.insert("future_profile_note".into(), serde_json::json!("keep"));
        let selected = Some(dynamics);
        let commit = document.commit(document.revision(), CommandBatch::single("元音动态", Command::SetModelDynamics { settings: selected.clone() })).unwrap().unwrap();
        assert!(commit.meta_changed);
        assert_eq!(document.undo_depth(), 1);
        let mut unchanged = document.project().render_preferences.clone();
        unchanged.model_dynamics = None;
        assert_eq!(unchanged, before);
        drop(document);
        let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 1);
        assert_eq!(document.project().render_preferences.model_dynamics, selected);
        let value = serde_json::to_value(document.project()).unwrap();
        assert_eq!(value["render_preferences"]["model_dynamics"]["strength"], 0.123456789);
        assert!(value["render_preferences"]["synthesis"].get("model_dynamics").is_none());
        let (parsed, report) = parse_project(&value.to_string()).unwrap();
        assert_eq!(parsed.render_preferences.model_dynamics, selected);
        assert!(report.preserved_unknown_keys.iter().any(|s| s == "/render_preferences/model_dynamics/future_profile_note"));
        document.undo().unwrap();
        assert_eq!(document.project().render_preferences, before);
        document.redo().unwrap();
        document.save().unwrap();
        drop(document);
        let (document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 0);
        assert_eq!(document.project().render_preferences.model_dynamics, selected);
        let mut unchanged = document.project().render_preferences.clone();
        unchanged.model_dynamics = None;
        assert_eq!(unchanged, before);
        drop(document);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn invalid_or_cancelled_dynamics_transactions_leave_document_and_history_untouched() {
    let mut store = ProjectStore::new(Project::new_empty());
    let original = store.project().render_preferences.clone();
    let revision = store.revision();
    for (strength, attenuation) in [(-0.01, 6.0), (1.01, 6.0), (f64::NAN, 6.0), (f64::INFINITY, 6.0),
            (0.25, -0.01), (0.25, 12.01), (0.25, f64::NAN), (0.25, f64::INFINITY)] {
        let mut settings = ModelDynamicsSettings::voiced_envelope_v1();
        settings.strength = strength;
        settings.max_attenuation_db = attenuation;
        let batch = CommandBatch { label: "invalid transaction".into(), commands: vec![
            Command::SetVocalTract { settings: Some(VocalTractSettings::bilinear_v1(1.125)) },
            Command::SetModelDynamics { settings: Some(settings) },
        ] };
        assert!(store.apply(revision, batch).is_err());
        assert_eq!(store.project().render_preferences, original);
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), 0);
    }
    let mut unsupported = ModelDynamicsSettings::voiced_envelope_v1();
    unsupported.method = "unknown".into();
    assert!(store.apply(revision, CommandBatch::single("bad", Command::SetModelDynamics { settings: Some(unsupported) })).is_err());
    let batch = CommandBatch { label: "cancelled dynamics".into(), commands: vec![
        Command::SetModelDynamics { settings: Some(ModelDynamicsSettings::voiced_envelope_v1()) },
        Command::SetModelDynamics { settings: None },
    ] };
    assert!(store.apply(revision, batch).unwrap().is_none());
    assert_eq!(store.project().render_preferences, original);
    assert_eq!(store.revision(), revision);
    assert_eq!(store.undo_depth(), 0);
}

#[test]
fn explicit_zero_and_endpoint_settings_remain_distinct_saved_choices() {
    for (strength, attenuation) in [(0.0, 6.0), (0.25, 0.0), (1.0, 12.0)] {
        let mut store = ProjectStore::new(Project::new_empty());
        let mut settings = ModelDynamicsSettings::voiced_envelope_v1();
        settings.strength = strength;
        settings.max_attenuation_db = attenuation;
        store.apply(store.revision(), CommandBatch::single("元音动态", Command::SetModelDynamics { settings: Some(settings.clone()) })).unwrap().unwrap();
        let text = serde_json::to_string(store.project()).unwrap();
        let (parsed, _) = parse_project(&text).unwrap();
        assert_eq!(parsed.render_preferences.model_dynamics, Some(settings));
        store.undo().unwrap();
        assert!(store.project().render_preferences.model_dynamics.is_none());
    }
}
