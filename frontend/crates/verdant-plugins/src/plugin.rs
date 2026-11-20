//! `PluginClient` — app-side owner of plugin discovery and every live
//! insert's host process. One client per app; one `verdant-plughost` child
//! per live insert.

use crate::error::{PluginError, Result};
use crate::insert::{InsertHandle, InsertSlot};
use crate::project::InsertSpec;
use crate::proto::ScannedPlugin;
use crate::scan::{self, ScanReport, ScanSummary};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Plugin container formats we can host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginFormat {
    Clap,
    Vst3,
    /// macOS Audio Unit (compile-gated).
    Au,
    /// The in-host diagnostic plugin (`verdant.builtin.gain`).
    Builtin,
    /// Anything else (unhosted / future).
    Unknown,
}

impl PluginFormat {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Clap => "clap",
            Self::Vst3 => "vst3",
            Self::Au => "au",
            Self::Builtin => "builtin",
            Self::Unknown => "unknown",
        }
    }
    pub fn from_tag(s: &str) -> Self {
        match s {
            "clap" => Self::Clap,
            "vst3" => Self::Vst3,
            "au" => Self::Au,
            "builtin" => Self::Builtin,
            _ => Self::Unknown,
        }
    }
}

/// One discovered plugin (what the "add insert" menu lists).
#[derive(Debug, Clone)]
pub struct PluginDesc {
    pub format: PluginFormat,
    /// Format-specific id used at instantiate time.
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    /// Bundle/file location it was found at.
    pub path: String,
    /// Native editor availability as discovered.
    pub has_editor: bool,
    /// Feature tags (CLAP features, VST3 category, AU type).
    pub features: Vec<String>,
}

impl From<&ScannedPlugin> for PluginDesc {
    fn from(s: &ScannedPlugin) -> Self {
        Self {
            format: PluginFormat::from_tag(&s.format),
            id: s.id.clone(),
            name: s.name.clone(),
            vendor: s.vendor.clone(),
            version: s.version.clone(),
            path: String::new(),
            has_editor: s.has_editor,
            features: s.features.clone(),
        }
    }
}

/// Async event flowing from any insert's host to the UI (crash notices,
/// param-change echoes, editor-closed, plugin log lines). The `insert` key
/// scopes it to one `InsertHandle`.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// The insert's host process died; the processor is permanently bypassed.
    Crashed { insert: InsertKey, detail: String },
    /// Forwarded `HostMessage::Event` (`event` is the clap-style name, e.g.
    /// `"param_changed"`, `"latency_changed"`, `"editor_closed"`, `"log"`).
    Event {
        insert: InsertKey,
        event: String,
        data: Map<String, Value>,
    },
    /// A `scan*` call finished.
    ScanFinished,
}

/// Stable per-insert identifier (also the tag on every `ClientEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InsertKey(pub u64);

/// Client construction knobs.
#[derive(Debug, Clone)]
pub struct PluginClientOpts {
    /// Explicit plughost path; `None` → [`crate::resolve_plughost`].
    pub plughost_path: Option<PathBuf>,
    /// Scan cache + blacklist directory (created on demand).
    pub cache_dir: PathBuf,
    /// Mixer/insert rate. Must be 48_000; devices resample at the output edge.
    pub sample_rate: f64,
    /// Device callback bound, at most `shm::MAX_BLOCK_FRAMES` (8192). Native DSP
    /// uses `shm::DSP_QUANTUM_FRAMES`; RT transport adds 9472 frames of latency.
    pub max_frames: usize,
}

/// The app's plugin host facade.
pub struct PluginClient {
    plughost: PathBuf,
    opts: PluginClientOpts,
    /// catalog as of the last scan: `format/id` → desc.
    catalog: Mutex<BTreeMap<(String, String), PluginDesc>>,
    /// `path → reason` for bundles that failed to probe.
    blacklist: Mutex<BTreeMap<PathBuf, String>>,
    events_tx: Sender<ClientEvent>,
    events_rx: Mutex<Receiver<ClientEvent>>,
    next_key: AtomicU64,
    /// All live handles — the client keeps them so `Drop` on remove actually
    /// tears down; callers hold `Arc`.
    handles: Mutex<BTreeMap<InsertKey, Arc<InsertHandle>>>,
}

impl PluginClient {
    /// Build a client. Fails when no `verdant-plughost` binary is resolvable;
    /// `opts.cache_dir` needn't exist yet.
    pub fn new(opts: PluginClientOpts) -> Result<Arc<Self>> {
        let plughost = crate::resolve_plughost(opts.plughost_path.as_deref()).ok_or_else(|| {
            PluginError::PlughostUnavailable(
                "verdant-plughost not found (set VERDANT_PLUGIN_HOST or install beside the app)"
                    .into(),
            )
        })?;
        Self::with_plughost(opts, plughost)
    }

    /// For tests and tools that already know the binary path.
    pub fn with_plughost(mut opts: PluginClientOpts, plughost: PathBuf) -> Result<Arc<Self>> {
        if opts.sample_rate != crate::SAMPLE_RATE {
            return Err(PluginError::Host("plugin processing requires 48 kHz".into()));
        }
        if opts.max_frames == 0 || opts.max_frames > crate::shm::MAX_BLOCK_FRAMES {
            return Err(PluginError::Host("plugin callbacks require a bound in 1..=8192 frames".into()));
        }
        opts.plughost_path = Some(plughost.clone());
        let (tx, rx) = channel();
        Ok(Arc::new(Self {
            plughost,
            opts,
            catalog: Mutex::new(BTreeMap::new()),
            blacklist: Mutex::new(BTreeMap::new()),
            events_tx: tx,
            events_rx: Mutex::new(rx),
            next_key: AtomicU64::new(1),
            handles: Mutex::new(BTreeMap::new()),
        }))
    }

    /// Drain pending events (call from the UI pump; never blocks).
    pub fn try_recv_event(&self) -> Option<ClientEvent> {
        self.events_rx.lock().unwrap().try_recv().ok()
    }

    /// The `verdant-plughost` binary in use.
    pub fn plughost_path(&self) -> &Path {
        &self.plughost
    }

    /// All plugins in the catalog (from the last scan + the builtin).
    pub fn catalog(&self) -> Vec<PluginDesc> {
        let mut v: Vec<PluginDesc> = self.catalog.lock().unwrap().values().cloned().collect();
        v.push(Self::builtin_desc());
        v
    }

    /// Blacklisted bundles + reasons (shown as "broken" in the UI, not
    /// silently dropped).
    pub fn blacklisted(&self) -> Vec<(PathBuf, String)> {
        self.blacklist
            .lock()
            .unwrap()
            .iter()
            .map(|(p, r)| (p.clone(), r.clone()))
            .collect()
    }

    /// Scan explicit directories (or plugin files). Probes run in
    /// subprocesses with timeouts; results merge into the catalog.
    pub fn scan(&self, dirs: &[PathBuf]) -> ScanSummary {
        let summary = scan::scan_paths(&self.plughost, &self.opts.cache_dir, dirs);
        self.merge_scan(&summary);
        let _ = self.events_tx.send(ClientEvent::ScanFinished);
        summary
    }

    /// Scan the platform-default plugin directories.
    pub fn scan_default_dirs(&self) -> ScanSummary {
        self.scan(&scan::default_scan_dirs())
    }

    /// Re-probe a single path (e.g. after install/update of one plugin).
    pub fn scan_file(&self, path: &Path) -> Result<ScanReport> {
        let rep = scan::run_scan_file(&self.plughost, path, scan::SCAN_TIMEOUT)?;
        let report = ScanReport {
            path: path.to_path_buf(),
            sha256: rep.sha256.clone(),
            file_size: rep.file_size,
            plugins: rep.plugins.clone(),
            error: rep.error.clone(),
        };
        let mut summary = ScanSummary::default();
        if let Some(e) = &report.error {
            summary.blacklisted.push((report.path.clone(), e.clone()));
        } else {
            summary
                .discovered
                .extend(report.plugins.iter().cloned().map(|p| (report.path.clone(), p)));
        }
        self.merge_scan(&summary);
        Ok(report)
    }

    fn merge_scan(&self, summary: &ScanSummary) {
        let mut cat = self.catalog.lock().unwrap();
        let mut bl = self.blacklist.lock().unwrap();
        for (path, p) in &summary.discovered {
            bl.remove(path);
            let mut d = PluginDesc::from(p);
            d.path = path.to_string_lossy().into_owned();
            cat.insert((p.format.clone(), p.id.clone()), d);
        }
        for (p, r) in &summary.blacklisted {
            cat.retain(|_, d| Path::new(&d.path) != p);
            bl.insert(p.clone(), r.clone());
        }
    }

    /// The always-present diagnostic plugin — real DSP inside the host
    /// process, used for bring-up and the e2e suite.
    fn builtin_desc() -> PluginDesc {
        PluginDesc {
            format: PluginFormat::Builtin,
            id: crate::BUILTIN_GAIN_ID.into(),
            name: "Verdant Gain (builtin)".into(),
            vendor: "Verdant".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            path: String::new(),
            has_editor: false,
            features: vec!["audio-effect".into()],
        }
    }

    /// A spec targeting the builtin gain (test/verification insert).
    pub fn builtin_gain_spec() -> InsertSpec {
        InsertSpec::new(
            PluginFormat::Builtin.tag(),
            crate::BUILTIN_GAIN_ID,
            "Verdant Gain (builtin)",
            "",
        )
    }

    /// Instantiate a live insert from a spec.
    pub fn insert(self: &Arc<Self>, spec: InsertSpec) -> Result<Arc<InsertHandle>> {
        let key = InsertKey(self.next_key.fetch_add(1, Ordering::Relaxed));
        let desc = self.resolve_desc(&spec)?;
        let handle = InsertHandle::spawn(
            key,
            spec,
            desc,
            &self.plughost,
            self.opts.sample_rate,
            self.events_tx.clone(),
        )?;
        let arc = Arc::new(handle);
        self.handles.lock().unwrap().insert(key, arc.clone());
        Ok(arc)
    }

    /// Fresh, exclusively owned host for an export. It never enters the live
    /// registry and cannot yield an RT processor. State/parameter changes in
    /// this instance cannot affect playback. Drop it on the export thread.
    pub fn offline_insert(&self, spec: InsertSpec) -> Result<InsertHandle> {
        let key = InsertKey(self.next_key.fetch_add(1, Ordering::Relaxed));
        let desc = self.resolve_desc(&spec)?;
        let mut handle = InsertHandle::spawn(
            key,
            spec,
            desc,
            &self.plughost,
            crate::SAMPLE_RATE,
            self.events_tx.clone(),
        )?;
        handle.make_offline()?;
        Ok(handle)
    }

    /// Restore a saved insert: finds the plugin by (`format`, `id`) — `path`
    /// in the spec is only a hint — and produces either a live handle or a
    /// `Missing` slot that keeps the spec (and its state bytes) intact.
    pub fn restore(self: &Arc<Self>, spec: &InsertSpec) -> InsertSlot {
        match self.insert(spec.clone()) {
            Ok(h) => InsertSlot::Live(h),
            Err(e) => InsertSlot::Missing {
                spec: spec.clone(),
                reason: e.to_string(),
            },
        }
    }

    /// Remove a live insert (processor should already be out of the chain —
    /// drop order: `Control::set_chain` replacement first, then this).
    pub fn remove(&self, key: InsertKey) {
        self.handles.lock().unwrap().remove(&key);
    }

    /// Look up the live handle for a key.
    pub fn handle(&self, key: InsertKey) -> Option<Arc<InsertHandle>> {
        self.handles.lock().unwrap().get(&key).cloned()
    }

    /// Map a spec to a catalog descriptor: builtin short-circuits, otherwise
    /// format+id lookup in the catalog (path is a hint for relocation only).
    fn resolve_desc(&self, spec: &InsertSpec) -> Result<PluginDesc> {
        let format = PluginFormat::from_tag(&spec.format);
        if format == PluginFormat::Builtin && spec.id == crate::BUILTIN_GAIN_ID {
            return Ok(Self::builtin_desc());
        }
        if matches!(format, PluginFormat::Unknown | PluginFormat::Builtin) {
            return Err(PluginError::NotInCatalog(format!("{}:{}", spec.format, spec.id)));
        }
        let cat = self.catalog.lock().unwrap();
        if let Some(d) = cat.get(&(spec.format.clone(), spec.id.clone())) {
            let mut d = d.clone();
            if d.path.is_empty() && !spec.path.is_empty() {
                d.path = spec.path.clone();
            }
            return Ok(d);
        }
        drop(cat);
        // Not scanned yet but the recorded path exists — use it directly.
        if !spec.path.is_empty() && Path::new(&spec.path).exists() {
            return Ok(PluginDesc {
                format,
                id: spec.id.clone(),
                name: spec.name.clone(),
                vendor: String::new(),
                version: spec.version.clone(),
                path: spec.path.clone(),
                has_editor: false,
                features: vec![],
            });
        }
        Err(PluginError::Missing {
            format: spec.format.clone(),
            id: spec.id.clone(),
            path: PathBuf::from(&spec.path),
        })
    }
}

/// Build a chain + keep-alive list for one track from restored/live slots.
///
/// `InsertSlot::Missing` contributes nothing to the chain but returns its spec
/// unchanged so the UI can draw the "missing" badge and re-save it verbatim.
pub fn build_chain(slots: Vec<InsertSlot>) -> (verdant_audio::Chain, Vec<Arc<InsertHandle>>, Vec<InsertSpec>) {
    let mut chain = Vec::new();
    let mut live = Vec::new();
    let mut missing = Vec::new();
    for slot in slots {
        match slot {
            InsertSlot::Live(h) => {
                let arc = h.clone();
                if let Some(p) = arc.processor() {
                    chain.push(p);
                }
                live.push(arc);
            }
            InsertSlot::Missing { spec, .. } => missing.push(spec),
        }
    }
    (chain, live, missing)
}
