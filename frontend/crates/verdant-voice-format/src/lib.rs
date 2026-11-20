//! The versioned, self-contained Verdant Voice runtime transport.
//!
//! Runtime token data and leaf records share one safetensors bank. Source-use receipts and
//! redistribution grants remain independent trust anchors: a package's own
//! digest declarations establish consistency, never a right to distribute it.
use std::{collections::BTreeMap, fs, io::Read, path::Path};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod tensors;
pub mod policy;
mod strict_json;
pub use strict_json::parse as parse_json;

pub const SCHEMA: &str = "verdant-voice-package/2";
pub const BANK_SCHEMA: &str = "verdant-voice-bank/1";
pub const NAME: &str = "verdant-voice";
pub const STRATEGY: &str = "native-mlpg-reference-en-aa-boundary-sustain-v2";
pub const MAX_ARTIFACT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 16_384;
pub const PACKAGE_FILES: &[(&str, &str)] = &[
    ("verdant_voice_manifest", "manifest.json"),
    ("bank_tensors", "bank.safetensors"),
    ("verdant_voice_context_controller", "context-controller.onnx"),
    ("verdant_voice_local_controller", "local-controller.onnx"),
    ("verdant_voice_decoder", "decoder-48k.onnx"),
];
pub const RECORD_FILES: &[(&str, &str)] = &[
    ("notice", "NOTICE.txt"), ("authorization", "AUTHORIZATION.json"), ("license", "LICENSE.txt"),
    ("conversion_notice", "CONVERSION.txt"),
    ("source_use_evidence", "SOURCE-USE-EVIDENCE.json"),
];

/// These are the actual, unchanged learned graphs.
pub const CONTEXT_CONTROLLER: (&str, usize, &str) = ("context-controller.onnx", 1_034_923,
    "fdc875f178a323a00949d12f7be4b045e31fe8b2aeef66a13589c6e53f8598b7");
pub const LOCAL_CONTROLLER: (&str, usize, &str) = ("local-controller.onnx", 1_029_887,
    "08342dcb3d0dd53fb5e07fa3ad00d6a395bc00cbf1101d893fa9b125490b7ae6");
pub const DECODER: (&str, usize, &str) = ("decoder-48k.onnx", 6_000_910,
    "8b6b7c584d1d2b0cd8550e38e3d303c24614e9a60f3689aea9320ad671f38412");
pub const PHONE_TABLE: (&str, usize, &str) = ("native-phone-table.json", 11_099,
    "77b7075bd954c3afd7d9e629f1fcb5816f5fd84580cffaf3175974d8cd3f5e60");

pub fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact { pub file: String, pub bytes: u64, pub sha256: String }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub name: String,
    pub version: String,
    pub strategy: String,
    pub artifacts: BTreeMap<String, Artifact>,
    pub records: BTreeMap<String, Artifact>,
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == SCHEMA && self.name == NAME && self.strategy == STRATEGY
            && semver::Version::parse(&self.version).is_ok(), "unsupported Verdant Voice package declaration");
        ensure!(self.artifacts.len() == PACKAGE_FILES.len() - 1, "unexpected Verdant Voice artifact inventory");
        for &(role, filename) in &PACKAGE_FILES[1..] {
            let artifact = self.artifacts.get(role).with_context(|| format!("missing Verdant Voice role {role}"))?;
            ensure!(artifact.file == filename && (1..=MAX_ARTIFACT_BYTES as u64).contains(&artifact.bytes)
                && valid_sha256(&artifact.sha256), "invalid Verdant Voice artifact declaration: {role}");
        }
        ensure!(self.records.contains_key("notice") && self.records.contains_key("authorization")
            && (2..=RECORD_FILES.len()).contains(&self.records.len()), "Verdant Voice requires its attribution and authorization records");
        for (role, artifact) in &self.records {
            let (_, filename) = RECORD_FILES.iter().find(|(name, _)| *name == role)
                .with_context(|| format!("unknown Verdant Voice record {role}"))?;
            ensure!(artifact.file == *filename && (1..=8 * 1024 * 1024).contains(&artifact.bytes)
                && valid_sha256(&artifact.sha256), "invalid Verdant Voice record declaration: {role}");
        }
        Ok(())
    }
}

pub fn read_file(directory: &Path, filename: &str, maximum: usize) -> Result<Vec<u8>> {
    ensure!(PACKAGE_FILES.iter().chain(RECORD_FILES).any(|(_, name)| *name == filename), "unknown Verdant Voice runtime filename");
    let metadata = fs::symlink_metadata(directory).context("Verdant Voice directory is unavailable")?;
    ensure!(metadata.file_type().is_dir() && !metadata.file_type().is_symlink(), "Verdant Voice root must be a regular directory");
    let root = directory.canonicalize()?;
    let path = root.join(filename);
    let metadata = fs::symlink_metadata(&path).with_context(|| format!("missing Verdant Voice artifact {filename}"))?;
    ensure!(metadata.file_type().is_file() && !metadata.file_type().is_symlink() && metadata.len() <= maximum as u64,
        "Verdant Voice {filename} must be a bounded regular file");
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(&path)?.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= maximum, "Verdant Voice artifact exceeds its size bound");
    Ok(bytes)
}

pub fn read_manifest(directory: &Path) -> Result<(Manifest, Vec<u8>)> {
    let bytes = read_file(directory, "manifest.json", MAX_MANIFEST_BYTES)?;
    let manifest: Manifest = serde_json::from_value(parse_json(&bytes)?)?;
    manifest.validate()?;
    Ok((manifest, bytes))
}

pub fn read_artifact(directory: &Path, artifact: &Artifact) -> Result<Vec<u8>> {
    let maximum = usize::try_from(artifact.bytes)?;
    ensure!((1..=MAX_ARTIFACT_BYTES).contains(&maximum), "invalid artifact size bound");
    let bytes = read_file(directory, &artifact.file, maximum)?;
    ensure!(bytes.len() as u64 == artifact.bytes && sha256(&bytes) == artifact.sha256,
        "Verdant Voice artifact size or SHA-256 differs: {}", artifact.file);
    Ok(bytes)
}

pub fn verify_base(bytes: &[u8], expected: (&str, usize, &str)) -> Result<()> {
    ensure!(bytes.len() == expected.1 && sha256(bytes) == expected.2,
        "Verdant Voice {} differs from its admitted base", expected.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration() -> Manifest {
        Manifest { schema: SCHEMA.into(), name: NAME.into(), version: "0.1.0".into(), strategy: STRATEGY.into(),
            artifacts: PACKAGE_FILES[1..].iter().map(|(role, file)| ((*role).into(),
                Artifact { file: (*file).into(), bytes: 1, sha256: "1".repeat(64) })).collect(),
            records: RECORD_FILES[..2].iter().map(|(role, file)| ((*role).into(),
                Artifact { file: (*file).into(), bytes: 1, sha256: "1".repeat(64) })).collect() }
    }

    #[test]
    fn current_package_requires_merged_bank_and_three_graph_artifacts() {
        let mut manifest = declaration();
        manifest.validate().unwrap();
        assert_eq!(PACKAGE_FILES.len(), 5);
        manifest.artifacts.remove("verdant_voice_local_controller");
        assert!(manifest.validate().is_err());
        let mut manifest = declaration();
        manifest.artifacts.get_mut("verdant_voice_decoder").unwrap().file = "../decoder.onnx".into();
        assert!(manifest.validate().is_err());
        let mut manifest = declaration();
        manifest.artifacts.insert("source_controller".into(), Artifact { file: "controller.onnx".into(), bytes: 1, sha256: "1".repeat(64) });
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn duplicate_json_cannot_redefine_an_artifact() {
        assert!(parse_json(br#"{"artifacts":{"decoder":{"bytes":1,"bytes":2}}}"#).is_err());
        assert!(parse_json(br#"{"a":1} {}"#).is_err());
    }
}
