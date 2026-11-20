//! Output enumeration, selection and recovery. One non-real-time worker owns
//! every device/stream and joins each old silent clock before starting another.
//! The callback exclusively owns its renderer; teardown returns that same
//! renderer through a one-element SPSC ring. No callback locks shared state,
//! creates/drops a device, or destroys a timeline/insert chain.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::callback::{CallbackBufferError, CallbackRenderer};
use crate::convert::DeviceSample;
use crate::engine::{Control, Shared, Transport, channel};

pub use crate::{DeviceError, DeviceErrorKind, MAX_BLOCK};
const ENGINE_RATE: u32 = 48_000;
const POLL: Duration = Duration::from_millis(20);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAYS: [Duration; 3] = [Duration::from_millis(250), Duration::from_millis(750), Duration::from_millis(1500)];
const MAX_EVENTS: usize = 64;

/// Numeric PCM formats supported by the output adapter. DSD is a different
/// signal representation and is never advertised as playable PCM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SampleFmt {
    I8, I16, I24, I32, I64, U8, U16, U24, U32, U64, F32, F64,
}

impl SampleFmt {
    pub fn from_cpal(format: cpal::SampleFormat) -> Option<Self> {
        Some(match format {
            cpal::SampleFormat::I8 => Self::I8,
            cpal::SampleFormat::I16 => Self::I16,
            cpal::SampleFormat::I24 => Self::I24,
            cpal::SampleFormat::I32 => Self::I32,
            cpal::SampleFormat::I64 => Self::I64,
            cpal::SampleFormat::U8 => Self::U8,
            cpal::SampleFormat::U16 => Self::U16,
            cpal::SampleFormat::U24 => Self::U24,
            cpal::SampleFormat::U32 => Self::U32,
            cpal::SampleFormat::U64 => Self::U64,
            cpal::SampleFormat::F32 => Self::F32,
            cpal::SampleFormat::F64 => Self::F64,
            _ => return None,
        })
    }
}

impl std::fmt::Display for SampleFmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioDeviceInfo {
    /// Stable cpal DeviceId string, if the backend supplies one.
    pub id: Option<String>,
    /// Exactly the device.description() text, rather than an invented label.
    pub name: String,
    pub is_default: bool,
    /// Advertised endpoints and standard rates within supported ranges.
    /// These are descriptive; the host negotiates the actual rate at open.
    pub sample_rates: Vec<u32>,
    pub channels: Vec<u16>,
    pub formats: Vec<SampleFmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceSelection {
    Default,
    /// A stable DeviceId string, or a description for devices without an id.
    Device(String),
    Silent,
}

impl From<bool> for DeviceSelection {
    fn from(prefer_silent: bool) -> Self {
        if prefer_silent { Self::Silent } else { Self::Default }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutputKind {
    Device { name: String, channels: u16, format: SampleFmt },
    /// Real-time transport clock; explicitly silent in every UI.
    SilentClock { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputInfo {
    pub kind: OutputKind,
    /// Fixed timeline/mixer/insert rate, also used by seek and position.
    /// Device changes never reinterpret existing PCM or transport frames.
    pub sample_rate: u32,
    /// Negotiated physical output rate, adapted at the callback boundary.
    pub device_sample_rate: u32,
    pub device_id: Option<String>,
    /// Fixed physical buffer size when reported, otherwise None.
    pub buffer_frames: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceStatus {
    Starting,
    Running(OutputInfo),
    Recovering { device: String, attempt: u32 },
    Failed(DeviceError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceEvent {
    OutputChanged(OutputInfo),
    DeviceLost { device: String, error: String },
    Recovered(OutputInfo),
    RecoveryFailed { error: DeviceError },
    Rejected(DeviceError),
    CallbackError { message: String },
    /// The backend already rerouted the active default stream.
    RouteChanged,
    /// Audio remains active without OS realtime scheduling promotion.
    RealtimeDenied,
}

/// Enumerate the actual default host's outputs. Hosts/streams stay worker-local
/// when called through OutputEngine::devices().
pub fn list_devices() -> Result<Vec<AudioDeviceInfo>, DeviceError> {
    CpalBackend.devices()
}

struct Snapshot {
    info: OutputInfo,
    status: DeviceStatus,
    events: VecDeque<DeviceEvent>,
}

impl Snapshot {
    fn event(&mut self, event: DeviceEvent) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

enum Command {
    Select(DeviceSelection),
    Enumerate(mpsc::Sender<Result<Vec<AudioDeviceInfo>, DeviceError>>),
    Shutdown,
}

/// Keeps every output worker and clock within a clear lifetime. Drop or
/// shutdown joins all workers; selecting does not replace Control/RtState.
pub struct OutputEngine {
    command: mpsc::Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    worker: Option<JoinHandle<()>>,
}

impl OutputEngine {
    /// bool is accepted for --silent-audio compatibility; true selects Silent.
    /// Failed device opens are reported and explicitly use a silent clock.
    pub fn open(selection: impl Into<DeviceSelection>) -> (Self, Control) {
        Self::open_backend(selection.into(), ENGINE_RATE, "silent clock selected".into(), Box::new(CpalBackend), RETRY_DELAYS)
    }

    pub fn open_device(name: Option<&str>) -> (Self, Control) {
        Self::open(match name {
            Some(name) if !name.is_empty() => DeviceSelection::Device(name.into()),
            _ => DeviceSelection::Default,
        })
    }

    fn open_backend(
        selection: DeviceSelection,
        rate: u32,
        silent_reason: String,
        backend: Box<dyn Backend>,
        retries: [Duration; 3],
    ) -> (Self, Control) {
        let rate = rate.max(1);
        let (control, rt) = channel(rate, MAX_BLOCK);
        let (command, receiver) = mpsc::channel();
        let (opened, initial) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot {
            info: silent_info(rate, "output initialization in progress".into()),
            status: DeviceStatus::Starting,
            events: VecDeque::new(),
        }));
        let shared = control.shared.clone();
        let worker_snapshot = snapshot.clone();
        let spawned = std::thread::Builder::new().name("verdant-audio-device".into()).spawn(move || {
            let mut worker = Worker {
                backend,
                receiver,
                snapshot: worker_snapshot,
                shared,
                renderer: Some(Box::new(CallbackRenderer::new(rt, rate))),
                active: None,
                selection: selection.clone(),
                recovery: None,
                retries,
                silent_reason,
            };
            worker.select(selection);
            let _ = opened.send(());
            worker.run();
        });
        let worker = match spawned {
            Ok(join) => {
                // A timeout keeps Starting observable; it never claims a
                // clock exists merely because a host open has not returned.
                let _ = initial.recv_timeout(REQUEST_TIMEOUT);
                Some(join)
            }
            Err(error) => {
                let mut s = snapshot.lock().unwrap_or_else(|p| p.into_inner());
                let error = DeviceError::new(DeviceErrorKind::Unavailable, format!("cannot start audio worker: {error}"));
                s.info = silent_info(rate, error.message.clone());
                s.status = DeviceStatus::Failed(error.clone());
                s.event(DeviceEvent::RecoveryFailed { error });
                None
            }
        };
        (OutputEngine { command, snapshot, worker }, control)
    }

    pub fn devices(&self) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
        let (answer, receiver) = mpsc::channel();
        self.command.send(Command::Enumerate(answer))
            .map_err(|_| DeviceError::new(DeviceErrorKind::Unavailable, "audio worker has stopped"))?;
        receiver.recv_timeout(REQUEST_TIMEOUT)
            .map_err(|e| DeviceError::new(DeviceErrorKind::Enumerate, e.to_string()))?
    }

    /// Asynchronous hot reselect. Every result is observable through status
    /// and events; a rejected preparation leaves the current driver running.
    pub fn select(&self, selection: DeviceSelection) {
        if self.command.send(Command::Select(selection)).is_err() {
            let mut s = self.snapshot.lock().unwrap_or_else(|p| p.into_inner());
            let error = DeviceError::new(DeviceErrorKind::Unavailable, "audio worker has stopped");
            s.status = DeviceStatus::Failed(error.clone());
            s.event(DeviceEvent::Rejected(error));
        }
    }

    /// Last bound output; retained during recovery. device_status() tells the
    /// UI whether a physical stream is currently running.
    pub fn output_info(&self) -> OutputInfo {
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).info.clone()
    }

    pub fn device_status(&self) -> DeviceStatus {
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).status.clone()
    }

    pub fn drain_events(&self) -> Vec<DeviceEvent> {
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).events.drain(..).collect()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.command.send(Command::Shutdown);
            let _ = worker.join();
        }
    }
}

impl Drop for OutputEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

// Worker-only driver abstractions. In particular Driver has no Send bound:
// cpal streams are not Send on every platform and never leave their worker.
trait Driver {
    fn play(&mut self) -> Result<(), DeviceError>;
}

struct PreparedOutput {
    info: OutputInfo,
    build: Box<dyn FnOnce(CallbackLease, Arc<FaultSignal>) -> Result<Box<dyn Driver>, DeviceError>>,
}

trait Backend: Send {
    fn devices(&self) -> Result<Vec<AudioDeviceInfo>, DeviceError>;
    fn prepare(&self, selection: &DeviceSelection, rate: u32) -> Result<PreparedOutput, DeviceError>;
}

/// At most one callback lease exists at a time. Its return ring starts empty
/// and receives exactly one item on destruction, including failed builds.
/// The worker retains the consumer until after stream teardown, so neither
/// renderer nor return-ring storage is destroyed on a callback thread.
struct CallbackLease {
    renderer: Option<Box<CallbackRenderer>>,
    returned: rtrb::Producer<Box<CallbackRenderer>>,
    fault: Arc<FaultSignal>,
}

impl CallbackLease {
    fn f32(&mut self, out: &mut [f32], channels: usize) {
        if let Some(renderer) = &mut self.renderer {
            if let Err(error) = renderer.process_f32(out, channels) { self.fault.buffer_error(error); }
        }
    }

    fn numeric<T: DeviceSample>(&mut self, out: &mut [T], channels: usize) {
        if let Some(renderer) = &mut self.renderer {
            if let Err(error) = renderer.process(out, channels) { self.fault.buffer_error(error); }
        }
    }
}

impl Drop for CallbackLease {
    fn drop(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            // There is one producer, one push, and an empty capacity-one ring.
            assert!(self.returned.push(renderer).is_ok(), "callback ownership return ring must be empty");
        }
    }
}

#[derive(Default)]
struct FaultSignal {
    fatal: AtomicU8,
    oversized_frames: AtomicU64,
    xruns: AtomicU64,
    notices: AtomicU8,
}

impl FaultSignal {
    /// Error callbacks only publish a code; formatting and recovery happen on
    /// the worker. Each stream gets a fresh signal, excluding stale errors.
    fn report(&self, error: cpal::Error) {
        let code = match error.kind() {
            cpal::ErrorKind::Xrun => {
                self.xruns.fetch_add(1, Ordering::Relaxed);
                return;
            }
            cpal::ErrorKind::DeviceChanged => {
                self.notices.fetch_or(1, Ordering::Release);
                return;
            }
            cpal::ErrorKind::RealtimeDenied => {
                self.notices.fetch_or(2, Ordering::Release);
                return;
            }
            cpal::ErrorKind::DeviceNotAvailable => 1,
            cpal::ErrorKind::StreamInvalidated => 3,
            cpal::ErrorKind::HostUnavailable => 4,
            cpal::ErrorKind::PermissionDenied => 5,
            _ => 6,
        };
        self.fatal.fetch_max(code, Ordering::Release);
    }

    fn buffer_error(&self, error: CallbackBufferError) {
        self.oversized_frames.store(error.frames as u64, Ordering::Relaxed);
        self.fatal.store(7, Ordering::Release);
    }

    fn failure(&self) -> Option<DeviceError> {
        let message = match self.fatal.swap(0, Ordering::AcqRel) {
            0 => return None,
            7 => return Some(DeviceError::new(DeviceErrorKind::UnsupportedConfig,
                format!("physical callback has {} frames; the admitted maximum is {MAX_BLOCK}", self.oversized_frames.load(Ordering::Relaxed)))),
            1 => "output device is no longer available",
            3 => "output stream was invalidated",
            4 => "audio host is unavailable",
            5 => "permission to use the output device was denied",
            _ => "audio backend reported a stream failure",
        };
        Some(DeviceError::new(DeviceErrorKind::Lost, message))
    }
}

struct ActiveOutput {
    driver: Box<dyn Driver>,
    returned: rtrb::Consumer<Box<CallbackRenderer>>,
    fault: Arc<FaultSignal>,
}

struct Recovery {
    attempt: usize,
    due: Instant,
    device: String,
}

struct Worker {
    backend: Box<dyn Backend>,
    receiver: mpsc::Receiver<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    shared: Arc<Shared>,
    renderer: Option<Box<CallbackRenderer>>,
    active: Option<ActiveOutput>,
    selection: DeviceSelection,
    recovery: Option<Recovery>,
    retries: [Duration; 3],
    silent_reason: String,
}

impl Worker {
    fn run(&mut self) {
        loop {
            match self.receiver.recv_timeout(POLL) {
                Ok(Command::Select(selection)) => self.select(selection),
                Ok(Command::Enumerate(answer)) => { let _ = answer.send(self.backend.devices()); }
                Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            self.poll_fault();
            self.recover_if_due();
        }
        self.retire();
    }

    fn rate(&self) -> u32 {
        self.shared.sample_rate.load(Ordering::Acquire) as u32
    }

    fn prepare(&self, selection: &DeviceSelection) -> Result<PreparedOutput, DeviceError> {
        let prepared = match selection {
            DeviceSelection::Silent => Ok(prepare_silent(self.rate(), self.silent_reason.clone())),
            _ => self.backend.prepare(selection, self.rate()),
        }?;
        if let OutputKind::Device { channels, .. } = &prepared.info.kind {
            CallbackRenderer::validate_device_rate(prepared.info.device_sample_rate)?;
            if *channels == 0 || prepared.info.buffer_frames.is_some_and(|frames| frames == 0 || frames > MAX_BLOCK as u32) {
                return Err(DeviceError::new(DeviceErrorKind::UnsupportedConfig,
                    format!("device needs nonzero channels and a callback of at most {MAX_BLOCK} frames")));
            }
        }
        Ok(prepared)
    }

    fn select(&mut self, selection: DeviceSelection) {
        // Resolving config first allows invalid explicit choices to leave the
        // currently audible stream and its clock completely untouched.
        match self.prepare(&selection) {
            Err(error) => {
                self.event(DeviceEvent::Rejected(error.clone()));
                if self.active.is_none() {
                    self.recovery = None;
                    self.fallback(error);
                }
            }
            Ok(prepared) => {
                self.recovery = None;
                self.selection = selection;
                match self.activate(prepared) {
                    Ok(info) => self.event(DeviceEvent::OutputChanged(info)),
                    Err(error) => {
                        self.event(DeviceEvent::Rejected(error.clone()));
                        self.fallback(error);
                    }
                }
            }
        }
    }

    fn retire(&mut self) {
        if let Some(active) = self.active.take() {
            // Stream destruction (and silent-clock join) runs on this worker.
            drop(active.driver);
            self.renderer = Some(receive_renderer(active.returned));
        }
    }

    fn activate(&mut self, prepared: PreparedOutput) -> Result<OutputInfo, DeviceError> {
        self.retire();
        let mut renderer = self.renderer.take().expect("one renderer belongs to the worker or its single active output");
        if matches!(&prepared.info.kind, OutputKind::SilentClock { .. }) {
            renderer.use_source_clock();
        } else if let Err(error) = renderer.set_device_rate(prepared.info.device_sample_rate) {
            self.renderer = Some(renderer);
            return Err(error);
        }
        let (returned, receiver) = rtrb::RingBuffer::new(1);
        let fault = Arc::new(FaultSignal::default());
        let lease = CallbackLease { renderer: Some(renderer), returned, fault: fault.clone() };
        let result = (prepared.build)(lease, fault.clone()).and_then(|mut driver| {
            // Error state belongs to output status, not transport intent.
            self.restore_transport();
            driver.play()?;
            Ok(driver)
        });
        match result {
            Ok(driver) => {
                let info = prepared.info;
                self.active = Some(ActiveOutput { driver, returned: receiver, fault });
                let mut s = self.snapshot.lock().unwrap_or_else(|p| p.into_inner());
                s.info = info.clone();
                s.status = DeviceStatus::Running(info.clone());
                Ok(info)
            }
            Err(error) => {
                // Build/play failure dropped the lease along with the failed
                // stream, returning the same mixer state for fallback/retry.
                self.renderer = Some(receive_renderer(receiver));
                Err(error)
            }
        }
    }

    fn restore_transport(&self) {
        let state = if self.shared.playback_requested() {
            Transport::Playing
        } else if self.shared.position.load(Ordering::Acquire) == 0 {
            Transport::Stopped
        } else {
            Transport::Paused
        };
        self.shared.state.store(state as u8, Ordering::Release);
    }

    fn event(&self, event: DeviceEvent) {
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).event(event);
    }

    fn fallback(&mut self, error: DeviceError) {
        match self.activate(prepare_silent(self.rate(), format!("audio device unavailable: {}", error.message))) {
            Ok(info) => self.event(DeviceEvent::OutputChanged(info)),
            Err(error) => {
                self.shared.state.store(Transport::DeviceError as u8, Ordering::Release);
                let mut s = self.snapshot.lock().unwrap_or_else(|p| p.into_inner());
                s.status = DeviceStatus::Failed(error.clone());
                s.event(DeviceEvent::RecoveryFailed { error });
            }
        }
    }

    fn poll_fault(&mut self) {
        let Some(active) = &self.active else { return };
        let notices = active.fault.notices.swap(0, Ordering::AcqRel);
        if notices & 1 != 0 {
            self.event(DeviceEvent::RouteChanged);
        }
        if notices & 2 != 0 {
            self.event(DeviceEvent::RealtimeDenied);
        }
        let xruns = active.fault.xruns.swap(0, Ordering::AcqRel);
        if xruns != 0 {
            self.shared.underruns.fetch_add(xruns, Ordering::Relaxed);
            self.event(DeviceEvent::CallbackError { message: format!("audio host reported {xruns} xrun(s)") });
        }
        let Some(error) = active.fault.failure() else { return };
        if error.kind == DeviceErrorKind::UnsupportedConfig {
            // A device that exceeds the admitted whole-callback bound cannot
            // be repaired by retrying that same stream or calling it an xrun.
            self.recovery = None;
            self.event(DeviceEvent::Rejected(error.clone()));
            self.fallback(error);
            return;
        }
        let device = match &self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).info.kind {
            OutputKind::Device { name, .. } => name.clone(),
            OutputKind::SilentClock { .. } => "silent clock".into(),
        };
        self.retire();
        self.shared.state.store(Transport::DeviceError as u8, Ordering::Release);
        self.event(DeviceEvent::DeviceLost { device: device.clone(), error: error.message });
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).status = DeviceStatus::Recovering { device: device.clone(), attempt: 1 };
        self.recovery = Some(Recovery { attempt: 0, due: Instant::now() + self.retries[0], device });
    }

    fn recover_if_due(&mut self) {
        let Some(recovery) = self.recovery.take() else { return };
        if Instant::now() < recovery.due {
            self.recovery = Some(recovery);
            return;
        }
        let attempt = recovery.attempt;
        self.snapshot.lock().unwrap_or_else(|p| p.into_inner()).status = DeviceStatus::Recovering {
            device: recovery.device.clone(), attempt: attempt as u32 + 1,
        };
        match self.prepare(&self.selection).and_then(|prepared| self.activate(prepared)) {
            Ok(info) => {
                self.event(DeviceEvent::Recovered(info.clone()));
                self.event(DeviceEvent::OutputChanged(info));
            }
            Err(error) if attempt + 1 < self.retries.len() => {
                self.recovery = Some(Recovery {
                    attempt: attempt + 1,
                    due: Instant::now() + self.retries[attempt + 1],
                    device: recovery.device,
                });
                let _ = error; // final failed attempt supplies RecoveryFailed
            }
            Err(error) => {
                self.event(DeviceEvent::RecoveryFailed { error: error.clone() });
                self.fallback(error);
            }
        }
    }
}

/// Called only after driver teardown. Native backends normally return the
/// lease during Stream::drop; asynchronously released callback closures may
/// finish later. Waiting here never runs on the audio callback.
fn receive_renderer(mut receiver: rtrb::Consumer<Box<CallbackRenderer>>) -> Box<CallbackRenderer> {
    loop {
        if let Ok(renderer) = receiver.pop() {
            return renderer;
        }
        if receiver.is_abandoned() {
            // The producer can push and drop between the first pop and this
            // observation; its final push must be observed before failing.
            return receiver.pop().expect("a driver must return its callback renderer on teardown");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn silent_info(rate: u32, reason: String) -> OutputInfo {
    OutputInfo { kind: OutputKind::SilentClock { reason }, sample_rate: rate, device_sample_rate: rate, device_id: None, buffer_frames: None }
}

fn prepare_silent(rate: u32, reason: String) -> PreparedOutput {
    PreparedOutput {
        info: silent_info(rate, reason),
        build: Box::new(move |lease, _| {
            // Spawn paused: the worker starts it after retiring the old driver.
            let (start, ready) = mpsc::channel();
            let (stop, stopped) = mpsc::channel();
            let join = std::thread::Builder::new().name("verdant-silent-clock".into()).spawn(move || {
                let mut lease = lease;
                if ready.recv().is_err() { return; }
                let block = (rate as usize / 100).clamp(1, MAX_BLOCK);
                let mut buffer = vec![0.0; block * 2];
                let period = Duration::from_secs_f64(block as f64 / rate as f64);
                let mut due = Instant::now();
                loop {
                    match stopped.try_recv() {
                        Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                    lease.f32(&mut buffer, 2);
                    due += period;
                    // Never catch up a paused/stalled clock by firing a burst
                    // of callbacks: re-anchor its wall-clock deadline instead.
                    if due < Instant::now() { due = Instant::now() + period; }
                    match stopped.recv_timeout(due.saturating_duration_since(Instant::now())) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
            }).map_err(|e| DeviceError::new(DeviceErrorKind::Unavailable, e.to_string()))?;
            Ok(Box::new(SilentDriver { start: Some(start), stop, join: Some(join) }))
        }),
    }
}

struct SilentDriver {
    start: Option<mpsc::Sender<()>>,
    stop: mpsc::Sender<()>,
    join: Option<JoinHandle<()>>,
}

impl Driver for SilentDriver {
    fn play(&mut self) -> Result<(), DeviceError> {
        if let Some(start) = self.start.take() {
            start.send(()).map_err(|_| DeviceError::new(DeviceErrorKind::Play, "silent clock failed to start"))?;
        }
        Ok(())
    }
}

impl Drop for SilentDriver {
    fn drop(&mut self) {
        // Dropping a not-yet-played driver must also release ready.recv().
        self.start.take();
        let _ = self.stop.send(());
        if let Some(join) = self.join.take() { let _ = join.join(); }
    }
}

struct CpalBackend;

impl Backend for CpalBackend {
    fn devices(&self) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
        let host = cpal::default_host();
        let default = host.default_output_device().and_then(|d| d.id().ok());
        let devices = host.output_devices().map_err(|e| DeviceError::new(DeviceErrorKind::Enumerate, e.to_string()))?;
        let mut result = Vec::new();
        for device in devices {
            let id = device.id().ok();
            let name = device.description().map_err(|e| DeviceError::new(DeviceErrorKind::Enumerate, e.to_string()))?.to_string();
            let mut sample_rates = Vec::new();
            let mut channels = Vec::new();
            let mut formats = Vec::new();
            let supported = device.supported_output_configs().map_err(|e| DeviceError::new(DeviceErrorKind::Enumerate, format!("{name}: {e}")))?;
            for config in supported {
                if let Some(format) = SampleFmt::from_cpal(config.sample_format()) {
                    let (min, max) = (config.min_sample_rate(), config.max_sample_rate());
                    sample_rates.extend([min, max]);
                    sample_rates.extend([8_000, 16_000, 22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 192_000]
                        .into_iter().filter(|rate| *rate >= min && *rate <= max));
                    channels.push(config.channels());
                    formats.push(format);
                }
            }
            sample_rates.sort_unstable(); sample_rates.dedup();
            channels.sort_unstable(); channels.dedup();
            formats.sort_unstable(); formats.dedup();
            result.push(AudioDeviceInfo { is_default: id.is_some() && id == default, id: id.map(|id| id.to_string()), name, sample_rates, channels, formats });
        }
        Ok(result)
    }

    fn prepare(&self, selection: &DeviceSelection, rate: u32) -> Result<PreparedOutput, DeviceError> {
        let host = cpal::default_host();
        let device = resolve_device(&host, selection)?;
        let name = device.description().map_err(|e| DeviceError::new(DeviceErrorKind::Open, e.to_string()))?.to_string();
        let device_id = device.id().ok().map(|id| id.to_string());
        let mut supported = device.default_output_config().map_err(|e| DeviceError::new(DeviceErrorKind::Open, e.to_string()))?;
        if SampleFmt::from_cpal(supported.sample_format()).is_none()
            || CallbackRenderer::validate_device_rate(supported.sample_rate()).is_err() {
            // A DSD default does not rule out this device's separate PCM modes.
            supported = device.supported_output_configs().map_err(|e| DeviceError::new(DeviceErrorKind::Open, e.to_string()))?
                .find(|config| SampleFmt::from_cpal(config.sample_format()).is_some()
                    && config.max_sample_rate() >= crate::callback::MIN_DEVICE_RATE)
                .map(|config| config.with_sample_rate(rate.clamp(config.min_sample_rate().max(crate::callback::MIN_DEVICE_RATE), config.max_sample_rate())))
                .ok_or_else(|| DeviceError::new(DeviceErrorKind::UnsupportedConfig, "device has no numeric PCM output at 44,100 Hz or above"))?;
        }
        let format = SampleFmt::from_cpal(supported.sample_format()).expect("PCM config selected");
        let buffer = match supported.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                let minimum = (*min).max(1);
                let maximum = (*max).min(MAX_BLOCK as u32);
                if minimum > maximum {
                    return Err(DeviceError::new(DeviceErrorKind::UnsupportedConfig,
                        format!("device buffer range {min}..{max} exceeds the admitted maximum {MAX_BLOCK}")));
                }
                // Prepared-PCM playback does not need a live-input-sized period.
                // Give the host scheduler room while respecting device limits
                // and the renderer's fixed preallocated callback capacity.
                cpal::BufferSize::Fixed(1024u32.clamp(minimum, maximum))
            }
            cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
        };
        let mut config: cpal::StreamConfig = supported.into();
        config.buffer_size = buffer;
        if config.channels == 0 || config.sample_rate == 0 {
            return Err(DeviceError::new(DeviceErrorKind::UnsupportedConfig, "device returned zero channels or sample rate"));
        }
        let info = OutputInfo {
            kind: OutputKind::Device { name, channels: config.channels, format },
            sample_rate: rate,
            device_sample_rate: config.sample_rate,
            device_id,
            buffer_frames: match config.buffer_size { cpal::BufferSize::Fixed(frames) => Some(frames), cpal::BufferSize::Default => None },
        };
        Ok(PreparedOutput {
            info,
            build: Box::new(move |lease, fault| {
                let stream = build_stream(&device, config, format, lease, fault)?;
                Ok(Box::new(CpalDriver(stream)))
            }),
        })
    }
}

fn resolve_device(host: &cpal::Host, selection: &DeviceSelection) -> Result<cpal::Device, DeviceError> {
    match selection {
        DeviceSelection::Default => host.default_output_device()
            .ok_or_else(|| DeviceError::new(DeviceErrorKind::NoDevice, "no default output device")),
        DeviceSelection::Device(want) => {
            if let Ok(id) = want.parse::<cpal::DeviceId>() {
                if let Some(device) = host.device_by_id(&id) { return Ok(device); }
            }
            host.output_devices().map_err(|e| DeviceError::new(DeviceErrorKind::Enumerate, e.to_string()))?
                .find(|device| device.id().is_ok_and(|id| id.to_string() == *want)
                    || device.description().is_ok_and(|description| description.to_string() == *want))
                .ok_or_else(|| DeviceError::new(DeviceErrorKind::DeviceNotFound, format!("output device {want:?} is not available")))
        }
        DeviceSelection::Silent => Err(DeviceError::new(DeviceErrorKind::UnsupportedConfig, "silent clock does not use a hardware device")),
    }
}

struct CpalDriver(cpal::Stream);

impl Driver for CpalDriver {
    fn play(&mut self) -> Result<(), DeviceError> {
        self.0.play().map_err(|e| DeviceError::new(DeviceErrorKind::Play, e.to_string()))
    }
}

fn build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: SampleFmt,
    mut lease: CallbackLease,
    fault: Arc<FaultSignal>,
) -> Result<cpal::Stream, DeviceError> {
    let channels = config.channels as usize;
    let error = move |error| fault.report(error);
    let result = match format {
        SampleFmt::F32 => device.build_output_stream::<f32, _, _>(config, move |data, _| lease.f32(data, channels), error, None),
        SampleFmt::I8 => build_numeric::<i8>(device, config, lease, error),
        SampleFmt::I16 => build_numeric::<i16>(device, config, lease, error),
        SampleFmt::I24 => build_numeric::<cpal::I24>(device, config, lease, error),
        SampleFmt::I32 => build_numeric::<i32>(device, config, lease, error),
        SampleFmt::I64 => build_numeric::<i64>(device, config, lease, error),
        SampleFmt::U8 => build_numeric::<u8>(device, config, lease, error),
        SampleFmt::U16 => build_numeric::<u16>(device, config, lease, error),
        SampleFmt::U24 => build_numeric::<cpal::U24>(device, config, lease, error),
        SampleFmt::U32 => build_numeric::<u32>(device, config, lease, error),
        SampleFmt::U64 => build_numeric::<u64>(device, config, lease, error),
        SampleFmt::F64 => build_numeric::<f64>(device, config, lease, error),
    };
    result.map_err(|e| DeviceError::new(DeviceErrorKind::Open, e.to_string()))
}

fn build_numeric<T: DeviceSample + cpal::SizedSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut lease: CallbackLease,
    error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error> {
    let channels = config.channels as usize;
    device.build_output_stream::<T, _, _>(config, move |data, _| lease.numeric(data, channels), error, None)
}

/// PR1 convenience entry points retain the engine in Control rather than
/// leaking it for process lifetime. Dropping Control stops and joins its clock.
pub fn open_output(prefer_silent: bool) -> (Control, OutputInfo) {
    let (engine, mut control) = OutputEngine::open(prefer_silent);
    let info = engine.output_info();
    control.output_owner = Some(engine);
    (control, info)
}

pub fn open_silent(sample_rate: u32, reason: String) -> (Control, OutputInfo) {
    let (engine, mut control) = OutputEngine::open_backend(DeviceSelection::Silent, sample_rate, reason, Box::new(CpalBackend), RETRY_DELAYS);
    let info = engine.output_info();
    control.output_owner = Some(engine);
    (control, info)
}

#[cfg(test)]
mod tests;
