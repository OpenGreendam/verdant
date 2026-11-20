//! The optional acoustic layer must survive real planning without rewriting
//! the requested language, IPA, timing, F0, or selected native model path.
#[path = "support/original.rs"]
mod original;

use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::{plan, Route};
use verdant_backend::wire::Capabilities;
use verdant_domain::{Note, PhoneticCorrectionSettings, Project, Pronunciation, PronunciationSource, SynthesisSettings};

fn project() -> Project {
    let mut p = original::project();
    let mut n = Note::new(0, 960, 69);
    n.pronunciation = Pronunciation {
        language: Some("ja".into()), alphabet: Some("ipa".into()), phonemes: vec!["ɾ".into(), "ɯ".into()],
        allow_approximation: true, source: PronunciationSource::Manual,
        ..Default::default()
    };
    p.tracks[0].notes.push(n);
    p.tracks[0].refresh_index();
    p.render_preferences.synthesis = Some(SynthesisSettings { wideband: None, ..SynthesisSettings::native_v10() });
    p
}

fn capabilities() -> Capabilities {
    original::capabilities(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["C0"], "max_phrase_seconds":30,
        "native_score":{"vowel_balance_db_range":[0,18], "continuous_f0_control":true,
            "wideband":{"routes":["C0"],"methods":["matched-balanced"],"presence_db_range":[0,24],"sample_rate":44100},
            "phonetic_correction":{"routes":["C0"],"methods":["articulatory-v1"],"strength_range":[0,1]},
            "phoneme_frontend":{"schema_version":"verdant-phonemes/2","alphabets":["ipa"],"languages":{"ja":"approximate"}}},
    }))
}

#[test]
fn correction_changes_cache_identity_but_keeps_target_phonemes_f0_and_timing() {
    let mut p = project();
    let caps = capabilities();
    let track = p.tracks[0].id.clone();
    for route in [Route::C0] {
        p.render_preferences.synthesis.as_mut().unwrap().phonetic_correction = None;
        let before = plan(&p, &track, route, &caps);
        assert!(!before.has_errors(), "{:?}", before.diagnostics);
        let base = payload_for(&before.phrases[0], route, 10008);
        p.render_preferences.synthesis.as_mut().unwrap().phonetic_correction = Some(PhoneticCorrectionSettings::articulatory_v1());
        let after = plan(&p, &track, route, &caps);
        assert!(!after.has_errors(), "{:?}", after.diagnostics);
        let changed = payload_for(&after.phrases[0], route, 10008);
        assert_eq!(base.score.as_ref().unwrap().segments, changed.score.as_ref().unwrap().segments);
        assert_eq!(base.control_track, changed.control_track);
        assert_eq!(base.native_seed, changed.native_seed);
        assert_ne!(render_digest(&base, route, &caps), render_digest(&changed, route, &caps));
        let value = serde_json::to_value(&changed).unwrap();
        assert_eq!(value["score"]["synthesis"]["phonetic_correction"], json!({"method":"articulatory-v1","strength":1.0}));
        assert_eq!(value["score"]["segments"][0]["phoneme"]["canonical_ipa"], "ɾ");
        assert_eq!(value["score"]["segments"][0]["phoneme"]["native_key"], "en:dx");
        assert_eq!(value["score"]["segments"][1]["phoneme"]["canonical_ipa"], "ɯ");
        assert_eq!(value["score"]["segments"][1]["phoneme"]["source_language"], "ja");
    }
}

#[test]
fn old_or_incompatible_worker_is_blocked_only_when_correction_selected() {
    let mut p = project();
    let track = p.tracks[0].id.clone();
    let mut caps = capabilities();
    p.render_preferences.synthesis.as_mut().unwrap().phonetic_correction = Some(PhoneticCorrectionSettings::articulatory_v1());
    let original = caps.native_score.as_ref().unwrap().phonetic_correction.clone();
    for case in 0..4 {
        caps.native_score.as_mut().unwrap().phonetic_correction = original.clone();
        let profile = &mut caps.native_score.as_mut().unwrap().phonetic_correction;
        match case {
            0 => *profile = None,
            1 => profile.as_mut().unwrap().methods.clear(),
            2 => profile.as_mut().unwrap().routes = vec!["C1".into()],
            _ => profile.as_mut().unwrap().strength_range = Some([0.0, 0.5]),
        }
        let blocked = plan(&p, &track, Route::C0, &caps);
        assert!(blocked.phrases.is_empty());
        assert!(blocked.diagnostics.iter().any(|d| d.code == "E_SYNTHESIS_CAPABILITY"));
    }
    p.render_preferences.synthesis.as_mut().unwrap().phonetic_correction = None;
    assert!(!plan(&p, &track, Route::C0, &caps).has_errors());
}
