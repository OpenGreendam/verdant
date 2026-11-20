//! Writable native user directories. Explicit Verdant overrides win on every host.
use std::{env, path::PathBuf};

fn variable(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    { variable("USERPROFILE").or_else(|| variable("HOME")) }
    #[cfg(not(windows))]
    { variable("HOME") }
}

fn user_dir(kind: &str, override_name: &str, xdg: &str, linux_suffix: &str) -> PathBuf {
    if let Some(path) = variable(override_name) { return path; }
    #[cfg(windows)]
    if let Some(base) = variable("LOCALAPPDATA").or_else(|| home_dir().map(|home| home.join("AppData/Local"))) {
        return base.join("Verdant").join(kind);
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = home_dir() {
        return if kind == "cache" { home.join("Library/Caches/Verdant") }
            else { home.join("Library/Application Support/Verdant").join(kind) };
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    if let Some(base) = variable(xdg).or_else(|| home_dir().map(|home| home.join(linux_suffix))) {
        return base.join("verdant");
    }
    let _ = (xdg, linux_suffix);
    // Retain the Linux state fallback used by the desktop, and make every
    // missing-home fallback visible instead of writing into the launch directory.
    let path = if cfg!(any(windows, target_os = "macos")) { env::temp_dir().join("Verdant").join(kind) }
        else { env::temp_dir().join(format!("verdant-{kind}")).join("verdant") };
    eprintln!("verdant: no native user directory for {kind}; using temporary {} (set {override_name} for persistent storage)", path.display());
    path
}

pub fn state_dir() -> PathBuf { user_dir("state", "VERDANT_STATE_DIR", "XDG_STATE_HOME", ".local/state") }
pub fn data_dir() -> PathBuf { user_dir("data", "VERDANT_USER_DATA_DIR", "XDG_DATA_HOME", ".local/share") }
pub fn cache_dir() -> PathBuf { user_dir("cache", "VERDANT_CACHE_DIR", "XDG_CACHE_HOME", ".cache") }

/// Read-only package resources; distinct from writable voice/user data.
pub fn resource_dir() -> Option<PathBuf> {
    if let Some(path) = variable("VERDANT_DATA_DIR") { return Some(path); }
    let executable = env::current_exe().ok()?;
    let directory = executable.parent()?;
    let candidate = if cfg!(target_os = "macos") { directory.join("../Resources/share/verdant") }
        else { directory.join("../share/verdant") };
    if candidate.is_dir() { return candidate.canonicalize().ok(); }
    // Linux's desktop executable lives in libexec/verdant rather than bin.
    let candidate = directory.join("../../share/verdant");
    candidate.is_dir().then(|| candidate.canonicalize().ok()).flatten()
}
