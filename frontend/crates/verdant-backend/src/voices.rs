//! Local singer registry for user-supplied source assets.
//!
//! Project files contain stable voice references and source hashes, never local
//! asset paths. Installation runs the worker's full model validator
//! before changing the registry, freezes every consumed file hash, and checks
//! again after validation. Resolution refuses changed bytes. Names and VID
//! fields cannot establish historical source identity or commercial parity.
//! Reimporting the same reference can relocate identical admitted assets after
//! full validation; it cannot replace weights, source identity or asset roles.
//!
//! The worker supplies the real DSP validator through admit_with/install_with;
//! this crate deliberately does not load native DSP or ONNX into the editor.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use verdant_domain::{Command, CommandBatch, Project, TrackId};
pub use verdant_voice_format::{PACKAGE_FILES, RECORD_FILES};

const REGISTRY_VERSION: u32 = 1;
const KNOWN_TOKEN_SHA: &str = "821b7cd9e7b489077dd55bf581ba90e1dffd3816c6fedcedc320744b498deead";
const KNOWN_MODEL_SHA: &str = "213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff";
/// Desktop mode for the exact user-supplied original raw source pair.
pub const DESKTOP_ORIGINAL_REFERENCE: &str = "verdant-original-sn16k";
pub const VOICE_PACKAGE_NAME: &str = "verdant-voice";
pub const VOICE_PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESKTOP_VOICE_REFERENCE: &str = concat!("verdant-voice-", env!("CARGO_PKG_VERSION"));
pub const VOICE_STRATEGY: &str = "native-mlpg-reference-en-aa-boundary-sustain-v2";

/// Accept only available raw roles whose hashes identify the original bytes.
/// Source claims and filenames cannot substitute for these runtime file roles.
pub fn is_desktop_original(voice: &crate::wire::VoiceEntry) -> bool {
    voice.available && original_raw_hashes(
        voice.files.get("token").filter(|file| file.available).and_then(|file| file.sha256.as_deref()),
        voice.files.get("model").filter(|file| file.available).and_then(|file| file.sha256.as_deref()),
    ) && !voice.files.keys().any(|role| role.starts_with("verdant_voice_")
        || role == "voice_source_admission" || VOICE_FILES.iter().any(|(key, _)| role.as_str() == *key))
        && !voice.files.contains_key("voice_json")
}

fn original_raw_hashes(token: Option<&str>, model: Option<&str>) -> bool {
    token.is_some_and(|hash| hash.eq_ignore_ascii_case(KNOWN_TOKEN_SHA))
        && model.is_some_and(|hash| hash.eq_ignore_ascii_case(KNOWN_MODEL_SHA))
}

fn validate_reserved_reference(reference: &str, assets: &VoiceAssets, files: &BTreeMap<String, String>) -> Result<(), VoiceError> {
    if reference == DESKTOP_ORIGINAL_REFERENCE && (assets.voice_package.is_some() || assets.voice_model.is_some()
        || !original_raw_hashes(files.get("token").map(String::as_str), files.get("model").map(String::as_str))) {
        return Err(VoiceError::AdmissionRejected("reserved desktop original reference requires the exact original raw token/model bytes".into()));
    }
    if reference.strip_prefix("verdant-voice-").is_some_and(|version| !valid_voice_version(version))
        || reference.starts_with("verdant-voice-") && (
        assets.voice_package.is_none() || PACKAGE_FILES.iter()
            .any(|(role, _)| !files.contains_key(*role))) {
        return Err(VoiceError::AdmissionRejected("reserved voice reference requires a complete versioned verdant-voice package".into()));
    }
    Ok(())
}
const MODERN_FILES: &[(&str, &str)] = &[
    ("voice_json", "voice.json"), ("voice_tensors", "tensors.safetensors"),
    ("voice_topology", "topology.json"), ("voice_provenance", "PROVENANCE.json"),
    ("voice_checksums", "SHA256SUMS"),
];
/// Historical source-package files, used only to identify legacy layouts.
/// Current runtime packages use PACKAGE_FILES and require explicit conversion.
pub const VOICE_FILES: &[(&str, &str)] = &[
    ("voice_source_manifest", "voice-source.json"), ("bank_json", "bank.json"),
    ("bank_tensors", "bank.safetensors"), ("bank_topology", "topology.json"),
    ("bank_token", "source.tkn"), ("source_controller", "controller.onnx"),
    ("source_decoder", "decoder.onnx"), ("source_rights", "rights.json"),
    ("source_export", "export.json"), ("source_contracts", "contracts.json"),
];

/// Runtime extension files nested under `verdant-voice/` in legacy packages.
pub const VOICE_RUNTIME_FILES: &[(&str, &str)] = &[
    ("verdant_voice_manifest", "manifest.json"),
    ("verdant_voice_context_controller", "context-controller.onnx"),
    ("verdant_voice_local_controller", "local-controller.onnx"),
    ("verdant_voice_decoder", "decoder-48k.onnx"),
    ("verdant_voice_phone_table", "native-phone-table.json"),
];

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VoicePackage {
    pub name: String,
    pub version: String,
}

pub fn voice_reference_matches_package(reference: &str, voice: &crate::wire::VoiceEntry) -> bool {
    !reference.starts_with("verdant-voice-") || voice.voice_package.as_ref().is_some_and(|package|
        package.name == VOICE_PACKAGE_NAME && valid_voice_version(&package.version)
            && reference.strip_prefix("verdant-voice-") == Some(package.version.as_str()))
}

pub fn valid_voice_version(version: &str) -> bool {
    semver::Version::parse(version).is_ok()
}

/// Current software admits only the canonical package root and runtime manifest.
pub fn package_files(directory: &Path) -> Result<&'static [(&'static str, &'static str)], VoiceError> {
    let (manifest, _) = read_package_manifest(directory)?;
    for (role, artifact) in manifest.artifacts.iter().chain(&manifest.records) {
        check_package_file(&directory.join(&artifact.file), role, artifact.bytes)?;
    }
    Ok(PACKAGE_FILES)
}

fn check_package_file(path: &Path, role: &str, expected_bytes: u64) -> Result<(), VoiceError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() != expected_bytes {
        return Err(VoiceError::SourceChanged(role.into()));
    }
    Ok(())
}

fn read_package_manifest(directory: &Path) -> Result<(verdant_voice_format::Manifest, Vec<u8>), VoiceError> {
    verdant_voice_format::read_manifest(directory)
        .map_err(|error| VoiceError::InvalidConfig(format!("invalid verdant-voice package: {error:#}")))
}

/// Snapshot each canonical asset once and check graph bytes against the root
/// manifest. This is integrity validation; original-source/ABI admission still
/// belongs to the worker's full validator.
pub fn package_hashes(directory: &Path) -> Result<BTreeMap<String, String>, VoiceError> {
    Ok(package_snapshot(directory)?.0)
}

fn package_snapshot(directory: &Path) -> Result<(BTreeMap<String, String>, VoicePackage), VoiceError> {
    let (manifest, bytes) = read_package_manifest(directory)?;
    let mut hashes = BTreeMap::from([("verdant_voice_manifest".into(), hex::encode(Sha256::digest(&bytes)))]);
    for (role, artifact) in manifest.artifacts.iter().chain(&manifest.records) {
        let path = directory.join(&artifact.file);
        check_package_file(&path, role, artifact.bytes)?;
        let (hash, size) = hash_file_snapshot(&path, Some(artifact.bytes))?;
        if size != artifact.bytes || hash != artifact.sha256 {
            return Err(VoiceError::SourceChanged(role.clone()));
        }
        if manifest.artifacts.contains_key(role) {
            hashes.insert(role.clone(), hash);
        }
    }
    Ok((hashes, VoicePackage { name: manifest.name, version: manifest.version }))
}
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceAssets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_package: Option<PathBuf>,
    /// Registry read compatibility for historical rows awaiting migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfinal_model: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_model: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Historical registry field. New admission never accepts this asset.
    pub vocoder: Option<PathBuf>,
}

/// Original files are supplied for admission only, never persisted as voice
/// runtime dependencies. The checkbox is a user statement, not a legal finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    pub token: PathBuf,
    pub model: PathBuf,
    pub rights_attested: bool,
}

impl SourceEvidence {
    pub fn verified_hashes(&self) -> Result<BTreeMap<String, String>, VoiceError> {
        if !self.rights_attested {
            return Err(VoiceError::AdmissionRejected("confirm the source-use attestation before importing the verdant-voice package".into()));
        }
        verify_original_raw_pair(&self.token, &self.model)
    }
}

/// Verifies the actual source bytes for the reserved original voice.
pub fn verify_original_raw_pair(token: &Path, model: &Path) -> Result<BTreeMap<String, String>, VoiceError> {
    let token = hash_file(token)?;
    let model = hash_file(model)?;
    if !original_raw_hashes(Some(&token), Some(&model)) {
        return Err(VoiceError::AdmissionRejected("the actual original token/binary bytes do not match this voice source bank".into()));
    }
    Ok(BTreeMap::from([("sn16k.tkn".into(), token), ("sn16k.bin".into(), model)]))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReceipt {
    pub schema: String,
    pub original_source_sha256: BTreeMap<String,String>,
    pub source_manifest_sha256: String,
    pub rights_attested: bool,
    pub attestation_is_legal_determination: bool,
}

impl SourceReceipt {
    /// Bind the native renderer to the imported root manifest and originals.
    /// A graph's own metadata or package rights statement
    /// never substitutes for the user's original-source import receipt.
    pub fn validate_for(
        &self,
        manifest_sha256: &str,
        original_source_sha256: &BTreeMap<String, String>,
    ) -> Result<(), VoiceError> {
        validate_receipt(self, manifest_sha256)?;
        if &self.original_source_sha256 != original_source_sha256 {
            return Err(VoiceError::InvalidConfig("voice admission and source fingerprints disagree".into()));
        }
        Ok(())
    }
}
/// Hash the actual user-selected originals and bind them to an already fully
/// verified verdant-voice package. The worker also loads the original pair and takes
/// a second snapshot before publishing the registry transaction.
pub fn verify_source_evidence(evidence:&SourceEvidence,manifest_sha256:&str)->Result<SourceReceipt,VoiceError>{
    let receipt=SourceReceipt{schema:"verdant-voice-source-admission/1".into(),original_source_sha256:evidence.verified_hashes()?,source_manifest_sha256:manifest_sha256.into(),rights_attested:true,attestation_is_legal_determination:false};
    validate_receipt(&receipt,manifest_sha256)?;Ok(receipt)
}

/// A package installation binds admitted content and records its installer version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageReceipt {
    pub schema: String,
    pub manifest_sha256: String,
    pub bank_sha256: String,
    pub app_version: String,
    /// Policy observed by the installer. Runtime trust comes from an independent pin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_policy_sha256: Option<String>,
}

impl PackageReceipt {
    pub fn new(manifest_sha256: String, bank_sha256: String, app_version: String) -> Self {
        Self {
            schema: "verdant-voice-package-installation/1".into(),
            manifest_sha256,
            bank_sha256,
            app_version,
            artifact_policy_sha256: None,
        }
    }

    pub fn validate_for(&self, manifest_sha256: &str, bank_sha256: &str, version: &str) -> Result<(), VoiceError> {
        if self.schema != "verdant-voice-package-installation/1"
            || !verdant_voice_format::valid_sha256(manifest_sha256) || self.manifest_sha256 != manifest_sha256
            || !verdant_voice_format::valid_sha256(bank_sha256) || self.bank_sha256 != bank_sha256
            || !valid_voice_version(version) || self.app_version != version
            || self.artifact_policy_sha256.as_deref().is_some_and(|hash| !verdant_voice_format::valid_sha256(hash)) {
            return Err(VoiceError::InvalidConfig("invalid voice package installation receipt".into()));
        }
        Ok(())
    }
}

/// Return this only after the real worker loader has passed admission. Source hashes are original token/model provenance from
/// that loader, not user-entered names or identity booleans. Unknown sources
/// may have an empty map; they never inherit the confirmed singer identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceAdmission {
    pub source_sha256: BTreeMap<String, String>,
    pub source_receipt: Option<SourceReceipt>,
    pub package_receipt: Option<PackageReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceSelection {
    pub reference: String,
    pub display_name: String,
    /// Declared acoustic support; `approximate` never means a native bank.
    pub languages: BTreeMap<String, String>,
    pub assets: VoiceAssets,
    /// Original source hashes plus all frozen runtime input hashes, by role.
    pub source_sha256: BTreeMap<String, String>,
    pub cache_identity: String,
    pub historical_identity_confirmed: bool,
    pub source_receipt: Option<SourceReceipt>,
    pub package_receipt: Option<PackageReceipt>,
    pub voice_package: Option<VoicePackage>,
}

impl VoiceSelection {
    /// Bind the selected bytes to an existing phrase/content digest.
    /// Paths and presentation names do not enter the cache identity.
    pub fn bind_cache_identity(&self, content_digest: &str) -> String {
        let mut hash = Sha256::new();
        hash.update(b"verdant-render-voice-v1\0");
        hash.update((content_digest.len() as u64).to_le_bytes());
        hash.update(content_digest.as_bytes());
        hash.update(self.cache_identity.as_bytes());
        hex::encode(hash.finalize())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredVoice {
    display_name: String,
    assets: VoiceAssets,
    files: BTreeMap<String, String>,
    #[serde(default)]
    source_sha256: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_receipt: Option<RegisteredSourceReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    package_receipt: Option<PackageReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum RegisteredSourceReceipt {
    Current(SourceReceipt),
    Historical(HistoricalSourceReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalSourceReceipt {
    schema: String,
    original_source_sha256: BTreeMap<String, String>,
    cfinal_manifest_sha256: String,
    rights_attested: bool,
    attestation_is_legal_determination: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceRegistry {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_voice: Option<String>,
    /// Only defaults previously chosen by a release installer may be upgraded
    /// automatically. An absent legacy marker is treated as a user's choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    managed_default: Option<String>,
    #[serde(default)]
    voices: BTreeMap<String, RegisteredVoice>,
}

impl Default for VoiceRegistry {
    fn default() -> Self { Self { version: REGISTRY_VERSION, default_voice: None, managed_default: None, voices: BTreeMap::new() } }
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("voice registry I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("voice registry JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid voice registry: {0}")]
    InvalidConfig(String),
    #[error("voice reference {0} is not installed")]
    UnknownReference(String),
    #[error("voice reference {0} is installed with different assets or source identity; choose a new stable reference for a different voicebank")]
    DuplicateReference(String),
    #[error("no default voice has been selected")]
    MissingDefault,
    #[error("voice admission failed: {0}")]
    AdmissionRejected(String),
    #[error("admitted voice asset {0} has changed; reinstall and admit the source")]
    SourceChanged(String),
    #[error("voice {0} uses a legacy package and requires explicit migration")]
    MigrationRequired(String),
    #[error("project source hash for {0} does not match the selected singer")]
    ProjectHashMismatch(String),
    #[error("project singer identity is confirmed but selected source bytes do not establish that identity")]
    IdentityMismatch,
    #[error("track {0} does not exist")]
    MissingTrack(String),
}

impl VoiceError {
    /// Safe machine-readable capability failure: the detailed local diagnostic
    /// may contain a private path, while a capability catalog never does.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "asset_io", Self::Json(_) => "invalid_json", Self::InvalidConfig(_) => "invalid_registry",
            Self::UnknownReference(_) => "unknown_voice", Self::DuplicateReference(_) => "duplicate_voice",
            Self::MissingDefault => "missing_default_voice", Self::AdmissionRejected(_) => "admission_rejected",
            Self::SourceChanged(_) => "source_changed", Self::ProjectHashMismatch(_) => "project_source_mismatch",
            Self::MigrationRequired(_) => "migration_required",
            Self::IdentityMismatch => "source_identity_mismatch", Self::MissingTrack(_) => "missing_track",
        }
    }
}

impl VoiceRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, VoiceError> {
        let path = path.as_ref();
        let mut registry: Self = serde_json::from_reader(File::open(path)?)?;
        if registry.version != REGISTRY_VERSION {
            return Err(VoiceError::InvalidConfig(format!("unsupported registry version {}", registry.version)));
        }
        let base = std::path::absolute(path)?.parent().unwrap_or(Path::new(".")).to_path_buf();
        for (reference, entry) in &mut registry.voices {
            validate_reference(reference)?;
            normalize_hashes(&mut entry.files);
            normalize_hashes(&mut entry.source_sha256);
            absolutize_assets(&mut entry.assets, &base);
            validate_registration(entry)?;
            if legacy_manifest_role(entry).is_none() {
                validate_reserved_reference(reference, &entry.assets, &entry.files)?;
            } else if reference == DESKTOP_ORIGINAL_REFERENCE {
                return Err(VoiceError::InvalidConfig("legacy package cannot use the reserved raw voice reference".into()));
            }
        }
        if let Some(reference) = &registry.default_voice {
            if !registry.voices.contains_key(reference) { return Err(VoiceError::UnknownReference(reference.clone())); }
        }
        if registry.managed_default.is_some() && registry.managed_default != registry.default_voice {
            return Err(VoiceError::InvalidConfig("managed default no longer matches the selected default".into()));
        }
        // Missing/drifted asset bytes are per-entry selection errors so one
        // unavailable singer does not hide the rest of the installed catalog.
        Ok(registry)
    }

    /// The app owns a registry path before the first singer is installed.
    /// Only an absent file means an empty catalog; malformed or unreadable
    /// registries must remain visible errors rather than losing registrations.
    pub fn load_or_empty(path:impl AsRef<Path>)->Result<Self,VoiceError>{
        match Self::load(path){
            Err(VoiceError::Io(error))if error.kind()==std::io::ErrorKind::NotFound=>Ok(Self::new()),
            result=>result,
        }
    }

    pub fn references(&self) -> Vec<String> { self.voices.keys().cloned().collect() }
    pub fn default_reference(&self) -> Option<&str> { self.default_voice.as_deref() }

    pub fn remove(&mut self, reference: &str) -> Result<(), VoiceError> {
        self.voices.remove(reference).ok_or_else(|| VoiceError::UnknownReference(reference.into()))?;
        if self.default_reference() == Some(reference) {
            self.default_voice = None;
            self.managed_default = None;
        }
        Ok(())
    }

    pub fn set_default(&mut self, reference: Option<&str>) -> Result<(), VoiceError> {
        if let Some(reference) = reference { self.selection(reference)?; }
        self.default_voice = reference.map(str::to_owned);
        self.managed_default = None;
        Ok(())
    }

    /// Change an empty or installer-managed default while preserving every
    /// explicit choice, including legacy registries without provenance.
    pub fn upgrade_default(&mut self, reference: &str) -> Result<bool, VoiceError> {
        self.selection(reference)?;
        Ok(self.upgrade_admitted_default(reference))
    }

    fn upgrade_admitted_default(&mut self, reference: &str) -> bool {
        if self.default_voice.is_some() && self.default_voice != self.managed_default { return false; }
        self.default_voice = Some(reference.to_owned());
        self.managed_default = Some(reference.to_owned());
        true
    }

    /// No registry mutation happens until the full validator and both source
    /// snapshots succeed. There is intentionally no hash-only install method.
    /// An existing reference admits only byte-identical relocation/reimport.
    /// Its label and identity stay fixed; only canonical asset paths may move.
    pub fn admit_with(
        &mut self, reference: impl Into<String>, display_name: impl Into<String>, assets: VoiceAssets,
        validator: impl FnOnce(&VoiceAssets) -> Result<VoiceAdmission, String>,
    ) -> Result<VoiceSelection, VoiceError> {
        let reference = reference.into();
        validate_reference(&reference)?;
        let display_name = display_name.into();
        if display_name.trim().is_empty() { return Err(VoiceError::InvalidConfig("display name must not be empty".into())); }
        if assets.vocoder.is_some() {
            return Err(VoiceError::AdmissionRejected("vocoder assets are retired".into()));
        }
        let assets = canonical_assets(assets)?;
        let (before, package) = asset_snapshot(&assets)?;
        validate_reserved_reference(&reference, &assets, &before)?;
        if let Some(package) = package {
            if package.version != VOICE_PACKAGE_VERSION
                || reference.starts_with("verdant-voice-") && reference != format!("verdant-voice-{}", package.version) {
                return Err(VoiceError::AdmissionRejected("new voice admission must match the executable version and reserved reference".into()));
            }
        }
        let mut admission = validator(&assets).map_err(VoiceError::AdmissionRejected)?;
        validate_source_hashes(&admission.source_sha256)?;
        normalize_hashes(&mut admission.source_sha256);
        let (after, package) = asset_snapshot(&assets)?;
        if before != after { return Err(VoiceError::SourceChanged("source changed during admission".into())); }
        validate_reserved_reference(&reference, &assets, &after)?;
        let mut entry = RegisteredVoice { display_name, assets, files: after, source_sha256: admission.source_sha256,
            source_receipt: admission.source_receipt.map(RegisteredSourceReceipt::Current), package_receipt: admission.package_receipt };
        validate_registration(&entry)?;
        if let Some(previous) = self.voices.get(&reference) {
            // The old directory may already have moved. Compare the frozen
            // admission record, not files at the unavailable former paths.
            // The new candidate still passes the full validator above,
            // including original-byte evidence and attestation for verdant-voice.
            validate_registration(previous)?;
            if entry.files != previous.files || entry.source_sha256 != previous.source_sha256
                || entry.source_receipt != previous.source_receipt || entry.package_receipt != previous.package_receipt {
                return Err(VoiceError::DuplicateReference(reference));
            }
            entry.display_name = previous.display_name.clone();
        }
        let selection = selection_from(&reference, &entry, package)?;
        self.voices.insert(reference, entry);
        Ok(selection)
    }

    /// A persistent install is one transaction: if validation or the atomic
    /// save fails, both the existing file and this in-memory registry survive.
    pub fn install_with(
        &mut self, path: impl AsRef<Path>, reference: impl Into<String>, display_name: impl Into<String>, assets: VoiceAssets,
        validator: impl FnOnce(&VoiceAssets) -> Result<VoiceAdmission, String>,
    ) -> Result<VoiceSelection, VoiceError> {
        let mut candidate = self.clone();
        let selection = candidate.admit_with(reference, display_name, assets, validator)?;
        if selection.reference == DESKTOP_VOICE_REFERENCE {
            candidate.upgrade_admitted_default(&selection.reference);
        }
        candidate.save_atomic(path)?;
        *self = candidate;
        Ok(selection)
    }

    pub fn resolve(&self, reference: &str) -> Result<VoiceAssets, VoiceError> { Ok(self.selection(reference)?.assets) }

    pub fn selection(&self, reference: &str) -> Result<VoiceSelection, VoiceError> {
        if self.version != REGISTRY_VERSION { return Err(VoiceError::InvalidConfig("unsupported registry version".into())); }
        let entry = self.voices.get(reference).ok_or_else(|| VoiceError::UnknownReference(reference.into()))?;
        validate_registration(entry)?;
        if legacy_manifest_role(entry).is_some() || entry.assets.vocoder.is_some() {
            return Err(VoiceError::MigrationRequired(reference.into()));
        }
        let (current, package) = asset_snapshot(&entry.assets)?;
        for (role, expected) in &entry.files {
            if current.get(role) != Some(expected) { return Err(VoiceError::SourceChanged(role.clone())); }
        }
        if let Some(role) = current.keys().find(|role| !entry.files.contains_key(*role)) {
            return Err(VoiceError::SourceChanged(role.clone()));
        }
        selection_from(reference, entry, package)
    }

    pub fn default_selection(&self) -> Result<VoiceSelection, VoiceError> {
        self.selection(self.default_voice.as_deref().ok_or(VoiceError::MissingDefault)?)
    }

    /// Resolve the authored stable reference and check any project source pins.
    /// None preserves the caller's explicitly configured legacy default; an
    /// unknown nonempty reference is always a visible selection error.
    pub fn resolve_track(&self, project: &Project, track_id: &TrackId) -> Result<Option<VoiceSelection>, VoiceError> {
        let track = project.track(track_id).ok_or_else(|| VoiceError::MissingTrack(track_id.to_string()))?;
        let Some(reference) = track.voice_ref.as_deref() else { return Ok(None); };
        let selection = self.selection(reference)?;
        if let Some(expected) = project.voice_refs.get(reference) {
            for (role, source_key, hash) in [("token", "sn16k.tkn", expected.token_sha256.as_deref()),
                ("model", "sn16k.bin", expected.model_sha256.as_deref())] {
                if let Some(hash) = hash {
                    let actual = selection.source_sha256.get(source_key).or_else(|| selection.source_sha256.get(role));
                    if !actual.is_some_and(|actual| actual.eq_ignore_ascii_case(hash)) {
                        return Err(VoiceError::ProjectHashMismatch(role.into()));
                    }
                }
            }
            if expected.historical_identity_confirmed && !selection.historical_identity_confirmed { return Err(VoiceError::IdentityMismatch); }
        }
        if let Some(pin) = track.unknown.get("voice_selection") {
            let hashes: BTreeMap<String, String> = serde_json::from_value(pin.get("source_sha256").cloned().unwrap_or(Value::Null))?;
            if pin["schema"] != "verdant-track-voice/1" || pin["reference"] != reference
                || pin["cache_identity"] != selection.cache_identity || hashes != selection.source_sha256 {
                return Err(VoiceError::ProjectHashMismatch("voice_selection".into()));
            }
        }
        Ok(Some(selection))
    }

    /// No absolute paths, source locators or validator diagnostics are exposed.
    /// An unavailable entry reports its typed error code and retains its ID.
    pub fn capabilities(&self) -> Value {
        let mut voices = serde_json::Map::new();
        for (reference, entry) in &self.voices {
            let value = match self.selection(reference) {
                Ok(selection) => {
                    let mut files: BTreeMap<_, _> = entry.files.iter().map(|(role, hash)| (role.clone(), serde_json::json!({"available":true,"sha256":hash}))).collect();
                    if entry.source_receipt.is_some() || entry.package_receipt.is_some() {
                        files.insert("voice_source_admission".into(), serde_json::json!({"available":true,
                            "sha256":entry.files["verdant_voice_manifest"]}));
                    }
                    let mut value = serde_json::json!({"reference":reference,"display_name":entry.display_name,"available":true,
                        "languages":selection.languages,
                        "cache_identity":selection.cache_identity,"source_sha256":selection.source_sha256,
                        "historical_identity_confirmed":selection.historical_identity_confirmed,"files":files});
                    if let Some(package) = selection.voice_package { value["voice_package"] = serde_json::to_value(package).expect("voice package is serializable"); }
                    value
                }
                Err(error) => serde_json::json!({"reference":reference,"display_name":entry.display_name,"available":false,
                    "historical_identity_confirmed":false,"error":error.code()}),
            };
            voices.insert(reference.clone(), value);
        }
        serde_json::json!({"version":REGISTRY_VERSION,"default_voice":self.default_voice,"voices":voices})
    }

    pub fn save_atomic(&self, path: impl AsRef<Path>) -> Result<(), VoiceError> {
        let path = path.as_ref();
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let name = path.file_name().ok_or_else(|| VoiceError::InvalidConfig("registry path needs a file name".into()))?.to_string_lossy();
        let temporary = parent.join(format!(".{name}.tmp.{}.{}", std::process::id(), TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)));
        let body = serde_json::to_vec_pretty(self)?;
        let result = (|| -> Result<(), VoiceError> {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
            file.write_all(&body)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            // Rename is the final fallible operation: a reported failed save
            // cannot follow a successful replacement of the registry.
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(&temporary); }
        result
    }
}

fn validate_reference(reference: &str) -> Result<(), VoiceError> {
    if reference.trim().is_empty() || reference.chars().any(char::is_control) {
        return Err(VoiceError::InvalidConfig("voice reference must be nonempty and contain no control characters".into()));
    }
    Ok(())
}

fn normalize_hashes(hashes: &mut BTreeMap<String, String>) {
    for hash in hashes.values_mut() { hash.make_ascii_lowercase(); }
}

fn validate_source_hashes(hashes: &BTreeMap<String, String>) -> Result<(), VoiceError> {
    if hashes.iter().any(|(name, hash)| name.trim().is_empty() || hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())) {
        return Err(VoiceError::InvalidConfig("source fingerprints must be named SHA-256 values".into()));
    }
    Ok(())
}

fn legacy_manifest_role(entry: &RegisteredVoice) -> Option<&'static str> {
    if entry.package_receipt.is_some() { return None; }
    if entry.assets.cfinal_model.is_some() {
        const BASE_ROLES: &[&str] = &["cfinal_json", "bank_json", "bank_tensors", "bank_topology", "bank_token",
            "cfinal_controller", "cfinal_decoder", "cfinal_rights", "cfinal_export", "cfinal_contracts"];
        const SUFFIXES: &[&str] = &["d_controller", "local_controller", "decoder", "phone_table"];
        let files = &entry.files;
        if BASE_ROLES.iter().all(|role| files.contains_key(*role)) {
            for family in ["r13b", "r14"] {
                if files.len() == BASE_ROLES.len() + SUFFIXES.len() + usize::from(family == "r14")
                    && SUFFIXES.iter().all(|suffix| files.contains_key(&format!("cfinal_{family}_{suffix}")))
                    && (family != "r14" || files.contains_key("cfinal_r14_strategy")) {
                    return Some("cfinal_json");
                }
            }
        }
        return None;
    }
    entry.assets.voice_package.as_ref()?;
    let exact_roles = |roles: &[(&str, &str)], other: &[(&str, &str)]| {
        entry.files.len() == roles.len() + other.len() + usize::from(entry.assets.vocoder.is_some())
            && roles.iter().chain(other).all(|(role, _)| entry.files.contains_key(*role))
            && entry.assets.vocoder.is_some() == entry.files.contains_key("vocoder")
    };
    if exact_roles(VOICE_FILES, VOICE_RUNTIME_FILES) { return Some("voice_source_manifest"); }
    // The former root format had the manifest and three graphs, with no bank.
    let pre_bank = PACKAGE_FILES.iter().filter(|(role, _)| *role != "bank_tensors");
    (entry.files.len() == pre_bank.clone().count() + usize::from(entry.assets.vocoder.is_some())
        && pre_bank.clone().all(|(role, _)| entry.files.contains_key(*role))
        && entry.assets.vocoder.is_some() == entry.files.contains_key("vocoder"))
        .then_some("verdant_voice_manifest")
}

fn validate_registration(entry: &RegisteredVoice) -> Result<(), VoiceError> {
    validate_source_hashes(&entry.files)?;
    validate_source_hashes(&entry.source_sha256)?;
    if entry.display_name.trim().is_empty() { return Err(VoiceError::InvalidConfig("display name must not be empty".into())); }
    if let Some(path) = &entry.assets.cfinal_model {
        if path.as_os_str().is_empty() || entry.assets.voice_package.is_some() || entry.assets.voice_model.is_some()
            || entry.assets.token.is_some() || entry.assets.model.is_some() || entry.assets.vocoder.is_some()
            || legacy_manifest_role(entry) != Some("cfinal_json") {
            return Err(VoiceError::InvalidConfig("invalid historical voice asset inventory".into()));
        }
        if matches!(&entry.source_receipt, Some(RegisteredSourceReceipt::Historical(receipt))
            if receipt.schema == "verdant-cfinal-source-admission/1" && receipt.rights_attested
                && !receipt.attestation_is_legal_determination && receipt.cfinal_manifest_sha256 == entry.files["cfinal_json"]
                && receipt.original_source_sha256 == entry.source_sha256
                && entry.source_sha256.len() == 2 && confirmed_originals(&entry.source_sha256)) {
            return Ok(());
        }
        return Err(VoiceError::InvalidConfig("invalid historical voice source receipt".into()));
    }
    validate_assets(&entry.assets)?;
    if let Some(role) = legacy_manifest_role(entry) {
        if let Some(RegisteredSourceReceipt::Current(receipt)) = &entry.source_receipt {
            return receipt.validate_for(&entry.files[role], &entry.source_sha256);
        }
        return Err(VoiceError::InvalidConfig("legacy voice lacks source admission".into()));
    }
    let mut expected = if entry.assets.voice_package.is_some() {
        PACKAGE_FILES.iter().map(|(role, _)| (*role).to_owned()).collect::<Vec<_>>()
    } else if entry.assets.voice_model.is_some() {
        MODERN_FILES.iter().map(|(role, _)| (*role).to_owned()).collect::<Vec<_>>()
    } else { vec!["token".to_owned(), "model".to_owned()] };
    if entry.assets.vocoder.is_some() { expected.push("vocoder".into()); }
    expected.sort();
    if entry.files.keys().ne(expected.iter()) { return Err(VoiceError::InvalidConfig("asset fingerprint roles do not match the selected source configuration".into())); }
    if entry.assets.voice_package.is_some() {
        match (&entry.source_receipt, &entry.package_receipt) {
            (Some(RegisteredSourceReceipt::Current(receipt)), None) => receipt.validate_for(&entry.files["verdant_voice_manifest"], &entry.source_sha256)?,
            (None, Some(receipt)) if entry.source_sha256.len() == 2 && confirmed_originals(&entry.source_sha256) => {
                receipt.validate_for(&entry.files["verdant_voice_manifest"], &entry.files["bank_tensors"], &receipt.app_version)?;
            }
            _ => return Err(VoiceError::InvalidConfig("verdant-voice requires one valid package installation or original-source receipt".into())),
        }
    } else if entry.source_receipt.is_some() || entry.package_receipt.is_some() {
        return Err(VoiceError::InvalidConfig("package receipts require a verdant-voice package".into()));
    }
    Ok(())
}

fn validate_assets(assets: &VoiceAssets) -> Result<(), VoiceError> {
    if assets.cfinal_model.is_some() {
        return Err(VoiceError::InvalidConfig("historical voice assets require explicit migration".into()));
    }
    let current_voice=assets.voice_package.is_some();
    let modern = assets.voice_model.is_some();
    let raw = assets.token.is_some() && assets.model.is_some();
    if usize::from(current_voice)+usize::from(modern)+usize::from(raw)!=1 || ((modern||current_voice)&&(assets.token.is_some()||assets.model.is_some())) {
        return Err(VoiceError::InvalidConfig("select one verdant-voice directory, modern directory, or complete token/model pair".into()));
    }
    if [&assets.voice_package, &assets.voice_model, &assets.token, &assets.model, &assets.vocoder].into_iter().flatten().any(|path| path.as_os_str().is_empty()) {
        return Err(VoiceError::InvalidConfig("asset paths must not be empty".into()));
    }
    Ok(())
}

fn absolutize_assets(assets: &mut VoiceAssets, base: &Path) {
    for path in [&mut assets.voice_package, &mut assets.cfinal_model, &mut assets.voice_model, &mut assets.token, &mut assets.model, &mut assets.vocoder].into_iter().flatten() {
        if path.is_relative() { *path = base.join(&*path); }
    }
}

fn canonical_assets(mut assets: VoiceAssets) -> Result<VoiceAssets, VoiceError> {
    validate_assets(&assets)?;
    for path in [&mut assets.voice_package, &mut assets.voice_model, &mut assets.token, &mut assets.model].into_iter().flatten() { *path = fs::canonicalize(&*path)?; }
    if assets.voice_package.as_ref().is_some_and(|path| !path.is_dir()) { return Err(VoiceError::InvalidConfig("voice model path must be a directory".into())); }
    if assets.voice_model.as_ref().is_some_and(|path| !path.is_dir()) { return Err(VoiceError::InvalidConfig("voice_model must be a directory".into())); }
    Ok(assets)
}

fn hash_file(path: &Path) -> Result<String, VoiceError> {
    Ok(hash_file_snapshot(path, None)?.0)
}

fn hash_file_snapshot(path: &Path, limit: Option<u64>) -> Result<(String, u64), VoiceError> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() { return Err(VoiceError::InvalidConfig("voice assets must be regular files".into())); }
    let mut file = file.take(limit.map_or(u64::MAX, |size| size + 1));
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut buffer = [0_u8; 65_536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hash.update(&buffer[..count]);
        size += count as u64;
    }
    Ok((hex::encode(hash.finalize()), size))
}

fn asset_snapshot(assets: &VoiceAssets) -> Result<(BTreeMap<String, String>, Option<VoicePackage>), VoiceError> {
    validate_assets(assets)?;
    let mut hashes = BTreeMap::new();
    let mut package = None;
    if let Some(directory) = &assets.voice_package {
        let (package_hashes, declaration) = package_snapshot(directory)?;
        hashes = package_hashes;
        package = Some(declaration);
    } else if let Some(directory) = &assets.voice_model {
        for (role, name) in MODERN_FILES { hashes.insert((*role).into(), hash_file(&directory.join(name))?); }
    } else {
        hashes.insert("token".into(), hash_file(assets.token.as_deref().expect("validated token path"))?);
        hashes.insert("model".into(), hash_file(assets.model.as_deref().expect("validated model path"))?);
    }
    Ok((hashes, package))
}

fn selection_from(reference: &str, entry: &RegisteredVoice, voice_package: Option<VoicePackage>) -> Result<VoiceSelection, VoiceError> {
    validate_reserved_reference(reference, &entry.assets, &entry.files)?;
    let mut sources = entry.source_sha256.clone();
    let mut languages: BTreeMap<String, String> = BTreeMap::new();
    if let Some(package) = &voice_package {
        if reference.starts_with("verdant-voice-") && reference != format!("verdant-voice-{}", package.version) {
            return Err(VoiceError::InvalidConfig("reserved voice reference and package version disagree".into()));
        }
        if let Some(receipt) = &entry.package_receipt {
            receipt.validate_for(&entry.files["verdant_voice_manifest"], &entry.files["bank_tensors"], &package.version)?;
        }
    }
    let confirmed = if entry.assets.voice_package.is_some() {
        // The worker admitted the immutable source format and exact originals.
        // Selection checks its frozen file roles and receipt rather than
        // implementing a second parser for historical source-package payloads.
        if !confirmed_originals(&entry.source_sha256) {
            return Err(VoiceError::InvalidConfig("voice admission lacks the exact original-source fingerprints".into()));
        }
        true
    } else if let Some(directory) = &entry.assets.voice_model {
        let voice: Value = serde_json::from_reader(File::open(directory.join("voice.json"))?)?;
        if let Some(declared) = voice.get("languages") {
            languages = serde_json::from_value(declared.clone())?;
            if languages.iter().any(|(language, support)| language.trim().is_empty()
                || language.chars().any(char::is_control) || !matches!(support.as_str(), "native" | "approximate")) {
                return Err(VoiceError::InvalidConfig("voice languages need nonempty language tags and native/approximate support".into()));
            }
        }
        let topology: Value = serde_json::from_reader(File::open(directory.join("topology.json"))?)?;
        let mut original: BTreeMap<String, String> = serde_json::from_value(voice.get("source_sha256").cloned().unwrap_or_else(|| serde_json::json!({})))?;
        let mut topology_sources: BTreeMap<String, String> = serde_json::from_value(topology.get("source_sha256").cloned().unwrap_or_else(|| serde_json::json!({})))?;
        normalize_hashes(&mut original);
        normalize_hashes(&mut topology_sources);
        if original != topology_sources || original != entry.source_sha256 {
            return Err(VoiceError::InvalidConfig("modern model, topology and admitted original-source fingerprints disagree".into()));
        }
        confirmed_originals(&original)
    } else {
        let token = entry.files.get("token").expect("validated raw token fingerprint");
        let model = entry.files.get("model").expect("validated raw model fingerprint");
        for (key, actual) in [("sn16k.tkn", token), ("sn16k.bin", model)] {
            if sources.get(key).is_some_and(|claimed| !claimed.eq_ignore_ascii_case(actual)) {
                return Err(VoiceError::InvalidConfig("raw source identity does not match the admitted bytes".into()));
            }
        }
        token.eq_ignore_ascii_case(KNOWN_TOKEN_SHA) && model.eq_ignore_ascii_case(KNOWN_MODEL_SHA)
    };
    // The frozen recovered source has a reviewed Mandarin/English inventory
    // and an explicitly approximate Japanese mapping. Unknown raw sources do
    // not inherit these declarations from a user-entered singer name.
    if confirmed && languages.is_empty() {
        languages = BTreeMap::from([("zh".into(), "native".into()), ("en".into(), "native".into()), ("ja".into(), "approximate".into())]);
    }
    for (role, hash) in &entry.files { sources.insert(role.clone(), hash.clone()); }
    let identity_bytes = match &entry.package_receipt {
        Some(receipt) => serde_json::to_vec(&(REGISTRY_VERSION, &entry.files, &entry.source_sha256, receipt))?,
        None => serde_json::to_vec(&(REGISTRY_VERSION, &entry.files, &entry.source_sha256))?,
    };
    let source_receipt = match &entry.source_receipt {
        Some(RegisteredSourceReceipt::Current(receipt)) => Some(receipt.clone()),
        _ => None,
    };
    Ok(VoiceSelection { reference: reference.into(), display_name: entry.display_name.clone(), languages, assets: entry.assets.clone(),
        source_sha256: sources, cache_identity: hex::encode(Sha256::digest(identity_bytes)), historical_identity_confirmed: confirmed,
        source_receipt, package_receipt: entry.package_receipt.clone(), voice_package })
}

fn validate_receipt(receipt:&SourceReceipt,manifest_sha256:&str)->Result<(),VoiceError>{
    if receipt.schema!="verdant-voice-source-admission/1"||!receipt.rights_attested||receipt.attestation_is_legal_determination
        ||receipt.source_manifest_sha256!=manifest_sha256||manifest_sha256.len()!=64||!manifest_sha256.bytes().all(|b|b.is_ascii_hexdigit())
        ||receipt.original_source_sha256.len()!=2||!confirmed_originals(&receipt.original_source_sha256){
        return Err(VoiceError::InvalidConfig("invalid voice source-admission receipt".into()));
    }Ok(())
}

/// An explicit singer choice replaces the old per-track pin in one undoable
/// transaction. Catalog admission and existing document source pins still
/// apply; reselecting a singer is the deliberate way to refresh its track pin.
pub fn singer_choice_batch(project: &Project, track_id: &TrackId, reference: &str, caps: &crate::wire::Capabilities)
    -> Result<CommandBatch, String> {
    let track = project.track(track_id).ok_or("轨道不存在")?;
    // Catalog resolution reads the reference and source pin, not note data.
    // Avoid copying a whole imported song just to change its singer.
    let mut candidate = verdant_domain::Track::new("", "teal");
    candidate.voice_ref = Some(reference.into());
    let (_, voice) = caps.voice_for_track(project, &candidate)?.ok_or("后端未提供已验证的歌手目录")?;
    let mut header = track.header();
    header.voice_ref = Some(reference.into());
    let metadata = serde_json::json!({"schema":"verdant-track-voice/1", "reference":reference,
        "cache_identity":voice.cache_identity, "source_sha256":voice.source_sha256,
        "display_name":voice.display_name, "historical_identity_confirmed":voice.historical_identity_confirmed});
    let mut commands = vec![
        Command::SetTrackHeader { track: track_id.clone(), header },
        Command::SetTrackData { track: track_id.clone(), key: "voice_selection".into(), value: Some(metadata) },
    ];
    // A language belongs to its selected bank, so switching banks requires
    // a deliberate new language choice instead of inheriting an old claim.
    if track.voice_ref.as_deref() != Some(reference) {
        commands.push(Command::SetTrackData { track: track_id.clone(), key: "voice_language".into(), value: None });
    }
    Ok(CommandBatch { label: "选择轨道歌手".into(), commands })
}

fn confirmed_originals(sources: &BTreeMap<String, String>) -> bool {
    sources.get("sn16k.tkn").is_some_and(|hash| hash.eq_ignore_ascii_case(KNOWN_TOKEN_SHA))
        && sources.get("sn16k.bin").is_some_and(|hash| hash.eq_ignore_ascii_case(KNOWN_MODEL_SHA))
}

#[cfg(test)]
mod voice_tests {
    use super::*;
    use serde_json::json;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> (Self, VoiceAssets, VoiceAdmission) {
            let directory = std::env::temp_dir().join(format!("verdant-voice-registry-{}-{}",
                std::process::id(), TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&directory).unwrap();
            for (_, name) in &PACKAGE_FILES[1..] { fs::write(directory.join(name), name.as_bytes()).unwrap(); }
            for name in ["NOTICE.txt", "AUTHORIZATION.json"] { fs::write(directory.join(name), name.as_bytes()).unwrap(); }
            let source_sha256 = BTreeMap::from([
                ("sn16k.tkn".into(), KNOWN_TOKEN_SHA.into()),
                ("sn16k.bin".into(), KNOWN_MODEL_SHA.into()),
            ]);
            let fixture = Self(directory.clone());
            fixture.set_version(VOICE_PACKAGE_VERSION);
            let source_receipt = Some(SourceReceipt { schema:"verdant-voice-source-admission/1".into(),
                original_source_sha256:source_sha256.clone(), source_manifest_sha256:hash_file(&directory.join("manifest.json")).unwrap(),
                rights_attested:true, attestation_is_legal_determination:false });
            let assets = VoiceAssets { voice_package:Some(directory), ..Default::default() };
            (fixture, assets, VoiceAdmission { source_sha256, source_receipt, package_receipt: None })
        }

        fn set_version(&self, version: &str) {
            let artifact = |file: &str| json!({"file":file,"bytes":file.len(),"sha256":hash_file(&self.0.join(file)).unwrap()});
            let artifacts = PACKAGE_FILES[1..].iter().map(|(role, file)| ((*role).to_string(), artifact(file)))
                .collect::<serde_json::Map<_, _>>();
            let manifest = json!({"schema":verdant_voice_format::SCHEMA,"name":VOICE_PACKAGE_NAME,"version":version,
                "strategy":VOICE_STRATEGY,"artifacts":artifacts,
                "records":{"notice":artifact("NOTICE.txt"),"authorization":artifact("AUTHORIZATION.json")}});
            fs::write(self.0.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn canonical_admission_pins_all_roles_and_preserves_source_receipt_bytes() {
        let (fixture, assets, admission) = Fixture::new();
        let original = fs::read(fixture.0.join("manifest.json")).unwrap();
        assert_eq!(package_files(&fixture.0).unwrap(), PACKAGE_FILES);
        assert_eq!(PACKAGE_FILES.len(), 5);
        assert!(PACKAGE_FILES.contains(&("bank_tensors", "bank.safetensors")));
        let mut registry = VoiceRegistry::new();
        let selected = registry.install_with(fixture.0.join("registry.json"), DESKTOP_VOICE_REFERENCE,
            format!("verdant-voice {VOICE_PACKAGE_VERSION}"), assets, |_| Ok(admission.clone())).unwrap();
        assert_eq!(fs::read(fixture.0.join("manifest.json")).unwrap(), original);
        assert_eq!(selected.source_receipt, admission.source_receipt);
        assert_eq!(selected.voice_package.as_ref().unwrap().version, VOICE_PACKAGE_VERSION);
        assert_eq!(registry.default_reference(), Some(DESKTOP_VOICE_REFERENCE));
        for (role, name) in PACKAGE_FILES {
            let mut incomplete = registry.clone();
            incomplete.voices.get_mut(DESKTOP_VOICE_REFERENCE).unwrap().files.remove(*role);
            assert!(incomplete.selection(DESKTOP_VOICE_REFERENCE).is_err(), "{role}");
            let path = fixture.0.join(name);
            let bytes = fs::read(&path).unwrap(); fs::write(&path, b"changed admitted input").unwrap();
            assert!(registry.selection(DESKTOP_VOICE_REFERENCE).is_err(), "{role}");
            fs::write(path, bytes).unwrap();
        }
        let mut extra = registry.clone();
        extra.voices.get_mut(DESKTOP_VOICE_REFERENCE).unwrap().files.insert("bank_json".into(), "a".repeat(64));
        assert!(extra.selection(DESKTOP_VOICE_REFERENCE).is_err());
        let graph = fixture.0.join("decoder-48k.onnx");
        fs::remove_file(&graph).unwrap();
        assert!(package_files(&fixture.0).is_err());
        fs::write(graph, b"decoder-48k.onnx").unwrap();
        fs::write(fixture.0.join("NOTICE.txt"), b"changed record").unwrap();
        assert!(registry.selection(DESKTOP_VOICE_REFERENCE).is_err());
        fs::write(fixture.0.join("NOTICE.txt"), b"NOTICE.txt").unwrap();
        let manifest = fixture.0.join("manifest.json");
        let mut altered = original;
        altered.push(b'\n');
        fs::write(&manifest, altered).unwrap();
        assert_eq!(package_files(&fixture.0).unwrap(), PACKAGE_FILES);
        assert!(matches!(registry.selection(DESKTOP_VOICE_REFERENCE), Err(VoiceError::SourceChanged(role))
            if role == "verdant_voice_manifest"));
        fs::remove_file(manifest).unwrap();
        assert!(registry.selection(DESKTOP_VOICE_REFERENCE).is_err());
        assert!(package_files(&fixture.0).is_err());
    }

    #[test]
    fn package_receipt_enables_selection_without_a_source_use_attestation() {
        let (fixture, assets, source_admission) = Fixture::new();
        let hashes = package_hashes(&fixture.0).unwrap();
        let mut receipt = PackageReceipt::new(hashes["verdant_voice_manifest"].clone(), hashes["bank_tensors"].clone(), VOICE_PACKAGE_VERSION.into());
        receipt.artifact_policy_sha256 = Some("a".repeat(64));
        let admission = VoiceAdmission { source_sha256: source_admission.source_sha256.clone(), source_receipt: None,
            package_receipt: Some(receipt.clone()) };
        let mut registry = VoiceRegistry::new();
        for field in ["manifest_sha256", "bank_sha256", "app_version", "policy_sha256", "source_hash", "both_receipts"] {
            let mut invalid = admission.clone();
            let proof = invalid.package_receipt.as_mut().unwrap();
            match field {
                "manifest_sha256" => proof.manifest_sha256 = "0".repeat(64),
                "bank_sha256" => proof.bank_sha256 = "0".repeat(64),
                "app_version" => proof.app_version = "0.0.0".into(),
                "policy_sha256" => proof.artifact_policy_sha256 = Some("bad digest".into()),
                "source_hash" => { invalid.source_sha256.remove("sn16k.bin"); },
                _ => invalid.source_receipt = source_admission.source_receipt.clone(),
            }
            assert!(registry.admit_with(DESKTOP_VOICE_REFERENCE, "Voice", assets.clone(), |_| Ok(invalid)).is_err(), "{field}");
        }
        let selected = registry.admit_with(DESKTOP_VOICE_REFERENCE, "Voice", assets, |_| Ok(admission)).unwrap();
        assert_eq!(registry.default_reference(), None);
        assert_eq!(selected.package_receipt, Some(receipt.clone()));
        assert!(selected.source_receipt.is_none());
        let caps = registry.capabilities();
        let status = &caps["voices"][DESKTOP_VOICE_REFERENCE]["files"]["voice_source_admission"];
        assert_eq!(status, &json!({"available":true,"sha256":receipt.manifest_sha256}));
        registry.set_default(Some(DESKTOP_VOICE_REFERENCE)).unwrap();
        let path = fixture.0.join("registry.json");
        registry.save_atomic(&path).unwrap();
        let mut loaded = VoiceRegistry::load(&path).unwrap();
        assert_eq!(loaded.default_selection().unwrap(), selected);
        let json: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let entry = &json["voices"][DESKTOP_VOICE_REFERENCE];
        assert!(entry.get("source_receipt").is_none());
        assert!(entry["package_receipt"].get("rights_attested").is_none());
        loaded.remove(DESKTOP_VOICE_REFERENCE).unwrap();
        assert!(loaded.references().is_empty());
        assert_eq!(loaded.default_reference(), None);
    }

    #[test]
    fn historical_registry_records_remain_listable_and_preserve_their_pins() {
        let (fixture, assets, admission) = Fixture::new();
        let mut registry = VoiceRegistry::new();
        registry.admit_with(DESKTOP_VOICE_REFERENCE, "Current voice", assets, |_| Ok(admission.clone())).unwrap();
        let mut json = serde_json::to_value(registry).unwrap();
        let source = json!(admission.source_sha256);
        let digest = "a".repeat(64);
        let base = ["cfinal_json", "bank_json", "bank_tensors", "bank_topology", "bank_token", "cfinal_controller",
            "cfinal_decoder", "cfinal_rights", "cfinal_export", "cfinal_contracts"];
        for family in ["r13b", "r14"] {
            let mut roles: BTreeMap<_, _> = base.iter().map(|role| (role.to_string(), digest.clone())).collect();
            for suffix in ["d_controller", "local_controller", "decoder", "phone_table"] {
                roles.insert(format!("cfinal_{family}_{suffix}"), digest.clone());
            }
            if family == "r14" { roles.insert("cfinal_r14_strategy".into(), digest.clone()); }
            json["voices"][family] = json!({"display_name":family,"assets":{"cfinal_model":fixture.0.join(family)},"files":roles,
                "source_sha256":source,"source_receipt":{"schema":"verdant-cfinal-source-admission/1",
                    "cfinal_manifest_sha256":digest,"original_source_sha256":source,"rights_attested":true,
                    "attestation_is_legal_determination":false}});
        }
        let nested: BTreeMap<_, _> = VOICE_FILES.iter().chain(VOICE_RUNTIME_FILES).map(|(role, _)| (*role, &digest)).collect();
        let root_v1: BTreeMap<_, _> = PACKAGE_FILES.iter().filter(|(role, _)| *role != "bank_tensors")
            .map(|(role, _)| (*role, &digest)).collect();
        for (reference, roles) in [("nested", nested), ("root-v1", root_v1)] {
            json["voices"][reference] = json!({"display_name":reference,"assets":{"voice_package":fixture.0.join(reference)},
                "files":roles,"source_sha256":source,"source_receipt":{"schema":"verdant-voice-source-admission/1",
                    "source_manifest_sha256":digest,"original_source_sha256":source,"rights_attested":true,
                    "attestation_is_legal_determination":false}});
        }
        let path = fixture.0.join("registry.json");
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let mut loaded = VoiceRegistry::load(&path).unwrap();
        assert!(loaded.selection(DESKTOP_VOICE_REFERENCE).is_ok());
        let caps = loaded.capabilities();
        for reference in ["r13b", "r14", "nested", "root-v1"] {
            assert!(loaded.references().iter().any(|id| id == reference));
            assert!(matches!(loaded.selection(reference), Err(VoiceError::MigrationRequired(id)) if id == reference));
            assert_eq!(caps["voices"][reference]["error"], "migration_required");
        }
        loaded.set_default(Some(DESKTOP_VOICE_REFERENCE)).unwrap();
        loaded.save_atomic(&path).unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["voices"], json["voices"]);
        let mut malformed = saved;
        malformed["voices"]["r14"]["files"].as_object_mut().unwrap().remove("cfinal_r14_strategy");
        fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
        assert!(VoiceRegistry::load(path).is_err());
    }

    #[test]
    fn canonical_import_rejects_wrong_versions_layouts_and_reserved_relabels_before_validation() {
        for version in ["0.0.1", "1", "01.0.0", "0.1.0-01", "0.1.0+"] {
            let (fixture, assets, _) = Fixture::new(); fixture.set_version(version);
            assert!(VoiceRegistry::new().admit_with(DESKTOP_VOICE_REFERENCE, "Voice", assets,
                |_| panic!("wrong version reached validator")).is_err(), "{version}");
        }
        let (fixture, assets, _) = Fixture::new();
        assert!(VoiceRegistry::new().admit_with("verdant-voice-99.0.0", "Voice", assets.clone(),
            |_| panic!("wrong reserved version reached validator")).is_err());
        let nested = fixture.0.join(VOICE_PACKAGE_NAME);
        fs::create_dir(&nested).unwrap();
        fs::rename(fixture.0.join("manifest.json"), nested.join("manifest.json")).unwrap();
        assert!(package_files(&fixture.0).is_err());
        assert!(VoiceRegistry::new().admit_with(DESKTOP_VOICE_REFERENCE, "Voice", assets,
            |_| panic!("missing runtime marker reached validator")).is_err());
    }

    #[test]
    fn persisted_compatible_version_and_exact_track_pin_survive_newer_software() {
        let (fixture, assets, admission) = Fixture::new();
        let mut registry = VoiceRegistry::new();
        registry.admit_with(DESKTOP_VOICE_REFERENCE, "Voice", assets, |_| Ok(admission)).unwrap();
        // Represent a registry written by another compatible executable.
        let path = fixture.0.join("manifest.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["version"] = "0.1.1".into(); fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let mut entry = registry.voices.remove(DESKTOP_VOICE_REFERENCE).unwrap();
        let changed_digest = hash_file(&path).unwrap();
        entry.files.insert("verdant_voice_manifest".into(), changed_digest.clone());
        let Some(RegisteredSourceReceipt::Current(receipt)) = &mut entry.source_receipt else { panic!("missing current receipt"); };
        receipt.source_manifest_sha256 = changed_digest;
        let reference = "verdant-voice-0.1.1";
        registry.voices.insert(reference.into(), entry);
        registry.set_default(Some(reference)).unwrap();
        let selected = registry.selection(reference).unwrap();
        let mut project = Project::new_empty(); project.tracks[0].voice_ref = Some(reference.into());
        project.tracks[0].unknown.insert("voice_selection".into(), json!({"schema":"verdant-track-voice/1",
            "reference":reference,"cache_identity":selected.cache_identity,"source_sha256":selected.source_sha256}));
        let registry_path = fixture.0.join("registry.json"); registry.save_atomic(&registry_path).unwrap();
        let loaded = VoiceRegistry::load(&registry_path).unwrap();
        let pinned = loaded.resolve_track(&project, &project.tracks[0].id).unwrap().unwrap();
        assert_eq!(pinned.voice_package.unwrap().version, "0.1.1");
        assert_eq!(pinned.cache_identity, selected.cache_identity);
        assert_eq!(loaded.capabilities()["voices"][reference]["voice_package"]["version"], "0.1.1");
        project.tracks[0].unknown.get_mut("voice_selection").unwrap()["cache_identity"] = "another package".into();
        assert!(loaded.resolve_track(&project, &project.tracks[0].id).is_err());
    }
}
