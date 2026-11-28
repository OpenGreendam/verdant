use verdant_backend::{Capabilities, Route, plan};
use verdant_backend::adapter::payload_for;
use verdant_domain::model::PitchPoint;
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource};

fn capabilities(ceiling: Option<f64>) -> Capabilities {
    let mut caps: Capabilities = serde_json::from_value(serde_json::json!({
        "engine":"high-register fixture", "protocol":"ogd-worker/1", "native_version":"fixture",
        "routes":["verdant-voice"], "test_double":true, "max_phrase_seconds":30.0,
        "native_score":{"inventory":[], "english_vowel_proxies":["a"], "continuous_f0_control":true},
        
        "files":{"token":{"available":true,"sha256":"11".repeat(32)},
                 "model":{"available":true,"sha256":"22".repeat(32)},
                 "vocoder":{"available":true,"sha256":"33".repeat(32)}}
    })).unwrap();
    caps.native_score.as_mut().unwrap().explicit_f0_hz_range = ceiling.map(|high| [35.0, high]);
    caps
}

fn high_project() -> Project {
    let mut project = Project::new_empty();
    let mut note = Note::new(0, 960, 92);
    note.pronunciation = Pronunciation { pinyin: "a".into(), source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(), ..Default::default() };
    project.tracks[0].notes = vec![note];
    project
}

#[test]
fn extended_backend_accepts_original_key_and_sends_unclamped_controls() {
    let mut project = high_project();
    project.tracks[0].notes[0].curves.pitch_millicent_points.push(PitchPoint {
        id: "explicit-flat-pit".into(), tick: 0, millicents: 0, unknown: Default::default(),
    });
    let route = Route::VerdantVoice;
    for (midi, ceiling) in [(92, 2000.0), (93, 1760.0)] {
        project.tracks[0].notes[0].midi_note = midi;
        let result = plan(&project, &project.tracks[0].id, route, &capabilities(Some(ceiling)));
        assert!(!result.has_errors() && result.blocked.is_empty());
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let control = payload_for(&result.phrases[0], route, 10008).control_track.unwrap();
        let expected = 440.0 * 2.0f64.powf(f64::from(midi - 69) / 12.0);
        assert!(control.f0_hz.iter().any(|&f| f > 1600.0));
        for (&f, &uv) in control.f0_hz.iter().zip(&control.uv) {
            if !uv { assert!((f - expected).abs() < 0.000501); }
        }
    }
}

#[test]
fn declared_narrow_range_and_modulated_overshoot_are_blocked() {
    let mut project = high_project();
    for ceiling in [Some(1000.0)] {
        assert!(plan(&project, &project.tracks[0].id, Route::VerdantVoice, &capabilities(ceiling)).has_errors());
    }
    // Base B6 is below 2000 Hz; a +100-cent authored curve reaches C7.
    project.tracks[0].notes[0].midi_note = 95;
    project.tracks[0].notes[0].curves.pitch_millicent_points.push(PitchPoint {
        id: "explicit-high-pit".into(), tick: 0, millicents: 100_000, unknown: Default::default(),
    });
    for route in [Route::VerdantVoice] {
        let result = plan(&project, &project.tracks[0].id, route, &capabilities(Some(2000.0)));
        assert!(result.diagnostics.iter().any(|d| d.code == "E_CONTROL_PITCH_RANGE"));
        assert!(result.phrases.is_empty());
    }
}


fn voice_capabilities() -> Capabilities {
    let mut caps=capabilities(Some(2000.0));caps.test_double=true;
    caps.routes.push("verdant-voice".into());
    caps.other.insert("voice_package".into(),serde_json::json!({
        "authored_f0_policy":"native-target-reference-v1","authored_f0_hz_range":[35.,2000.]}));
    caps
}

#[test]
fn voice_target_range_admits_high_notes_without_warnings_and_rejects_c7_c8() {
    let caps=voice_capabilities();
    for (midi,admitted) in [(84,true),(92,true),(95,true),(96,false),(108,false)] {
        let mut project=high_project();project.tracks[0].notes[0].midi_note=midi;
        let before=project.clone();
        let result=plan(&project,&project.tracks[0].id,Route::VerdantVoice,&caps);
        assert_eq!(!result.has_errors(),admitted,"{:?}",result.diagnostics);
        assert_eq!(project,before);
        if admitted {
            assert!(result.diagnostics.is_empty(), "MIDI {midi}: {:?}", result.diagnostics);
            assert_eq!(result.phrases.len(), 1);
            assert!(result.blocked.is_empty());
        } else {
            assert!(result.phrases.is_empty());
            let error=result.diagnostics.iter().find(|d|d.code=="E_PITCH_RANGE").unwrap();
            assert!(error.message.contains("35.0–2000.0 Hz"));
        }
    }
}

#[test]
fn voice_curve_overshoot_reports_the_authored_note_and_native_limit() {
    let mut project=high_project();let note=&mut project.tracks[0].notes[0];
    note.midi_note=95;note.lyric="亮".into();
    note.curves.pitch_millicent_points.push(PitchPoint {
        id:"native-range-overshoot".into(),tick:0,millicents:100_000,unknown:Default::default()});
    let id=note.id.clone();
    let blocked=plan(&project,&project.tracks[0].id,Route::VerdantVoice,&voice_capabilities());
    let error=blocked.diagnostics.iter().find(|d|d.code=="E_CONTROL_PITCH_RANGE").unwrap();
    assert_eq!(error.note_id.as_ref(),Some(&id));
    assert!(error.message.contains("35.0–2000.0 Hz"));
    assert!(!blocked.diagnostics.iter().any(|d|d.code=="E_PITCH_RANGE"));
    assert!(blocked.phrases.is_empty()&&blocked.blocked.len()==1);
}

#[test]
fn vowel_anchors_preserve_admitted_high_register_targets() {
    let mut caps=voice_capabilities();
    caps.native_score.as_mut().unwrap().phoneme_timing=Some(serde_json::from_value(
        serde_json::json!({"methods":["vowel-anchor-v1"],"routes":["verdant-voice"],
            "timing_resolution_seconds":0.005})).unwrap());
    let mut project=high_project();project.tracks[0].notes[0].midi_note=95;
    project.render_preferences.phoneme_timing=Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1());
    project.tracks[0].refresh_index();
    let result=plan(&project,&project.tracks[0].id,Route::VerdantVoice,&caps);
    assert!(!result.has_errors(),"{:?}",result.diagnostics);
    let control=result.phrases[0].control_track.as_ref().unwrap();
    let expected=440.0*2.0_f64.powf(26.0/12.0);
    assert!(control.f0_hz.iter().zip(&control.uv).any(|(&hz,&uv)|!uv&&(hz-expected).abs()<0.000501));
}
