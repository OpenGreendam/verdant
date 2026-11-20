//! Load/save with atomic replace (SPEC §4.3): sync the complete temporary
//! file before replacing the destination in the same directory.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::model::{FORMAT, Project};

/// The only native project filename extension. Document headers are validated
/// independently after filename admission.
pub const PROJECT_EXTENSION: &str = "verp";

pub const RETIRED_PROJECT_FORMAT_MESSAGE: &str = "旧格式已不再支持：请使用 .verp 工程或受支持的乐谱交换格式";

/// Reject retired project format names before reading inputs or writing outputs.
pub fn reject_retired_project_format(format: &str) -> Result<(), IoError> {
    let format = format.trim_start_matches('.');
    if ["json", "verdant.json", "gdmproj"].iter().any(|retired| format.eq_ignore_ascii_case(retired)) {
        return Err(IoError::RetiredProjectFormat);
    }
    Ok(())
}

/// Candidate native filenames only; callers must still validate the document.
pub fn has_project_extension(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(PROJECT_EXTENSION))
}

/// Admit a native filename before any filesystem or recovery-log operation.
pub fn require_project_path(path: &Path) -> Result<(), IoError> {
    reject_retired_project_format(path.extension().and_then(|value| value.to_str()).unwrap_or(""))?;
    if has_project_extension(path) { return Ok(()); }
    Err(IoError::Document("native projects require a .verp filename".into()))
}

/// Give an extensionless new destination the native suffix; refuse other formats.
pub fn project_output_path(path: &Path) -> Result<PathBuf, IoError> {
    if path.file_name().is_none() {
        return Err(IoError::Document("project destination needs a filename".into()));
    }
    if path.extension().is_none() { return Ok(path.with_extension(PROJECT_EXTENSION)); }
    require_project_path(path)?;
    Ok(path.to_path_buf())
}

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("{}", RETIRED_PROJECT_FORMAT_MESSAGE)]
    RetiredProjectFormat,
    #[error("I/O error on {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("not a greendam-project document: {0}")]
    Format(String),
    #[error("unsupported greendam-project version {0} (this build reads 0.x)")]
    Version(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid project document: {0}")]
    Document(String),
}

#[derive(Debug, Default, Clone)]
pub struct LoadReport {
    /// Unknown top-level or per-object keys that will be preserved verbatim.
    pub preserved_unknown_keys: Vec<String>,
}

fn check_document_header(format: &str, version: &str) -> Result<(), IoError> {
    if format != FORMAT {
        return Err(IoError::Format(format.to_string()));
    }
    if !version.starts_with("0.") {
        return Err(IoError::Version(version.to_string()));
    }
    Ok(())
}

pub fn parse_project(text: &str) -> Result<(Project, LoadReport), IoError> {
    let mut project: Project = serde_json::from_str(text)?;
    check_document_header(&project.format, &project.version)?;
    if project.project_id.is_empty() || crate::Revision::new(project.revision).is_none() {
        return Err(IoError::Document("empty project_id or revision outside 0..=2^53-1".into()));
    }
    let mut report = LoadReport::default();
    for t in &mut project.tracks {
        t.refresh_index();
    }
    report_keys(&mut report, "", project.unknown.keys());
    report_keys(&mut report, "/render_preferences", project.render_preferences.unknown.keys());
    if let Some(settings) = &project.render_preferences.synthesis {
        report_keys(&mut report, "/render_preferences/synthesis", settings.unknown.keys());
        if let Some(wideband) = &settings.wideband {
            report_keys(&mut report, "/render_preferences/synthesis/wideband", wideband.unknown.keys());
        }
        if let Some(correction) = &settings.phonetic_correction {
            report_keys(&mut report, "/render_preferences/synthesis/phonetic_correction", correction.unknown.keys());
        }
    }
    if let Some(guard) = &project.render_preferences.vocoder_highband {
        report_keys(&mut report, "/render_preferences/vocoder_highband", guard.unknown.keys());
    }
    if let Some(tract) = &project.render_preferences.vocal_tract {
        report_keys(&mut report, "/render_preferences/vocal_tract", tract.unknown.keys());
    }
    if let Some(dynamics) = &project.render_preferences.model_dynamics {
        report_keys(&mut report, "/render_preferences/model_dynamics", dynamics.unknown.keys());
    }
    if let Some(timing) = &project.render_preferences.phoneme_timing {
        report_keys(&mut report, "/render_preferences/phoneme_timing", timing.unknown.keys());
    }
    if let Some(dynamics) = &project.render_preferences.consonant_dynamics {
        report_keys(&mut report, "/render_preferences/consonant_dynamics", dynamics.unknown.keys());
    }
    if let Some(settings) = &project.render_preferences.output_transients {
        report_keys(&mut report, "/render_preferences/output_transients", settings.unknown.keys());
    }
    for (name, voice) in &project.voice_refs {
        let name = name.replace('~', "~0").replace('/', "~1");
        report_keys(&mut report, &format!("/voice_refs/{name}"), voice.unknown.keys());
    }
    for (index, point) in project.tempo_map.iter().enumerate() {
        report_keys(&mut report, &format!("/tempo_map/{index}"), point.unknown.keys());
    }
    for (index, signature) in project.time_signatures.iter().enumerate() {
        report_keys(&mut report, &format!("/time_signatures/{index}"), signature.unknown.keys());
    }
    for (ti, t) in project.tracks.iter().enumerate() {
        report_keys(&mut report, &format!("/tracks/{ti}"), t.unknown.keys());
        for (ni, n) in t.notes.iter().enumerate() {
            let base = format!("/tracks/{ti}/notes/{ni}");
            report_keys(&mut report, &base, n.unknown.keys());
            report_keys(&mut report, &format!("{base}/pronunciation"), n.pronunciation.unknown.keys());
            report_keys(&mut report, &format!("{base}/curves"), n.curves.unknown.keys());
            if let Some(vibrato) = &n.vibrato {
                report_keys(&mut report, &format!("{base}/vibrato"), vibrato.unknown.keys());
            }
            for (index, point) in n.curves.pitch_millicent_points.iter().enumerate() {
                report_keys(&mut report, &format!("{base}/curves/pitch_millicent_points/{index}"), point.unknown.keys());
            }
            for (index, point) in n.curves.gain_millidecibel_points.iter().enumerate() {
                report_keys(&mut report, &format!("{base}/curves/gain_millidecibel_points/{index}"), point.unknown.keys());
            }
            for (parameter, points) in &n.curves.expression_points {
                for (index, point) in points.iter().enumerate() {
                    report_keys(&mut report, &format!("{base}/curves/expression_points/{parameter}/{index}"), point.unknown.keys());
                }
            }
        }
    }
    Ok((project, report))
}

fn report_keys<'a>(report: &mut LoadReport, base: &str, keys: impl IntoIterator<Item = &'a String>) {
    report.preserved_unknown_keys.extend(keys.into_iter().map(|key| {
        let key = key.replace('~', "~0").replace('/', "~1");
        format!("{base}/{key}")
    }));
}

pub fn load_project(path: &Path) -> Result<(Project, LoadReport), IoError> {
    require_project_path(path)?;
    let text = fs::read_to_string(path).map_err(|source| IoError::Io { path: path.into(), source })?;
    parse_project(&text)
}

pub fn save_project_atomic(path: &Path, project: &Project) -> Result<(), IoError> {
    save_project(path, project, false)
}

pub(crate) fn save_project_new(path: &Path, project: &Project) -> Result<(), IoError> {
    save_project(path, project, true)
}

fn save_project(path: &Path, project: &Project, new_only: bool) -> Result<(), IoError> {
    require_project_path(path)?;
    atomic_write(path, new_only, |file| {
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, project)?;
        writer.write_all(b"\n").and_then(|()| writer.flush())
            .map_err(|source| IoError::Io { path: path.into(), source })
    })
}

/// Same-directory replacement for document saves, journal checkpoints and
/// distribution exports.
pub(crate) fn atomic_replace(path: &Path, write: impl FnOnce(&mut File) -> Result<(), IoError>) -> Result<(), IoError> {
    atomic_write(path, false, write)
}

/// Publish a complete file only if the destination is still absent. A
/// same-directory hard link provides an atomic no-replace operation; an
/// unsupported filesystem returns an error rather than risking an overwrite.
pub(crate) fn atomic_create(path: &Path, write: impl FnOnce(&mut File) -> Result<(), IoError>) -> Result<(), IoError> {
    atomic_write(path, true, write)
}

fn atomic_write(path: &Path, new_only: bool, write: impl FnOnce(&mut File) -> Result<(), IoError>) -> Result<(), IoError> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    ensure_directory(dir)?;
    let tmp = dir.join(format!(".verdant-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|source| IoError::Io { path: tmp.clone(), source })?;
        write(&mut file)?;
        file.sync_all().map_err(|source| IoError::Io { path: tmp.clone(), source })?;
        drop(file);
        if new_only {
            fs::hard_link(&tmp, path).map_err(|source| IoError::Io { path: path.into(), source })?;
            fs::remove_file(&tmp).map_err(|source| IoError::Io { path: tmp.clone(), source })?;
            #[cfg(not(windows))]
            sync_directory(dir)?;
            Ok(())
        } else {
            replace_file(dir, &tmp, path)
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(dir: &Path, temporary: &Path, destination: &Path) -> Result<(), IoError> {
    fs::rename(temporary, destination)
        .map_err(|source| IoError::Io { path: destination.into(), source })?;
    sync_directory(dir)
}

#[cfg(windows)]
fn replace_file(dir: &Path, temporary: &Path, destination: &Path) -> Result<(), IoError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};

    let filename = destination.file_name()
        .ok_or_else(|| IoError::Document("destination needs a filename".into()))?;
    // Resolve the parent once to obtain the verbatim path for long names.
    let mut absolute = dir.canonicalize().map_err(|source| IoError::Io { path: dir.into(), source })?;
    absolute.push(temporary.file_name().expect("temporary replacement filename"));
    let source: Vec<u16> = absolute.as_os_str().encode_wide().chain([0]).collect();
    absolute.set_file_name(filename);
    let target: Vec<u16> = absolute.as_os_str().encode_wide().chain([0]).collect();
    if target[..target.len() - 1].contains(&0) {
        return Err(IoError::Document("destination contains a NUL character".into()));
    }

    // Both names are NUL-terminated paths that stay live through the call.
    let replaced = unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) };
    if replaced != 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.kind() != std::io::ErrorKind::PermissionDenied {
        return Err(IoError::Io { path: destination.into(), source: error });
    }
    // Rust's Windows rename can replace a delete-shared open destination
    // via FileRenameInfoEx. Checkpointing leaves the old writer open until
    // the replacement succeeds.
    fs::rename(temporary, destination)
        .map_err(|source| IoError::Io { path: destination.into(), source })
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), IoError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| IoError::Io { path: path.into(), source })
}

fn ensure_directory(path: &Path) -> Result<(), IoError> {
    if path.is_dir() {
        return Ok(());
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    ensure_directory(parent)?;
    match fs::create_dir(path) {
        #[cfg(not(windows))]
        Ok(()) => sync_directory(parent),
        #[cfg(windows)]
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        Err(source) => Err(IoError::Io { path: path.into(), source }),
    }
}
