use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::params::{self, Param};
use verdant_backend::{Capabilities, Route, plan};
use verdant_domain::model::{ExpressionPoint, TempoPoint};
use verdant_domain::{Command, CommandBatch, Note, OutputTransientsSettings, PhonemeTimingSettings, Project,
    ProjectStore, Pronunciation, PronunciationSource, TempoMap, VocalTractSettings};

const VOICE_KEYS:[&str;6]=["BRE","BRI","CLE","GEN","OPE","GWL"];

fn caps() -> Capabilities {
    serde_json::from_value(json!({"protocol":"ogd-worker/1","test_double":true,
        "routes":["verdant-voice"],"max_phrase_seconds":30,
        "native_score":{"inventory":["ma"],"sub_syllable_timing":true,
            "phoneme_duration_control":true,"continuous_f0_control":true,"explicit_f0_hz_range":[35,2000],
            "vocal_tract":{"methods":["bilinear-v1"],"routes":["verdant-voice"],"ratio_range":[0.8,1.25]},
            "phoneme_timing":{"methods":["vowel-anchor-v1"],"routes":["verdant-voice"],"timing_resolution_seconds":0.005,
                "continuity":{"method":"continuous-runs-v1","max_seconds":120,"max_segments":2048,
                    "max_control_points":32768,"minimum_silence_seconds":0.005}},
            "expression":{"methods":["vocaloid-controls-v1"],"parameters":VOICE_KEYS,"routes":["verdant-voice"],
                "maximum_compiled_curve_points":32768},
            "output_transients":{"methods":["phoneme-output-envelope-v1"],"routes":["verdant-voice"],
                "minimum_strength":0,"maximum_strength":1,"minimum_attenuation_db":0,"maximum_attenuation_db":12}},
        })).unwrap()
}

fn project(count:usize) -> Project {
    let mut p=Project::new_empty();p.tempo_map[0].micro_bpm=120_000_000;
    for i in 0..count {
        let mut n=Note::new(960+i as i64*960,960,60+(i%5) as i32);
        n.id=format!("n{i}").as_str().into();
        n.pronunciation=Pronunciation{pinyin:"ma1".into(),source:PronunciationSource::Manual,..Default::default()};
        p.tracks[0].notes.push(n);
    }
    p.tracks[0].refresh_index();
    p.render_preferences.phoneme_timing=Some(PhonemeTimingSettings::vowel_anchor_v1());
    p.render_preferences.vocal_tract=Some(VocalTractSettings::bilinear_v1(1.125));
    p
}

fn put(note:&mut Note,key:&str,value:u8) {
    note.curves.expression_points.insert(key.into(),vec![ExpressionPoint{
        id:format!("{}-{key}",note.id).as_str().into(),tick:0,value,unknown:Default::default()}]);
}

fn put_curve(note: &mut Note, key: &str, points: &[(i64, u8)]) {
    note.curves.expression_points.insert(key.into(), points.iter().enumerate().map(|(index, &(tick, value))|
        ExpressionPoint { id: format!("{}-{key}-{index}", note.id).as_str().into(), tick, value, unknown: Default::default() }
    ).collect());
}

// Independent sampling of the worker's seconds-based linear wire contract.
fn wire_value(points: &[[f64; 2]], elapsed: f64) -> f64 {
    if elapsed <= points[0][0] { return points[0][1]; }
    for pair in points.windows(2) {
        if elapsed <= pair[1][0] {
            let fraction = (elapsed-pair[0][0])/(pair[1][0]-pair[0][0]);
            return pair[0][1]+fraction*(pair[1][1]-pair[0][1]);
        }
    }
    points.last().unwrap()[1]
}

#[test]
fn expression_curves_preserve_tick_interpolation_at_unmarked_tempo_changes() {
    for route in [Route::VerdantVoice] { for anchored in [false, true] {
        let mut p = project(1);
        if !anchored { p.render_preferences.phoneme_timing = None; }
        p.tempo_map.push(TempoPoint { tick: 1440, micro_bpm: 60_000_000, unknown: Default::default() });
        for key in VOICE_KEYS { put_curve(&mut p.tracks[0].notes[0], key, &[(0, 0), (960, 127)]); }
        let before = p.clone();
        let result = plan(&p, &p.tracks[0].id, route, &caps());
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let segment = &result.phrases[0].expression.as_ref().unwrap().segments[0];
        let tempo = TempoMap::new(&p.tempo_map, p.ppq).unwrap();
        for key in VOICE_KEYS {
            let points = &segment.curves[key];
            // No authored vertex is at tick480: 63.5 is a derived value, not
            // rounded to the author's integer controller representation.
            assert_eq!(points, &vec![[0., 0.], [0.25, 63.5], [0.75, 127.]]);
            for tick in [0, 120, 240, 360, 480, 600, 720, 840, 960] {
                let elapsed = tempo.seconds_at((960+tick) as f64)-tempo.seconds_at(960.);
                assert!((wire_value(points, elapsed)-127.*tick as f64/960.).abs() < 1e-10,
                    "{route:?}/{key}/{anchored}/{tick}");
            }
        }
        assert_eq!(p, before, "compilation cannot add derived points to the saved project");
    }}
}

#[test]
fn expression_tempo_knots_preserve_note_onset_prefix_and_trimmed_ownership() {
    let mut p = project(2);
    // The first change lies in the leading consonant; changes at both note
    // boundaries and three unmarked interior changes must keep separate clocks.
    for (tick, bpm) in [(900, 90), (960, 150), (1200, 100), (1440, 200), (1680, 75), (1920, 130)] {
        p.tempo_map.push(TempoPoint { tick, micro_bpm: bpm*1_000_000, unknown: Default::default() });
    }
    for note in &mut p.tracks[0].notes { put_curve(note, "BRI", &[(0, 12), (960, 108)]); }
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(result.phrases.len(), 1);
    let phrase = &result.phrases[0];
    let tempo = TempoMap::new(&p.tempo_map, p.ppq).unwrap();
    let segments = &phrase.expression.as_ref().unwrap().segments;
    let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
    assert!(tempo.seconds_at(900.) > phrase.start_seconds);
    assert!(tempo.seconds_at(900.) < tempo.seconds_at(960.));
    assert!(trace[0].prefix_seconds > 0. && trace[0].tail_trim_seconds > 0.);
    assert!((segments[0].start_seconds).abs() < 1e-10);
    assert!((segments[0].note_onset_seconds-trace[0].prefix_seconds).abs() < 1e-10);
    assert!((segments[0].end_seconds-segments[1].start_seconds).abs() < 1e-10);
    let points = &segments[0].curves["BRI"];
    assert_eq!(points.len(), 5);
    assert_eq!(points.iter().map(|point| point[1]).collect::<Vec<_>>(), vec![12., 36., 60., 84., 108.]);
    assert!(points.windows(2).all(|pair| pair[0][0] < pair[1][0]));
    let prefix_mid = (segments[0].start_seconds+segments[0].note_onset_seconds)/2.;
    assert_eq!(wire_value(points, prefix_mid-segments[0].note_onset_seconds), 12.);
    for tick in [0, 120, 240, 360, 480, 600, 720] {
        let score_seconds = tempo.seconds_at((960+tick) as f64)-phrase.start_seconds;
        assert!(score_seconds < segments[0].end_seconds);
        assert!((wire_value(points, score_seconds-segments[0].note_onset_seconds)-(12.+tick as f64/10.)).abs() < 1e-10);
    }
    // A tempo change exactly at the next note's onset must not duplicate its
    // tick0 vertex or move it to the start of its anticipatory consonant.
    assert_eq!(segments[1].curves["BRI"].len(), 2);
    assert_eq!(segments[1].curves["BRI"][0], [0., 12.]);
    assert!(segments[1].start_seconds < segments[1].note_onset_seconds);
}

#[test]
fn phrase_split_keeps_the_authored_expression_endpoint_after_the_borrowed_tail() {
    let mut p = project(2);
    let first = &mut p.tracks[0].notes[0];
    first.pronunciation.phoneme_durations_permille = Some(vec![100, 900]);
    put_curve(first, "BRI", &[(0, 12), (960, 108)]);
    let before = p.clone();
    let mut c = caps();
    c.max_phrase_seconds = 0.7;
    c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().continuity = None;
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &c);
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let [first, next] = result.phrases.as_slice() else { panic!("both half-second notes cannot fit one request") };
    assert!((first.start_seconds + first.seconds - next.start_seconds).abs() < 1e-10);
    let payload = payload_for(first, Route::VerdantVoice, 10008);
    let trace = &payload.score.as_ref().unwrap().phoneme_timing.as_ref().unwrap().notes[0];
    let expression = &payload.expression.as_ref().unwrap().segments[0];
    assert!((first.seconds - 0.44).abs() < 1e-10);
    assert!((trace.tail_trim_seconds - 0.11).abs() < 1e-10);
    assert!((expression.start_seconds - (trace.note_onset_seconds - trace.prefix_seconds)).abs() < 1e-10);
    assert_eq!(expression.note_onset_seconds, trace.note_onset_seconds);
    assert!((expression.end_seconds - first.seconds).abs() < 1e-10);
    let knots = &expression.curves["BRI"];
    assert_eq!(knots.as_slice(), &[[0.0, 12.0], [0.5, 108.0]]);
    assert!(knots[1][0] > first.seconds);
    assert_eq!(p, before);
}

#[test]
fn expression_tempo_knots_preserve_explicit_vertices_held_ends_and_flat_defaults() {
    let mut p = project(1);
    for (offset, bpm) in [(0, 100), (100, 120), (200, 150), (450, 90), (700, 180), (800, 120), (960, 150)] {
        p.tempo_map.push(TempoPoint { tick: 960+offset, micro_bpm: bpm*1_000_000, unknown: Default::default() });
    }
    put_curve(&mut p.tracks[0].notes[0], "BRI", &[(200, 10), (700, 110)]);
    put_curve(&mut p.tracks[0].notes[0], "GEN", &[(200, 64), (700, 64)]);
    put_curve(&mut p.tracks[0].notes[0], "OPE", &[(450, 127)]);
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let curves = &result.phrases[0].expression.as_ref().unwrap().segments[0].curves;
    let tempo = TempoMap::new(&p.tempo_map, p.ppq).unwrap();
    let elapsed = |tick| tempo.seconds_at((960+tick) as f64)-tempo.seconds_at(960.);
    assert_eq!(curves["BRI"], vec![[elapsed(200), 10.], [elapsed(450), 60.], [elapsed(700), 110.]]);
    assert_eq!(curves["GEN"], vec![[elapsed(200), 64.], [elapsed(700), 64.]]);
    assert_eq!(curves["OPE"], vec![[elapsed(450), 127.]]);
    for tick in [0, 100, 200, 300, 450, 600, 700, 800, 960] {
        let expected = 10.+(tick as f64-200.).clamp(0., 500.)/5.;
        assert!((wire_value(&curves["BRI"], elapsed(tick))-expected).abs() < 1e-10);
    }
}

#[test]
fn expression_transport_budget_allows_tempo_knots_beyond_the_authored_point_budget() {
    let mut p = project(1);
    p.render_preferences.phoneme_timing = None;
    let note = &mut p.tracks[0].notes[0];
    note.duration_tick = 41000;
    let points: Vec<_> = (0..4096).map(|index| (index*10, if index%2 == 0 { 0 } else { 127 })).collect();
    put_curve(note, "BRI", &points);
    p.tracks[0].refresh_index();
    p.tempo_map.push(TempoPoint { tick: 965, micro_bpm: 121_000_000, unknown: Default::default() });
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(p.tracks[0].notes[0].curves.expression("BRI").len(), 4096);
    let points = &result.phrases[0].expression.as_ref().unwrap().segments[0].curves["BRI"];
    assert_eq!(points.len(), 4097);
    assert_eq!(points[1][1], 63.5);
    assert!(points.windows(2).all(|pair| pair[0][0] < pair[1][0]));
    let mut old = caps();
    old.native_score.as_mut().unwrap().expression.as_mut().unwrap().maximum_compiled_curve_points = None;
    let limited = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &old);
    assert!(limited.phrases.is_empty(), "undeclared larger budgets must not be sent to old v1 workers");
    assert!(limited.diagnostics.iter().any(|d| d.code == "E_EXPRESSION_POINT_LIMIT" && d.message.contains("4096")));
}

#[test]
fn expression_expansion_over_the_transport_budget_is_a_named_preflight_error() {
    let mut p = project(1);
    p.render_preferences.phoneme_timing = None;
    p.tracks[0].notes[0].duration_tick = 41000;
    let points: Vec<_> = (0..4096).map(|index| (index*10, if index%2 == 0 { 0 } else { 127 })).collect();
    put_curve(&mut p.tracks[0].notes[0], "BRI", &points);
    p.tracks[0].refresh_index();
    // No vocoder/model work is needed: this is purely the planner's bounded
    // expansion of legal author points plus intervening tempo boundaries.
    for offset in 1..=32769 {
        p.tempo_map.push(TempoPoint { tick: 960+offset, micro_bpm: if offset%2 == 0 { 120_000_000 } else { 121_000_000 },
            unknown: Default::default() });
    }
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps());
    assert!(result.phrases.is_empty());
    assert_eq!(result.blocked.len(), 1);
    let diagnostic = result.diagnostics.iter().find(|d| d.code == "E_EXPRESSION_POINT_LIMIT").unwrap();
    assert_eq!(diagnostic.note_id.as_ref(), Some(&p.tracks[0].notes[0].id));
    assert!(diagnostic.detail.contains("compiled_points_at_least=32769"));
}

#[test]
fn por_editor_exposes_only_the_effective_onset_without_mutating_imported_curves() {
    let mut p = project(2);
    let note = &mut p.tracks[0].notes[1];
    put_curve(note, "POR", &[(240, 32), (720, 127), (960, 0)]);
    note.curves.expression_points.get_mut("POR").unwrap()[0].unknown.insert("future_shape".into(), json!("retain"));
    let before = note.clone();
    let id = before.curves.expression("POR")[0].id.clone();
    assert_eq!(Param::Portamento.samples(note), vec![(0, 32.)]);
    assert_eq!(Param::Portamento.point_samples(note), vec![(id.clone(), 0, 32.)]);
    // Copy/paste and old clipboard data both use the same note-on semantics.
    assert_eq!(Param::Portamento.interpolate(&[(240, 32.), (720, 127.), (960, 0.)], 800.), 32.);
    assert_eq!(Param::Portamento.interpolate(&[], 800.), 64.);
    assert_eq!(*note, before, "displaying/copying an imported curve cannot rewrite it");
    for value in [0., 64., 127.] {
        let points = params::portamento_points_for_value(note, value);
        assert_eq!(points.len(), 1);
        assert_eq!((points[0].tick, points[0].value), (0, value as u8));
        assert_eq!(points[0].id, id);
        assert_eq!(points[0].unknown, before.curves.expression("POR")[0].unknown);
    }
}

#[test]
fn por_note_value_edit_changes_f0_and_cache_identity_and_undo_restores_imported_data() {
    let mut p = project(2);
    put_curve(&mut p.tracks[0].notes[1], "POR", &[(240, 0), (720, 127)]);
    let original = p.clone();
    let c = caps();
    let before = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &c);
    assert!(!before.has_errors());
    let before = payload_for(&before.phrases[0], Route::VerdantVoice, 6789);
    let track = p.tracks[0].id.clone();
    let note_id = p.tracks[0].notes[1].id.clone();
    let mut store = ProjectStore::new(p.clone());
    let points = params::portamento_points_for_value(&p.tracks[0].notes[1], 127.);
    store.apply(store.revision(), CommandBatch::single("POR 音符起点值", Command::SetExpressionCurve {
        track: track.clone(), note_id: note_id.clone(), parameter: "POR".into(), points })).unwrap();
    let changed = plan(store.project(), &track, Route::VerdantVoice, &c);
    assert!(!changed.has_errors());
    let changed = payload_for(&changed.phrases[0], Route::VerdantVoice, 6789);
    assert_ne!(changed.control_track, before.control_track);
    assert_ne!(render_digest(&changed, Route::VerdantVoice, &c), render_digest(&before, Route::VerdantVoice, &c));
    assert!(changed.expression.is_none(), "POR belongs to F0, not final spectral expression");
    let saved = serde_json::to_string(store.project()).unwrap();
    let (loaded, _) = verdant_domain::io::parse_project(&saved).unwrap();
    assert_eq!(loaded.tracks[0].notes[1].curves.expression("POR")[0].tick, 0);
    assert_eq!(loaded.tracks[0].notes[1].curves.expression("POR").len(), 1);
    store.undo().unwrap();
    assert_eq!(store.project().tracks, original.tracks);
    let restored = plan(store.project(), &track, Route::VerdantVoice, &c);
    assert_eq!(payload_for(&restored.phrases[0], Route::VerdantVoice, 6789), before);
    store.redo().unwrap();
    let points = params::portamento_points_for_value(&store.project().tracks[0].notes[1], 64.);
    store.apply(store.revision(), CommandBatch::single("重置 POR", Command::SetExpressionCurve {
        track: track.clone(), note_id, parameter: "POR".into(), points })).unwrap();
    p.tracks[0].notes[1].curves.expression_points.remove("POR");
    let neutral = plan(&p, &track, Route::VerdantVoice, &c);
    let erased = plan(store.project(), &track, Route::VerdantVoice, &c);
    assert_eq!(payload_for(&erased.phrases[0], Route::VerdantVoice, 6789), payload_for(&neutral.phrases[0], Route::VerdantVoice, 6789));
}

#[test]
fn all_ten_gpui_parameters_report_real_route_support_and_missing_routes_disable_voice_controls() {
    let c=caps();assert_eq!(params::ALL.len(),10);
    for route in [Route::VerdantVoice] {
        for parameter in params::ALL {assert!(params::support(parameter,route,Some(&c)).is_effective(),"{parameter:?}/{route:?}");}
    }
    for parameter in [Param::Breathiness,Param::Brightness,Param::Clearness,Param::Gender,Param::Opening,Param::Growl] {
        let mut missing=c.clone();missing.native_score.as_mut().unwrap().expression=None;
        assert!(!params::support(parameter,Route::VerdantVoice,Some(&missing)).is_effective());
        let mut other=c.clone();other.native_score.as_mut().unwrap().expression.as_mut().unwrap().routes=vec!["C1".into()];
        assert!(!params::support(parameter,Route::VerdantVoice,Some(&other)).is_effective(),"UI must not offer a controller rejected by the planner");
    }
    assert_eq!(Param::Brightness.default_value(),64.);assert_eq!(Param::Gender.default_value(),64.);
    assert_eq!(Param::Portamento.default_value(),64.);assert_eq!(Param::Opening.default_value(),127.);
    assert_eq!(Param::Breathiness.default_value(),0.);assert_eq!(Param::Clearness.default_value(),0.);assert_eq!(Param::Growl.default_value(),0.);
}

#[test]
fn complete_run_regrouping_rebuilds_expression_ownership_and_integrates_tempo_inside_notes() {
    let mut p=project(80);
    // A tempo change inside note0 tests point time conversion independently
    // of fixed-tempo tick/duration scaling.
    p.tempo_map.push(TempoPoint{tick:1440,micro_bpm:60_000_000,unknown:Default::default()});
    for (index,note) in p.tracks[0].notes.iter_mut().enumerate() {
        for key in VOICE_KEYS {put(note,key,(index%100) as u8);
            note.curves.expression_points.get_mut(key).unwrap().push(ExpressionPoint{
                id:format!("{}-{key}-mid",note.id).as_str().into(),tick:480,value:100,unknown:Default::default()});
            note.curves.expression_points.get_mut(key).unwrap().push(ExpressionPoint{
                id:format!("{}-{key}-end",note.id).as_str().into(),tick:960,value:127,unknown:Default::default()});}
        put(note,"POR",64);
    }
    let before=p.clone();let c=caps();let result=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&c);
    assert!(!result.has_errors(),"{:?}",result.diagnostics);assert_eq!(p,before);
    assert_eq!(result.phrases.len(),1);let phrase=&result.phrases[0];assert!(phrase.seconds>60.);
    let expression=phrase.expression.as_ref().unwrap();assert_eq!(expression.segments.len(),80);
    let trace=&phrase.score.phoneme_timing.as_ref().unwrap().notes;
    let tempo=TempoMap::new(&p.tempo_map,p.ppq).unwrap();
    for (index,(segment,note)) in expression.segments.iter().zip(&p.tracks[0].notes).enumerate() {
        assert_eq!(segment.curves.len(),6);assert!(!segment.curves.contains_key("POR"));
        assert!((segment.note_onset_seconds-(tempo.seconds_at(note.start_tick as f64)-phrase.start_seconds)).abs()<1e-10);
        assert!((segment.start_seconds-(trace[index].note_onset_seconds-trace[index].prefix_seconds)).abs()<1e-10);
        assert!((segment.end_seconds-(trace[index].note_end_seconds-trace[index].tail_trim_seconds)).abs()<1e-10);
        assert_eq!(segment.curves["BRI"][0],[0.,(index%100) as f64]);
        if index>0 {assert!((expression.segments[index-1].end_seconds-segment.start_seconds).abs()<1e-9);}
    }
    assert_eq!(expression.segments[0].curves["BRI"],vec![[0.,0.],[0.25,100.],[0.75,127.]]);
    assert_eq!(expression.segments[1].curves["BRI"],vec![[0.,1.],[0.5,100.],[1.,127.]]);
    let payload=payload_for(phrase,Route::VerdantVoice,6789);let before_digest=render_digest(&payload,Route::VerdantVoice,&c);
    p.tracks[0].notes[70].curves.expression_points.get_mut("GWL").unwrap()[0].value+=1;
    let changed=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&c);
    let changed=payload_for(&changed.phrases[0],Route::VerdantVoice,6789);
    assert_eq!(changed.score,payload.score);assert_eq!(changed.control_track,payload.control_track);
    assert_eq!(changed.vocal_tract,payload.vocal_tract);assert_eq!(changed.native_seed,payload.native_seed);
    assert_ne!(render_digest(&changed,Route::VerdantVoice,&c),before_digest);
}

#[test]
fn every_authored_voice_controller_requires_its_declared_method_parameter_and_route() {
    for key in VOICE_KEYS {for route in [Route::VerdantVoice] {for case in 0..4 {
        let mut p=project(1);put(&mut p.tracks[0].notes[0],key,64);
        let mut c=caps();let capability=&mut c.native_score.as_mut().unwrap().expression;
        match case {0=>*capability=None,1=>capability.as_mut().unwrap().methods.clear(),
            2=>capability.as_mut().unwrap().parameters.retain(|p|p!=key),
            _=>capability.as_mut().unwrap().routes=vec!["C1".into()]}
        let result=plan(&p,&p.tracks[0].id,route,&c);
        assert!(result.phrases.is_empty());assert!(result.diagnostics.iter().any(|d|d.code=="E_EXPRESSION_CAPABILITY"));
        p.tracks[0].notes[0].curves.expression_points.clear();
        let restored=plan(&p,&p.tracks[0].id,route,&c);assert!(!restored.has_errors());
        assert!(payload_for(&restored.phrases[0],route,123).expression.is_none());
    }}}
}

#[test]
fn final_transients_are_independent_cache_inputs_and_reject_missing_or_nonfinite_capabilities() {
    for route in [Route::VerdantVoice] {
        let mut p=project(2);let c=caps();
        let baseline=plan(&p,&p.tracks[0].id,route,&c);assert!(!baseline.has_errors());
        let baseline=payload_for(&baseline.phrases[0],route,4567);
        assert!(serde_json::to_value(&baseline).unwrap().get("output_transients").is_none());
        for (strength,cap) in [(0.,6.),(1.,0.),(1.,6.)] {
            let mut setting=OutputTransientsSettings::phoneme_output_envelope_v1();setting.strength=strength;setting.max_attenuation_db=cap;
            setting.unknown.insert("future_tag".into(),json!("not a render option"));p.render_preferences.output_transients=Some(setting);
            let changed=plan(&p,&p.tracks[0].id,route,&c);assert!(!changed.has_errors());
            let changed=payload_for(&changed.phrases[0],route,4567);let mut stripped=changed.clone();stripped.output_transients=None;
            assert_eq!(stripped,baseline);assert_ne!(render_digest(&changed,route,&c),render_digest(&baseline,route,&c));
            assert_eq!(serde_json::to_value(&changed).unwrap()["output_transients"],json!({"method":"phoneme-output-envelope-v1","strength":strength,"max_attenuation_db":cap}));
        }
        p.render_preferences.output_transients=Some(OutputTransientsSettings::phoneme_output_envelope_v1());
        for case in 0..6 {
            let mut bad=c.clone();let field=&mut bad.native_score.as_mut().unwrap().output_transients;
            match case {0=>*field=None,1=>field.as_mut().unwrap().methods.clear(),2=>field.as_mut().unwrap().routes.clear(),
                3=>field.as_mut().unwrap().maximum_strength=None,
                4=>field.as_mut().unwrap().maximum_strength=Some(f64::INFINITY),
                _=>field.as_mut().unwrap().minimum_attenuation_db=Some(f64::NEG_INFINITY)}
            let result=plan(&p,&p.tracks[0].id,route,&bad);
            assert!(result.phrases.is_empty(),"case{case}");assert!(result.diagnostics.iter().any(|d|d.code=="E_OUTPUT_TRANSIENTS_CAPABILITY"));
        }
        let final_plan=plan(&p,&p.tracks[0].id,route,&c);let payload=payload_for(&final_plan.phrases[0],route,4567);
        let mut source=c.clone();let key="synthesis:native/verdant_acoustics/output_transients.py".to_string();
        source.files.insert(key.clone(),serde_json::from_value(json!({"available":true,"sha256":"before"})).unwrap());
        let before=render_digest(&payload,route,&source);source.files.get_mut(&key).unwrap().sha256=Some("after".into());
        assert_ne!(before,render_digest(&payload,route,&source));
    }
}
