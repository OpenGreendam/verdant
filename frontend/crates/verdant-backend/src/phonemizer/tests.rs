use super::*;
use verdant_domain::{Note, PronunciationSource};

fn pinyin(text: &str) -> Pronunciation {
    Pronunciation { pinyin: text.into(), source: PronunciationSource::Manual, frontend_profile: DICTIONARY_SCHEMA.into(), ..Default::default() }
}

fn track(lyrics: &[&str]) -> Track {
    let mut track = Track::new("pronunciation fixture", "teal");
    track.notes = lyrics.iter().enumerate().map(|(index, lyric)| {
        let mut note = Note::new(index as i64 * 960, 960, 60);
        note.id = NoteId(format!("n{index}"));
        note.lyric = (*lyric).into();
        note
    }).collect();
    track.refresh_index();
    track
}

fn add_phrase(settings: &mut PhonemizerSettings, lyrics: &[&str], readings: &[&str]) {
    settings.dictionary.phrases.insert(lyrics.concat(), PhraseEntry {
        lyrics: lyrics.iter().map(|lyric| (*lyric).into()).collect(),
        readings: readings.iter().map(|reading| pinyin(reading)).collect(),
        ..Default::default()
    });
}

fn resolved<'a>(result: &'a ResolvedTrack, note: &Note) -> &'a ResolvedNote { &result.notes[&note.id] }

#[test]
fn automatic_default_resolves_actual_pinyin_kana_and_english() {
    let source = track(&["ren", "zhe", "mei", "chu", "lian", "mu", "biao", "ji", "song", "di", "dou", "bu", "ke", "lu", "you", "か", "シ", "hello", "world"]);
    let settings = settings_from_project(&Project::new_empty()).unwrap();
    assert_eq!(settings.selected, AUTOMATIC);
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for note in &source.notes[..15] {
        assert!(!resolved(&result, note).pronunciation.pinyin.is_empty(), "{}", note.lyric);
    }
    assert_eq!(resolved(&result, &source.notes[15]).pronunciation.language.as_deref(), Some("ja"));
    assert_eq!(resolved(&result, &source.notes[17]).pronunciation.language.as_deref(), Some("en"));
    let english = resolve_track(&track(&["song", "you", "love", "hello"]), &settings);
    assert!(english.diagnostics.is_empty(), "{:?}", english.diagnostics);
    assert!(english.notes.values().all(|note| note.pronunciation.language.as_deref() == Some("en")));
}

#[test]
fn explicit_language_controls_ambiguous_latin_and_romaji() {
    let mut source = track(&["you"]);
    for (language, expected) in [("en", "en"), ("ja", "ja"), ("zh", "zh")] {
        source.notes[0].pronunciation.language = Some(language.into());
        let result = resolve_track(&source, &PhonemizerSettings::default());
        assert!(result.diagnostics.is_empty(), "{language}: {:?}", result.diagnostics);
        let reading = &resolved(&result, &source.notes[0]).pronunciation;
        assert_eq!(reading.language.as_deref().unwrap_or("zh"), expected);
    }
    let japanese = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    let result = resolve_track(&track(&["sakura", "gakkou"]), &japanese);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.notes.values().all(|note| note.pronunciation.language.as_deref() == Some("ja")));
    let english = PhonemizerSettings { selected: ENGLISH_G2P.into(), ..Default::default() };
    let result = resolve_track(&track(&["ren", "song"]), &english);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.notes.values().all(|note| note.pronunciation.language.as_deref() == Some("en")));
}

#[test]
fn actual_imported_aliases_are_finite_playable_readings() {
    let aliases = ["i t", "a t", "e t", "e k", "u sh", "o V", "u V", "i k", "す2", "に3"];
    let source = track(&aliases);
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for note in &source.notes {
        let reading = &resolved(&result, note).pronunciation;
        let phones = crate::reading::resolve_pronunciation(reading).unwrap();
        let durations = crate::phonemes::allocate_for_note(&phones, 0.8, None, reading.mora_weights.as_deref()).unwrap();
        assert!((durations.iter().sum::<f64>() - 0.8).abs() < 1e-12, "{}", note.lyric);
        assert!(phones.phones.iter().all(|phone| phone.source_language == "ja"));
    }
    assert!(resolve_track(&track(&["す3"]), &PhonemizerSettings::default()).notes.is_empty());
}

#[test]
fn automatic_romaji_preserves_pinyin_and_english_ambiguities() {
    for (text, language) in [("sakura", "ja"), ("gakkou", "ja"), ("tsunami", "ja"),
        ("nǐ", "zh"), ("ni3", "zh"), ("shi", "zh"), ("ren", "zh"),
        ("banana", "en"), ("piano", "en"), ("radio", "en"), ("video", "en"),
        ("anyone", "en"), ("share", "en"), ("choose", "en")] {
        let source = track(&[text]);
        let result = resolve_track(&source, &PhonemizerSettings::default());
        assert!(result.diagnostics.is_empty(), "{text}: {:?}", result.diagnostics);
        assert_eq!(resolved(&result, &source.notes[0]).pronunciation.language.as_deref().unwrap_or("zh"), language, "{text}");
    }
    let source = track(&["arigatou", "か", "shi", "hello"]);
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(resolved(&result, &source.notes[0]).pronunciation.language.as_deref(), Some("ja"));
    assert_eq!(resolved(&result, &source.notes[2]).pronunciation.language.as_deref(), Some("ja"));
    // A supplied English hint also governs pinyin-like fragments at note boundaries.
    let mut source = track(&["か", "shi", "na", "la", "don't", "it's", "yeah"]);
    for note in &mut source.notes[1..] { note.pronunciation.language = Some("en".into()); }
    let before = serde_json::to_value(&source).unwrap();
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for note in &source.notes[1..] {
        assert_eq!(resolved(&result, note).pronunciation.language.as_deref(), Some("en"));
    }
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}

#[test]
fn automatic_isolated_romaji_vowels_follow_touching_kana() {
    for (roman, kana, ipa) in [("a", "あ", "a"), ("i", "い", "i"), ("u", "う", "ɯ"), ("e", "え", "e"), ("o", "お", "o")] {
        for spelling in [roman.to_string(), roman.to_ascii_uppercase()] {
            for roman_first in [false, true] {
                let lyrics = if roman_first { [spelling.as_str(), kana] } else { [kana, spelling.as_str()] };
                let source = track(&lyrics);
                let before = serde_json::to_value(&source).unwrap();
                let result = resolve_track(&source, &PhonemizerSettings::default());
                assert!(result.diagnostics.is_empty(), "{lyrics:?}: {:?}", result.diagnostics);
                for note in &source.notes {
                    let reading = &resolved(&result, note).pronunciation;
                    assert_eq!(reading.language.as_deref(), Some("ja"), "{lyrics:?}");
                    assert_eq!(reading.phonemes, [ipa], "{lyrics:?}");
                    let phones = crate::reading::resolve_pronunciation(reading).unwrap();
                    assert_eq!(phones.phones.len(), 1, "{lyrics:?}");
                    assert_eq!(phones.phones[0].native.role, "nucleus", "{lyrics:?}");
                    assert!(phones.phones[0].native.voiced, "{lyrics:?}");
                }
                assert_eq!(serde_json::to_value(&source).unwrap(), before);
            }
        }
    }
}

#[test]
fn automatic_romaji_vowel_context_stops_at_existing_boundaries() {
    for boundary in ["gap", "overlap", "mute", "segment"] {
        let mut source = track(&["あ", "a"]);
        let mut settings = PhonemizerSettings::default();
        match boundary {
            "gap" => source.notes[1].start_tick += 1,
            "overlap" => source.notes[1].start_tick -= 1,
            "mute" => source.notes[0].set_muted(true),
            "segment" => { settings.segments.insert("first".into(), SegmentEntry {
                track: source.id.clone(), start_tick: 0, end_tick: 960,
                selected: AUTOMATIC.into(), unknown: Map::new(),
            }); },
            _ => unreachable!(),
        }
        let result = resolve_track(&source, &settings);
        assert!(result.diagnostics.is_empty(), "{boundary}: {:?}", result.diagnostics);
        let reading = &resolved(&result, &source.notes[1]).pronunciation;
        assert_eq!(reading.language.as_deref(), Some("en"), "{boundary}");
        assert_eq!(reading.phonemes, ["ə"], "{boundary}");
    }
}

#[test]
fn automatic_romaji_vowels_keep_explicit_reading_and_language_precedence() {
    let mut source = track(&["あ", "a"]);
    source.notes[1].pronunciation.language = Some("en".into());
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(resolved(&result, &source.notes[1]).pronunciation.phonemes, ["ə"]);

    for provenance in [PronunciationSource::Manual, PronunciationSource::Imported] {
        let mut source = track(&["あ", "a"]);
        let (_, mut reading) = crate::reading::parse_phonetic_input("a/[en/ipa:ə]").unwrap().unwrap();
        reading.source = provenance;
        source.notes[1].pronunciation = reading.clone();
        let result = resolve_track(&source, &PhonemizerSettings::default());
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(serde_json::to_value(&resolved(&result, &source.notes[1]).pronunciation).unwrap(),
            serde_json::to_value(&reading).unwrap());
    }

    let source = track(&["あ", "a"]);
    let (_, reading) = crate::reading::parse_phonetic_input("a/[en/ipa:ə]").unwrap().unwrap();
    let mut settings = PhonemizerSettings::default();
    settings.dictionary.lyrics.insert("a".into(), LyricEntry { reading, ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(resolved(&result, &source.notes[1]).pronunciation.phonemes, ["ə"]);

    let english = super::automatic::reading("a", ENGLISH_G2P, None, Some("ja")).unwrap();
    assert_eq!(english.language.as_deref(), Some("en"));
    assert_eq!(english.phonemes, ["ə"]);
}

#[test]
fn automatic_vowel_context_does_not_change_english_words() {
    for lyrics in [&["a"][..], &["hello", "a"][..], &["あ", "hello"][..], &["あ", "you"][..], &["あ", "song"][..]] {
        let source = track(lyrics);
        let result = resolve_track(&source, &PhonemizerSettings::default());
        assert!(result.diagnostics.is_empty(), "{lyrics:?}: {:?}", result.diagnostics);
        let reading = &resolved(&result, source.notes.last().unwrap()).pronunciation;
        assert_eq!(reading.language.as_deref(), Some("en"), "{lyrics:?}");
        if source.notes.last().unwrap().lyric == "a" { assert_eq!(reading.phonemes, ["ə"]); }
    }
}

#[test]
fn english_contractions_do_not_split_off_consonant_syllables() {
    for text in ["don't", "can't", "won't", "I'm", "it's", "let's"] {
        let reading = super::english::reading(text).unwrap();
        assert!(!reading.tokens.iter().any(|token| token == "."), "{text}: {:?}", reading.tokens);
    }
    for invalid in ["123", "???", "か", "''", "---"] {
        assert!(super::english::reading(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn standalone_sokuon_uses_only_the_next_resolved_japanese_onset() {
    let mut source = track(&["っ", "て"]);
    let settings = PhonemizerSettings::default();
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let reading = &resolved(&result, &source.notes[0]).pronunciation;
    assert_eq!(reading.phonemes, ["tː"]);
    assert_eq!(resolved(&result, &source.notes[0]).nucleus, None);
    let cached = reading.clone();
    // Stored contextual output must follow a changed following note.
    source.notes[0].pronunciation = cached;
    source.notes[1].lyric = "か".into();
    let changed = resolve_track(&source, &settings);
    assert_eq!(resolved(&changed, &source.notes[0]).pronunciation.phonemes, ["kː"]);
    for lyric in ["あ", "hello"] {
        source.notes[1].lyric = lyric.into();
        assert!(!resolve_track(&source, &settings).notes.contains_key(&source.notes[0].id));
    }
    source.notes[1].lyric = "て".into();
    source.notes[1].start_tick += 1;
    assert!(!resolve_track(&source, &settings).notes.contains_key(&source.notes[0].id));
}

#[test]
fn sokuon_dictionaries_and_authored_segment_boundaries_win() {
    let source = track(&["っ", "て"]);
    let mut settings = PhonemizerSettings::default();
    settings.segments.insert("next".into(), SegmentEntry { track: source.id.clone(),
        start_tick: 960, end_tick: 1920, selected: JAPANESE_KANA.into(), unknown: Map::new() });
    assert!(!resolve_track(&source, &settings).notes.contains_key(&source.notes[0].id));
    settings.segments.clear();
    settings.dictionary.lyrics.insert("て".into(), LyricEntry { reading: pinyin("te4"), ..Default::default() });
    assert!(!resolve_track(&source, &settings).notes.contains_key(&source.notes[0].id));
    settings.dictionary.lyrics.insert("っ".into(), LyricEntry { reading: pinyin("a1"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert_eq!(resolved(&result, &source.notes[0]).pronunciation.pinyin, "a1");
}

#[test]
fn mixed_note_parts_split_exactly_and_preserve_language_and_manual_timing() {
    let source = track(&["  ren   か hello  "]);
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let mut reading = resolved(&result, &source.notes[0]).pronunciation.clone();
    let phones = crate::reading::resolve_pronunciation(&reading).unwrap();
    assert_eq!(phones.parts.iter().map(|part| part.language.as_str()).collect::<Vec<_>>(), ["zh", "ja", "en"]);
    let durations = crate::phonemes::allocate_for_note(&phones, 1.2, None, None).unwrap();
    let mut offset = 0;
    for part in &phones.parts {
        assert!((durations[offset..offset + part.phone_count].iter().sum::<f64>() - 0.4).abs() < 1e-12);
        offset += part.phone_count;
    }
    assert!(crate::phonemes::allocate_for_note(&phones, 0.05, None, None).unwrap_err().contains("25 ms"));
    let compact = crate::phonemes::allocate_for_note_with_minimum(&phones, 0.12, None, None, 0.005).unwrap();
    let mut compact_offset = 0;
    for part in &phones.parts {
        let end = compact_offset + part.phone_count;
        assert!((compact[compact_offset..end].iter().sum::<f64>() - 0.04).abs() < 1e-12);
        assert!(compact[compact_offset..end].iter().all(|&duration| duration >= 0.005 - 1e-12));
        compact_offset = end;
    }
    assert_eq!(crate::phonemes::allocate_for_note_with_minimum(&phones, 1.2, None, None, 0.005).unwrap(), durations);
    let count = phones.phones.len();
    let mut weights = vec![1000 / count as u32; count];
    weights[count - 1] += 1000 - weights.iter().sum::<u32>();
    reading.phoneme_durations_permille = Some(weights.clone());
    let durations = crate::phonemes::allocate_for_note(&phones, 3.0, Some(&weights), None).unwrap();
    assert!((durations[0] - 3.0 * weights[0] as f64 / 1000.0).abs() < 1e-12);
    reading.phonemes[0] = "b".into();
    assert!(crate::reading::resolve_pronunciation(&reading).is_err(), "stale part provenance must fail");
}

fn multilingual_caps() -> crate::wire::Capabilities {
    serde_json::from_value(serde_json::json!({
        "engine":"multilingual adapter fixture", "protocol":"ogd-worker/1", "native_version":"fixture", "test_double":true,
        "routes":["verdant-voice"], "max_active_jobs":1, "max_phrase_seconds":30.0,
        "native_score":{"inventory":crate::pinyin::recovered_syllables(), "sub_syllable_timing":true,
            "phoneme_duration_control":true, "continuous_f0_control":false,
            "phoneme_frontend":{"schema_version":crate::phonemes::SCHEMA, "alphabets":["ipa"],
                "languages":{"zh":"native", "en":"native", "ja":"approximate"}}},
        "files":{"token":{"available":true,"sha256":"11".repeat(32)},"model":{"available":true,"sha256":"22".repeat(32)}}
    })).unwrap()
}

#[test]
fn authored_phone_timing_survives_automatic_and_dictionary_resolution() {
    let mut source = track(&["か", "海"]);
    for note in &mut source.notes { note.pronunciation.phoneme_durations_permille = Some(vec![400, 600]); }
    let mut settings = PhonemizerSettings::default();
    settings.dictionary.lyrics.insert("海".into(), LyricEntry { reading: pinyin("hai4"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for note in &source.notes {
        assert_eq!(resolved(&result, note).pronunciation.phoneme_durations_permille, Some(vec![400, 600]));
    }
    source.notes[0].pronunciation.phoneme_durations_permille = Some(vec![1000]);
    let invalid = resolve_track(&source, &settings);
    assert!(!invalid.notes.contains_key(&source.notes[0].id));
    assert!(invalid.diagnostics.iter().any(|diagnostic| diagnostic.code == "E_PHONEME_TIMING"));
}

#[test]
fn mixed_parts_reach_normal_audio_payload_and_missing_language_blocks() {
    let mut project = Project::new_empty();
    project.tracks[0] = track(&["ren か hello"]);
    let original = project.tracks[0].notes.clone();
    let caps = multilingual_caps();
    let plan = crate::adapter::plan(&project, &project.tracks[0].id, crate::wire::Route::VerdantVoice, &caps);
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    assert_eq!(plan.phrases.len(), 1);
    let languages: std::collections::BTreeSet<_> = plan.phrases[0].score.segments.iter()
        .filter_map(|segment| segment.phoneme.as_ref().map(|phone| phone.source_language.as_str())).collect();
    assert_eq!(languages, std::collections::BTreeSet::from(["zh", "ja", "en"]));
    assert_eq!(project.tracks[0].notes, original);
    let mut unavailable = caps.clone();
    unavailable.native_score.as_mut().unwrap().phoneme_frontend.as_mut().unwrap().languages.remove("en");
    let blocked = crate::adapter::plan(&project, &project.tracks[0].id, crate::wire::Route::VerdantVoice, &unavailable);
    assert!(blocked.phrases.is_empty());
    assert!(blocked.diagnostics.iter().any(|diagnostic| diagnostic.code == "E_RENDER_CAPABILITY"));
}

#[test]
fn longest_exact_phrase_beats_lyric_and_builtin_but_preserves_explicit_note() {
    let mut source = track(&["重", "庆", "市"]);
    for (note, reading) in source.notes.iter_mut().zip(["zhong4", "qing4", "shi4"]) {
        note.pronunciation = pinyin(reading);
        note.pronunciation.source = PronunciationSource::ExplicitMap;
        note.pronunciation.input_text = Some(note.lyric.clone());
    }
    source.notes[1].pronunciation = pinyin("qing1");
    source.notes[1].pronunciation.source = PronunciationSource::Imported;
    let original = serde_json::to_value(&source).unwrap();
    let mut settings = PhonemizerSettings::default();
    add_phrase(&mut settings, &["重", "庆"], &["zhong4", "qing4"]);
    add_phrase(&mut settings, &["重", "庆", "市"], &["chong2", "qing4", "shi4"]);
    settings.dictionary.lyrics.insert("重".into(), LyricEntry { reading: pinyin("zhong1"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(resolved(&result, &source.notes[0]).pronunciation.pinyin, "chong2");
    assert_eq!(resolved(&result, &source.notes[0]).origin, ReadingOrigin::UserPhrase { phrase: "重庆市".into() });
    assert_eq!(resolved(&result, &source.notes[1]).pronunciation.pinyin, "qing1");
    assert_eq!(resolved(&result, &source.notes[1]).origin, ReadingOrigin::ExplicitNote);
    assert_eq!(serde_json::to_value(&source).unwrap(), original);
    settings.dictionary.phrases.clear();
    let result = resolve_track(&source, &settings);
    assert_eq!(resolved(&result, &source.notes[0]).pronunciation.pinyin, "zhong1");
    assert_eq!(resolved(&result, &source.notes[0]).origin, ReadingOrigin::UserLyric { lyric: "重".into() });
}

#[test]
fn invalid_explicit_note_stays_unresolved_instead_of_using_dictionary() {
    let mut source = track(&["海"]);
    source.notes[0].pronunciation = pinyin("not-a-reading");
    let mut settings = PhonemizerSettings::default();
    settings.dictionary.lyrics.insert("海".into(), LyricEntry { reading: pinyin("hai3"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert!(result.notes.is_empty());
    assert_eq!(result.diagnostics[0].code, "E_UNSUPPORTED_SYLLABLE");
    assert_eq!(result.diagnostics[0].note_id.as_ref(), Some(&source.notes[0].id));
}

#[test]
fn automatic_kana_reading_is_recompiled_after_a_lyric_edit() {
    let mut source = track(&["か"]);
    let settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    let first = resolve_track(&source, &settings);
    source.notes[0].pronunciation = resolved(&first, &source.notes[0]).pronunciation.clone();
    let unchanged = resolve_track(&source, &settings);
    assert_eq!(resolved(&unchanged, &source.notes[0]).origin, ReadingOrigin::ExistingBuiltin);
    source.notes[0].lyric = "き".into();
    let authored = serde_json::to_value(&source).unwrap();
    let updated = resolve_track(&source, &settings);
    assert!(updated.diagnostics.is_empty(), "{:?}", updated.diagnostics);
    let reading = resolved(&updated, &source.notes[0]);
    assert_eq!(reading.origin, ReadingOrigin::Builtin);
    assert_eq!(reading.pronunciation.phonemes, ["kʲ", "i"]);
    assert_eq!(reading.pronunciation.input_text.as_deref(), Some("き"));
    assert_eq!(reading.pronunciation.reading_provenance.as_ref().unwrap()["reading"]["original_input"], "き");
    assert_eq!(serde_json::to_value(&source).unwrap(), authored, "resolution must not mutate the saved note");

    // A formerly valid kana reading must not conceal an unresolved new lyric.
    source.notes[0].lyric = "海".into();
    let unsupported = resolve_track(&source, &settings);
    assert!(unsupported.notes.is_empty());
    assert_eq!(unsupported.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    assert_eq!(unsupported.diagnostics[0].note_id.as_ref(), Some(&source.notes[0].id));
}

#[test]
fn explicit_kana_reading_keeps_precedence_when_displayed_lyric_changes() {
    let settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    for provenance in [PronunciationSource::Manual, PronunciationSource::Imported] {
        let mut source = track(&["学校"]);
        source.notes[0].pronunciation = crate::reading::compile_kana("がっこう", "explicit", true).unwrap();
        source.notes[0].pronunciation.source = provenance;
        let expected = source.notes[0].pronunciation.clone();
        let result = resolve_track(&source, &settings);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(resolved(&result, &source.notes[0]).origin, ReadingOrigin::ExplicitNote);
        assert_eq!(resolved(&result, &source.notes[0]).pronunciation, expected);
    }
}

#[test]
fn phrases_require_exact_lyric_boundaries_and_touching_unmuted_notes() {
    let mut settings = PhonemizerSettings::default();
    add_phrase(&mut settings, &["重", "庆"], &["chong2", "qing4"]);
    for boundary in ["gap", "overlap", "mute", "continuation", "other-track"] {
        let mut source = track(&["重", "庆"]);
        match boundary {
            "gap" => source.notes[1].start_tick += 1,
            "overlap" => source.notes[1].start_tick -= 1,
            "mute" => source.notes[1].set_muted(true),
            "continuation" => source.notes[1].set_continuation(true),
            "other-track" => { source.notes.pop(); },
            _ => unreachable!(),
        }
        let result = resolve_track(&source, &settings);
        assert!(result.notes.values().all(|note| !matches!(note.origin, ReadingOrigin::UserPhrase { .. })), "{boundary}");
        assert_eq!(resolved(&result, &source.notes[0]).pronunciation.pinyin, "zhong4", "{boundary}");
    }
    let source = track(&["重庆"]);
    let result = resolve_track(&source, &settings);
    assert!(result.notes.is_empty(), "a two-note phrase is not guessed inside one note");
    assert_eq!(result.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
}

#[test]
fn preferred_polyphone_keeps_candidates_and_unknown_lyrics_stay_unresolved() {
    let source = track(&["好", "🦀"]);
    let mut settings = PhonemizerSettings::default();
    let result = resolve_track(&source, &settings);
    let automatic = &resolved(&result, &source.notes[0]).pronunciation;
    assert_eq!(automatic.pinyin, "hao3");
    assert_eq!(automatic.reading_provenance.as_ref().unwrap()["candidates"], serde_json::json!(verdant_domain::hanzi_readings("好")));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    settings.dictionary.lyrics.insert("好".into(), LyricEntry { reading: pinyin("hao4"), ..Default::default() });
    let chosen = resolve_track(&source, &settings);
    assert_eq!(resolved(&chosen, &source.notes[0]).pronunciation.pinyin, "hao4");
    assert_eq!(resolved(&chosen, &source.notes[0]).origin, ReadingOrigin::UserLyric { lyric: "好".into() });
    assert_eq!(source.notes[0].pronunciation, Pronunciation::default());
}

#[test]
fn contextual_mandarin_uses_words_without_mutating_notes() {
    for (lyrics, index, reading) in [
        (["银", "行"], 1, "hang2"), (["行", "走"], 0, "xing2"),
        (["重", "复"], 0, "chong2"), (["重", "要"], 0, "zhong4"),
        (["长", "大"], 0, "zhang3"), (["长", "城"], 0, "chang2"),
        (["音", "乐"], 1, "yue4"), (["快", "乐"], 1, "le4"),
        (["爱", "好"], 1, "hao4"), (["好", "人"], 0, "hao3"),
        (["中", "奖"], 0, "zhong4"), (["中", "间"], 0, "zhong1"),
    ] {
        let source = track(&lyrics);
        let before = serde_json::to_value(&source).unwrap();
        let result = resolve_track(&source, &PhonemizerSettings::default());
        assert!(result.diagnostics.is_empty(), "{lyrics:?}: {:?}", result.diagnostics);
        let pronunciation = &resolved(&result, &source.notes[index]).pronunciation;
        assert_eq!(pronunciation.pinyin, reading, "{lyrics:?}");
        assert_eq!(pronunciation.reading_provenance.as_ref().unwrap()["basis"], "phrase");
        assert_eq!(serde_json::to_value(&source).unwrap(), before);
    }
}

#[test]
fn contextual_mandarin_stops_at_rest_segment_and_foreign_reading() {
    for boundary in ["gap", "overlap", "mute", "continuation", "segment", "foreign"] {
        let mut source = track(&["银", "行"]);
        let mut settings = PhonemizerSettings::default();
        match boundary {
            "gap" => source.notes[1].start_tick += 1,
            "overlap" => source.notes[1].start_tick -= 1,
            "mute" => source.notes[0].set_muted(true),
            "continuation" => source.notes[0].set_continuation(true),
            "segment" => { settings.segments.insert("one".into(), SegmentEntry {
                track: source.id.clone(), start_tick: 0, end_tick: 960,
                selected: MANDARIN.into(), unknown: Map::new(),
            }); },
            "foreign" => { source.notes[0].pronunciation = crate::reading::compile_kana("ぎん", "explicit", true).unwrap(); },
            _ => unreachable!(),
        }
        let result = resolve_track(&source, &settings);
        assert_eq!(resolved(&result, &source.notes[1]).pronunciation.pinyin, "xing2", "{boundary}");
    }
    let source = track(&["银", "行"]);
    for selected in [EXPLICIT_ONLY, JAPANESE_KANA] {
        let settings = PhonemizerSettings { selected: selected.into(), ..Default::default() };
        assert!(!resolve_track(&source, &settings).notes.contains_key(&source.notes[1].id));
    }
    let source = track(&["🦀", "🙂", "银行"]);
    let result = resolve_track(&source, &PhonemizerSettings::default());
    assert!(result.notes.is_empty());
    assert!(result.diagnostics.iter().all(|diagnostic| diagnostic.code == "E_PRONUNCIATION_UNSET"));
    let uncovered = track(&["单"]);
    let result = resolve_track(&uncovered, &PhonemizerSettings::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let reading = &resolved(&result, &uncovered.notes[0]).pronunciation;
    assert_eq!(reading.pinyin, "dan1");
    assert_eq!(reading.pinyin, verdant_domain::hanzi_primary_reading("单").unwrap());
    assert!(verdant_domain::hanzi_readings("单").contains(&reading.pinyin));
    assert_eq!(reading.reading_provenance.as_ref().unwrap()["basis"], "dictionary_primary");
    assert_eq!(uncovered.notes[0].pronunciation, Pronunciation::default());
    assert_eq!(verdant_domain::hanzi_primary_reading("银行"), None);
    assert_eq!(verdant_domain::hanzi_primary_reading("🦀"), None);
}

#[test]
fn contextual_mandarin_keeps_explicit_and_project_reading_priority() {
    let mut source = track(&["银", "行"]);
    let mut settings = PhonemizerSettings::default();
    add_phrase(&mut settings, &["银", "行"], &["yin2", "heng2"]);
    assert_eq!(resolved(&resolve_track(&source, &settings), &source.notes[1]).pronunciation.pinyin, "heng2");
    settings.dictionary.phrases.clear();
    settings.dictionary.lyrics.insert("行".into(), LyricEntry { reading: pinyin("xing4"), ..Default::default() });
    assert_eq!(resolved(&resolve_track(&source, &settings), &source.notes[1]).pronunciation.pinyin, "xing4");
    for provenance in [PronunciationSource::Manual, PronunciationSource::Imported, PronunciationSource::ExplicitMap] {
        source.notes[1].pronunciation = pinyin("xing2");
        source.notes[1].pronunciation.source = provenance;
        if provenance == PronunciationSource::ExplicitMap { settings.dictionary.lyrics.clear(); }
        assert_eq!(resolved(&resolve_track(&source, &settings), &source.notes[1]).pronunciation.pinyin, "xing2");
    }
}

#[test]
fn contextual_tables_only_choose_mit_dictionary_candidates() {
    use super::resolution::polyphone::{PHRASES, PREFERRED};
    let mut words = std::collections::BTreeSet::new();
    for &(word, readings) in PHRASES {
        assert!(words.insert(word), "duplicate lexical word {word}");
        assert_eq!(word.chars().count(), readings.len());
        for (character, reading) in word.chars().zip(readings) {
            assert!(verdant_domain::hanzi_readings(&character.to_string()).iter().any(|candidate| candidate.as_str() == *reading), "{word}: {character}/{reading}");
        }
    }
    for &(character, reading) in PREFERRED {
        assert!(verdant_domain::hanzi_readings(character).iter().any(|candidate| candidate == reading), "{character}/{reading}");
    }
}

#[test]
fn dictionary_import_rejects_duplicates_at_every_object_level_and_invalid_readings() {
    for input in [
        r#"{"schema_version":"verdant-pronunciation-dictionary/1","lyrics":{},"lyrics":{}}"#,
        r#"{"schema_version":"verdant-pronunciation-dictionary/1","lyrics":{"海":{"reading":{"pinyin":"hai3","source":"manual"}},"\u6d77":{"reading":{"pinyin":"hai4","source":"manual"}}}}"#,
        r#"{"schema_version":"verdant-pronunciation-dictionary/1","future":{"key":1,"key":2}}"#,
        r#"{"schema_version":"verdant-pronunciation-dictionary/1","lyrics":{"海":{"reading":{"pinyin":"hai3","pinyin":"hai4","source":"manual"}}}}"#,
    ] {
        assert!(import_dictionary(input).unwrap_err().contains("duplicate JSON key"), "{input}");
    }
    let mut dictionary = UserDictionary::default();
    for reading in [pinyin("hai9"), pinyin("hai3 hao3"), Pronunciation::default(),
        crate::reading::parse_phonetic_input("x/[en:NOT_A_PHONE]").unwrap().unwrap().1] {
        dictionary.lyrics.insert("海".into(), LyricEntry { reading, ..Default::default() });
        let input = serde_json::to_string(&dictionary).unwrap();
        assert!(import_dictionary(&input).is_err(), "{input}");
        assert!(export_dictionary(&dictionary).is_err());
    }
    let mut invalid_weights = pinyin("hai3");
    invalid_weights.phoneme_durations_permille = Some(vec![500, 501]);
    dictionary.lyrics.insert("海".into(), LyricEntry { reading: invalid_weights, ..Default::default() });
    assert!(dictionary.validate().is_err());
    assert!(import_dictionary("{}").is_err());
    assert!(import_dictionary("[]").is_err());
}

#[test]
fn settings_and_dictionary_round_trip_unknown_data_and_selected_frontend() {
    let mut settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    add_phrase(&mut settings, &["重", "庆"], &["chong2", "qing4"]);
    settings.unknown.insert("future-setting".into(), serde_json::json!({"mode": [1, {"opaque": true}]}));
    settings.dictionary.unknown.insert("future-dictionary".into(), serde_json::json!(["kept"]));
    let phrase = settings.dictionary.phrases.get_mut("重庆").unwrap();
    phrase.unknown.insert("future-entry".into(), serde_json::json!({"source": "user"}));
    phrase.readings[0].unknown.insert("future-reading".into(), serde_json::json!(42));
    phrase.readings[0].reading_provenance = Some(serde_json::json!({"unrecognized": {"keep": "verbatim"}}));
    let exported = export_dictionary(&settings.dictionary).unwrap();
    assert_eq!(import_dictionary(&exported).unwrap(), settings.dictionary);
    let mut project = Project::new_empty();
    project.extensions.insert(EXTENSION_KEY.into(), settings_to_extension(&settings).unwrap());
    let serialized = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&serialized).unwrap();
    assert_eq!(settings_from_project(&restored).unwrap(), settings);
    let mut duplicate_field = settings.dictionary.clone();
    duplicate_field.unknown.insert("lyrics".into(), serde_json::json!({}));
    assert!(export_dictionary(&duplicate_field).is_err());
}

#[test]
fn selected_kana_frontend_keeps_existing_mapping_and_never_uses_mandarin_for_kanji() {
    let mut source = track(&["か", "海"]);
    source.notes[1].pronunciation = pinyin("hai3");
    source.notes[1].pronunciation.source = PronunciationSource::ExplicitMap;
    let settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    let result = resolve_track(&source, &settings);
    let kana = resolved(&result, &source.notes[0]);
    assert_eq!(kana.pronunciation.language.as_deref(), Some("ja"));
    assert_eq!(kana.pronunciation.phonemes, ["k", "a"]);
    assert_eq!(kana.pronunciation.mora_weights, Some(vec![1]));
    assert!(kana.pronunciation.allow_approximation);
    assert!(kana.pronunciation.pinyin.is_empty());
    assert!(!result.notes.contains_key(&source.notes[1].id));
    assert_eq!(result.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    let only = PhonemizerSettings { selected: EXPLICIT_ONLY.into(), ..Default::default() };
    assert!(resolve_track(&track(&["海"]), &only).notes.is_empty());
}

#[test]
fn continuation_chains_keep_stable_anchor_and_dictionary_changes_reach_copied_seeds() {
    let mut source = track(&["重", "+", "-"]);
    source.notes[0].pronunciation = pinyin("zhong4");
    source.notes[0].pronunciation.source = PronunciationSource::ExplicitMap;
    source.notes[0].pronunciation.input_text = Some("重".into());
    for index in 1..source.notes.len() {
        source.notes[index].pronunciation = source.notes[0].pronunciation.clone();
        source.notes[index].pronunciation.source = PronunciationSource::Manual;
    }
    let original = serde_json::to_value(&source).unwrap();
    let mut settings = PhonemizerSettings::default();
    settings.dictionary.lyrics.insert("重".into(), LyricEntry { reading: pinyin("chong2"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for note in &source.notes[1..] {
        let continuation = resolved(&result, note);
        assert_eq!(continuation.continuation_of.as_ref(), Some(&source.notes[0].id));
        assert_eq!(continuation.nucleus, Some(1));
        assert_eq!(continuation.pronunciation.pinyin, "chong2");
    }
    assert_eq!(serde_json::to_value(&source).unwrap(), original);
}

#[test]
fn continuation_reports_missing_gapped_muted_unresolved_and_consonant_only_anchors() {
    let settings = PhonemizerSettings::default();
    let absent = resolve_track(&track(&["+"]), &settings);
    assert_eq!(absent.diagnostics[0].code, "E_READING_CONTINUATION_ANCHOR");
    for overlap in [false, true] {
        let mut source = track(&["海", "+"]);
        source.notes[1].start_tick += if overlap { -1 } else { 1 };
        let result = resolve_track(&source, &settings);
        assert_eq!(result.diagnostics[0].code, "E_READING_CONTINUATION_BOUNDARY");
    }
    let mut source = track(&["海", "+"]);
    source.notes[0].set_muted(true);
    assert_eq!(resolve_track(&source, &settings).diagnostics[0].code, "E_READING_CONTINUATION_MUTED");
    let result = resolve_track(&track(&["🦀", "+"]), &settings);
    assert!(result.diagnostics.iter().any(|error| error.code == "E_READING_CONTINUATION_UNRESOLVED"));
    let mut source = track(&["t", "+"]);
    source.notes[0].pronunciation = crate::reading::parse_phonetic_input("t/[en:t]").unwrap().unwrap().1;
    let result = resolve_track(&source, &settings);
    assert_eq!(result.diagnostics[0].code, "E_READING_CONTINUATION_NUCLEUS");
    let mut source = track(&["海", "+"]);
    source.notes[1].pronunciation = pinyin("hao3");
    assert_eq!(resolve_track(&source, &settings).diagnostics[0].code, "E_READING_CONTINUATION_OVERRIDE");
}

/// This test exercises the real adapter consumer rather than manually applying
/// the map. Capabilities are model-free fixtures, not a claim about audio quality.
#[test]
fn persisted_phrase_dictionary_changes_real_adapter_segments_and_render_digest() {
    let mut project = Project::new_empty();
    project.tracks[0] = track(&["重", "庆"]);
    project.tracks[0].notes[0].pronunciation = pinyin("zhong4");
    project.tracks[0].notes[0].pronunciation.source = PronunciationSource::ExplicitMap;
    project.tracks[0].notes[1].pronunciation = pinyin("qing4");
    project.tracks[0].notes[1].pronunciation.source = PronunciationSource::ExplicitMap;
    let original_notes = project.tracks[0].notes.clone();
    let caps: crate::wire::Capabilities = serde_json::from_value(serde_json::json!({
        "engine": "pronunciation adapter fixture (not a voice)", "protocol": "ogd-worker/1",
        "native_version": "fixture-only", "test_double": true, "routes": ["verdant-voice"],
        "max_active_jobs": 1, "max_phrase_seconds": 30.0,
        "native_score": {"inventory": crate::pinyin::recovered_syllables(), "sub_syllable_timing": true,
            "english_vowel_proxies": [], "continuous_f0_control": false, "coarticulation_recovered": false},
        "files": {"token": {"available": true, "sha256": "11".repeat(32)}, "model": {"available": true, "sha256": "22".repeat(32)}}
    })).unwrap();
    let before = crate::adapter::plan(&project, &project.tracks[0].id, crate::wire::Route::VerdantVoice, &caps);
    assert!(!before.has_errors(), "{:?}", before.diagnostics);
    let mut settings = PhonemizerSettings::default();
    add_phrase(&mut settings, &["重", "庆"], &["chong2", "qing4"]);
    project.extensions.insert(EXTENSION_KEY.into(), settings_to_extension(&settings).unwrap());
    let after = crate::adapter::plan(&project, &project.tracks[0].id, crate::wire::Route::VerdantVoice, &caps);
    assert!(!after.has_errors(), "{:?}", after.diagnostics);
    assert!(before.phrases[0].score.segments.iter().any(|segment| segment.label.starts_with("zhong4:")));
    assert!(after.phrases[0].score.segments.iter().any(|segment| segment.label.starts_with("chong2:")));
    let payload = |plan: &crate::adapter::PhrasePlan| crate::adapter::payload_for(&plan.phrases[0], crate::wire::Route::VerdantVoice, 1);
    assert_ne!(crate::adapter::render_digest(&payload(&before), crate::wire::Route::VerdantVoice, &caps),
        crate::adapter::render_digest(&payload(&after), crate::wire::Route::VerdantVoice, &caps));
    assert_eq!(project.tracks[0].notes, original_notes);
}

#[test]
fn mandarin_segment_over_opening_hanzi_keeps_kana_default_elsewhere() {
    let source = track(&["死", "妈", "海", "い", "が", "し"]);
    let mut settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    settings.segments.insert("opening-hanzi".into(), SegmentEntry { track: source.id.clone(), start_tick: 0, end_tick: 2880, selected: MANDARIN.into(), unknown: Map::new() });
    let original = serde_json::to_value(&source).unwrap();
    let result = resolve_track(&source, &settings);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for (index, reading) in [(0, "si3"), (1, "ma1"), (2, "hai3")] {
        let hanzi = resolved(&result, &source.notes[index]);
        assert_eq!(hanzi.pronunciation.pinyin, reading);
        assert_eq!(hanzi.pronunciation.source, PronunciationSource::ExplicitMap);
        assert_eq!(hanzi.pronunciation.input_text.as_deref(), Some(source.notes[index].lyric.as_str()));
        assert_eq!(hanzi.origin, ReadingOrigin::Builtin);
    }
    for (index, phones) in [(3, &["i"][..]), (4, &["ɡ", "a"][..]), (5, &["ɕ", "i"][..])] {
        let kana = resolved(&result, &source.notes[index]);
        assert_eq!(kana.pronunciation.language.as_deref(), Some("ja"));
        assert_eq!(kana.pronunciation.phonemes, phones);
        assert!(kana.pronunciation.allow_approximation);
        assert!(kana.pronunciation.pinyin.is_empty());
        assert_eq!(kana.origin, ReadingOrigin::Builtin);
    }
    assert_eq!(serde_json::to_value(&source).unwrap(), original);
}

#[test]
fn segment_boundary_includes_start_tick_and_excludes_end_tick() {
    let mut source = track(&["い", "死", "去", "い"]);
    for (note, start) in source.notes.iter_mut().zip([0, 960, 2880, 3840]) { note.start_tick = start; }
    source.refresh_index();
    let mut settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    settings.segments.insert("hanzi".into(), SegmentEntry { track: source.id.clone(), start_tick: 960, end_tick: 2880, selected: MANDARIN.into(), unknown: Map::new() });
    let result = resolve_track(&source, &settings);
    assert_eq!(result.diagnostics.len(), 1, "{:?}", result.diagnostics);
    assert_eq!(result.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    assert_eq!(result.diagnostics[0].note_id.as_ref(), Some(&source.notes[2].id));
    assert!(!result.notes.contains_key(&source.notes[2].id));
    for index in [0, 3] {
        let kana = resolved(&result, &source.notes[index]);
        assert_eq!(kana.pronunciation.phonemes, ["i"]);
        assert_eq!(kana.pronunciation.language.as_deref(), Some("ja"));
        assert_eq!(kana.origin, ReadingOrigin::Builtin);
    }
    let hanzi = resolved(&result, &source.notes[1]);
    assert_eq!(hanzi.pronunciation.pinyin, "si3");
    assert_eq!(hanzi.origin, ReadingOrigin::Builtin);
    assert_eq!(settings.segment_selection(&source.id, 0), None);
    assert_eq!(settings.segment_selection(&source.id, 959), None);
    assert_eq!(settings.segment_selection(&source.id, 960), Some(MANDARIN));
    assert_eq!(settings.segment_selection(&source.id, 2879), Some(MANDARIN));
    assert_eq!(settings.segment_selection(&source.id, 2880), None);
}

#[test]
fn explicit_note_and_user_dictionary_readings_still_beat_a_segment() {
    let mut source = track(&["死", "去", "海"]);
    source.notes[0].pronunciation = pinyin("si4");
    let mut settings = PhonemizerSettings::default();
    settings.segments.insert("kana".into(), SegmentEntry { track: source.id.clone(), start_tick: 0, end_tick: 2880, selected: JAPANESE_KANA.into(), unknown: Map::new() });
    settings.dictionary.lyrics.insert("去".into(), LyricEntry { reading: pinyin("qu3"), ..Default::default() });
    let result = resolve_track(&source, &settings);
    let explicit = resolved(&result, &source.notes[0]);
    assert_eq!(explicit.pronunciation.pinyin, "si4");
    assert_eq!(explicit.origin, ReadingOrigin::ExplicitNote);
    let lyric = resolved(&result, &source.notes[1]);
    assert_eq!(lyric.pronunciation.pinyin, "qu3");
    assert_eq!(lyric.origin, ReadingOrigin::UserLyric { lyric: "去".into() });
    assert!(!result.notes.contains_key(&source.notes[2].id));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    assert_eq!(result.diagnostics[0].note_id.as_ref(), Some(&source.notes[2].id));
    settings.segments.clear();
    let inherited = resolve_track(&source, &settings);
    assert!(inherited.diagnostics.is_empty(), "{:?}", inherited.diagnostics);
    assert_eq!(resolved(&inherited, &source.notes[2]).pronunciation.pinyin, "hai3");
    assert_eq!(resolved(&inherited, &source.notes[2]).origin, ReadingOrigin::Builtin);
}

#[test]
fn a_segment_only_selects_the_frontend_of_its_own_track() {
    let first = track(&["死"]);
    let mut second = track(&["死"]);
    second.id = TrackId::from("track-b");
    let mut settings = PhonemizerSettings { selected: MANDARIN.into(), ..Default::default() };
    settings.segments.insert("kana".into(), SegmentEntry { track: first.id.clone(), start_tick: 0, end_tick: 960, selected: JAPANESE_KANA.into(), unknown: Map::new() });
    let scoped = resolve_track(&first, &settings);
    assert!(scoped.notes.is_empty());
    assert_eq!(scoped.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    assert_eq!(scoped.diagnostics[0].note_id.as_ref(), Some(&first.notes[0].id));
    assert!(scoped.diagnostics[0].message.contains("日语假名"), "{:?}", scoped.diagnostics);
    let untouched = resolve_track(&second, &settings);
    assert!(untouched.diagnostics.is_empty(), "{:?}", untouched.diagnostics);
    let hanzi = resolved(&untouched, &second.notes[0]);
    assert_eq!(hanzi.pronunciation.pinyin, "si3");
    assert_eq!(hanzi.origin, ReadingOrigin::Builtin);
    assert_eq!(settings.segment_selection(&first.id, 0), Some(JAPANESE_KANA));
    assert_eq!(settings.segment_selection(&second.id, 0), None);
}

#[test]
fn segment_validation_rejects_invalid_ranges_frontends_and_overlaps_but_allows_separate_tracks() {
    let track_a = TrackId::from("track-a");
    let track_b = TrackId::from("track-b");
    let entry = |track: &TrackId, start: i64, end: i64, selected: &str| SegmentEntry { track: track.clone(), start_tick: start, end_tick: end, selected: selected.into(), unknown: Map::new() };
    let mut settings = PhonemizerSettings::default();
    settings.segments.insert("a".into(), entry(&track_a, 0, 960, MANDARIN));
    settings.segments.insert("b".into(), entry(&track_a, 960, 1920, JAPANESE_KANA));
    settings.segments.insert("c".into(), entry(&track_b, 0, 1920, JAPANESE_KANA));
    settings.validate().unwrap();
    let mut overlap = settings.clone();
    overlap.segments.insert("d".into(), entry(&track_a, 1919, 2880, MANDARIN));
    let error = overlap.validate().unwrap_err();
    assert!(error.contains("重叠"), "{error}");
    assert!(error.contains("\"b\"") && error.contains("\"d\""), "{error}");
    let mut nested = settings.clone();
    nested.segments.insert("d".into(), entry(&track_a, 100, 200, MANDARIN));
    assert!(nested.validate().unwrap_err().contains("重叠"));
    for reserved in ["track", "start_tick", "end_tick", "selected"] {
        let mut duplicated = settings.clone();
        duplicated.segments.get_mut("a").unwrap().unknown.insert(reserved.into(), serde_json::json!({"opaque": true}));
        assert_eq!(duplicated.validate().unwrap_err(), format!("extension field {reserved:?} duplicates a defined field"));
    }
    let cases = [
        (entry(&track_a, 960, 960, MANDARIN), "分段 \"bad\" 的结束 tick 必须大于起始 tick"),
        (entry(&track_a, 1920, 960, MANDARIN), "分段 \"bad\" 的结束 tick 必须大于起始 tick"),
        (entry(&track_a, -1, 960, MANDARIN), "分段 \"bad\" 的起始 tick 不能为负数"),
        (entry(&track_a, 2880, 3840, "romaji-g2p-v1"), "unsupported phonemizer \"romaji-g2p-v1\"; choose mandarin-dictionary-v1, explicit-only-v1, japanese-kana-v1, english-g2p-v1 or automatic-text-v1"),
    ];
    for (bad, message) in cases {
        let mut invalid = settings.clone();
        invalid.segments.insert("bad".into(), bad);
        assert_eq!(invalid.validate().unwrap_err(), message);
    }
}

#[test]
fn segment_settings_round_trip_and_legacy_settings_stay_compatible() {
    let mut settings = PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() };
    let mut opening = SegmentEntry { track: TrackId::from("track-a"), start_tick: 960, end_tick: 24240, selected: MANDARIN.into(), unknown: Map::new() };
    opening.unknown.insert("future-segment".into(), serde_json::json!({"opaque": [1, true]}));
    settings.segments.insert("opening-hanzi".into(), opening);
    settings.segments.insert("later-kana".into(), SegmentEntry { track: TrackId::from("track-b"), start_tick: 0, end_tick: 960, selected: JAPANESE_KANA.into(), unknown: Map::new() });
    let extension = settings_to_extension(&settings).unwrap();
    assert_eq!(extension["segments"]["opening-hanzi"]["track"], serde_json::json!("track-a"));
    assert_eq!(extension["segments"]["opening-hanzi"]["start_tick"], serde_json::json!(960));
    assert_eq!(extension["segments"]["opening-hanzi"]["end_tick"], serde_json::json!(24240));
    assert_eq!(extension["segments"]["opening-hanzi"]["selected"], serde_json::json!(MANDARIN));
    assert_eq!(extension["segments"]["opening-hanzi"]["future-segment"], serde_json::json!({"opaque": [1, true]}));
    let mut project = Project::new_empty();
    project.extensions.insert(EXTENSION_KEY.into(), extension);
    let serialized = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&serialized).unwrap();
    assert_eq!(settings_from_project(&restored).unwrap(), settings);
    let legacy = PhonemizerSettings::default();
    let value = settings_to_extension(&legacy).unwrap();
    assert!(value.get("segments").is_none(), "{value}");
    let mut older = Project::new_empty();
    older.extensions.insert(EXTENSION_KEY.into(), serde_json::json!({"schema_version": SCHEMA, "selected": JAPANESE_KANA}));
    let restored_legacy = settings_from_project(&older).unwrap();
    assert_eq!(restored_legacy, PhonemizerSettings { selected: JAPANESE_KANA.into(), ..Default::default() });
    assert!(restored_legacy.segments.is_empty());
}
