//! Read-only release resources have a separate admission from local imports.
//! Only a policy digest compiled into this worker can authorize a bundle; the
//! resource directory and the package's own rights declarations cannot do so.
use std::{collections::{BTreeMap, BTreeSet}, fs::{self, File}, io::Read, path::{Path, PathBuf}, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use verdant_backend::voices::{self, VoicePackage};

use crate::{Config, runtime};

const MAX_POLICY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;
const PROVENANCE_SCHEMA: &str = "verdant-bundled-voice-admission/1";
const POLICY_ROLE: &str = "bundled_voice_policy";

pub fn compiled_policy_sha256() -> Option<&'static str> {
    option_env!("VERDANT_BUNDLED_VOICE_POLICY_SHA256")
}

/// A runtime record of an independently pinned redistribution admission. This
/// is never serialized as the local-use SourceReceipt or an original attestation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    schema: String,
    policy_sha256: String,
    reference: String,
    voice_package: VoicePackage,
    source_sha256: BTreeMap<String, String>,
    cache_identity: String,
    historical_identity_confirmed: bool,
}

#[derive(Debug)]
pub struct Bundle {
    directory: PathBuf,
    provenance: Provenance,
}

// These fields match verdant-release-artifact-policy/1. The worker independently
// checks the packaged policy rather than trusting a packager's success marker.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile { role: String, path: String, bytes: u64, sha256: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence { path: String, bytes: u64, sha256: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantedArtifact { role: String, sha256: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    id: String, rights_holder: Option<String>, release_authority: Option<String>,
    permission: String, artifact_specific: bool,
    evidence: Evidence, artifacts: Vec<GrantedArtifact>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String, name: String, version: String, compatible_app_version: String,
    authorization_status: String, files: Vec<PolicyFile>, grants: Vec<Grant>,
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn portable_path(value: &str) -> Result<()> {
    ensure!(!value.is_empty() && !value.starts_with('/') && !value.contains(['\\', ':'])
        && !value.chars().any(char::is_control)
        && value.split('/').all(|part| !matches!(part, "" | "." | "..")),
        "unsafe bundled voice path: {value}");
    Ok(())
}

fn required_roles() -> impl Iterator<Item = &'static str> {
    voices::PACKAGE_FILES.iter().map(|(role, _)| *role)
}

impl Policy {
    fn validate(&self) -> Result<()> {
        ensure!(self.schema == "verdant-release-artifact-policy/1" && self.name == voices::VOICE_PACKAGE_NAME,
            "bundled voice policy has an unsupported schema or package name");
        ensure!(self.authorization_status == "granted",
            "bundled voice policy lacks granted artifact-specific redistribution authorization");
        ensure!(voices::valid_voice_version(&self.version) && self.version == self.compatible_app_version
            && self.compatible_app_version == env!("CARGO_PKG_VERSION"),
            "bundled voice policy is incompatible with this worker version");
        let mut roles = BTreeMap::new();
        let mut paths = BTreeSet::new();
        let (mut licenses, mut notices) = (0, 0);
        for file in &self.files {
            portable_path(&file.path)?;
            ensure!(valid_sha(&file.sha256) && (1..=MAX_ARTIFACT_BYTES).contains(&file.bytes)
                && paths.insert(file.path.as_str()) && roles.insert(file.role.as_str(), file).is_none(),
                "invalid/repeated bundled voice artifact role or path: {}", file.role);
            if let Some((_, expected)) = voices::PACKAGE_FILES.iter().find(|(role, _)| *role == file.role) {
                ensure!(file.path == *expected, "bundled voice role {} must use {expected}", file.role);
            } else {
                let (kind, id) = file.role.split_once(':').context("extra bundle roles must be license:ID, notice:ID or authorization:ID")?;
                ensure!(!id.is_empty() && matches!(kind, "license" | "notice" | "authorization")
                    && matches!(Path::new(&file.path).extension().and_then(|value| value.to_str()), Some("txt" | "md" | "json")),
                    "unapproved extra bundled voice artifact: {}", file.role);
                licenses += usize::from(kind == "license");
                notices += usize::from(kind == "notice");
            }
        }
        ensure!(licenses > 0 && notices > 0, "bundled voice policy requires licenses and attribution notices");
        for role in required_roles() {
            ensure!(roles.contains_key(role), "bundled voice policy omits required artifact role {role}");
        }
        let mut ids = BTreeSet::new();
        let mut granted = BTreeSet::new();
        for grant in &self.grants {
            let authority = match (&grant.rights_holder, &grant.release_authority) {
                (Some(authority), None) | (None, Some(authority)) => !authority.trim().is_empty(),
                _ => false,
            };
            ensure!(authority && !grant.id.trim().is_empty() && ids.insert(grant.id.as_str())
                && grant.permission == "redistribute-bundled-runtime" && grant.artifact_specific && !grant.artifacts.is_empty(),
                "bundled voice grant requires an identified issuer and artifact-specific redistribute-bundled-runtime permission");
            let evidence = self.files.iter().find(|file| file.path == grant.evidence.path)
                .context("bundled voice grant evidence must be included in the bundle")?;
            ensure!(evidence.role.starts_with("authorization:") && evidence.bytes == grant.evidence.bytes
                && evidence.sha256 == grant.evidence.sha256,
                "bundled voice grant evidence lacks its exact authorization role, size or digest");
            let mut grant_roles = BTreeSet::new();
            for artifact in &grant.artifacts {
                let file = roles.get(artifact.role.as_str()).context("bundled voice grant covers an undeclared artifact role")?;
                ensure!(file.sha256 == artifact.sha256 && grant_roles.insert(artifact.role.as_str()),
                    "bundled voice grant does not cover the exact artifact digest: {}", artifact.role);
                granted.insert(artifact.role.as_str());
            }
        }
        for role in required_roles() {
            ensure!(granted.contains(role), "bundled voice lacks an artifact-specific redistribution grant for {role}");
        }
        Ok(())
    }

    fn verify_files(&self, directory: &Path) -> Result<BTreeMap<String, String>> {
        regular_directory(directory)?;
        let mut pending = vec![directory.to_owned()];
        let mut actual = BTreeSet::new();
        let expected: BTreeSet<_> = self.files.iter().map(|file| Path::new(&file.path)).collect();
        while let Some(parent) = pending.pop() {
            regular_directory(&parent)?;
            for entry in fs::read_dir(parent)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                let path = entry.path();
                if kind.is_dir() { pending.push(path); }
                else if kind.is_file() {
                    let relative = path.strip_prefix(directory)?.to_owned();
                    ensure!(expected.contains(relative.as_path()), "bundled voice contains an undeclared file");
                    actual.insert(relative);
                } else { bail!("bundled voice contains a symlink or special file"); }
            }
        }
        ensure!(actual.len() == self.files.len(), "bundled voice contains missing artifacts");
        let mut hashes = BTreeMap::new();
        for file in &self.files {
            verify_file(&directory.join(&file.path), file.bytes, &file.sha256)
                .with_context(|| format!("bundled voice artifact changed: {}", file.role))?;
            hashes.insert(file.role.clone(), file.sha256.clone());
        }
        Ok(hashes)
    }
}

fn reject_symlink_ancestors(path: &Path) -> Result<()> {
    for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        ensure!(!fs::symlink_metadata(ancestor)?.file_type().is_symlink(),
            "bundled voice resources cannot contain symlink paths");
    }
    Ok(())
}

fn regular_directory(path: &Path) -> Result<()> {
    reject_symlink_ancestors(path)?;
    ensure!(fs::symlink_metadata(path)?.is_dir(), "bundled voice resources require a regular directory");
    Ok(())
}

fn regular(path: &Path) -> Result<File> {
    reject_symlink_ancestors(path)?;
    let metadata = fs::symlink_metadata(path)?;
    ensure!(metadata.is_file(), "bundled voice input must be a regular file");
    let file = File::open(path)?;
    ensure!(file.metadata()?.is_file(), "bundled voice input must be a regular file");
    Ok(file)
}

fn verify_file(path: &Path, expected_bytes: u64, expected_sha256: &str) -> Result<()> {
    let mut input = regular(path)?;
    ensure!(input.metadata()?.len() == expected_bytes, "artifact size differs from its admitted size");
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 { break; }
        bytes += count as u64;
        ensure!(bytes <= expected_bytes, "artifact grew during admission");
        hash.update(&buffer[..count]);
    }
    ensure!(bytes == expected_bytes && hex::encode(hash.finalize()) == expected_sha256,
        "artifact SHA-256 differs from its admission");
    reject_symlink_ancestors(path)?;
    Ok(())
}

fn read_policy(path: &Path, expected: &str) -> Result<(Policy, u64)> {
    let file = regular(path)?;
    let size = file.metadata()?.len();
    ensure!(size <= MAX_POLICY_BYTES, "bundled voice policy exceeds the admission size limit");
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(MAX_POLICY_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 == size && hex::encode(Sha256::digest(&bytes)) == expected,
        "bundled voice policy differs from the SHA-256 compiled into this worker");
    // Every object has a typed struct with deny_unknown_fields; serde rejects
    // duplicate fields at every nesting level without a lossy Value preparse.
    let policy: Policy = serde_json::from_slice(&bytes).context("invalid bundled voice policy")?;
    policy.validate()?;
    Ok((policy, size))
}

struct Resources { directory: PathBuf, policy_path: PathBuf, policy_bytes: u64, policy: Policy, sha256: &'static str }
impl Resources {
    fn hashes(&self) -> Result<BTreeMap<String, String>> {
        let mut hashes = self.policy.verify_files(&self.directory)?;
        verify_file(&self.policy_path, self.policy_bytes, self.sha256).context("bundled voice policy changed after admission")?;
        hashes.insert(POLICY_ROLE.into(), self.sha256.into());
        Ok(hashes)
    }

    fn bundle(&self, package: &verdant_synthesis::model::VoicePackage, mut hashes: BTreeMap<String, String>) -> Result<Bundle> {
        ensure!(package.voice_version == self.policy.version && package.source_manifest_sha256() == hashes["verdant_voice_manifest"],
            "bundled voice root manifest differs from the authorized version or digest");
        for (role, hash) in &package.hashes {
            ensure!(hashes.get(role) == Some(hash), "bundled voice package admission differs from authorized role {role}");
        }
        let records = package.manifest["records"].as_object().context("bundled voice manifest lacks source records")?;
        for (kind, record) in records {
            let policy_kind = match kind.as_str() { "conversion_notice" => "notice", "source_use_evidence" => "authorization", kind => kind };
            let file = self.policy.files.iter().find(|file| record["file"] == file.path.as_str() &&
                file.role.split_once(':').is_some_and(|(role_kind, _)| role_kind == policy_kind))
                .context("bundled voice record is missing its matching policy role")?;
            ensure!(record["bytes"].as_u64() == Some(file.bytes) && record["sha256"] == file.sha256.as_str(),
                "bundled voice record differs from its policy digest: {kind}");
        }
        hashes.extend(runtime::voice_sources(&package.voice)?);
        let identity = serde_json::to_vec(&(PROVENANCE_SCHEMA, self.sha256, &hashes))?;
        Ok(Bundle { directory: self.directory.clone(), provenance: Provenance {
            schema: PROVENANCE_SCHEMA.into(), policy_sha256: self.sha256.into(), reference: format!("verdant-voice-{}", self.policy.version),
            voice_package: VoicePackage { name: self.policy.name.clone(), version: self.policy.version.clone() },
            source_sha256: hashes, cache_identity: hex::encode(Sha256::digest(identity)),
            historical_identity_confirmed: package.voice.source_identity_confirmed(),
        } })
    }
}

fn resources() -> Result<Option<Resources>> {
    let Some(directory) = verdant_backend::platform::resource_dir() else { return Ok(None); };
    let directory = runtime::absolute(&directory)?;
    let policy_path = directory.join("BUNDLE-POLICY.json");
    match fs::symlink_metadata(&policy_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("reading bundled voice policy"),
        Ok(_) => {},
    }
    ensure!(!directory.components().any(|part| matches!(part, std::path::Component::ParentDir)),
        "bundled voice resource path must not contain ..");
    let sha256 = compiled_policy_sha256().context("BUNDLE-POLICY.json is present but this worker has no compiled VERDANT_BUNDLED_VOICE_POLICY_SHA256 authorization pin; an environment variable cannot authorize a running binary")?;
    ensure!(valid_sha(sha256), "compiled bundled voice policy pin must be a lowercase SHA-256 digest");
    let (policy, policy_bytes) = read_policy(&policy_path, sha256)?;
    Ok(Some(Resources { directory: directory.join("bundled-voice"), policy_path, policy_bytes, policy, sha256 }))
}

/// Called only for a normal worker startup or for an already selected bundle in
/// a job child. No registry write or local-use receipt is produced.
pub fn discover(config: &Config) -> Result<Option<Bundle>> {
    let Some(resources) = resources()? else { return Ok(None); };
    let hashes = resources.hashes()?;
    let package = verdant_synthesis::model::load_voice_package_with_manifest_pin(&resources.directory,
        &hashes["verdant_voice_manifest"])?;
    let mut selected = config.clone();
    selected.original = None;
    selected.voice_package = Some(resources.directory.clone());
    let options = json!({"onnxruntime_library":runtime::onnxruntime_path(config)?});
    runtime::validate_voice_package(&selected, &package, &options)?;
    ensure!(resources.hashes()? == hashes, "bundled voice resources changed during graph admission");
    Ok(Some(resources.bundle(&package, hashes)?))
}

pub fn load_package(config: &Config) -> Result<verdant_synthesis::model::VoicePackage> {
    let resources = resources()?.context("selected bundled voice lacks a compiled artifact policy")?;
    ensure!(runtime::voice_runtime(config)? == resources.directory.as_path(),
        "selected bundled voice directory differs from the policy-bound resources");
    let manifest = resources.policy.files.iter().find(|file| file.role == "verdant_voice_manifest")
        .context("compiled bundle policy lacks a root manifest digest")?;
    verdant_synthesis::model::load_voice_package_with_manifest_pin(&resources.directory, &manifest.sha256)
}

impl Bundle {
    pub fn reference(&self) -> &str { &self.provenance.reference }

    fn selection(&self) -> Value {
        json!({"reference":self.provenance.reference,"source_sha256":self.provenance.source_sha256,
            "cache_identity":self.provenance.cache_identity,"historical_identity_confirmed":self.provenance.historical_identity_confirmed,
            "voice_package":self.provenance.voice_package,"bundled_voice_admission":self.provenance})
    }

    pub fn select(&self, config: &Config) -> Config {
        let mut selected = config.clone();
        selected.original = None;
        selected.voice_package = Some(self.directory.clone());
        selected.selection = Some(self.selection());
        selected.bundled_voice = Some(self.provenance.clone());
        selected
    }

    fn matches(&self, config: &Config) -> Result<()> {
        ensure!(config.original.is_none()
            && config.voice_package.as_ref() == Some(&self.directory)
            && config.bundled_voice.as_ref() == Some(&self.provenance)
            && config.selection.as_ref() == Some(&self.selection()),
            "bundled voice configuration differs from its independently verified admission");
        Ok(())
    }
}

pub fn validate_selection(config: &Config, package: &verdant_synthesis::model::VoicePackage) -> Result<()> {
    let resources = resources()?.context("selected bundled voice is missing its compiled policy or package resources")?;
    let hashes = resources.hashes()?;
    resources.bundle(package, hashes)?.matches(config)
}

pub fn asset_hashes(config: &Config) -> Result<BTreeMap<String, String>> {
    let provenance = config.bundled_voice.as_ref().context("bundled voice admission is missing")?;
    let resources = resources()?.context("selected bundled voice is missing its compiled policy or package resources")?;
    let hashes = resources.hashes()?;
    ensure!(provenance.schema == PROVENANCE_SCHEMA && provenance.policy_sha256 == resources.sha256
        && provenance.voice_package.name == resources.policy.name && provenance.voice_package.version == resources.policy.version
        && hashes.iter().all(|(role, hash)| provenance.source_sha256.get(role) == Some(hash)),
        "bundled voice snapshot differs from its compiled policy and authorized artifacts");
    Bundle { directory: resources.directory, provenance: provenance.clone() }.matches(config)?;
    Ok(hashes)
}

/// Serialized paths and provenance are inputs, never trust anchors. The child
/// repeats package integrity and live graph ABI admission using its own binary
/// pin before the existing request/asset snapshot checks execute.
pub fn restore_child(config: &mut Config) -> Result<()> {
    if config.bundled_voice.is_some() {
        let bundle = discover(config)?.context("selected bundled voice resources are unavailable in the job child")?;
        bundle.matches(config)?;
        config.bundle = Some(Arc::new(bundle));
    }
    Ok(())
}

pub fn extend_capabilities(config: &Config, result: &mut Value) -> Result<()> {
    let Some(bundle) = &config.bundle else { return Ok(()); };
    let selected = bundle.select(config);
    let hashes = asset_hashes(&selected)?;
    let catalog = result.as_object_mut().context("capabilities must be an object")?
        .entry("voices").or_insert_with(|| json!({"version":1,"default_voice":null,"voices":{}}));
    if catalog.get("error").is_some() { return Ok(()); }
    let voices = catalog["voices"].as_object_mut().context("voice capabilities must contain a catalog")?;
    if voices.contains_key(bundle.reference()) { return Ok(()); }
    let mut files: BTreeMap<_, _> = hashes.iter().map(|(role, hash)| (role.clone(), json!({"available":true,"sha256":hash}))).collect();
    files.insert("voice_source_admission".into(), json!({"available":true,"sha256":hashes["verdant_voice_manifest"]}));
    voices.insert(bundle.reference().into(), json!({"reference":bundle.reference(),
        "display_name":format!("verdant-voice {}", bundle.provenance.voice_package.version),"available":true,
        "languages":{"zh":"native","en":"native","ja":"approximate"},
        "cache_identity":bundle.provenance.cache_identity,"source_sha256":bundle.provenance.source_sha256,
        "historical_identity_confirmed":bundle.provenance.historical_identity_confirmed,
        "voice_package":bundle.provenance.voice_package,"files":files,"bundled_voice_admission":bundle.provenance}));
    if config.bundled_voice.is_some() && catalog["default_voice"].is_null() {
        catalog["default_voice"] = bundle.reference().into();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().canonicalize().unwrap().join(format!("verdant-bundle-policy-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    // Public marker digests exercise policy structure only. These declarations
    // do not supply model bytes, graph admission or a redistribution decision.
    fn policy() -> Value {
        let hash = "1".repeat(64);
        let mut files: Vec<_> = voices::PACKAGE_FILES.iter().map(|(role, path)|
            json!({"role":role,"path":path,"bytes":1,"sha256":hash})).collect();
        files.extend([json!({"role":"license:test","path":"LICENSE.txt","bytes":1,"sha256":hash}),
            json!({"role":"notice:test","path":"NOTICE.txt","bytes":1,"sha256":hash}),
            json!({"role":"authorization:test","path":"AUTHORIZATION.json","bytes":1,"sha256":hash})]);
        json!({"schema":"verdant-release-artifact-policy/1","name":"verdant-voice","version":env!("CARGO_PKG_VERSION"),
            "compatible_app_version":env!("CARGO_PKG_VERSION"),"authorization_status":"granted","files":files,
            "grants":[{"id":"test-structure-only","rights_holder":"test fixture","permission":"redistribute-bundled-runtime",
                "artifact_specific":true,"evidence":{"path":"AUTHORIZATION.json","bytes":1,"sha256":hash},
                "artifacts":required_roles().map(|role| json!({"role":role,"sha256":hash})).collect::<Vec<_>>()}]})
    }

    #[test]
    fn policy_requires_exact_grants_and_bound_authorization_evidence() {
        let value = policy();
        serde_json::from_value::<Policy>(value.clone()).unwrap().validate().unwrap();
        for change in ["status", "permission", "specific", "evidence", "artifact", "missing"] {
            let mut invalid = value.clone();
            match change {
                "status" => invalid["authorization_status"] = "pending".into(),
                "permission" => invalid["grants"][0]["permission"] = "local-source-use".into(),
                "specific" => invalid["grants"][0]["artifact_specific"] = false.into(),
                "evidence" => invalid["grants"][0]["evidence"]["sha256"] = "2".repeat(64).into(),
                "artifact" => invalid["grants"][0]["artifacts"][0]["sha256"] = "2".repeat(64).into(),
                _ => { invalid["grants"][0]["artifacts"].as_array_mut().unwrap().pop(); },
            }
            assert!(serde_json::from_value::<Policy>(invalid).unwrap().validate().is_err(), "{change}");
        }
    }

    #[test]
    fn policy_rejects_unsafe_duplicate_or_incompatible_artifacts() {
        for path in ["../bank.json", "/bank.json", "bank\\bank.json", "C:bank.json", "a//bank.json", "a/./bank.json"] {
            assert!(portable_path(path).is_err(), "{path}");
        }
        let mut value = policy();
        let duplicate = value["files"][0].clone();
        value["files"].as_array_mut().unwrap().push(duplicate);
        assert!(serde_json::from_value::<Policy>(value).unwrap().validate().is_err());
        let mut value = policy();
        value["compatible_app_version"] = "999.0.0".into();
        assert!(serde_json::from_value::<Policy>(value).unwrap().validate().is_err());
        let mut value = policy();
        value["files"][0]["bytes"] = (MAX_ARTIFACT_BYTES + 1).into();
        assert!(serde_json::from_value::<Policy>(value).unwrap().validate().is_err());
    }

    #[test]
    fn policy_requires_root_graph_paths_and_keeps_legal_extras() {
        for role in required_roles().chain(["license:test", "notice:test", "authorization:test"]) {
            let mut missing = policy();
            missing["files"].as_array_mut().unwrap().retain(|file| file["role"] != role);
            assert!(serde_json::from_value::<Policy>(missing).unwrap().validate().is_err(), "{role}");
        }
        let mut nested = policy();
        nested["files"][0]["path"] = "verdant-voice/manifest.json".into();
        assert!(serde_json::from_value::<Policy>(nested).unwrap().validate().is_err());
    }

    #[test]
    fn policy_parser_rejects_duplicate_fields_and_trailing_json() {
        assert!(serde_json::from_slice::<Policy>(br#"{"schema":"a","schema":"b"}"#).is_err());
        let mut bytes = serde_json::to_vec(&policy()).unwrap();
        bytes.extend_from_slice(b" {}");
        assert!(serde_json::from_slice::<Policy>(&bytes).is_err());
    }

    #[test]
    fn file_snapshot_rejects_changed_bytes_and_sizes() {
        let fixture = Fixture::new();
        let path = fixture.0.join("public-marker.txt");
        let digest = hex::encode(Sha256::digest(b"public marker"));
        fs::write(&path, b"public marker").unwrap();
        verify_file(&path, 13, &digest).unwrap();
        fs::write(&path, b"changed bytes").unwrap();
        assert!(verify_file(&path, 13, &digest).is_err());
        fs::write(&path, b"short").unwrap();
        assert!(verify_file(&path, 13, &digest).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn file_snapshot_rejects_symlink_leaves_and_ancestors() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let directory = fixture.0.join("real");
        fs::create_dir(&directory).unwrap();
        let path = directory.join("public-marker.txt");
        fs::write(&path, b"public marker").unwrap();
        let digest = hex::encode(Sha256::digest(b"public marker"));
        let alias = fixture.0.join("alias.txt");
        symlink(&path, &alias).unwrap();
        assert!(verify_file(&alias, 13, &digest).is_err());
        let alias = fixture.0.join("alias-directory");
        symlink(&directory, &alias).unwrap();
        assert!(verify_file(&alias.join("public-marker.txt"), 13, &digest).is_err());
    }
}
