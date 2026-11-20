use verdant_domain::{Command,CommandBatch,ConsonantDynamicsSettings,Project,ProjectStore,PhonemeTimingSettings};

#[test]
fn consonant_dynamic_changes_preserve_other_choices_and_are_atomic_undoable_and_serialized() {
    let mut project=Project::new_empty();
    project.render_preferences.phoneme_timing=Some(PhonemeTimingSettings::vowel_anchor_v1());
    let mut store=ProjectStore::new(project);
    let before=store.project().render_preferences.clone();
    for (strength,cap) in [(0.25,6.0),(0.0,6.0),(0.25,0.0),(1.0,12.0)] {
        let mut setting=ConsonantDynamicsSettings::unvoiced_model_envelope_v1();
        setting.strength=strength;setting.max_attenuation_db=cap;
        store.apply(store.revision(),CommandBatch::single("辅音动态",Command::SetConsonantDynamics{settings:Some(setting.clone())})).unwrap().unwrap();
        let text=serde_json::to_string(store.project()).unwrap();
        let (restored,_)=verdant_domain::io::parse_project(&text).unwrap();
        assert_eq!(restored.render_preferences.consonant_dynamics,Some(setting));
        let mut unchanged=restored.render_preferences.clone();unchanged.consonant_dynamics=None;
        assert_eq!(unchanged,before);
        store.undo().unwrap();
        assert_eq!(store.project().render_preferences,before);
        store.redo().unwrap();
        store.undo().unwrap();
    }
    let mut invalid=ConsonantDynamicsSettings::unvoiced_model_envelope_v1();invalid.strength=f64::NAN;
    let revision=store.revision();
    let batch=CommandBatch{label:"invalid".into(),commands:vec![
        Command::SetPhonemeTiming{settings:None},
        Command::SetConsonantDynamics{settings:Some(invalid)},
    ]};
    assert!(store.apply(revision,batch).is_err());
    assert_eq!(store.revision(),revision);
    assert_eq!(store.project().render_preferences,before);
}
