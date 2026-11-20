use super::*;
use verdant_domain::{Note, Project, ProjectStore};
use verdant_domain::model::TempoPoint;

fn fixture() -> (ProjectStore, TempoMap) {
    let mut project = Project::new_empty();
    project.ppq = 1000;
    project.tempo_map = vec![
        TempoPoint { tick: 0, micro_bpm: 60_000_000, unknown: Default::default() },
        TempoPoint { tick: 150, micro_bpm: 120_000_000, unknown: Default::default() },
    ];
    project.tracks[0].notes = vec![Note::new(100, 50, 69), Note::new(150, 50, 72)];
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    (ProjectStore::new(project), tempo)
}

#[test]
fn drawn_pitch_stroke_samples_project_ticks_undoes_once_and_reaches_control_frames() {
    let (mut store, tempo) = fixture();
    let before = store.snapshot();
    let samples = [(110., 0.), (130., 100.), (170., -100.), (190., 0.)];
    let batch = pitch_stroke_batch(&store.project().tracks[0], &tempo, &samples, false, false).unwrap();
    assert_eq!(batch.commands.len(), 2, "one release must include both crossed notes");
    assert_eq!(store.undo_depth(), 0, "building the preview must not edit the document");
    store.apply(store.revision(), batch).unwrap().unwrap();
    assert_eq!(store.undo_depth(), 1);
    let notes = &store.project().tracks[0].notes;
    let point = |note: usize, tick| notes[note].curves.pitch_millicent_points.iter().find(|p| p.tick == tick).unwrap().millicents;
    assert_eq!(point(0, 10), 0);
    assert_eq!(point(0, 15), 25_000, "5 ms at 60 BPM / 1000 PPQ is five ticks");
    assert_eq!(point(0, 30), 100_000);
    assert_eq!(point(1, 10), -50_000, "5 ms after the tempo change is ten ticks");
    assert_eq!(point(1, 20), -100_000);
    assert_eq!(point(1, 40), 0);

    let refs: Vec<_> = notes.iter().collect();
    let control = verdant_backend::f0::control_track(&refs, &tempo, 0.1, 0.075);
    for (time, midi, cents) in [(0.01, 69., 0.), (0.02, 69., 50.), (0.03, 69., 100.),
        (0.04, 69., 50.), (0.05, 72., 0.), (0.055, 72., -50.), (0.06, 72., -100.), (0.07, 72., 0.)] {
        let frame = control.frame_times_seconds.iter().position(|t| (*t-time).abs() < 1e-12).unwrap();
        let expected = (verdant_backend::f0::cents_to_hz(midi*100.+cents)*1000.).round()/1000.;
        assert!(!control.uv[frame]);
        assert!((control.f0_hz[frame]-expected).abs() < 0.0011, "wrong pitch at frame {time}");
    }
    assert!(*control.uv.last().unwrap(), "the final control boundary stays unvoiced");
    store.undo().unwrap().unwrap();
    assert_eq!(store.undo_depth(), 0);
    assert_eq!(store.project().tracks, before.tracks);
    assert!(store.undo().unwrap().is_none(), "the entire stroke must be one undo step");
    store.redo().unwrap().unwrap();
    assert_eq!(store.undo_depth(), 1);
    assert_eq!(store.project().tracks[0].notes[1].curves.pitch_millicent_points.iter().find(|p| p.tick == 20).unwrap().millicents, -100_000);
}

#[test]
fn pitch_backtracking_overwrites_old_vertices_and_fractional_endpoints_survive() {
    let mut track = Track::new("测试", "teal");
    track.notes.push(Note::new(0, 200, 69));
    track.refresh_index();
    let tempo = TempoMap::constant(120.);
    let batch = pitch_stroke_batch(&track, &tempo, &[(20., 0.), (33., 100.), (80., 0.), (20., -100.)], false, false).unwrap();
    let Command::SetPitchCurve { points, .. } = &batch.commands[0] else { panic!("expected pitch curve"); };
    assert_eq!(points.iter().find(|p| p.tick == 33).unwrap().millicents, -78_333);
    assert!(points.windows(2).all(|p| p[0].tick < p[1].tick));
    let batch = pitch_stroke_batch(&track, &tempo, &[(20.4, 12.), (33.4, 90.)], false, false).unwrap();
    let Command::SetPitchCurve { points, .. } = &batch.commands[0] else { panic!("expected pitch curve"); };
    assert_eq!(points.iter().find(|p| p.tick == 20).unwrap().millicents, 12_000);
    assert_eq!(points.iter().find(|p| p.tick == 33).unwrap().millicents, 90_000);
}

#[test]
fn pitch_smoothing_is_optional_and_erase_preserves_outside_values() {
    let mut project = Project::new_empty();
    project.tracks[0].notes.push(Note::new(0, 200, 69));
    let mut store = ProjectStore::new(project);
    let tempo = TempoMap::constant(120.);
    let samples = [(20., 0.), (40., 0.), (60., 160.), (80., 0.), (100., 0.)];
    for (smooth, expected) in [(false, 160_000), (true, 60_000)] {
        let batch = pitch_stroke_batch(&store.project().tracks[0], &tempo, &samples, smooth, false).unwrap();
        let Command::SetPitchCurve { points, .. } = &batch.commands[0] else { panic!("expected pitch curve"); };
        assert_eq!(points.iter().find(|p| p.tick == 60).unwrap().millicents, expected);
    }
    let batch = pitch_stroke_batch(&store.project().tracks[0], &tempo, &samples, false, false).unwrap();
    store.apply(store.revision(), batch).unwrap();
    let original = store.project().tracks[0].notes[0].clone();
    let batch = pitch_stroke_batch(&store.project().tracks[0], &tempo, &[(55., 100.), (65., 100.)], false, true).unwrap();
    store.apply(store.revision(), batch).unwrap();
    let edited = &store.project().tracks[0].notes[0];
    assert_eq!(verdant_backend::params::Param::Pit.interpolate_note(edited, 60.), 0.);
    for tick in [20., 40., 54., 66., 80., 100.] {
        assert!((verdant_backend::params::Param::Pit.interpolate_note(edited, tick)
            -verdant_backend::params::Param::Pit.interpolate_note(&original, tick)).abs() < 0.0011);
    }
}

#[test]
fn drawing_on_nonlinear_pitch_bakes_old_shape_and_renders_the_drawn_line() {
    let (mut store, tempo) = fixture();
    let mut note = store.project().tracks[0].notes[0].clone();
    note.curves.pitch_millicent_points = vec![
        PitchPoint { id: PointId::generate(), tick: 0, millicents: 0, unknown: Default::default() },
        PitchPoint { id: PointId::generate(), tick: 50, millicents: 100_000, unknown: Default::default() },
    ];
    note.curves.set_interpolation("PIT", Interpolation::Cosine);
    let track_id = store.project().tracks[0].id.clone();
    store.apply(store.revision(), CommandBatch::single("fixture", Command::ReplaceNotes { track: track_id, notes: vec![note.clone()] })).unwrap();
    let batch = pitch_stroke_batch(&store.project().tracks[0], &tempo, &[(120., -100.), (130., 100.)], false, false).unwrap();
    store.apply(store.revision(), batch).unwrap();
    let edited = &store.project().tracks[0].notes[0];
    assert_eq!(edited.curves.interpolation("PIT"), Interpolation::Linear);
    let pit = verdant_backend::params::Param::Pit;
    assert_eq!(pit.interpolate_note(edited, 20.), -100.);
    assert_eq!(pit.interpolate_note(edited, 25.), 0.);
    assert_eq!(pit.interpolate_note(edited, 30.), 100.);
    for tick in [0., 5., 10., 15., 35., 40., 45., 50.] {
        assert!((pit.interpolate_note(edited, tick)-pit.interpolate_note(&note, tick)).abs() < 0.0011);
    }
}
