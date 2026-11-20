use super::*;
use serde_json::json;
use verdant_backend::{Capabilities, Route, plan};
use verdant_domain::editing::Interpolation;
use verdant_domain::model::{ExpressionPoint, TempoPoint};

fn capabilities() -> Capabilities {
    serde_json::from_value(json!({"protocol":"ogd-worker/1", "test_double":true,
        "routes":["verdant-voice"], "max_phrase_seconds":30,
        "native_score":{"inventory":["ma"], "sub_syllable_timing":true,
            "phoneme_duration_control":true, "continuous_f0_control":true,
            "explicit_f0_hz_range":[35,2000],
            "expression":{"methods":["vocaloid-controls-v1"],
                "parameters":["BRE","BRI","CLE","GEN","OPE","GWL"],
                "routes":["verdant-voice"], "maximum_compiled_curve_points":32768}}})).unwrap()
}

fn project() -> Project {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    project.tempo_map.push(TempoPoint { tick:1440, micro_bpm:60_000_000, unknown:Default::default() });
    let mut note = Note::new(960, 960, 60);
    note.pronunciation = Pronunciation { pinyin:"ma1".into(), source:PronunciationSource::Manual, ..Default::default() };
    project.tracks[0].notes.push(note);
    project.tracks[0].refresh_index();
    project
}

fn wire_value(points: &[[f64;2]], seconds: f64) -> f64 {
    let right = points.partition_point(|point| point[0] <= seconds);
    if right == 0 { return points[0][1]; }
    if right == points.len() { return points[right-1][1]; }
    let (a,b) = (points[right-1],points[right]);
    a[1] + (b[1]-a[1])*(seconds-a[0])/(b[0]-a[0])
}

fn replace(store: &mut ProjectStore, note: Note) {
    let track = store.project().tracks[0].id.clone();
    store.apply(store.revision(), CommandBatch::single("区域曲线", Command::ReplaceNotes { track, notes:vec![note] })).unwrap();
}

#[test]
fn region_expression_edits_compile_at_native_frames_and_bypass_restores_defaults() {
    let caps = capabilities();
    for mode in [Interpolation::Linear, Interpolation::Hold, Interpolation::Cosine, Interpolation::Cubic] {
        let mut store = ProjectStore::new(project());
        let track = store.project().tracks[0].id.clone();
        let id = store.project().tracks[0].notes[0].id.clone();
        let mut commands = vec![Command::SetGainCurve { track:track.clone(), note_id:id.clone(),
            points:vec![GainPoint { id:"gain-a".into(), tick:0, millidecibels:-12_000, unknown:Default::default() },
                GainPoint { id:"gain-b".into(), tick:960, millidecibels:0, unknown:Default::default() }] }];
        for param in verdant_backend::params::ALL.into_iter().filter(|param| param.is_expression() && *param != Param::Portamento) {
            commands.push(Command::SetExpressionCurve { track:track.clone(), note_id:id.clone(), parameter:param.short().into(),
                points:vec![ExpressionPoint { id:format!("{}-a",param.short()).as_str().into(), tick:0, value:12, unknown:Default::default() },
                    ExpressionPoint { id:format!("{}-b",param.short()).as_str().into(), tick:960, value:108, unknown:Default::default() }] });
        }
        store.apply(store.revision(), CommandBatch { label:"绘制区域曲线".into(), commands }).unwrap();
        let mut note = store.project().tracks[0].notes[0].clone();
        for param in verdant_backend::params::ALL.into_iter().filter(|param| region_parameter(*param)) {
            note.curves.set_interpolation(param.short(), mode);
        }
        replace(&mut store, note);
        for route in [Route::VerdantVoice] {
            let planned = plan(store.project(), &track, route, &caps);
            assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
            let phrase = &planned.phrases[0];
            let segment = &phrase.expression.as_ref().unwrap().segments[0];
            let note = &store.project().tracks[0].notes[0];
            let tempo = TempoMap::new(&store.project().tempo_map, store.project().ppq).unwrap();
            let origin = tempo.seconds_at(note.start_tick as f64);
            let envelope = verdant_backend::post::dyn_envelope(&[note], &tempo, origin, 16000, 12000).unwrap();
            // These are real 5 ms frames, spanning the unmarked tempo change.
            for (frame,tick) in [(0,0), (25,240), (50,480), (75,600), (100,720), (125,840)] {
                let t = tick as f64/960.;
                let fraction = match mode { Interpolation::Hold => 0., Interpolation::Cosine => (1.-(std::f64::consts::PI*t).cos())/2., _ => t };
                let seconds = frame as f64*0.005;
                assert!((tempo.seconds_at((note.start_tick+tick) as f64)-origin-seconds).abs() < 1e-12);
                for points in segment.curves.values() {
                    assert!((wire_value(points,seconds)-(12.+96.*fraction)).abs() < 1e-9, "{route:?}/{mode:?}/frame{frame}");
                }
                let expected = 10f64.powf((-12.+12.*fraction)/20.) as f32;
                assert_eq!(envelope[frame*80].to_bits(), expected.to_bits(), "DYN {mode:?}/frame{frame}");
            }
        }
        let authored = store.project().tracks[0].notes[0].clone();
        let mut bypassed = authored.clone();
        for param in verdant_backend::params::ALL.into_iter().filter(|param| region_parameter(*param)) {
            set_region_lane_enabled(&mut bypassed, param, false).unwrap();
        }
        replace(&mut store, bypassed);
        let planned = plan(store.project(), &track, Route::VerdantVoice, &caps);
        assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
        let segment = &planned.phrases[0].expression.as_ref().unwrap().segments[0];
        for (key,points) in &segment.curves {
            let default = verdant_domain::model::expression_default(key).unwrap() as f64;
            assert!(points.iter().all(|point| point[1].to_bits() == default.to_bits()), "{key} must compile to its exact neutral value");
        }
        let note = &store.project().tracks[0].notes[0];
        let tempo = TempoMap::new(&store.project().tempo_map, store.project().ppq).unwrap();
        let envelope = verdant_backend::post::dyn_envelope(&[note], &tempo, 0.5, 16000, 12000).unwrap();
        assert!(envelope.iter().all(|gain| gain.to_bits() == 1f32.to_bits()));
        let encoded = serde_json::to_string(note).unwrap();
        let mut restored: Note = serde_json::from_str(&encoded).unwrap();
        for param in verdant_backend::params::ALL.into_iter().filter(|param| region_parameter(*param)) {
            set_region_lane_enabled(&mut restored, param, true).unwrap();
        }
        assert_eq!(restored, authored, "enable must restore point IDs, metadata, interpolation and values");
        store.undo().unwrap();
        assert_eq!(&store.project().tracks[0].notes[0], &authored, "one undo restores the entire region");
    }
}

#[test]
fn voice_keeps_all_ten_parameter_lanes_effective() {
    let caps = capabilities();
    for parameter in verdant_backend::params::ALL {
        assert!(verdant_backend::params::support(parameter, Route::VerdantVoice, Some(&caps)).is_effective(),
            "{} disappeared from verdant-voice authoring", parameter.short());
    }
}

#[test]
fn region_lane_point_move_and_delete_change_the_rendered_curve() {
    let mut store = ProjectStore::new(project());
    let mut note = store.project().tracks[0].notes[0].clone();
    note.curves.expression_points.insert("BRE".into(),vec![
        ExpressionPoint { id:"bre-a".into(), tick:0, value:0, unknown:Default::default() },
        ExpressionPoint { id:"bre-b".into(), tick:960, value:96, unknown:Default::default() }]);
    replace(&mut store,note.clone());
    let track = store.project().tracks[0].id.clone();
    let before = plan(store.project(),&track,Route::VerdantVoice,&capabilities());
    assert!(!before.has_errors(),"{:?}",before.diagnostics);
    assert_eq!(wire_value(&before.phrases[0].expression.as_ref().unwrap().segments[0].curves["BRE"],0.25),48.);
    // The ordinary point-drag command moves the second point from the end to
    // the tempo boundary, keeping its identity and changing the audible ramp.
    note.curves.expression_points.get_mut("BRE").unwrap()[1].tick = 480;
    store.apply(store.revision(),CommandBatch::single("移动曲线控制点",Command::SetExpressionCurve {
        track:track.clone(),note_id:note.id.clone(),parameter:"BRE".into(),points:note.curves.expression("BRE").to_vec() })).unwrap();
    let moved = plan(store.project(),&track,Route::VerdantVoice,&capabilities());
    assert!(!moved.has_errors(),"{:?}",moved.diagnostics);
    assert_eq!(wire_value(&moved.phrases[0].expression.as_ref().unwrap().segments[0].curves["BRE"],0.25),96.);
    let selected = [(note.id.clone(),"bre-b".into())].into_iter().collect();
    delete_region_points(&mut note,Param::Breathiness,&selected);
    assert_eq!(note.curves.expression("BRE").len(),1);
    assert_eq!(note.curves.expression("BRE")[0].id.as_str(),"bre-a");
    replace(&mut store,note.clone());
    let deleted = plan(store.project(),&track,Route::VerdantVoice,&capabilities());
    assert!(!deleted.has_errors(),"{:?}",deleted.diagnostics);
    assert_eq!(wire_value(&deleted.phrases[0].expression.as_ref().unwrap().segments[0].curves["BRE"],0.25),0.);
    delete_region_points(&mut note,Param::Breathiness,&Default::default());
    replace(&mut store,note);
    let cleared = plan(store.project(),&track,Route::VerdantVoice,&capabilities());
    assert!(!cleared.has_errors(),"{:?}",cleared.diagnostics);
    assert!(cleared.phrases[0].expression.is_none(),"clearing the region restores the original absent-expression path");
}

#[test]
fn region_lane_bypass_is_independent_and_preserves_stretched_geometry() {
    let mut note = Note::new(0,960,60);
    note.curves.expression_points.insert("BRE".into(), vec![ExpressionPoint {
        id:"bre-point".into(), tick:480, value:72, unknown:Default::default() }]);
    note.curves.expression_points.insert("GEN".into(), vec![ExpressionPoint {
        id:"gen-point".into(), tick:480, value:96, unknown:Default::default() }]);
    let gender = note.curves.expression("GEN").to_vec();
    set_region_lane_enabled(&mut note, Param::Breathiness, false).unwrap();
    let disabled = note.clone();
    set_region_lane_enabled(&mut note, Param::Breathiness, false).unwrap();
    assert_eq!(note,disabled,"repeated bypass cannot overwrite the retained value");
    note.duration_tick = 1920;
    verdant_domain::command::scale_curve_ticks(&mut note,960);
    set_region_lane_enabled(&mut note, Param::Breathiness, true).unwrap();
    assert_eq!(note.curves.expression("BRE")[0].tick,960);
    assert_eq!(note.curves.expression("BRE")[0].value,72);
    assert_eq!(note.curves.expression("BRE")[0].id.as_str(),"bre-point");
    assert_eq!(note.curves.expression("GEN")[0].value,gender[0].value);
    assert!(!lane_disabled(&note,Param::Gender));
}

#[test]
fn region_lane_rejects_corrupt_retained_values_without_mutation() {
    let mut note = Note::new(0,960,60);
    note.curves.gain_millidecibel_points.push(GainPoint {
        id:"gain-point".into(), tick:0, millidecibels:-6000, unknown:Default::default() });
    set_region_lane_enabled(&mut note,Param::Dyn,false).unwrap();
    note.curves.gain_millidecibel_points[0].unknown.insert(AUTHORED_VALUE.into(),json!("invalid"));
    let before = note.clone();
    assert!(set_region_lane_enabled(&mut note,Param::Dyn,true).is_err());
    assert_eq!(note,before);
}

#[test]
fn roll_pitch_pen_starts_in_a_rest_crosses_notes_and_undoes_as_one_batch() {
    let mut original = project();
    original.tracks[0].notes.push(Note::new(2400, 480, 62));
    original.tracks[0].notes.push(Note::new(4000, 480, 67));
    original.tracks[0].refresh_index();
    let mut store = ProjectStore::new(original.clone());
    let tempo = TempoMap::new(&original.tempo_map, original.ppq).unwrap();
    // Neither endpoint hits a note or an existing control point. The preview
    // is an absolute horizontal line across two different note base pitches.
    let samples = [(500.25, 61.5), (3200.75, 61.5)];
    let batch = crate::curve_span::pitch_line_batch(&original.tracks[0], &tempo, &samples, false).unwrap();
    assert_eq!(batch.commands.len(), 2);
    store.apply(store.revision(), batch).unwrap();
    let notes = &store.project().tracks[0].notes;
    for (note, expected_cents) in [(&notes[0], 100.), (&notes[1], -100.)] {
        for tick in [0., note.duration_tick as f64 / 2., note.duration_tick as f64] {
            let cents = Param::Pit.interpolate_note(note, tick);
            assert_eq!(cents, expected_cents);
            assert_eq!(note.midi_note as f64 + 0.5 + cents / 100., samples[0].1,
                "the committed curve must meet the visible absolute pitch preview");
        }
    }
    assert!(notes[2].curves.pitch_millicent_points.is_empty());
    store.undo().unwrap();
    assert_eq!(store.project(), &original, "one undo restores every crossed note");
}

#[test]
fn roll_pitch_pen_backtracking_replaces_the_latest_drawn_segment() {
    let original = project();
    let tempo = TempoMap::new(&original.tempo_map, original.ppq).unwrap();
    let samples = [(1000.4, 60.5), (1800.6, 62.5), (1000.4, 61.5)];
    let batch = crate::curve_span::pitch_line_batch(&original.tracks[0], &tempo, &samples, false).unwrap();
    let mut store = ProjectStore::new(original);
    store.apply(store.revision(), batch).unwrap();
    let note = &store.project().tracks[0].notes[0];
    assert_eq!(Param::Pit.interpolate_note(note, 40.), 100.);
    assert!(note.curves.pitch_millicent_points.windows(2).all(|pair| pair[0].tick < pair[1].tick));
}
