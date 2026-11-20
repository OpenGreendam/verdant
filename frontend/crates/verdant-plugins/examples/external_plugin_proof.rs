//! Bounded proof for deterministic external gain/delay fixtures. This drives
//! the real public client API; it never substitutes the builtin diagnostic.
//! See frontend/docs/PLUGIN-HOST.md for exact fixture commands and limits.

use base64::Engine;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use verdant_audio::Processor;
use verdant_plugins::{ClientEvent, InsertHandle, InsertSpec, InsertStatus,
    ParamDesc, PluginClient, PluginClientOpts, PluginError};

const QUANTUM: usize = verdant_plugins::shm::DSP_QUANTUM_FRAMES;
const TRANSPORT: usize = verdant_plugins::shm::TRANSPORT_LATENCY_FRAMES;
const CALLBACK_BOUND: usize = verdant_plugins::shm::MAX_BLOCK_FRAMES;
const MAX_LATENCY: u32 = 96_000;
const STIMULUS: usize = 4096 + 37;
const TOTAL_BUDGET: Duration = Duration::from_secs(120);
const USAGE: &str = "external_plugin_proof --host PATH --module PATH --format clap|vst3|au --id ID --out NEW_DIR [--param ID --a VALUE --b VALUE | --latency-only] [--expected-latency SAMPLES] [--stall]";
type Result<T, E = String> = std::result::Result<T, E>;

// Only allocations on the thread while Processor::process is executing are
// counted. Reader/watchdog/control threads remain outside this measurement.
thread_local! { static IN_PROCESS: Cell<bool> = const { Cell::new(false) }; }
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
struct ProbeAllocator;
#[global_allocator]
static ALLOCATOR: ProbeAllocator = ProbeAllocator;
fn count(counter: &AtomicU64) {
    if IN_PROCESS.try_with(Cell::get).unwrap_or(false) { counter.fetch_add(1, Ordering::Relaxed); }
}
unsafe impl GlobalAlloc for ProbeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { count(&ALLOCATIONS); unsafe { System.alloc(layout) } }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { count(&ALLOCATIONS); unsafe { System.alloc_zeroed(layout) } }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { count(&DEALLOCATIONS); unsafe { System.dealloc(ptr, layout) } }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(&ALLOCATIONS); count(&DEALLOCATIONS); unsafe { System.realloc(ptr, layout, size) }
    }
}
struct ProcessScope;
impl Drop for ProcessScope { fn drop(&mut self) { IN_PROCESS.with(|flag| flag.set(false)); } }

struct Options {
    host: PathBuf, module: PathBuf, format: String, id: String, out: PathBuf,
    parameter: Option<String>, a: Option<f64>, b: Option<f64>,
    latency_only: bool, expected_latency: Option<u32>, stall: bool,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut args = std::env::args_os().skip(1);
        let (mut host, mut module, mut format, mut id, mut out) = (None, None, None, None, None);
        let (mut parameter, mut a, mut b, mut expected_latency) = (None, None, None, None);
        let (mut latency_only, mut stall) = (false, false);
        while let Some(flag) = args.next() {
            let flag = flag.to_str().ok_or("option names must be UTF-8")?;
            if flag == "--latency-only" { latency_only = true; continue; }
            if flag == "--stall" { stall = true; continue; }
            let value = args.next().ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--host" => host = Some(PathBuf::from(value)),
                "--module" => module = Some(PathBuf::from(value)),
                "--out" => out = Some(PathBuf::from(value)),
                "--format" => format = Some(value.into_string().map_err(|_| "format must be UTF-8")?),
                "--id" => id = Some(value.into_string().map_err(|_| "plugin id must be UTF-8")?),
                "--param" => parameter = Some(value.into_string().map_err(|_| "parameter id must be UTF-8")?),
                "--a" | "--b" => {
                    let number: f64 = value.to_str().ok_or("parameter must be UTF-8")?.parse().map_err(|_| "invalid parameter value")?;
                    if !number.is_finite() { return Err("parameter values must be finite".into()); }
                    if flag == "--a" { a = Some(number); } else { b = Some(number); }
                }
                "--expected-latency" => {
                    let samples: u32 = value.to_str().ok_or("latency must be UTF-8")?.parse().map_err(|_| "invalid latency")?;
                    if samples > MAX_LATENCY { return Err("proof latency limit is 96000 samples".into()); }
                    expected_latency = Some(samples);
                }
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        let options = Self {
            host: host.ok_or("--host is required")?, module: module.ok_or("--module is required")?,
            format: format.ok_or("--format is required")?, id: id.ok_or("--id is required")?,
            out: out.ok_or("--out is required")?, parameter, a, b, latency_only, expected_latency, stall,
        };
        if !matches!(options.format.as_str(), "clap" | "vst3" | "au") || options.id.is_empty() {
            return Err("a real CLAP/VST3/AU format and nonempty external plugin id are required".into());
        }
        if !options.latency_only && (options.parameter.is_none() || options.a.is_none() || options.b.is_none()) {
            return Err("full proof needs --param, --a and --b; use --latency-only for the delay fixture".into());
        }
        if options.latency_only && (options.parameter.is_some() || options.a.is_some() || options.b.is_some()) {
            return Err("--latency-only does not accept parameter selectors".into());
        }
        if options.a.is_some() && options.a == options.b { return Err("--a and --b must differ".into()); }
        Ok(options)
    }
}

#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status { NotRun, Pass, Fail, Unavailable }
#[derive(Serialize)]
struct Check { name: &'static str, status: Status, evidence: Value }
#[derive(Serialize)]
struct Report {
    schema: &'static str, outcome: &'static str, identity: Value,
    host_pids: Vec<u32>, checks: Vec<Check>, failure: Option<String>,
}
impl Report {
    fn new(options: &Options) -> Self {
        let mut names = vec!["scan", "offline_ownership", "offline_pcm", "offline_latency",
            "rt_latency", "rt_bursts", "rt_resampled", "bypass", "crash", "rt_no_alloc", "teardown"];
        if !options.latency_only { names.extend(["parameter_set", "state_restore", "parameter_pcm", "state_pcm"]); }
        if options.stall { names.push("stall"); }
        Self {
            schema: "verdant-external-plugin-proof/2", outcome: "not_run",
            identity: json!({"host": options.host, "module": options.module, "format": options.format,
                "plugin_id": options.id, "sample_rate": 48000, "dsp_quantum_frames": QUANTUM,
                "rt_transport_frames": TRANSPORT, "max_callback_frames": CALLBACK_BOUND,
                "driver": "paced public Processor, no physical audio device",
                "case": if options.latency_only { "latency_only" } else { "parameter_state_pcm" },
                "budget_seconds": TOTAL_BUDGET.as_secs()}),
            host_pids: Vec::new(), checks: names.into_iter().map(|name| Check {
                name, status: Status::NotRun, evidence: Value::Null,
            }).collect(), failure: None,
        }
    }
    fn record(&mut self, name: &'static str, status: Status, evidence: Value) {
        let check = self.checks.iter_mut().find(|check| check.name == name).expect("registered proof gate");
        check.status = status; check.evidence = evidence;
    }
    fn require(&mut self, name: &'static str, condition: bool, evidence: Value) -> Result<()> {
        self.record(name, if condition { Status::Pass } else { Status::Fail }, evidence);
        if condition { Ok(()) } else { Err(format!("{name} failed")) }
    }
    fn failed(&mut self, name: &'static str, error: impl ToString) -> String {
        let error = error.to_string();
        self.record(name, Status::Fail, json!({"error": error})); error
    }
    fn pid(&mut self, pid: u32) { if !self.host_pids.contains(&pid) { self.host_pids.push(pid); } }
}

struct Supervisor {
    pids: Arc<Mutex<BTreeSet<u32>>>, stop: mpsc::Sender<()>,
    timeout: Arc<AtomicBool>, thread: Option<JoinHandle<()>>,
    _tree: ProcessTree,
}
impl Supervisor {
    fn new() -> Result<Self> {
        // Arm native ownership before any scan/insert can spawn a child.
        // PID leases below are receipts, not the boundary for forced cleanup.
        let started = Instant::now();
        let tree = ProcessTree::new()?;
        let pids = Arc::new(Mutex::new(BTreeSet::<u32>::new()));
        let timeout = Arc::new(AtomicBool::new(false));
        let (stop, receiver) = mpsc::channel();
        let expired = timeout.clone();
        let thread = std::thread::Builder::new().name("external-proof-deadline".into()).spawn(move || {
            if receiver.recv_timeout(TOTAL_BUDGET.saturating_sub(started.elapsed())) != Err(mpsc::RecvTimeoutError::Timeout) { return; }
            expired.store(true, Ordering::Release);
            eprintln!("external plugin proof exceeded 120 seconds; terminating its owned process tree");
            if receiver.recv_timeout(Duration::from_secs(3)) == Err(mpsc::RecvTimeoutError::Timeout) {
                std::process::exit(124);
            }
        }).map_err(|e| e.to_string())?;
        Ok(Self { pids, stop, timeout, thread: Some(thread), _tree: tree })
    }
    fn lease(&self, pid: u32) -> PidLease {
        self.pids.lock().unwrap().insert(pid);
        PidLease { pid, pids: self.pids.clone() }
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) { let _ = self.stop.send(()); if let Some(thread) = self.thread.take() { let _ = thread.join(); } }
}

#[cfg(unix)]
struct ProcessTree { guard: std::process::Child, input: Option<std::process::ChildStdin> }
#[cfg(unix)]
impl ProcessTree {
    fn new() -> Result<Self> {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};
        let group = unsafe { libc::getpid() };
        if unsafe { libc::getpgrp() } != group && unsafe { libc::setpgid(0, 0) } != 0 {
            return Err(format!("create proof process group: {}", std::io::Error::last_os_error()));
        }
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command.arg("--guard-process-group").arg(group.to_string())
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
        unsafe { command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
        }); }
        let mut guard = command.spawn().map_err(|e| format!("spawn proof group guard: {e}"))?;
        let input = guard.stdin.take();
        let mut output = guard.stdout.take().expect("piped guard readiness");
        let tree = Self { guard, input };
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err("proof process group guard readiness timed out".into()); }
            let mut fd = libc::pollfd { fd: output.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let result = unsafe { libc::poll(&mut fd, 1, remaining.as_millis().max(1) as i32) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted { continue; }
                return Err(format!("proof guard readiness: {error}"));
            }
            if result == 0 { continue; }
            let mut ready = [0];
            std::io::Read::read_exact(&mut output, &mut ready).map_err(|e| format!("proof guard readiness: {e}"))?;
            if ready != *b"R" { return Err("invalid proof guard readiness".into()); }
            return Ok(tree);
        }
    }
}
#[cfg(unix)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        if let Some(mut input) = self.input.take() { let _ = input.write_all(b"D"); }
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            if matches!(self.guard.try_wait(), Ok(Some(_))) { break; }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = self.guard.kill(); let _ = self.guard.wait(); break;
            }
            std::thread::sleep(remaining.min(Duration::from_millis(5)));
        }
    }
}
#[cfg(unix)]
fn guard_process_group() -> ExitCode {
    let group = std::env::args().nth(2).and_then(|value| value.parse::<i32>().ok());
    let Some(group) = group.filter(|&group| group > 1 && group == unsafe { libc::getppid() }
        && group != unsafe { libc::getpgrp() }) else { return ExitCode::from(2); };
    if unsafe { libc::write(1, b"R".as_ptr().cast(), 1) } != 1 { return ExitCode::from(1); }
    let mut command = [0];
    if std::io::Read::read_exact(&mut std::io::stdin(), &mut command).is_ok() && command == *b"D" {
        return ExitCode::SUCCESS;
    }
    // Abrupt parent exit closes the only write endpoint. This independent
    // group kills every inherited host, even one still inside instantiate.
    unsafe { libc::kill(-group, libc::SIGKILL); }
    ExitCode::from(124)
}

#[cfg(windows)]
struct ProcessTree { job: windows_sys::Win32::Foundation::HANDLE }
#[cfg(windows)]
impl ProcessTree {
    fn new() -> Result<Self> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::{JobObjects::*, Threading::GetCurrentProcess}};
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() { return Err(format!("create proof Job Object: {}", std::io::Error::last_os_error())); }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(job, JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), std::mem::size_of_val(&limits) as u32) == 0
                || AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
                let error = std::io::Error::last_os_error(); CloseHandle(job);
                return Err(format!("arm proof Job Object: {error}"));
            }
            Ok(Self { job })
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            if SetInformationJobObject(self.job, JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), std::mem::size_of_val(&limits) as u32) == 0 {
                eprintln!("disarm proof Job Object: {}", std::io::Error::last_os_error());
                std::process::exit(1);
            }
            CloseHandle(self.job);
        }
    }
}
struct PidLease { pid: u32, pids: Arc<Mutex<BTreeSet<u32>>> }
impl Drop for PidLease { fn drop(&mut self) { self.pids.lock().unwrap().remove(&self.pid); } }

// Field order is intentional: unregister a PID only after the corresponding
// process owner has performed its bounded drop/reap.
struct Offline { handle: InsertHandle, _pid: PidLease }
fn offline(client: &PluginClient, supervisor: &Supervisor, spec: InsertSpec, report: &mut Report) -> Result<Offline> {
    let handle = client.offline_insert(spec).map_err(|e| e.to_string())?;
    let pid = handle.host_pid(); report.pid(pid);
    let lease = supervisor.lease(pid);
    Ok(Offline { handle, _pid: lease })
}
struct Live {
    processor: Box<dyn Processor>, handle: Arc<InsertHandle>, client: Arc<PluginClient>, _pid: PidLease,
}
impl Drop for Live { fn drop(&mut self) { self.client.remove(self.handle.key()); } }
fn live(client: &Arc<PluginClient>, supervisor: &Supervisor, spec: InsertSpec, report: &mut Report) -> Result<Live> {
    let handle = client.insert(spec).map_err(|e| e.to_string())?;
    let Some(processor) = handle.processor() else { client.remove(handle.key()); return Err("fresh live processor unavailable".into()); };
    let pid = handle.host_pid(); report.pid(pid);
    Ok(Live { processor, handle, client: client.clone(), _pid: supervisor.lease(pid) })
}

#[derive(Clone)]
struct Audio { l: Vec<f32>, r: Vec<f32> }
impl Audio {
    fn zeros(frames: usize) -> Self { Self { l: vec![0.0; frames], r: vec![0.0; frames] } }
    fn signal(tail: usize) -> Self {
        let frames = STIMULUS + tail;
        let mut audio = Self::zeros(frames + usize::from(frames % QUANTUM == 0));
        let mut state = 0x1234_5678u32;
        for i in 0..STIMULUS {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            audio.l[i] = ((state >> 16) as f32 / 65535.0 - 0.5) * 0.2;
            audio.r[i] = audio.l[i] * -0.5;
        }
        audio
    }
    fn peak(&self) -> f32 { self.l.iter().chain(&self.r).map(|v| v.abs()).fold(0.0, f32::max) }
    fn finite(&self) -> bool { self.l.iter().chain(&self.r).all(|v| v.is_finite()) }
    fn peak_at(&self) -> Option<usize> {
        self.l.iter().enumerate().filter(|(_, value)| value.abs() > 1e-7)
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()).then_with(|| b.0.cmp(&a.0))).map(|(i, _)| i)
    }
    fn difference(&self, other: &Self) -> f64 {
        if self.l.len() != other.l.len() || self.r.len() != other.r.len() { return f64::INFINITY; }
        self.l.iter().chain(&self.r).zip(other.l.iter().chain(&other.r))
            .map(|(a, b)| (*a as f64 - *b as f64).abs()).fold(0.0, f64::max)
    }
    fn identical(&self, other: &Self) -> bool {
        self.l.len() == other.l.len() && self.r.len() == other.r.len()
            && self.l.iter().chain(&self.r).zip(other.l.iter().chain(&other.r)).all(|(a, b)| a.to_bits() == b.to_bits())
    }
    fn digest(&self) -> String {
        let mut hash = Sha256::new();
        for (l, r) in self.l.iter().zip(&self.r) { hash.update(l.to_le_bytes()); hash.update(r.to_le_bytes()); }
        hex::encode(hash.finalize())
    }
    fn wav(&self, path: &Path) -> Result<()> {
        let mut file = BufWriter::new(OpenOptions::new().create_new(true).write(true).open(path).map_err(|e| e.to_string())?);
        let size = (self.l.len() * 8) as u32;
        let write = |file: &mut BufWriter<std::fs::File>, bytes: &[u8]| file.write_all(bytes).map_err(|e| e.to_string());
        write(&mut file, b"RIFF")?; write(&mut file, &(size + 48).to_le_bytes())?; write(&mut file, b"WAVEfmt ")?;
        for bytes in [16u32.to_le_bytes(), [3, 0, 2, 0], 48000u32.to_le_bytes(), 384000u32.to_le_bytes(), [8, 0, 32, 0]] { write(&mut file, &bytes)?; }
        write(&mut file, b"fact")?; write(&mut file, &4u32.to_le_bytes())?; write(&mut file, &(self.l.len() as u32).to_le_bytes())?;
        write(&mut file, b"data")?; write(&mut file, &size.to_le_bytes())?;
        for (l, r) in self.l.iter().zip(&self.r) { write(&mut file, &l.to_le_bytes())?; write(&mut file, &r.to_le_bytes())?; }
        file.flush().map_err(|e| e.to_string())
    }
}
fn latency(handle: &InsertHandle) -> Result<u32> {
    let samples = handle.plugin_latency_samples();
    if samples > MAX_LATENCY { return Err(format!("reported latency {samples} exceeds bounded proof limit {MAX_LATENCY}")); }
    if handle.latency_samples() != samples + handle.transport_latency_samples() {
        return Err("transport latency report is inconsistent".into());
    }
    Ok(samples)
}
fn warm_up(handle: &mut InsertHandle) -> Result<()> {
    let mut zeros = Audio::zeros(QUANTUM * 2);
    handle.process_offline(&mut zeros.l, &mut zeros.r).map_err(|e| e.to_string())
}
fn parameter(handle: &InsertHandle, id: &str) -> Result<ParamDesc> {
    handle.params().map_err(|e| e.to_string())?.into_iter().find(|p| p.id == id).ok_or_else(|| format!("parameter {id} missing"))
}
fn same_parameter(parameter: &ParamDesc, expected: f64) -> bool {
    (parameter.value - expected).abs() <= (parameter.max - parameter.min).abs() * 1e-6 + 1e-8
}
fn state_metadata(state: &str) -> Result<Value> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(state).map_err(|e| e.to_string())?;
    Ok(json!({"bytes": bytes.len(), "sha256": hex::encode(Sha256::digest(&bytes))}))
}
fn render(client: &PluginClient, supervisor: &Supervisor, spec: &InsertSpec, report: &mut Report) -> Result<(Audio, u32)> {
    let mut owned = offline(client, supervisor, spec.clone(), report)?;
    let latency = latency(&owned.handle)?;
    let mut audio = Audio::signal(latency as usize + QUANTUM);
    owned.handle.process_offline(&mut audio.l, &mut audio.r).map_err(|e| e.to_string())?;
    Ok((audio, latency))
}

#[derive(Default, Serialize)]
struct RtStats { calls: u64, allocations: u64, deallocations: u64, max_call_ns: u64 }
fn measured_process<T>(stats: &mut RtStats, process: impl FnOnce() -> T) -> T {
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let deallocations = DEALLOCATIONS.load(Ordering::Relaxed);
    let start = Instant::now();
    IN_PROCESS.with(|flag| flag.set(true));
    let result = {
        let _scope = ProcessScope;
        process()
    };
    stats.calls += 1;
    stats.max_call_ns = stats.max_call_ns.max(start.elapsed().as_nanos().min(u64::MAX as u128) as u64);
    stats.allocations += ALLOCATIONS.load(Ordering::Relaxed) - allocations;
    stats.deallocations += DEALLOCATIONS.load(Ordering::Relaxed) - deallocations;
    result
}
fn callback(live: &mut Live, audio: &mut Audio, stats: &mut RtStats) {
    measured_process(stats, || live.processor.process(&mut audio.l, &mut audio.r));
}
fn paced_callback(live: &mut Live, audio: &mut Audio, stats: &mut RtStats) {
    callback(live, audio, stats);
    std::thread::sleep(Duration::from_secs_f64(audio.l.len() as f64 / 48_000.0));
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CallbackPhase { Warmup, Capture }

#[derive(Serialize)]
struct CallbackDiagnostic {
    phase: CallbackPhase,
    source_frame_start: usize,
    source_frame_end: usize,
    frames: usize,
    begin_ns: u64,
    end_ns: u64,
    xruns_before: u64,
    xruns_after: u64,
    xrun_delta: u64,
}

fn diagnosed_paced_callback(live: &mut Live, audio: &mut Audio, stats: &mut RtStats,
    phase: CallbackPhase, source_frame_start: usize, origin: Instant) -> CallbackDiagnostic {
    let xruns_before = live.handle.xruns();
    let begin = Instant::now();
    callback(live, audio, stats);
    let end = Instant::now();
    let xruns_after = live.handle.xruns();
    let diagnostic = CallbackDiagnostic {
        phase, source_frame_start, source_frame_end: source_frame_start + audio.l.len(),
        frames: audio.l.len(),
        begin_ns: begin.duration_since(origin).as_nanos().min(u64::MAX as u128) as u64,
        end_ns: end.duration_since(origin).as_nanos().min(u64::MAX as u128) as u64,
        xruns_before, xruns_after, xrun_delta: xruns_after.saturating_sub(xruns_before),
    };
    // Diagnostics contain only scalar values. The caller reserves all record
    // storage before the first callback and stores each record outside the
    // measured process call. Pacing still occurs only after the whole call.
    std::thread::sleep(Duration::from_secs_f64(audio.l.len() as f64 / 48_000.0));
    diagnostic
}

fn prove_rt_bursts(client: &Arc<PluginClient>, supervisor: &Supervisor, spec: &InsertSpec,
    reference: &Audio, out: &Path, report: &mut Report, stats: &mut RtStats) -> Result<(bool, Value)> {
    let cases: [(&str, &[usize]); 5] = [
        ("480", &[480]), ("512", &[512]), ("1024", &[1024]),
        ("non_multiple", &[278, 279, 278, 279, 557, 73]),
        ("maximum_and_partial", &[8192, 37, 480, 512, 1024, 441]),
    ];
    let mut evidence = Vec::new();
    let mut passed = true;
    for (name, sizes) in cases {
        let mut realtime = live(client, supervisor, spec.clone(), report)?;
        let plugin_samples = latency(&realtime.handle)? as usize;
        let transport = realtime.handle.transport_latency_samples() as usize;
        let largest = *sizes.iter().max().unwrap();
        let smallest = *sizes.iter().min().unwrap();
        let warmup_frames = transport + plugin_samples + largest;
        let capture_frames = transport + reference.l.len() + largest;
        // Every call except the final capture tail contains at least smallest
        // frames, so this bound reserves the entire case before any callback.
        let diagnostic_capacity = warmup_frames.div_ceil(smallest) + capture_frames.div_ceil(smallest);
        let mut diagnostics = Vec::with_capacity(diagnostic_capacity);
        let origin = Instant::now();
        let mut call = 0;
        let mut warmed = 0;
        while warmed < warmup_frames {
            let n = sizes[call % sizes.len()];
            let mut zeros = Audio::zeros(n);
            diagnostics.push(diagnosed_paced_callback(&mut realtime, &mut zeros, stats,
                CallbackPhase::Warmup, warmed, origin));
            warmed += n; call += 1;
        }
        let mut captured = Audio::zeros(capture_frames);
        let mut offset = 0;
        while offset < captured.l.len() {
            let n = sizes[call % sizes.len()].min(captured.l.len() - offset);
            let mut block = Audio::zeros(n);
            if offset == 0 { block.l[0] = 0.25; block.r[0] = -0.125; }
            // Exactly one public process call per device burst. The only
            // pacing sleep occurs after that complete call has returned.
            diagnostics.push(diagnosed_paced_callback(&mut realtime, &mut block, stats,
                CallbackPhase::Capture, warmed + offset, origin));
            captured.l[offset..offset + n].copy_from_slice(&block.l);
            captured.r[offset..offset + n].copy_from_slice(&block.r);
            offset += n; call += 1;
        }
        let mut expected = Audio::zeros(captured.l.len());
        expected.l[transport..transport + reference.l.len()].copy_from_slice(&reference.l);
        expected.r[transport..transport + reference.r.len()].copy_from_slice(&reference.r);
        let difference = captured.difference(&expected);
        let measured = captured.peak_at();
        let xruns = realtime.handle.xruns();
        let key = realtime.handle.key();
        let pid = realtime.handle.host_pid();
        let status = realtime.handle.status();
        drop(realtime);
        let registry_retired = client.handle(key).is_none();
        let ok = transport == TRANSPORT && captured.finite() && difference <= 1e-6
            && measured == Some(transport + plugin_samples) && xruns == 0 && registry_retired;
        passed &= ok;
        evidence.push(json!({"case": name, "callback_frames": sizes, "pid": pid,
            "reported_plugin_samples": plugin_samples, "reported_transport_samples": transport,
            "measured_impulse_samples": measured, "max_pcm_delta_from_delayed_offline": difference,
            "xruns": xruns, "host_status_after_case": format!("{status:?}"),
            "registry_retired": registry_retired, "pass": ok,
            "callback_diagnostics": {
                "clock": "monotonic nanoseconds since this case began; begin/end exclude pacing sleep",
                "source_frame_origin": "first warmup input frame; capture starts at warmup_source_frames",
                "warmup_source_frames": warmed, "capture_source_frames": capture_frames,
                "preallocated_record_capacity": diagnostic_capacity, "calls": diagnostics,
            }}));
        captured.wav(&out.join(format!("realtime-burst-{name}.wav")))?;
    }
    Ok((passed, json!(evidence)))
}
struct RendererHost { handle: Arc<InsertHandle>, client: Arc<PluginClient>, _pid: PidLease }
impl Drop for RendererHost { fn drop(&mut self) { self.client.remove(self.handle.key()); } }
fn prove_resampled_callbacks(client: &Arc<PluginClient>, supervisor: &Supervisor,
    spec: &InsertSpec, out: &Path, report: &mut Report, stats: &mut RtStats) -> Result<(bool, Value)> {
    use verdant_audio::{CallbackRenderer, Clip, Timeline};
    use verdant_audio::engine::channel;
    let mut reference_host = offline(client, supervisor, spec.clone(), report)?;
    let plugin_samples = latency(&reference_host.handle)? as usize;
    let mut source = Audio::zeros(TRANSPORT + plugin_samples + STIMULUS + QUANTUM * 4);
    source.l[QUANTUM * 4] = 0.125;
    source.r.copy_from_slice(&source.l);
    let mut native = source.clone();
    reference_host.handle.process_offline(&mut native.l, &mut native.r).map_err(|e| e.to_string())?;
    if native.l.iter().zip(&native.r).any(|(l, r)| (*l - *r).abs() > 1e-6) {
        return Err("resampled proof requires a symmetric stereo gain/delay fixture".into());
    }
    drop(reference_host);
    let timeline_frames = source.l.len() + TRANSPORT + QUANTUM * 2;
    let sizes = [480, 512, 1024, 8192, 73, 441];
    let mut evidence = Vec::new();
    let mut passed = true;
    for rate in [44_100u32, 48_000, 96_000] {
        let handle = client.insert(spec.clone()).map_err(|e| e.to_string())?;
        let pid = handle.host_pid(); report.pid(pid);
        let owner = RendererHost { handle, client: client.clone(), _pid: supervisor.lease(pid) };
        let processor = owner.handle.processor().ok_or("resampled live processor unavailable")?;
        let (mut control, rt) = channel(48_000, CALLBACK_BOUND);
        let mut timeline = Timeline::new(48_000, vec![Clip { start: 0,
            samples: source.l.clone().into(), gain: 1.0, track: 0 }]);
        timeline.length = timeline_frames as u64;
        control.set_timeline(timeline);
        if !control.set_chain(0, vec![processor]) { return Err("install resampled insert chain failed".into()); }
        control.play();
        let mut renderer = CallbackRenderer::new(rt, 48_000);
        renderer.set_device_rate(rate).map_err(|e| e.to_string())?;
        let (mut reference_control, reference_rt) = channel(48_000, CALLBACK_BOUND);
        let mut timeline = Timeline::new(48_000, vec![Clip { start: TRANSPORT as u64,
            samples: native.l.clone().into(), gain: 1.0, track: 0 }]);
        timeline.length = timeline_frames as u64;
        reference_control.set_timeline(timeline); reference_control.play();
        let mut reference = CallbackRenderer::new(reference_rt, 48_000);
        reference.set_device_rate(rate).map_err(|e| e.to_string())?;
        let frames = ((timeline_frames as u64 * rate as u64).div_ceil(48_000) + QUANTUM as u64) as usize;
        let mut captured = Audio::zeros(frames);
        let mut expected = Audio::zeros(frames);
        let mut offset = 0;
        let mut call = 0;
        while offset < frames {
            let n = sizes[call % sizes.len()].min(frames - offset);
            let mut actual_block = vec![0.0; n * 2];
            let mut reference_block = vec![0.0; n * 2];
            // The real output adapter may call the source mixer repeatedly.
            // It gets no artificial pacing between those inner source calls.
            measured_process(stats, || renderer.process_f32(&mut actual_block, 2)).map_err(|e| e.to_string())?;
            reference.process_f32(&mut reference_block, 2).map_err(|e| e.to_string())?;
            for i in 0..n {
                captured.l[offset + i] = actual_block[i * 2];
                captured.r[offset + i] = actual_block[i * 2 + 1];
                expected.l[offset + i] = reference_block[i * 2];
                expected.r[offset + i] = reference_block[i * 2 + 1];
            }
            std::thread::sleep(Duration::from_secs_f64(n as f64 / rate as f64));
            offset += n; call += 1;
        }
        let difference = captured.difference(&expected);
        let xruns = owner.handle.xruns();
        let key = owner.handle.key();
        let transport = owner.handle.transport_latency_samples();
        drop(renderer); drop(control);
        drop(reference); drop(reference_control);
        drop(owner);
        let retired = client.handle(key).is_none();
        let ok = captured.finite() && captured.peak() > 1e-7 && difference <= 1e-6
            && xruns == 0 && transport == TRANSPORT as u32 && retired;
        passed &= ok;
        evidence.push(json!({"device_rate": rate, "source_rate": 48000, "device_callback_frames": sizes,
            "driver": "actual CallbackRenderer::process_f32, paced only after whole device callback",
            "reported_transport_samples": transport, "pid": pid, "xruns": xruns,
            "max_pcm_delta_from_resampled_delayed_offline": difference, "registry_retired": retired, "pass": ok}));
        captured.wav(&out.join(format!("realtime-resampled-{rate}.wav")))?;
    }
    Ok((passed, json!(evidence)))
}
fn wait_crash(client: &PluginClient, handle: &InsertHandle) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        while let Some(event) = client.try_recv_event() {
            if let ClientEvent::Crashed { insert, detail } = event {
                if insert == handle.key() && matches!(handle.status(), InsertStatus::Crashed { .. }) { return Ok(detail); }
            }
        }
        if Instant::now() >= deadline { return Err("missing crashed status/event within four seconds".into()); }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn fd_count() -> Option<usize> {
    #[cfg(target_os = "linux")]
    { fs::read_dir("/proc/self/fd").ok().map(|entries| entries.count()) }
    #[cfg(not(target_os = "linux"))]
    { None }
}

fn run(options: &Options, supervisor: &Supervisor, report: &mut Report) -> Result<()> {
    if options.format == "au" && !cfg!(target_os = "macos") {
        report.record("scan", Status::Unavailable, json!({"reason": "native AU requires macOS"}));
        return Ok(());
    }
    if !options.host.is_file() || !options.module.exists() { return Err("host or external module path does not exist".into()); }
    let initial_fds = fd_count();
    let client = PluginClient::new(PluginClientOpts {
        plughost_path: Some(options.host.clone()), cache_dir: options.out.join("scan-cache"),
        sample_rate: 48_000.0, max_frames: CALLBACK_BOUND,
    }).map_err(|e| report.failed("scan", e))?;
    let scan = client.scan_file(&options.module).map_err(|e| report.failed("scan", e))?;
    if let Some(error) = scan.error { return Err(report.failed("scan", error)); }
    let descriptor = scan.plugins.iter().find(|p| p.format == options.format &&
        (p.id == options.id || options.format == "vst3" && p.id.eq_ignore_ascii_case(&options.id)))
        .ok_or_else(|| report.failed("scan", "requested external plugin id absent from scan"))?;
    let mut spec = InsertSpec::new(&options.format, &descriptor.id, &descriptor.name,
        options.module.to_str().ok_or("module path must be UTF-8 for the public protocol")?);
    spec.version = descriptor.version.clone();
    report.record("scan", Status::Pass, json!({"sha256": scan.sha256, "plugin_id": descriptor.id,
        "name": descriptor.name, "version": descriptor.version, "module": options.module}));

    let mut owned = offline(&client, supervisor, spec.clone(), report).map_err(|e| report.failed("offline_ownership", e))?;
    let pid = owned.handle.host_pid();
    let registry_shared = client.handle(owned.handle.key()).is_some();
    let rt_processor_available = owned.handle.processor().is_some();
    let status = owned.handle.status();
    report.require("offline_ownership", pid != 0 && !registry_shared
        && !rt_processor_available && owned.handle.transport_latency_samples() == 0
        && matches!(status, InsertStatus::Running),
        json!({"pid": pid, "registry_shared": registry_shared,
            "transport_samples": owned.handle.transport_latency_samples(),
            "rt_processor_available": rt_processor_available, "status": format!("{status:?}")}))?;
    let mut render_spec = spec.clone();
    if !options.latency_only {
        let params = owned.handle.params().map_err(|e| report.failed("parameter_set", e))?;
        if params.is_empty() {
            report.record("parameter_set", Status::Unavailable, json!({"reason": "plugin reports no parameters"}));
            return Ok(());
        }
        let id = options.parameter.as_deref().expect("validated parameter selector");
        let a = options.a.expect("validated a"); let b = options.b.expect("validated b");
        // No audio, including silent warm-up, may reach these instances before
        // the immediate set/save/restore checks. Controller readback alone can
        // hide a processor parameter queue that has not reached native DSP.
        owned.handle.set_param(id, a).map_err(|e| report.failed("parameter_set", e))?;
        let state_a = match owned.handle.get_state_b64() {
            Err(PluginError::Host(error)) if error == "CLAP has no state extension" => {
                report.record("state_restore", Status::Unavailable, json!({"reason": error})); return Ok(());
            }
            state => state.map_err(|e| report.failed("state_restore", e))?,
        };
        let pa = parameter(&owned.handle, id).map_err(|e| report.failed("parameter_set", e))?;
        owned.handle.set_param(id, b).map_err(|e| report.failed("parameter_set", e))?;
        let state_b = owned.handle.get_state_b64().map_err(|e| report.failed("state_restore", e))?;
        let pb = parameter(&owned.handle, id).map_err(|e| report.failed("parameter_set", e))?;
        report.require("parameter_set", same_parameter(&pa, a) && same_parameter(&pb, b), json!({"a": pa, "b": pb}))?;
        owned.handle.set_state_b64(&state_a).map_err(|e| report.failed("state_restore", e))?;
        let resaved_a = owned.handle.get_state_b64().map_err(|e| report.failed("state_restore", e))?;
        let in_place = parameter(&owned.handle, id).map_err(|e| report.failed("state_restore", e))?;
        if !same_parameter(&in_place, a) {
            return Err(report.failed("state_restore", "set_state_b64 did not restore parameter A"));
        }
        let mut spec_a = spec.clone(); spec_a.state_b64 = state_a;
        render_spec.state_b64 = state_b;
        drop(owned);
        let restored = offline(&client, supervisor, render_spec.clone(), report).map_err(|e| report.failed("state_restore", e))?;
        let restored_param = parameter(&restored.handle, id).map_err(|e| report.failed("state_restore", e))?;
        restored.handle.set_state_b64(&resaved_a).map_err(|e| report.failed("state_restore", e))?;
        let resaved_param = parameter(&restored.handle, id).map_err(|e| report.failed("state_restore", e))?;
        report.require("state_restore", restored.handle.host_pid() != pid && same_parameter(&restored_param, b)
            && same_parameter(&resaved_param, a),
            json!({"saved_pid": pid, "restored_pid": restored.handle.host_pid(), "parameter": restored_param,
                "in_place_parameter": in_place, "in_place_resaved_parameter": resaved_param,
                "audio_frames_before_save": 0, "audio_frames_before_restore_check": 0,
                "in_place_resaved_state": state_metadata(&resaved_a)?,
                "a_state": state_metadata(&spec_a.state_b64)?, "b_state": state_metadata(&render_spec.state_b64)?}))?;
        drop(restored);
        let (audio_a, _) = render(&client, supervisor, &spec_a, report).map_err(|e| report.failed("parameter_pcm", e))?;
        let (audio_b, _) = render(&client, supervisor, &render_spec, report).map_err(|e| report.failed("parameter_pcm", e))?;
        let difference = audio_a.difference(&audio_b);
        report.require("parameter_pcm", audio_a.finite() && audio_b.finite() && difference > 1e-5,
            json!({"max_delta": difference, "a_sha256": audio_a.digest(), "b_sha256": audio_b.digest()}))?;
        let (repeat, _) = render(&client, supervisor, &render_spec, report).map_err(|e| report.failed("state_pcm", e))?;
        report.require("state_pcm", repeat.finite() && repeat.difference(&audio_b) <= 1e-6,
            json!({"max_delta": repeat.difference(&audio_b), "fresh_state_pcm_sha256": repeat.digest()}))?;
        report.require("offline_pcm", audio_b.peak() > 1e-7 && audio_b.finite(),
            json!({"frames": audio_b.l.len(), "final_block_frames": audio_b.l.len() % QUANTUM, "peak": audio_b.peak(), "sha256": audio_b.digest()}))?;
        audio_a.wav(&options.out.join("parameter-a.wav"))?;
        audio_b.wav(&options.out.join("parameter-b.wav"))?;
    } else {
        warm_up(&mut owned.handle).map_err(|e| report.failed("offline_pcm", e))?;
        let samples = latency(&owned.handle).map_err(|e| report.failed("offline_pcm", e))?;
        let mut audio = Audio::signal(samples as usize + QUANTUM);
        owned.handle.process_offline(&mut audio.l, &mut audio.r).map_err(|e| report.failed("offline_pcm", e))?;
        report.require("offline_pcm", audio.finite() && audio.peak() > 1e-7,
            json!({"frames": audio.l.len(), "final_block_frames": audio.l.len() % QUANTUM, "peak": audio.peak(), "sha256": audio.digest()}))?;
        audio.wav(&options.out.join("offline.wav"))?;
        drop(owned);
    }

    let mut impulse_host = offline(&client, supervisor, render_spec.clone(), report).map_err(|e| report.failed("offline_latency", e))?;
    warm_up(&mut impulse_host.handle).map_err(|e| report.failed("offline_latency", e))?;
    let samples = latency(&impulse_host.handle).map_err(|e| report.failed("offline_latency", e))?;
    let mut impulse = Audio::zeros(samples as usize + QUANTUM * 2 + 37);
    impulse.l[0] = 0.25; impulse.r[0] = -0.125;
    impulse_host.handle.process_offline(&mut impulse.l, &mut impulse.r).map_err(|e| report.failed("offline_latency", e))?;
    report.require("offline_latency", impulse.finite() && impulse.peak_at() == Some(samples as usize)
        && options.expected_latency.is_none_or(|expected| expected == samples),
        json!({"reported_plugin_samples": samples, "measured_impulse_samples": impulse.peak_at(),
            "reported_total_samples": impulse_host.handle.latency_samples(), "expected_samples": options.expected_latency,
            "expected_pipeline_in_offline_pcm": 0}))?;
    impulse.wav(&options.out.join("offline-impulse.wav"))?;
    let dry = Audio::signal(0); let mut bypassed = dry.clone();
    impulse_host.handle.set_bypass(true);
    impulse_host.handle.process_offline(&mut bypassed.l, &mut bypassed.r).map_err(|e| report.failed("bypass", e))?;
    let offline_bypass = bypassed.identical(&dry);
    drop(impulse_host);

    let mut stats = RtStats::default();
    let (bursts_passed, burst_evidence) = prove_rt_bursts(&client, supervisor, &render_spec,
        &impulse, &options.out, report, &mut stats).map_err(|e| report.failed("rt_bursts", e))?;
    report.require("rt_bursts", bursts_passed, burst_evidence)?;
    let (resampled_passed, resampled_evidence) = prove_resampled_callbacks(&client, supervisor, &render_spec,
        &options.out, report, &mut stats).map_err(|e| report.failed("rt_resampled", e))?;
    report.require("rt_resampled", resampled_passed, resampled_evidence)?;
    let mut realtime = live(&client, supervisor, render_spec.clone(), report).map_err(|e| report.failed("rt_latency", e))?;
    let transport = realtime.handle.transport_latency_samples() as usize;
    let warmup_frames = transport + realtime.handle.plugin_latency_samples() as usize + QUANTUM;
    for _ in 0..warmup_frames.div_ceil(QUANTUM) {
        let mut zeros = Audio::zeros(QUANTUM); paced_callback(&mut realtime, &mut zeros, &mut stats);
    }
    let rt_samples = latency(&realtime.handle).map_err(|e| report.failed("rt_latency", e))?;
    let frames = (rt_samples as usize + transport + QUANTUM * 3).div_ceil(QUANTUM) * QUANTUM;
    let mut captured = Audio::zeros(frames);
    for offset in (0..frames).step_by(QUANTUM) {
        let mut block = Audio::zeros(QUANTUM);
        if offset == 0 { block.l[0] = 0.25; block.r[0] = -0.125; }
        paced_callback(&mut realtime, &mut block, &mut stats);
        captured.l[offset..offset + QUANTUM].copy_from_slice(&block.l);
        captured.r[offset..offset + QUANTUM].copy_from_slice(&block.r);
    }
    report.require("rt_latency", transport == TRANSPORT && captured.finite()
        && captured.peak_at() == Some(rt_samples as usize + transport)
        && options.expected_latency.is_none_or(|expected| expected == rt_samples),
        json!({"reported_plugin_samples": rt_samples, "measured_impulse_samples": captured.peak_at(),
            "transport_frames": transport, "dsp_quantum_frames": QUANTUM, "xruns": realtime.handle.xruns()}))?;
    captured.wav(&options.out.join("realtime-impulse.wav"))?;
    let mut block = Audio::signal(0); block.l.truncate(73); block.r.truncate(73);
    let dry = block.clone();
    realtime.handle.set_bypass(true);
    callback(&mut realtime, &mut block, &mut stats);
    let rt_bypass = block.identical(&dry);
    realtime.handle.set_bypass(false);
    report.require("bypass", offline_bypass && rt_bypass && !realtime.handle.bypassed(),
        json!({"offline_exact_dry": offline_bypass, "realtime_exact_dry": rt_bypass, "callback_frames": 73}))?;
    let xruns = realtime.handle.xruns(); let key = realtime.handle.key(); let killed_pid = realtime.handle.host_pid();
    realtime.handle.kill_host();
    let detail = wait_crash(&client, &realtime.handle).map_err(|e| report.failed("crash", e))?;
    let mut block = dry.clone(); callback(&mut realtime, &mut block, &mut stats);
    report.require("crash", block.identical(&dry) && realtime.handle.xruns() > xruns,
        json!({"pid": killed_pid, "detail": detail, "exact_dry": block.identical(&dry), "xruns": realtime.handle.xruns()}))?;
    drop(realtime);
    let mut retired = vec![key];
    if options.stall {
        #[cfg(unix)]
        {
            let mut stalled = live(&client, supervisor, render_spec, report).map_err(|e| report.failed("stall", e))?;
            let pid = stalled.handle.host_pid();
            if unsafe { libc::kill(pid as i32, libc::SIGSTOP) } != 0 { return Err(report.failed("stall", std::io::Error::last_os_error())); }
            let mut block = Audio::zeros(QUANTUM); block.l.fill(0.125); block.r.fill(-0.25);
            callback(&mut stalled, &mut block, &mut stats);
            let detail = wait_crash(&client, &stalled.handle).map_err(|e| report.failed("stall", e))?;
            let mut block = dry.clone(); callback(&mut stalled, &mut block, &mut stats);
            report.require("stall", detail.contains("stalled") && block.identical(&dry),
                json!({"pid": pid, "detail": detail, "exact_dry": block.identical(&dry)}))?;
            retired.push(stalled.handle.key()); drop(stalled);
        }
        #[cfg(not(unix))]
        report.record("stall", Status::Unavailable, json!({"reason": "SIGSTOP proof requires a POSIX host"}));
    }
    report.require("rt_no_alloc", stats.allocations == 0 && stats.deallocations == 0, json!(stats))?;
    let final_fds = fd_count();
    let remaining_hosts = supervisor.pids.lock().unwrap().len();
    let fds_retired = match (initial_fds, final_fds) {
        (Some(before), Some(after)) => after <= before,
        (None, None) => !cfg!(target_os = "linux"),
        _ => false,
    };
    let registry_retired = retired.iter().all(|&key| client.handle(key).is_none());
    report.require("teardown", registry_retired
        && remaining_hosts == 0 && fds_retired,
        json!({"registry_retired": registry_retired, "owned_hosts_remaining": remaining_hosts,
            "linux_fds_before": initial_fds, "linux_fds_after": final_fds,
            "fd_observation": if initial_fds.is_some() { "native /proc/self/fd" } else { "unavailable on this platform" }}))?;
    Ok(())
}

fn main() -> ExitCode {
    #[cfg(unix)]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--guard-process-group")) {
        return guard_process_group();
    }
    let options = match Options::parse() {
        Ok(options) => options,
        Err(error) => { eprintln!("{error}\n{USAGE}"); return ExitCode::from(2); }
    };
    // Preserve prior receipts: use a fresh output directory for each run.
    if let Err(error) = fs::create_dir(&options.out) { eprintln!("create proof directory: {error}"); return ExitCode::from(2); }
    let supervisor = match Supervisor::new() {
        Ok(supervisor) => supervisor,
        Err(error) => { eprintln!("supervisor: {error}"); return ExitCode::from(1); }
    };
    let mut report = Report::new(&options);
    let result = run(&options, &supervisor, &mut report);
    if let Err(error) = result { report.failure = Some(error); }
    let code = if supervisor.timeout.load(Ordering::Acquire) { report.outcome = "timeout"; 124 }
        else if report.failure.is_some() || report.checks.iter().any(|c| c.status == Status::Fail) { report.outcome = "fail"; 1 }
        else if report.checks.iter().any(|c| c.status == Status::Unavailable) { report.outcome = "unavailable"; 77 }
        else if report.checks.iter().any(|c| c.status != Status::Pass) { report.outcome = "fail"; report.failure = Some("requested gates did not run".into()); 1 }
        else { report.outcome = "pass"; 0 };
    let write = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string()).and_then(|mut bytes| {
        bytes.push(b'\n');
        fs::write(options.out.join("report.json"), &bytes).map_err(|e| e.to_string())?;
        std::io::stdout().write_all(&bytes).map_err(|e| e.to_string())
    });
    if let Err(error) = write { eprintln!("write proof receipt: {error}"); return ExitCode::from(1); }
    ExitCode::from(code)
}
