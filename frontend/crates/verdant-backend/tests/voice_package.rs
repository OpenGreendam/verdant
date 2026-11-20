//! Protocol, source-admission and route isolation regressions. Registry tests
//! use an explicit validator double; they do not claim the fixture is a bank.
use std::{collections::BTreeMap,fs,path::PathBuf,sync::atomic::{AtomicU64,Ordering}};
use serde_json::json;
use sha2::{Digest,Sha256};
use verdant_backend::{artifact,voices::{self,SourceEvidence,SourceReceipt,VoiceAdmission,VoiceAssets,VoiceRegistry},wire::{Capabilities,FileStatus,Request,Route,WorkerEvent,parse_event}};
const TOKEN:&str="821b7cd9e7b489077dd55bf581ba90e1dffd3816c6fedcedc320744b498deead";
const MODEL:&str="213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff";
struct Temp(PathBuf);
static NEXT:AtomicU64=AtomicU64::new(0);
impl Temp{fn new()->Self{let path=std::env::temp_dir().join(format!("verdant-voice-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir_all(&path).unwrap();Self(path)}}
impl Drop for Temp{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn hash(bytes:&[u8])->String{hex::encode(Sha256::digest(bytes))}
fn sources()->BTreeMap<String,String>{BTreeMap::from([("sn16k.tkn".into(),TOKEN.into()),("sn16k.bin".into(),MODEL.into())])}
fn write_runtime_manifest(directory: &std::path::Path) -> String {
    let artifact = |file: &str| {
        let bytes = fs::read(directory.join(file)).unwrap();
        json!({"file":file,"bytes":bytes.len(),"sha256":hash(&bytes)})
    };
    let artifacts = voices::PACKAGE_FILES[1..].iter().map(|(role, file)| ((*role).to_string(), artifact(file)))
        .collect::<serde_json::Map<_, _>>();
    let manifest=json!({"schema":"verdant-voice-package/2","name":"verdant-voice",
        "version":voices::VOICE_PACKAGE_VERSION,"strategy":voices::VOICE_STRATEGY,
        "artifacts":artifacts,"records":{"notice":artifact("NOTICE.txt"),"authorization":artifact("AUTHORIZATION.json")}});
    let bytes = serde_json::to_vec(&manifest).unwrap();
    fs::write(directory.join("manifest.json"), &bytes).unwrap();
    hash(&bytes)
}

#[test]
fn voice_import_uses_the_real_worker_operation_and_typed_completion(){
    let request=Request::admit_voice_package("import1","local-voice".into(),"Local".into(),VoiceAssets{voice_package:Some("/local/package".into()),..Default::default()},SourceEvidence{token:"/chosen/source.tkn".into(),model:"/chosen/source.bin".into(),rights_attested:true});
    let value:serde_json::Value=serde_json::from_str(&request.to_line().unwrap()).unwrap();
    assert_eq!(value["op"],"admit_voice");assert_eq!(value["source_evidence"]["model"],"/chosen/source.bin");
    assert_eq!(value["assets"],json!({"voice_package":"/local/package"}));
    let event=parse_event(&json!({"protocol":"ogd-worker/1","event":"voice_admitted","request_id":"import1","reference":"local-voice","display_name":"Local","cache_identity":"a".repeat(64),"source_sha256":sources(),"historical_identity_confirmed":true}).to_string()).unwrap();
    assert!(matches!(event,WorkerEvent::VoiceAdmitted{reference,display_name,..}if reference=="local-voice"&&display_name=="Local"));
}

#[test]
fn legacy_files_never_satisfy_voice_and_route_artifacts_never_alias(){
    let caps=Capabilities{routes:vec!["verdant-voice".into(),"C0".into(),"C1".into()],files:BTreeMap::from([("token".into(),FileStatus{available:true,sha256:None}),("model".into(),FileStatus{available:true,sha256:None}),("vocoder".into(),FileStatus{available:true,sha256:None})]),..Default::default()};
    assert!(caps.missing_files(Route::C0).is_empty());
    assert!(caps.missing_files(Route::VerdantVoice).contains(&"verdant_voice_context_controller"));
    assert!(caps.missing_files(Route::VerdantVoice).contains(&"voice_source_admission"));
    assert_eq!(Route::parse("verdant-voice"),Some(Route::VerdantVoice));
    assert_eq!(artifact::playable_suffix_for(Route::VerdantVoice,true),"/verdant-voice.wav");
    assert!(Route::parse("C1").is_none());
    assert_ne!(artifact::playable_suffix(Route::VerdantVoice),artifact::playable_suffix(Route::C0));
}

#[test]
fn missing_route_uses_the_current_project_default(){
    let project=verdant_domain::Project::new_empty();let mut value=serde_json::to_value(project).unwrap();
    value["render_preferences"].as_object_mut().unwrap().remove("route");
    let restored:verdant_domain::Project=serde_json::from_value(value).unwrap();
    assert_eq!(restored.render_preferences.route,"verdant-voice");
    assert_eq!(serde_json::to_string(&Route::VerdantVoice).unwrap(),"\"verdant-voice\"");
}

#[test]
fn desktop_render_requests_bind_only_eligible_sources() {
    use verdant_backend::{Action, RenderScheduler, adapter::plan_desktop, voices::DESKTOP_ORIGINAL_REFERENCE};
    let mut project = verdant_domain::Project::new_empty();
    let mut note = verdant_domain::model::Note::new(0, 480, 64);
    note.lyric = "啊".into();
    note.pronunciation = verdant_domain::Pronunciation {
        pinyin: "a1".into(), source: verdant_domain::PronunciationSource::Manual,
        ..Default::default()
    };
    project.tracks[0].notes.push(note);
    let track_id = project.tracks[0].id.clone();
    let manifest = "b".repeat(64);
    let mut caps: Capabilities = serde_json::from_value(json!({
        "test_double": true, "routes": ["verdant-voice", "C0", "C1"], "max_active_jobs": 1,
        "native_score": {"inventory": ["a"], "sub_syllable_timing": true, "continuous_f0_control": true},
        "voices": {"version": 1, "default_voice": DESKTOP_ORIGINAL_REFERENCE, "voices": {
            "learned": {"available": true, "cache_identity": "learned", "files": {
                "voice_source_admission": {"available": true, "sha256": manifest}
            }},
            (DESKTOP_ORIGINAL_REFERENCE): {"available": true, "cache_identity": "original",
                "historical_identity_confirmed": true, "source_sha256": sources(), "files": {
                    "token": {"available": true, "sha256": TOKEN},
                    "model": {"available": true, "sha256": MODEL}
                }}
        }}
    })).unwrap();
    let voice=caps.voices.as_mut().unwrap().voices.get_mut("learned").unwrap();
    voice.voice_package=Some(voices::VoicePackage {name:"verdant-voice".into(),version:voices::VOICE_PACKAGE_VERSION.into()});
    for (role,_) in voices::PACKAGE_FILES {
        voice.files.insert((*role).into(),FileStatus {available:true,sha256:Some(manifest.clone())});
    }
    let submit = |project: &verdant_domain::Project, caps: &Capabilities| {
        let plan = plan_desktop(project, &track_id, caps);
        assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
        let mut scheduler = RenderScheduler::new();
        scheduler.on_event(WorkerEvent::Ready { session_id: "desktop-routing".into(), workspace: "/unused".into() });
        scheduler.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps.clone()) });
        scheduler.set_desktop_project_voices(project);
        scheduler.set_plan(1, &plan, 10008);
        scheduler.poll().into_iter().find_map(|action| match action {
            Action::Send(request) => Some((request.route.unwrap(), request.voice_ref.unwrap())),
            _ => None,
        }).unwrap()
    };
    project.tracks[0].render_route = Some("C1".into());
    assert!(plan_desktop(&project, &track_id, &caps).diagnostics.iter().any(|error| error.code == "E_RENDER_ROUTE"));
    project.tracks[0].render_route = Some("C0".into());
    project.tracks[0].voice_ref = Some(DESKTOP_ORIGINAL_REFERENCE.into());
    assert_eq!(submit(&project, &caps), (Route::C0, DESKTOP_ORIGINAL_REFERENCE.into()));
    caps.voices.as_mut().unwrap().voices.get_mut(DESKTOP_ORIGINAL_REFERENCE).unwrap()
        .files.get_mut("model").unwrap().sha256 = Some("c".repeat(64));
    assert!(plan_desktop(&project, &track_id, &caps).diagnostics.iter().any(|error| error.code == "E_VOICE_SELECTION"));
}

#[test]
fn voice_new_project_defaults_and_neutral_settings_emit_no_legacy_processors(){
    let caps:Capabilities=serde_json::from_value(json!({"test_double":true,"routes":["verdant-voice"],"max_phrase_seconds":30.,
        "native_score":{"english_vowel_proxies":["a","i","u","e","o"],"f0_hz_range":[35.,2000.],"vowel_balance_db_range":[0.,18.]}})).unwrap();
    let mut project=verdant_domain::Project::new_empty();project.render_preferences.route="verdant-voice".into();
    let mut note=verdant_domain::model::Note::new(0,480,64);note.lyric="啊".into();
    note.pronunciation=verdant_domain::Pronunciation{pinyin:"a".into(),source:verdant_domain::PronunciationSource::Manual,frontend_profile:"english-vowel-proxy-native-0.2".into(),..Default::default()};
    project.tracks[0].notes.push(note);
    for neutral in [None,Some(verdant_domain::SynthesisSettings::default())]{
        project.render_preferences.synthesis=neutral;
        let plan=verdant_backend::plan(&project,&project.tracks[0].id,Route::VerdantVoice,&caps);
        assert_eq!(plan.phrases.len(),1,"{:?}",plan.diagnostics);
        let payload=serde_json::to_value(verdant_backend::adapter::payload_for(&plan.phrases[0],Route::VerdantVoice,10008)).unwrap();
        for field in ["source_profile","gain_mode","vowel_brightness_db","phonetic_correction"]{assert!(payload.pointer(&format!("/score/synthesis/{field}")).is_none());}
        for field in ["wideband","vocoder_highband","vocal_tract","model_dynamics","consonant_dynamics"]{assert!(payload.get(field).is_none());}
        assert!(payload.pointer("/score/synthesis/vowel_balance_db").is_none_or(|v|v.as_f64()==Some(0.)));
    }
    project.render_preferences.synthesis=Some(verdant_domain::SynthesisSettings::native_v10());
    let plan=verdant_backend::plan(&project,&project.tracks[0].id,Route::VerdantVoice,&caps);
    assert_eq!(plan.phrases.len(),1,"{:?}",plan.diagnostics);
    assert!(plan.phrases[0].score.synthesis.is_none());assert!(plan.phrases[0].wideband.is_none());
    assert_eq!(project.render_preferences.synthesis,Some(verdant_domain::SynthesisSettings::native_v10()));
}

#[test]
fn verdant_onnx_short_pinyin_fits_complete_native_phones() {
    let caps: Capabilities = serde_json::from_value(json!({
        "test_double": true, "routes": ["verdant-voice", "C0"], "max_phrase_seconds": 30,
        "native_score": {"inventory": ["ba", "a"], "sub_syllable_timing": true,
            "continuous_f0_control": true, "explicit_f0_hz_range": [35., 2000.]}
    })).unwrap();
    for (lyric, seconds, phones) in [("ba1", 0.05, 2), ("a1", 0.025, 1)] {
        let mut project = verdant_domain::Project::new_empty();
        let ticks = (seconds * project.ppq as f64 * 2.0).round() as i64;
        let mut note = verdant_domain::model::Note::new(0, ticks, 64);
        note.lyric = lyric.into();
        project.tracks[0].notes.push(note);
        let id = project.tracks[0].id.clone();
        let plan = verdant_backend::plan(&project, &id, Route::VerdantVoice, &caps);
        assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
        let segments = &plan.phrases[0].score.segments;
        assert_eq!(segments.len(), phones);
        assert!(segments.iter().all(|segment| (segment.duration_seconds - 0.025).abs() < 1e-12));
        assert!((plan.phrases[0].seconds - seconds).abs() < 1e-12);
        assert!(verdant_backend::plan(&project, &id, Route::C0, &caps).diagnostics.iter()
            .any(|d| d.code == "E_SYLLABLE_DOMAIN"));
        project.tracks[0].notes[0].duration_tick -= 1;
        let shorter = verdant_backend::plan(&project, &id, Route::VerdantVoice, &caps);
        assert!(shorter.phrases.is_empty());
        assert!(shorter.diagnostics.iter().any(|d| d.code == "E_SYLLABLE_DOMAIN"));
    }
}

#[test]
fn mixed_routes_project_only_their_processors_and_route_undo_preserves_authored_data() {
    use verdant_backend::adapter::{payload_for, plan_all};
    use verdant_domain::{Command, CommandBatch, Project, ProjectStore, Track};
    let caps: Capabilities = serde_json::from_value(json!({"test_double":true,
        "routes":["verdant-voice","C0","C1"],"max_phrase_seconds":30.,
        "native_score":{"inventory":["a"],"sub_syllable_timing":true,"continuous_f0_control":true,
            "explicit_f0_hz_range":[35.,2000.],"vowel_balance_db_range":[0.,18.],
            "wideband":{"methods":["matched-balanced"],"routes":["C0","C1"],"sample_rate":44100,"presence_db_range":[0.,24.]},
            "phonetic_correction":{"methods":["articulatory-v1"],"routes":["C0","C1"],"strength_range":[0.,1.]},
            "vocal_tract":{"methods":["bilinear-v1"],"routes":["C0","C1"],"ratio_range":[0.8,1.25]},
            "model_dynamics":{"methods":["voiced-envelope-v1"],"routes":["C0","C1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.},
            "consonant_dynamics":{"methods":["unvoiced-model-envelope-v1"],"routes":["C0","C1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.},
            "phoneme_timing":{"methods":["vowel-anchor-v1"],"routes":["verdant-voice","C0","C1"],"timing_resolution_seconds":0.005},
            "expression":{"methods":["vocaloid-controls-v1"],"parameters":["BRE","BRI","CLE","GEN","OPE","GWL"],"routes":["verdant-voice","C0","C1"]},
            "output_transients":{"methods":["phoneme-output-envelope-v1"],"routes":["verdant-voice","C0","C1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.}},
        "neural":{"provided_control_track":true,"vocoder_highband":{"methods":["source-coherent-v1"],"routes":["C1"],"requires_native_control":true,"shift_semitones":[0.],"strength_range":[0.,1.]}}
    })).unwrap();
    let mut project = Project::new_empty();
    project.render_preferences.synthesis = Some(verdant_domain::SynthesisSettings::native_v10());
    project.render_preferences.synthesis.as_mut().unwrap().phonetic_correction = Some(verdant_domain::PhoneticCorrectionSettings::articulatory_v1());
    project.render_preferences.vocal_tract = Some(verdant_domain::VocalTractSettings::bilinear_v1(1.125));
    project.render_preferences.model_dynamics = Some(verdant_domain::ModelDynamicsSettings::voiced_envelope_v1());
    project.render_preferences.consonant_dynamics = Some(verdant_domain::ConsonantDynamicsSettings::unvoiced_model_envelope_v1());
    project.render_preferences.vocoder_highband = Some(verdant_domain::VocoderHighbandSettings::source_coherent_v1());
    project.render_preferences.phoneme_timing = Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1());
    project.render_preferences.output_transients = Some(verdant_domain::OutputTransientsSettings::phoneme_output_envelope_v1());
    let mut other = Track::new("Legacy singer", "teal");
    other.render_route = Some("C1".into());
    project.tracks.push(other);
    for track in &mut project.tracks {
        let mut note = verdant_domain::Note::new(0, 960, 64);
        note.lyric = "啊".into();
        note.pronunciation = verdant_domain::Pronunciation { pinyin:"a1".into(), source:verdant_domain::PronunciationSource::Manual, ..Default::default() };
        for key in ["BRE","BRI","CLE","GEN","OPE","GWL","POR"] {
            note.curves.expression_points.insert(key.into(), vec![verdant_domain::model::ExpressionPoint {
                id:verdant_domain::PointId::generate(), tick:0, value:72, unknown:Default::default(),
            }]);
        }
        note.vibrato = Some(verdant_domain::Vibrato::DEFAULT);
        track.notes.push(note);
        track.refresh_index();
    }
    let active = project.tracks[0].id.clone();
    let original = project.clone();
    let mut store = ProjectStore::new(project);
    let mut header = store.project().tracks[0].header();
    header.render_route = Some("verdant-voice".into());
    store.apply(store.revision(), CommandBatch::single("选择 verdant-voice", Command::SetTrackHeader { track:active.clone(), header })).unwrap().unwrap();
    let before_plan = serde_json::to_vec(store.project()).unwrap();
    let plans = plan_all(store.project(), &active, &caps);
    assert_eq!(plans.len(), 2);
    assert!(!plans[0].has_errors(), "{:?}", plans[0].diagnostics);
    assert!(plans[1].diagnostics.iter().any(|error| error.code == "E_RENDER_ROUTE"));
    let final_phrase = &plans[0].phrases[0];
    let payload = serde_json::to_value(payload_for(final_phrase, Route::VerdantVoice, 10008)).unwrap();
    for key in ["wideband","vocoder_highband","vocal_tract","model_dynamics","consonant_dynamics"] {
        assert!(payload.get(key).is_none(), "verdant-voice inherited {key}");
    }
    assert!(payload["score"].get("synthesis").is_none());
    assert!(payload["score"]["phoneme_timing"].is_object());
    assert!(payload["output_transients"].is_object());
    assert!(payload["control_track"].is_object());
    let expression = final_phrase.expression.as_ref().unwrap();
    for key in ["BRE","BRI","CLE","GEN","OPE","GWL"] {
        assert!(expression.segments.iter().any(|segment| segment.curves.get(key).is_some_and(|points| points.iter().any(|point| point[1] == 72.))), "verdant-voice lost authored {key}");
    }
    assert!(plans[1].phrases.is_empty());
    assert_eq!(serde_json::to_vec(store.project()).unwrap(), before_plan, "planning changed the saved document");
    assert_eq!(store.project().render_preferences, original.render_preferences);
    assert_eq!(store.project().tracks[0].notes, original.tracks[0].notes);
    let restored: Project = serde_json::from_slice(&before_plan).unwrap();
    // Serde intentionally omits each track's interval index. The persisted
    // contract is authored data plus the render requests after ordinary load,
    // not equality between populated and freshly deserialized runtime caches.
    assert_eq!(serde_json::to_vec(&restored).unwrap(), before_plan);
    assert_eq!(restored.render_preferences, store.project().render_preferences);
    assert_eq!(restored.tracks[0].render_route.as_deref(), Some("verdant-voice"));
    assert_eq!(restored.tracks[1].render_route.as_deref(), Some("C1"));
    let reopened = ProjectStore::new(restored);
    let reopened_plans = plan_all(reopened.project(), &active, &caps);
    assert_eq!(reopened_plans.len(), plans.len());
    for (reloaded, expected) in reopened_plans.iter().zip(&plans) {
        assert_eq!(reloaded.diagnostics, expected.diagnostics);
        assert_eq!(reloaded.route, expected.route);
        assert_eq!(reloaded.phrases.len(), expected.phrases.len());
        for (actual, expected) in reloaded.phrases.iter().zip(&expected.phrases) {
            assert_eq!(payload_for(actual, reloaded.route, 10008), payload_for(expected, reloaded.route, 10008));
        }
    }
    store.undo().unwrap();
    assert_eq!(serde_json::to_vec(store.project()).unwrap(), serde_json::to_vec(&original).unwrap(),
        "route undo must restore every authored setting and curve");
    store.redo().unwrap();
    assert_eq!(serde_json::to_vec(store.project()).unwrap(), before_plan);
}

#[test]
fn first_install_catalog_preserves_only_the_unpinned_legacy_cli_alias(){
    let temp=Temp::new();let path=temp.0.join("voices.json");
    let registry=VoiceRegistry::load_or_empty(&path).unwrap();assert!(registry.references().is_empty());assert!(!path.exists());
    registry.save_atomic(&path).unwrap();assert!(VoiceRegistry::load_or_empty(&path).unwrap().references().is_empty());
    fs::write(&path,b"not a registry").unwrap();assert!(VoiceRegistry::load_or_empty(&path).is_err());
    let mut project=verdant_domain::Project::new_empty();let mut caps=Capabilities::default();
    for catalog in [json!({"version":1,"voices":{}}),json!({"version":1,"voices":{"other-voice":{"available":true,"cache_identity":"fixture"}}})]{
        caps.voices=Some(serde_json::from_value(catalog).unwrap());
        for reference in [None,Some("voice-sn")]{
            project.tracks[0].voice_ref=reference.map(str::to_owned);
            assert!(caps.voice_for_track(&project,&project.tracks[0]).unwrap().is_none());
        }
        project.tracks[0].voice_ref=Some("missing-voice".into());assert!(caps.voice_for_track(&project,&project.tracks[0]).is_err());
        project.tracks[0].voice_ref=Some("voice-sn".into());project.tracks[0].unknown.insert("voice_selection".into(),json!({"schema":"verdant-track-voice/1","reference":"voice-sn","cache_identity":"previous-installation"}));
        assert!(caps.voice_for_track(&project,&project.tracks[0]).is_err());project.tracks[0].unknown.remove("voice_selection");
    }
}

#[test]
fn a_checkbox_and_self_declared_hashes_cannot_replace_actual_original_bytes(){
    let temp=Temp::new();let token=temp.0.join("source.tkn");let model=temp.0.join("source.bin");
    fs::write(&token,b"not the original token").unwrap();fs::write(&model,b"not the original binary").unwrap();
    let mut evidence=SourceEvidence{token,model,rights_attested:false};
    assert!(voices::verify_source_evidence(&evidence,&"a".repeat(64)).is_err());
    evidence.rights_attested=true;
    assert!(voices::verify_source_evidence(&evidence,&"a".repeat(64)).is_err());
}

#[test]
fn voice_requires_a_bound_receipt_and_every_model_byte_stays_pinned(){
    let temp=Temp::new();
    let (assets, admission) = relocation_fixture(&temp.0, false);
    let mut registry=VoiceRegistry::new();
    assert!(registry.admit_with("missing","Missing evidence",assets.clone(),|_|Ok(VoiceAdmission{source_sha256:sources(),..Default::default()})).is_err());assert!(registry.references().is_empty());
    let receipt=admission.source_receipt.unwrap();
    let mut wrong=receipt.clone();wrong.source_manifest_sha256="0".repeat(64);
    assert!(registry.admit_with("wrong","Wrong binding",assets.clone(),|_|Ok(VoiceAdmission{source_sha256:sources(),source_receipt:Some(wrong),..Default::default()})).is_err());assert!(registry.references().is_empty());
    registry.admit_with("bound","Bound fixture",assets,|_|Ok(VoiceAdmission{source_sha256:sources(),source_receipt:Some(receipt),..Default::default()})).unwrap();
    let caps=registry.capabilities();assert_eq!(caps["voices"]["bound"]["files"]["voice_source_admission"]["available"],true);
    assert!(registry.selection("bound").is_ok());
    fs::write(temp.0.join("context-controller.onnx"),b"changed model after admission").unwrap();
    assert!(matches!(registry.selection("bound"),Err(voices::VoiceError::SourceChanged(_))));
}

/// Deliberate registry-validator double, not executable ONNX/source assets.
fn relocation_fixture(directory: &std::path::Path, vocoder: bool) -> (VoiceAssets, VoiceAdmission) {
    fs::create_dir_all(directory).unwrap();
    for name in voices::PACKAGE_FILES[1..].iter().map(|(_, name)| *name).chain(["NOTICE.txt", "AUTHORIZATION.json"]) {
        fs::write(directory.join(name), b"registry relocation fixture").unwrap();
    }
    let manifest_hash = write_runtime_manifest(directory);
    let vocoder = vocoder.then(|| {
        let path = directory.join("fixture-vocoder.onnx");
        fs::write(&path, b"registry vocoder fixture").unwrap();
        path
    });
    (VoiceAssets { voice_package:Some(directory.to_owned()), vocoder, ..Default::default() },
        VoiceAdmission { source_sha256:sources(), source_receipt:Some(SourceReceipt {
            schema:"verdant-voice-source-admission/1".into(), original_source_sha256:sources(),
            source_manifest_sha256:manifest_hash, rights_attested:true, attestation_is_legal_determination:false,
        }), ..Default::default() })
}

#[test]
fn voice_reimport_revalidates_and_relocates_missing_paths_without_repinning_tracks() {
    let temp = Temp::new();
    let path = temp.0.join("registry.json");
    let original_dir = temp.0.join("original");
    let (assets, admission) = relocation_fixture(&original_dir, false);
    let mut registry = VoiceRegistry::new();
    let mut validations = 0;
    let first = registry.install_with(&path, "portable", "Saved name", assets.clone(), |_| {
        validations += 1; Ok(admission.clone())
    }).unwrap();
    registry.set_default(Some("portable")).unwrap();
    registry.save_atomic(&path).unwrap();
    let original_bytes = fs::read(&path).unwrap();
    let repeated = registry.install_with(&path, "portable", "A reimport cannot rename the entry", assets, |_| {
        validations += 1; Ok(admission.clone())
    }).unwrap();
    assert_eq!(repeated, first);
    assert_eq!(fs::read(&path).unwrap(), original_bytes, "same-path reimport must be idempotent");
    let mut project = verdant_domain::Project::new_empty();
    project.tracks[0].voice_ref = Some("portable".into());
    project.tracks[0].unknown.insert("voice_selection".into(), json!({
        "schema":"verdant-track-voice/1","reference":"portable","cache_identity":first.cache_identity,
        "source_sha256":first.source_sha256,
    }));
    let moved_dir = temp.0.join("moved");
    fs::rename(&original_dir, &moved_dir).unwrap();
    assert!(registry.selection("portable").is_err());
    // Loading an unavailable entry must retain the frozen identity for repair.
    registry = VoiceRegistry::load(&path).unwrap();
    let moved_assets = VoiceAssets { voice_package:Some(moved_dir.clone()), ..Default::default() };
    let relocated = registry.install_with(&path, "portable", "Ignored rename", moved_assets, |chosen| {
        validations += 1;
        assert_eq!(chosen.voice_package.as_deref(), Some(moved_dir.as_path()));
        Ok(admission.clone())
    }).unwrap();
    assert_eq!(validations, 3, "repeated/relocated imports must call the full validator");
    assert_eq!(relocated.cache_identity, first.cache_identity);
    assert_eq!(relocated.bind_cache_identity("song"), first.bind_cache_identity("song"));
    assert_eq!(relocated.source_sha256, first.source_sha256);
    assert_eq!(relocated.source_receipt, first.source_receipt);
    assert_eq!(relocated.display_name, first.display_name);
    assert_eq!(relocated.historical_identity_confirmed, first.historical_identity_confirmed);
    assert_eq!(relocated.languages, first.languages);
    let restored = VoiceRegistry::load(&path).unwrap();
    assert_eq!(restored.references(), vec!["portable"]);
    assert_eq!(restored.default_reference(), Some("portable"));
    assert_eq!(restored.resolve_track(&project, &project.tracks[0].id).unwrap(), Some(relocated));
    assert_eq!(restored.capabilities()["voices"]["portable"]["available"], true);
    assert!(!original_dir.exists());
}

#[test]
fn voice_same_reference_cannot_upgrade_weights_change_roles_or_skip_source_admission() {
    let temp = Temp::new();
    let path = temp.0.join("registry.json");
    let (assets, admission) = relocation_fixture(&temp.0.join("installed"), false);
    let mut registry = VoiceRegistry::new();
    let installed = registry.install_with(&path, "fixed", "Fixed", assets, |_| Ok(admission.clone())).unwrap();
    let stored = fs::read(&path).unwrap();
    let memory = serde_json::to_vec(&registry).unwrap();
    for case in [0, 1, 4, 5, 6, 7] {
        let directory = temp.0.join(format!("candidate-{case}"));
        let (candidate, mut proof) = relocation_fixture(&directory, false);
        match case {
            0 | 1 => {
                let graph = if case == 0 { "context-controller.onnx" } else { "decoder-48k.onnx" };
                fs::write(directory.join(graph), b"new graph weights").unwrap();
                proof.source_receipt.as_mut().unwrap().source_manifest_sha256 = write_runtime_manifest(&directory);
            }
            4 => proof.source_receipt = None,
            5 => proof.source_receipt.as_mut().unwrap().rights_attested = false,
            6 => {
                proof.source_sha256.insert("sn16k.bin".into(), "e".repeat(64));
                proof.source_receipt.as_mut().unwrap().original_source_sha256 = proof.source_sha256.clone();
            }
            _ => {
                fs::write(directory.join("bank.safetensors"), b"different bank fixture").unwrap();
                proof.source_receipt.as_mut().unwrap().source_manifest_sha256 = write_runtime_manifest(&directory);
            }
        }
        let mut validated = false;
        let result = registry.install_with(&path, "fixed", "Replacement", candidate, |_| {
            validated = true; Ok(proof)
        });
        assert!(validated);
        assert!(result.is_err(), "replacement case {case} changed a stable reference");
        if case < 2 || case == 7 { assert!(matches!(result, Err(voices::VoiceError::DuplicateReference(_)))); }
        assert_eq!(fs::read(&path).unwrap(), stored);
        assert_eq!(serde_json::to_vec(&registry).unwrap(), memory);
        assert_eq!(registry.selection("fixed").unwrap(), installed);
    }
    // Adding a previously absent optional role is also a change of identity.
    let (mut no_vocoder, proof) = relocation_fixture(&temp.0.join("without-vocoder"), true);
    let extra_vocoder = no_vocoder.vocoder.take();
    registry.install_with(&path, "without", "Without", no_vocoder.clone(), |_| Ok(proof.clone())).unwrap();
    no_vocoder.vocoder = extra_vocoder;
    let before = fs::read(&path).unwrap();
    assert!(matches!(registry.install_with(&path, "without", "With", no_vocoder, |_| Ok(proof)),
        Err(voices::VoiceError::AdmissionRejected(_))));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn voice_relocation_failure_preserves_memory_disk_and_original_registration() {
    let temp = Temp::new();
    let path = temp.0.join("registry.json");
    let (original, admission) = relocation_fixture(&temp.0.join("installed"), false);
    let (candidate, _) = relocation_fixture(&temp.0.join("relocated"), false);
    let mut registry = VoiceRegistry::new();
    let first = registry.install_with(&path, "stable", "Stable", original, |_| Ok(admission.clone())).unwrap();
    let memory = serde_json::to_vec(&registry).unwrap();
    let disk = fs::read(&path).unwrap();
    // A full validator failure cannot be bypassed just because hashes match.
    assert!(matches!(registry.install_with(&path, "stable", "Stable", candidate.clone(), |_| Err("source-use declaration missing".into())),
        Err(voices::VoiceError::AdmissionRejected(_))));
    assert_eq!(serde_json::to_vec(&registry).unwrap(), memory);
    assert_eq!(fs::read(&path).unwrap(), disk);
    // Induce a real atomic-rename failure: a file cannot replace a directory.
    let invalid_target = temp.0.join("not-a-registry-file");
    fs::create_dir(&invalid_target).unwrap();
    let mut validated = false;
    assert!(matches!(registry.install_with(&invalid_target, "stable", "Stable", candidate.clone(), |_| {
        validated = true; Ok(admission.clone())
    }), Err(voices::VoiceError::Io(_))));
    assert!(validated);
    assert_eq!(serde_json::to_vec(&registry).unwrap(), memory);
    assert_eq!(fs::read(&path).unwrap(), disk);
    assert_eq!(registry.selection("stable").unwrap(), first);
    assert!(!fs::read_dir(&temp.0).unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with(".not-a-registry-file.tmp.")));
    // Rehashing the input after validation also protects a relocation.
    assert!(matches!(registry.install_with(&path, "stable", "Stable", candidate, |chosen| {
        fs::write(chosen.voice_package.as_ref().unwrap().join("context-controller.onnx"), b"validation-time drift").unwrap();
        Ok(admission)
    }), Err(voices::VoiceError::SourceChanged(_))));
    assert_eq!(serde_json::to_vec(&registry).unwrap(), memory);
    assert_eq!(fs::read(&path).unwrap(), disk);
    assert_eq!(VoiceRegistry::load(&path).unwrap().selection("stable").unwrap(), first);
}
