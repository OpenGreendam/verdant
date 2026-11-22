//! Verdant plugin host client (see `frontend/docs/PLUGIN-HOST.md`).
//!
//! Process-isolated hosting of CLAP, VST3 and (macOS) Audio Unit effects. Each
//! live insert is served by its own `verdant-plughost` child process: audio
//! flows through bounded shared-memory queues with 9472 frames of RT transport
//! latency (197⅓ ms at 48 kHz); native DSP blocks remain 256 frames. Control uses
//! a line-delimited JSON protocol on
//! pipes. A crashed or stalled plugin can only turn into passthrough + a
//! visible xrun/missing badge — the real-time callback never waits.
//!
//! Persistence lives in `track.unknown["inserts"]` (serde-flattened map on
//! `verdant_domain::Track`; PLUGIN-HOST.md's `track.extensions` is that map).
//! Use [`project::read_inserts`] / [`project::write_inserts`].
//!
//! Compile string parameter ids on the control thread with
//! `InsertHandle::compile_parameter`. Store native [`ParamEvent`] tuples in
//! caller-owned buffers and pass them to `Processor::process_automated`, or to
//! `InsertHandle::process_offline_events` for export. The same shared-memory
//! slot owns audio and its sample offsets. A quantum accepts at most
//! [`MAX_PARAM_EVENTS`]; invalid input rejects the entire call before publication.
//! Parameter IPC, editor/state operations and target discovery stay off RT.
//! Discrete parameters accept native step values; changing linear curves must
//! be rejected by the schedule compiler rather than silently quantized.

#[cfg(feature = "client")]
pub mod bridge;
pub mod builtin_dsp;
pub mod error;
#[cfg(feature = "client")]
pub mod insert;
#[cfg(feature = "client")]
pub mod plugin;
pub mod project;
pub mod proto;
pub mod preset_interop;
pub mod scan;
pub mod shm;
pub mod telemetry;

#[cfg(feature = "client")]
pub use bridge::{DelayPad, RemoteInsert, RemoteState};
pub use error::PluginError;
#[cfg(feature = "client")]
pub use insert::{InsertHandle, InsertSlot, InsertStatus};
pub use proto::{AutomationError, AutomationParam, ParamDesc, ParamEvent, ParamTarget, MAX_PARAM_EVENTS};
#[cfg(feature = "client")]
pub use plugin::{
    build_chain, ClientEvent, InsertKey, PluginClient, PluginClientOpts, PluginDesc, PluginFormat,
};
pub use project::InsertSpec;
pub use scan::{ScanReport, ScanSummary};
pub use shm::{MAX_BLOCK_FRAMES, STATUS_CRASHED, STATUS_MISSING, STATUS_OK, STATUS_STALE_RATE};

/// Fixed source, insert and export rate; hardware adaptation belongs to audio.
pub const SAMPLE_RATE: f64 = 48_000.0;

/// The always-available in-host diagnostic plugin (see PLUGIN-HOST.md step 1).
pub const BUILTIN_GAIN_ID: &str = "verdant.builtin.gain";

/// Resolution order for the `verdant-plughost` binary:
/// explicit path → `VERDANT_PLUGIN_HOST` → `VERDANT_PLUGHOST` → sibling of the
/// current executable → `exe/../libexec/verdant/verdant-plughost` (package
/// layout set by the launcher).
pub fn resolve_plughost(explicit: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    use std::path::{Path, PathBuf};
    fn non_empty_path(v: Option<std::ffi::OsString>) -> Option<PathBuf> {
        v.and_then(|s| if s.is_empty() { None } else { Some(PathBuf::from(s)) })
            .filter(|p| p.is_file())
    }
    if let Some(p) = explicit.filter(|p| p.is_file()) {
        return Some(p.to_path_buf());
    }
    if let Some(p) = non_empty_path(std::env::var_os("VERDANT_PLUGIN_HOST")) {
        return Some(p);
    }
    if let Some(p) = non_empty_path(std::env::var_os("VERDANT_PLUGHOST")) {
        return Some(p);
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let binary = if cfg!(windows) { "verdant-plughost.exe" } else { "verdant-plughost" };
    for cand in [
        dir.join(binary),
        dir.join("..").join("libexec").join("verdant").join(binary),
    ] {
        if cand.is_file() {
            return Some(cand);
        }
    }
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    path_dirs
        .into_iter()
        .map(|d| d.join(binary))
        .find(|p| Path::new(p).is_file())
}
