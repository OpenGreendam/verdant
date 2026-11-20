//! Plugin discovery: per-file probes executed by short-lived
//! `verdant-plughost scan-file` subprocesses under a timeout, with a
//! sha256-keyed JSON cache and a blacklist for crashing bundles
//! (PLUGIN-HOST.md §扫描).

use crate::error::{PluginError, Result};
use crate::proto::ScanFileReport;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Per-bundle probe deadline; a plugin that can't open in 8s is broken.
pub const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
/// `.vst3` bundle recursion depth (Contents/…/bundle.vst3 sits ~3 deep).
const VST3_DEPTH: usize = 4;

/// One scan outcome per plugin file/bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub path: PathBuf,
    pub sha256: String,
    pub file_size: u64,
    /// Descriptors on success.
    #[serde(default)]
    pub plugins: Vec<crate::proto::ScannedPlugin>,
    /// Probe failure — also recorded so we don't re-probe a crashing bundle
    /// until its bytes change.
    #[serde(default)]
    pub error: Option<String>,
}

/// Aggregate of a `PluginClient::scan*` call.
#[derive(Debug, Default)]
pub struct ScanSummary {
    /// New descriptors discovered this scan, with their source bundle.
    pub discovered: Vec<(PathBuf, crate::proto::ScannedPlugin)>,
    /// Files that are (newly) blacklisted.
    pub blacklisted: Vec<(PathBuf, String)>,
    /// How many entries were served from cache without probing.
    pub cached: usize,
}

/// On-disk scan cache: `cache_dir/plugin-scan.json`, keyed by path.
#[derive(Default, Serialize, Deserialize)]
struct ScanCache {
    entries: Vec<ScanReport>,
}

fn cache_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("plugin-scan.json")
}

pub fn load_cache(cache_dir: &Path) -> Vec<ScanReport> {
    fs::read(cache_path(cache_dir))
        .ok()
        .and_then(|b| serde_json::from_slice::<ScanCache>(&b).ok())
        .map(|c| c.entries)
        .unwrap_or_default()
}

fn save_cache(cache_dir: &Path, entries: &[ScanReport]) -> io::Result<()> {
    fs::create_dir_all(cache_dir)?;
    let tmp = cache_path(cache_dir).with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec(&ScanCache {
        entries: entries.to_vec(),
    })?)?;
    fs::rename(&tmp, cache_path(cache_dir))?;
    Ok(())
}

/// Hash every regular bundle file, with length-delimited relative names and
/// bytes. Directory enumeration errors invalidate the probe rather than
/// admitting a partial fingerprint. Stream large binaries in bounded memory.
pub fn sha256_bundle(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    if path.is_dir() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_files(path, &mut files)?;
        files.sort();
        for f in files {
            let rel = f.strip_prefix(path).unwrap_or(&f).to_string_lossy();
            hasher.update((rel.len() as u64).to_le_bytes());
            hasher.update(rel.as_bytes());
            hash_file(&mut hasher, &f)?;
        }
    } else {
        hash_file(&mut hasher, path)?;
    }
    Ok(hex::encode(hasher.finalize()))
}

fn hash_file(hasher: &mut Sha256, path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        hasher.update(b"symlink");
        hasher.update(target.as_os_str().as_encoded_bytes());
        // A linked binary must also invalidate the cache when its target changes.
        if !path.is_file() { return Err(io::Error::other("bundle directory symlink unsupported")); }
    }
    let mut file = fs::File::open(path)?;
    hasher.update(file.metadata()?.len().to_le_bytes());
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(())
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for e in fs::read_dir(dir)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            collect_files(&e.path(), out)?;
        } else {
            out.push(e.path());
        }
    }
    Ok(())
}

/// Find candidate plugin files under `dirs`.
/// `.clap` = file, `.vst3` = bundle dir, `.component` = AU bundle dir.
pub fn discover_candidates(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        if is_candidate(dir) { out.push(dir.clone()); }
        else { walk(dir, 0, &mut out); }
    }
    out.sort();
    out.dedup();
    out
}

fn is_candidate(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("clap" | "vst3" | "component") => true,
        Some("dll") => cfg!(windows),
        _ => false,
    }
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > VST3_DEPTH {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if is_candidate(&p) {
            out.push(p);
        } else if e.file_type().is_ok_and(|t| t.is_dir()) && !name.starts_with('.') {
            walk(&p, depth + 1, out);
        }
    }
}

/// Run `verdant-plughost scan-file <path>` under the deadline.
pub fn run_scan_file(plughost: &Path, path: &Path, timeout: Duration) -> Result<ScanFileReport> {
    let mut child = Command::new(plughost)
        .arg("scan-file")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| PluginError::PlughostUnavailable(format!("scan spawn: {e}")))?;

    let result = (|| {
        let mut stdout = child.stdout.take().expect("piped scan stdout");
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let fd = stdout.as_raw_fd();
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
                    return Err(io::Error::last_os_error().into());
                }
            }
        }
        let deadline = std::time::Instant::now() + timeout;
        let mut out = Vec::new();
        loop {
            drain_scan_stdout(&mut stdout, &mut out, deadline)?;
            if let Some(status) = child.try_wait()? {
                // All host bytes were written before exit; one final pass
                // drains every remaining chunk on both Unix and Windows.
                // Never wait for EOF from an unrelated inherited writer.
                drain_scan_stdout(&mut stdout, &mut out, deadline)?;
                let mut report: ScanFileReport = serde_json::from_slice(&out).map_err(|e| {
                    PluginError::Protocol(format!("scan-file {path:?}: {status}: {e}"))
                })?;
                if !status.success() && report.error.is_none() {
                    report.error = Some(format!("probe exited {status}"));
                }
                if report.plugins.is_empty() && report.error.is_none() {
                    report.error = Some("bundle exposes no supported audio effects".into());
                }
                return Ok(report);
            }
            if std::time::Instant::now() >= deadline {
                return Err(PluginError::Timeout(format!("scan-file {path:?} exceeded {timeout:?}")));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    })();
    if result.is_err() { let _ = child.kill(); }
    let _ = child.wait();
    result
}

fn drain_scan_stdout(
    stdout: &mut std::process::ChildStdout,
    out: &mut Vec<u8>,
    deadline: std::time::Instant,
) -> Result<()> {
    let mut bytes = [0u8; 4096];
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(PluginError::Timeout("scan output deadline exceeded".into()));
        }
        #[cfg(windows)]
        let limit = {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_PIPE_NOT_CONNECTED};
            let mut available = 0;
            let ok = unsafe {
                windows_sys::Win32::System::Pipes::PeekNamedPipe(
                    stdout.as_raw_handle(), std::ptr::null_mut(), 0,
                    std::ptr::null_mut(), &mut available, std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if matches!(error.raw_os_error(), Some(code) if code == ERROR_BROKEN_PIPE as i32 || code == ERROR_PIPE_NOT_CONNECTED as i32) {
                    return Ok(());
                }
                return Err(error.into());
            }
            if available == 0 { return Ok(()); }
            (available as usize).min(bytes.len())
        };
        #[cfg(unix)]
        let limit = bytes.len();
        match stdout.read(&mut bytes[..limit]) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                if out.len() + n > crate::proto::MAX_LINE {
                    return Err(PluginError::Protocol("scan output exceeds limit".into()));
                }
                out.extend_from_slice(&bytes[..n]);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
}

/// Scan `paths` (files or dirs). Uses `cache_dir/plugin-scan.json` keyed by
/// `path+sha256`: unchanged bytes → cached result, changed bytes → re-probe.
/// Failed probes are recorded as `ScanReport.error` (the blacklist).
pub fn scan_paths(
    plughost: &Path,
    cache_dir: &Path,
    paths: &[PathBuf],
) -> ScanSummary {
    let candidates = discover_candidates(paths);
    let mut cache = load_cache(cache_dir);
    let mut summary = ScanSummary::default();
    let mut changed = false;

    for path in candidates {
        let sha = match sha256_bundle(&path) {
            Ok(s) => s,
            Err(error) => {
                summary.blacklisted.push((path.clone(), format!("bundle fingerprint: {error}")));
                let old_len = cache.len();
                cache.retain(|entry| entry.path != path);
                changed |= old_len != cache.len();
                // No valid content identity exists for a cache entry. Keep
                // the per-path failure visible and remove stale descriptors.
                continue;
            }
        };
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let idx = cache.iter().position(|e| e.path == path);
        if let Some(i) = idx {
            if cache[i].sha256 == sha {
                // Unchanged: report cached error as still-blacklisted.
                if let Some(err) = &cache[i].error {
                    summary.blacklisted.push((path.clone(), err.clone()));
                } else {
                    summary.cached += 1;
                    summary.discovered.extend(cache[i].plugins.iter().cloned().map(|p| (path.clone(), p)));
                }
                continue;
            }
            cache.remove(i);
        }

        let rep = match run_scan_file(plughost, &path, SCAN_TIMEOUT) {
            Ok(r) => {
                if r.sha256 != sha {
                    summary.blacklisted.push((path.clone(), "bundle fingerprint changed or failed during probe".into()));
                    changed = true;
                    continue;
                }
                ScanReport {
                    path: path.clone(), sha256: sha,
                    file_size: r.file_size, plugins: r.plugins, error: r.error,
                }
            }
            Err(e) => ScanReport {
                path: path.clone(),
                sha256: sha.clone(),
                file_size: size,
                plugins: vec![],
                error: Some(e.to_string()),
            },
        };
        if let Some(err) = &rep.error {
            summary.blacklisted.push((path.clone(), err.clone()));
        } else {
            summary
                .discovered
                .extend(rep.plugins.iter().cloned().map(|p| (path.clone(), p)));
        }
        cache.push(rep);
        changed = true;
    }
    if changed {
        let _ = save_cache(cache_dir, &cache);
    }
    summary
}

/// Default plugin directories for the current platform.
pub fn default_scan_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(target_os = "linux")]
    {
        if let Some(h) = std::env::var_os("HOME") {
            v.push(Path::new(&h).join(".clap"));
            v.push(Path::new(&h).join(".vst3"));
        }
        v.push(PathBuf::from("/usr/lib/clap"));
        v.push(PathBuf::from("/usr/local/lib/clap"));
        v.push(PathBuf::from("/usr/lib/vst3"));
        v.push(PathBuf::from("/usr/local/lib/vst3"));
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(h) = std::env::var_os("HOME") {
            v.push(Path::new(&h).join("Library/Audio/Plug-Ins/CLAP"));
            v.push(Path::new(&h).join("Library/Audio/Plug-Ins/VST3"));
            v.push(Path::new(&h).join("Library/Audio/Plug-Ins/Components"));
        }
        v.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
        v.push(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
        v.push(PathBuf::from("/Library/Audio/Plug-Ins/Components"));
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            v.push(Path::new(&pf).join("Common Files/CLAP"));
            v.push(Path::new(&pf).join("Common Files/VST3"));
        }
        if let Some(la) = std::env::var_os("LOCALAPPDATA") {
            v.push(Path::new(&la).join("Programs/Common/CLAP"));
            v.push(Path::new(&la).join("Programs/Common/VST3"));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_find_bundles_and_files() {
        let tmp = std::env::temp_dir().join(format!("vp-scan-{}", std::process::id()));
        let nested = tmp.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(tmp.join("x.vst3").join("Contents")).unwrap();
        fs::write(tmp.join("y.clap"), b"fake").unwrap();
        let got = discover_candidates(&[tmp.clone()]);
        assert!(got.iter().any(|p| p.ends_with("x.vst3")));
        assert!(got.iter().any(|p| p.ends_with("y.clap")));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn cache_key_is_content_not_mtime() {
        let tmp = std::env::temp_dir().join(format!("vp-sha-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("p.clap");
        fs::write(&f, b"abc").unwrap();
        let a = sha256_bundle(&f).unwrap();
        fs::write(&f, b"abd").unwrap();
        let b = sha256_bundle(&f).unwrap();
        assert_ne!(a, b);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[cfg(unix)]
    #[test]
    fn fingerprint_failure_is_visible_and_evicts_old_identity() {
        let tmp = std::env::temp_dir().join(format!("vp-scan-error-{}", uuid::Uuid::new_v4()));
        let bundle = tmp.join("broken.vst3");
        let cache_dir = tmp.join("cache");
        fs::create_dir_all(&bundle).unwrap();
        let target = tmp.join("directory");
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, bundle.join("linked-directory")).unwrap();
        save_cache(&cache_dir, &[ScanReport { path: bundle.clone(), sha256: "old-identity".into(),
            file_size: 0, plugins: vec![], error: None }]).unwrap();
        let summary = scan_paths(Path::new("unused-host"), &cache_dir, &[bundle.clone()]);
        assert!(summary.discovered.is_empty());
        assert_eq!(summary.blacklisted.len(), 1);
        assert_eq!(summary.blacklisted[0].0, bundle);
        assert!(summary.blacklisted[0].1.contains("bundle fingerprint:"));
        assert!(load_cache(&cache_dir).is_empty());
        fs::remove_dir_all(tmp).unwrap();
    }
}
