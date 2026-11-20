#[path = "support/original.rs"]
mod original;

use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::{Capabilities, Route, plan};
use verdant_domain::{ConsonantDynamicsSettings, ModelDynamicsSettings, Note, Project, Pronunciation, PronunciationSource};

fn caps() -> Capabilities {
    original::capabilities(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["C0"],
        "native_score":{"inventory":["gu"],"continuous_f0_control":true,
            "consonant_dynamics":{"methods":["unvoiced-model-envelope-v1"],"routes":["C0"],"minimum_strength":0,"maximum_strength":1,"minimum_attenuation_db":0,"maximum_attenuation_db":12},
            "model_dynamics":{"methods":["voiced-envelope-v1"],"routes":["C0"],"minimum_strength":0,"maximum_strength":1,"minimum_attenuation_db":0,"maximum_attenuation_db":12}},
    }))
}

fn project() -> Project {
    let mut project = original::project();
    let mut note = Note::new(960,960,60);
    note.pronunciation = Pronunciation {pinyin:"gu1".into(),source:PronunciationSource::Manual,..Default::default()};
    project.tracks[0].notes.push(note);
    project.tracks[0].refresh_index();
    project
}

#[test]
fn consonant_settings_alone_change_the_wire_field_and_digest_in_the_unlocked_original() {
    for route in [Route::C0] {
        for vowel in [None,Some(ModelDynamicsSettings::voiced_envelope_v1())] {
            let mut p=project();
            p.render_preferences.model_dynamics=vowel;
            let old=plan(&p,&p.tracks[0].id,route,&caps());
            assert!(!old.has_errors());
            let old=payload_for(&old.phrases[0],route,1234);
            assert!(serde_json::to_value(&old).unwrap().get("consonant_dynamics").is_none());
            let mut digests=std::collections::BTreeSet::from([render_digest(&old,route,&caps())]);
            for (strength,cap) in [(0.25,6.0),(0.0,6.0),(0.25,0.0),(1.0,12.0)] {
                let mut settings=ConsonantDynamicsSettings::unvoiced_model_envelope_v1();
                settings.strength=strength;settings.max_attenuation_db=cap;
                p.render_preferences.consonant_dynamics=Some(settings);
                let new=plan(&p,&p.tracks[0].id,route,&caps());
                assert!(!new.has_errors(),"{:?}",new.diagnostics);
                let new=payload_for(&new.phrases[0],route,1234);
                assert!(digests.insert(render_digest(&new,route,&caps())));
                let mut without=new.clone();without.consonant_dynamics=None;
                assert_eq!(without,old,"No seed, B, F0, timing or synthesis-setting side effects");
                assert_eq!(serde_json::to_value(new).unwrap()["consonant_dynamics"],json!({"method":"unvoiced-model-envelope-v1","strength":strength,"max_attenuation_db":cap}));
            }
        }
    }
}

#[test]
fn invalid_parameters_or_missing_capabilities_cannot_silently_drop_an_enabled_effect() {
    for route in [Route::C0] {
        for case in 0..11 {
            let mut p=project();
            let mut c=caps();
            let mut setting=ConsonantDynamicsSettings::unvoiced_model_envelope_v1();
            match case {
                0=>setting.strength=-0.01,
                1=>setting.strength=f64::NAN,
                2=>setting.max_attenuation_db=12.01,
                3=>setting.max_attenuation_db=f64::INFINITY,
                4=>setting.method="unrecognized".into(),
                5=>c.native_score.as_mut().unwrap().consonant_dynamics=None,
                6=>c.native_score.as_mut().unwrap().consonant_dynamics.as_mut().unwrap().routes.clear(),
                7=>c.native_score.as_mut().unwrap().consonant_dynamics.as_mut().unwrap().methods.clear(),
                8=>c.native_score.as_mut().unwrap().consonant_dynamics.as_mut().unwrap().minimum_strength=Some(2.0),
                9=>c.native_score.as_mut().unwrap().consonant_dynamics.as_mut().unwrap().maximum_attenuation_db=None,
                _=>c.native_score.as_mut().unwrap().consonant_dynamics.as_mut().unwrap().maximum_attenuation_db=Some(f64::INFINITY),
            }
            p.render_preferences.consonant_dynamics=Some(setting);
            let result=plan(&p,&p.tracks[0].id,route,&c);
            assert!(result.phrases.is_empty());
            assert!(result.diagnostics.iter().any(|diagnostic|diagnostic.code=="E_SYNTHESIS_CAPABILITY"));
            p.render_preferences.consonant_dynamics=None;
            assert!(!plan(&p,&p.tracks[0].id,route,&c).has_errors());
        }
    }
}
