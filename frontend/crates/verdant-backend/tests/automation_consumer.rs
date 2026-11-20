use verdant_backend::automation_plan::compile_automation;
use verdant_domain::automation::{AUTOMATION_KEY, AutomationDocument, AutomationLane, AutomationOwner, AutomationPoint, AutomationTarget, Interpolation};
use verdant_domain::model::TempoPoint;
use verdant_domain::{Command, CommandBatch, Project, ProjectStore, Revision, Track};

fn save_automation(project: &mut Project, document: &AutomationDocument) {
    document.validate(project).unwrap();
    match document.extension_value().unwrap() {
        Some(value) => { project.extensions.insert(AUTOMATION_KEY.into(), value); }
        None => { project.extensions.remove(AUTOMATION_KEY); }
    }
}

#[test]
fn persisted_tick_ramp_follows_tempo_and_absolute_seek_ranges() {
    let mut project = Project::new_empty();
    project.tempo_map = vec![TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() },
        TempoPoint { tick: 960, micro_bpm: 60_000_000, unknown: Default::default() }];
    let mut lane = AutomationLane::new(AutomationTarget::TrackPan { track_id: project.tracks[0].id.clone() });
    lane.set_point(AutomationPoint::new(0, -1000.0)).unwrap();
    lane.set_point(AutomationPoint::new(1920, 1000.0)).unwrap();
    let mut document = AutomationDocument::default();
    document.set_lane(lane);
    save_automation(&mut project, &document);
    let serialized = serde_json::to_vec(&project).unwrap();
    let restored: Project = serde_json::from_slice(&serialized).unwrap();
    let schedule = compile_automation(&restored, 48_000).unwrap();
    let curve = &schedule.tracks[0].pan;
    assert_eq!(curve.points().iter().map(|point| point.sample).collect::<Vec<_>>(), vec![0, 24_000, 72_000]);
    assert_eq!(curve.value_at(12_000, 0.0), -500.0);
    assert_eq!(curve.value_at(48_000, 0.0), 500.0);
    let whole = curve.values(23_900, 500, 0.0).collect::<Vec<_>>();
    let chunks = curve.values(23_900, 100, 0.0).chain(curve.values(24_000, 400, 0.0)).collect::<Vec<_>>();
    assert_eq!(whole, chunks, "device blocks and export ranges use identical sample values");
    assert_eq!(curve.events(48_000, 1, 0.0).next().unwrap().value, 500.0, "seek reconstructs held/ramped state");
    assert_eq!(curve.events(0, 1, 0.0).next().unwrap().value, -1000.0, "loop restart reconstructs its first value");
}

#[test]
fn track_and_insert_reordering_bind_stable_identities() {
    let mut project = Project::new_empty();
    let first = project.tracks[0].id.clone();
    project.tracks[0].unknown.insert("inserts".into(), serde_json::json!([
        {"format":"clap","id":"vendor.eq","instance_id":"eq-A"},
        {"format":"clap","id":"vendor.eq","instance_id":"eq-B"}
    ]));
    project.tracks.push(Track::new("harmony", "blue"));
    let mut document = AutomationDocument::default();
    let mut gain = AutomationLane::new(AutomationTarget::TrackGain { track_id: first.clone() });
    gain.set_point(AutomationPoint::new(0, -6000.0)).unwrap();
    document.set_lane(gain);
    let mut plugin = AutomationLane::new(AutomationTarget::PluginParameter { owner: AutomationOwner::Track { track_id: first },
        instance_id: "eq-B".into(), parameter_id: "73".into() });
    let mut point = AutomationPoint::new(240, 4.5);
    point.interpolation = Interpolation::Step;
    plugin.set_point(point).unwrap();
    document.set_lane(plugin);
    save_automation(&mut project, &document);
    project.tracks[0].unknown.get_mut("inserts").unwrap().as_array_mut().unwrap().reverse();
    project.tracks.reverse();
    let schedule = compile_automation(&project, 48_000).unwrap();
    assert!(schedule.tracks[0].is_empty());
    assert!((schedule.tracks[1].gain_at(0, 1.0) - 0.5011872).abs() < 1e-6);
    assert_eq!(schedule.tracks[1].plugins[0].instance_id, "eq-B");
    assert_eq!(schedule.tracks[1].plugins[0].parameter_id, "73");
    assert_eq!(schedule.tracks[1].plugins[0].curve.events(6000, 1, 0.0).next().unwrap().value, 4.5);
}

#[test]
fn automation_and_channel_edit_commit_and_undo_as_one_intent() {
    let project = Project::new_empty();
    let mut store = ProjectStore::new(project);
    let before = store.content_digest();
    let track_id = store.project().tracks[0].id.clone();
    let mut header = store.project().tracks[0].header();
    header.gain_millidecibels = -3000;
    let mut lane = AutomationLane::new(AutomationTarget::TrackGain { track_id: track_id.clone() });
    lane.set_point(AutomationPoint::new(0, -12000.0)).unwrap();
    let mut document = AutomationDocument::default();
    document.set_lane(lane);
    store.apply(store.revision(), CommandBatch { label: "channel and automation".into(), commands: vec![
        Command::SetTrackHeader { track: track_id, header },
        Command::SetProjectExtension { key: AUTOMATION_KEY.into(), value: document.extension_value().unwrap() },
    ] }).unwrap();
    assert_eq!(store.undo_depth(), 1);
    let after = store.content_digest();
    assert_ne!(before, after);
    assert!(!compile_automation(store.project(), 48_000).unwrap().is_empty());
    store.undo().unwrap();
    assert_eq!(store.content_digest(), before);
    assert!(!store.project().extensions.contains_key(AUTOMATION_KEY));
    store.redo().unwrap();
    assert_eq!(store.content_digest(), after);
    assert_eq!(store.revision(), Revision::new(3).unwrap());
}

#[test]
fn invalid_final_automation_rolls_back_the_whole_consumer_batch() {
    let mut store = ProjectStore::new(Project::new_empty());
    let before = store.content_digest();
    let track_id = store.project().tracks[0].id.clone();
    let mut header = store.project().tracks[0].header();
    header.pan_permille = 500;
    let invalid = serde_json::json!({"version":1,"lanes":[{"target":{"kind":"track_gain","track_id":"deleted"},
        "points":[{"tick":0,"value":0.0}]}]});
    assert!(store.apply(store.revision(), CommandBatch { label: "invalid mixed edit".into(), commands: vec![
        Command::SetTrackHeader { track: track_id, header },
        Command::SetProjectExtension { key: AUTOMATION_KEY.into(), value: Some(invalid) },
    ] }).is_err());
    assert_eq!(store.content_digest(), before);
    assert_eq!(store.undo_depth(), 0);
}
