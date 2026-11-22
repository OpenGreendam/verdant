//! Offline Agent Skills bundles with verified content and conservative managed installs.
//! Installation never overwrites an untracked skill directory, an edited managed
//! file, or a symbolic link. All selected skills are checked before any replacement.
mod bundle;
mod files;
mod install;

use serde::{Deserialize, Serialize};
use std::fmt;

pub use bundle::{Bundle, parse_manifest, sha256, validate_name};
pub use install::{Action, uninstall};
pub const BUNDLE_SCHEMA: &str = "verdant-agent-skills/1";
pub const RECEIPT_SCHEMA: &str = "verdant-agent-skill-install/1";
pub const RECEIPT_FILE: &str = ".verdant-skill.json";
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub version: String,
    pub skills: Vec<Skill>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub files: Vec<SkillFile>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    pub path: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub executable: bool,
}
fn is_false(value: &bool) -> bool { !value }

#[derive(Debug)]
pub struct Error { pub code: &'static str, pub message: String }
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error { Error { code: "INVALID_ARGUMENT", message: message.into() } }
fn integrity(message: impl Into<String>) -> Error { Error { code: "INTEGRITY_ERROR", message: message.into() } }
fn conflict(message: impl Into<String>) -> Error { Error { code: "CONFLICT", message: message.into() } }
fn io_error(path: &std::path::Path, error: impl fmt::Display) -> Error {
    Error { code: "IO_ERROR", message: format!("{}: {error}", path.display()) }
}
