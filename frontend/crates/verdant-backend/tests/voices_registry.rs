//! Registry routing and transaction tests use small private input files only.
//! They exercise the validator boundary, pins and consumer cache identity; they
//! do not implement a voice loader or assert acoustic/model equivalence.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use verdant_backend::voices::{VoiceAdmission, VoiceAssets, VoiceError, VoiceRegistry};
use verdant_domain::model::VoiceRef;
use verdant_domain::{Project, Track};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Inputs { root: PathBuf }
impl Inputs {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("verdant-voice-registry-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
    fn raw(&self, name: &str, token: &[u8], model: &[u8], vocoder: Option<&[u8]>) -> VoiceAssets {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("source.tkn"), token).unwrap();
        fs::write(dir.join("source.bin"), model).unwrap();
        if let Some(vocoder) = vocoder { fs::write(dir.join("source.onnx"), vocoder).unwrap(); }
        VoiceAssets { voice_package: None, voice_model: None, token: Some(dir.join("source.tkn")), model: Some(dir.join("source.bin")),
            vocoder: vocoder.map(|_| dir.join("source.onnx")), ..Default::default() }
    }
}
impl Drop for Inputs { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.root); } }

fn registry_proof(_: &VoiceAssets) -> Result<VoiceAdmission, String> { Ok(VoiceAdmission::default()) }

#[test]
fn release_default_upgrades_only_its_own_selection_and_preserves_legacy_choices() {
    let inputs=Inputs::new();
    let mut registry=VoiceRegistry::new();
    registry.admit_with("old","Old",inputs.raw("old",b"token-old",b"model-old",None),registry_proof).unwrap();
    registry.admit_with("new","New",inputs.raw("new",b"token-new",b"model-new",None),registry_proof).unwrap();
    assert!(registry.upgrade_default("old").unwrap());
    assert!(registry.upgrade_default("new").unwrap());
    assert_eq!(registry.default_reference(),Some("new"));
    // Even choosing the same current singer explicitly ends automatic upgrades.
    registry.set_default(Some("new")).unwrap();
    assert!(!registry.upgrade_default("old").unwrap());
    assert_eq!(registry.default_reference(),Some("new"));
    let path=inputs.root.join("registry.json");registry.save_atomic(&path).unwrap();
    let bytes=fs::read(&path).unwrap();
    let value:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("managed_default").is_none());
    let mut legacy=VoiceRegistry::load(&path).unwrap();
    assert!(!legacy.upgrade_default("old").unwrap());
    assert_eq!(legacy.default_reference(),Some("new"));
    assert!(legacy.upgrade_default("missing").is_err());
    assert_eq!(legacy.default_reference(),Some("new"));
    assert_eq!(fs::read(&path).unwrap(),bytes);
}

#[test]
fn full_validator_failure_and_validation_time_drift_are_atomic() {
    let inputs = Inputs::new();
    let assets = inputs.raw("first", b"source-token-A", b"source-model-A", None);
    let path = inputs.root.join("registry.json");
    let mut registry = VoiceRegistry::new();
    registry.save_atomic(&path).unwrap();
    let original = fs::read(&path).unwrap();
    let result = registry.install_with(&path, "singer-a", "User singer", assets.clone(), |_| Err("real native parser rejected source".into()));
    assert!(matches!(result, Err(VoiceError::AdmissionRejected(_))));
    assert!(registry.references().is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
    let result = registry.admit_with("singer-a", "User singer", assets, |selected| {
        fs::write(selected.model.as_ref().unwrap(), b"changed while validating").unwrap();
        Ok(VoiceAdmission::default())
    });
    assert!(matches!(result, Err(VoiceError::SourceChanged(_))));
    assert!(registry.references().is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn failed_persistence_preserves_in_memory_registry() {
    let inputs = Inputs::new();
    let assets = inputs.raw("source", b"token", b"model", None);
    let mut registry = VoiceRegistry::new();
    // A directory cannot be atomically replaced by the registry's regular file.
    assert!(registry.install_with(&inputs.root, "source", "Source", assets, registry_proof).is_err());
    assert!(registry.references().is_empty());
}

#[test]
fn source_bytes_bind_cache_identity_without_machine_paths() {
    let inputs = Inputs::new();
    let first = inputs.raw("first", b"token", b"model", None);
    let moved = inputs.raw("moved", b"token", b"model", None);
    let other = inputs.raw("other", b"token", b"changed model", None);
    let mut registry = VoiceRegistry::new();
    let a = registry.admit_with("a", "First name", first.clone(), registry_proof).unwrap();
    let b = registry.admit_with("b", "Relocated name", moved, registry_proof).unwrap();
    let c = registry.admit_with("c", "Different source", other, registry_proof).unwrap();
    assert_eq!(a.cache_identity, b.cache_identity);
    assert_eq!(a.bind_cache_identity("phrase-content"), b.bind_cache_identity("phrase-content"));
    assert_ne!(a.cache_identity, c.cache_identity);
    assert!(!a.historical_identity_confirmed, "a label cannot confirm unknown bytes");
    registry.set_default(Some("a")).unwrap();
    assert_eq!(registry.default_reference(), Some("a"));
    let registry_path = inputs.root.join("registry.json");
    registry.save_atomic(&registry_path).unwrap();
    let loaded = VoiceRegistry::load(&registry_path).unwrap();
    assert_eq!(loaded.resolve("a").unwrap(), a.assets);
    assert_eq!(loaded.default_selection().unwrap().cache_identity, a.cache_identity);
    let catalog = loaded.capabilities().to_string();
    assert!(!catalog.contains(&inputs.root.to_string_lossy().to_string()));
    fs::write(first.model.unwrap(), b"post-admission drift").unwrap();
    assert!(matches!(loaded.resolve("a"), Err(VoiceError::SourceChanged(_))));
    assert_eq!(loaded.capabilities()["voices"]["a"]["available"], false);
    assert_eq!(loaded.capabilities()["voices"]["b"]["available"], true);
}

#[test]
fn historical_vocoder_registration_decodes_without_render_admission() {
    let inputs = Inputs::new();
    let assets = inputs.raw("historical", b"token", b"model", Some(b"vocoder"));
    let path = inputs.root.join("registry.json");
    let files: BTreeMap<_, _> = [("token", &assets.token), ("model", &assets.model), ("vocoder", &assets.vocoder)]
        .into_iter().map(|(role, path)| (role, verdant_domain::digest::sha256_hex(&fs::read(path.as_ref().unwrap()).unwrap())))
        .collect();
    fs::write(&path, serde_json::to_vec(&serde_json::json!({"version":1,"default_voice":"historical","voices":{
        "historical":{"display_name":"Historical","assets":assets,"files":files}
    }})).unwrap()).unwrap();
    let loaded = VoiceRegistry::load(&path).unwrap();
    assert_eq!(loaded.references(), ["historical"]);
    assert!(matches!(loaded.selection("historical"), Err(VoiceError::MigrationRequired(_))));
    assert!(VoiceRegistry::new().admit_with("new", "New", assets, |_| panic!("retired asset reached validator")).is_err());
}

#[test]
fn each_authored_track_resolves_its_source_and_hash_pins_after_reordering() {
    let inputs = Inputs::new();
    let mut registry = VoiceRegistry::new();
    let a = registry.admit_with("user-a", "Singer A", inputs.raw("a", b"a-token", b"a-model", None), registry_proof).unwrap();
    let b = registry.admit_with("user-b", "Singer B", inputs.raw("b", b"b-token", b"b-model", None), registry_proof).unwrap();
    let mut project = Project::new_empty();
    project.tracks[0].voice_ref = Some("user-a".into());
    let first_id = project.tracks[0].id.clone();
    let mut second = Track::new("Second", "blue");
    second.voice_ref = Some("user-b".into());
    let second_id = second.id.clone();
    project.tracks.push(second);
    project.voice_refs.insert("user-a".into(), serde_json::from_value::<VoiceRef>(serde_json::json!({"name":"user-a",
        "historical_identity_confirmed":false,"token_sha256":a.source_sha256["token"],"model_sha256":a.source_sha256["model"]})).unwrap());
    project.tracks.reverse();
    assert_eq!(registry.resolve_track(&project, &first_id).unwrap().unwrap().cache_identity, a.cache_identity);
    assert_eq!(registry.resolve_track(&project, &second_id).unwrap().unwrap().cache_identity, b.cache_identity);
    project.voice_refs.get_mut("user-a").unwrap().token_sha256 = Some("f".repeat(64));
    assert!(matches!(registry.resolve_track(&project, &first_id), Err(VoiceError::ProjectHashMismatch(_))));
    project.tracks.iter_mut().find(|track| track.id == second_id).unwrap().voice_ref = Some("not-installed".into());
    assert!(matches!(registry.resolve_track(&project, &second_id), Err(VoiceError::UnknownReference(_))));
}

#[test]
fn mixed_sources_and_unavailable_default_never_inherit_confirmed_identity() {
    let inputs = Inputs::new();
    let mut registry = VoiceRegistry::new();
    assert!(matches!(registry.default_selection(), Err(VoiceError::MissingDefault)));
    let admitted = registry.admit_with("voice-sn", "童声01（user-entered label）", inputs.raw("unrecognized", b"unknown", b"mixed", None), registry_proof).unwrap();
    assert!(!admitted.historical_identity_confirmed);
    let project = Project::new_empty();
    assert!(matches!(registry.resolve_track(&project, &project.tracks[0].id), Err(VoiceError::IdentityMismatch)));
    let before = registry.default_reference().map(str::to_owned);
    assert!(registry.set_default(Some("not-installed")).is_err());
    assert_eq!(registry.default_reference(), before.as_deref());
    assert!(registry.admit_with("voice-sn", "Duplicate", inputs.raw("duplicate", b"x", b"y", None), registry_proof).is_err());
}

#[test]
fn admitted_source_attestation_cannot_relabel_a_mixed_raw_pair() {
    let inputs = Inputs::new();
    let assets = inputs.raw("mixed", b"unrecognized-token", b"unrecognized-model", None);
    let mut registry = VoiceRegistry::new();
    let error = registry.admit_with("mixed", "User-entered historical label", assets, |_| Ok(VoiceAdmission {
        source_sha256: BTreeMap::from([("sn16k.tkn".into(), "a".repeat(64)), ("sn16k.bin".into(), "b".repeat(64))]),
        ..Default::default()
    })).unwrap_err();
    assert!(matches!(error, VoiceError::InvalidConfig(_)));
    assert!(registry.references().is_empty());
}

#[test]
fn modern_registry_pins_every_required_file_and_crosschecks_source_provenance() {
    let inputs = Inputs::new();
    let dir = inputs.root.join("modern");
    fs::create_dir_all(&dir).unwrap();
    let sources = BTreeMap::from([("sn16k.tkn".to_owned(), "a".repeat(64)), ("sn16k.bin".to_owned(), "b".repeat(64))]);
    let metadata = serde_json::to_vec(&serde_json::json!({"source_sha256":sources})).unwrap();
    for (name, bytes) in [("voice.json", metadata.as_slice()), ("topology.json", metadata.as_slice()),
        ("tensors.safetensors", b"registry pin fixture".as_slice()), ("PROVENANCE.json", b"{}".as_slice()), ("SHA256SUMS", b"registry pin fixture".as_slice())] {
        fs::write(dir.join(name), bytes).unwrap();
    }
    let mut registry = VoiceRegistry::new();
    let assets = VoiceAssets { voice_model: Some(dir.clone()), ..VoiceAssets::default() };
    assert!(registry.admit_with("disagrees", "Source", assets.clone(), registry_proof).is_err());
    assert!(registry.references().is_empty());
    let selection = registry.admit_with("modern", "Modern source", assets, |_| Ok(VoiceAdmission { source_sha256: sources, ..Default::default() })).unwrap();
    assert!(!selection.historical_identity_confirmed);
    for role in ["voice_json", "voice_tensors", "voice_topology", "voice_provenance", "voice_checksums"] {
        assert!(selection.source_sha256.contains_key(role));
    }
    fs::write(dir.join("PROVENANCE.json"), b"changed").unwrap();
    assert!(matches!(registry.selection("modern"), Err(VoiceError::SourceChanged(_))));
}
