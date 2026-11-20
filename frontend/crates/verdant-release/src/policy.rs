//! A hash-bound external policy is an explicit artifact admission decision.
//! Neither an embedded manifest nor a local source-use checkbox grants it.
use std::{collections::{BTreeMap, BTreeSet}, path::{Path, PathBuf}};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use crate::{Args, files};

pub(crate) use verdant_voice_format::PACKAGE_FILES as REQUIRED_ROLES;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PolicyFile { pub role: String, pub path: String, pub bytes: u64, pub sha256: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evidence { pub path: String, pub bytes: u64, pub sha256: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GrantedArtifact { pub role: String, pub sha256: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Grant {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rights_holder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_authority: Option<String>,
    pub permission: String, pub artifact_specific: bool,
    pub evidence: Evidence, pub artifacts: Vec<GrantedArtifact>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    pub schema: String, pub name: String, pub version: String, pub compatible_app_version: String,
    pub authorization_status: String, pub files: Vec<PolicyFile>, pub grants: Vec<Grant>,
}

pub(crate) struct AdmittedPolicy { pub policy: Policy, pub path: PathBuf, pub sha256: String }

pub(crate) fn external(args: &mut Args) -> Result<Option<AdmittedPolicy>> {
    let path = args.take("--artifact-policy").map(PathBuf::from);
    let sha256 = args.take("--artifact-policy-sha256");
    ensure!(path.is_some() == sha256.is_some(), "--artifact-policy and --artifact-policy-sha256 are required together");
    let Some(path) = path else { return Ok(None); };
    let sha256 = sha256.context("missing artifact-policy digest")?;
    let policy: Policy = files::hash_bound(&path, &sha256)?;
    policy.validate()?;
    Ok(Some(AdmittedPolicy { policy, path, sha256 }))
}

impl Policy {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.schema == "verdant-release-artifact-policy/1" && self.name == "verdant-voice"
            && self.authorization_status == "granted" && self.version == self.compatible_app_version
            && semver::Version::parse(&self.version).is_ok(), "voice bundle lacks compatible, artifact-specific redistribution admission");
        let mut roles = BTreeMap::new();
        let mut paths = BTreeSet::new();
        let mut licenses = 0;
        let mut notices = 0;
        for file in &self.files {
            files::portable_path(&file.path)?;
            ensure!(files::valid_sha(&file.sha256) && file.bytes > 0 && file.bytes <= 256 * 1024 * 1024
                && paths.insert(file.path.as_str()) && roles.insert(file.role.as_str(), file).is_none(), "invalid/repeated voice artifact role or path");
            let canonical = REQUIRED_ROLES.iter().find(|(role, _)| *role == file.role);
            if let Some((_, expected)) = canonical { ensure!(file.path == *expected, "voice role {} must use {expected}", file.role); }
            else {
                let kind = file.role.split_once(':').context("extra bundle roles must be license:ID, notice:ID or authorization:ID")?;
                ensure!(!kind.1.is_empty() && matches!(kind.0, "license" | "notice" | "authorization")
                    && matches!(Path::new(&file.path).extension().and_then(|s| s.to_str()), Some("txt" | "md" | "json")), "unapproved extra voice artifact");
                licenses += usize::from(kind.0 == "license"); notices += usize::from(kind.0 == "notice");
            }
        }
        ensure!(licenses > 0 && notices > 0, "bundled voice requires its licenses and attribution notices");
        for (role, _) in REQUIRED_ROLES { ensure!(roles.contains_key(role), "bundle omits required voice role {role}"); }
        let mut ids = BTreeSet::new();
        let mut granted = BTreeSet::new();
        for grant in &self.grants {
            let authority = match (&grant.rights_holder, &grant.release_authority) {
                (Some(authority), None) | (None, Some(authority)) => !authority.trim().is_empty(),
                _ => false,
            };
            ensure!(authority && !grant.id.trim().is_empty() && ids.insert(grant.id.as_str())
                && grant.permission == "redistribute-bundled-runtime" && grant.artifact_specific && !grant.artifacts.is_empty(),
                "voice bundle requires an identified issuer and artifact-specific redistribution authorization");
            let evidence = self.files.iter().find(|file| file.path == grant.evidence.path).context("grant evidence must be included in the bundle")?;
            ensure!(evidence.role.starts_with("authorization:") && evidence.bytes == grant.evidence.bytes
                && evidence.sha256 == grant.evidence.sha256, "grant evidence lacks its exact authorization role/digest");
            let mut grant_roles = BTreeSet::new();
            for artifact in &grant.artifacts {
                let file = roles.get(artifact.role.as_str()).context("grant covers an undeclared artifact role")?;
                ensure!(file.sha256 == artifact.sha256 && grant_roles.insert(artifact.role.as_str()), "grant does not cover the exact artifact digest");
                granted.insert(artifact.role.as_str());
            }
        }
        for (role, _) in REQUIRED_ROLES { ensure!(granted.contains(role), "missing artifact-specific redistribution grant for {role}"); }
        Ok(())
    }

    pub(crate) fn bundle(&self, root: &Path) -> Result<()> {
        self.validate()?;
        let actual = files::walk(root)?;
        ensure!(actual.len() == self.files.len(), "voice bundle contains missing or undeclared files");
        let (manifest, bytes) = verdant_voice_format::read_manifest(root)?;
        for file in &self.files {
            let (size, sha256) = if file.role == "verdant_voice_manifest" {
                (bytes.len() as u64, verdant_voice_format::sha256(&bytes))
            } else {
                let path = files::input(root, &file.path)?;
                files::hash_reader(&mut files::regular(&path)?)?
            };
            ensure!(size == file.bytes && sha256 == file.sha256, "bundle artifact changed: {}", file.role);
        }
        self.bind_manifest(&manifest)
    }

    pub(crate) fn runtime_manifest(&self, bytes: &[u8]) -> Result<()> {
        ensure!(bytes.len() <= verdant_voice_format::MAX_MANIFEST_BYTES, "voice root manifest exceeds its size limit");
        let manifest: verdant_voice_format::Manifest = serde_json::from_value(verdant_voice_format::parse_json(bytes)?)?;
        manifest.validate()?;
        self.bind_manifest(&manifest)
    }

    fn bind_manifest(&self, manifest: &verdant_voice_format::Manifest) -> Result<()> {
        ensure!(manifest.name == self.name && manifest.version == self.version,
            "voice root manifest differs from the authorized package/version");
        for (role, artifact) in &manifest.artifacts {
            let file = self.files.iter().find(|file| file.role == *role).context("voice graph role is not authorized")?;
            ensure!(file.path == artifact.file && file.bytes == artifact.bytes && file.sha256 == artifact.sha256,
                "voice graph declaration differs from the authorized role {role}");
        }
        for (kind, record) in &manifest.records {
            let policy_kind = match kind.as_str() { "conversion_notice" => "notice", "source_use_evidence" => "authorization", kind => kind };
            let file = self.files.iter().find(|file| file.path == record.file &&
                file.role.split_once(':').is_some_and(|(role_kind, _)| role_kind == policy_kind))
                .context("voice record is missing its matching policy role")?;
            ensure!(file.bytes == record.bytes && file.sha256 == record.sha256,
                "voice record declaration differs from its policy digest: {kind}");
        }
        Ok(())
    }
}

pub(crate) fn forbidden_name(relative: &str, file: bool) -> bool {
    let normalized = relative.replace('\\', "/").to_lowercase();
    let name = normalized.rsplit('/').next().unwrap_or("");
    let suffix = Path::new(name).extension().and_then(|s| s.to_str()).unwrap_or("");
    (file && matches!(suffix, "safetensors" | "npz" | "npy" | "pt" | "pth" | "ckpt" | "onnx"
        | "wav" | "flac" | "mp3" | "ogg" | "bin" | "tkn" | "whl" | "oudep" | "7z" | "rar"
        | "cab" | "msi" | "exe" | "dll" | "sgm24" | "so" | "dylib" | "a" | "lib" | "o" | "rlib"))
        || normalized.split('/').any(|part| matches!(part, "voice-ts01" | "models" | "wheelhouse" | "venvs" | "reruns" | "private-evidence" | "private-modules"))
        || ["sn16k", "pc_nsf_hifigan", "comparison-preview"].iter().any(|pattern| name.contains(pattern))
        || matches!(name, "opengreendam-reproduction-full-20261113.zip" | "opengreendam-reproduction-source-docs-20261113.zip")
}
