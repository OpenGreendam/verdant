//! Explicit opt-in `ogd-worker/1` protocol fixture. All audio is synthetic
//! formant tones; no voice, Python interpreter, or production worker fault mode
//! is involved. Build with `--features verdant-test-worker/protocol-fixture`.

mod admission;
mod artifacts;
mod random;
mod synthesis;

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};

const PROTOCOL: &str = "ogd-worker/1";
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_PHRASE_SECONDS: f64 = 30.0;
static WRITE_LOCK: Mutex<()> = Mutex::new(());

enum RenderError {
    Cancelled,
    Failed(String),
}

impl From<io::Error> for RenderError {
    fn from(error: io::Error) -> Self { Self::Failed(error.to_string()) }
}

impl From<String> for RenderError {
    fn from(message: String) -> Self { Self::Failed(message) }
}

impl From<&str> for RenderError {
    fn from(message: &str) -> Self { Self::Failed(message.into()) }
}

struct Args {
    workspace: PathBuf,
    delay: Duration,
    crash_after_accept: bool,
    corrupt_artifact: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args_os().skip(1);
        let mut workspace = None;
        let mut delay = Duration::from_millis(150);
        let mut crash_after_accept = false;
        let mut corrupt_artifact = false;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--workspace") => workspace = Some(PathBuf::from(args.next().ok_or("--workspace requires a directory")?)),
                Some("--delay") => {
                    let value = args.next().ok_or("--delay requires seconds")?;
                    let seconds = value.to_str().ok_or("--delay must be UTF-8")?.parse::<f64>().map_err(|error| error.to_string())?;
                    delay = Duration::try_from_secs_f64(seconds).map_err(|error| error.to_string())?;
                }
                Some("--crash-after-accept") => crash_after_accept = true,
                Some("--corrupt-artifact") => corrupt_artifact = true,
                Some("--help" | "-h") => {
                    println!("Synthetic ogd-worker/1 protocol fixture, not a voice.\n\n--workspace DIR (required)\n--delay SECONDS (default 0.15, after process_started)\n--crash-after-accept (exit 3 mid-job)\n--corrupt-artifact (publish incorrect WAV hashes)");
                    std::process::exit(0);
                }
                _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
            }
        }
        Ok(Self { workspace: workspace.ok_or("--workspace is required")?, delay, crash_after_accept, corrupt_artifact })
    }
}

struct Active {
    job_id: String,
    cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    finished: HashMap<String, &'static str>,
}

struct Inner {
    args: Args,
    state: Mutex<State>,
}

struct Worker {
    inner: Arc<Inner>,
    // The main thread owns every join handle, including jobs whose active slot
    // was released immediately before their terminal event. EOF/shutdown must
    // join those writers too, as the Python fixture's non-daemon threads did.
    threads: Vec<JoinHandle<()>>,
}

impl Worker {
    fn new(mut args: Args) -> io::Result<Self> {
        std::fs::create_dir_all(&args.workspace)?;
        args.workspace = args.workspace.canonicalize()?;
        Ok(Self { inner: Arc::new(Inner { args, state: Mutex::new(State::default()) }), threads: Vec::new() })
    }

    fn handle(&mut self, request: Value) -> io::Result<bool> {
        let object = match admission::request(&request) {
            Ok(object) => object,
            Err(message) => return reject(&request, "INVALID_REQUEST", &message),
        };
        match object.get("op").and_then(Value::as_str) {
            Some("capabilities") => {
                emit("capabilities", Some(&request), json!({"capabilities": capabilities()}))?;
                Ok(true)
            }
            Some("render") => self.start_render(request),
            Some("cancel") => {
                let Some(job_id) = admission::job_id(object.get("job_id")) else {
                    return reject(&request, "INVALID_REQUEST", "cancel requires a valid job_id");
                };
                let (matches, previous) = {
                    let state = self.inner.state.lock().expect("fixture state lock");
                    let matches = state.active.as_ref().is_some_and(|active| active.job_id == job_id);
                    if matches {
                        state.active.as_ref().expect("matching active job").cancelled.store(true, Ordering::Relaxed);
                    }
                    (matches, state.finished.get(job_id).copied())
                };
                emit("cancel_ack", Some(&request), json!({"active_match": matches, "previous_terminal": previous}))?;
                Ok(true)
            }
            Some("shutdown") => {
                self.stop()?;
                emit("shutdown", Some(&request), json!({}))?;
                Ok(false)
            }
            _ => reject(&request, "INVALID_REQUEST", "Unknown op"),
        }
    }

    fn start_render(&mut self, request: Value) -> io::Result<bool> {
        let job_id = match admission::render(request.as_object().expect("admitted request object")) {
            Ok(job_id) => job_id,
            Err(message) => return reject(&request, "INVALID_REQUEST", &message),
        };
        let mut state = self.inner.state.lock().expect("fixture state lock");
        if state.active.is_some() {
            return reject(&request, "BUSY", "One active job; cancel/coalesce revisions in the client");
        }
        let folder = self.inner.args.workspace.join("jobs").join(job_id);
        let prepare = (|| -> io::Result<bool> {
            if folder.try_exists()? || state.finished.contains_key(job_id) {
                return Ok(false);
            }
            std::fs::create_dir_all(folder.parent().expect("job directory has parent"))?;
            std::fs::create_dir(&folder)?;
            Ok(true)
        })();
        match prepare {
            Ok(true) => {}
            Ok(false) => return reject(&request, "DUPLICATE_JOB", "Use a fresh job_id; archived jobs are immutable"),
            Err(error) => return reject(&request, "INVALID_REQUEST", &error.to_string()),
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let job_id = job_id.to_owned();
        emit("accepted", Some(&request), json!({"job_directory": folder.strip_prefix(&self.inner.args.workspace).expect("owned job directory")}))?;
        let inner = Arc::clone(&self.inner);
        let render_cancelled = Arc::clone(&cancelled);
        // Keep the slot lock through spawn: even a zero-delay job cannot retire
        // its slot before the accepted event and active job are installed.
        state.active = Some(Active { job_id, cancelled });
        let spawned = thread::Builder::new().name("verdant-protocol-fixture".into()).spawn(move || run(&inner, &request, &folder, &render_cancelled));
        match spawned {
            Ok(handle) => self.threads.push(handle),
            Err(error) => {
                state.active = None;
                return Err(error);
            }
        }
        Ok(true)
    }

    fn reap(&mut self) -> io::Result<()> {
        let mut index = 0;
        while index < self.threads.len() {
            if self.threads[index].is_finished() {
                self.threads.swap_remove(index).join().map_err(|_| io::Error::other("protocol fixture render thread panicked"))?;
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        if let Some(active) = self.inner.state.lock().expect("fixture state lock").active.as_ref() {
            active.cancelled.store(true, Ordering::Relaxed);
        }
        let mut failure = None;
        for handle in self.threads.drain(..) {
            if handle.join().is_err() {
                failure = Some(io::Error::other("protocol fixture render thread panicked"));
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

fn run(inner: &Inner, request: &Value, folder: &std::path::Path, cancelled: &AtomicBool) {
    let directory = folder.strip_prefix(&inner.args.workspace).expect("owned job directory");
    let (status, fields) = match render(inner, request, folder, cancelled) {
        Ok(artifacts) => ("completed", json!({"artifacts": artifacts, "identity_status": "test_double_not_a_voice", "job_directory": directory})),
        Err(RenderError::Cancelled) => ("cancelled", json!({"partial_outputs_valid": false, "job_directory": directory})),
        Err(RenderError::Failed(message)) => ("failed", json!({"code": "RENDER_FAILED", "message": message, "job_directory": directory})),
    };
    {
        let mut state = inner.state.lock().expect("fixture state lock");
        let active = state.active.take().expect("render owns active job");
        state.finished.insert(active.job_id, status);
    }
    if let Err(error) = emit(status, Some(request), fields) {
        eprintln!("verdant-test-worker: {error}");
        std::process::exit(1);
    }
}

fn render(inner: &Inner, request: &Value, folder: &std::path::Path, cancelled: &AtomicBool) -> Result<Vec<Value>, RenderError> {
    artifacts::json_file(&folder.join("request.json"), request, true)?;
    progress(request, "native", None)?;
    progress(request, "native", Some("process_started"))?;
    thread::sleep(inner.args.delay);
    if inner.args.crash_after_accept {
        // All emitted records were flushed; this deliberately skips cleanup.
        std::process::exit(3);
    }
    let payload = &request["payload"];
    let segments = synthesis::segments(payload["score"]["segments"].as_array().expect("admitted segments"))?;
    let rate = payload.pointer("/voice_music/sample_rate").and_then(Value::as_u64).unwrap_or(48_000) as u32;
    let track = payload.get("control_track").filter(|value| !value.is_null());
    let samples = if let Some(track) = track {
        artifacts::json_file(&folder.join("provided_control.json"), track, false)?;
        synthesis::synth_tracked(&segments, track, rate, cancelled)?
    } else {
        let seed = payload.get("native_seed").and_then(Value::as_u64).unwrap_or(10008);
        synthesis::synth(&segments, rate, cancelled, seed)?
    };
    artifacts::wav(&folder.join("verdant-voice.wav"), rate, &samples)?;
    artifacts::f0_npz(&folder.join("native.npz"), &segments, track)?;
    if cancelled.load(Ordering::Relaxed) {
        return Err(RenderError::Cancelled);
    }
    artifacts::manifest(&inner.args.workspace, folder, inner.args.corrupt_artifact)
}

fn emit(event: &str, request: Option<&Value>, fields: Value) -> io::Result<()> {
    let mut record = json!({"protocol": PROTOCOL, "event": event});
    let object = record.as_object_mut().expect("event object");
    let Value::Object(fields) = fields else { unreachable!("event fields object") };
    object.extend(fields);
    if let Some(request) = request.and_then(Value::as_object) {
        for key in ["request_id", "job_id", "revision"] {
            if let Some(value) = request.get(key) {
                object.insert(key.into(), value.clone());
            }
        }
    }
    let _guard = WRITE_LOCK.lock().expect("fixture output lock");
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &record).map_err(io::Error::other)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn reject(request: &Value, code: &str, message: &str) -> io::Result<bool> {
    emit("rejected", Some(request), json!({"code": code, "message": message}))?;
    Ok(true)
}

fn progress(request: &Value, stage: &str, phase: Option<&str>) -> io::Result<()> {
    let mut fields = json!({"stage": stage, "progress_kind": "stage_boundary", "fraction": null});
    if let Some(phase) = phase {
        fields["phase"] = phase.into();
    }
    emit("progress", Some(request), fields)
}

fn capabilities() -> Value {
    json!({
        "engine": "Verdant protocol test double (synthetic formant tones; NOT the sn voice)",
        "protocol": PROTOCOL, "native_version": "test-double-0", "test_double": true,
        "routes": ["verdant-voice"], "max_active_jobs": 1, "max_phrase_seconds": MAX_PHRASE_SECONDS,
        "audio_device_required_for_render": false, "wine_required": false,
        "dependency_mode": "rust_protocol_test_double",
        "native_score": {
            "inventory": verdant_backend::pinyin::recovered_syllables(),
            "inventory_note": "synthetic vowels only",
            "english_vowel_proxies": ["a", "i", "u", "e", "o"],
            "explicit_f0_hz_range": [35, 2000], "continuous_f0_control": true,
            "compact_phone_states": true, "phoneme_duration_control": true,
            "phoneme_timing": {"methods":["vowel-anchor-v1"],"routes":["verdant-voice"],"timing_resolution_seconds":0.005,
                "continuity":{"method":"continuous-runs-v1","max_seconds":120,"max_segments":2048,"max_control_points":32768,"minimum_silence_seconds":0.005,"requires_phoneme_timing":"vowel-anchor-v1"}},
        },
        "voice_package": {"name":"verdant-voice","sample_rate":48000,"hop_samples":240,
            "authored_f0_hz_range":[35,2000],"music":{"methods":["verdant-voice-music-v1"],"routes":["verdant-voice"],
                "sample_rates":[44100,48000],"default_sample_rate":48000,"preserves_learned_conditions":true}},
        "candidate_voice": null, "files": {},
        "progress": "real stage boundaries only; no fabricated percentage",
    })
}

fn read_line(input: &mut impl BufRead, line: &mut Vec<u8>) -> io::Result<()> {
    line.clear();
    while line.len() <= MAX_REQUEST_BYTES {
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            break;
        }
        let available = bytes.len().min(MAX_REQUEST_BYTES + 1 - line.len());
        let newline = bytes[..available].iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available, |index| index + 1);
        line.extend_from_slice(&bytes[..count]);
        input.consume(count);
        if newline.is_some() {
            break;
        }
    }
    Ok(())
}

fn serve(worker: &mut Worker) -> io::Result<()> {
    emit("ready", None, json!({"session_id": uuid::Uuid::new_v4().to_string(), "workspace": worker.inner.args.workspace}))?;
    let mut input = io::stdin().lock();
    let mut line = Vec::new();
    loop {
        read_line(&mut input, &mut line)?;
        if line.is_empty() {
            break;
        }
        if line.len() > MAX_REQUEST_BYTES {
            emit("rejected", None, json!({"code": "REQUEST_TOO_LARGE", "message": "Maximum request is1MiB; closing connection"}))?;
            break;
        }
        match serde_json::from_slice(&line) {
            Ok(request) => if !worker.handle(request)? { break; },
            Err(error) => { reject(&Value::Null, "INVALID_REQUEST", &error.to_string())?; }
        }
        worker.reap()?;
    }
    Ok(())
}

fn main() {
    let result = (|| -> Result<(), String> {
        let args = Args::parse()?;
        let mut worker = Worker::new(args).map_err(|error| error.to_string())?;
        let served = serve(&mut worker);
        let stopped = worker.stop();
        served.and(stopped).map_err(|error| error.to_string())
    })();
    if let Err(error) = result {
        eprintln!("verdant-test-worker: {error}");
        std::process::exit(1);
    }
}
