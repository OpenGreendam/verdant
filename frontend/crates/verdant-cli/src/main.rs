mod catalog;
mod exchange;
mod integrations;
mod jobs;
mod pack;
mod project;
mod protocol;
mod render;

use std::{env, fs::File, io::{self, Read, Write}, process::ExitCode, sync::atomic::{AtomicBool, Ordering}};

use serde_json::{Map, Value, json};

pub(crate) use protocol::CliError;
use protocol::{MAX_REQUEST_BYTES, PROTOCOL, Request, Response, read_request};

type Outcome = Result<Value, CliError>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Text, Json, Events }

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn interrupted() -> bool { INTERRUPTED.load(Ordering::Relaxed) }

#[cfg(unix)]
fn install_interrupt_handler() -> io::Result<()> {
    extern "C" fn interrupt(_: libc::c_int) { INTERRUPTED.store(true, Ordering::Relaxed); }
    // A signal handler writes a lock-free flag; the job supervisor owns all I/O.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = interrupt as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGINT, libc::SIGTERM] {
            if libc::sigaction(signal, &action, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn install_interrupt_handler() -> io::Result<()> {
    #[link(name = "Kernel32")]
    unsafe extern "system" { fn SetConsoleCtrlHandler(handler: extern "system" fn(u32) -> i32, add: i32) -> i32; }
    extern "system" fn interrupt(event: u32) -> i32 {
        if matches!(event, 0 | 1) { INTERRUPTED.store(true, Ordering::Relaxed); 1 } else { 0 }
    }
    if unsafe { SetConsoleCtrlHandler(interrupt, 1) } == 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn install_interrupt_handler() -> io::Result<()> { Ok(()) }

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("verdant-cli: {error}");
            ExitCode::from(5)
        }
    }
}

fn run() -> io::Result<u8> {
    install_interrupt_handler()?;
    let mut stdout = io::stdout().lock();
    let args = match env::args_os().skip(1).map(|arg| arg.into_string().map_err(|_| invalid("arguments must be UTF-8"))).collect::<Result<Vec<_>, _>>() {
        Ok(args) => args,
        Err(error) => return write_response(&mut stdout, None, Err(error), Mode::Text),
    };
    if args.is_empty() || args.iter().any(|arg| matches!(arg.as_str(), "--help" | "-h")) {
        stdout.write_all(include_bytes!("help.txt"))?;
        return Ok(0);
    }
    if args == ["--version"] {
        writeln!(stdout, "verdant-cli {}", env!("CARGO_PKG_VERSION"))?;
        return Ok(0);
    }
    if args == ["--runtime-features"] {
        json_line(&mut stdout, &catalog::runtime_features())?;
        return Ok(0);
    }
    if args == ["serve", "--stdio"] {
        let mut stdin = io::stdin().lock();
        let mut bytes = Vec::new();
        loop {
            let count = read_request(&mut stdin, &mut bytes)?;
            if count == 0 { break; }
            let oversized = count > MAX_REQUEST_BYTES + 1;
            if bytes.last() == Some(&b'\n') { bytes.pop(); }
            execute_request(&mut stdout, Request::parse(&bytes), Mode::Events)?;
            if interrupted() { return Ok(130); }
            if oversized { break; }
        }
        return Ok(0);
    }
    let (args, mode) = match output_mode(args) {
        Ok(output) => output,
        Err(error) => return write_response(&mut stdout, None, Err(error), Mode::Json),
    };
    let request = if args == ["request"] {
        let mut bytes = Vec::new();
        io::stdin().lock().take(MAX_REQUEST_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        Request::parse(&bytes)
    } else {
        parse_command(args)
    };
    execute_request(&mut stdout, request, mode)
}

fn execute_request(writer: &mut impl Write, request: Result<Request, CliError>, mode: Mode) -> io::Result<u8> {
    let Request { request_id, op, params, .. } = match request {
        Ok(request) => request,
        Err(error) => return write_response(writer, None, Err(error), mode),
    };
    let mut sequence = 0u64;
    let mut output_error = None;
    let outcome = dispatch(&op, params, &mut |data| {
        if mode == Mode::Events && output_error.is_none() {
            sequence += 1;
            let event = json!({"protocol":PROTOCOL,"request_id":request_id,"sequence":sequence,
                "event":"progress","job_id":data.get("job_id"),"data":data});
            output_error = json_line(writer, &event).err();
            if output_error.is_some() { INTERRUPTED.store(true, Ordering::Relaxed); }
        }
    });
    if let Some(error) = output_error { return Err(error); }
    if mode == Mode::Events {
        let response = Response::new(Some(&request_id), outcome);
        let code = response.exit_code();
        let mut event = serde_json::to_value(response)?;
        event["event"] = json!("result");
        event["sequence"] = json!(sequence + 1);
        if let Some(id) = event.pointer("/result/job_id").or_else(|| event.pointer("/error/details/job_id")).cloned() {
            event["job_id"] = id;
        }
        json_line(writer, &event)?;
        Ok(code)
    } else {
        write_response(writer, Some(&request_id), outcome, mode)
    }
}

fn dispatch(op: &str, params: Value, progress: &mut dyn FnMut(Value)) -> Outcome {
    match op {
        "capabilities" => catalog::capabilities(params),
        "schema" => catalog::schema(params),
        "doctor" => render::doctor(params),
        "render" | "render.plan" | "export.audio" => render::dispatch(op, params, progress),
        "project.pack" | "project.unpack" => pack::dispatch(op, params).expect("portable project dispatch"),
        "project.save-as" => project::dispatch("project.save_as", params).expect("project save_as dispatch"),
        "import" | "export.project" => exchange::dispatch(op, params).expect("exchange dispatch"),
        "batch" => batch(params, progress),
        _ if op.starts_with("job.") => render::job(op, params, progress),
        _ if op.starts_with("cache.") => render::cache(op, params),
        _ if op.starts_with("voice.") => voice(op, params),
        _ if op.starts_with("media.") || op.starts_with("effect.") => integrations::dispatch(op, params)
            .unwrap_or_else(|| Err(invalid(format!("unknown operation {op}")))),
        _ => project::dispatch(op, params).unwrap_or_else(|| Err(invalid(format!("unknown operation {op}")))),
    }
}

fn batch(params: Value, progress: &mut dyn FnMut(Value)) -> Outcome {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Batch { requests: Vec<Request>, #[serde(default)] stop_on_error: bool }
    let batch: Batch = serde_json::from_value(params).map_err(|error| invalid(error.to_string()))?;
    if batch.requests.is_empty() || batch.requests.len() > 256 {
        return Err(invalid("batch requires 1–256 requests"));
    }
    let mut requests = Vec::with_capacity(batch.requests.len());
    for request in batch.requests {
        if request.op == "batch" { return Err(invalid("batches cannot be nested")); }
        requests.push(request.validate()?);
    }
    let total = requests.len();
    let mut results = Vec::with_capacity(total);
    let mut failure = 0;
    for Request { request_id, op, params, .. } in &requests {
        let outcome = dispatch(op, params.clone(), &mut |data| progress(json!({"item_request_id":request_id,"data":data})));
        let response = Response::new(Some(request_id), outcome);
        if response.exit_code() != 0 { failure += 1; }
        results.push(serde_json::to_value(response).map_err(internal)?);
        if batch.stop_on_error && failure != 0 { break; }
    }
    let skipped = total - results.len();
    let report = json!({"results":results,"failed":failure,"skipped":skipped});
    if failure != 0 {
        return Err(CliError::new("BATCH_FAILED", format!("{failure} batch item(s) failed")).with_details(report));
    }
    Ok(report)
}

fn voice(op: &str, mut params: Value) -> Outcome {
    if op == "voice.verify" {
        return render::voice_query(op, params);
    }
    let verb = match op {
        "voice.convert" => "convert", "voice.install" => "install", "voice.list" => "list", "voice.remove" => "remove",
        "voice.set-default" => "set-default", "voice.import-originals" => "import-originals",
        _ => return Err(invalid(format!("unknown voice operation {op}"))),
    };
    let object = params.as_object_mut().ok_or_else(|| invalid("params must be an object"))?;
    const FLAGS: &[&str] = &[
        "source", "registry", "destination", "source_token", "source_model", "token", "model", "set_default",
        "artifact_policy", "artifact_policy_sha256",
    ];
    let mut args = vec![std::ffi::OsString::from(verb)];
    let positional = match verb { "install" => Some("package"), "remove" | "set-default" => Some("reference"), _ => None };
    if let Some(name) = positional {
        let value = object.remove(name).and_then(|value| value.as_str().map(str::to_owned))
            .filter(|value| !value.is_empty()).ok_or_else(|| invalid(format!("{name} must be a nonempty string")))?;
        args.push(value.into());
    }
    for (name, value) in std::mem::take(object) {
        if !FLAGS.contains(&name.as_str()) { return Err(invalid(format!("unknown voice option {name}"))); }
        let flag = format!("--{}", name.replace('_', "-"));
        match value {
            Value::Bool(true) => args.push(flag.into()),
            Value::Bool(false) => {},
            Value::String(text) => args.extend([flag.into(), text.into()]),
            _ => return Err(invalid(format!("invalid value for {name}"))),
        }
    }
    verdant_voice::execute(args).map_err(|error| CliError::new("VOICE_NOT_ADMITTED", error.to_string()))
}

fn output_mode(args: Vec<String>) -> Result<(Vec<String>, Mode), CliError> {
    let mut mode = Mode::Text;
    let mut rest = Vec::with_capacity(args.len());
    let mut positional_only = false;
    for arg in args {
        if arg == "--" { positional_only = true; }
        let requested = match arg.as_str() { "--json" => Mode::Json, "--events" => Mode::Events, _ => Mode::Text };
        if !positional_only && requested != Mode::Text {
            if mode != Mode::Text { return Err(invalid("choose only one of --json or --events")); }
            mode = requested;
        } else {
            rest.push(arg);
        }
    }
    Ok((rest, mode))
}

fn parse_command(mut args: Vec<String>) -> Result<Request, CliError> {
    if args.is_empty() { return Err(invalid("operation is required")); }
    let mut op = args.remove(0);
    if catalog::FAMILIES.contains(&op.as_str()) {
        if args.first().is_none_or(|arg| arg.starts_with('-')) { return Err(invalid(format!("{op} requires an operation"))); }
        op.push('.');
        op.push_str(&args.remove(0));
    }
    let mut params = Map::new();
    let mut positional = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--" { positional.extend(args); break; }
        let Some(option) = arg.strip_prefix("--") else { positional.push(arg); continue; };
        let (flag, inline) = option.split_once('=').map_or((option, None), |(key, value)| (key, Some(value.to_owned())));
        let key = flag.replace('-', "_");
        let value = if matches!(key.as_str(), "dry_run"|"overwrite"|"force"|"strict"|"stop_on_error"|"allow_external_media"|"allow_loss"|"no_effects"|"attest_source_use") && inline.is_none() {
            json!(true)
        } else {
            let text = inline.or_else(|| args.next()).ok_or_else(|| invalid(format!("--{flag} requires a value")))?;
            match key.as_str() {
                "params" | "operations" | "requests" | "project" | "options" => read_json_file(&text)?,
                "paths" | "asset_roots" | "asset_paths" | "owner" | "expect_revision" | "sample_rate" | "channels" | "start_tick" | "end_tick"
                | "seed" | "timeout_seconds" | "limit" | "ppq" | "max_duration_seconds" | "max_archive_bytes" | "max_expanded_bytes" =>
                    serde_json::from_str(&text).map_err(|error| invalid(format!("--{flag}: {error}")))?,
                "effects" | "overwrite" | "force" | "strict" | "dry_run" | "allow_loss" | "allow_external_media" =>
                    serde_json::from_str::<bool>(&text).map(Value::Bool).map_err(|error| invalid(format!("--{flag}: {error}")))?,
                _ => Value::String(text),
            }
        };
        if key == "params" {
            let Value::Object(fields) = value else { return Err(invalid("--params requires a JSON object")); };
            for (key, value) in fields { field(&mut params, key, value)?; }
        } else if key == "no_effects" {
            field(&mut params, "effects".into(), Value::Bool(false))?;
        } else {
            field(&mut params, key, value)?;
        }
    }
    let names = catalog::positionals(&op);
    if positional.len() > names.len() { return Err(invalid(format!("unexpected positional argument for {op}"))); }
    for (name, value) in names.iter().zip(positional) { field(&mut params, (*name).into(), Value::String(value))?; }
    Ok(Request { protocol: PROTOCOL.into(), request_id: uuid::Uuid::new_v4().to_string(), op, params: Value::Object(params) })
}

fn read_json_file(path: &str) -> Outcome {
    let mut bytes = Vec::new();
    File::open(path).and_then(|file| file.take(MAX_REQUEST_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| CliError::new("IO_ERROR", format!("{path}: {error}")))?;
    if bytes.len() > MAX_REQUEST_BYTES { return Err(CliError::new("RESOURCE_EXHAUSTED", "JSON input exceeds 2 MiB")); }
    serde_json::from_slice(&bytes).map_err(|error| invalid(format!("{path}: {error}")))
}

fn field(params: &mut Map<String, Value>, key: String, value: Value) -> Result<(), CliError> {
    if params.contains_key(&key) { return Err(invalid(format!("duplicate parameter {key}"))); }
    params.insert(key, value);
    Ok(())
}

fn json_line(writer: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn write_response(writer: &mut impl Write, request_id: Option<&str>, outcome: Outcome, mode: Mode) -> io::Result<u8> {
    if mode == Mode::Text {
        return match outcome {
            Ok(value) => { serde_json::to_writer_pretty(&mut *writer, &value)?; writeln!(writer)?; Ok(0) }
            Err(error) => { eprintln!("verdant-cli: {error}"); Ok(error.exit_code()) }
        };
    }
    let response = Response::new(request_id, outcome);
    let code = response.exit_code();
    if mode == Mode::Events {
        let mut event = serde_json::to_value(response)?;
        event["event"] = json!("result");
        event["sequence"] = json!(1);
        json_line(writer, &event)?;
    } else {
        json_line(writer, &response)?;
    }
    Ok(code)
}

fn invalid(message: impl Into<String>) -> CliError { CliError::new("INVALID_ARGUMENT", message) }
fn internal(error: impl std::fmt::Display) -> CliError { CliError::new("INTERNAL_ERROR", error.to_string()) }
