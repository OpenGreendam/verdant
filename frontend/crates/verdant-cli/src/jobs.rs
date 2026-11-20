//! Durable receipts for one supervised render per workspace. The index lock
//! protects receipt transitions; a separate OS lease is held for the entire
//! invocation and its cleanup. PIDs are diagnostic data, never liveness tests.

use std::{
    fs::{self, File, TryLockError},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(not(unix))]
use std::fs::OpenOptions;

use serde::{Deserialize, Serialize, ser::{SerializeMap, SerializeSeq}};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{CliError, protocol::MAX_REQUEST_BYTES};

const SCHEMA: &str = "verdant-cli-job/1";
const MAX_RECEIPT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PROGRESS_BYTES: usize = 64 * 1024;
const MAX_ARTIFACTS: usize = 4096;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum State { Queued, Running, Cancelling, Succeeded, Failed, Cancelled, Interrupted }

impl State {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted)
    }
}

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct Idempotency { scope: String, key: String }

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Owner { generation: String, pid: u32 }

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Termination {
    reason: String,
    observed_at_ms: u64,
    // Only a supervisor-supplied observation may contain an exit status.
    process: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    bytes: u64,
    sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<ArtifactRole>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactRole { Audio, Report }

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    job_id: String,
    workspace: String,
    request_digest: String,
    fingerprint: String,
    idempotency: Option<Idempotency>,
    retry_of: Option<String>,
    input: Value,
    state: State,
    created_at_ms: u64,
    updated_at_ms: u64,
    started_at_ms: Option<u64>,
    owner: Option<Owner>,
    progress_sequence: u64,
    progress: Option<Value>,
    termination: Option<Termination>,
    error: Option<Value>,
    result: Option<Value>,
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CancelMarker {
    schema: String,
    job_id: String,
    request_digest: String,
    generation: String,
    requested_at_ms: u64,
}

#[derive(Clone)]
struct Store { workspace: PathBuf, root: PathBuf, workspace_text: String }

/// Hold this guard across cache mutations that must exclude a render. It uses
/// the same bounded workspace slot as `run`, rather than a PID or lockfile age.
pub struct WorkspaceLease { _file: File }

/// The runner must stop and reap its children before returning, including on
/// cancellation. Returning from the callback transfers terminal ownership here.
pub struct JobContext {
    pub id: String,
    pub request_digest: String,
    pub fingerprint: String,
    store: Store,
    generation: String,
}

impl JobContext {
    pub fn is_cancelled(&self) -> Result<bool, CliError> {
        let path = self.store.job_dir(&self.id).join("cancel.json");
        let Some(marker) = read_optional::<CancelMarker>(&path, MAX_PROGRESS_BYTES)? else { return Ok(false); };
        if marker.schema != SCHEMA || marker.job_id != self.id
            || marker.request_digest != self.request_digest || marker.generation != self.generation
        {
            return Err(invalid_receipt(&self.id, "cancellation marker belongs to another invocation"));
        }
        Ok(true)
    }

    pub fn check_cancelled(&self) -> Result<(), CliError> {
        if self.is_cancelled()? { Err(cancelled(&self.id)) } else { Ok(()) }
    }

    /// Persist actual runner progress. This is deliberately not an estimated
    /// scheduler, and progress never implies a successful terminal state.
    pub fn progress(&self, data: Value) -> Result<(), CliError> {
        check_size(&data, MAX_PROGRESS_BYTES)?;
        let _index = self.store.index_lock()?;
        let mut receipt = self.store.read(&self.id)?;
        if !matches!(receipt.state, State::Running | State::Cancelling)
            || receipt.owner.as_ref().is_none_or(|owner| owner.generation != self.generation)
        {
            return Err(invalid_receipt(&self.id, "progress came from an inactive invocation"));
        }
        receipt.progress_sequence = receipt.progress_sequence.checked_add(1)
            .ok_or_else(|| invalid_receipt(&self.id, "progress sequence overflow"))?;
        receipt.progress = Some(data);
        receipt.touch();
        self.store.save(&receipt)
    }
}

/// Execute an already-pinned request. `fingerprint` binds the project snapshot,
/// revision, admitted voice, media, effects and effective render configuration;
/// it must exclude destinations and job/idempotency control fields. The request
/// digest additionally binds every effective parameter and output destination.
pub fn run(
    params: &Value,
    fingerprint: &str,
    invoke: impl FnOnce(&JobContext) -> Result<Value, CliError>,
) -> Result<Value, CliError> {
    valid_text(fingerprint, "fingerprint", 4096)?;
    let controls = Controls::parse(params)?;
    let store = Store::open(params, true)?.ok_or_else(|| internal("job store was not created"))?;
    let input = normalized_input(params, &store)?;
    let digest = request_digest(&input, fingerprint)?;
    let index = store.index_lock()?;
    let mut existing = None;
    if let Some(id) = &controls.job_id {
        existing = Some(store.read(id)?);
    } else if let Some(identity) = &controls.idempotency {
        for id in store.ids()? {
            let mut receipt = store.read(&id)?;
            store.reconcile(&mut receipt)?;
            if receipt.idempotency.as_ref() == Some(identity) {
                if existing.is_some() {
                    return Err(invalid_receipt(&id, "idempotency identity has multiple receipts"));
                }
                existing = Some(receipt);
            }
        }
    }
    if let Some(receipt) = &mut existing {
        store.reconcile(receipt)?;
        if receipt.request_digest != digest || receipt.fingerprint != fingerprint
            || receipt.retry_of != controls.retry_of
            || controls.idempotency.as_ref().is_some_and(|identity| receipt.idempotency.as_ref() != Some(identity))
        {
            return Err(CliError::new("INVALID_ARGUMENT", "job or idempotency key is bound to a different pinned request")
                .with_details(json!({"job_id":receipt.job_id,"expected_request_digest":receipt.request_digest,
                    "received_request_digest":digest})));
        }
        if receipt.state.terminal() {
            let receipt = existing.take().expect("existing receipt");
            drop(index);
            return replay(&store, receipt);
        }
        if receipt.state != State::Queued {
            return Err(CliError::new("PROJECT_BUSY", "the original job still holds its execution lease")
                .with_details(json!({"job_id":receipt.job_id,"state":receipt.state})));
        }
    }
    let _slot = store.slot()?;
    let new_job = existing.is_none();
    let mut receipt = match existing {
        Some(receipt) => receipt,
        None => {
            let id = Uuid::new_v4().to_string();
            Receipt::new(&store, id, input, fingerprint.to_owned(), digest, controls.idempotency, None)
        }
    };
    let generation = Uuid::new_v4().to_string();
    receipt.state = State::Running;
    receipt.started_at_ms = Some(now_ms());
    receipt.owner = Some(Owner { generation: generation.clone(), pid: std::process::id() });
    receipt.touch();
    let _lease = if new_job { store.create(&receipt)? } else {
        let lease = store.try_job_lease(&receipt.job_id)?
            .ok_or_else(|| CliError::new("PROJECT_BUSY", "job execution lease is already held"))?;
        store.save(&receipt)?;
        lease
    };
    let context = JobContext { id: receipt.job_id.clone(), request_digest: receipt.request_digest.clone(),
        fingerprint: receipt.fingerprint.clone(), store: store.clone(), generation };
    drop(index);

    let outcome = context.check_cancelled().and_then(|()| invoke(&context)).and_then(|mut result| {
        bind_result(&receipt, &mut result)?;
        let artifacts = verify_outputs(&store, &receipt, &result)?;
        result["artifacts"] = serde_json::to_value(&artifacts).map_err(internal)?;
        Ok((result, artifacts))
    });
    let _index = store.index_lock()?;
    let mut current = store.read(&receipt.job_id)?;
    if !matches!(current.state, State::Running | State::Cancelling)
        || current.request_digest != receipt.request_digest
        || current.owner.as_ref().is_none_or(|owner| owner.generation != context.generation)
    {
        return Err(invalid_receipt(&receipt.job_id, "terminal event came from an inactive invocation"));
    }
    // Cancellation and successful publication share the same index transaction.
    // Once one terminal state commits, a late cancellation cannot replace it.
    let cancellation = context.is_cancelled();
    let outcome = match cancellation {
        Err(error) => Err(error),
        Ok(true) if outcome.is_ok() => Err(cancelled(&context.id)),
        _ => outcome,
    };
    match outcome {
        Ok((result, artifacts)) => {
            current.state = State::Succeeded;
            current.termination = Some(Termination { reason: "completed".into(), observed_at_ms: now_ms(),
                process: result.get("termination").cloned() });
            current.artifacts = artifacts;
            current.result = Some(result);
            current.touch();
            store.save(&current)?;
            Ok(current.result.take().expect("successful result"))
        }
        Err(error) => {
            let error = with_job(error, &context.id);
            current.fail(&error, if error.code == "CANCELLED" { State::Cancelled } else { State::Failed })?;
            store.save(&current)?;
            Err(error)
        }
    }
}

pub fn workspace_lease(params: &Value) -> Result<WorkspaceLease, CliError> {
    let store = Store::open(params, true)?.ok_or_else(|| internal("job store was not created"))?;
    let _index = store.index_lock()?;
    store.slot()
}

pub fn dispatch(op: &str, params: Value) -> Option<Result<Value, CliError>> {
    Some(match op {
        "job.list" => list(&params),
        "job.inspect" => inspect(&params, false),
        "job.verify" => inspect(&params, true),
        "job.cancel" => cancel(&params),
        "job.retry" => prepare_retry(&params),
        _ => return None,
    })
}

/// Reserve a distinct, unstarted job and return `{job_id,retry_of,params}`.
/// The dispatcher runs those params through normal preflight and `run`. Failed
/// preflight must call `fail_prepared`; no render is started by this function.
pub fn prepare_retry(params: &Value) -> Result<Value, CliError> {
    fields(params, &["workspace", "job_id", "output", "report"])?;
    let id = job_id(params)?;
    let store = existing_store(params)?;
    let _index = store.index_lock()?;
    let mut previous = store.read(id)?;
    store.reconcile(&mut previous)?;
    if !previous.state.terminal() {
        return Err(CliError::new("PROJECT_BUSY", "only a terminal job can be retried")
            .with_details(json!({"job_id":id,"state":previous.state})));
    }
    let new_id = Uuid::new_v4().to_string();
    let mut input = previous.input.clone();
    for field in ["output", "report"] {
        if let Some(old) = input.get(field).and_then(Value::as_str) {
            let destination = retry_destination(old, params.get(field), &new_id)?;
            for old_field in ["path", "output", "report"] {
                if previous.input.get(old_field).and_then(Value::as_str) == Some(destination.as_str()) {
                    return Err(invalid("retry destination must differ from the original project and outputs"));
                }
            }
            input[field] = Value::String(destination);
        } else if params.get(field).is_some() {
            return Err(invalid(format!("the original job has no {field} destination")));
        }
    }
    if input.get("output").is_some() && input.get("output") == input.get("report") {
        return Err(invalid("retry output and report must have different destinations"));
    }
    input["overwrite"] = Value::Bool(false);
    check_size(&input, MAX_REQUEST_BYTES)?;
    let digest = request_digest(&input, &previous.fingerprint)?;
    let receipt = Receipt::new(&store, new_id.clone(), input, previous.fingerprint, digest, None, Some(id.to_owned()));
    let _lease = store.create(&receipt)?;
    let mut request = receipt.input;
    request["job_id"] = json!(new_id);
    request["retry_of"] = json!(id);
    Ok(json!({"job_id":new_id,"retry_of":id,"state":"queued","prepared":true,"params":request}))
}

pub fn fail_prepared(params: &Value, error: &CliError) -> Result<(), CliError> {
    let controls = Controls::parse(params)?;
    let id = controls.job_id.as_deref().ok_or_else(|| invalid("prepared job_id is required"))?;
    let store = existing_store(params)?;
    let _index = store.index_lock()?;
    let mut receipt = store.read(id)?;
    let input = normalized_input(params, &store)?;
    if receipt.request_digest != request_digest(&input, &receipt.fingerprint)? || receipt.retry_of != controls.retry_of {
        return Err(invalid_receipt(id, "preflight failure belongs to a different prepared request"));
    }
    store.reconcile(&mut receipt)?;
    if receipt.state.terminal() { return Ok(()); }
    if receipt.state != State::Queued || receipt.owner.is_some() {
        return Err(CliError::new("PROJECT_BUSY", "prepared job has already started").with_details(json!({"job_id":id})));
    }
    let _lease = store.try_job_lease(id)?
        .ok_or_else(|| CliError::new("PROJECT_BUSY", "prepared job execution lease is held"))?;
    receipt.fail(error, State::Failed)?;
    store.save(&receipt)
}

fn list(params: &Value) -> Result<Value, CliError> {
    fields(params, &["workspace", "limit"])?;
    let limit = match params.get("limit") {
        None => 100,
        Some(value) => value.as_u64().filter(|limit| (1..=1000).contains(limit))
            .ok_or_else(|| invalid("limit must be an integer from 1 to 1000"))? as usize,
    };
    let Some(store) = Store::open(params, false)? else { return Ok(json!({"jobs":[],"total":0})); };
    let _index = store.index_lock()?;
    let mut receipts = Vec::new();
    for id in store.ids()? {
        let mut receipt = store.read(&id)?;
        store.reconcile(&mut receipt)?;
        receipts.push(receipt);
    }
    receipts.sort_unstable_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms).then_with(|| a.job_id.cmp(&b.job_id)));
    let total = receipts.len();
    let jobs: Vec<Value> = receipts.into_iter().take(limit).map(|receipt| json!({
        "job_id":receipt.job_id,"state":receipt.state,"retry_of":receipt.retry_of,
        "created_at_ms":receipt.created_at_ms,"updated_at_ms":receipt.updated_at_ms,
        "request_digest":receipt.request_digest,"fingerprint":receipt.fingerprint,
        "progress_sequence":receipt.progress_sequence,"progress":receipt.progress,"error":receipt.error,
        "artifacts_verified":false,
    })).collect();
    Ok(json!({"jobs":jobs,"total":total}))
}

fn inspect(params: &Value, require_success: bool) -> Result<Value, CliError> {
    fields(params, &["workspace", "job_id"])?;
    let id = job_id(params)?;
    let store = existing_store(params)?;
    let index = store.index_lock()?;
    let mut receipt = store.read(id)?;
    store.reconcile(&mut receipt)?;
    drop(index);
    if require_success && receipt.state != State::Succeeded {
        return Err(invalid("job has no successful output graph to verify")
            .with_details(json!({"job_id":id,"state":receipt.state})));
    }
    let verified = receipt.state == State::Succeeded;
    if let Some(result) = receipt.result.as_ref().filter(|_| verified) {
        let artifacts = verify_outputs(&store, &receipt, result)?;
        if artifacts != receipt.artifacts { return Err(invalid_receipt(id, "verified artifacts differ from the receipt")); }
    }
    let mut value = serde_json::to_value(receipt).map_err(internal)?;
    value["artifacts_verified"] = json!(verified);
    Ok(value)
}

fn cancel(params: &Value) -> Result<Value, CliError> {
    fields(params, &["workspace", "job_id"])?;
    let id = job_id(params)?;
    let store = existing_store(params)?;
    let _index = store.index_lock()?;
    let mut receipt = store.read(id)?;
    store.reconcile(&mut receipt)?;
    if receipt.state.terminal() {
        return Ok(json!({"job_id":id,"state":receipt.state,"cancel_requested":false,"terminal":true}));
    }
    if receipt.state == State::Queued {
        let _lease = store.try_job_lease(id)?
            .ok_or_else(|| CliError::new("PROJECT_BUSY", "queued job execution lease is held"))?;
        receipt.fail(&cancelled(id), State::Cancelled)?;
        store.save(&receipt)?;
        return Ok(json!({"job_id":id,"state":"cancelled","cancel_requested":true,"terminal":true}));
    }
    let owner = receipt.owner.as_ref().ok_or_else(|| invalid_receipt(id, "running job has no lease generation"))?;
    let marker = CancelMarker { schema: SCHEMA.into(), job_id: id.to_owned(), request_digest: receipt.request_digest.clone(),
        generation: owner.generation.clone(), requested_at_ms: now_ms() };
    let marker_path = store.job_dir(id).join("cancel.json");
    if let Some(old) = read_optional::<CancelMarker>(&marker_path, MAX_PROGRESS_BYTES)? {
        if old.schema != marker.schema || old.job_id != marker.job_id
            || old.request_digest != marker.request_digest || old.generation != marker.generation
        {
            return Err(invalid_receipt(id, "cancellation marker belongs to another invocation"));
        }
    } else {
        atomic_json(&marker_path, &marker)?;
    }
    receipt.state = State::Cancelling;
    receipt.touch();
    store.save(&receipt)?;
    Ok(json!({"job_id":id,"state":"cancelling","cancel_requested":true,"terminal":false}))
}

impl Receipt {
    fn new(store: &Store, id: String, input: Value, fingerprint: String, digest: String,
        idempotency: Option<Idempotency>, retry_of: Option<String>) -> Self
    {
        let created = now_ms();
        Self { schema: SCHEMA.into(), job_id: id, workspace: store.workspace_text.clone(), request_digest: digest,
            fingerprint, idempotency, retry_of, input, state: State::Queued, created_at_ms: created, updated_at_ms: created,
            started_at_ms: None, owner: None, progress_sequence: 0, progress: None, termination: None, error: None,
            result: None, artifacts: Vec::new() }
    }

    fn touch(&mut self) { self.updated_at_ms = self.updated_at_ms.max(now_ms()); }

    fn fail(&mut self, error: &CliError, state: State) -> Result<(), CliError> {
        self.state = state;
        self.termination = Some(Termination { reason: match state {
            State::Cancelled => "cancelled", State::Interrupted => "lease_released_without_terminal", _ => "render_failed",
        }.into(), observed_at_ms: now_ms(), process: error.details.as_ref().and_then(|details| details.get("termination")).cloned() });
        self.error = Some(serde_json::to_value(error).map_err(internal)?);
        self.result = None;
        self.artifacts.clear();
        self.touch();
        Ok(())
    }
}

impl Store {
    fn open(params: &Value, create: bool) -> Result<Option<Self>, CliError> {
        let text = params.get("workspace").and_then(Value::as_str).ok_or_else(|| invalid("workspace is required"))?;
        valid_text(text, "workspace", 32 * 1024)?;
        if create { fs::create_dir_all(text).map_err(io_error)?; }
        let workspace = fs::canonicalize(text).map_err(io_error)?;
        if !workspace.is_dir() { return Err(invalid("workspace must be a directory")); }
        let workspace_text = utf8_path(&workspace)?;
        let parent = workspace.join(".verdant-cli");
        let root = parent.join("jobs");
        for path in [&parent, &root] {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {},
                Ok(_) => return Err(invalid("job store must contain real directories, without symlinks")),
                Err(error) if error.kind() == io::ErrorKind::NotFound && create => {
                    match fs::create_dir(path) {
                        Ok(()) => sync_directory(path.parent().expect("store directory parent"))?,
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                            let metadata = fs::symlink_metadata(path).map_err(io_error)?;
                            if !metadata.is_dir() || metadata.file_type().is_symlink() { return Err(invalid("unsafe job store directory")); }
                        }
                        Err(error) => return Err(io_error(error)),
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(io_error(error)),
            }
        }
        Ok(Some(Self { workspace, root, workspace_text }))
    }

    fn job_dir(&self, id: &str) -> PathBuf { self.root.join(id) }

    fn index_lock(&self) -> Result<File, CliError> {
        let file = open_lock(&self.root.join("index.lock"))?;
        file.lock().map_err(lock_error)?;
        Ok(file)
    }

    fn slot(&self) -> Result<WorkspaceLease, CliError> {
        let file = open_lock(&self.root.join("workspace.lock"))?;
        if !try_lock(&file)? {
            return Err(CliError::new("RESOURCE_EXHAUSTED", "workspace render/cache budget is already in use")
                .with_details(json!({"workspace":self.workspace_text,"max_active_jobs":1})));
        }
        Ok(WorkspaceLease { _file: file })
    }

    fn try_job_lease(&self, id: &str) -> Result<Option<File>, CliError> {
        validate_id(id)?;
        let file = open_lock(&self.job_dir(id).join("lease.lock"))?;
        if try_lock(&file)? { Ok(Some(file)) } else { Ok(None) }
    }

    fn ids(&self) -> Result<Vec<String>, CliError> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let name = entry.file_name().into_string().map_err(|_| invalid("job store entry is not UTF-8"))?;
            // A crash before the directory rename has not published a job ID.
            if matches!(name.as_str(), "index.lock" | "workspace.lock") || name.starts_with(".prepare-") { continue; }
            validate_id(&name).map_err(|_| invalid_receipt(&name, "unexpected job store entry"))?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(io_error)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(invalid_receipt(&name, "job directory must not be a symlink"));
            }
            ids.push(name);
        }
        ids.sort_unstable();
        Ok(ids)
    }

    fn read(&self, id: &str) -> Result<Receipt, CliError> {
        validate_id(id)?;
        let receipt: Receipt = read_required(&self.job_dir(id).join("receipt.json"), MAX_RECEIPT_BYTES)?;
        self.validate(&receipt, id)?;
        Ok(receipt)
    }

    fn validate(&self, receipt: &Receipt, id: &str) -> Result<(), CliError> {
        let bad = |message| invalid_receipt(id, message);
        if receipt.schema != SCHEMA || receipt.job_id != id || receipt.workspace != self.workspace_text
            || !receipt.input.is_object() || receipt.input.get("workspace").and_then(Value::as_str) != Some(self.workspace_text.as_str())
            || receipt.input.as_object().is_some_and(|input| ["job_id", "retry_of", "idempotency_key", "idempotency_scope"].iter().any(|key| input.contains_key(*key)))
            || receipt.updated_at_ms < receipt.created_at_ms
        { return Err(bad("receipt identity or input does not match its workspace")); }
        valid_text(&receipt.fingerprint, "receipt fingerprint", 4096).map_err(|_| bad("invalid pinned fingerprint"))?;
        if request_digest(&receipt.input, &receipt.fingerprint)? != receipt.request_digest {
            return Err(bad("receipt does not match its canonical pinned request digest"));
        }
        if let Some(identity) = &receipt.idempotency {
            valid_text(&identity.key, "idempotency_key", 256).map_err(|_| bad("invalid stored idempotency key"))?;
            valid_text(&identity.scope, "idempotency_scope", 32 * 1024).map_err(|_| bad("invalid stored idempotency scope"))?;
        }
        if let Some(previous) = &receipt.retry_of {
            validate_id(previous).map_err(|_| bad("invalid retry lineage"))?;
            if previous == id { return Err(bad("job cannot retry itself")); }
        }
        if let Some(owner) = &receipt.owner { validate_id(&owner.generation).map_err(|_| bad("invalid execution generation"))?; }
        if receipt.owner.is_some() != receipt.started_at_ms.is_some() {
            return Err(bad("execution owner and start time disagree"));
        }
        match receipt.state {
            State::Queued if receipt.owner.is_some() || receipt.termination.is_some() || receipt.error.is_some()
                || receipt.result.is_some() || !receipt.artifacts.is_empty() => return Err(bad("queued receipt contains execution results")),
            State::Running | State::Cancelling if receipt.owner.is_none() || receipt.termination.is_some()
                || receipt.error.is_some() || receipt.result.is_some() || !receipt.artifacts.is_empty() => return Err(bad("active receipt has inconsistent execution state")),
            State::Succeeded => {
                let result = receipt.result.as_ref().ok_or_else(|| bad("successful receipt has no result"))?;
                if receipt.owner.is_none() || receipt.termination.as_ref().is_none_or(|end| end.reason != "completed")
                    || receipt.error.is_some() || receipt.artifacts.is_empty()
                    || result.get("job_id").and_then(Value::as_str) != Some(id)
                    || result.get("request_digest").and_then(Value::as_str) != Some(receipt.request_digest.as_str())
                    || result.get("fingerprint").and_then(Value::as_str) != Some(receipt.fingerprint.as_str())
                    || result.get("state").and_then(Value::as_str) != Some("succeeded")
                { return Err(bad("successful receipt is missing its complete result binding")); }
                let artifacts = normalized_artifacts(self, receipt, result.get("artifacts"))?;
                if artifacts != receipt.artifacts { return Err(bad("result artifact manifest does not match the receipt")); }
            }
            State::Failed | State::Cancelled | State::Interrupted => {
                if receipt.termination.is_none() || receipt.error.as_ref().is_none_or(|error| !error.is_object())
                    || receipt.result.is_some() || !receipt.artifacts.is_empty()
                { return Err(bad("unsuccessful terminal receipt contains incomplete or published outputs")); }
            }
            _ => {},
        }
        Ok(())
    }

    fn create(&self, receipt: &Receipt) -> Result<File, CliError> {
        self.validate(receipt, &receipt.job_id)?;
        let temporary = tempfile::Builder::new().prefix(".prepare-").tempdir_in(&self.root).map_err(io_error)?;
        let lease = open_lock(&temporary.path().join("lease.lock"))?;
        if !try_lock(&lease)? { return Err(internal("new job lease is unexpectedly held")); }
        atomic_json(&temporary.path().join("receipt.json"), receipt)?;
        fs::rename(temporary.path(), self.job_dir(&receipt.job_id)).map_err(io_error)?;
        let _ = temporary.keep();
        sync_directory(&self.root)?;
        Ok(lease)
    }

    fn save(&self, receipt: &Receipt) -> Result<(), CliError> {
        atomic_json(&self.job_dir(&receipt.job_id).join("receipt.json"), receipt)
    }

    fn reconcile(&self, receipt: &mut Receipt) -> Result<(), CliError> {
        if !matches!(receipt.state, State::Running | State::Cancelling) { return Ok(()); }
        let Some(_lease) = self.try_job_lease(&receipt.job_id)? else { return Ok(()); };
        let error = CliError::new("WORKER_FAILED", "execution lease was released without a committed terminal receipt")
            .with_details(json!({"job_id":receipt.job_id,"state":"interrupted","retry_required":true}));
        receipt.fail(&error, State::Interrupted)?;
        self.save(receipt)
    }
}

struct Controls { job_id: Option<String>, retry_of: Option<String>, idempotency: Option<Idempotency> }

impl Controls {
    fn parse(params: &Value) -> Result<Self, CliError> {
        if !params.is_object() { return Err(invalid("params must be an object")); }
        let job_id = optional_text(params, "job_id", 36)?;
        let retry_of = optional_text(params, "retry_of", 36)?;
        if let Some(id) = &job_id { validate_id(id)?; }
        if let Some(id) = &retry_of {
            validate_id(id)?;
            if job_id.is_none() { return Err(invalid("retry_of requires a prepared job_id")); }
        }
        let key = optional_text(params, "idempotency_key", 256)?;
        let scope = optional_text(params, "idempotency_scope", 32 * 1024 - 8)?;
        if scope.is_some() && key.is_none() { return Err(invalid("idempotency_scope requires idempotency_key")); }
        let idempotency = key.map(|key| -> Result<Idempotency, CliError> {
            let scope = match scope {
                Some(scope) => format!("session:{scope}"),
                None => format!("project:{}", params.get("path").and_then(Value::as_str).unwrap_or("workspace")),
            };
            valid_text(&scope, "idempotency scope", 32 * 1024)?;
            Ok(Idempotency { scope, key })
        }).transpose()?;
        Ok(Self { job_id, retry_of, idempotency })
    }
}

fn normalized_input(params: &Value, store: &Store) -> Result<Value, CliError> {
    let mut input = params.as_object().ok_or_else(|| invalid("params must be an object"))?.clone();
    for key in ["job_id", "retry_of", "idempotency_key", "idempotency_scope"] { input.remove(key); }
    input.insert("workspace".into(), Value::String(store.workspace_text.clone()));
    let input = Value::Object(input);
    check_size(&input, MAX_REQUEST_BYTES)?;
    Ok(input)
}

fn replay(store: &Store, receipt: Receipt) -> Result<Value, CliError> {
    if receipt.state == State::Succeeded {
        let result = receipt.result.as_ref().ok_or_else(|| invalid_receipt(&receipt.job_id, "successful result is missing"))?;
        if verify_outputs(store, &receipt, result)? != receipt.artifacts {
            return Err(invalid_receipt(&receipt.job_id, "replayed artifacts differ from the receipt"));
        }
        let mut result = receipt.result.expect("validated successful result");
        result["replayed"] = Value::Bool(true);
        return Ok(result);
    }
    let error = receipt.error.as_ref().ok_or_else(|| invalid_receipt(&receipt.job_id, "terminal error is missing"))?;
    let stored_code = error.get("code").and_then(Value::as_str).unwrap_or("WORKER_FAILED");
    let code = match stored_code {
        "INVALID_ARGUMENT" => "INVALID_ARGUMENT", "INVALID_PROJECT" => "INVALID_PROJECT",
        "DECISION_REQUIRED" => "DECISION_REQUIRED", "REVISION_CONFLICT" => "REVISION_CONFLICT",
        "PROJECT_BUSY" => "PROJECT_BUSY", "UNSUPPORTED_CAPABILITY" => "UNSUPPORTED_CAPABILITY",
        "VOICE_NOT_ADMITTED" => "VOICE_NOT_ADMITTED", "VERSION_MISMATCH" => "VERSION_MISMATCH",
        "MISSING_ASSET" => "MISSING_ASSET", "RESOURCE_EXHAUSTED" => "RESOURCE_EXHAUSTED",
        "CANCELLED" => "CANCELLED", "ARTIFACT_INVALID" => "ARTIFACT_INVALID", "IO_ERROR" => "IO_ERROR",
        "INTERNAL_ERROR" => "INTERNAL_ERROR", _ => "WORKER_FAILED",
    };
    let mut outcome = CliError::new(code, error.get("message").and_then(Value::as_str).unwrap_or("previous job did not succeed"))
        .with_details(json!({"job_id":receipt.job_id,"state":receipt.state,"retry_required":true,"original_error":error}));
    if let Some(retryable) = error.get("retryable").and_then(Value::as_bool) { outcome.retryable = retryable; }
    Err(outcome)
}

fn bind_result(receipt: &Receipt, result: &mut Value) -> Result<(), CliError> {
    let object = result.as_object_mut().ok_or_else(|| invalid_receipt(&receipt.job_id, "render result must be an object"))?;
    for (key, expected) in [("job_id", receipt.job_id.as_str()), ("request_digest", receipt.request_digest.as_str()),
        ("fingerprint", receipt.fingerprint.as_str()), ("state", "succeeded")]
    {
        if object.get(key).is_some_and(|actual| actual.as_str() != Some(expected)) {
            return Err(invalid_receipt(&receipt.job_id, format!("render result has a nonmatching {key}")));
        }
        object.insert(key.into(), json!(expected));
    }
    Ok(())
}

fn normalized_artifacts(store: &Store, receipt: &Receipt, value: Option<&Value>) -> Result<Vec<Artifact>, CliError> {
    let values = value.and_then(Value::as_array).filter(|values| !values.is_empty() && values.len() <= MAX_ARTIFACTS)
        .ok_or_else(|| invalid_receipt(&receipt.job_id, "artifacts must contain 1–4096 file records"))?;
    let mut artifacts = Vec::with_capacity(values.len());
    for value in values {
        let mut artifact = Artifact::deserialize(value)
            .map_err(|error| invalid_receipt(&receipt.job_id, format!("invalid artifact: {error}")))?;
        if artifact.sha256.len() != 64 || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid_receipt(&receipt.job_id, "artifact SHA-256 must contain 64 hexadecimal digits"));
        }
        artifact.sha256.make_ascii_lowercase();
        artifact.path = utf8_path(&artifact_path(store, receipt, &artifact.path)?)?;
        artifacts.push(artifact);
    }
    artifacts.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    if artifacts.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(invalid_receipt(&receipt.job_id, "artifact manifest has duplicate paths"));
    }
    Ok(artifacts)
}

fn verify_outputs(store: &Store, receipt: &Receipt, result: &Value) -> Result<Vec<Artifact>, CliError> {
    let artifacts = normalized_artifacts(store, receipt, result.get("artifacts"))?;
    let report_path = receipt.input.get("report").and_then(Value::as_str)
        .map(|path| artifact_path(store, receipt, path).and_then(|path| utf8_path(&path))).transpose()?;
    let stems = receipt.input.get("scope").and_then(Value::as_str) == Some("Stems");
    for field in ["output", "report"] {
        if stems && field == "output" {
            let matching = artifacts.iter().filter(|artifact| Some(artifact.path.as_str()) != report_path.as_deref());
            let mut count = 0;
            for artifact in matching {
                if !stem_path(&receipt.input, Path::new(&artifact.path)) {
                    return Err(invalid_receipt(&receipt.job_id, "artifact is outside the requested stem output set"));
                }
                count += 1;
            }
            if count == 0 { return Err(invalid_receipt(&receipt.job_id, "stem output set is empty")); }
            continue;
        }
        if let Some(path) = receipt.input.get(field).and_then(Value::as_str) {
            let path = utf8_path(&artifact_path(store, receipt, path)?)?;
            if !artifacts.iter().any(|artifact| artifact.path == path) {
                return Err(invalid_receipt(&receipt.job_id, format!("artifact manifest is missing declared {field}")));
            }
        }
    }
    let mut report_bytes = None;
    for artifact in &artifacts {
        let is_report = report_path.as_deref() == Some(artifact.path.as_str());
        if artifact.role.is_some_and(|role| (role == ArtifactRole::Report) != is_report) {
            return Err(invalid_receipt(&receipt.job_id, "artifact role disagrees with declared destination"));
        }
        let bytes = verify_file(&receipt.job_id, artifact, is_report)?;
        if is_report { report_bytes = bytes; }
    }
    if let Some(bytes) = report_bytes {
        let report: Value = serde_json::from_slice(&bytes)
            .map_err(|error| invalid_receipt(&receipt.job_id, format!("invalid completion report: {error}")))?;
        for (key, expected) in [("job_id", receipt.job_id.as_str()), ("fingerprint", receipt.fingerprint.as_str()),
            ("request_digest", receipt.request_digest.as_str())]
        {
            if report.get(key).and_then(Value::as_str) != Some(expected) {
                return Err(invalid_receipt(&receipt.job_id, format!("completion report has a nonmatching {key}")));
            }
        }
        let report_artifacts = normalized_artifacts(store, receipt, report.get("artifacts"))?;
        let outputs = artifacts.iter().filter(|artifact| Some(artifact.path.as_str()) != report_path.as_deref());
        if !report_artifacts.iter().eq(outputs) {
            return Err(invalid_receipt(&receipt.job_id, "completion report does not describe the complete output graph"));
        }
    }
    Ok(artifacts)
}

fn artifact_path(store: &Store, receipt: &Receipt, text: &str) -> Result<PathBuf, CliError> {
    valid_text(text, "artifact path", 32 * 1024).map_err(|_| invalid_receipt(&receipt.job_id, "invalid artifact path"))?;
    let path = Path::new(text);
    if path.components().any(|component| matches!(component, Component::ParentDir | Component::CurDir)) {
        return Err(invalid_receipt(&receipt.job_id, "artifact path contains traversal components"));
    }
    let path = if path.is_absolute() { path.to_owned() } else { store.workspace.join(path) };
    let declared = ["output", "report"].iter().filter_map(|field| receipt.input.get(*field).and_then(Value::as_str))
        .any(|declared| Path::new(declared).is_absolute() && path == Path::new(declared));
    if !path.is_absolute() || path.starts_with(&store.root) || (!path.starts_with(&store.workspace) && !declared && !stem_path(&receipt.input, &path)) {
        return Err(invalid_receipt(&receipt.job_id, "artifact escapes the workspace or its declared output destinations"));
    }
    Ok(path)
}

fn stem_path(input: &Value, path: &Path) -> bool {
    if input.get("scope").and_then(Value::as_str) != Some("Stems") { return false; }
    let Some(output) = input.get("output").and_then(Value::as_str).map(Path::new) else { return false; };
    if output.parent() != path.parent() || path.extension().and_then(|ext| ext.to_str()) != Some("wav") { return false; }
    let Some(base) = output.file_stem().and_then(|stem| stem.to_str()) else { return false; };
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { return false; };
    let Some((index, name)) = stem.strip_prefix(base).and_then(|stem| stem.strip_prefix('-')).and_then(|stem| stem.split_once('-')) else { return false; };
    index.len() >= 2 && index.bytes().all(|byte| byte.is_ascii_digit()) && index.parse::<usize>().is_ok_and(|value| value > 0)
        && name.chars().all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_')) && name.chars().count() <= 80
}

fn verify_file(id: &str, artifact: &Artifact, retain: bool) -> Result<Option<Vec<u8>>, CliError> {
    let bad = |message| invalid_receipt(id, format!("{}: {message}", artifact.path));
    let mut file = open_regular(Path::new(&artifact.path)).map_err(|error| bad(error.to_string()))?;
    let before = file.metadata().map_err(|error| bad(error.to_string()))?;
    if before.len() != artifact.bytes { return Err(bad("artifact byte count does not match".into())); }
    if retain && artifact.bytes > MAX_RECEIPT_BYTES as u64 { return Err(bad("completion report exceeds 8 MiB".into())); }
    let mut retained = retain.then(|| Vec::with_capacity(artifact.bytes as usize));
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut count = 0u64;
    loop {
        let length = file.read(&mut buffer).map_err(|error| bad(error.to_string()))?;
        if length == 0 { break; }
        count = count.checked_add(length as u64).ok_or_else(|| bad("artifact length overflow".into()))?;
        if count > artifact.bytes { return Err(bad("artifact grew during verification".into())); }
        hash.update(&buffer[..length]);
        if let Some(bytes) = &mut retained { bytes.extend_from_slice(&buffer[..length]); }
    }
    let after = file.metadata().map_err(|error| bad(error.to_string()))?;
    let current = open_regular(Path::new(&artifact.path)).and_then(|file| file.metadata())
        .map_err(|error| bad(error.to_string()))?;
    if count != artifact.bytes || before.len() != after.len() || before.modified().ok() != after.modified().ok()
        || !same_file(&before, &current) || hex::encode(hash.finalize()) != artifact.sha256
    { return Err(bad("artifact changed or its SHA-256 does not match".into())); }
    Ok(retained)
}

fn retry_destination(old: &str, override_value: Option<&Value>, id: &str) -> Result<String, CliError> {
    let old = Path::new(old);
    if !old.is_absolute() { return Err(invalid("stored retry destinations must be absolute")); }
    let path = match override_value {
        Some(value) => PathBuf::from(value.as_str().ok_or_else(|| invalid("retry destinations must be strings"))?),
        None => {
            let stem = old.file_stem().and_then(|part| part.to_str()).ok_or_else(|| invalid("invalid stored destination"))?;
            let extension = old.extension().and_then(|part| part.to_str()).map(|extension| format!(".{extension}")).unwrap_or_default();
            old.with_file_name(format!("{stem}.retry-{id}{extension}"))
        }
    };
    if path.components().any(|component| matches!(component, Component::ParentDir | Component::CurDir)) {
        return Err(invalid("retry destination contains traversal components"));
    }
    let name = path.file_name().ok_or_else(|| invalid("retry destination must name a file"))?;
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let path = parent.canonicalize().map_err(io_error)?.join(name);
    match fs::symlink_metadata(&path) {
        Ok(_) => return Err(invalid("retry destination already exists")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {},
        Err(error) => return Err(io_error(error)),
    }
    let path = utf8_path(&path)?;
    valid_text(&path, "retry destination", 32 * 1024)?;
    Ok(path)
}

// Sorted object keys at every depth, stable JSON numbers, and no whitespace.
// The pin and input are separate JSON members, avoiding concatenation ambiguity.
struct Canonical<'a>(&'a Value);
impl Serialize for Canonical<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(object) => {
                let mut keys: Vec<&String> = object.keys().collect();
                keys.sort_unstable();
                let mut map = serializer.serialize_map(Some(keys.len()))?;
                for key in keys { map.serialize_entry(key, &Canonical(&object[key]))?; }
                map.end()
            }
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values { sequence.serialize_element(&Canonical(value))?; }
                sequence.end()
            }
            value => value.serialize(serializer),
        }
    }
}

struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> { self.0.update(bytes); Ok(bytes.len()) }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn request_digest(input: &Value, fingerprint: &str) -> Result<String, CliError> {
    #[derive(Serialize)]
    struct Pinned<'a> { fingerprint: &'a str, input: Canonical<'a> }
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, &Pinned { fingerprint, input: Canonical(input) }).map_err(internal)?;
    Ok(hex::encode(writer.0.finalize()))
}

struct LimitedWriter<W> { inner: W, remaining: usize }
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining { return Err(io::Error::other("serialized job data exceeds its size limit")); }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

fn check_size(value: &Value, limit: usize) -> Result<(), CliError> {
    serde_json::to_writer(LimitedWriter { inner: io::sink(), remaining: limit }, value)
        .map_err(|_| CliError::new("RESOURCE_EXHAUSTED", format!("job data exceeds {limit} bytes")))
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| internal("receipt path has no parent"))?;
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.is_file() || metadata.file_type().is_symlink() { return Err(invalid("receipt destination is not a regular file")); }
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    serde_json::to_writer(LimitedWriter { inner: temporary.as_file_mut(), remaining: MAX_RECEIPT_BYTES - 1 }, value)
        .map_err(|error| CliError::new("IO_ERROR", format!("persist job receipt: {error}")))?;
    temporary.write_all(b"\n").map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary.persist(path).map_err(|error| io_error(error.error))?;
    sync_directory(parent)
}

fn read_optional<T: for<'de> Deserialize<'de>>(path: &Path, limit: usize) -> Result<Option<T>, CliError> {
    let file = match open_regular(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(error)),
    };
    if file.metadata().map_err(io_error)?.len() > limit as u64 {
        return Err(CliError::new("ARTIFACT_INVALID", "job record exceeds its size limit"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(io_error)?;
    if bytes.len() > limit { return Err(CliError::new("ARTIFACT_INVALID", "job record exceeds its size limit")); }
    serde_json::from_slice(&bytes).map(Some)
        .map_err(|error| CliError::new("ARTIFACT_INVALID", format!("invalid job record: {error}")))
}

fn read_required<T: for<'de> Deserialize<'de>>(path: &Path, limit: usize) -> Result<T, CliError> {
    read_optional(path, limit)?.ok_or_else(|| invalid("job receipt does not exist"))
}

#[cfg(unix)]
fn open_path(path: &Path, final_flags: rustix::fs::OFlags) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags, open, openat};
    if !path.is_absolute() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an absolute path")); }
    let mut directory = open("/", OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty())?;
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => name,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "path contains traversal components")),
        };
        let flags = if components.peek().is_none() { final_flags }
            else { OFlags::RDONLY | OFlags::DIRECTORY };
        directory = openat(&directory, name, flags | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR)?;
    }
    Ok(File::from(directory))
}

fn open_regular(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    let file = open_path(path, rustix::fs::OFlags::RDONLY)?;
    #[cfg(not(unix))]
    let file = {
        reject_symlinks(path)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)] {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
        }
        options.open(path)?
    };
    if !file.metadata()?.is_file() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a regular file")); }
    Ok(file)
}

fn open_lock(path: &Path) -> Result<File, CliError> {
    #[cfg(unix)]
    let file = open_path(path, rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE).map_err(io_error)?;
    #[cfg(not(unix))]
    let file = {
        reject_symlinks(path.parent().ok_or_else(|| invalid("lock has no parent"))?).map_err(io_error)?;
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(invalid("job lease must not be a symlink"));
        }
        OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path).map_err(io_error)?
    };
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() { return Err(invalid("job lease must be a regular file")); }
    #[cfg(unix)] {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 { return Err(invalid("job lease must not have hardlink aliases")); }
    }
    Ok(file)
}

#[cfg(not(unix))]
fn reject_symlinks(path: &Path) -> io::Result<()> {
    check_path_components(path, |current| {
        if fs::symlink_metadata(current)?.file_type().is_symlink() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "path must not contain symlinks"));
        }
        Ok(())
    })
}

#[cfg(not(unix))]
fn check_path_components(path: &Path, mut check: impl FnMut(&Path) -> io::Result<()>) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an absolute path"));
    }
    let mut current = PathBuf::new();
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        match component {
            Component::ParentDir | Component::CurDir => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "path contains traversal components"));
            }
            Component::Prefix(prefix) => {
                // Windows treats a verbatim drive prefix as absolute even
                // when the caller omitted the root separator.
                #[cfg(windows)]
                if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_))
                    && !matches!(components.peek(), Some(Component::RootDir))
                {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an explicit drive root"));
                }
                // UNC prefixes can have an implicit root separator.
                current.push(prefix.as_os_str());
                current.push(Component::RootDir.as_os_str());
            }
            Component::RootDir if !current.as_os_str().is_empty() => continue,
            _ => current.push(component.as_os_str()),
        }
        check(&current)?;
    }
    Ok(())
}

fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)] {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))] { a.len() == b.len() && a.created().ok() == b.created().ok() && a.modified().ok() == b.modified().ok() }
}

fn sync_directory(path: &Path) -> Result<(), CliError> {
    #[cfg(unix)] { open_path(path, rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY)
        .map_err(io_error)?.sync_all().map_err(io_error)?; }
    #[cfg(not(unix))] { let _ = path; }
    Ok(())
}

fn try_lock(file: &File) -> Result<bool, CliError> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(error)) => Err(lock_error(error)),
    }
}

fn lock_error(error: io::Error) -> CliError {
    if error.kind() == io::ErrorKind::Unsupported {
        CliError::new("UNSUPPORTED_CAPABILITY", "workspace filesystem does not support required OS file leases")
    } else { io_error(error) }
}

fn existing_store(params: &Value) -> Result<Store, CliError> {
    Store::open(params, false)?.ok_or_else(|| invalid("workspace has no job receipts"))
}

fn fields(params: &Value, allowed: &[&str]) -> Result<(), CliError> {
    let object = params.as_object().ok_or_else(|| invalid("params must be an object"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(format!("unknown job parameter {key}")));
    }
    Ok(())
}

fn job_id(params: &Value) -> Result<&str, CliError> {
    let id = params.get("job_id").and_then(Value::as_str).ok_or_else(|| invalid("job_id is required"))?;
    validate_id(id)?;
    Ok(id)
}

fn validate_id(id: &str) -> Result<(), CliError> {
    if id.len() != 36 || id.bytes().enumerate().any(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) { byte != b'-' }
        else { !matches!(byte, b'0'..=b'9' | b'a'..=b'f') }
    }) { return Err(invalid("job_id must be a canonical lowercase UUID")); }
    Ok(())
}

fn optional_text(params: &Value, name: &str, max: usize) -> Result<Option<String>, CliError> {
    params.get(name).map(|value| {
        let text = value.as_str().ok_or_else(|| invalid(format!("{name} must be a string")))?;
        valid_text(text, name, max)?;
        Ok(text.to_owned())
    }).transpose()
}

fn valid_text(text: &str, name: &str, max: usize) -> Result<(), CliError> {
    if text.is_empty() || text.len() > max || text.chars().any(char::is_control) {
        return Err(invalid(format!("{name} must contain 1–{max} bytes without control characters")));
    }
    Ok(())
}

fn utf8_path(path: &Path) -> Result<String, CliError> {
    path.to_str().map(str::to_owned).ok_or_else(|| invalid("job and artifact paths must be UTF-8"))
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_millis().min(u64::MAX as u128) as u64)
}

fn with_job(mut error: CliError, id: &str) -> CliError {
    let mut details = error.details.take().unwrap_or_else(|| json!({}));
    if let Some(object) = details.as_object_mut() {
        if object.get("job_id").is_none_or(|value| value.as_str() == Some(id)) {
            object.insert("job_id".into(), json!(id));
            error.details = Some(details);
            return error;
        }
    }
    error.details = Some(json!({"job_id":id,"cause":details}));
    error
}

fn cancelled(id: &str) -> CliError {
    CliError::new("CANCELLED", "render was cancelled").with_details(json!({"job_id":id}))
}

fn invalid_receipt(id: &str, message: impl Into<String>) -> CliError {
    CliError::new("ARTIFACT_INVALID", message).with_details(json!({"job_id":id}))
}

fn invalid(message: impl Into<String>) -> CliError { CliError::new("INVALID_ARGUMENT", message) }
fn internal(error: impl std::fmt::Display) -> CliError { CliError::new("INTERNAL_ERROR", error.to_string()) }
fn io_error(error: io::Error) -> CliError { CliError::new("IO_ERROR", error.to_string()) }

#[cfg(all(test, windows))]
mod windows_path_tests {
    use super::*;

    #[test]
    fn checks_complete_drive_and_share_roots_before_descendants() {
        for (root, separator) in [
            (r"C:\", ""),
            (r"\\?\C:\", ""),
            (r"\\server\share", "\\"),
            (r"\\server\share\", ""),
            (r"\\?\UNC\server\share", "\\"),
            (r"\\?\UNC\server\share\", ""),
            (r"\\?\Volume{00000000-0000-0000-0000-000000000000}\", ""),
        ] {
            let rooted = format!("{root}{separator}");
            for suffix in ["", r"workspace", r"workspace\receipt.json"] {
                let input = if suffix.is_empty() { root.to_owned() } else { format!("{rooted}{suffix}") };
                let mut checked = Vec::new();
                check_path_components(Path::new(&input), |path| {
                    checked.push(path.as_os_str().to_owned());
                    Ok(())
                }).unwrap();
                let mut expected = vec![std::ffi::OsString::from(&rooted)];
                if !suffix.is_empty() { expected.push(format!("{rooted}workspace").into()); }
                if suffix.ends_with("receipt.json") { expected.push(format!("{rooted}{suffix}").into()); }
                assert_eq!(checked, expected, "{input}");
            }
        }
    }

    #[test]
    fn rejects_ambiguous_roots_and_traversal() {
        for path in [r"C:", r"C:workspace", r"\\?\C:", r"\workspace", "workspace"] {
            let error = check_path_components(Path::new(path), |_| panic!("must reject {path} before checking metadata"))
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{path}");
        }
        for path in [r"C:\workspace\..\receipt.json", r"\\?\C:\workspace\.\receipt.json",
            r"\\?\UNC\server\share\workspace\..\receipt.json"]
        {
            let error = check_path_components(Path::new(path), |_| Ok(())).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{path}");
        }
    }

    #[test]
    fn root_check_failure_stops_the_walk() {
        let mut checked = 0;
        let error = check_path_components(Path::new(r"\\?\C:\workspace\receipt.json"), |_| {
            checked += 1;
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "root denied"))
        }).unwrap_err();
        assert_eq!(checked, 1);
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn canonical_workspace_can_create_and_reopen_its_index_lock() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = directory.path().canonicalize().unwrap();
        reject_symlinks(&workspace).unwrap();
        let params = json!({"workspace":workspace.to_str().unwrap()});
        let store = Store::open(&params, true).unwrap().unwrap();
        let lock_path = store.root.join("index.lock");
        assert!(!lock_path.exists());
        drop(store.index_lock().unwrap());
        assert!(lock_path.is_file());
        drop(store.index_lock().unwrap());
    }
}
