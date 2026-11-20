use verdant_domain::io::parse_project;
use verdant_domain::{Command, CommandBatch, Project, ProjectPersistence, ProjectStore, SynthesisSettings, VocalTractSettings};

#[test]
fn independent_tract_setting_recovers_undoes_saves_and_preserves_unknown_fields() {
    let old = serde_json::to_value(Project::new_empty()).unwrap();
    assert!(old["render_preferences"].get("vocal_tract").is_none());
    assert!(parse_project(&old.to_string()).unwrap().0.render_preferences.vocal_tract.is_none());
    for (index, synthesis) in [None, Some(SynthesisSettings::default()), Some(SynthesisSettings::native_v10())].into_iter().enumerate() {
        let root = std::env::temp_dir().join(format!("verdant-vocal-tract-journal-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("song.verp");
        let mut initial = Project::new_empty();
        initial.render_preferences.synthesis = synthesis.clone();
        let before = initial.render_preferences.clone();
        let mut document = ProjectPersistence::create(&path, initial).unwrap();
        let mut tract = VocalTractSettings::bilinear_v1(1.123456789);
        tract.unknown.insert("future_profile_note".into(), serde_json::json!("keep"));
        let selected = Some(tract);
        let commit = document.commit(document.revision(), CommandBatch::single("声道变换", Command::SetVocalTract { settings: selected.clone() })).unwrap().unwrap();
        assert!(commit.meta_changed);
        assert_eq!(document.undo_depth(), 1);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        drop(document);
        let (mut document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 1);
        assert_eq!(document.project().render_preferences.vocal_tract, selected);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        assert_eq!(document.project().render_preferences.native_seed, before.native_seed);
        let value = serde_json::to_value(document.project()).unwrap();
        assert_eq!(value["render_preferences"]["vocal_tract"]["ratio"], 1.123456789);
        assert!(value["render_preferences"]["synthesis"].get("vocal_tract").is_none());
        let (parsed, report) = parse_project(&value.to_string()).unwrap();
        assert_eq!(parsed.render_preferences.vocal_tract, selected);
        assert!(report.preserved_unknown_keys.iter().any(|s| s == "/render_preferences/vocal_tract/future_profile_note"));
        document.undo().unwrap();
        assert_eq!(document.project().render_preferences, before);
        document.redo().unwrap();
        assert_eq!(document.project().render_preferences.vocal_tract, selected);
        document.save().unwrap();
        drop(document);
        let (document, recovery) = ProjectPersistence::open(&path).unwrap();
        assert_eq!(recovery.replayed_commits, 0);
        assert_eq!(document.project().render_preferences.vocal_tract, selected);
        assert_eq!(document.project().render_preferences.synthesis, synthesis);
        drop(document);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn invalid_tract_and_cancelled_transactions_preserve_document_and_history() {
    let mut store = ProjectStore::new(Project::new_empty());
    let original = store.project().render_preferences.clone();
    let revision = store.revision();
    for ratio in [0.799, 1.251, f64::NAN, f64::INFINITY] {
        let batch = CommandBatch { label: "invalid transaction".into(), commands: vec![
            Command::SetSynthesis { settings: Some(SynthesisSettings::native_v10()) },
            Command::SetVocalTract { settings: Some(VocalTractSettings::bilinear_v1(ratio)) },
        ] };
        assert!(store.apply(revision, batch).is_err());
        assert_eq!(store.project().render_preferences, original);
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), 0);
    }
    let mut unsupported = VocalTractSettings::bilinear_v1(1.25);
    unsupported.method = "unknown".into();
    assert!(store.apply(revision, CommandBatch::single("bad", Command::SetVocalTract { settings: Some(unsupported) })).is_err());
    let batch = CommandBatch { label: "cancelled transform".into(), commands: vec![
        Command::SetVocalTract { settings: Some(VocalTractSettings::bilinear_v1(1.25)) },
        Command::SetVocalTract { settings: None },
    ] };
    assert!(store.apply(revision, batch).unwrap().is_none());
    assert_eq!(store.project().render_preferences, original);
    assert_eq!(store.revision(), revision);
    assert_eq!(store.undo_depth(), 0);
}

#[test]
fn anchored_global_profile_roundtrips_and_rejects_mismatched_parameters() {
    let mut store = ProjectStore::new(Project::new_empty());
    let before = store.project().render_preferences.clone();
    let setting = VocalTractSettings::anchored_warp_v1(1.125, [0.9, 1.1]);
    store.apply(store.revision(), CommandBatch::single("全局声道", Command::SetVocalTract {
        settings: Some(setting.clone()),
    })).unwrap().unwrap();
    let value = serde_json::to_value(store.project()).unwrap();
    assert_eq!(value["render_preferences"]["vocal_tract"], serde_json::json!({
        "method": "anchored-warp-v1", "ratio": 1.125, "anchor_ratios": [0.9, 1.1],
    }));
    let (parsed, report) = parse_project(&value.to_string()).unwrap();
    assert_eq!(parsed.render_preferences.vocal_tract, Some(setting.clone()));
    assert!(!report.preserved_unknown_keys.iter().any(|key| key.contains("anchor_ratios")));
    store.undo().unwrap();
    assert_eq!(store.project().render_preferences, before);
    store.redo().unwrap();
    assert_eq!(store.project().render_preferences.vocal_tract, Some(setting));
    for anchors in [[0.799, 1.0], [1.0, 1.251], [f64::NAN, 1.0], [1.0, f64::INFINITY]] {
        assert!(VocalTractSettings::anchored_warp_v1(1.125, anchors).validate().is_err());
    }
    let mut missing = VocalTractSettings::bilinear_v1(1.125);
    missing.method = "anchored-warp-v1".into();
    assert!(missing.validate().is_err());
    missing.method = "bilinear-v1".into();
    missing.anchor_ratios = Some([1.0, 1.0]);
    assert!(missing.validate().is_err());
    for anchors in [serde_json::json!([1.0]), serde_json::json!([1.0, 1.0, 1.0])] {
        assert!(serde_json::from_value::<VocalTractSettings>(serde_json::json!({
            "method": "anchored-warp-v1", "ratio": 1.125, "anchor_ratios": anchors,
        })).is_err());
    }
}
