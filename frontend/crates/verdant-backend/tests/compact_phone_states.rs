use serde_json::json;
use verdant_backend::{Capabilities, Route, plan};
use verdant_backend::adapter::{minimum_segment_seconds, plan_desktop};
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
                "timing_resolution_seconds": 0.005, "constrained_nucleus_timing": true}}
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
        near(starts[trace[index].nucleus_segment_index.unwrap()], trace[index].note_onset_seconds);
        near(trace[index].nucleus_delay_seconds, 0.0);
    }
    assert_eq!(p, before);
    p.tracks[0].notes[1].pronunciation.phoneme_durations_permille = Some(vec![600, 400]);
    // The authored final is only 4 ms: preserve the weights and reject it.
    let rejected = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps(true));
    assert!(rejected.phrases.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.message.contains("5 ms")));
}

fn anchored_project(notes: Vec<Note>) -> Project {
    let mut p = project(notes);
    p.render_preferences.phoneme_timing = Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1());
    p
}

#[test]
fn seven_millisecond_pinyin_uses_a_full_or_partial_lead() {
    let caps = caps(true);
    for (start, lead) in [(1000, 0.005), (3, 0.003)] {
        let mut p = anchored_project(vec![note(start, 7, "ba1")]);
        let before = p.clone();
        let track = &p.tracks[0];
        let result = plan_desktop(&p, &track.id, &caps);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let [phrase] = result.phrases.as_slice() else { panic!("expected one phrase") };
        let [initial, vowel] = phrase.score.segments.as_slice() else { panic!("expected ba initial and final") };
        let [trace] = phrase.score.phoneme_timing.as_ref().unwrap().notes.as_slice() else { panic!("expected one source note") };
        let onset = start as f64 / 1000.0;
        assert_eq!((phrase.start_tick, phrase.end_tick), (start, start + 7));
        assert_eq!(trace.note_id, track.notes[0].id.as_str());
        assert_eq!((trace.first_segment_index, trace.nucleus_segment_index, trace.end_segment_index), (0, Some(1), 2));
        assert_eq!(initial.context.as_ref().unwrap().features["C_SM"], "b");
        assert_eq!(vowel.context.as_ref().unwrap().features["C_YM"], "a");
        assert_eq!((initial.midi_note, vowel.midi_note), (Some(64.0), Some(64.0)));
        near(phrase.start_seconds, onset - lead);
        near(phrase.start_seconds + trace.note_onset_seconds, onset);
        near(phrase.start_seconds + trace.note_end_seconds, onset + 0.007);
        near(trace.prefix_seconds, lead);
        near(trace.nucleus_delay_seconds, 0.005 - lead);
        near(trace.tail_trim_seconds, 0.0);
        near(initial.duration_seconds, 0.005);
        near(initial.gap_after_seconds, 0.0);
        near(initial.duration_seconds, trace.note_onset_seconds + trace.nucleus_delay_seconds);
        near(vowel.duration_seconds, 0.007 - trace.nucleus_delay_seconds);
        near(vowel.gap_after_seconds, 0.0);
        near(initial.duration_seconds + vowel.duration_seconds, phrase.seconds);
        near(phrase.start_seconds + phrase.seconds, onset + 0.007);
        assert_eq!(p, before);

        p.render_preferences.phoneme_timing = None;
        let legacy = plan_desktop(&p, &p.tracks[0].id, &caps);
        assert!(legacy.phrases.is_empty());
        assert!(legacy.errors_for(&p.tracks[0].notes[0].id).any(|d| d.code == "E_SYLLABLE_DOMAIN"));
    }
}

#[test]
fn an_impossible_opening_lead_blocks_only_its_phrase() {
    let p = anchored_project(vec![note(0, 7, "ba1"), note(7, 5, "a1"), note(1000, 7, "ba1")]);
    let track = &p.tracks[0];
    let result = plan_desktop(&p, &track.id, &caps(true));
    assert!(result.errors_for(&track.notes[0].id).any(|d| d.code == "E_PHONEME_TIMING_BUDGET"));
    let [blocked] = result.blocked.as_slice() else { panic!("expected just the opening phrase to be blocked") };
    assert_eq!(blocked.note_ids, [track.notes[0].id.clone(), track.notes[1].id.clone()]);
    assert_eq!((blocked.start_tick, blocked.end_tick), (0, 12));
    let [later] = result.phrases.as_slice() else { panic!("the note after the long rest should render") };
    let isolated = anchored_project(vec![track.notes[2].clone()]);
    let single = plan_desktop(&isolated, &isolated.tracks[0].id, &caps(true));
    let [expected] = single.phrases.as_slice() else { panic!("expected the isolated short note to render") };
    assert_eq!(later.note_ids, expected.note_ids);
    assert_eq!(later.score.segments, expected.score.segments);
    assert_eq!(later.score.phoneme_timing, expected.score.phoneme_timing);
    near(later.start_seconds, expected.start_seconds);
    near(later.seconds, expected.seconds);
}

#[test]
fn an_impossible_lead_cannot_trim_the_previous_request() {
    let p = anchored_project(vec![note(0, 30_000, "a1"), note(30_000, 500, "ma1"),
        note(30_500, 5, "a1"), note(30_505, 7, "ma1")]);
    let track = &p.tracks[0];
    let mut capabilities = caps(true);
    capabilities.native_score.as_mut().unwrap().inventory.push("ma".into());
    let result = plan_desktop(&p, &track.id, &capabilities);
    assert!(result.errors_for(&track.notes[3].id).any(|d| d.code == "E_PHONEME_TIMING_BUDGET"));
    let [blocked] = result.blocked.as_slice() else { panic!("expected just the successor group to be blocked") };
    assert_eq!(blocked.note_ids, track.notes[1..].iter().map(|n| n.id.clone()).collect::<Vec<_>>());
    let [phrase] = result.phrases.as_slice() else { panic!("expected the preceding vowel to render") };
    assert_eq!(phrase.note_ids, [track.notes[0].id.clone()]);
    near(phrase.seconds, 30.0);
    near(phrase.score.segments[0].duration_seconds, 30.0);
    near(phrase.score.phoneme_timing.as_ref().unwrap().notes[0].tail_trim_seconds, 0.0);
}

#[test]
fn short_ipa_clusters_and_kana_keep_their_resolved_phones() {
    for (input, ticks, alphabet, nucleus, phones) in [
        ("word/[en/ipa:s k æ t]", 11, "ipa", 2, &[("s", 0.005), ("k", 0.005), ("æ", 0.006), ("t", 0.005)][..]),
        ("word/[en/ipa:k æ iː]", 13, "ipa", 1, &[("k", 0.005), ("æ", 0.006), ("iː", 0.007)][..]),
        ("か/[ja/kana:か]", 7, "kana", 1, &[("k", 0.005), ("a", 0.007)][..]),
    ] {
        let mut n = Note::new(1000, ticks, 64);
        (n.lyric, n.pronunciation) = verdant_backend::reading::parse_phonetic_input(input).unwrap().unwrap();
        let p = anchored_project(vec![n]);
        let result = plan_desktop(&p, &p.tracks[0].id, &caps(true));
        assert!(!result.has_errors(), "{input}: {:?}", result.diagnostics);
        let [phrase] = result.phrases.as_slice() else { panic!("expected one phrase") };
        let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
        assert_eq!(trace.nucleus_segment_index, Some(nucleus));
        near(trace.prefix_seconds, nucleus as f64 * 0.005);
        near(trace.nucleus_delay_seconds, 0.0);
        near(phrase.start_seconds + trace.note_onset_seconds, 1.0);
        let mut clock = phrase.start_seconds;
        assert_eq!(phrase.score.segments.len(), phones.len());
        for (index, (segment, &(symbol, seconds))) in phrase.score.segments.iter().zip(phones).enumerate() {
            if index == nucleus { near(clock, 1.0); }
            let phone = segment.phoneme.as_ref().unwrap();
            assert_eq!(phone.canonical_ipa, symbol);
            assert_eq!(phone.input_alphabet, alphabet);
            near(segment.duration_seconds, seconds);
            near(segment.gap_after_seconds, 0.0);
            clock += segment.duration_seconds;
        }
        near(clock, 1.0 + ticks as f64 / 1000.0);
    }
}

#[test]
fn a_preroll_cannot_replace_declared_phone_mora_or_lyric_part_budgets() {
    let mut too_short = note(1000, 7, "ba1");
    too_short.pronunciation.phoneme_durations_permille = Some(vec![500, 500]);
    let mut malformed = note(1000, 7, "ba1");
    malformed.pronunciation.phoneme_durations_permille = Some(vec![1000]);
    let mut moras = Note::new(1000, 17, 64);
    (moras.lyric, moras.pronunciation) = verdant_backend::reading::parse_phonetic_input("かか/[ja/kana:かか]").unwrap().unwrap();
    let mut parts = Note::new(1000, 17, 64);
    parts.lyric = "か か".into();
    for n in [too_short, malformed, moras, parts] {
        let p = anchored_project(vec![n]);
        let result = plan_desktop(&p, &p.tracks[0].id, &caps(true));
        assert!(result.phrases.is_empty());
        assert!(result.errors_for(&p.tracks[0].notes[0].id).any(|d| d.code == "E_PHONEME_TIMING"));
    }
}
