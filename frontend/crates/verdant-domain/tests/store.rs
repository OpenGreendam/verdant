use verdant_domain::io::{load_project, parse_project, save_project_atomic};
use verdant_domain::model::{Note, Project};
use verdant_domain::{Command, CommandBatch, Edge, NoteId, ProjectStore, Revision, TrackId};

const EXAMPLE: &str = include_str!("../../../../spec/examples/project.gdm.json");

fn fixture() -> (ProjectStore, TrackId) {
    let mut p = Project::new_empty();
    p.revision = 7;
    let track = p.tracks[0].id.clone();
    for (id, start, pitch) in [("n1", 960, 60), ("n2", 1440, 64)] {
        let mut n = Note::new(start, 480, pitch);
        n.id = NoteId::from(id);
        p.tracks[0].notes.push(n);
    }
    (ProjectStore::new(p), track)
}

fn rev(n: u64) -> Revision {
    Revision::new(n).unwrap()
}

#[test]
fn example_project_round_trips_without_loss() {
    let (p, report) = parse_project(EXAMPLE).unwrap();
    assert!(report.preserved_unknown_keys.is_empty());
    let original: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
    let again = serde_json::to_value(&p).unwrap();
    assert_eq!(original, again);
}

#[test]
fn unknown_fields_are_preserved() {
    let mut v: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
    v["future_field"] = serde_json::json!({"x": 1});
    v["tracks"][0]["notes"][0]["vibrato_future"] = serde_json::json!(true);
    let (p, report) = parse_project(&v.to_string()).unwrap();
    assert_eq!(report.preserved_unknown_keys.len(), 2);
    assert_eq!(serde_json::to_value(&p).unwrap(), v);
}

#[test]
fn rejects_wrong_format_and_major_version() {
    assert!(parse_project(r#"{"format":"other","version":"0.1"}"#).is_err());
    let mut v: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
    v["version"] = "1.0".into();
    assert!(parse_project(&v.to_string()).is_err());
}

#[test]
fn group_move_is_one_revision_and_one_undo() {
    let (mut s, t) = fixture();
    let c = s
        .apply(
            rev(7),
            CommandBatch::single(
                "移动",
                Command::MoveNotes { track: t.clone(), note_ids: vec!["n1".into(), "n2".into()], delta_tick: 240, delta_semitones: 2 },
            ),
        )
        .unwrap()
        .unwrap();
    assert_eq!(c.revision, rev(8));
    assert_eq!(s.undo_depth(), 1);
    let tr = s.project().track(&t).unwrap();
    assert_eq!((tr.notes[0].start_tick, tr.notes[0].midi_note), (1200, 62));
    assert_eq!((tr.notes[1].start_tick, tr.notes[1].midi_note), (1680, 66));
}

#[test]
fn noop_creates_no_revision() {
    let (mut s, t) = fixture();
    let r = s
        .apply(
            rev(7),
            CommandBatch::single("noop", Command::MoveNotes { track: t, note_ids: vec!["n1".into()], delta_tick: 0, delta_semitones: 0 }),
        )
        .unwrap();
    assert!(r.is_none());
    assert_eq!(s.revision(), rev(7));
    assert_eq!(s.undo_depth(), 0);
}

#[test]
fn stale_base_is_refused() {
    let (mut s, t) = fixture();
    let e = s.apply(rev(6), CommandBatch::single("x", Command::DeleteNotes { track: t, note_ids: vec!["n1".into()] }));
    assert!(e.is_err());
    assert_eq!(s.revision(), rev(7));
}

#[test]
fn undo_redo_are_new_monotonic_revisions() {
    let (mut s, t) = fixture();
    let before = s.content_digest();
    s.apply(rev(7), CommandBatch::single("删", Command::DeleteNotes { track: t.clone(), note_ids: vec!["n1".into()] })).unwrap();
    assert!(s.is_dirty());
    let u = s.undo().unwrap().unwrap();
    assert_eq!(u.revision, rev(9));
    assert_eq!(s.content_digest(), before, "undo restores content exactly");
    assert!(!s.is_dirty());
    let r = s.redo().unwrap().unwrap();
    assert_eq!(r.revision, rev(10));
    assert!(s.project().track(&t).unwrap().note(&"n1".into()).is_none());
}

#[test]
fn group_move_clamps_as_a_group() {
    let (mut s, t) = fixture();
    s.apply(
        rev(7),
        CommandBatch::single(
            "m",
            Command::MoveNotes { track: t.clone(), note_ids: vec!["n1".into(), "n2".into()], delta_tick: -5000, delta_semitones: 100 },
        ),
    )
    .unwrap();
    let tr = s.project().track(&t).unwrap();
    // Shape preserved: still 480 ticks and 4 semitones apart.
    assert_eq!(tr.notes[0].start_tick, 0);
    assert_eq!(tr.notes[1].start_tick - tr.notes[0].start_tick, 480);
    assert_eq!(tr.notes[1].midi_note, 127);
    assert_eq!(tr.notes[1].midi_note - tr.notes[0].midi_note, 4);
}

#[test]
fn resize_never_flips_or_goes_negative() {
    // T04 domain layer.
    let (mut s, t) = fixture();
    s.apply(
        rev(7),
        CommandBatch::single(
            "r",
            Command::ResizeNotes { track: t.clone(), note_ids: vec!["n1".into()], edge: Edge::End, delta_tick: -100_000 },
        ),
    )
    .unwrap();
    let n = s.project().track(&t).unwrap().note(&"n1".into()).unwrap().clone();
    assert_eq!((n.start_tick, n.duration_tick), (960, 1));
    s.apply(
        rev(8),
        CommandBatch::single(
            "r",
            Command::ResizeNotes { track: t.clone(), note_ids: vec!["n2".into()], edge: Edge::Start, delta_tick: 100_000 },
        ),
    )
    .unwrap();
    let n = s.project().track(&t).unwrap().note(&"n2".into()).unwrap().clone();
    assert_eq!((n.start_tick, n.duration_tick), (1919, 1));
}

#[test]
fn duplicate_then_undo_leaves_no_ids() {
    let (mut s, t) = fixture();
    s.apply(
        rev(7),
        CommandBatch::single(
            "dup",
            Command::DuplicateNotes {
                track: t.clone(),
                source_ids: vec!["n1".into()],
                new_ids: vec!["n1-copy".into()],
                delta_tick: 960,
                delta_semitones: 0,
            },
        ),
    )
    .unwrap();
    assert_eq!(s.project().note_count(), 3);
    s.undo().unwrap();
    assert_eq!(s.project().note_count(), 2);
    assert!(s.project().track(&t).unwrap().note(&"n1-copy".into()).is_none());
}

#[test]
fn failed_batch_is_atomic() {
    let (mut s, t) = fixture();
    let digest = s.content_digest();
    let r = s.apply(
        rev(7),
        CommandBatch {
            label: "partial".into(),
            commands: vec![
                Command::SetLyric { track: t.clone(), note_id: "n1".into(), text: "绿".into() },
                Command::SetLyric { track: t, note_id: "missing".into(), text: "坝".into() },
            ],
        },
    );
    assert!(r.is_err());
    assert_eq!(s.content_digest(), digest);
    assert_eq!(s.revision(), rev(7));
}

#[test]
fn atomic_save_and_reload() {
    let dir = std::env::temp_dir().join(format!("verdant-save-{}", uuid::Uuid::new_v4()));
    let path = dir.join("project.verp");
    let (s, _) = fixture();
    save_project_atomic(&path, &s.project_for_save()).unwrap();
    let (p, _) = load_project(&path).unwrap();
    assert_eq!(p.revision, 7);
    assert_eq!(p.note_count(), 2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn overlap_detection_for_monophonic_tracks() {
    let (mut s, t) = fixture();
    s.apply(
        rev(7),
        CommandBatch::single(
            "m",
            Command::MoveNotes { track: t.clone(), note_ids: vec!["n2".into()], delta_tick: -200, delta_semitones: 0 },
        ),
    )
    .unwrap();
    assert_eq!(s.project().track(&t).unwrap().overlaps().len(), 1);
}

// ---- multi-track & parameters ---------------------------------------------------

use verdant_domain::model::{GainPoint, PitchPoint, TrackHeader};
use verdant_domain::{PointId, Track, Vibrato};

#[test]
fn add_and_remove_tracks_undo_exactly() {
    let (mut s, t0) = fixture();
    let before = s.content_digest();
    let mut harmony = Track::new("和声", "blue");
    let mut n = Note::new(0, 480, 67);
    n.id = "h1".into();
    harmony.notes.push(n);
    let hid = harmony.id.clone();
    s.apply(rev(7), CommandBatch::single("新建轨道", Command::AddTrack { index: 1, track: harmony })).unwrap();
    assert_eq!(s.project().tracks.len(), 2);
    assert_eq!(s.project().tracks[1].id, hid);
    s.apply(rev(8), CommandBatch::single("删除主轨", Command::RemoveTrack { track: t0.clone() })).unwrap();
    assert_eq!(s.project().tracks.len(), 1);
    // The last track can never be removed.
    assert!(s.apply(rev(9), CommandBatch::single("x", Command::RemoveTrack { track: hid.clone() })).is_err());
    s.undo().unwrap();
    assert_eq!(s.project().tracks[0].id, t0, "removed track restored at its index");
    s.undo().unwrap();
    assert_eq!(s.content_digest(), before);
}

#[test]
fn track_header_edits_are_one_undoable_transaction() {
    let (mut s, t) = fixture();
    let mut h: TrackHeader = s.project().track(&t).unwrap().header();
    h.gain_millidecibels = -6000;
    h.pan_permille = -400;
    h.solo = true;
    h.render_route = Some("C1".into());
    s.apply(rev(7), CommandBatch::single("推子", Command::SetTrackHeader { track: t.clone(), header: h.clone() })).unwrap();
    assert_eq!(s.project().track(&t).unwrap().header(), h);
    s.undo().unwrap();
    assert_eq!(s.project().track(&t).unwrap().gain_millidecibels, 0);
    let mut bad = h.clone();
    bad.pan_permille = 5000;
    assert!(s.apply(s.revision(), CommandBatch::single("x", Command::SetTrackHeader { track: t, header: bad })).is_err());
}

#[test]
fn new_fields_stay_out_of_v01_files_when_unused() {
    let (p, _) = parse_project(EXAMPLE).unwrap();
    let v = serde_json::to_value(&p).unwrap();
    let track = &v["tracks"][0];
    for k in ["gain_millidecibels", "pan_permille", "muted", "solo", "render_route", "color"] {
        assert!(track.get(k).is_none(), "{k} must not appear unless set");
    }
    assert!(v["tracks"][0]["notes"][0].get("vibrato").is_none());
}

#[test]
fn curves_replace_dedupe_and_stretch_with_the_note() {
    let (mut s, t) = fixture();
    let pts = vec![
        PitchPoint { id: PointId::from("a"), tick: 0, millicents: 0, unknown: Default::default() },
        PitchPoint { id: PointId::from("b"), tick: 240, millicents: 50_000, unknown: Default::default() },
        PitchPoint { id: PointId::from("c"), tick: 240, millicents: 80_000, unknown: Default::default() },
        PitchPoint { id: PointId::from("d"), tick: 480, millicents: 0, unknown: Default::default() },
    ];
    s.apply(rev(7), CommandBatch::single("PIT", Command::SetPitchCurve { track: t.clone(), note_id: "n1".into(), points: pts })).unwrap();
    let n = s.project().track(&t).unwrap().note(&"n1".into()).unwrap().clone();
    assert_eq!(n.curves.pitch_millicent_points.len(), 3, "same tick: later point wins");
    assert_eq!(n.curves.pitch_millicent_points[1].millicents, 80_000);
    // Points must stay inside the note.
    let outside = vec![PitchPoint { id: PointId::from("z"), tick: 9999, millicents: 0, unknown: Default::default() }];
    assert!(
        s.apply(rev(8), CommandBatch::single("x", Command::SetPitchCurve { track: t.clone(), note_id: "n1".into(), points: outside }))
            .is_err()
    );
    // Doubling the length doubles point ticks (proportional stretch).
    s.apply(
        rev(8),
        CommandBatch::single(
            "拉长",
            Command::ResizeNotes { track: t.clone(), note_ids: vec!["n1".into()], edge: Edge::End, delta_tick: 480 },
        ),
    )
    .unwrap();
    let n = s.project().track(&t).unwrap().note(&"n1".into()).unwrap().clone();
    assert_eq!(n.curves.pitch_millicent_points.iter().map(|p| p.tick).collect::<Vec<_>>(), vec![0, 480, 960]);
    let dyn_pts = vec![GainPoint { id: PointId::from("g"), tick: 100, millidecibels: -3000, unknown: Default::default() }];
    s.apply(rev(9), CommandBatch::single("DYN", Command::SetGainCurve { track: t.clone(), note_id: "n1".into(), points: dyn_pts }))
        .unwrap();
    assert!(s.project().track(&t).unwrap().note(&"n1".into()).unwrap().has_f0_modulation());
}

#[test]
fn vibrato_and_portamento_validate() {
    let (mut s, t) = fixture();
    let ids = vec![NoteId::from("n1"), NoteId::from("n2")];
    s.apply(
        rev(7),
        CommandBatch::single("颤音", Command::SetVibrato { track: t.clone(), note_ids: ids.clone(), vibrato: Some(Vibrato::DEFAULT) }),
    )
    .unwrap();
    s.apply(
        rev(8),
        CommandBatch::single("滑音", Command::SetPortamento { track: t.clone(), note_ids: ids.clone(), portamento_ms: Some(80) }),
    )
    .unwrap();
    let silly = Vibrato { depth_cents: 900, rate_millihertz: 5000, delay_permille: 0, unknown: Default::default() };
    assert!(
        s.apply(rev(9), CommandBatch::single("x", Command::SetVibrato { track: t.clone(), note_ids: ids, vibrato: Some(silly) })).is_err()
    );
    assert_eq!(s.undo_depth(), 2);
}
