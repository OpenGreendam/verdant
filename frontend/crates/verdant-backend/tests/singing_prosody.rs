use verdant_backend::adapter::payload_for;
use verdant_backend::f0::{automatic_transition, cents_to_hz, control_track, note_cents_at_with_next, validate};
use verdant_backend::{Capabilities, Route, plan};
use verdant_domain::model::{PitchPoint, ExpressionPoint};
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource, TempoMap, Vibrato};

fn note(start: i64, duration: i64, midi: i32) -> Note {
    let mut n = Note::new(start, duration, midi);
    n.pronunciation = Pronunciation { pinyin: "a".into(), source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(), ..Default::default() };
    n
}

#[test]
fn default_curve_matches_visible_pitch_on_both_sides_and_preserves_short_note() {
    let tempo = TempoMap::constant(120.0);
    let a = note(0, 960, 60);
    let b = note(960, 96, 67); // Only 50 ms; retain at least 60% target plateau.
    let c = note(1056, 960, 64);
    let notes = [&a, &b, &c];
    let track = control_track(&notes, &tempo, 0.0, 1.05);
    validate(&track).unwrap();
    let window = automatic_transition(&a, &b, &tempo).unwrap();
    assert!((window.0 - 0.46).abs() < 1e-12 && (window.1 - 0.51).abs() < 1e-12);
    for (&t, &f) in track.frame_times_seconds.iter().zip(&track.f0_hz) {
        if t >= 1.05 { continue; }
        let (n, previous, next) = if t < 0.5 { (&a, None, Some(&b)) }
            else if t < 0.55 { (&b, Some(&a), Some(&c)) } else { (&c, Some(&b), None) };
        let visible = note_cents_at_with_next(n, previous, next, &tempo, t).unwrap();
        assert!((cents_to_hz(visible) - f).abs() <= 0.000501, "visible and sent curves differ at {t}");
        if (0.51..=0.54).contains(&t) { assert!((f - cents_to_hz(6700.0)).abs() <= 0.000501); }
        assert!(f >= cents_to_hz(6000.0) - 0.000501 && f <= cents_to_hz(6700.0) + 0.000501);
    }
}

#[test]
fn automatic_curve_respects_rests_zero_manual_pit_and_vibrato() {
    let tempo = TempoMap::constant(120.0);
    let a = note(0, 960, 60);
    let mut b = note(960, 960, 64);
    assert!(automatic_transition(&a, &b, &tempo).is_some());
    b.portamento_ms = Some(0);
    assert!(automatic_transition(&a, &b, &tempo).is_none());
    let stored = serde_json::to_string(&b).unwrap();
    assert_eq!(serde_json::from_str::<Note>(&stored).unwrap().portamento_ms, Some(0));
    b.portamento_ms = None;
    let stored = serde_json::to_string(&b).unwrap();
    assert_eq!(serde_json::from_str::<Note>(&stored).unwrap().portamento_ms, None);
    b.curves.pitch_millicent_points.push(PitchPoint { id: "authored".into(), tick: 0, millicents: 25_000, unknown: Default::default() });
    assert!(automatic_transition(&a, &b, &tempo).is_none());
    b.curves.pitch_millicent_points.clear();
    b.vibrato = Some(Vibrato::DEFAULT);
    assert!(automatic_transition(&a, &b, &tempo).is_none());
    b.vibrato = None;
    b.start_tick += 1;
    assert!(automatic_transition(&a, &b, &tempo).is_none());
}

#[test]
fn capability_enabled_voice_sends_automatic_control() {
    let caps: Capabilities = serde_json::from_value(serde_json::json!({
        "engine":"control fixture", "protocol":"ogd-worker/1", "native_version":"fixture",
        "routes":["verdant-voice"], "test_double":true, "max_phrase_seconds":30.0,
        "native_score":{"inventory":[], "english_vowel_proxies":["a"], "continuous_f0_control":true},
        
        "files":{"token":{"available":true,"sha256":"11".repeat(32)},
                 "model":{"available":true,"sha256":"22".repeat(32)},
                 "vocoder":{"available":true,"sha256":"33".repeat(32)}}
    })).unwrap();
    let mut project = Project::new_empty();
    project.tracks[0].notes = vec![note(0,960,60), note(960,960,64)];
    let before = project.clone();
    let c0 = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let c1 = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(!c0.has_errors() && !c1.has_errors());
    let p0 = payload_for(&c0.phrases[0], Route::VerdantVoice, 10008);
    let p1 = payload_for(&c1.phrases[0], Route::VerdantVoice, 10008);
    assert!(p0.control_track.is_some());
    assert_eq!(p0.control_track, p1.control_track);
    assert_eq!(project, before, "automatic connections do not write explicit values back into notes");
}

#[test]
fn native_curve_range_checks_authored_peaks_and_exact_caps_with_note_location() {
    let mut caps: Capabilities = serde_json::from_value(serde_json::json!({
        "engine":"control fixture", "protocol":"ogd-worker/1", "native_version":"fixture",
        "routes":["verdant-voice"], "test_double":true, "max_phrase_seconds":30.0,
        "native_score":{"inventory":[], "english_vowel_proxies":["a"], "continuous_f0_control":true,
                        "explicit_f0_hz_range":[35.0,1000.0]},
        
        "files":{"token":{"available":true,"sha256":"11".repeat(32)},
                 "model":{"available":true,"sha256":"22".repeat(32)},
                 "vocoder":{"available":true,"sha256":"33".repeat(32)}}
    })).unwrap();
    let mut project = Project::new_empty();
    let lead = note(2880, 960, 60); // Nonzero phrase origin and a second-note error.
    let mut high = note(3840, 1920, 83); // B5 base < 1000 Hz; +100 cents exceeds it.
    high.lyric = "高".into();
    high.curves.pitch_millicent_points.push(PitchPoint { id:"pit".into(), tick:0,
        millicents:100_000, unknown:Default::default() });
    project.tracks[0].notes = vec![lead.clone(), high.clone()];
    for route in [Route::VerdantVoice] {
        let planned = plan(&project, &project.tracks[0].id, route, &caps);
        assert!(planned.phrases.is_empty() && planned.blocked.len() == 1);
        let error = planned.diagnostics.iter().find(|d| d.code == "E_CONTROL_PITCH_RANGE").unwrap();
        assert_eq!(error.note_id.as_ref(), Some(&high.id));
        assert!(error.message.contains("2.000s") && error.detail.contains("note_start_tick=3840"));
        assert!(!planned.diagnostics.iter().any(|d| d.code == "E_PITCH_RANGE"), "base pitch itself is valid");
    }
    high.curves.pitch_millicent_points.clear();
    high.vibrato = Some(Vibrato { depth_cents:100, delay_permille:0, ..Vibrato::DEFAULT });
    project.tracks[0].notes = vec![lead, high.clone()];
    assert!(plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps).diagnostics.iter().any(|d|
        d.code == "E_CONTROL_PITCH_RANGE" && d.note_id.as_ref() == Some(&high.id)));

    // Follow the declared capability, include its exact endpoint, and reject
    // a transmitted 0.001 Hz step above it instead of clipping the curve.
    caps.native_score.as_mut().unwrap().explicit_f0_hz_range = Some([35.0, 880.0]);
    let mut edge = note(0, 960, 69);
    edge.curves.pitch_millicent_points.push(PitchPoint { id:"edge".into(), tick:0,
        millicents:1_200_000, unknown:Default::default() });
    project.tracks[0].notes = vec![edge.clone()];
    assert!(!plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps).has_errors());
    project.tracks[0].notes[0].curves.pitch_millicent_points[0].millicents += 1;
    assert!(plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps).has_errors());
    caps.native_score.as_mut().unwrap().explicit_f0_hz_range = Some([35.0, 1000.0]);
    assert!(!plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps).has_errors());

    let mut low = note(0, 960, 33); // A1 / 2 = 27.5 Hz, below native minimum.
    low.curves.pitch_millicent_points.push(PitchPoint { id:"low".into(), tick:0,
        millicents:-1_200_000, unknown:Default::default() });
    project.tracks[0].notes = vec![low];
    assert!(plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps).diagnostics.iter().any(|d| d.code == "E_CONTROL_PITCH_RANGE"));

    // The generic control API covers the ONNX host range. Native admission
    // still applies each route's declared capability before sending a curve.
    let broad = note(0, 960, 93);
    let curve = control_track(&[&broad], &TempoMap::constant(120.0), 0.0, 0.5);
    validate(&curve).unwrap();
    assert_eq!(curve.f0_hz[0], 1760.0);
    let c8 = note(0, 960, 108);
    let mut curve = control_track(&[&c8], &TempoMap::constant(120.0), 0.0, 0.5);
    assert!(curve.f0_hz[0] > 4000.0 && curve.f0_hz[0] < 5000.0);
    validate(&curve).unwrap();
    project.tracks[0].notes = vec![c8.clone()];
    for route in [Route::VerdantVoice] {
        let planned = plan(&project, &project.tracks[0].id, route, &caps);
        assert!(planned.phrases.is_empty() && planned.blocked.len() == 1);
        assert!(planned.diagnostics.iter().any(|d|
            d.code == "E_PITCH_RANGE" && d.note_id.as_ref() == Some(&c8.id)));
    }
    // The shared structural endpoint is inclusive; the next millihertz fails.
    curve.f0_hz[0] = 5000.0;
    validate(&curve).unwrap();
    curve.f0_hz[0] = 5000.001;
    assert!(validate(&curve).is_err());
}

fn por(note: &mut Note, value: u8) {
    note.curves.expression_points.insert("POR".into(), vec![ExpressionPoint {
        id:"por".into(),tick:0,value,unknown:Default::default() }]);
}

#[test]
fn por_moves_the_glide_window_without_changing_its_width_or_neutral_output() {
    let tempo=TempoMap::constant(120.0);
    let a=note(0,960,60);
    let mut b=note(960,960,67);
    let neutral=control_track(&[&a,&b],&tempo,0.,1.);
    por(&mut b,64);
    assert_eq!(neutral,control_track(&[&a,&b],&tempo,0.,1.));
    for (value,start,end) in [(0,0.42,0.5),(64,0.46,0.54),(127,0.5,0.58)] {
        por(&mut b,value);
        let window=automatic_transition(&a,&b,&tempo).unwrap();
        assert!((window.0-start).abs()<1e-12 && (window.1-end).abs()<1e-12);
        assert!((window.1-window.0-0.08).abs()<1e-12);
        let at_boundary=note_cents_at_with_next(&b,Some(&a),None,&tempo,0.5).unwrap();
        let expected=match value {0=>6700.,64=>6350.,_=>6000.};
        assert!((at_boundary-expected).abs()<1e-8);
        let track=control_track(&[&a,&b],&tempo,0.,1.);
        for pair in track.f0_hz[..track.f0_hz.len()-1].windows(2) { assert!(pair[1]>=pair[0]); }
    }
}

#[test]
fn por_uses_destination_note_on_value_and_bounds_short_notes_without_reverting_pitch() {
    let tempo=TempoMap::constant(120.0);
    let a=note(0,960,67);
    let mut b=note(960,96,60);
    por(&mut b,0);
    b.curves.expression_points.get_mut("POR").unwrap().push(ExpressionPoint {
        id:"later".into(),tick:48,value:127,unknown:Default::default() });
    let window=automatic_transition(&a,&b,&tempo).unwrap();
    assert!(window.1<0.5);
    let prior=note_cents_at_with_next(&a,None,Some(&b),&tempo,0.499).unwrap();
    let current=note_cents_at_with_next(&b,Some(&a),None,&tempo,0.501).unwrap();
    assert_eq!(prior,6000.);
    assert_eq!(current,6000.);
    por(&mut b,127);
    let (start,end)=automatic_transition(&a,&b,&tempo).unwrap();
    assert!(start>=0.0 && end<=0.55 && (end-start-0.05).abs()<1e-12);
}

#[test]
fn por_positions_explicit_millisecond_glides_but_zero_still_disables_them() {
    let tempo=TempoMap::constant(120.0);
    let a=note(0,960,60);
    let mut b=note(960,960,64);
    b.portamento_ms=Some(100);
    let old=control_track(&[&a,&b],&tempo,0.,1.);
    por(&mut b,64);
    assert_eq!(old,control_track(&[&a,&b],&tempo,0.,1.));
    por(&mut b,0);
    assert!((note_cents_at_with_next(&b,Some(&a),None,&tempo,0.5).unwrap()-6200.).abs()<1e-8);
    assert!(note_cents_at_with_next(&a,None,Some(&b),&tempo,0.475).unwrap()>6000.);
    por(&mut b,127);
    assert_eq!(note_cents_at_with_next(&b,Some(&a),None,&tempo,0.525).unwrap(),6000.);
    assert!((note_cents_at_with_next(&b,Some(&a),None,&tempo,0.6).unwrap()-6200.).abs()<1e-8);
    b.portamento_ms=Some(0);
    assert_eq!(note_cents_at_with_next(&b,Some(&a),None,&tempo,0.5).unwrap(),6400.);
}
