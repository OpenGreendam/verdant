use serde_json::json;
use verdant_backend::{Capabilities, Route, plan};
use verdant_backend::adapter::minimum_segment_seconds;
use verdant_backend::phoneme_timing::{drag_boundary, minimum_permille, note_budget};
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource, TempoMap};

fn caps(compact: bool) -> Capabilities {
    serde_json::from_value(json!({
        "test_double": true, "routes": ["verdant-voice", "C0"], "max_phrase_seconds": 30,
        "native_score": {"inventory": ["a", "ba"], "sub_syllable_timing": true,
            "compact_phone_states": compact, "phoneme_duration_control": true,
            "continuous_f0_control": true, "explicit_f0_hz_range": [35, 2000],
            "phoneme_frontend": {"schema_version": "verdant-phonemes/2", "alphabets": ["ipa"],
                "languages": {"en": "native", "ja": "approximate", "zh": "native"}},
            "phoneme_timing": {"methods": ["vowel-anchor-v1"], "routes": ["verdant-voice"],
                "timing_resolution_seconds": 0.005}}
    })).unwrap()
}

fn note(start: i64, duration: i64, pinyin: &str) -> Note {
    let mut note = Note::new(start, duration, 64);
    note.lyric = pinyin.into();
    note.pronunciation = Pronunciation { pinyin: pinyin.into(), source: PronunciationSource::Manual,
        ..Default::default() };
    note
}

fn project(notes: Vec<Note>) -> Project {
    let mut project = Project::new_empty();
    // One tick is exactly 1 ms, making admission boundaries unambiguous.
    project.ppq = 1000;
    project.tempo_map[0].micro_bpm = 60_000_000;
    project.tracks[0].notes = notes;
    project.tracks[0].refresh_index();
    project
}

fn near(a: f64, b: f64) { assert!((a-b).abs() < 1e-12, "{a} != {b}"); }

#[test]
fn capability_defaults_false_and_is_voice_only() {
    let old: Capabilities = serde_json::from_value(json!({"native_score": {}})).unwrap();
    assert!(!old.native_score.as_ref().unwrap().compact_phone_states);
    assert_eq!(minimum_segment_seconds(Route::C0, &caps(true)), 0.025);
}

#[test]
fn admission_uses_complete_phones_and_preserves_positive_onsets_and_gaps() {
    let p = project(vec![note(1000, 10, "ba1"), note(1013, 5, "a1")]);
    let before = p.clone();
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let phrase = &result.phrases[0];
    near(phrase.start_seconds, 1.0);
    assert_eq!(phrase.score.segments.len(), 3);
    for segment in &phrase.score.segments { near(segment.duration_seconds, 0.005); }
    near(phrase.score.segments[1].gap_after_seconds, 0.003);
    near(phrase.seconds, 0.018);
    assert_eq!(p, before);
    for (route, c) in [(Route::VerdantVoice, caps(false))] {
        let rejected = plan(&p, &p.tracks[0].id, route, &c);
        assert!(rejected.phrases.is_empty());
        assert!(rejected.diagnostics.iter().any(|d| d.code == "E_SYLLABLE_DOMAIN"));
    }
    let short = project(vec![note(1000, 4, "a1")]);
    let rejected = plan(&short, &short.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(rejected.diagnostics.iter().any(|d| d.code == "E_NOTE_TOO_SHORT" && d.message.contains("5 ms")));
}

#[test]
fn ordinary_allocations_remain_identical_and_weights_stay_authoritative() {
    let p = project(vec![note(1000, 200, "ba1")]);
    let legacy = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(false));
    let compact = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(!legacy.has_errors() && !compact.has_errors());
    assert_eq!(legacy.phrases, compact.phrases);
    let mut n = note(1000, 20, "ba1");
    n.pronunciation.phoneme_durations_permille = Some(vec![250, 750]);
    let p = project(vec![n]);
    let tempo = TempoMap::new(&p.tempo_map, p.ppq).unwrap();
    let budget = note_budget(&p.tracks[0].notes[0], &tempo, &caps(true), Route::VerdantVoice).unwrap();
    assert_eq!(budget.weights_permille, Some(vec![250, 750]));
    near(budget.durations_seconds[0], 0.005);
    near(budget.durations_seconds[1], 0.015);
    assert_eq!(budget.minimum_segment_seconds, 0.005);
    assert_eq!(budget.minimum_permille(), 250);
    let batch = drag_boundary(&p, &p.tracks[0].id, &p.tracks[0].notes[0].id, &caps(true), 1, 0.005).unwrap();
    assert_eq!(batch.commands.len(), 1);
    assert!(drag_boundary(&p, &p.tracks[0].id, &p.tracks[0].notes[0].id, &caps(true), 1, 0.004).unwrap_err().contains("5 ms"));
    assert!(note_budget(&p.tracks[0].notes[0], &tempo, &caps(false), Route::VerdantVoice).is_err());
}

#[test]
fn explicit_phones_and_continuations_keep_their_native_segment_choice() {
    let mut first = note(1000, 15, "");
    first.pronunciation = Pronunciation { language: Some("en".into()), alphabet: Some("ipa".into()),
        phonemes: vec!["k".into(), "æ".into()], source: PronunciationSource::Manual,
        phoneme_durations_permille: Some(vec![400, 600]), ..Default::default() };
    let mut continuation = Note::new(1015, 5, 64);
    continuation.lyric = "+".into();
    let p = project(vec![first, continuation]);
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let segments = &result.phrases[0].score.segments;
    assert_eq!(segments.len(), 3);
    near(segments[0].duration_seconds, 0.006);
    near(segments[1].duration_seconds, 0.009);
    near(segments[2].duration_seconds, 0.005);
    assert_eq!(segments[2].continuation_of, Some(1));
    assert!(segments[2].phoneme.as_ref().unwrap().canonical_ipa == "æ");
}

#[test]
fn whole_permille_boundary_roundoff_is_shared_by_controls() {
    for (total, floor, expected) in [(0.5, 0.025, 50), (0.05, 0.025, 500),
        (0.005 * 1000.0 / 120.0, 0.005, 120), (0.019, 0.005, 264)] {
        assert_eq!(minimum_permille(total, floor), expected);
    }
}

#[test]
fn anchored_short_notes_use_the_actual_floor_without_moving_authored_geometry() {
    let mut p = project(vec![note(1000, 10, "ba1"), note(1010, 10, "ba1")]);
    p.render_preferences.phoneme_timing = Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1());
    let before = p.clone();
    let result = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let phrase = &result.phrases[0];
    near(phrase.start_seconds, 0.995);
    let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
    let mut elapsed = 0.0;
    let mut starts = Vec::new();
    for segment in &phrase.score.segments {
        starts.push(elapsed);
        assert!(segment.duration_seconds >= 0.005 - 1e-12);
        elapsed += segment.duration_seconds + segment.gap_after_seconds;
    }
    for (index, note) in p.tracks[0].notes.iter().enumerate() {
        near(phrase.start_seconds + trace[index].note_onset_seconds, note.start_tick as f64 / 1000.0);
        near(starts[trace[index].nucleus_segment_index], trace[index].note_onset_seconds);
    }
    assert_eq!(p, before);
    p.tracks[0].notes[1].pronunciation.phoneme_durations_permille = Some(vec![600, 400]);
    // The authored final is only 4 ms: preserve the weights and reject it.
    let rejected = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(rejected.phrases.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.message.contains("5 ms")));
}
