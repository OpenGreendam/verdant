//! Native file interchange. No proprietary synthesis engine is invoked.
//! Normal score fields are materialized in `Project`; reports describe every
//! retained source-only field and every numerical conversion.
mod common;
mod midi;
mod svp;
mod ust;
mod ufdata;
mod vocaloid;
mod xml;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use verdant_domain::Project;

pub type Result<T> = std::result::Result<T, ExchangeError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format { Midi, Ust, Ustx, Svp, Vsqx, Vpr, Ufdata }
impl Format {
    pub fn extension(self) -> &'static str {
        match self { Self::Midi => "mid", Self::Ust => "ust", Self::Ustx => "ustx", Self::Svp => "svp", Self::Vsqx => "vsqx", Self::Vpr => "vpr", Self::Ufdata => "ufdata" }
    }
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "mid" | "midi" => Some(Self::Midi), "ust" => Some(Self::Ust), "ustx" => Some(Self::Ustx),
            "svp" => Some(Self::Svp), "vsqx" => Some(Self::Vsqx), "vpr" => Some(Self::Vpr), "ufdata" => Some(Self::Ufdata), _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_file_bytes: usize,
    pub max_archive_bytes: u64,
    pub max_archive_members: usize,
    pub max_tracks: usize,
    pub max_notes: usize,
    pub max_events: usize,
}
impl Default for Limits {
    fn default() -> Self { Self { max_file_bytes: 64 * 1024 * 1024, max_archive_bytes: 128 * 1024 * 1024,
        max_archive_members: 256, max_tracks: 4096, max_notes: 1_000_000, max_events: 4_000_000 } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImportOptions {
    pub source_name: Option<String>,
    pub preserve_extensions: bool,
    pub limits: Limits,
}
impl Default for ImportOptions {
    fn default() -> Self { Self { source_name: None, preserve_extensions: true, limits: Limits::default() } }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UstEncoding { #[default] Utf8, Cp932 }
/// Actual destination voice metadata supplied by the user or an imported file.
/// A compID identifies a voice; it does not grant an engine/voice license.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VocaloidVoice {
    pub comp_id: String,
    pub name: String,
    pub language_id: u8,
    pub program: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExportOptions {
    /// UST has one vocal lane; selection is required for multitrack projects.
    /// For other formats a selection exports that track and the global maps.
    pub track: Option<usize>,
    pub preserve_extensions: bool,
    pub midi_ppq: Option<u16>,
    pub ust_encoding: UstEncoding,
    pub vocaloid_voice: Option<VocaloidVoice>,
    pub limits: Limits,
}
impl Default for ExportOptions {
    fn default() -> Self { Self { track: None, preserve_extensions: true, midi_ppq: None,
        ust_encoding: UstEncoding::Utf8, vocaloid_voice: None, limits: Limits::default() } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction { Import, Export }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Kept for compatible file roundtrip, without execution in Verdant.
    RetainedUnsupported,
    /// The destination format has no admitted representation for this field.
    Unsupported,
    /// A finite numeric conversion required rounding; original value is here.
    Quantized,
    Metadata,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldIssue {
    pub path: String,
    pub disposition: Disposition,
    pub detail: String,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExchangeReport {
    pub schema: String,
    pub format: Format,
    pub direction: Direction,
    pub source_name: Option<String>,
    pub source_sha256: Option<String>,
    pub source_ppq: i64,
    pub issues: Vec<FieldIssue>,
}
impl ExchangeReport {
    pub fn has_losses(&self) -> bool {
        self.issues.iter().any(|i| matches!(i.disposition, Disposition::Unsupported | Disposition::Quantized | Disposition::RetainedUnsupported))
    }
    fn new(format: Format, direction: Direction) -> Self {
        Self { schema: "verdant-exchange/1".into(), format, direction, source_name: None,
            source_sha256: None, source_ppq: verdant_domain::PPQ, issues: Vec::new() }
    }
    pub(crate) fn issue(&mut self, path: impl Into<String>, disposition: Disposition, value: impl Into<Value>, detail: impl Into<String>) {
        self.issues.push(FieldIssue { path: path.into(), disposition, value: value.into(), detail: detail.into() });
    }
}
#[derive(Debug)]
pub struct ImportResult { pub project: Project, pub report: ExchangeReport }
#[derive(Debug)]
pub struct ExportResult { pub bytes: Vec<u8>, pub report: ExchangeReport }
#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
impl ExchangeError {
    pub(crate) fn at(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid { path: path.into(), message: message.into() }
    }
}

pub fn import(format: Format, bytes: &[u8], options: &ImportOptions) -> Result<ImportResult> {
    if bytes.len() > options.limits.max_file_bytes { return Err(ExchangeError::at("/", "file byte limit exceeded")); }
    let mut report = ExchangeReport::new(format, Direction::Import);
    report.source_name = options.source_name.clone();
    report.source_sha256 = Some(hex::encode(Sha256::digest(bytes)));
    let mut project = match format {
        Format::Midi => midi::import(bytes, options, &mut report)?,
        Format::Svp => svp::import(bytes, options, &mut report)?,
        Format::Vsqx => xml::import(bytes, options, &mut report)?,
        Format::Vpr => vocaloid::import_vpr(bytes, options, &mut report)?,
        Format::Ufdata => ufdata::import(bytes, options, &mut report)?,
        Format::Ust | Format::Ustx => return Err(ExchangeError::at("/format", "UST/USTX import is owned by verdant-import")),
    };
    for track in &mut project.tracks { track.refresh_index(); }
    common::validate(&project, &options.limits, None)?;
    if !options.preserve_extensions { for issue in &mut report.issues { if issue.disposition == Disposition::RetainedUnsupported { issue.disposition=Disposition::Unsupported;issue.detail.push_str(" Extension retention was disabled by the caller."); } } }
    project.extensions.insert("exchange_report".into(), serde_json::to_value(&report)?);
    Ok(ImportResult { project, report })
}

pub fn export(format: Format, project: &Project, options: &ExportOptions) -> Result<ExportResult> {
    common::validate(project, &options.limits, Some(format))?;
    if options.track.is_some_and(|i| i >= project.tracks.len()) { return Err(ExchangeError::at("/options/track", "track does not exist")); }
    let mut report = ExchangeReport::new(format, Direction::Export);
    report.source_ppq = project.ppq;
    common::report_project_fields(project, options, &mut report)?;
    let bytes = match format {
        Format::Midi => midi::export(project, options, &mut report)?,
        Format::Svp => svp::export(project, options, &mut report)?,
        Format::Vsqx => xml::export(project, options, &mut report)?,
        Format::Vpr => vocaloid::export_vpr(project, options, &mut report)?,
        Format::Ufdata => ufdata::export(project, options, &mut report)?,
        Format::Ust => ust::export_ust(project, options, &mut report)?,
        Format::Ustx => ust::export_ustx(project, options, &mut report)?,
    };
    if bytes.len() > options.limits.max_file_bytes { return Err(ExchangeError::at("/", "output byte limit exceeded")); }
    Ok(ExportResult { bytes, report })
}

/// Decode a native project with the domain's format/version admission,
/// recursive duplicate-key rejection and interchange resource/timing limits.
pub fn parse_project_json(bytes: &[u8], limits: &Limits) -> Result<Project> {
    if bytes.len() > limits.max_file_bytes { return Err(ExchangeError::at("/", "project byte limit exceeded")); }
    let mut project: Project = serde_json::from_value(common::parse_json(bytes, false)?)?;
    // Match domain::io::parse_project: the native family is forward-compatible
    // within 0.x, but a filename or a structurally similar JSON object is not
    // permission to interpret another format/major version as this document.
    if project.format != verdant_domain::model::FORMAT {
        return Err(ExchangeError::at("/format", "expected greendam-project native document"));
    }
    if !project.version.starts_with("0.") {
        return Err(ExchangeError::at("/version", "unsupported native project version; expected 0.x"));
    }
    if project.project_id.is_empty() || verdant_domain::Revision::new(project.revision).is_none() {
        return Err(ExchangeError::at("/project_id", "empty project_id or revision outside the native exact range"));
    }
    for track in &mut project.tracks { track.refresh_index(); }
    common::validate(&project, limits, None)?;
    Ok(project)
}
/// Decode a CLI/export-options sidecar without silently overwriting duplicates.
pub fn parse_export_options_json(bytes: &[u8]) -> Result<ExportOptions> {
    if bytes.len() > 1024 * 1024 { return Err(ExchangeError::at("/options", "options sidecar exceeds one MiB")); }
    Ok(serde_json::from_value(common::parse_json(bytes, false)?)?)
}
