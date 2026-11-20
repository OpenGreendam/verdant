//! Compatibility of main's dictionary/inventory/long-note admission with the
//! PR's authored clock and explicit F0 contract. No voice model is required.
use verdant_backend::adapter::{NoteSupport, note_support};
use verdant_backend::f0::validate;
use verdant_backend::wire::Capabilities;
use verdant_backend::{Route, plan};
use verdant_domain::model::PitchPoint;
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource, TempoMap};

fn capabilities() -> Capabilities {
    serde_json::from_value(serde_json::json!({
        "engine": "main merge test fixture", "protocol": "ogd-worker/1", "native_version": "fixture",
        "routes":["verdant-voice"], "test_double":true, "max_active_jobs": 1, "max_phrase_seconds": 30.0,
        "native_score": {
            "inventory": verdant_backend::pinyin::recovered_syllables(),
            "english_vowel_proxies": ["a", "i", "u", "e", "o"],
            "continuous_f0_control": true, "coarticulation_recovered": false,
            "sub_syllable_timing": true, "phoneme_duration_control": true
        },
        
        "files": {
            "token": {"available": true, "sha256": "11".repeat(32)},
            "model": {"available": true, "sha256": "22".repeat(32)},
            "vocoder": {"available": true, "sha256": "33".repeat(32)}
        }
    }))
    .unwrap()
}

fn dictionary_note(start: i64, duration: i64) -> Note {
    let mut note = Note::new(start, duration, 64);
    note.lyric = "海".into();
    note.pronunciation = verdant_domain::pronunciation_for_lyric(&Pronunciation::default(), "", &note.lyric, None).unwrap();
    note.curves.pitch_millicent_points = vec![PitchPoint { id: "pit".into(), tick: 0, millicents: 100_000, unknown: Default::default() }];
    note
}

fn project_with(note: Note) -> Project {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 100_000_000;
    project.tracks[0].notes = vec![note];
    project
}

#[test]
fn dictionary_long_note_keeps_explicit_pitch_and_subsyllable_budget() {
    let mut note = dictionary_note(18, 9600); // Six seconds, beyond the old five-second domain.
    assert_eq!(note.pronunciation.pinyin, "hai3");
    assert_eq!(note.pronunciation.source, PronunciationSource::ExplicitMap);
    note.pronunciation.phoneme_durations_permille = Some(vec![200, 800]);
    let before = note.clone();
    let project = project_with(note);
    let plan = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &capabilities());
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    let phrase = &plan.phrases[0];
    assert!((phrase.seconds - 6.0).abs() < 1e-12);
    assert_eq!(phrase.score.segments.len(), 2);
    let initial = &phrase.score.segments[0];
    let final_ = &phrase.score.segments[1];
    assert_eq!(initial.context.as_ref().unwrap().features["C_SM"], "h");
    assert_eq!(final_.context.as_ref().unwrap().features["C_YM"], "ai");
    assert_eq!(final_.context.as_ref().unwrap().features["C_Tone"], "3");
    assert!((initial.duration_seconds - 1.2).abs() < 1e-12);
    assert!((initial.duration_seconds + final_.duration_seconds - phrase.seconds).abs() < 1e-12);
    let control = phrase.control_track.as_ref().unwrap();
    validate(control).unwrap();
    let expected_hz = verdant_domain::timing::midi_to_hz(65, 0); // E4 + authored 100 cents.
    for (&hz, &uv) in control.f0_hz[..control.f0_hz.len() - 1].iter().zip(&control.uv) {
        assert!(!uv);
        assert!((hz - expected_hz).abs() < 0.001); // Lexical tone does not add a contour.
    }
    assert_eq!(control.frame_times_seconds.last(), Some(&phrase.seconds));
    assert_eq!(control.uv.last(), Some(&true));
    assert_eq!(project.tracks[0].notes[0], before);
}

#[test]
fn thirty_second_dictionary_note_at_nonzero_origin_is_not_roundoff_rejected() {
    let tempo = TempoMap::constant(100.0);
    let raw_seconds = tempo.seconds_at(48_003.0) - tempo.seconds_at(3.0);
    assert!(raw_seconds > 30.0); // Regression trigger.
    let note = dictionary_note(3, 48_000);
    let direct = verdant_backend::f0::control_track(&[&note], &tempo, tempo.seconds_at(3.0), raw_seconds);
    validate(&direct).unwrap();
    assert_eq!(direct.frame_times_seconds.last(), Some(&30.0));
    assert!(verdant_backend::f0::control_track(&[&note], &tempo, tempo.seconds_at(3.0), 30.0 + 1e-8).uv.is_empty());
    let project = project_with(dictionary_note(3, 48_000));
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &capabilities());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let phrase = &result.phrases[0];
    assert_eq!(phrase.seconds, 30.0);
    assert_eq!(phrase.score.segments.iter().map(|s| s.duration_seconds + s.gap_after_seconds).sum::<f64>(), 30.0);
    let control = phrase.control_track.as_ref().unwrap();
    validate(control).unwrap();
    assert_eq!(control.frame_times_seconds.last(), Some(&30.0));
    assert_eq!(control.uv.last(), Some(&true));

    let longer = project_with(dictionary_note(3, 48_001));
    let rejected = plan(&longer, &longer.tracks[0].id, Route::VerdantVoice, &capabilities());
    assert!(rejected.phrases.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.code == "E_PHRASE_TOO_LONG"));
}

#[test]
fn dictionary_admission_never_falls_back_to_a_legacy_vowel_proxy() {
    let caps = capabilities();
    let mut note = Note::new(0, 960, 60);
    note.lyric = "行".into();
    assert_eq!(note_support(&note, &caps).unwrap_err().code, "E_PRONUNCIATION_AMBIGUOUS");
    note.pronunciation = Pronunciation {
        pinyin: "u".into(),
        source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(),
        ..Default::default()
    };
    assert!(matches!(note_support(&note, &caps), Ok(NoteSupport::VowelProxy { .. })));
    note.pronunciation.source = PronunciationSource::ExplicitMap;
    assert!(matches!(note_support(&note, &caps), Ok(NoteSupport::Pinyin(token)) if token.final_ == "u" && token.syllable == "wu"));
}
