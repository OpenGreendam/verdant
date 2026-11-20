//! Output-profile negotiation does not change the learned score or old routes.
use serde_json::{Value, json};
use verdant_backend::{Capabilities, Route, adapter::{payload_for, render_digest}, wire::VoiceMusic};
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource};

fn declared_music() -> Value {
    json!({"methods":["verdant-voice-music-v1"],"routes":["verdant-voice"],
        "sample_rates":[44100,48000],"default_sample_rate":48000})
}

fn capabilities(music: Option<Value>) -> Capabilities {
    let mut caps: Capabilities = serde_json::from_value(json!({"test_double":true,
        "routes":["verdant-voice","C0","C1"],"max_phrase_seconds":30.,
        "native_score":{"english_vowel_proxies":["a"],"explicit_f0_hz_range":[35.,2000.],
            "continuous_f0_control":true,"vowel_balance_db_range":[0.,18.],
            "phoneme_frontend":{"schema_version":"verdant-phonemes/2","alphabets":["ipa"],"languages":{"ja":"approximate"}}},
        "neural":{"provided_control_track":true}})).unwrap();
    if let Some(music) = music { caps.other.insert("voice_package".into(), json!({"sample_rate":48000,"hop_samples":240,"music":music})); }
    caps
}

fn project() -> Project {
    let mut project = Project::new_empty();
    project.render_preferences.route = "verdant-voice".into();
    let mut note = Note::new(0, 960, 64);
    note.lyric = "啊".into();
    note.pronunciation = Pronunciation { pinyin:"a".into(), source:PronunciationSource::Manual,
        frontend_profile:"english-vowel-proxy-native-0.2".into(), ..Default::default() };
    project.tracks[0].notes.push(note);
    project.tracks[0].refresh_index();
    project
}

#[test]
fn an_advertised_music_profile_changes_only_the_voice_output_request() {
    let mut project = project();
    // Saved legacy sound settings remain available for route undo and C0/C1.
    project.render_preferences.synthesis = Some(verdant_domain::SynthesisSettings::native_v10());
    let before = serde_json::to_value(&project).unwrap();
    let old_caps = capabilities(None);
    let old = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &old_caps);
    assert!(!old.has_errors(), "{:?}", old.diagnostics);
    let raw = payload_for(&old.phrases[0], Route::VerdantVoice, 10008);
    assert!(serde_json::to_value(&raw).unwrap().get("voice_music").is_none());

    let caps = capabilities(Some(declared_music()));
    let planned = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
    let output = payload_for(&planned.phrases[0], Route::VerdantVoice, 10008);
    assert_eq!(output.voice_music, Some(VoiceMusic { method:"verdant-voice-music-v1".into(), sample_rate:48000 }));
    let mut without_profile = output.clone();
    without_profile.voice_music = None;
    assert_eq!(without_profile, raw, "music negotiation changed the native learned score or authored controls");
    assert!(output.score.as_ref().unwrap().synthesis.is_none());
    assert!(output.wideband.is_none());
    assert_eq!(serde_json::to_value(&project).unwrap(), before, "planning mutated the saved project");
    assert_ne!(render_digest(&output, Route::VerdantVoice, &caps), render_digest(&raw, Route::VerdantVoice, &caps));
    let mut cd_rate = output.clone();
    cd_rate.voice_music.as_mut().unwrap().sample_rate = 44100;
    assert_ne!(render_digest(&output, Route::VerdantVoice, &caps), render_digest(&cd_rate, Route::VerdantVoice, &caps));
}

#[test]
fn incomplete_or_unrecognized_music_capabilities_keep_the_legacy_request() {
    let mut declarations = vec![json!(null), json!({})];
    for (key, value) in [("methods", json!(["future-music"])), ("routes", json!(["C0"])),
        ("sample_rates", json!([16000,44100])), ("default_sample_rate", json!(null)),
        ("default_sample_rate", json!(44100)), ("sample_rates", json!(["48000"]))] {
        let mut declaration = declared_music();
        declaration[key] = value;
        declarations.push(declaration);
    }
    let project = project();
    let baseline = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &capabilities(None));
    let baseline = payload_for(&baseline.phrases[0], Route::VerdantVoice, 10008);
    for declaration in declarations {
        let caps = capabilities(Some(declaration.clone()));
        assert!(caps.default_voice_music().is_none(), "{declaration}");
        let planned = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
        assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
        assert_eq!(payload_for(&planned.phrases[0], Route::VerdantVoice, 10008), baseline);
    }
}

#[test]
fn retired_routes_cannot_inherit_voice_music() {
    let mut project = project();
    let caps = capabilities(Some(declared_music()));
    for stored in ["C0", "C1", "A"] {
        project.tracks[0].render_route = Some(stored.into());
        let plan = verdant_backend::adapter::plan_desktop(&project, &project.tracks[0].id, &caps);
        assert!(plan.diagnostics.iter().any(|error| error.code == "E_RENDER_ROUTE"));
        assert!(plan.phrases.is_empty());
    }
}

#[test]
fn explicit_music_configuration_keeps_a_small_versioned_contract() {
    for sample_rate in [44100,48000] {
        VoiceMusic { method:"verdant-voice-music-v1".into(), sample_rate }.validate().unwrap();
    }
    for sample_rate in [0,16000,22050,96000] {
        assert!(VoiceMusic { method:"verdant-voice-music-v1".into(), sample_rate }.validate().is_err());
    }
    assert!(VoiceMusic { method:"articulatory-v1".into(), sample_rate:48000 }.validate().is_err());
    assert!(serde_json::from_value::<VoiceMusic>(json!({"method":"verdant-voice-music-v1","sample_rate":48000,"strength":1.})).is_err());
}

#[test]
fn music_note_ownership_preserves_palatal_groups_and_continuations_without_retiming() {
    let mut project = Project::new_empty();
    project.render_preferences.route = "verdant-voice".into();
    let (lyric, pronunciation) = verdant_backend::reading::parse_phonetic_input("きゃ/[ja/kana:きゃ]").unwrap().unwrap();
    for index in 0..3 {
        let mut note = Note::new(index * 960, 960, 60);
        note.id = verdant_domain::NoteId(format!("owner-{index}"));
        note.lyric = lyric.clone();
        note.pronunciation = pronunciation.clone();
        if index == 2 { note.set_continuation(true); }
        project.tracks[0].notes.push(note);
    }
    project.tracks[0].refresh_index();
    let saved = serde_json::to_value(&project).unwrap();
    let old = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &capabilities(None));
    let caps = capabilities(Some(declared_music()));
    let new = verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(!old.has_errors(), "{:?}", old.diagnostics);
    assert!(!new.has_errors(), "{:?}", new.diagnostics);
    let raw = payload_for(&old.phrases[0], Route::VerdantVoice, 10008);
    let output = payload_for(&new.phrases[0], Route::VerdantVoice, 10008);
    let score = output.score.as_ref().unwrap();
    assert!(score.phoneme_timing.is_none(), "note ownership must not enable vowel-anchor scheduling");
    let owners: Vec<_> = score.segments.iter().map(|segment| segment.phoneme.as_ref().unwrap().source_note_id.as_deref()).collect();
    assert_eq!(owners, [Some("owner-0"),Some("owner-0"),Some("owner-0"),
        Some("owner-1"),Some("owner-1"),Some("owner-1"),Some("owner-1")]);
    assert_eq!(score.segments[6].continuation_of, Some(5));
    let mut without_metadata = output.clone();
    without_metadata.voice_music = None;
    for segment in &mut std::sync::Arc::make_mut(without_metadata.score.as_mut().unwrap()).segments {
        segment.phoneme.as_mut().unwrap().source_note_id = None;
    }
    assert_eq!(without_metadata, raw, "owner metadata changed F0, duration, native mapping or source contexts");
    assert_eq!(serde_json::to_value(&project).unwrap(), saved);
}
