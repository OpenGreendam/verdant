//! Format dispatch: `builtin` (always) + `clap`/`vst3`/`au` (feature/platform
//! gated). A `HostedPlugin` is one live plugin instance inside this process —
//! plugin crashes crash *this* process only, which is exactly the isolation
//! PLUGIN-HOST.md wants (the parent sees it as `InsertStatus::Crashed`).

pub mod builtin;
#[cfg(feature = "clap")]
pub mod clap;
#[cfg(all(feature = "au", target_os = "macos"))]
pub mod au;
#[cfg(feature = "vst3")]
pub mod vst3;

use verdant_plugins::error::PluginError;
use verdant_plugins::proto::{AutomationParam, ParamDesc, ParamEvent};
use verdant_plugins::proto::{ScanFileReport, ScannedPlugin};
use std::path::Path;

/// What `scan-file` reports about one bundle.
pub struct ScanOutcome {
    pub format: &'static str,
    pub plugins: Vec<ScannedPlugin>,
}

/// One instantiated plugin — implemented once per format.
pub trait HostedPlugin {
    /// Stereo in-place: `out_*` initially mirrors `in_*`? No — the caller
    /// guarantees `out` buffers are sized `n` and the implementation MUST fill
    /// them completely (input is NOT pre-copied).
    fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) -> Result<(), String>;

    /// Native sample-offset delivery. Unsupported formats must reject events,
    /// never report successful audio after silently ignoring automation.
    fn process_events(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32],
        events: &[ParamEvent]) -> Result<(), String> {
        if !events.is_empty() { return Err("native parameter automation unsupported".into()); }
        self.process(in_l, in_r, out_l, out_r)
    }

    /// Control-side discovery filters native writable/automatable flags.
    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        Err("native parameter automation unsupported".into())
    }

    /// Plugin-reported processing latency in samples.
    fn latency(&mut self) -> u64 {
        0
    }

    /// All parameters (id/name/value/min/max/unit/step_count as `ParamDesc`).
    fn params(&mut self) -> Result<Vec<ParamDesc>, String> {
        Ok(vec![])
    }
    /// Set one parameter by the id string from `params()`.
    fn set_param(&mut self, _id: &str, _value: f64) -> Result<(), String> {
        Err("no parameters".into())
    }

    /// Opaque state blob (format-native serialization).
    fn state(&mut self) -> Result<Vec<u8>, String> {
        Err("no state".into())
    }
    /// Restore a blob produced by `state()` on an instance of the same plugin.
    fn set_state(&mut self, _data: &[u8]) -> Result<(), String> {
        Err("no state".into())
    }

    /// Whether this plugin can show a native editor.
    fn has_editor(&self) -> bool {
        false
    }
    /// Open the native editor inside a host-created window titled `title`.
    fn open_editor(&mut self, _title: &str) -> Result<(), String> {
        Err("no editor".into())
    }
    fn close_editor(&mut self) {}
    fn editor_open(&self) -> bool {
        false
    }

    /// Re-activate for a new rate/block size (called on `reconfigure`).
    fn reconfigure(&mut self, _sample_rate: f64, _max_frames: u32) -> Result<(), String> {
        Err("reconfigure unsupported".into())
    }

    fn set_offline(&mut self, enabled: bool) -> Result<(), String>;

    /// Periodic work on the session worker: editor event pumps, tail/latency
    /// refreshes. Called at most every `IDLE_PERIOD`.
    fn service(&mut self) -> Result<(), String> { Ok(()) }
}

/// Descriptor returned by `instantiate` (for the reply).
pub struct InstDesc {
    pub id: String,
    pub name: String,
}

/// Channel the format uses to surface async host/plugin events
/// (`"param_changed"`, `"latency_changed"`, `"state_dirty"`, `"log"`, …);
/// the session forwards them as `HostMessage::Event`.
pub type EventTx = std::sync::mpsc::Sender<(
    String,
    serde_json::Map<String, serde_json::Value>,
)>;

/// Instantiate `plugin_id` from `path` for `format`.
/// `format` is the persisted tag (`clap`|`vst3`|`au`|`builtin`).
pub fn instantiate(
    format: &str,
    plugin_id: &str,
    path: &str,
    sample_rate: f64,
    max_frames: u32,
    events: EventTx,
) -> Result<(Box<dyn HostedPlugin>, InstDesc), PluginError> {
    if sample_rate != verdant_plugins::SAMPLE_RATE || max_frames == 0
        || max_frames as usize > verdant_plugins::shm::MAX_BLOCK_FRAMES {
        return Err(PluginError::Host("effects require 48 kHz and a bounded nonzero block".into()));
    }
    match format {
        "builtin" if plugin_id == verdant_plugins::BUILTIN_GAIN_ID => {
            let p = builtin::Gain::new();
            Ok((
                Box::new(p),
                InstDesc {
                    id: verdant_plugins::BUILTIN_GAIN_ID.into(),
                    name: "Verdant Gain (builtin)".into(),
                },
            ))
        }
        #[cfg(feature = "clap")]
        "clap" => clap::instantiate(path, plugin_id, sample_rate, max_frames, events),
        #[cfg(feature = "vst3")]
        "vst3" => vst3::instantiate(path, plugin_id, sample_rate, max_frames, events),
        #[cfg(all(feature = "au", target_os = "macos"))]
        "au" => au::instantiate(path, plugin_id, sample_rate, max_frames, events),
        #[cfg(not(all(feature = "au", target_os = "macos")))]
        "au" => Err(PluginError::Host(
            "au hosting unavailable on this platform".into(),
        )),
        f => Err(PluginError::Host(format!("format '{f}' not enabled in this host"))),
    }
}

/// Formats this binary was built with.
pub fn supported_formats() -> Vec<String> {
    let mut v = vec!["builtin".to_string()];
    #[cfg(feature = "clap")]
    v.push("clap".into());
    #[cfg(feature = "vst3")]
    v.push("vst3".into());
    #[cfg(all(feature = "au", target_os = "macos"))]
    v.push("au".into());
    v
}

/// `verdant-plughost scan-file <path>`: identify the format, hash the bytes,
/// enumerate the plugins inside. Never instantiates — load only.
pub fn scan_file(path: &Path) -> Result<ScanFileReport, PluginError> {
    crate::window::initialize().map_err(PluginError::Host)?;
    let started = std::time::Instant::now();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let sha = verdant_plugins::scan::sha256_bundle(path)?;
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let outcome = match ext.as_str() {
        #[cfg(feature = "clap")]
        "clap" => clap::scan(path),
        #[cfg(feature = "vst3")]
        "vst3" | "dll" => vst3::scan(path),
        #[cfg(all(feature = "au", target_os = "macos"))]
        "component" => au::scan(path),
        _ => Err(PluginError::Host(format!(
            "unrecognized plugin container '.{ext}' at {path:?}"
        ))),
    };

    let report = match outcome {
        Ok(o) => ScanFileReport {
            path: path.to_string_lossy().into_owned(),
            format: o.format.into(),
            sha256: sha,
            file_size,
            plugins: o.plugins,
            error: None,
            scan_ms: started.elapsed().as_millis() as u64,
        },
        Err(e) => ScanFileReport {
            path: path.to_string_lossy().into_owned(),
            format: ext,
            sha256: sha,
            file_size,
            plugins: vec![],
            error: Some(e.to_string()),
            scan_ms: started.elapsed().as_millis() as u64,
        },
    };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Unautomated { calls: usize }
    impl HostedPlugin for Unautomated {
        fn process(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32]) -> Result<(), String> {
            self.calls += 1;
            ol.copy_from_slice(il);
            orr.copy_from_slice(ir);
            Ok(())
        }
        fn set_offline(&mut self, _: bool) -> Result<(), String> { Ok(()) }
    }

    #[test]
    fn unsupported_automation_is_an_error_before_processing_audio() {
        let mut plugin = Unautomated { calls: 0 };
        let mut l = [2.0; 1];
        let mut r = [3.0; 1];
        assert!(plugin.automation_params().is_err());
        assert!(plugin.process_events(&[1.0], &[1.0], &mut l, &mut r, &[(0, 0, 0.5)]).is_err());
        assert_eq!(plugin.calls, 0);
        assert_eq!(l, [2.0]);
        assert_eq!(r, [3.0]);
        plugin.process_events(&[1.0], &[1.0], &mut l, &mut r, &[]).unwrap();
        assert_eq!(plugin.calls, 1);
    }
}
