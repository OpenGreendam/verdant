use serde_json::json;
use verdant_backend::{Capabilities, Route, adapter::{payload_for, plan_desktop, render_digest}};
use verdant_domain::{Note, Project, PronunciationSource, VocoderHighbandSettings};

#[test]
fn historical_highband_remains_readable_without_affecting_current_render() {
    let mut project = Project::new_empty();
    let mut note = Note::new(0, 960, 60);
    note.pronunciation.pinyin = "a1".into();
    note.pronunciation.source = PronunciationSource::Manual;
    project.tracks[0].notes.push(note);
    let track = project.tracks[0].id.clone();
    let caps: Capabilities = serde_json::from_value(json!({"test_double":true,
        "routes":["verdant-voice"], "native_score":{"inventory":["a"],"sub_syllable_timing":true}})).unwrap();
    let before = plan_desktop(&project, &track, &caps);
    let baseline = payload_for(&before.phrases[0], Route::VerdantVoice, 10008);
    project.render_preferences.vocoder_highband = Some(VocoderHighbandSettings::source_coherent_v1());
    let reopened: Project = serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
    assert_eq!(reopened.render_preferences.vocoder_highband, project.render_preferences.vocoder_highband);
    let after = plan_desktop(&reopened, &track, &caps);
    let payload = payload_for(&after.phrases[0], Route::VerdantVoice, 10008);
    assert_eq!(payload, baseline);
    assert_eq!(render_digest(&payload, Route::VerdantVoice, &caps), render_digest(&baseline, Route::VerdantVoice, &caps));
    assert!(serde_json::to_value(&payload).unwrap().get("vocoder_highband").is_none());
}
