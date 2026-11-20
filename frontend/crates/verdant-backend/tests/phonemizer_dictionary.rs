//! Ordinary persisted dictionary and singer/language selection, without a model
//! or worker process. These exercise the resolver used by the score adapter.
use verdant_backend::phonemizer::{
    self, EXPLICIT_ONLY, EXTENSION_KEY, JAPANESE_KANA, MANDARIN,
    LyricEntry, PhonemizerSettings, ReadingOrigin, ResolvedTrack,
};
use verdant_domain::{
    Command, CommandBatch, HANZI_PINYIN_PROFILE, Note, Project, ProjectStore,
    Pronunciation, PronunciationSource, Track,
};

fn source_track(singer: Option<&str>) -> Track {
    let mut track = Track::new("词典与语言", "teal");
    track.voice_ref = singer.map(str::to_string);
    for (index, lyric) in ["海", "か"].iter().enumerate() {
        let mut note = Note::new(index as i64 * 960, 960, 60);
        note.lyric = (*lyric).into();
        track.notes.push(note);
    }
    track.refresh_index();
    track
}

fn save_settings(store: &mut ProjectStore, settings: &PhonemizerSettings) {
    store.apply(store.revision(), CommandBatch::single("读音器与用户词典", Command::SetProjectExtension {
        key: EXTENSION_KEY.into(), value: Some(phonemizer::settings_to_extension(settings).unwrap()),
    })).unwrap();
}

fn resolve(project: &Project, index: usize) -> ResolvedTrack {
    let settings = phonemizer::settings_from_project(project).unwrap();
    phonemizer::resolve_track(&project.tracks[index], &settings)
}

#[test]
fn dictionary_override_edit_delete_and_undo_restore_the_finite_builtin() {
    let mut project = Project::new_empty();
    let mut track = source_track(None);
    track.notes.truncate(1);
    // Include an already stored builtin as well as a note awaiting the frontend.
    let mut stored = Note::new(960, 960, 60);
    stored.lyric = "海".into();
    stored.pronunciation = Pronunciation {
        input_text: Some("海".into()), pinyin: "hai3".into(),
        source: PronunciationSource::ExplicitMap, frontend_profile: HANZI_PINYIN_PROFILE.into(),
        ..Default::default()
    };
    track.notes.push(stored);
    track.refresh_index();
    project.tracks[0] = track;
    let original_notes = project.tracks[0].notes.clone();
    let mut store = ProjectStore::new(project);
    let before = resolve(store.project(), 0);
    assert!(before.diagnostics.is_empty());
    assert!(before.notes.values().all(|note| note.pronunciation.pinyin == "hai3"));

    let mut settings = PhonemizerSettings::default();
    settings.dictionary = phonemizer::import_dictionary(r#"{
        "schema_version":"verdant-pronunciation-dictionary/1",
        "lyrics":{"海":{"reading":{"pinyin":"hai4","source":"manual"}}}
    }"#).unwrap();
    save_settings(&mut store, &settings);
    assert_eq!(store.undo_depth(), 1);
    // Resolve after a genuine project serialization, not a private draft map.
    let restored: Project = serde_json::from_str(&serde_json::to_string(store.project()).unwrap()).unwrap();
    let overridden = resolve(&restored, 0);
    assert!(overridden.diagnostics.is_empty());
    for note in overridden.notes.values() {
        assert_eq!(note.pronunciation.pinyin, "hai4");
        assert_eq!(note.origin, ReadingOrigin::UserLyric { lyric: "海".into() });
    }
    assert_eq!(phonemizer::import_dictionary(&phonemizer::export_dictionary(&settings.dictionary).unwrap()).unwrap(), settings.dictionary);

    settings.dictionary.lyrics.get_mut("海").unwrap().reading.pinyin = "hai2".into();
    save_settings(&mut store, &settings);
    assert!(resolve(store.project(), 0).notes.values().all(|note| note.pronunciation.pinyin == "hai2"));
    settings.dictionary.lyrics.remove("海");
    save_settings(&mut store, &settings);
    let deleted = resolve(store.project(), 0);
    assert!(deleted.diagnostics.is_empty());
    assert_eq!(deleted.notes[&original_notes[0].id].origin, ReadingOrigin::Builtin);
    assert_eq!(deleted.notes[&original_notes[1].id].origin, ReadingOrigin::ExistingBuiltin);
    assert!(deleted.notes.values().all(|note| note.pronunciation.pinyin == "hai3"));
    store.undo().unwrap();
    assert!(resolve(store.project(), 0).notes.values().all(|note| note.pronunciation.pinyin == "hai2"));
    assert_eq!(store.project().tracks[0].notes, original_notes);
}

#[test]
fn imported_dictionary_rejects_escaped_duplicate_lyric_keys_atomically() {
    let mut settings = PhonemizerSettings::default();
    settings.dictionary = phonemizer::import_dictionary(r#"{
        "schema_version":"verdant-pronunciation-dictionary/1",
        "lyrics":{"海":{"reading":{"pinyin":"hai4","source":"manual"}}}
    }"#).unwrap();
    let original = settings.clone();
    let duplicate = r#"{
        "schema_version":"verdant-pronunciation-dictionary/1",
        "lyrics":{
            "海":{"reading":{"pinyin":"hai3","source":"manual"}},
            "\u6d77":{"reading":{"pinyin":"hai2","source":"manual"}}
        }
    }"#;
    // This is the editor's atomic import boundary: replace only on success.
    let result = phonemizer::import_dictionary(duplicate).map(|dictionary| settings.dictionary = dictionary);
    assert!(result.unwrap_err().contains("duplicate JSON key"));
    assert_eq!(settings, original);
}

#[test]
fn persisted_singer_phonemizer_applies_to_matching_tracks_and_clear_restores_inheritance() {
    let mut project = Project::new_empty();
    project.tracks = vec![source_track(Some("singer-ja")), source_track(Some("singer-ja")), source_track(Some("other"))];
    let mut settings = PhonemizerSettings::default();
    settings.singer_selections.insert("singer-ja".into(), JAPANESE_KANA.into());
    project.extensions.insert(EXTENSION_KEY.into(), phonemizer::settings_to_extension(&settings).unwrap());
    let serialized = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&serialized).unwrap();
    for index in [0, 1] {
        let result = resolve(&restored, index);
        let track = &restored.tracks[index];
        assert!(!result.notes.contains_key(&track.notes[0].id), "kanji must not fall back to Mandarin");
        let kana = &result.notes[&track.notes[1].id].pronunciation;
        assert_eq!(kana.language.as_deref(), Some("ja"));
        assert!(kana.pinyin.is_empty());
        assert_eq!(kana.phonemes, ["k", "a"]);
    }
    let other = resolve(&restored, 2);
    assert_eq!(other.notes[&restored.tracks[2].notes[0].id].pronunciation.pinyin, "hai3");
    assert_eq!(other.notes[&restored.tracks[2].notes[1].id].pronunciation.language.as_deref(), Some("ja"));

    let mut store = ProjectStore::new(restored);
    settings.singer_selections.remove("singer-ja");
    save_settings(&mut store, &settings);
    assert_eq!(resolve(store.project(), 0).notes[&store.project().tracks[0].notes[0].id].pronunciation.pinyin, "hai3");
    store.undo().unwrap();
    assert!(!resolve(store.project(), 0).notes.contains_key(&store.project().tracks[0].notes[0].id));
}

#[test]
fn explicit_note_language_and_dictionary_override_track_frontend() {
    let mut track = source_track(Some("singer"));
    track.unknown.insert("voice_language".into(), serde_json::json!({
        "schema":"verdant-track-language/1", "language":"ja", "support":"approximate"
    }));
    track.notes[0].pronunciation = Pronunciation {
        pinyin: "hai3".into(), language: Some("zh".into()),
        source: PronunciationSource::ExplicitMap, frontend_profile: HANZI_PINYIN_PROFILE.into(),
        ..Default::default()
    };
    let mut settings = PhonemizerSettings::default();
    settings.singer_selections.insert("singer".into(), MANDARIN.into());
    let result = phonemizer::resolve_track(&track, &settings);
    assert_eq!(result.notes[&track.notes[0].id].pronunciation.pinyin, "hai3");
    assert_eq!(result.notes[&track.notes[1].id].pronunciation.language.as_deref(), Some("ja"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

    // Supplying an actual Japanese reading is explicit and wins over frontends.
    let (_, reading) = verdant_backend::reading::parse_phonetic_input("海/[ja/kana:うみ]").unwrap().unwrap();
    settings.dictionary.lyrics.insert("海".into(), LyricEntry { reading, ..Default::default() });
    let resolved = phonemizer::resolve_track(&track, &settings);
    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.notes[&track.notes[0].id].pronunciation.language.as_deref(), Some("ja"));
    assert!(resolved.notes[&track.notes[0].id].pronunciation.pinyin.is_empty());

    track.unknown.remove("voice_language");
    assert_eq!(settings.selected_for_track(&track).unwrap(), MANDARIN);
    settings.singer_selections.insert("singer".into(), EXPLICIT_ONLY.into());
    assert_eq!(settings.selected_for_track(&track).unwrap(), EXPLICIT_ONLY);
    track.unknown.insert("voice_language".into(), serde_json::json!({
        "schema":"verdant-track-language/1", "language":"unknown", "support":"native"
    }));
    let invalid = phonemizer::resolve_track(&track, &settings);
    assert!(invalid.notes.is_empty());
    assert_eq!(invalid.diagnostics[0].code, "E_PHONEMIZER_CONFIG");
}
