//! Production `ogd-worker/1` server. Every numerical stage executes Rust/native
//! code in a supervised Rust child; stdout is exclusively the NDJSON protocol.
#![recursion_limit = "256"]
mod admission;
mod artifacts;
mod bundled_voice;
mod capabilities;
mod pipeline;
mod runtime;

use std::{collections::BTreeMap, fs, io::{self, BufRead, BufReader, Read, Write}, path::{Path, PathBuf},
    process::{Child, Command, Stdio}, sync::{Arc, LazyLock, Mutex, OnceLock, atomic::{AtomicBool, Ordering}, mpsc}, thread, time::{Duration, Instant}};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const PROTOCOL: &str = "ogd-worker/1";
pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static OUTPUT: OnceLock<Mutex<()>> = OnceLock::new();

fn max_active_jobs() -> usize {
    static LIMIT: LazyLock<usize> = LazyLock::new(|| if thread::available_parallelism().is_ok_and(|cpus| cpus.get() >= 4) { 2 } else { 1 });
    *LIMIT
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub workspace: PathBuf,
    pub project_root: PathBuf,
    #[serde(default)]
    pub voice_package: Option<PathBuf>,
    #[serde(default)]
    pub original: Option<(PathBuf, PathBuf)>,
    pub voice_registry: Option<PathBuf>,
    pub onnxruntime_library: Option<PathBuf>,
    #[serde(default)]
    pub selection: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled_voice: Option<bundled_voice::Provenance>,
    #[serde(skip)]
    bundle: Option<Arc<bundled_voice::Bundle>>,
    #[serde(default)]
    pub asset_sha256: BTreeMap<String,String>,
    #[serde(default)]
    pub request_sha256: String,
}
impl Config {
    fn parse() -> Result<(Self, Option<PathBuf>, Option<PathBuf>, Option<(String,bool)>)> {
        let mut config = Self {
            workspace: runtime::cache_dir().join("render"),
            project_root: std::env::var_os("VERDANT_REPO").map(PathBuf::from).unwrap_or(std::env::current_dir()?),
            voice_package: std::env::var_os("VERDANT_VOICE_PACKAGE_DIR").map(PathBuf::from),
            original: None,
            voice_registry: std::env::var_os("VERDANT_VOICE_REGISTRY").map(PathBuf::from)
                .or_else(|| cfg!(any(windows, target_os="macos")).then(|| verdant_backend::platform::state_dir().join("voices.json"))),
            onnxruntime_library: std::env::var_os("VERDANT_ONNXRUNTIME_LIBRARY").map(PathBuf::from),
            selection: None,
            bundled_voice: None,
            bundle: None,
            asset_sha256: BTreeMap::new(),
            request_sha256: String::new(),
        };
        let mut args = std::env::args().skip(1);
        let mut execute = None;
        let mut job_config = None;
        let mut default_voice = None;
        while let Some(arg) = args.next() {
            let (key, inline) = arg.split_once('=').map_or((arg.as_str(), None), |(k,v)| (k,Some(v.to_owned())));
            if key == "--runtime-features" {
                println!("{}",json!({"schema":"verdant-runtime-features/1","role":"worker",
                    "version":env!("CARGO_PKG_VERSION"),"protocol":PROTOCOL,
                    "voice_package":{"name":"verdant-voice","version":env!("CARGO_PKG_VERSION")},
                    "model_extensions":["verdant-voice"],"strategies":[verdant_synthesis::voice::STRATEGY],
                    "voice_bank_sha256":verdant_synthesis::model::compiled_voice_bank_sha256(),
                    "bundled_voice_policy_sha256":bundled_voice::compiled_policy_sha256(),
                    "registry_schema_version":1,"managed_default":true,"immutable_voice_pins":true}));
                std::process::exit(0);
            }
            if matches!(key, "-h" | "--help") {
                println!("verdant-worker [--workspace DIR] [--voice-package DIR] [--project-root DIR] [--voice-registry FILE] [--onnxruntime-library FILE]\nUse --set-default-voice REF for an explicit selection, or --upgrade-default-voice REF to preserve user-selected defaults. Both require an admitted --voice-registry entry.\nverdant-voice package directory: VERDANT_VOICE_PACKAGE_DIR.");
                std::process::exit(0);
            }
            let value = inline.or_else(|| args.next()).with_context(|| format!("{key} requires a value"))?;
            match key {
                "--workspace" => config.workspace = value.into(),
                "--project-root" => config.project_root = value.into(),
                "--voice-package" => config.voice_package = Some(value.into()),
                "--voice-registry" => config.voice_registry = Some(value.into()),
                "--onnxruntime-library" => config.onnxruntime_library = Some(value.into()),
                "--execute-job" => execute = Some(PathBuf::from(value)),
                "--job-config" => job_config = Some(PathBuf::from(value)),
                "--set-default-voice" | "--upgrade-default-voice" => {
                    ensure!(default_voice.is_none(),"choose one default-voice operation");
                    default_voice=Some((value,key=="--upgrade-default-voice"));
                }
                _ => bail!("unknown worker option {key}"),
            }
        }
        ensure!(default_voice.is_none()||(execute.is_none()&&job_config.is_none()),"default selection cannot accompany job execution");
        if execute.is_some() { return Ok((config,execute,job_config,default_voice)); }
        fs::create_dir_all(&config.workspace)?;
        config.workspace = config.workspace.canonicalize()?;
        config.project_root = config.project_root.canonicalize().context("project root")?;
        config.voice_package = config.voice_package.as_deref().map(runtime::absolute).transpose()?;
        config.voice_registry = config.voice_registry.as_deref().map(runtime::absolute).transpose()?;
        config.onnxruntime_library = config.onnxruntime_library.as_deref().map(runtime::absolute).transpose()?;
        if let Some(bundle) = bundled_voice::discover(&config)? {
            let configured_voice = config.voice_package.is_some();
            let registry = config.voice_registry.as_ref().map(verdant_backend::voices::VoiceRegistry::load_or_empty).transpose()?;
            let available_default = registry.as_ref().is_none_or(|registry| registry.default_reference().is_none()
                && !registry.references().iter().any(|reference|reference==bundle.reference()));
            let bundle = Arc::new(bundle);
            config.bundle = Some(bundle.clone());
            if !configured_voice && available_default { config = bundle.select(&config); }
        }
        Ok((config, execute, job_config,default_voice))
    }
    fn selected(&self, request: &Value) -> Result<Self> {
        admission::route(request)?;
        let reference = request.get("voice_ref").and_then(Value::as_str);
        let original = request["route"] == "C0";
        let Some(path) = &self.voice_registry else {
            ensure!(!original, "original synthesis requires a registered original pair");
            if let Some(bundle) = self.bundle.as_ref().filter(|bundle|reference==Some(bundle.reference())) {
                return Ok(bundle.select(self));
            }
            ensure!(reference.is_none() || reference == Some("voice-sn"), "selected voice requires --voice-registry");
            return Ok(self.clone());
        };
        let registry = verdant_backend::voices::VoiceRegistry::load_or_empty(path)?;
        // An unregistered historical document alias retains the configured
        // singer instead of selecting a different registry default.
        if reference==Some("voice-sn")&&!registry.references().iter().any(|value|value=="voice-sn"){return Ok(self.clone());}
        let Some(reference) = reference.or_else(||registry.default_reference()) else { return Ok(self.clone()); };
        if let Some(bundle) = self.bundle.as_ref().filter(|bundle|reference==bundle.reference()
            && !registry.references().iter().any(|value|value==reference)) {
            return Ok(bundle.select(self));
        }
        let selected = registry.selection(reference)?;
        let assets = &selected.assets;
        let mut config = self.clone();
        config.bundled_voice=None;
        config.original = if original {
            ensure!(assets.voice_package.is_none() && assets.voice_model.is_none(), "original synthesis requires the exact unlocked raw pair");
            Some((assets.token.clone().context("original token is missing")?,
                assets.model.clone().context("original model is missing")?))
        } else { None };
        config.voice_package = if original { None } else {
            Some(assets.voice_package.clone().context("selected voice requires an admitted verdant-voice package")?)
        };
        config.selection = Some(json!({"reference":selected.reference,"source_sha256":selected.source_sha256,
            "cache_identity":selected.cache_identity,"historical_identity_confirmed":selected.historical_identity_confirmed,
            "source_receipt":selected.source_receipt,"package_receipt":selected.package_receipt}));
        if let Some(package)=selected.voice_package { config.selection.as_mut().expect("selection was set")["voice_package"]=serde_json::to_value(package)?; }
        Ok(config)
    }
}

pub fn emit(event: &str, request: Option<&Value>, fields: Value) -> Result<()> {
    let mut record = fields.as_object().cloned().unwrap_or_default();
    record.insert("protocol".into(), PROTOCOL.into()); record.insert("event".into(), event.into());
    if let Some(request) = request {
        for key in ["request_id", "job_id", "revision"] { if let Some(value) = request.get(key) { record.insert(key.into(), value.clone()); } }
    }
    let _guard = OUTPUT.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|p| p.into_inner());
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &record)?; stdout.write_all(b"\n")?; stdout.flush()?;
    Ok(())
}

#[cfg(unix)]
unsafe extern "C" { fn kill(pid: i32, signal: i32) -> i32; fn signal(sig: i32, handler: usize) -> usize; }
#[cfg(unix)]
extern "C" fn interrupt(_: i32) { INTERRUPTED.store(true, Ordering::Release); }
fn signal_child(child: &mut Child, force: bool) {
    #[cfg(unix)] unsafe { kill(-(child.id() as i32), if force { 9 } else { 15 }); }
    #[cfg(not(unix))] { let _ = force; let _ = child.kill(); }
}
struct Active { job_id: String, cancel: Arc<AtomicBool>, child: Arc<Mutex<Child>>, join: thread::JoinHandle<()> }
struct State { active: Vec<Active>, finished: BTreeMap<String,String> }
struct Worker { config: Config, state: Arc<Mutex<State>> }
impl Worker {
    fn stop(&mut self) {
        let active = std::mem::take(&mut self.state.lock().unwrap().active);
        for job in &active { job.cancel.store(true, Ordering::Release); }
        for job in &active { signal_child(&mut job.child.lock().unwrap(), false); }
        for job in active { let _ = job.join.join(); }
    }
    fn reap(&self) {
        let mut state = self.state.lock().unwrap();
        let mut index = 0;
        while index < state.active.len() {
            if state.active[index].join.is_finished() {
                let active = state.active.swap_remove(index);
                drop(state);
                let _ = active.join.join();
                state = self.state.lock().unwrap();
            } else { index += 1; }
        }
    }
    fn handle(&mut self, request: &Value) -> Result<bool> {
        admission::request(request)?;
        match request["op"].as_str().unwrap() {
            "capabilities" => { emit("capabilities", Some(request), json!({"capabilities":capabilities::describe(&self.config)?}))?; }
            "admit_voice" => {
                let path = self.config.voice_registry.as_ref().context("admit_voice requires --voice-registry")?;
                let mut registry = verdant_backend::voices::VoiceRegistry::load_or_empty(path)?;
                let value = &request["assets"];
                let asset = |key:&str| -> Result<Option<PathBuf>> { value.get(key).map(|v|runtime::absolute(Path::new(v.as_str().context("voice asset path")?))).transpose() };
                let reference = request["reference"].as_str().unwrap();
                let assets = verdant_backend::voices::VoiceAssets {
                    voice_package: asset("voice_package")?, token: asset("token")?, model: asset("model")?, ..Default::default()
                };
                if reference == verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE {
                    ensure!(assets.voice_package.is_none() && assets.token.is_some() && assets.model.is_some(), "original admission requires the exact raw token and model");
                } else {
                    ensure!(assets.voice_package.is_some() && assets.token.is_none() && assets.model.is_none(), "voice admission requires a verdant-voice package");
                }
                let selected = registry.install_with(path,request["reference"].as_str().unwrap(),request["display_name"].as_str().unwrap(),assets,|assets| {
                    (||->Result<verdant_backend::voices::VoiceAdmission>{
                        let mut source_receipt=None;
                        let voice = if let Some(path)=&assets.voice_package {
                            let mut evidence:verdant_backend::voices::SourceEvidence=serde_json::from_value(request.get("source_evidence").cloned().context("verdant-voice import requires actual original token/binary and rights attestation")?)?;
                            evidence.token=runtime::absolute(&evidence.token)?;evidence.model=runtime::absolute(&evidence.model)?;
                            let before = evidence.verified_hashes()?;
                            let package=verdant_synthesis::model::load_voice_package_for_source_import(path, &evidence.token, &evidence.model)?;
                            let library=runtime::onnxruntime_path(&self.config)?;
                            let options=json!({"onnxruntime_library":library});
                            let mut imported=self.config.clone();
                            imported.original=None;imported.voice_package=Some(path.clone());
                            imported.bundled_voice=None;
                            imported.selection=None;imported.asset_sha256.clear();
                            runtime::validate_voice_package(&imported,&package,&options)?;
                            let after=verdant_backend::voices::verify_source_evidence(&evidence,package.source_manifest_sha256())?;
                            ensure!(before==after.original_source_sha256,"original source bytes changed during voice import");
                            source_receipt=Some(after);package.voice
                        } else {
                            let mut original = self.config.clone();
                            original.original = Some((assets.token.clone().context("token required")?, assets.model.clone().context("model required")?));
                            runtime::load_original(&original)?
                        };
                        Ok(verdant_backend::voices::VoiceAdmission { source_sha256:runtime::voice_sources(&voice)?,source_receipt,package_receipt:None })
                    })().map_err(|e|format!("{e:#}"))
                })?;
                emit("voice_admitted",Some(request),json!({"reference":selected.reference,"display_name":selected.display_name,"cache_identity":selected.cache_identity,"source_sha256":selected.source_sha256,"historical_identity_confirmed":selected.historical_identity_confirmed}))?;
            }
            "render" => {
                let mut config = self.config.selected(request)?;
                admission::render(request, &config)?;
                config.asset_sha256 = runtime::asset_hashes(&config)?;
                self.reap();
                let job_id = request["job_id"].as_str().unwrap().to_owned();
                let folder = config.workspace.join("jobs").join(&job_id);
                let mut state = self.state.lock().unwrap();
                if state.active.iter().filter(|active| !state.finished.contains_key(&active.job_id)).count() >= max_active_jobs() {
                    emit("rejected", Some(request), json!({"code":"BUSY","message":"Worker active-job limit reached; cancel/coalesce revisions in the client"}))?; return Ok(true);
                }
                if folder.exists() || state.finished.contains_key(&job_id) {
                    emit("rejected", Some(request), json!({"code":"DUPLICATE_JOB","message":"Use a fresh job_id; archived jobs are immutable"}))?; return Ok(true);
                }
                fs::create_dir_all(folder.parent().unwrap())?;
                ensure!(folder.parent().unwrap().canonicalize()?.starts_with(&config.workspace), "jobs directory escapes workspace");
                match fs::create_dir(&folder) {
                    Ok(()) => {},
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        emit("rejected", Some(request), json!({"code":"DUPLICATE_JOB","message":"Use a fresh job_id; archived jobs are immutable"}))?; return Ok(true);
                    }
                    Err(e) => return Err(e.into()),
                }
                artifacts::write_json(&folder.join("request.json"), request)?;
                config.request_sha256=artifacts::sha(&folder.join("request.json"))?;
                artifacts::write_json(&folder.join("worker-config.json"), &serde_json::to_value(&config)?)?;
                let executable=std::env::current_exe()?;
                let mut command = Command::new(&executable);
                command.current_dir(&config.project_root);
                if config.bundled_voice.is_some() {
                    // The child changes cwd; retain an absolute resource locator
                    // even when the parent received a relative VERDANT_DATA_DIR.
                    command.env("VERDANT_DATA_DIR",config.voice_package.as_ref().and_then(|path|path.parent())
                        .context("bundled voice resource directory is missing")?);
                }
                command.args(["--execute-job"]).arg(folder.join("request.json")).arg("--job-config").arg(folder.join("worker-config.json"));
                command.stdout(Stdio::piped()).stderr(Stdio::from(fs::File::create(folder.join("worker.stderr.log"))?));
                #[cfg(target_os="linux")]
                command.stdin(Stdio::null());
                #[cfg(not(target_os="linux"))]
                {
                    // The worker owns the only writer. EOF retires the render
                    // even if its parent is terminated without normal cleanup.
                    command.stdin(Stdio::piped()).env("VERDANT_JOB_PARENT_STDIN", "1");
                }
                #[cfg(unix)] {
                    use std::os::unix::process::CommandExt;
                    command.process_group(0);
                }
                #[cfg(target_os="linux")] {
                    use std::os::unix::process::CommandExt;
                    // A worker that dies abruptly (OOM, SIGKILL, panic abort) must
                    // not leave this render writing into an archived job directory.
                    // The child therefore asks the kernel for SIGTERM on parent
                    // death and rejects a spawn whose parent already vanished.
                    let owner = std::process::id();
                    unsafe {
                        command.pre_exec(move || {
                            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                                return Err(std::io::Error::last_os_error());
                            }
                            if libc::getppid() != owner as libc::pid_t {
                                return Err(std::io::Error::new(std::io::ErrorKind::Other, "worker exited before the job started"));
                            }
                            Ok(())
                        });
                    }
                }
                let mut child = match command.spawn() {
                    Ok(child) => child,
                    Err(error) => {
                        let fields = json!({"code":"RENDER_FAILED","message":error.to_string(),"job_directory":format!("jobs/{job_id}")});
                        let mut record = fields.clone(); record["protocol"] = PROTOCOL.into(); record["event"] = "failed".into();
                        record["job_id"] = job_id.clone().into(); record["revision"] = request["revision"].clone();
                        artifacts::write_json(&folder.join("worker-result.json"), &record)?;
                        state.finished.insert(job_id, "failed".into());
                        emit("accepted", Some(request), json!({"job_directory":record["job_directory"]}))?;
                        emit("failed", Some(request), fields)?; return Ok(true);
                    }
                };
                if let Err(error)=artifacts::write_json(&folder.join("commands.json"), &json!([{
                    "stage":"rust_job", "argv":[executable.display().to_string(), "--execute-job",folder.join("request.json").display().to_string(),"--job-config",folder.join("worker-config.json").display().to_string()],
                    "cwd":config.project_root,"pid":child.id()
                }])) {
                    // The job directory now exists, so the id is burned either way:
                    // still publish a terminal record instead of leaving it silent.
                    signal_child(&mut child,true); let _=child.wait();
                    let fields = json!({"code":"RENDER_FAILED","message":format!("job command record failed: {error:#}"),"job_directory":format!("jobs/{job_id}")});
                    let mut record = fields.clone(); record["protocol"] = PROTOCOL.into(); record["event"] = "failed".into();
                    record["job_id"] = job_id.clone().into(); record["revision"] = request["revision"].clone();
                    artifacts::write_json(&folder.join("worker-result.json"), &record)?;
                    state.finished.insert(job_id, "failed".into());
                    emit("accepted", Some(request), json!({"job_directory":record["job_directory"]}))?;
                    emit("failed", Some(request), fields)?; return Ok(true);
                }
                let stdout = child.stdout.take().context("worker execution stdout")?;
                let cancel = Arc::new(AtomicBool::new(false));
                let child = Arc::new(Mutex::new(child));
                let request_owned = request.clone(); let thread_state = self.state.clone();
                let thread_child = child.clone(); let thread_cancel = cancel.clone();
                if let Err(error) = emit("accepted", Some(request), json!({"job_directory":format!("jobs/{job_id}")})) {
                    let mut process = child.lock().unwrap();
                    signal_child(&mut process, true);
                    let _ = process.wait();
                    return Err(error);
                }
                let join = thread::spawn(move || {
                    let started = Instant::now();
                    let forward_request = request_owned.clone();
                    let forward = thread::spawn(move || {
                        let mut log = fs::File::create(folder.join("worker.stdout.log")).ok();
                        for line in BufReader::new(stdout).lines() {
                            let Ok(line) = line else { break; };
                            if let Some(log) = log.as_mut() { let _ = writeln!(log, "{line}"); }
                            if let Ok(mut event) = serde_json::from_str::<Value>(&line) {
                                if event["protocol"] == PROTOCOL && event["event"] == "progress" {
                                    if let Some(object) = event.as_object_mut() { object.remove("protocol"); object.remove("event"); }
                                    let _ = emit("progress", Some(&forward_request), event);
                                }
                            }
                        }
                        folder
                    });
                    let mut cancel_started = None;
                    let process_status = loop {
                        let mut process = thread_child.lock().unwrap();
                        match process.try_wait() {
                            Ok(Some(status)) => break Ok(status),
                            Err(error) => { signal_child(&mut process,true); let _=process.wait(); break Err(error); },
                            Ok(None) => {},
                        }
                        if thread_cancel.load(Ordering::Acquire) {
                            if cancel_started.is_none() { cancel_started = Some(Instant::now()); signal_child(&mut process, false); }
                            else if cancel_started.is_some_and(|t:Instant| t.elapsed() > Duration::from_secs(2)) { signal_child(&mut process, true); }
                        }
                        drop(process); thread::sleep(Duration::from_millis(50));
                    };
                    // Descendants can keep stdout open after their process-group
                    // leader exits. Keep cancellation escalation alive until the
                    // forwarding thread has drained and can be joined as well.
                    while !forward.is_finished() {
                        if thread_cancel.load(Ordering::Acquire) {
                            let mut process = thread_child.lock().unwrap();
                            if cancel_started.is_none() { cancel_started = Some(Instant::now()); signal_child(&mut process, false); }
                            else if cancel_started.is_some_and(|t:Instant| t.elapsed() > Duration::from_secs(2)) { signal_child(&mut process, true); }
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    if thread_cancel.load(Ordering::Acquire) { signal_child(&mut thread_child.lock().unwrap(), true); }
                    let folder = match forward.join() { Ok(folder) => folder, Err(_) => config.workspace.join("jobs").join(request_owned["job_id"].as_str().unwrap()) };
                    let cancelled = thread_cancel.load(Ordering::Acquire);
                    // A cancel that lands after the child already exited successfully
                    // must not relabel verified work as a partial cancellation: the
                    // artifact graph below is re-read and re-hashed regardless.
                    let verified = if process_status.as_ref().is_ok_and(|status| status.success()) {
                        (|| -> anyhow::Result<Value> {
                            let result: Value = serde_json::from_slice(&fs::read(folder.join("execution-result.json"))?)?;
                            ensure!(result["status"] == "completed", "execution completion marker is invalid");
                            let mut result = result;
                            result["artifacts"] = serde_json::to_value(artifacts::collect(&folder, &config.workspace)?)?;
                            result["elapsed_seconds"] = started.elapsed().as_secs_f64().into();
                            result["job_directory"] = format!("jobs/{}",request_owned["job_id"].as_str().unwrap()).into();
                            if let Some(object) = result.as_object_mut() { object.remove("status"); }
                            Ok(result)
                        })().ok()
                    } else { None };
                    let (status, fields) = match (cancelled, verified) {
                        (false, Some(fields)) => ("completed", fields),
                        (false, None) => {
                            let error = match process_status.map_err(anyhow::Error::from) {
                                Ok(status) => anyhow::anyhow!("Rust synthesis process exited {status}; see worker.stderr.log"),
                                Err(error) => error,
                            };
                            ("failed",json!({"code":"RENDER_FAILED","message":format!("{error:#}"),"job_directory":format!("jobs/{}",request_owned["job_id"].as_str().unwrap())}))
                        }
                        (true, Some(fields)) => ("completed", fields),
                        (true, None) => ("cancelled", json!({"partial_outputs_valid":false,"job_directory":format!("jobs/{}",request_owned["job_id"].as_str().unwrap())})),
                    };
                    let mut terminal = fields.clone(); terminal["protocol"] = PROTOCOL.into(); terminal["event"] = status.into();
                    terminal["job_id"] = request_owned["job_id"].clone(); terminal["revision"] = request_owned["revision"].clone();
                    let publication = artifacts::write_json(&folder.join("worker-result.json"), &terminal);
                    let (status,fields) = match publication {
                        Ok(()) => (status,fields),
                        Err(e) => ("failed",json!({"code":"RENDER_FAILED","message":format!("terminal publication failed: {e:#}"),"job_directory":terminal["job_directory"]})),
                    };
                    // Capacity is released under the same lock used by submission,
                    // before the single terminal event becomes visible to the client.
                    thread_state.lock().unwrap().finished.insert(request_owned["job_id"].as_str().unwrap().to_owned(), status.into());
                    let _ = emit(status, Some(&request_owned), fields);
                });
                state.active.push(Active { job_id, cancel, child, join });
            }
            "cancel" => {
                let state = self.state.lock().unwrap();
                let job_id = request["job_id"].as_str().unwrap();
                let active = state.active.iter().find(|active| active.job_id == job_id && !state.finished.contains_key(job_id));
                let matches = active.is_some();
                if let Some(active) = active { active.cancel.store(true,Ordering::Release); signal_child(&mut active.child.lock().unwrap(), false); }
                emit("cancel_ack",Some(request),json!({"active_match":matches,"previous_terminal":state.finished.get(job_id)}))?;
            }
            "shutdown" => { self.stop(); emit("shutdown",Some(request),json!({}))?; return Ok(false); }
            _ => unreachable!(),
        }
        Ok(true)
    }
}
impl Drop for Worker { fn drop(&mut self) { self.stop(); } }
fn change_default_voice(config:&Config,reference:&str,managed_upgrade:bool)->Result<()> {
    let path=config.voice_registry.as_ref().context("default selection requires --voice-registry")?;
    let registry_before=fs::read(path)?;
    let mut registry=verdant_backend::voices::VoiceRegistry::load(path)?;
    let before=registry.selection(reference)?;
    let route=if reference==verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE{"C0"}else{"verdant-voice"};
    let selected=config.selected(&json!({"voice_ref":reference,"route":route}))?;
    if selected.original.is_none() {
        let package=runtime::admitted_voice_package(&selected)?;
        let options=json!({"onnxruntime_library":runtime::onnxruntime_path(&selected)?});
        runtime::validate_voice_package(&selected,&package,&options)?;
    } else {runtime::load_original(&selected)?;}
    ensure!(registry.selection(reference)?.source_sha256==before.source_sha256,"voice assets changed during default selection");
    ensure!(fs::read(path)?==registry_before,"registry changed during default validation; retry against the current selection");
    let changed=if managed_upgrade{registry.upgrade_default(reference)?}else{registry.set_default(Some(reference))?;true};
    if changed{registry.save_atomic(path)?;}
    emit("default_voice_selected",None,json!({"requested_reference":reference,
        "default_voice":registry.default_reference(),"changed":changed,
        "preserved_explicit_default":managed_upgrade&&!changed,"track_selections_changed":false}))
}
fn main() {
    if let Err(error) = run() { eprintln!("verdant-worker: {error:#}"); std::process::exit(1); }
}
fn run() -> Result<()> {
    let (config, execute, job_config, default_voice) = Config::parse()?;
    if let Some(request_path) = execute {
        #[cfg(not(target_os="linux"))]
        if std::env::var_os("VERDANT_JOB_PARENT_STDIN").as_deref() == Some(std::ffi::OsStr::new("1")) {
            thread::Builder::new().name("verdant-job-parent".into()).spawn(|| {
                let mut byte = [0; 1];
                loop {
                    match io::stdin().read(&mut byte) {
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Ok(0) | Err(_) => std::process::exit(1),
                        Ok(_) => {},
                    }
                }
            }).context("starting render parent monitor")?;
        }
        let mut config: Config = serde_json::from_slice(&fs::read(job_config.context("--execute-job requires --job-config")?)?)?;
        bundled_voice::restore_child(&mut config)?;
        let request: Value = serde_json::from_slice(&fs::read(&request_path)?)?;
        return pipeline::execute(&config, &request, request_path.parent().context("job directory")?);
    }
    ensure!(job_config.is_none(), "--job-config is internal to --execute-job");
    if let Some((reference,managed_upgrade))=default_voice {
        return change_default_voice(&config,&reference,managed_upgrade);
    }
    #[cfg(unix)] unsafe { signal(15, interrupt as *const () as usize); signal(2, interrupt as *const () as usize); }
    let mut worker = Worker { config, state:Arc::new(Mutex::new(State { active:Vec::with_capacity(max_active_jobs()),finished:BTreeMap::new() })) };
    emit("ready", None, json!({"session_id":uuid::Uuid::new_v4().to_string(),"workspace":worker.config.workspace}))?;
    let (tx,rx) = mpsc::sync_channel(2);
    thread::spawn(move || {
        let mut reader = BufReader::new(io::stdin());
        loop {
            let mut line = Vec::new();
            let result = (&mut reader).take(MAX_REQUEST_BYTES as u64+1).read_until(b'\n', &mut line);
            let closed = matches!(result,Ok(0)) || result.is_err() || line.len()>MAX_REQUEST_BYTES;
            if tx.send(result.map(|_|line)).is_err() || closed { break; }
        }
    });
    while !INTERRUPTED.load(Ordering::Acquire) {
        worker.reap();
        let line = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => result?,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if line.is_empty() { break; }
        if line.len()>MAX_REQUEST_BYTES { emit("rejected",None,json!({"code":"REQUEST_TOO_LARGE","message":"Maximum request is 8 MiB; closing connection"}))?; break; }
        let parsed = serde_json::from_slice::<Value>(&line);
        match parsed {
            Ok(request) => match worker.handle(&request) {
                Ok(true) => {}, Ok(false) => break,
                Err(error) => emit("rejected",Some(&request),json!({"code":"INVALID_REQUEST","message":format!("{error:#}")}))?,
            },
            Err(error) => emit("rejected",None,json!({"code":"INVALID_REQUEST","message":error.to_string()}))?,
        }
    }
    worker.stop();
    Ok(())
}
