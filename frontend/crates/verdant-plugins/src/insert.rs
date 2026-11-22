//! Control-side lifetime of one isolated host. No operation here runs in the
//! audio callback; the sole ring writer is either an RT processor or an owned
//! offline handle, never both.

use crate::bridge::{RemoteInsert, RemoteState};
use crate::error::{PluginError, Result};
use crate::plugin::{ClientEvent, InsertKey, PluginDesc};
use crate::project::InsertSpec;
use crate::proto::{self, validate_event_quanta, AutomationError, AutomationParam,
    HostMessage, HostRequest, ParamDesc, ParamEvent, ParamTarget};
use crate::shm::{Channel, Ring, Wake, STATUS_CRASHED, STATUS_OK};
use crate::telemetry::{TelemetryFrame, TelemetrySnapshot};
use serde_json::Value;
use std::io::{self, Read};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Control-side browser snapshot. Rows come exclusively from the client's host
/// catalog; filtering never manufactures entries or changes their identities.
pub struct PluginBrowser {
    catalog: Vec<PluginDesc>,
    pub query: String,
    pub format: Option<crate::PluginFormat>,
}

impl PluginBrowser {
    pub fn new(catalog: Vec<PluginDesc>) -> Self {
        Self { catalog, query: String::new(), format: None }
    }

    pub fn refresh(&mut self, catalog: Vec<PluginDesc>) { self.catalog = catalog; }

    pub fn entries(&self) -> Vec<&PluginDesc> {
        let query = self.query.trim().to_lowercase();
        self.catalog.iter().filter(|desc| {
            self.format.is_none_or(|format| desc.format == format)
                && (query.is_empty() || desc.name.to_lowercase().contains(&query)
                    || desc.vendor.to_lowercase().contains(&query)
                    || desc.id.to_lowercase().contains(&query))
        }).collect()
    }

    /// Allocate identity only when inserting, then persist that same spec through
    /// undo/save/restore. Slot positions may change without changing this UUID.
    pub fn insert(&self, specs: &mut Vec<InsertSpec>, slot: usize, format: &str, id: &str) -> Result<String> {
        if slot > specs.len() { return Err(PluginError::Host("insert slot no longer exists".into())); }
        let desc = self.catalog.iter().find(|desc| desc.format.tag() == format && desc.id == id)
            .ok_or_else(|| PluginError::NotInCatalog(format!("{format}:{id}")))?;
        let mut spec = InsertSpec::new(desc.format.tag(), &desc.id, &desc.name, &desc.path);
        spec.version = desc.version.clone();
        let instance_id = spec.instance_id.clone();
        specs.insert(slot, spec);
        Ok(instance_id)
    }
}

/// Generic-editor coercion only. Automation retains strict native admission and
/// must never silently quantize scheduled events to make them fit a descriptor.
pub fn parameter_editor_value(desc: &ParamDesc, value: f64) -> Result<f64> {
    if !desc.min.is_finite() || !desc.max.is_finite() || desc.min > desc.max
        || !(desc.max - desc.min).is_finite() || desc.step_count < 0 {
        return Err(AutomationError::InvalidDescriptor.into());
    }
    if !value.is_finite() { return Err(AutomationError::ValueOutOfRange.into()); }
    let value = value.clamp(desc.min, desc.max);
    if desc.step_count == 0 || desc.min == desc.max { return Ok(value); }
    let steps = desc.step_count as f64;
    let index = ((value - desc.min) / (desc.max - desc.min) * steps).round();
    Ok((desc.min + index / steps * (desc.max - desc.min)).clamp(desc.min, desc.max))
}

/// Reset uses the declared native default, independently of the current value.
/// A host that omits this metadata must not silently invent zero or a preset.
pub fn parameter_editor_default(desc: &ParamDesc) -> Result<f64> {
    parameter_editor_value(desc, desc.default_value
        .ok_or_else(|| PluginError::Protocol("plugin did not report a parameter default".into()))?)
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertStatus {
    Running,
    Crashed { detail: String },
}

pub enum InsertSlot {
    Live(Arc<InsertHandle>),
    Missing { spec: InsertSpec, reason: String },
}

#[derive(Default)]
struct TelemetryCache {
    requested_at: Option<Instant>,
    snapshot: Option<Arc<TelemetrySnapshot>>,
}

pub struct InsertHandle {
    key: InsertKey,
    spec: InsertSpec,
    ring: Arc<Ring>,
    state: Arc<RemoteState>,
    in_w: Wake,
    out_r: Wake,
    writer: Mutex<ChildStdin>,
    replies: Mutex<Receiver<HostMessage>>,
    // Serialize the entire request, including writing and waiting. Locking
    // just the receiver permits concurrent callers to consume each other's replies.
    request: Mutex<()>,
    pending_midi:Mutex<std::collections::VecDeque<(u32,f64)>>,
    crashed: Arc<AtomicBool>,
    detail: Arc<Mutex<String>>,
    processor_taken: AtomicBool,
    offline: bool,
    has_editor: bool,
    child: Arc<Mutex<Child>>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    watcher: Option<JoinHandle<()>>,
    next_req: AtomicU64,
    origin: Instant,
    // A watchdog also bounds a blocked pipe write, not just reply waits.
    control_deadline_ms: Arc<AtomicU64>,
    // The snapshot lock is held only for an Arc clone/swap. Refresh work has a
    // separate gate, so UI reads never wait for a pipe or spectral analysis.
    telemetry_cache: Mutex<TelemetryCache>,
    telemetry_refresh: Mutex<()>,
}

impl InsertHandle {
    pub fn spawn(
        key: InsertKey,
        spec: InsertSpec,
        desc: PluginDesc,
        plughost: &std::path::Path,
        sample_rate: f64,
        events_tx: Sender<ClientEvent>,
    ) -> Result<Self> {
        if sample_rate != crate::SAMPLE_RATE {
            return Err(PluginError::Host("plugin processing requires 48 kHz".into()));
        }
        let chan = Channel::create(crate::shm::DSP_QUANTUM_FRAMES)?;
        let mut cmd = Command::new(plughost);
        cmd.arg("host").arg(format!("--channel={}", chan.child_arg()));
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
        #[cfg(unix)]
        let staged = wire_fds(&mut cmd, chan.parent_fds)?;
        let mut child = cmd.spawn().map_err(|e| {
            PluginError::PlughostUnavailable(format!("spawn {plughost:?}: {e}"))
        })?;
        #[cfg(unix)]
        drop(staged);
        let stdout = child.stdout.take().expect("piped stdout");
        let writer = child.stdin.take().expect("piped stdin");
        if let Err(e) = prepare_stdout(&stdout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.into());
        }
        // Dropping Channel after the spawn closes the parent's copies of the
        // child's wake endpoints. They must not keep a dead peer looking alive.
        let Channel { ring, app_in_w, app_out_r, .. } = chan;
        let ring = Arc::new(ring);
        let child = Arc::new(Mutex::new(child));
        let crashed = Arc::new(AtomicBool::new(false));
        let detail = Arc::new(Mutex::new(String::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let control_deadline_ms = Arc::new(AtomicU64::new(0));
        let origin = Instant::now();
        let (rep_tx, rep_rx) = channel();
        let reader = {
            let ring = ring.clone();
            let crashed = crashed.clone();
            let detail = detail.clone();
            let events = events_tx.clone();
            let stop = stop.clone();
            std::thread::Builder::new().name(format!("ph-reader-{}", key.0)).spawn(move || {
                if let Err(e) = read_messages(stdout, &stop, &rep_tx, &events, key) {
                    if !stop.load(Ordering::Acquire) {
                        declare_dead(&ring, &crashed, &detail, &events, key, e.to_string());
                    }
                }
            })
        };
        let reader = match reader {
            Ok(h) => h,
            Err(e) => {
                let mut c = child.lock().unwrap();
                let _ = c.kill();
                let _ = c.wait();
                return Err(e.into());
            }
        };
        let watcher = {
            let child = child.clone();
            let ring = ring.clone();
            let crashed = crashed.clone();
            let detail = detail.clone();
            let events = events_tx;
            let stop = stop.clone();
            let deadline = control_deadline_ms.clone();
            std::thread::Builder::new().name(format!("ph-wait-{}", key.0)).spawn(move || {
                let mut output_seq = ring.out_seq();
                let mut pending_since = None;
                while !stop.load(Ordering::Acquire) {
                    let mut c = child.lock().unwrap();
                    // Drop may have started while this watcher waited for the
                    // child lock. A deliberately retired host is not a crash.
                    if stop.load(Ordering::Acquire) { return; }
                    let reason = match c.try_wait() {
                        Ok(Some(s)) => Some(format!("host exited: {s}")),
                        Err(e) => Some(format!("host wait failed: {e}")),
                        Ok(None) => {
                            let now = Instant::now();
                            let limit = deadline.load(Ordering::Acquire);
                            let next = ring.out_seq();
                            if next != output_seq || ring.in_seq() <= next {
                                output_seq = next;
                                pending_since = None;
                            } else if pending_since.is_none() {
                                pending_since = Some(now);
                            }
                            if crashed.load(Ordering::Acquire) {
                                Some("host disconnected".into())
                            } else if limit != 0 && now.duration_since(origin).as_millis() as u64 >= limit {
                                Some("control request timed out".into())
                            } else if pending_since.is_some_and(|t| now.duration_since(t) >= Duration::from_secs(2)) {
                                Some("audio processing stalled for 2 seconds".into())
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(reason) = reason {
                        // Teardown can signal the child while we hold this
                        // lock, so recheck after observing its exit as well.
                        if stop.load(Ordering::Acquire) { return; }
                        declare_dead(&ring, &crashed, &detail, &events, key, reason);
                        let _ = c.kill();
                        let _ = c.wait();
                        return;
                    }
                    drop(c);
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
        };
        let watcher = match watcher {
            Ok(h) => h,
            Err(e) => {
                stop.store(true, Ordering::Release);
                let mut c = child.lock().unwrap();
                let _ = c.kill();
                let _ = c.wait();
                drop(c);
                let _ = reader.join();
                return Err(e.into());
            }
        };
        let mut handle = Self {
            key, spec, ring,
            state: Arc::new(RemoteState::default()),
            in_w: app_in_w, out_r: app_out_r,
            writer: Mutex::new(writer), replies: Mutex::new(rep_rx), request: Mutex::new(()),
            pending_midi:Mutex::new(std::collections::VecDeque::with_capacity(1024)),
            crashed, detail, processor_taken: AtomicBool::new(false), offline: false,
            has_editor: false, child, stop, reader: Some(reader), watcher: Some(watcher),
            next_req: AtomicU64::new(1), origin, control_deadline_ms,
            telemetry_cache: Mutex::new(TelemetryCache::default()),
            telemetry_refresh: Mutex::new(()),
        };
        match handle.call(HostRequest::Hello { id: 0 }, Duration::from_secs(5))? {
            HostMessage::HelloAck { protocol, .. } if protocol == proto::PROTOCOL_VERSION => {}
            msg => return Err(PluginError::Protocol(format!("unexpected handshake: {msg:?}"))),
        }
        let msg = handle.call(HostRequest::Instantiate {
            id: 0, format: handle.spec.format.clone(), plugin_id: handle.spec.id.clone(),
            path: desc.path, sample_rate, max_frames: handle.ring.max_frames() as u32,
            state_b64: handle.spec.state_b64.clone(),
        }, Duration::from_secs(20))?;
        match msg {
            HostMessage::Reply { data, .. } => {
                handle.has_editor = data.get("has_editor").and_then(Value::as_bool).unwrap_or(false);
            }
            msg => return Err(PluginError::Protocol(format!("instantiate: {msg:?}"))),
        }
        handle.set_bypass(handle.spec.bypass);
        Ok(handle)
    }

    /// Queue from the UI before scheduling background delivery. Every control
    /// read/write drains earlier messages under the same request lock, so
    /// immediate Save/Export cannot overtake a controller gesture.
    pub fn enqueue_midi(&self,events:&[(u32,f64)])->Result<()>{
        if events.len()>8192||events.iter().any(|&(id,value)|!proto::valid_note_event(id,value)&&proto::decode_midi(id,value).is_none()){
            return Err(PluginError::Protocol("invalid native MIDI batch".into()));
        }
        let mut queue=self.pending_midi.lock().unwrap();
        if queue.len().saturating_add(events.len())>8192{return Err(PluginError::Protocol("native MIDI queue full".into()));}
        queue.extend(events.iter().copied());Ok(())
    }
    pub fn stop_midi(&self){
        let mut queue=self.pending_midi.lock().unwrap();queue.clear();
        for channel in 0..16{for cc in [120,123,121]{let(id,_,value)=proto::midi_control(channel,cc,0,0.);queue.push_back((id,value));}}
    }
    pub fn flush_midi(&self)->Result<()>{
        let _serial=self.request.lock().unwrap();self.flush_midi_locked()
    }
    fn flush_midi_locked(&self)->Result<()>{
        let messages:Vec<_>={let mut queue=self.pending_midi.lock().unwrap();queue.drain(..).collect()};
        for chunk in messages.chunks(1024){
            self.call_locked(HostRequest::MidiEvents{id:0,events:chunk.to_vec()},Duration::from_secs(2))?;
        }
        Ok(())
    }
    fn call(&self,req:HostRequest,timeout:Duration)->Result<HostMessage>{
        let _serial=self.request.lock().unwrap();
        self.flush_midi_locked()?;
        self.call_locked(req,timeout)
    }
    fn call_locked(&self, mut req: HostRequest, timeout: Duration) -> Result<HostMessage> {
        if self.crashed.load(Ordering::Acquire) {
            return Err(PluginError::Host(self.detail.lock().unwrap().clone()));
        }
        let id = self.next_req.fetch_add(1, Ordering::Relaxed);
        stamp_id(&mut req, id);
        let deadline = Instant::now() + timeout;
        self.control_deadline_ms.store(
            deadline.duration_since(self.origin).as_millis() as u64 + 1, Ordering::Release,
        );
        let result = (|| {
            proto::write_line(&mut *self.writer.lock().unwrap(), &req)?;
            let replies = self.replies.lock().unwrap();
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    self.kill_host();
                    return Err(PluginError::Timeout(format!("request {id}")));
                }
                match replies.recv_timeout(remaining) {
                    Ok(msg @ HostMessage::HelloAck { .. }) if matches!(req, HostRequest::Hello { .. }) => return Ok(msg),
                    Ok(HostMessage::Reply { id: rid, ok, error, data }) if rid == id => {
                        return if ok {
                            Ok(HostMessage::Reply { id: rid, ok, error, data })
                        } else {
                            Err(PluginError::Host(error.unwrap_or_else(|| "host error".into())))
                        };
                    }
                    Ok(_) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        self.kill_host();
                        return Err(PluginError::Timeout(format!("request {id}")));
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(PluginError::Host("control channel closed".into()));
                    }
                }
            }
        })();
        self.control_deadline_ms.store(0, Ordering::Release);
        result
    }

    pub(crate) fn make_offline(&mut self) -> Result<()> {
        self.call(HostRequest::SetOffline { id: 0, enabled: true }, Duration::from_secs(10))?;
        self.offline = true;
        Ok(())
    }

    /// One owner per ring. Keep this processor when retaining an audio chain.
    pub fn processor(&self) -> Option<Box<dyn verdant_audio::Processor>> {
        if self.offline { return None; }
        let in_w = self.in_w.dup().ok()?;
        let out_r = self.out_r.dup().ok()?;
        if self.processor_taken.swap(true, Ordering::AcqRel) { return None; }
        Some(Box::new(RemoteInsert::new(self.ring.clone(), in_w, out_r,
            self.state.clone(), self.spec.name.clone())))
    }

    pub fn set_bypass(&self, on: bool) { self.state.bypass.store(on, Ordering::Release); }
    pub fn bypassed(&self) -> bool { self.state.bypass.load(Ordering::Acquire) }
    pub fn plugin_latency_samples(&self) -> u32 { self.ring.plugin_latency() }
    pub fn transport_latency_samples(&self) -> u32 {
        if self.offline { 0 } else { crate::shm::TRANSPORT_LATENCY_FRAMES as u32 }
    }
    pub fn latency_samples(&self) -> u32 {
        self.plugin_latency_samples().saturating_add(self.transport_latency_samples())
    }
    pub fn xruns(&self) -> u64 { self.state.xruns.load(Ordering::Relaxed) }
    pub fn automation_faulted(&self) -> bool { self.state.automation_fault.load(Ordering::Acquire) }
    pub fn status(&self) -> InsertStatus {
        if self.crashed.load(Ordering::Acquire) || self.ring.status() != STATUS_OK {
            InsertStatus::Crashed { detail: self.detail.lock().unwrap().clone() }
        } else { InsertStatus::Running }
    }
    pub fn host_pid(&self) -> u32 { self.child.lock().unwrap().id() }
    pub fn key(&self) -> InsertKey { self.key }
    pub fn spec(&self) -> &InsertSpec { &self.spec }
    pub fn spec_mut(&mut self) -> &mut InsertSpec { &mut self.spec }

    pub fn params(&self) -> Result<Vec<ParamDesc>> {
        match self.call(HostRequest::GetParams { id: 0 }, Duration::from_secs(5))? {
            HostMessage::Reply { mut data, .. } => {
                let params = data.remove("params").ok_or_else(|| PluginError::Protocol("missing params".into()))?;
                Ok(serde_json::from_value(params)?)
            }
            msg => Err(PluginError::Protocol(format!("get_params: {msg:?}"))),
        }
    }

    /// Reset measurement history while retaining the age of the captured PCM.
    /// Control thread only: the reset uses the serialized host protocol.
    pub fn reset_meters(&self)->Result<()> {
        self.call(HostRequest::ResetMeters{id:0},Duration::from_secs(5))?;
        self.telemetry_cache.lock().unwrap().requested_at=None;Ok(())
    }
    pub fn midi_events(&self,events:&[(u32,f64)])->Result<()>{
        if events.len()>1024{return Err(PluginError::Protocol("MIDI batch too large".into()));}
        self.call(HostRequest::MidiEvents{id:0,events:events.to_vec()},Duration::from_secs(5))?;Ok(())
    }
    /// Latest measured signal, if polled. This never performs IPC, allocates a
    /// signal buffer, or waits for a background refresh. Safe for UI rendering.
    pub fn cached_telemetry(&self) -> Option<Arc<TelemetrySnapshot>> {
        self.telemetry_cache.try_lock().ok()?.snapshot.clone()
    }

    /// Background/control thread only: synchronous IPC and a bounded FFT.
    /// Poll only visible native editors. Requests are limited to 10 Hz per
    /// instance; unchanged processed-frame counts retain their original time
    /// so stopped playback cannot masquerade as a fresh measurement.
    pub fn refresh_telemetry(&self) -> Result<Arc<TelemetrySnapshot>> {
        let _refresh = self.telemetry_refresh.lock().unwrap();
        let now = Instant::now();
        {
            let mut cache = self.telemetry_cache.lock().unwrap();
            if cache.requested_at.is_some_and(|at| now.duration_since(at) < Duration::from_millis(100)) {
                if let Some(snapshot) = cache.snapshot.as_ref() { return Ok(snapshot.clone()); }
            }
            cache.requested_at = Some(now);
        }
        let frame: TelemetryFrame = match self.call(HostRequest::GetTelemetry { id: 0 }, Duration::from_secs(5))? {
            HostMessage::Reply { mut data, .. } => serde_json::from_value(data.remove("telemetry")
                .ok_or_else(|| PluginError::Protocol("missing signal telemetry".into()))?)?,
            msg => return Err(PluginError::Protocol(format!("get_telemetry: {msg:?}"))),
        };
        let mut snapshot = frame.analyze(Instant::now())
            .map_err(|error| PluginError::Protocol(error.to_string()))?;
        let mut cache = self.telemetry_cache.lock().unwrap();
        if let Some(previous) = cache.snapshot.as_ref().filter(|previous|
            previous.frames_total == snapshot.frames_total && previous.window_frames == snapshot.window_frames
                && previous.sample_rate == snapshot.sample_rate) {
            if previous.loudness == snapshot.loudness && previous.gain_reduction_db == snapshot.gain_reduction_db {
                return Ok(previous.clone());
            }
            // Meter-only control changes are visible while stopped, without
            // making the unchanged PCM appear to have just been processed.
            snapshot.captured_at = previous.captured_at;
        }
        let snapshot = Arc::new(snapshot);
        cache.snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }
    /// Query native writable/automatable metadata. Control thread only: IPC.
    pub fn automation_params(&self) -> Result<Vec<AutomationParam>> {
        match self.call(HostRequest::GetAutomationParams { id: 0 }, Duration::from_secs(5))? {
            HostMessage::Reply { mut data, .. } => {
                let params = data.remove("params").ok_or_else(|| PluginError::Protocol("missing automation params".into()))?;
                let params: Vec<AutomationParam> = serde_json::from_value(params)?;
                for param in &params { param.target.validate()?; }
                Ok(params)
            }
            msg => Err(PluginError::Protocol(format!("get_automation_params: {msg:?}"))),
        }
    }

    /// Resolve the persistent parameter id before preparing the callback chain.
    /// Missing, read-only and non-automatable parameters are explicit errors.
    pub fn compile_parameter(&self, id: &str) -> Result<ParamTarget> {
        self.automation_params()?.into_iter().find(|param| param.id == id)
            .map(|param| param.target).ok_or_else(|| AutomationError::UnknownParameter.into())
    }

    /// Immediate control-side edit. This locks/writes IPC and MUST NOT run in
    /// an audio callback; scheduled edits use `Processor::process_automated`.
    pub fn set_param(&self, param_id: &str, value: f64) -> Result<()> {
        if !value.is_finite() { return Err(PluginError::Host("parameter must be finite".into())); }
        self.call(HostRequest::SetParam { id: 0, param_id: param_id.into(), value }, Duration::from_secs(5))?;
        Ok(())
    }
    pub fn get_state_b64(&self) -> Result<String> {
        match self.call(HostRequest::GetState { id: 0 }, Duration::from_secs(10))? {
            HostMessage::Reply { data, .. } => data.get("state_b64").and_then(Value::as_str)
                .map(str::to_owned).ok_or_else(|| PluginError::Protocol("missing state_b64".into())),
            msg => Err(PluginError::Protocol(format!("get_state: {msg:?}"))),
        }
    }
    pub fn set_state_b64(&self, state_b64: &str) -> Result<()> {
        self.call(HostRequest::SetState { id: 0, state_b64: state_b64.into() }, Duration::from_secs(10))?;
        Ok(())
    }
    /// Device changes never call this: source and inserts always run at 48 kHz.
    pub fn reconfigure(&self, sample_rate: f64, max_frames: u32) -> Result<()> {
        if sample_rate != crate::SAMPLE_RATE || max_frames != self.ring.max_frames() as u32 {
            return Err(PluginError::Host("live insert rate/block quantum is fixed".into()));
        }
        self.call(HostRequest::Reconfigure { id: 0, sample_rate, max_frames }, Duration::from_secs(10))?;
        Ok(())
    }
    pub fn open_editor(&self, title: &str) -> Result<()> {
        self.call(HostRequest::OpenEditor { id: 0, title: title.into() }, Duration::from_secs(10))?;
        Ok(())
    }
    pub fn close_editor(&self) -> Result<()> {
        self.call(HostRequest::CloseEditor { id: 0 }, Duration::from_secs(5))?;
        Ok(())
    }
    pub fn has_editor(&self) -> bool { self.has_editor }

    /// Export-only synchronous processing, without the RT pipeline delay.
    /// Errors abort export; a dead/stalled plugin never becomes dry success.
    pub fn process_offline(&mut self, l: &mut [f32], r: &mut [f32]) -> Result<()> {
        self.process_offline_events(l, r, &[])
    }

    /// Export uses exactly the same native event units and quantum offsets as
    /// playback. All event offsets/densities are checked before the first write.
    /// This owned offline path may wait; the live Processor never does.
    pub fn process_offline_events(&mut self, l: &mut [f32], r: &mut [f32], events: &[ParamEvent]) -> Result<()> {
        self.process_offline_keyed(l, r, None, events)
    }

    /// As `process_offline_events`, with an aligned sidechain key.
    pub fn process_offline_keyed(&mut self, l: &mut [f32], r: &mut [f32], key: Option<(&[f32], &[f32])>,
        events: &[ParamEvent]) -> Result<()> {
        if key.is_some_and(|(kl, kr)| kl.len() != l.len() || kr.len() != l.len()) {
            return Err(PluginError::Host("offline sidechain length differs".into()));
        }
        if !self.offline || self.processor_taken.load(Ordering::Acquire) {
            return Err(PluginError::Host("offline processing requires an export-owned host".into()));
        }
        if l.len() != r.len() {
            return Err(PluginError::Host("offline stereo lengths differ".into()));
        }
        let max = self.ring.max_frames();
        validate_event_quanta(events, l.len(), max, 0, 0)?;
        if self.bypassed() { return Ok(()); }
        let mut seq = self.ring.in_seq();
        let mut offset = 0;
        let mut next_event = 0;
        for (ls, rs) in l.chunks_mut(max).zip(r.chunks_mut(max)) {
            let first = next_event;
            while next_event < events.len() && (events[next_event].1 as usize) < offset + ls.len() {
                next_event += 1;
            }
            let chunk = key.map(|(kl, kr)| (&kl[offset..offset + ls.len()], &kr[offset..offset + ls.len()]));
            if self.ring.status() != STATUS_OK
                || !self.ring.try_write_input_keyed(seq, ls, rs, chunk, &events[first..next_event], offset as u32) {
                return Err(AutomationError::TransportUnavailable.into());
            }
            self.in_w.signal();
            let ready = self.out_r.wait_until(Instant::now() + Duration::from_secs(30), || {
                self.ring.out_seq() > seq || self.ring.status() != STATUS_OK
            });
            if !ready || self.ring.status() != STATUS_OK || !self.ring.read_output(seq, ls, rs)? {
                self.kill_host();
                return Err(PluginError::Host("offline output lost, host failed or timed out".into()));
            }
            seq += 1;
            offset += ls.len();
        }
        Ok(())
    }

    pub fn kill_host(&self) { let _ = self.child.lock().unwrap().kill(); }
}

fn stamp_id(req: &mut HostRequest, id: u64) {
    match req {
        HostRequest::Hello { id: r } | HostRequest::Instantiate { id: r, .. }
        | HostRequest::Reconfigure { id: r, .. } | HostRequest::SetParam { id: r, .. }
        | HostRequest::SetOffline { id: r, .. }
        | HostRequest::GetParams { id: r } | HostRequest::GetAutomationParams { id: r }
        | HostRequest::GetTelemetry { id: r } | HostRequest::ResetMeters { id:r } | HostRequest::MidiEvents { id:r,.. }
        | HostRequest::GetState { id: r }
        | HostRequest::SetState { id: r, .. } | HostRequest::OpenEditor { id: r, .. }
        | HostRequest::CloseEditor { id: r } | HostRequest::Shutdown { id: r } => *r = id,
    }
}

fn declare_dead(ring: &Ring, crashed: &AtomicBool, detail: &Mutex<String>,
    events: &Sender<ClientEvent>, key: InsertKey, reason: String) {
    if crashed.swap(true, Ordering::AcqRel) { return; }
    *detail.lock().unwrap() = reason.clone();
    ring.set_status(STATUS_CRASHED);
    let _ = events.send(ClientEvent::Crashed { insert: key, detail: reason });
}

impl Drop for InsertHandle {
    fn drop(&mut self) {
        // Publish retirement before requesting shutdown or making stdout
        // reach EOF. Both observers must stop reporting faults before the
        // deliberate exit, including a watcher already waiting on child.
        self.stop.store(true, Ordering::Release);
        self.ring.request_shutdown();
        self.ring.set_status(STATUS_CRASHED);
        self.in_w.signal();
        // Do not write to a stalled control pipe during teardown.
        {
            let mut c = self.child.lock().unwrap();
            let deadline = Instant::now() + Duration::from_millis(250);
            // wait-timeout's Unix SIGCHLD singleton permanently retains two
            // socket FDs. Poll only this owned child, without global resources.
            loop {
                match c.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        if !remaining.is_zero() {
                            std::thread::sleep(remaining.min(Duration::from_millis(5)));
                            continue;
                        }
                    }
                    Err(_) => {}
                }
                let _ = c.kill();
                let _ = c.wait();
                break;
            }
        }
        for h in [self.watcher.take(), self.reader.take()].into_iter().flatten() {
            let _ = h.join();
        }
    }
}

fn read_messages(mut stdout: ChildStdout, stop: &AtomicBool, replies: &Sender<HostMessage>,
    events: &Sender<ClientEvent>, key: InsertKey) -> io::Result<()> {
    let mut pending = Vec::new();
    let mut bytes = [0u8; 4096];
    while !stop.load(Ordering::Acquire) {
        if !stdout_ready(&stdout)? {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        match stdout.read(&mut bytes) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "host stdout closed")),
            Ok(n) => {
                for &b in &bytes[..n] {
                    if b == b'\n' {
                        let msg: HostMessage = serde_json::from_slice(&pending)?;
                        pending.clear();
                        match msg {
                            HostMessage::Event { event, data } => {
                                let _ = events.send(ClientEvent::Event { insert: key, event, data });
                            }
                            msg => { let _ = replies.send(msg); }
                        }
                    } else {
                        if pending.len() >= proto::MAX_LINE {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, "host line exceeds limit"));
                        }
                        pending.push(b);
                    }
                }
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn prepare_stdout(stdout: &ChildStdout) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let fd = stdout.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
#[cfg(windows)]
fn prepare_stdout(_: &ChildStdout) -> io::Result<()> { Ok(()) }
#[cfg(unix)]
fn stdout_ready(_: &ChildStdout) -> io::Result<bool> { Ok(true) }
#[cfg(windows)]
fn stdout_ready(stdout: &ChildStdout) -> io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    let mut available = 0;
    unsafe {
        if windows_sys::Win32::System::Pipes::PeekNamedPipe(stdout.as_raw_handle(),
            std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut available, std::ptr::null_mut()) == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(available != 0)
}

/// Owned staging FDs close in the parent on both spawn success and failure.
#[cfg(unix)]
fn wire_fds(cmd: &mut Command, fds: (libc::c_int, libc::c_int, libc::c_int)) -> Result<Vec<std::os::fd::OwnedFd>> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    let mut staged = Vec::<OwnedFd>::new();
    for fd in [fds.0, fds.1, fds.2] {
        let dup = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 64) };
        if dup < 0 { return Err(io::Error::last_os_error().into()); }
        staged.push(unsafe { OwnedFd::from_raw_fd(dup) });
    }
    let raw = [staged[0].as_raw_fd(), staged[1].as_raw_fd(), staged[2].as_raw_fd()];
    unsafe {
        cmd.pre_exec(move || {
            for (src, dst) in raw.into_iter().zip([9, 10, 11]) {
                if libc::dup2(src, dst) < 0 { return Err(io::Error::last_os_error()); }
            }
            Ok(())
        });
    }
    Ok(staged)
}
