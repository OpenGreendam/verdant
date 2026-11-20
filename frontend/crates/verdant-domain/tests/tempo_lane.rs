use verdant_domain::io::{load_project, parse_project, save_project_atomic};
use verdant_domain::model::{PPQ, Project, TempoPoint};
use verdant_domain::{Command, CommandBatch, ProjectStore, TempoMap};

fn point(tick: i64, micro_bpm: u64) -> TempoPoint {
    TempoPoint { tick, micro_bpm, unknown: Default::default() }
}

fn apply_event(store: &mut ProjectStore, tick: i64, micro_bpm: Option<u64>) {
    store.apply(store.revision(), CommandBatch::single("编辑速度事件", Command::SetTempoEvent { tick, micro_bpm }))
        .unwrap().unwrap();
}

#[test]
fn inserting_and_deleting_mid_piece_tempo_preserves_exact_prior_integration() {
    let mut project = Project::new_empty();
    project.tempo_map = vec![point(0, 123_456_789), point(4 * PPQ, 91_500_000)];
    let original = project.tempo_map.clone();
    let baseline = TempoMap::new(&original, PPQ).unwrap();
    let mut store = ProjectStore::new(project);
    let boundary = 2 * PPQ;
    apply_event(&mut store, boundary, Some(60_000_000));
    assert_eq!(store.undo_depth(), 1);
    let inserted = TempoMap::new(&store.project().tempo_map, PPQ).unwrap();
    for tick in [-0.5, 0.0, 1.25, (boundary - 1) as f64, boundary as f64] {
        assert_eq!(inserted.seconds_at(tick), baseline.seconds_at(tick));
    }
    let later = (boundary + PPQ) as f64;
    assert!(inserted.seconds_at(later) > baseline.seconds_at(later));
    assert_eq!(inserted.seconds_at(later), baseline.seconds_at(boundary as f64) + 1.0);
    assert_eq!(inserted.bpm_at(4 * PPQ), 91.5);

    apply_event(&mut store, boundary, None);
    assert_eq!(store.undo_depth(), 2);
    assert_eq!(store.project().tempo_map, original);
    let restored = TempoMap::new(&store.project().tempo_map, PPQ).unwrap();
    for tick in [-0.5, 0.0, 1.25, boundary as f64, later, 4.0 * PPQ as f64, 10.5 * PPQ as f64] {
        assert_eq!(restored.seconds_at(tick).to_bits(), baseline.seconds_at(tick).to_bits());
    }
    store.undo().unwrap();
    assert_eq!(store.project().tempo_map, inserted.points());
    store.undo().unwrap();
    assert_eq!(store.project().tempo_map, original);
}

#[test]
fn imported_multi_point_tempo_round_trips_through_load_and_save_unchanged() {
    let mut project = Project::new_empty();
    project.tempo_map = vec![point(0, 91_500_001), point(137, 129_123_456), point(7 * PPQ + 3, 82_765_432)];
    project.tempo_map[1].unknown.insert("imported_event".into(), serde_json::json!({"id": 3, "shape": "hold"}));
    let source = serde_json::to_value(&project).unwrap();
    let (imported, _) = parse_project(&source.to_string()).unwrap();
    let store = ProjectStore::new(imported);
    assert!(!store.is_dirty());
    assert_eq!(store.undo_depth(), 0);
    let directory = std::env::temp_dir().join(format!("verdant-tempo-{}", uuid::Uuid::new_v4()));
    let path = directory.join("imported.verp");
    save_project_atomic(&path, &store.project_for_save()).unwrap();
    let (reloaded, _) = load_project(&path).unwrap();
    assert_eq!(reloaded.tempo_map, project.tempo_map);
    assert_eq!(serde_json::to_value(&reloaded).unwrap(), source);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn tempo_edits_are_sorted_preserve_metadata_and_reject_invalid_events_atomically() {
    let mut project = Project::new_empty();
    project.tempo_map = vec![point(0, 120_000_000), point(4 * PPQ, 90_000_000)];
    project.tempo_map[1].unknown.insert("source_event".into(), serde_json::json!("kept"));
    let mut store = ProjectStore::new(project);
    apply_event(&mut store, 2 * PPQ, Some(100_000_000));
    apply_event(&mut store, PPQ, Some(110_000_000));
    apply_event(&mut store, 4 * PPQ, Some(95_123_456));
    apply_event(&mut store, 4 * PPQ, Some(1_200_000_000));
    assert_eq!(store.project().tempo_map.iter().map(|p| p.tick).collect::<Vec<_>>(), [0, PPQ, 2 * PPQ, 4 * PPQ]);
    assert_eq!(store.project().tempo_map[3].unknown["source_event"], "kept");
    let before = store.snapshot();
    let revision = store.revision();
    let depth = store.undo_depth();
    for (tick, micro_bpm) in [(-1, Some(120_000_000)), (PPQ, Some(0)), (0, None)] {
        assert!(store.apply(revision, CommandBatch::single("无效速度事件", Command::SetTempoEvent { tick, micro_bpm })).is_err());
        assert_eq!(store.project(), before.as_ref());
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), depth);
    }
    assert!(store.apply(revision, CommandBatch::single("相同速度", Command::SetTempoEvent { tick: PPQ, micro_bpm: Some(110_000_000) })).unwrap().is_none());
    assert_eq!(store.undo_depth(), depth);
}

#[test]
fn tempo_point_drag_is_one_undo_step_and_serializes_for_journal_replay() {
    let mut project = Project::new_empty();
    project.tempo_map = vec![point(0, 120_000_000), point(PPQ, 100_000_000), point(4 * PPQ, 90_000_000)];
    project.tempo_map[1].unknown.insert("source_id".into(), serde_json::json!(17));
    let original = project.tempo_map.clone();
    let mut store = ProjectStore::new(project);
    assert!(store.apply(store.revision(), CommandBatch::single("点击速度点", Command::SetTempoEvent {
        tick: PPQ, micro_bpm: Some(100_000_000),
    })).unwrap().is_none());
    assert_eq!(store.undo_depth(), 0);
    assert!(!store.is_dirty());
    let batch = CommandBatch { label: "移动速度事件".into(), commands: vec![
        Command::SetTempoEvent { tick: PPQ, micro_bpm: None },
        Command::SetTempoEvent { tick: 2 * PPQ, micro_bpm: Some(105_123_456) },
    ] };
    let serialized = serde_json::to_string(&batch).unwrap();
    let replay: CommandBatch = serde_json::from_str(&serialized).unwrap();
    store.apply(store.revision(), replay).unwrap().unwrap();
    assert_eq!(store.undo_depth(), 1);
    assert_eq!(store.project().tempo_map[1].tick, 2 * PPQ);
    assert_eq!(store.project().tempo_map[1].micro_bpm, 105_123_456);
    let moved = store.project().tempo_map.clone();
    store.undo().unwrap();
    assert_eq!(store.project().tempo_map, original);
    store.redo().unwrap();
    assert_eq!(store.project().tempo_map, moved);
}
