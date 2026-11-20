//! Singer editor transactions through registry, preflight and worker requests.
//! Small synthetic assets exercise routing/admission boundaries only; they are
//! not loadable voice banks or evidence of acoustic equivalence.
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use verdant_backend::voices::{VoiceAdmission, VoiceAssets, VoiceError, VoiceRegistry, singer_choice_batch};
use verdant_backend::wire::Capabilities;
use verdant_backend::{Action, RenderScheduler, Route, WorkerEvent, plan};
use verdant_domain::{Note, Project, ProjectStore, Pronunciation, PronunciationSource};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Inputs(PathBuf);
impl Inputs {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-singer-selection-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn modern(&self, name: &str) -> (VoiceAssets, BTreeMap<String, String>) {
        let path = self.0.join(name);
        fs::create_dir_all(&path).unwrap();
        let sources: BTreeMap<String, String> = BTreeMap::from([("sn16k.tkn".into(), "a".repeat(64)), ("sn16k.bin".into(), "b".repeat(64))]);
        fs::write(path.join("voice.json"), serde_json::to_vec(&serde_json::json!({"source_sha256":sources,"languages":{"zh":"native","ja":"approximate"}})).unwrap()).unwrap();
        fs::write(path.join("topology.json"), serde_json::to_vec(&serde_json::json!({"source_sha256":sources})).unwrap()).unwrap();
        for file in ["tensors.safetensors", "PROVENANCE.json", "SHA256SUMS"] { fs::write(path.join(file), format!("{name}-{file}")).unwrap(); }
        (VoiceAssets { voice_model: Some(path), ..Default::default() }, sources)
    }
}
impl Drop for Inputs { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

#[test]
fn singer_selection_pins_registry_sources_routes_worker_and_invalidates_cache() {
    let inputs = Inputs::new();
    let mut registry = VoiceRegistry::new();
    let mut selections = Vec::new();
    for name in ["singer-a", "singer-b"] {
        let (assets, sources) = inputs.modern(name);
        selections.push(registry.admit_with(name, name, assets, |_| Ok(VoiceAdmission { source_sha256: sources, ..Default::default() })).unwrap());
    }
    let caps: Capabilities = serde_json::from_value(serde_json::json!({
        "test_double":true,"engine":"synthetic routing fixture", "protocol":"ogd-worker/1", "routes":["verdant-voice"],
        "max_active_jobs":1, "max_phrase_seconds":30.0, "voices":registry.capabilities(),
        "native_score":{"inventory":verdant_backend::pinyin::recovered_syllables(),"sub_syllable_timing":true}
    })).unwrap();
    let mut project = Project::new_empty();
    let mut note = Note::new(0, 480, 60);
    note.lyric = "啊".into();
    note.pronunciation = Pronunciation { pinyin: "a".into(), source: PronunciationSource::Manual, ..Default::default() };
    project.tracks[0].notes.push(note);
    project.tracks[0].refresh_index();
    let track = project.tracks[0].id.clone();
    let mut store = ProjectStore::new(project);
    let mut digests = Vec::new();
    for selection in &selections {
        let batch = singer_choice_batch(store.project(), &track, &selection.reference, &caps).unwrap();
        store.apply(store.revision(), batch).unwrap().unwrap();
        let selected = registry.resolve_track(store.project(), &track).unwrap().unwrap();
        assert_eq!(selected.source_sha256, selection.source_sha256);
        assert_eq!(selected.cache_identity, selection.cache_identity);
        assert_eq!(caps.voices.as_ref().unwrap().voices[&selection.reference].languages["ja"], "approximate");
        let mut scheduler = RenderScheduler::new();
        scheduler.on_event(WorkerEvent::Ready { session_id: "routing".into(), workspace: "/unused".into() });
        scheduler.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps.clone()) });
        let planned = plan(store.project(), &track, Route::VerdantVoice, &caps);
        assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
        assert_eq!(planned.phrases.len(), 1);
        scheduler.set_project_voices(store.project());
        scheduler.set_plan(store.revision().get(), &planned, 10008);
        digests.push(scheduler.wanted()[0].digest.clone());
        let request = scheduler.poll().into_iter().find_map(|action| if let Action::Send(request) = action { Some(request) } else { None }).unwrap();
        assert_eq!(request.voice_ref.as_deref(), Some(selection.reference.as_str()));
    }
    assert_ne!(digests[0], digests[1], "changing only the bank must not reuse the old audio");
    assert!(singer_choice_batch(store.project(), &track, "unknown", &caps).is_err());
    let mut unknown = store.project().clone();
    unknown.tracks[0].voice_ref = Some("unknown".into());
    assert!(matches!(registry.resolve_track(&unknown, &track), Err(VoiceError::UnknownReference(_))));
    assert!(plan(&unknown, &track, Route::VerdantVoice, &caps).has_errors());
    let mut mismatched = store.project().clone();
    mismatched.tracks[0].unknown.get_mut("voice_selection").unwrap()["source_sha256"]["voice_tensors"] = serde_json::json!("f".repeat(64));
    assert!(matches!(registry.resolve_track(&mismatched, &track), Err(VoiceError::ProjectHashMismatch(_))));
    assert!(caps.voice_for_track(&mismatched, &mismatched.tracks[0]).is_err());
    store.undo().unwrap();
    assert_eq!(store.project().tracks[0].voice_ref.as_deref(), Some("singer-a"));
    assert_eq!(registry.resolve_track(store.project(), &track).unwrap().unwrap().source_sha256, selections[0].source_sha256);
}
