#[path = "support/original.rs"]
mod original;

use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::wire::Capabilities;
use verdant_backend::{plan, Route};
use verdant_domain::{ModelDynamicsSettings, Note, Project, Pronunciation, PronunciationSource, SynthesisSettings, Vibrato, VocalTractSettings};

fn project() -> Project {
    let mut p = original::project();
    let mut n = Note::new(0, 960, 69);
    n.pronunciation = Pronunciation {
        pinyin: "a".into(), source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(), ..Default::default()
    };
    n.vibrato = Some(Vibrato::DEFAULT);
    p.tracks[0].notes.push(n);
    p.tracks[0].refresh_index();
    p.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(1.125));
    p
}

fn capabilities() -> Capabilities {
    original::capabilities(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["C0"], "max_phrase_seconds":30,
        "native_score":{"english_vowel_proxies":["a"], "vowel_balance_db_range":[0,18], "continuous_f0_control":true,
            "wideband":{"routes":["C0"],"methods":["matched-balanced"],"presence_db_range":[0,24],"sample_rate":44100},
            "vocal_tract":{"routes":["C0"],"methods":["bilinear-v1"],"ratio_range":[0.8,1.25]},
            "model_dynamics":{"routes":["C0"],"methods":["voiced-envelope-v1"],
                "minimum_strength":0,"maximum_strength":1,"minimum_attenuation_db":0,"maximum_attenuation_db":12}},
    }))
}

#[test]
fn dynamics_changes_only_its_payload_and_cache_field_in_the_unlocked_original_and_all_synthesis_modes() {
    for synthesis in [None, Some(SynthesisSettings::default()), Some(SynthesisSettings { wideband: None, ..SynthesisSettings::native_v10() })] {
        for route in [Route::C0] {
            let mut p = project();
            p.render_preferences.synthesis = synthesis.clone();
            let track = p.tracks[0].id.clone();
            let mut caps = capabilities();
            if synthesis.is_none() { caps.native_score.as_mut().unwrap().vowel_balance_db_range = None; }
            let before = plan(&p, &track, route, &caps);
            assert!(!before.has_errors(), "{:?}", before.diagnostics);
            let base = payload_for(&before.phrases[0], route, 6789);
            assert!(base.control_track.is_some());
            assert!(serde_json::to_value(&base).unwrap().get("model_dynamics").is_none());
            let mut digests = std::collections::BTreeSet::from([render_digest(&base, route, &caps)]);
            for (strength, attenuation) in [(0.0, 6.0), (0.25, 0.0), (0.25, 6.0), (0.5, 6.0), (0.25, 4.0), (1.0, 12.0)] {
                let mut d = ModelDynamicsSettings::voiced_envelope_v1();
                d.strength = strength;
                d.max_attenuation_db = attenuation;
                d.unknown.insert("future_annotation".into(), json!("retained only in project"));
                p.render_preferences.model_dynamics = Some(d);
                let after = plan(&p, &track, route, &caps);
                assert!(!after.has_errors(), "{:?}", after.diagnostics);
                let changed = payload_for(&after.phrases[0], route, 6789);
                let mut stripped = changed.clone();
                stripped.model_dynamics = None;
                assert_eq!(stripped, base, "F0/UV, durations, B, synthesis and seeds must remain identical");
                assert!(digests.insert(render_digest(&changed, route, &caps)));
                let value = serde_json::to_value(&changed).unwrap();
                assert_eq!(value["model_dynamics"], json!({"method":"voiced-envelope-v1","strength":strength,"max_attenuation_db":attenuation}));
                assert!(value["score"].get("model_dynamics").is_none());
                assert!(value["score"]["synthesis"].get("model_dynamics").is_none());
            }
            p.render_preferences.model_dynamics = None;
            let restored = plan(&p, &track, route, &caps);
            let restored = payload_for(&restored.phrases[0], route, 6789);
            assert_eq!(restored, base);
            assert_eq!(render_digest(&restored, route, &caps), render_digest(&base, route, &caps));
        }
    }
}

#[test]
fn selected_dynamics_requires_each_method_route_and_finite_ordered_bound() {
    for route in [Route::C0] {
        for case in 0..15 {
            let mut p = project();
            let track = p.tracks[0].id.clone();
            let mut caps = capabilities();
            let dynamics = &mut caps.native_score.as_mut().unwrap().model_dynamics;
            match case {
                0 => *dynamics = None,
                1 => dynamics.as_mut().unwrap().methods.clear(),
                2 => dynamics.as_mut().unwrap().routes = vec!["verdant-voice".into()],
                3 => dynamics.as_mut().unwrap().minimum_strength = None,
                4 => dynamics.as_mut().unwrap().maximum_strength = None,
                5 => dynamics.as_mut().unwrap().minimum_attenuation_db = None,
                6 => dynamics.as_mut().unwrap().maximum_attenuation_db = None,
                7 => dynamics.as_mut().unwrap().minimum_strength = Some(f64::NAN),
                8 => dynamics.as_mut().unwrap().maximum_strength = Some(f64::INFINITY),
                9 => dynamics.as_mut().unwrap().minimum_attenuation_db = Some(f64::NAN),
                10 => dynamics.as_mut().unwrap().maximum_attenuation_db = Some(f64::INFINITY),
                11 => dynamics.as_mut().unwrap().minimum_strength = Some(2.0),
                12 => dynamics.as_mut().unwrap().minimum_attenuation_db = Some(13.0),
                13 => dynamics.as_mut().unwrap().maximum_strength = Some(0.2),
                _ => dynamics.as_mut().unwrap().maximum_attenuation_db = Some(5.0),
            }
            p.render_preferences.model_dynamics = Some(ModelDynamicsSettings::voiced_envelope_v1());
            let blocked = plan(&p, &track, route, &caps);
            assert!(blocked.phrases.is_empty(), "{route:?} case {case}");
            assert!(blocked.diagnostics.iter().any(|d| d.code == "E_SYNTHESIS_CAPABILITY" && d.message.contains("元音动态")));
            p.render_preferences.model_dynamics = None;
            assert!(!plan(&p, &track, route, &caps).has_errors(), "off must not require new capability");
        }
    }
}

#[test]
fn imported_invalid_values_are_blocked_and_explicit_zero_still_requires_capability() {
    for (strength, attenuation, method) in [(-0.01, 6.0, "voiced-envelope-v1"), (1.01, 6.0, "voiced-envelope-v1"),
            (0.25, -0.01, "voiced-envelope-v1"), (0.25, 12.01, "voiced-envelope-v1"),
            (f64::NAN, 6.0, "voiced-envelope-v1"), (0.25, f64::NAN, "voiced-envelope-v1"), (0.25, 6.0, "future-method")] {
        let mut p = project();
        let mut d = ModelDynamicsSettings::voiced_envelope_v1();
        d.strength = strength; d.max_attenuation_db = attenuation; d.method = method.into();
        p.render_preferences.model_dynamics = Some(d);
        let blocked = plan(&p, &p.tracks[0].id, Route::C0, &capabilities());
        assert!(blocked.phrases.is_empty());
        assert!(blocked.diagnostics.iter().any(|d| d.code == "E_SYNTHESIS_CAPABILITY"));
    }
    let mut p = project();
    let mut d = ModelDynamicsSettings::voiced_envelope_v1();
    d.strength = 0.0;
    p.render_preferences.model_dynamics = Some(d);
    let mut caps = capabilities();
    caps.native_score.as_mut().unwrap().model_dynamics = None;
    assert!(plan(&p, &p.tracks[0].id, Route::C0, &caps).has_errors());
}

#[test]
fn dynamics_source_hash_invalidation_reaches_the_existing_cache_key() {
    let mut p = project();
    p.render_preferences.model_dynamics = Some(ModelDynamicsSettings::voiced_envelope_v1());
    let mut caps = capabilities();
    let pl = plan(&p, &p.tracks[0].id, Route::C0, &caps);
    let payload = payload_for(&pl.phrases[0], Route::C0, 6789);
    let key = "synthesis:native/verdant_acoustics/model_dynamics.py".to_string();
    caps.files.insert(key.clone(), serde_json::from_value(json!({"available":true,"sha256":"before"})).unwrap());
    let old_digest = render_digest(&payload, Route::C0, &caps);
    caps.files.get_mut(&key).unwrap().sha256 = Some("after".into());
    assert_ne!(render_digest(&payload, Route::C0, &caps), old_digest);
}
