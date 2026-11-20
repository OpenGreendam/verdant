#[path = "support/original.rs"]
mod original;

use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::wire::Capabilities;
use verdant_backend::{plan, Route};
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource, SynthesisSettings, VocalTractSettings};

fn project() -> Project {
    let mut p = original::project();
    let mut n = Note::new(0, 960, 69);
    n.pronunciation = Pronunciation {
        pinyin: "a".into(), source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(), ..Default::default()
    };
    p.tracks[0].notes.push(n);
    p.tracks[0].refresh_index();
    p
}

fn capabilities() -> Capabilities {
    original::capabilities(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["C0"], "max_phrase_seconds":30,
        "native_score":{"english_vowel_proxies":["a"], "vowel_balance_db_range":[0,18], "continuous_f0_control":true,
            "wideband":{"routes":["C0"],"methods":["matched-balanced"],"presence_db_range":[0,24],"sample_rate":44100},
            "vocal_tract":{"routes":["C0"],"methods":["bilinear-v1"],"ratio_range":[0.8,1.25]}},
    }))
}

#[test]
fn tract_changes_only_its_payload_and_cache_field_for_the_unlocked_original_and_legacy_seed_modes() {
    for synthesis in [None, Some(SynthesisSettings::default()), Some(SynthesisSettings { wideband: None, ..SynthesisSettings::native_v10() })] {
        for route in [Route::C0] {
            let mut p = project();
            p.render_preferences.synthesis = synthesis.clone();
            let track = p.tracks[0].id.clone();
            let mut caps = capabilities();
            if synthesis.is_none() { caps.native_score.as_mut().unwrap().vowel_balance_db_range = None; }
            let before = plan(&p, &track, route, &caps);
            assert!(!before.has_errors(), "{:?}", before.diagnostics);
            let base = payload_for(&before.phrases[0], route, 10008);
            assert!(serde_json::to_value(&base).unwrap().get("vocal_tract").is_none());
            let mut digests = std::collections::BTreeSet::from([render_digest(&base, route, &caps)]);
            for ratio in [0.8, 1.0, 1.125, 1.25] {
                p.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(ratio));
                let after = plan(&p, &track, route, &caps);
                assert!(!after.has_errors(), "{:?}", after.diagnostics);
                let changed = payload_for(&after.phrases[0], route, 10008);
                let mut stripped = changed.clone();
                stripped.vocal_tract = None;
                assert_eq!(stripped, base);
                assert_eq!(p.render_preferences.synthesis, synthesis);
                assert_eq!(changed.native_seed, base.native_seed);
                assert!(digests.insert(render_digest(&changed, route, &caps)));
                let value = serde_json::to_value(&changed).unwrap();
                assert_eq!(value["vocal_tract"], json!({"method":"bilinear-v1","ratio":ratio}));
                assert!(value["score"]["synthesis"].get("vocal_tract").is_none());
            }
            p.render_preferences.vocal_tract = None;
            let restored = plan(&p, &track, route, &caps);
            let restored = payload_for(&restored.phrases[0], route, 10008);
            assert_eq!(restored, base);
            assert_eq!(render_digest(&restored, route, &caps), render_digest(&base, route, &caps));
        }
    }
}

#[test]
fn selected_tract_requires_method_bounds_and_the_original_route_explicitly_advertised() {
    let mut p = project();
    let track = p.tracks[0].id.clone();
    for route in [Route::C0] {
        for case in 0..7 {
            let mut caps = capabilities();
            let tract = &mut caps.native_score.as_mut().unwrap().vocal_tract;
            match case {
                0 => *tract = None,
                1 => tract.as_mut().unwrap().methods.clear(),
                2 => tract.as_mut().unwrap().routes = vec!["verdant-voice".into()],
                3 => tract.as_mut().unwrap().ratio_range = Some([0.8, 1.1]),
                4 => tract.as_mut().unwrap().ratio_range = Some([0.8, f64::NAN]),
                5 => tract.as_mut().unwrap().ratio_range = None,
                _ => tract.as_mut().unwrap().ratio_range = Some([1.25, 0.8]),
            }
            p.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(1.25));
            let blocked = plan(&p, &track, route, &caps);
            assert!(blocked.phrases.is_empty(), "{route:?} case {case}");
            assert!(blocked.diagnostics.iter().any(|d| d.code == "E_SYNTHESIS_CAPABILITY"));
            p.render_preferences.vocal_tract = None;
            assert!(!plan(&p, &track, route, &caps).has_errors());
        }
    }
}


#[test]
fn anchored_profiles_require_explicit_anchor_capability_and_bind_both_cache_coordinates() {
    for route in [Route::C0] {
        let mut p = project();
        let track = p.tracks[0].id.clone();
        let mut caps = capabilities();
        let legacy_plan = plan(&p, &track, route, &caps);
        let legacy = payload_for(&legacy_plan.phrases[0], route, 10008);
        p.render_preferences.vocal_tract = Some(VocalTractSettings::anchored_warp_v1(1.125, [0.9, 1.1]));
        assert!(plan(&p, &track, route, &caps).has_errors());
        let tract = caps.native_score.as_mut().unwrap().vocal_tract.as_mut().unwrap();
        tract.methods.push("anchored-warp-v1".into());
        tract.anchor_ratio_range = Some([0.8, 1.25]);
        tract.anchor_fractions = Some([0.1, 0.3]);
        let mut digests = std::collections::BTreeSet::new();
        for anchors in [[0.9, 1.1], [1.0, 1.1], [0.9, 1.0], [1.0, 1.0]] {
            p.render_preferences.vocal_tract = Some(VocalTractSettings::anchored_warp_v1(1.125, anchors));
            let planned = plan(&p, &track, route, &caps);
            assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
            let payload = payload_for(&planned.phrases[0], route, 10008);
            let mut stripped = payload.clone();
            stripped.vocal_tract = None;
            assert_eq!(stripped, legacy);
            assert_eq!(serde_json::to_value(&payload).unwrap()["vocal_tract"], json!({
                "method": "anchored-warp-v1", "ratio": 1.125, "anchor_ratios": anchors,
            }));
            assert!(digests.insert(render_digest(&payload, route, &caps)));
        }
        for case in 0..6 {
            let mut bad_caps = caps.clone();
            let tract = bad_caps.native_score.as_mut().unwrap().vocal_tract.as_mut().unwrap();
            match case {
                0 => tract.anchor_ratio_range = None,
                1 => tract.anchor_fractions = None,
                2 => tract.anchor_fractions = Some([0.2, 0.3]),
                3 => tract.anchor_ratio_range = Some([1.1, 1.25]),
                4 => tract.anchor_ratio_range = Some([f64::NAN, 1.25]),
                _ => tract.anchor_ratio_range = Some([1.25, 0.8]),
            }
            let blocked = plan(&p, &track, route, &bad_caps);
            assert!(blocked.phrases.is_empty(), "{route:?} case {case}");
            assert!(blocked.diagnostics.iter().any(|d| d.code == "E_SYNTHESIS_CAPABILITY"));
        }
    }
}
