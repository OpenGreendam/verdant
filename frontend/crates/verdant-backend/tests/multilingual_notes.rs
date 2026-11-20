//! Model-free integration of editor reading input, project transactions and
//! the real ScoreAdapter. Capabilities and file hashes below are fixtures;
//! these tests establish wire/authoring contracts, not acoustic quality.
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::reading::parse_phonetic_input;
use verdant_backend::{Capabilities, NoteSupport, Phrase, PhrasePlan, Route, note_support, plan};
use verdant_domain::io::{load_project, save_project_atomic};
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectStore, Pronunciation};

fn fixture_capabilities() -> Capabilities {
    // No worker is started and no file is opened from these synthetic hashes.
    serde_json::from_value(serde_json::json!({
        "engine": "multilingual note integration fixture (not a voice)",
        "protocol": "ogd-worker/1", "native_version": "fixture-only",
        "test_double": true, "routes":["verdant-voice"],
        "max_active_jobs": 1, "max_phrase_seconds": 30.0,
        "native_score": {
            "inventory": verdant_backend::pinyin::recovered_syllables(),
            "english_vowel_proxies": ["a", "i", "u", "e", "o"],
            "continuous_f0_control": false, "coarticulation_recovered": false,
            "sub_syllable_timing": true, "phoneme_duration_control": true,
            "phoneme_frontend": {
                "schema_version": "verdant-phonemes/2",
                "alphabets": ["ipa", "xsampa", "vocaloid-xsampa"],
                "languages": {
                    "zh": "fixture Mandarin mapping",
                    "en": "fixture English mapping",
                    "ja": "fixture explicit cross-language approximation"
                }
            }
        },
        
        "files": {
            "token": {"available": true, "sha256": "11".repeat(32)},
            "model": {"available": true, "sha256": "22".repeat(32)},
            "vocoder": {"available": true, "sha256": "33".repeat(32)}
        }
    })).unwrap()
}

fn sung(input: &str, start: i64, duration: i64) -> Note {
    let (lyric, pronunciation) = parse_phonetic_input(input).unwrap().unwrap();
    let mut note = Note::new(start, duration, 60);
    note.lyric = lyric;
    note.pronunciation = pronunciation;
    note
}

fn project_with(notes: Vec<Note>) -> Project {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    project.tracks[0].notes = notes;
    project
}

fn single_phrase(result: &PhrasePlan) -> &Phrase {
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(result.blocked.is_empty());
    assert_eq!(result.phrases.len(), 1);
    &result.phrases[0]
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual:.17} != {expected:.17}");
}

fn group_seconds(phrase: &Phrase, group: usize) -> f64 {
    phrase.score.segments.iter()
        .filter(|segment| segment.phoneme.as_ref().unwrap().syllable_index == group)
        .map(|segment| segment.duration_seconds).sum()
}

fn cache_key(project: &Project, caps: &Capabilities, route: Route) -> String {
    let result = plan(project, &project.tracks[0].id, route, caps);
    render_digest(&payload_for(single_phrase(&result), route, 10008), route, caps)
}

fn set_pronunciation(store: &mut ProjectStore, pronunciation: Pronunciation) {
    let track = store.project().tracks[0].id.clone();
    let note_id = store.project().tracks[0].notes[0].id.clone();
    let batch = CommandBatch::single("Edit explicit pronunciation", Command::SetPronunciation {
        track, note_id, pronunciation,
    });
    assert!(store.apply(store.revision(), batch).unwrap().is_some());
}

#[test]
fn three_languages_preserve_phone_trace() {
    let caps = fixture_capabilities();
    let notes = vec![
        sung("上/[zh:ʂ a]", 18, 960),
        sung(r"red/[en/xsampa:r\ E d]", 978, 960),
        sung("ら/[ja:ɾ a]", 1938, 960),
    ];
    // Resolution has note-local diagnostic neighbors. They must not freeze
    // absent boundaries into a phrase made from several adjacent notes.
    for note in &notes {
        let NoteSupport::Phonemes(resolved) = note_support(note, &caps).unwrap() else { panic!("expected explicit phones") };
        assert_eq!(resolved.phones[0].context.features["L_Phone"], "0");
        assert_eq!(resolved.phones.last().unwrap().context.features["R_Phone"], "0");
    }
    let project = project_with(notes);
    let c0 = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let c1 = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let a = single_phrase(&c0);
    let b = single_phrase(&c1);
    assert_eq!(a.note_ids, project.tracks[0].notes.iter().map(|n| n.id.clone()).collect::<Vec<_>>());
    assert_eq!(a.score, b.score);
    close(a.start_seconds, 18.0 / 1920.0);
    close(a.seconds, 1.5);
    let expected = [
        ("ʂ", "ʂ", "ipa", "zh", "zh:sh", "59", 0),
        ("a", "a", "ipa", "zh", "zh:a", "3", 1),
        ("ɹ", r"r\", "xsampa", "en", "en:r", "224", 0),
        ("ɛ", "E", "xsampa", "en", "en:eh", "183", 1),
        ("d", "d", "xsampa", "en", "en:d", "204", 2),
        ("ɾ", "ɾ", "ipa", "ja", "en:dx", "207", 0),
        ("a", "a", "ipa", "ja", "zh:a", "3", 1),
    ];
    assert_eq!(a.score.segments.len(), expected.len());
    for (segment, (ipa, input, alphabet, language, native, id, source_index)) in a.score.segments.iter().zip(expected) {
        let trace = segment.phoneme.as_ref().unwrap();
        assert_eq!(trace.canonical_ipa, ipa);
        assert_eq!(trace.input_symbol, input);
        assert_eq!(trace.input_alphabet, alphabet);
        assert_eq!(trace.source_language, language);
        assert_eq!(trace.native_key, native);
        assert_eq!(trace.source_index, source_index);
        let context = segment.context.as_ref().unwrap();
        assert_eq!(context.features["C_Phone"], id);
        for name in ["L_Phone", "R_Phone", "L_Tone", "R_Tone", "L_Stress", "R_Stress"] {
            assert!(!context.features.contains_key(name), "premature {name} on {native}");
        }
        assert!(!context.assumptions.iter().any(|s| s.starts_with("L/R phone IDs use")));
        assert!(context.assumptions.iter().any(|s| s.contains("Phrase renderer supplies score-adjacent neighbors")));
        assert!(segment.duration_seconds >= 0.025 - 1e-12);
        assert_eq!(segment.midi_note, Some(60.0));
        assert!(segment.context_mode.is_none() && segment.vowel.is_none());
        assert_eq!(trace.approximation_reason.is_some(), language == "ja");
    }
    close(a.score.segments.iter().map(|s| s.duration_seconds + s.gap_after_seconds).sum(), a.seconds);
    assert!(c0.diagnostics.iter().any(|d| d.code == "W_PHONEME_APPROXIMATION"
        && d.note_id.as_ref() == Some(&project.tracks[0].notes[2].id)));
    let p0 = payload_for(a, Route::VerdantVoice, 10008);
    let p1 = payload_for(b, Route::VerdantVoice, 10008);
    assert_eq!(p0.score, p1.score);
    assert!(p0.control_track.is_none() && p1.control_track.is_none());
}

#[test]
fn missing_old_or_incomplete_phoneme_capabilities_block_the_phrase() {
    let note = sung("red/[en:ɹ ɛ d]", 0, 960);
    let project = project_with(vec![note.clone()]);
    for kind in ["missing", "old-schema", "missing-alphabet", "missing-language", "missing-native-score"] {
        let mut caps = fixture_capabilities();
        match kind {
            "missing" => caps.native_score.as_mut().unwrap().phoneme_frontend = None,
            "old-schema" => caps.native_score.as_mut().unwrap().phoneme_frontend.as_mut().unwrap().schema_version = "verdant-phonemes/1".into(),
            "missing-alphabet" => caps.native_score.as_mut().unwrap().phoneme_frontend.as_mut().unwrap().alphabets = vec!["xsampa".into()],
            "missing-language" => { caps.native_score.as_mut().unwrap().phoneme_frontend.as_mut().unwrap().languages.remove("en"); }
            "missing-native-score" => caps.native_score = None,
            _ => unreachable!(),
        }
        assert_eq!(note_support(&note, &caps).unwrap_err().code, "E_RENDER_CAPABILITY", "{kind}");
        for route in [Route::VerdantVoice] {
            let result = plan(&project, &project.tracks[0].id, route, &caps);
            assert!(result.phrases.is_empty(), "{kind}: {route:?}");
            assert_eq!(result.blocked.len(), 1, "{kind}");
            assert_eq!(result.blocked[0].note_ids, vec![note.id.clone()]);
            assert!(result.errors_for(&note.id).any(|d| d.code == "E_RENDER_CAPABILITY"), "{kind}");
        }
    }
}

#[test]
fn standard_xsampa_trill_and_approximant_are_distinct_at_preflight() {
    let caps = fixture_capabilities();
    let trill = sung("trill/[en/xsampa:r]", 0, 960);
    let canonical = verdant_backend::phonemes::canonicalize(&trill.pronunciation.phonemes, "en", "xsampa", None).unwrap();
    assert_eq!(canonical[0].symbol, "r");
    assert_eq!(note_support(&trill, &caps).unwrap_err().code, "E_UNSUPPORTED_PHONEME");
    let rejected = project_with(vec![trill]);
    assert!(plan(&rejected, &rejected.tracks[0].id, Route::VerdantVoice, &caps).phrases.is_empty());

    let accepted = project_with(vec![sung(r"approximant/[en/xsampa:r\]", 0, 960)]);
    let result = plan(&accepted, &accepted.tracks[0].id, Route::VerdantVoice, &caps);
    let segment = &single_phrase(&result).score.segments[0];
    assert_eq!(segment.phoneme.as_ref().unwrap().canonical_ipa, "ɹ");
    assert_eq!(segment.phoneme.as_ref().unwrap().input_symbol, r"r\");
    assert_eq!(segment.context.as_ref().unwrap().features["C_Phone"], "224");
    assert_eq!(segment.phoneme.as_ref().unwrap().support, "native");
}

#[test]
fn japanese_profile_requires_the_recorded_approximation_permission() {
    let caps = fixture_capabilities();
    for input in ["さ/[ja:s a]", "さ/[ja~:s a]"] {
        let note = sung(input, 0, 960);
        assert!(note.pronunciation.allow_approximation);
        let project = project_with(vec![note]);
        let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        assert!(single_phrase(&result).score.segments.iter().all(|s| {
            let trace = s.phoneme.as_ref().unwrap();
            trace.source_language == "ja" && trace.support == "approximate" && trace.approximation_reason.is_some()
        }));
        assert!(result.diagnostics.iter().any(|d| d.code == "W_PHONEME_APPROXIMATION"));
    }
    let strict = sung("さ/[ja!:s a]", 0, 960);
    assert!(!strict.pronunciation.allow_approximation);
    assert_eq!(note_support(&strict, &caps).unwrap_err().code, "E_UNSUPPORTED_PHONEME");
    let strict_kana = sung("さ/[ja!/kana:さ]", 0, 960);
    assert!(!strict_kana.pronunciation.allow_approximation);
    assert_eq!(note_support(&strict_kana, &caps).unwrap_err().code, "E_UNSUPPORTED_PHONEME");
}

#[test]
fn duration_overrides_address_expanded_native_phones_not_ipa_token_count() {
    let caps = fixture_capabilities();
    let note = sung("near/[en~:ɪə]", 0, 960);
    assert_eq!(note.pronunciation.phonemes.len(), 1);
    let mut store = ProjectStore::new(project_with(vec![note]));
    let mut pronunciation = store.project().tracks[0].notes[0].pronunciation.clone();
    pronunciation.phoneme_durations_permille = Some(vec![400, 600]);
    set_pronunciation(&mut store, pronunciation.clone());
    let project = store.snapshot();
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let phrase = single_phrase(&result);
    assert_eq!(phrase.score.segments.len(), 2);
    close(phrase.score.segments[0].duration_seconds, 0.2);
    close(phrase.score.segments[1].duration_seconds, 0.3);
    for (segment, native) in phrase.score.segments.iter().zip(["en:ih", "en:ax"]) {
        let trace = segment.phoneme.as_ref().unwrap();
        assert_eq!(trace.canonical_ipa, "ɪə");
        assert_eq!(trace.source_index, 0);
        assert_eq!(trace.native_key, native);
    }
    // A valid document can defer acoustic expansion to preflight; one weight
    // for this one written IPA token must then be rejected, never duplicated.
    pronunciation.phoneme_durations_permille = Some(vec![1000]);
    set_pronunciation(&mut store, pronunciation);
    let rejected = plan(store.project(), &store.project().tracks[0].id, Route::VerdantVoice, &caps);
    assert!(rejected.phrases.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.code == "E_PHONEME_TIMING"));
}

#[test]
fn kana_mora_budgets_and_trace_survive_note_expansion() {
    let caps = fixture_capabilities();
    for (alphabet, expected_weights, expected_groups, expected_ipa) in [
        ("kana-common", vec![1, 3], vec![0.2, 0.6], vec!["ɡ", "a", "kː", "oː"]),
        ("kana", vec![1, 2, 1], vec![0.2, 0.4, 0.2], vec!["ɡ", "a", "kː", "o", "ɯ"]),
    ] {
        let note = sung(&format!("学校/[ja/{alphabet}:がっこう]"), 18, 1536);
        assert_eq!(note.pronunciation.mora_weights, Some(expected_weights));
        let provenance = note.pronunciation.reading_provenance.as_ref().unwrap();
        assert_eq!(provenance["schema_version"], "verdant-kana/1");
        assert_eq!(provenance["reading"]["original_input"], "がっこう");
        let project = project_with(vec![note]);
        let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        let phrase = single_phrase(&result);
        let actual: Vec<_> = phrase.score.segments.iter().map(|s| s.phoneme.as_ref().unwrap().canonical_ipa.as_str()).collect();
        assert_eq!(actual, expected_ipa);
        for (group, seconds) in expected_groups.into_iter().enumerate() { close(group_seconds(phrase, group), seconds); }
        for segment in &phrase.score.segments {
            let trace = segment.phoneme.as_ref().unwrap();
            assert_eq!(trace.input_alphabet, "kana");
            assert_eq!(trace.source_language, "ja");
            assert_eq!(trace.source_index, trace.syllable_index);
            assert_eq!(trace.input_symbol, project.tracks[0].notes[0].pronunciation.reading_provenance.as_ref().unwrap()
                ["reading"]["moras"][trace.syllable_index]["kana"].as_str().unwrap());
            assert!(segment.duration_seconds >= 0.025 - 1e-12);
        }
        close(phrase.score.segments.iter().map(|s| s.duration_seconds).sum(), 0.8);
    }

    let mut note = sung("学校/[ja/kana-common:がっこう]", 0, 1536);
    note.pronunciation.phoneme_durations_permille = Some(vec![250; 4]);
    let project = project_with(vec![note]);
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let phrase = single_phrase(&result);
    // Explicit expanded-phone overrides take precedence over default mora rhythm.
    close(group_seconds(phrase, 0), 0.4);
    close(group_seconds(phrase, 1), 0.4);
}

#[test]
fn kana_nasal_realization_reaches_voice_wire_without_changing_explicit_ipa() {
    let caps = fixture_capabilities();
    for route in [Route::VerdantVoice] {
        let project = project_with(vec![sung("散歩/[ja/kana:さんぽ]", 0, 1152)]);
        let result = plan(&project, &project.tracks[0].id, route, &caps);
        let phrase = single_phrase(&result);
        let nasal = phrase.score.segments.iter().find(|segment| segment.phoneme.as_ref().is_some_and(|p| p.canonical_ipa == "ɴ")).unwrap();
        let trace = nasal.phoneme.as_ref().unwrap();
        assert_eq!(trace.native_key, "zh:m");
        assert_eq!(trace.source_language, "ja");
        assert_eq!(trace.input_symbol, "ん");
        assert_eq!(trace.support, "approximate");
        assert_eq!(trace.realization.as_ref().unwrap().target_ipa, "m");
        close(nasal.duration_seconds, 0.2);
        let wire = serde_json::to_value(trace).unwrap();
        assert_eq!(wire["canonical_ipa"], "ɴ");
        assert_eq!(wire["realization"]["schema_version"], "verdant-japanese-realization/1");
    }
    for input in ["N/[ja:ɴ p o]", "区切り/[ja/kana:さん ぽ]"] {
        let project = project_with(vec![sung(input, 0, 1152)]);
        let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        let phrase = single_phrase(&result);
        let trace = phrase.score.segments.iter().find_map(|s| s.phoneme.as_ref().filter(|p| p.canonical_ipa == "ɴ")).unwrap();
        assert_eq!(trace.native_key, "en:ng");
        assert!(trace.realization.is_none());
    }
    let mut edited = sung("散歩/[ja/kana:さんぽ]", 0, 1152);
    *edited.pronunciation.phonemes.iter_mut().find(|symbol| *symbol == "p").unwrap() = "k".into();
    let project = project_with(vec![edited]);
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let phrase = single_phrase(&result);
    assert!(phrase.score.segments.iter().all(|s| s.phoneme.as_ref().unwrap().realization.is_none()), "edited canonical units must invalidate the old kana allophone rule");
    assert!(phrase.score.segments.iter().all(|s| s.phoneme.as_ref().unwrap().input_alphabet == "ipa"), "stale kana provenance must not relabel edited phonetic input");
}

#[test]
fn unknown_symbols_and_contradictory_boundaries_never_emit_a_score() {
    let caps = fixture_capabilities();
    for phones in [". ə", "ə .", "ə . . i", "ˈ ˌi", "ə ˈ", "NOT_A_PHONE"] {
        let note = sung(&format!("word/[en:{phones}]"), 0, 960);
        let project = project_with(vec![note]);
        let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        assert!(result.phrases.is_empty(), "{phones}");
        assert_eq!(result.blocked.len(), 1, "{phones}");
        assert!(result.diagnostics.iter().any(|d| d.code == "E_UNSUPPORTED_PHONEME"), "{phones}");
    }
    for input in ["word/[unknown:a]", "word/[en/unknown:a]", "word/[en:a", "学校/[ja/kana:学校]",
        "学校/[ja/kana:っ]", "学校/[ja/kana:っあ]", "学校/[ja/kana:ーが]"] {
        assert!(parse_phonetic_input(input).is_err(), "{input}");
    }
    let mut mismatched = sung("学校/[ja/kana-common:がっこう]", 0, 1536);
    mismatched.pronunciation.mora_weights = Some(vec![1, 2, 1]); // Three groups for two explicit groups.
    let project = project_with(vec![mismatched]);
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(result.phrases.is_empty());
    assert!(result.diagnostics.iter().any(|d| d.code == "E_PHONEME_TIMING"));
}

#[test]
fn twenty_five_millisecond_floor_and_authored_gaps_preserve_total_duration() {
    let caps = fixture_capabilities();
    // At 120 BPM / PPQ 960, 48 ticks are exactly the 25 ms admission boundary.
    let project = project_with(vec![sung("啊/[zh:a]", 18, 48), sung("啊/[zh:a]", 114, 48)]);
    let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    let phrase = single_phrase(&result);
    assert_eq!(phrase.score.segments.len(), 2);
    close(phrase.score.segments[0].duration_seconds, 0.025);
    close(phrase.score.segments[0].gap_after_seconds, 0.025);
    close(phrase.score.segments[1].duration_seconds, 0.025);
    close(phrase.score.segments.iter().map(|s| s.duration_seconds + s.gap_after_seconds).sum(), phrase.seconds);
    close(phrase.seconds, 0.075);

    let too_short = project_with(vec![sung("啊/[zh:a]", 18, 47)]);
    let rejected = plan(&too_short, &too_short.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(rejected.phrases.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.code == "E_NOTE_TOO_SHORT"));
    for (ticks, weights, should_accept) in [(96, vec![500, 500], true), (95, vec![500, 500], false), (96, vec![499, 501], false)] {
        let mut note = sung("near/[en~:ɪə]", 18, ticks);
        note.pronunciation.phoneme_durations_permille = Some(weights);
        let project = project_with(vec![note]);
        let result = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        if should_accept {
            let phrase = single_phrase(&result);
            for segment in &phrase.score.segments { close(segment.duration_seconds, 0.025); }
            close(phrase.seconds, 0.05);
        } else {
            assert!(result.phrases.is_empty(), "{ticks} ticks");
            assert!(result.diagnostics.iter().any(|d| d.code == "E_PHONEME_TIMING"));
        }
    }
}

struct SavedProject(PathBuf);
impl SavedProject {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!("verdant-multilingual-notes-{}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&directory).unwrap();
        Self(directory.join("pronunciation.verp"))
    }
}
impl Drop for SavedProject {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(self.0.parent().unwrap()); }
}

#[test]
fn phonetic_project_save_undo_and_redo_preserve_provenance_and_cache_identity() {
    let caps = fixture_capabilities();
    let mut note = sung("学校/[ja/kana-common:がっこう]", 0, 1536);
    note.pronunciation.reading_provenance.as_mut().unwrap()["fixture_extension"] = serde_json::json!({"kept": true});
    let mut store = ProjectStore::new(project_with(vec![note]));
    let initial = store.snapshot();
    let before = cache_key(&initial, &caps, Route::VerdantVoice);
    let mut edited = initial.tracks[0].notes[0].pronunciation.clone();
    edited.phoneme_durations_permille = Some(vec![250; 4]);
    set_pronunciation(&mut store, edited.clone());
    assert_eq!(store.undo_depth(), 1);
    assert!(store.is_dirty());
    let after = cache_key(store.project(), &caps, Route::VerdantVoice);
    assert_ne!(before, after);
    assert!(initial.tracks[0].notes[0].pronunciation.phoneme_durations_permille.is_none());

    let saved = SavedProject::new();
    save_project_atomic(&saved.0, &store.project_for_save()).unwrap();
    store.mark_saved();
    assert!(!store.is_dirty());
    let (loaded, report) = load_project(&saved.0).unwrap();
    assert!(report.preserved_unknown_keys.is_empty());
    assert_eq!(loaded.tracks[0].notes[0].pronunciation, edited);
    assert_eq!(loaded.tracks[0].notes[0].pronunciation.reading_provenance.as_ref().unwrap()["fixture_extension"]["kept"], true);
    let reopened = ProjectStore::new(loaded);
    assert_eq!(cache_key(reopened.project(), &caps, Route::VerdantVoice), after);
    let edit_revision = store.revision();
    assert!(store.undo().unwrap().is_some());
    assert!(store.revision() > edit_revision);
    assert_eq!(store.project().tracks[0].notes[0].pronunciation, initial.tracks[0].notes[0].pronunciation);
    assert_eq!(cache_key(store.project(), &caps, Route::VerdantVoice), before);
    assert!(store.redo().unwrap().is_some());
    assert_eq!(store.project().tracks[0].notes[0].pronunciation, edited);
    assert_eq!(cache_key(store.project(), &caps, Route::VerdantVoice), after);
    assert!(!store.is_dirty());
}
