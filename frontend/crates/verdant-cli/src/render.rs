use std::{env, fs::{self, File}, io::{self, Write}, path::{Path, PathBuf}, time::Duration};

use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use verdant_backend::{ArtifactCache, WorkerConfig, platform};
use verdant_domain::{Project, TrackId};
use verdant_media::{AssetPolicy, MediaImporter};
use verdant_plugins::{PluginClient, PluginClientOpts};
use verdant_service::{
    audio::{AudioScope, ExportOptions, RenderSettings, WavEncoding, render_outputs},
    media::project_policy,
    project_audio::PhraseSource,
    render::{RenderError, RenderSession},
};

use crate::{CliError, Outcome, internal, invalid, jobs, project};

#[derive(Deserialize)]
struct WorkerOptions {
    worker: Option<PathBuf>,
    voice_registry: Option<PathBuf>,
    voice_package: Option<PathBuf>,
    project_root: Option<PathBuf>,
    onnxruntime_library: Option<PathBuf>,
    timeout_seconds: Option<u64>,
}

impl WorkerOptions {
    fn config(&self, workspace: &Path) -> Result<WorkerConfig, CliError> {
        let program = self.worker.clone().or_else(|| env::var_os("VERDANT_WORKER_BINARY").map(PathBuf::from))
            .or_else(|| {
                let binary = if cfg!(windows) { "verdant-worker.exe" } else { "verdant-worker" };
                let exe = env::current_exe().ok()?;
                let directory = exe.parent()?;
                [directory.join(binary), directory.join("../libexec/verdant").join(binary)].into_iter().find(|path| path.is_file())
            }).ok_or_else(|| CliError::new("UNSUPPORTED_CAPABILITY", "verdant-worker is missing; pass --worker"))?;
        let program = program.canonicalize().map_err(|e| CliError::new("UNSUPPORTED_CAPABILITY", format!("{}: {e}", program.display())))?;
        if !program.is_file() { return Err(invalid("worker executable must be a regular file")); }
        let registry = self.voice_registry.clone().or_else(|| env::var_os("VERDANT_VOICE_REGISTRY").map(PathBuf::from))
            .unwrap_or_else(|| platform::state_dir().join("voices.json"));
        let mut extra = Vec::new();
        let registry = Some(registry);
        for (flag, path) in [
            ("--voice-registry", &registry), ("--voice-package", &self.voice_package),
            ("--project-root", &self.project_root), ("--onnxruntime-library", &self.onnxruntime_library),
        ] {
            if let Some(path) = path {
                extra.push(flag);
                extra.push(path.to_str().ok_or_else(|| invalid(format!("{flag} path is not UTF-8")))?);
            }
        }
        Ok(WorkerConfig::native_worker(program, workspace, &extra))
    }

    fn timeout(&self) -> Result<Duration, CliError> {
        let seconds = self.timeout_seconds.unwrap_or(3600);
        if !(1..=86_400).contains(&seconds) { return Err(invalid("timeout_seconds must be 1–86400")); }
        Ok(Duration::from_secs(seconds))
    }
}

#[derive(Deserialize)]
struct Input {
    path: PathBuf,
    output: Option<PathBuf>,
    report: Option<PathBuf>,
    workspace: Option<PathBuf>,
    recovery: Option<String>,
    allowed_creator: Option<String>,
    expect_revision: Option<u64>,
    idempotency_key: Option<String>,
    overwrite: Option<bool>,
    seed: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    encoding: Option<WavEncoding>,
    scope: Option<AudioScope>,
    track: Option<TrackId>,
    start_tick: Option<i64>,
    end_tick: Option<i64>,
    max_duration_seconds: Option<u32>,
    effects: Option<bool>,
    plughost: Option<PathBuf>,
    #[serde(default)]
    asset_roots: Vec<PathBuf>,
    #[serde(flatten)]
    worker: WorkerOptions,
}

const WORKER_FIELDS: &[&str] = &[
    "worker", "voice_registry", "voice_package", "project_root",
    "onnxruntime_library", "timeout_seconds",
];
const RENDER_FIELDS: &[&str] = &[
    "path", "output", "report", "workspace", "recovery", "allowed_creator", "expect_revision", "idempotency_key",
    "overwrite", "seed", "sample_rate", "channels", "encoding", "scope", "track", "start_tick", "end_tick",
    "effects", "plughost", "asset_roots", "job_id", "retry_of", "max_duration_seconds", "idempotency_scope",
];

fn fields(params: &Value, accepted: &[&str], worker: bool) -> Result<(), CliError> {
    for name in params.as_object().ok_or_else(|| invalid("params must be an object"))?.keys() {
        if !accepted.contains(&name.as_str()) && !(worker && WORKER_FIELDS.contains(&name.as_str())) {
            return Err(invalid(format!("unknown parameter {name}")));
        }
    }
    Ok(())
}

pub fn doctor(params: Value) -> Outcome {
    fields(&params, &[], true)?;
    let input: WorkerOptions = serde_json::from_value(params).map_err(|e| invalid(e.to_string()))?;
    let temporary = tempfile::tempdir().map_err(io_error)?;
    let config = input.config(temporary.path())?;
    let session = RenderSession::start(config, input.timeout()?.min(Duration::from_secs(30))).map_err(render_error)?;
    let capabilities = session.capabilities();
    Ok(json!({"version":env!("CARGO_PKG_VERSION"),"capabilities":capabilities,
        "render_ready":!capabilities.routes.is_empty(),"max_active_jobs":capabilities.max_active_jobs}))
}

pub fn voice_query(op: &str, params: Value) -> Outcome {
    let mut report = doctor(params)?;
    if op == "voice.verify" {
        if report["capabilities"]["voices"]["voices"].as_object().is_none_or(|voices| voices.is_empty()) {
            return Err(CliError::new("VOICE_NOT_ADMITTED", "the worker did not admit any voice"));
        }
        return Ok(report);
    }
    Ok(report["capabilities"]["voices"].take())
}

pub fn dispatch(op: &str, mut params: Value, progress: &mut dyn FnMut(Value)) -> Outcome {
    fields(&params, RENDER_FIELDS, true)?;
    let mut input: Input = serde_json::from_value(params.clone()).map_err(|e| invalid(e.to_string()))?;
    let project_path = input.path.canonicalize().map_err(io_error)?;
    let project = project::read_snapshot(&project_path, input.recovery.as_deref(), input.allowed_creator.as_deref())?;
    if input.expect_revision.is_some_and(|expected| expected != project.revision) {
        return Err(CliError::new("REVISION_CONFLICT", "project revision differs from the expected revision")
            .with_details(json!({"revision":project.revision,"expected":input.expect_revision})));
    }
    let scope = input.scope.unwrap_or_default();
    if input.track.as_ref().is_some_and(|id| project.track(id).is_none()) {
        return Err(invalid("track does not exist"));
    }
    if scope == AudioScope::ActiveTrack && input.track.is_none() { return Err(invalid("ActiveTrack scope requires track")); }
    let range_ticks = match (input.start_tick, input.end_tick) {
        (None, None) => None,
        (Some(start), Some(end)) if start >= 0 && end > start => Some((start, end)),
        _ => return Err(invalid("range requires 0 <= start_tick < end_tick")),
    };
    let options = ExportOptions {
        scope, sample_rate: input.sample_rate.unwrap_or(48_000), channels: input.channels.unwrap_or(2),
        encoding: input.encoding.unwrap_or_default(), effects: input.effects.unwrap_or(true),
        ..ExportOptions::default()
    };
    if ![44_100, 48_000, 88_200, 96_000, 192_000].contains(&options.sample_rate) || !matches!(options.channels, 1 | 2) {
        return Err(invalid("unsupported output sample rate or channel count"));
    }
    let track_filter = if scope == AudioScope::ActiveTrack || op == "render.plan" { input.track.as_ref() } else { None };
    let duration = duration_upper_bound(&project, track_filter, range_ticks, options.effects)?;
    if let Some(limit) = input.max_duration_seconds {
        if limit == 0 { return Err(invalid("max_duration_seconds must be positive")); }
        if duration > f64::from(limit) {
            return Err(CliError::new("RESOURCE_EXHAUSTED", "timeline exceeds the requested duration budget")
                .with_details(json!({"duration_seconds":duration,"max_duration_seconds":limit})));
        }
    }
    let timeout = input.worker.timeout()?;
    let worker_workspace = tempfile::tempdir().map_err(io_error)?;
    let config = input.worker.config(worker_workspace.path())?;
    let worker_sha = artifact(&config.program, "worker")?["sha256"].clone();
    let mut session = Some(RenderSession::start(config, timeout.min(Duration::from_secs(30))).map_err(render_error)?);
    let native = session.as_ref().expect("started worker");
    let tracks: Vec<_> = if scope == AudioScope::ActiveTrack || op == "render.plan" {
        input.track.iter().cloned().collect()
    } else { Vec::new() };
    let plans = native.plan(&project, false, &tracks);
    let worker = native.capabilities().clone();
    if op == "render.plan" {
        return Ok(json!({"project_id":project.project_id,"revision":project.revision,"duration_seconds":duration,"plans":plans,"worker":worker}));
    }

    let output = destination(input.output.as_deref().ok_or_else(|| invalid("output is required"))?)?;
    if !output.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("wav")) {
        return Err(invalid("audio output must have a .wav extension"));
    }
    let report = destination(&input.report.take().unwrap_or_else(|| output.with_extension("render.json")))?;
    if output == report || output == project_path || report == project_path {
        return Err(invalid("project, audio, and report must use distinct paths"));
    }
    let workspace = ensure_workspace(input.workspace.as_deref())?;
    params["workspace"] = json!(workspace);
    params["path"] = json!(project_path);
    params["output"] = json!(output);
    params["report"] = json!(report);
    if let Some(key) = input.idempotency_key.take() { params["idempotency_key"] = json!(key); }
    let overwrite = input.overwrite.unwrap_or(false);
    let seed = input.seed.unwrap_or(1);
    let project_digest = digest(&project)?;
    let policy = AssetPolicy::new(input.asset_roots).and_then(|policy| project_policy(&project_path, policy))
        .map_err(|e| CliError::new("MISSING_ASSET", e.to_string()))?;
    let media_pins = media_pins(&project, &policy)?;
    let effects = effect_pins(&project, options.effects)?;
    let plughost = if options.effects { verdant_plugins::resolve_plughost(input.plughost.as_deref()) } else { None };
    let plughost_pin = plughost.as_deref().map(|path| artifact(path, "plughost")).transpose()?;
    let pin = digest(&json!({"project_sha256":project_digest,"worker_sha256":worker_sha,"worker":worker,
        "plans":plans,"seed":seed,"audio":options,"track":input.track,"range":range_ticks,
        "media":media_pins,"effects":effects,"plughost":plughost_pin}))?;
    let settings = RenderSettings {
        options, active_track: input.track.or_else(|| project.tracks.first().map(|track| track.id.clone()))
            .unwrap_or_else(|| TrackId::from("master")), range_ticks, media_policy: policy,
    };
    jobs::run(&params, &pin, |context| {
        if !overwrite && (output.exists() || report.exists()) {
            return Err(CliError::new("DECISION_REQUIRED", "output or report already exists; choose a new path or allow overwrite"));
        }
        let mut progress_event = |mut data: Value| {
            data["job_id"] = json!(context.id);
            progress(data);
        };
        progress_event(json!({"stage":"running"}));
        let cancelled = || crate::interrupted() || context.is_cancelled().unwrap_or(true);
        context.check_cancelled()?;
        let cache = ArtifactCache::open(workspace.join("cache")).map_err(io_error)?;
        let rendered = session.take().expect("one render per job").render(
            &project, project.revision, &plans, false, seed, &cache, timeout,
            cancelled, &mut progress_event,
        ).map_err(render_error)?;
        context.check_cancelled()?;
        context.progress(json!({"stage":"mixing","phrases":rendered.len()}))?;
        progress_event(json!({"stage":"mixing","phrases":rendered.len()}));
        let phrase_pins = rendered.iter().map(|item| json!({"track":item.phrase.track,
            "index":item.phrase.index,"start_seconds":item.phrase.start_seconds,"duration_seconds":item.phrase.seconds,
            "route":item.phrase.route,"content_digest":item.digest,"audio_sha256":item.audio.sha256,"sample_rate":item.audio.sample_rate})).collect::<Vec<_>>();
        let sources = rendered.into_iter().map(|item| PhraseSource {
            track: item.phrase.track, start_seconds: item.phrase.start_seconds, note_ids: item.phrase.note_ids,
            phoneme_timing: item.phrase.score.phoneme_timing.clone(), audio: item.audio, playback_key: None,
        }).collect();
        let plugins = plughost.map(|binary| PluginClient::new(PluginClientOpts {
            plughost_path: Some(binary), cache_dir: workspace.join("plugins"), sample_rate: verdant_plugins::SAMPLE_RATE,
            max_frames: verdant_plugins::shm::DSP_QUANTUM_FRAMES,
        })).transpose().map_err(|e| CliError::new("UNSUPPORTED_CAPABILITY", e.to_string()))?;
        if effect_pins(&project, settings.options.effects)? != effects {
            return Err(CliError::new("REVISION_CONFLICT", "effect binaries changed before export"));
        }
        let rendered = render_outputs(&output, &project, sources, plugins.as_deref(), &settings, overwrite,
            &cancelled).map_err(|e| {
                if cancelled() { CliError::new("CANCELLED", "audio export cancelled") }
                else { CliError::new("IO_ERROR", e) }
            })?;
        let artifacts = rendered.paths.iter().map(|path| artifact(path, "audio")).collect::<Result<Vec<_>, _>>()?;
        let mut result = json!({"schema":"verdant-render/1","job_id":context.id,"project_id":project.project_id,
            "revision":project.revision,"project_sha256":project_digest,"input_sha256":pin,"frames":rendered.frames,
            "fingerprint":context.fingerprint,"request_digest":context.request_digest,
            "sample_rate":settings.options.sample_rate,"audio":settings.options,"range_ticks":range_ticks,"seed":seed,
            "worker_sha256":worker_sha,"voices":worker.voices,"media":media_pins,"effects":effects,"phrases":phrase_pins,
            "artifacts":artifacts});
        publish_report(&report, &result, overwrite)?;
        result["artifacts"].as_array_mut().expect("artifact array").push(artifact(&report, "report")?);
        Ok(result)
    })
}

fn digest(value: &impl serde::Serialize) -> Result<String, CliError> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value).map_err(internal)?)))
}

fn media_pins(project: &Project, policy: &AssetPolicy) -> Result<Vec<Value>, CliError> {
    let importer = MediaImporter::new(policy.clone(), Default::default());
    let mut seen = std::collections::BTreeSet::new();
    let mut pins = Vec::new();
    for track in &project.tracks {
        for clip in verdant_media::read_clips(track).map_err(|e| CliError::new("INVALID_PROJECT", e.to_string()))? {
            if seen.insert((clip.source.path.clone(), clip.source.sha256.clone())) {
                importer.load_source(&clip.source).map_err(|e| CliError::new("MISSING_ASSET", e.to_string()))?;
                pins.push(json!({"sha256":clip.source.sha256,"bytes":clip.source.byte_len,"frames":clip.source.frames,"sample_rate":clip.source.sample_rate}));
            }
        }
    }
    Ok(pins)
}

fn duration_upper_bound(project: &Project, selected: Option<&TrackId>, range: Option<(i64, i64)>, effects: bool) -> Result<f64, CliError> {
    use verdant_plugins::project::read_inserts;
    let tempo = verdant_domain::TempoMap::new(&project.tempo_map, project.ppq)
        .map_err(|e| CliError::new("INVALID_PROJECT", e.to_string()))?;
    let tail = f64::from(verdant_service::audio::INSERT_TAIL_SECONDS);
    let mut end = 0.0f64;
    for track in project.tracks.iter().filter(|track| selected.is_none_or(|id| &track.id == id)) {
        let note_end = track.notes.iter().map(|note| note.end_tick()).max().unwrap_or(0);
        let mut track_end = tempo.seconds_at(note_end as f64);
        for clip in verdant_media::read_clips(track).map_err(|e| CliError::new("INVALID_PROJECT", e.to_string()))? {
            track_end = track_end.max(tempo.seconds_at(clip.start_tick as f64) + clip.duration_frames as f64 / f64::from(clip.source.sample_rate));
        }
        if effects && track_end > 0.0 && read_inserts(&track.unknown).iter().any(|effect| !effect.bypass) {
            track_end += tail;
        }
        end = end.max(track_end);
    }
    if effects && end > 0.0 && verdant_service::project_audio::master_insert_specs(project).iter().any(|effect| !effect.bypass) {
        end += tail;
    }
    if let Some((_, range_end)) = range { end = end.max(tempo.seconds_at(range_end as f64)); }
    if !end.is_finite() { return Err(CliError::new("INVALID_PROJECT", "timeline duration is not finite")); }
    Ok(end)
}

fn effect_pins(project: &Project, enabled: bool) -> Result<Vec<Value>, CliError> {
    use verdant_plugins::project::read_inserts;
    if !enabled { return Ok(Vec::new()); }
    project.tracks.iter().flat_map(|track| read_inserts(&track.unknown))
        .chain(verdant_service::project_audio::master_insert_specs(project))
        .filter(|effect| !effect.bypass).map(|effect| {
            let binary = if effect.format == "builtin" { None } else {
                Some(verdant_plugins::scan::sha256_bundle(Path::new(&effect.path))
                    .map_err(|e| CliError::new("MISSING_ASSET", format!("effect {}: {e}", effect.name)))?)
            };
            Ok(json!({"instance_id":effect.instance_id,"format":effect.format,"id":effect.id,"version":effect.version,"binary":binary}))
        }).collect()
}

fn artifact(path: &Path, role: &str) -> Outcome {
    let file = File::open(path).map_err(io_error)?;
    let committed = file.metadata().map_err(io_error)?;
    if !committed.is_file() { return Err(invalid(format!("{} is not a regular file", path.display()))); }
    let mut hash = Sha256::new();
    let bytes = io::copy(&mut io::BufReader::new(file), &mut hash).map_err(io_error)?;
    if bytes != committed.len() { return Err(CliError::new("ARTIFACT_INVALID", format!("{} changed while hashing", path.display()))); }
    Ok(json!({"path":path,"role":role,"bytes":bytes,"sha256":hex::encode(hash.finalize())}))
}

fn destination(path: &Path) -> Result<PathBuf, CliError> {
    let name = path.file_name().ok_or_else(|| invalid("destination needs a filename"))?;
    let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Ok(parent.canonicalize().map_err(io_error)?.join(name))
}

fn publish_report(path: &Path, value: &Value, overwrite: bool) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| invalid("report needs a parent directory"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    serde_json::to_writer_pretty(&mut file, value).map_err(internal)?;
    file.write_all(b"\n").map_err(io_error)?;
    file.as_file().sync_all().map_err(io_error)?;
    if overwrite { file.persist(path) } else { file.persist_noclobber(path) }.map_err(|e| io_error(e.error))?;
    Ok(())
}

fn workspace_path() -> PathBuf { platform::cache_dir().join("cli") }
fn ensure_workspace(path: Option<&Path>) -> Result<PathBuf, CliError> {
    let path = path.map(Path::to_path_buf).unwrap_or_else(workspace_path);
    fs::create_dir_all(&path).map_err(io_error)?;
    path.canonicalize().map_err(io_error)
}

pub fn job(op: &str, mut params: Value, progress: &mut dyn FnMut(Value)) -> Outcome {
    if params["workspace"].is_null() { params["workspace"] = json!(workspace_path()); }
    if op == "job.retry" {
        let prepared = jobs::prepare_retry(&params)?;
        let params = &prepared["params"];
        return match dispatch("render", params.clone(), progress) {
            Ok(result) => Ok(result),
            Err(error) => {
                if let Err(record) = jobs::fail_prepared(params, &error) {
                    let details = json!({"failure":error,"receipt_failure":record});
                    return Err(CliError::new("IO_ERROR", "retry failed and its receipt could not be updated").with_details(details));
                }
                Err(error)
            }
        };
    }
    jobs::dispatch(op, params).unwrap_or_else(|| Err(invalid(format!("unknown job operation {op}"))))
}

pub fn cache(op: &str, params: Value) -> Outcome {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query { workspace: Option<PathBuf>, digest: Option<String> }
    let query: Query = serde_json::from_value(params).map_err(|e| invalid(e.to_string()))?;
    let workspace = query.workspace.unwrap_or_else(workspace_path);
    let _lease = if op == "cache.list" { None } else {
        Some(jobs::workspace_lease(&json!({"workspace":workspace}))?)
    };
    let root = workspace.join("cache");
    let cache = ArtifactCache::open(root).map_err(io_error)?;
    let digests = if let Some(digest) = query.digest {
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
            return Err(invalid("digest must be a lowercase SHA-256"));
        }
        vec![digest]
    } else { cache.entries() };
    match op {
        "cache.list" => Ok(json!({"digests":digests})),
        "cache.verify" => {
            let mut results = Vec::with_capacity(digests.len());
            let mut invalid = false;
            for digest in digests {
                let result = match cache.restore(&digest) {
                    Ok(Some(entry)) => json!({"digest":digest,"ok":true,"audio_sha256":entry.audio.sha256}),
                    Ok(None) => { invalid = true; json!({"digest":digest,"ok":false,"error":"missing entry"}) }
                    Err(error) => { invalid = true; json!({"digest":digest,"ok":false,"error":error.to_string()}) }
                };
                results.push(result);
            }
            if invalid { Err(CliError::new("ARTIFACT_INVALID", "one or more cache entries failed verification").with_details(json!({"entries":results}))) }
            else { Ok(json!({"entries":results})) }
        }
        "cache.clean" => {
            let mut removed = Vec::new();
            for digest in digests {
                let path = cache.root().join(&digest);
                match fs::symlink_metadata(&path) {
                    Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
                        fs::remove_dir_all(path).map_err(io_error)?;
                        removed.push(digest);
                    }
                    Ok(_) => return Err(CliError::new("ARTIFACT_INVALID", "cache entry is not a regular directory")),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {},
                    Err(error) => return Err(io_error(error)),
                }
            }
            Ok(json!({"removed":removed}))
        }
        _ => Err(invalid(format!("unknown cache operation {op}"))),
    }
}

fn io_error(error: io::Error) -> CliError { CliError::new("IO_ERROR", error.to_string()) }
fn render_error(error: RenderError) -> CliError {
    let mut result = CliError::new(error.code, error.message);
    result.details = error.details;
    result
}
