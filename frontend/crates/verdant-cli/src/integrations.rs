//! Headless media admission and native effect discovery. Persisted edits use
//! the same domain batches, revision admission and project writer as apply.
//! Inspect/catalog/status never instantiate a plugin or authorize saved paths.

use std::{collections::BTreeSet, fs::{self, File}, io::Read, path::{Path, PathBuf}};

use serde::Deserialize;
use serde_json::{Value, json};
use verdant_domain::{Command, CommandBatch, Project, Revision, TrackId};
use verdant_domain::automation::{AutomationDocument, AutomationOwner, MASTER_INSERTS_KEY};
use verdant_media::{AssetPolicy, AudioClip, ImportLimits, MediaError, MediaImporter};
use verdant_plugins::{InsertSpec, ScanReport, scan};

use crate::{CliError, project::Params};

type Outcome = Result<Value, CliError>;

const MAX_SCAN_CACHE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(serde::Serialize)]
struct Mutation {
    path: PathBuf,
    expect_revision: Revision,
    #[serde(skip_serializing_if = "Option::is_none")]
    expect_project_id: Option<std::string::String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery: Option<std::string::String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_creator: Option<std::string::String>,
    dry_run: bool,
}

impl Mutation {
    fn take(params: &mut Params) -> Result<Self, CliError> {
        Ok(Self {
            path: params.required("path")?,
            expect_revision: params.required("expect_revision")?,
            expect_project_id: params.optional("expect_project_id")?,
            recovery: params.optional("recovery")?,
            allowed_creator: params.optional("allowed_creator")?,
            dry_run: params.optional("dry_run")?.unwrap_or(false),
        })
    }

    fn snapshot(&mut self) -> Result<Project, CliError> {
        let project = crate::project::read_snapshot(&self.path, self.recovery.as_deref(), self.allowed_creator.as_deref())?;
        if self.expect_project_id.as_ref().is_some_and(|id| id != &project.project_id) {
            return Err(CliError::new("REVISION_CONFLICT", "project identity differs from expect_project_id"));
        }
        // Bind a batch compiled from this snapshot to its document identity,
        // including when the caller only supplied the mandatory revision.
        self.expect_project_id = Some(project.project_id.clone());
        Ok(project)
    }

    fn apply(self, batch: CommandBatch) -> Outcome {
        crate::project::apply_domain_batch(serde_json::to_value(self).map_err(crate::internal)?, batch)
    }
}

struct Selection {
    path: PathBuf,
    recovery: Option<std::string::String>,
    allowed_creator: Option<std::string::String>,
    track: Option<TrackId>,
    clip_id: Option<uuid::Uuid>,
}

impl Selection {
    fn take(params: &mut Params) -> Result<Self, CliError> {
        Ok(Self {
            path: params.required("path")?, recovery: params.optional("recovery")?,
            allowed_creator: params.optional("allowed_creator")?, track: params.optional("track")?,
            clip_id: params.optional("clip_id")?,
        })
    }

    fn read(&self) -> Result<(Project, Vec<AudioClip>), CliError> {
        let project = crate::project::read_snapshot(&self.path, self.recovery.as_deref(), self.allowed_creator.as_deref())?;
        if let Some(track) = &self.track {
            if project.track(track).is_none() { return Err(invalid(format!("unknown track {track}"))); }
        }
        let mut clips = Vec::new();
        for track in &project.tracks {
            if self.track.as_ref().is_some_and(|id| id != &track.id) { continue; }
            clips.extend(verdant_media::read_clips(track).map_err(media_error)?.into_iter()
                .filter(|clip| self.clip_id.is_none_or(|id| id == clip.id)));
        }
        if self.clip_id.is_some() && clips.is_empty() { return Err(invalid("unknown clip_id in the selected tracks")); }
        Ok((project, clips))
    }
}

fn import(mut params: Params) -> Outcome {
    let mut mutation = Mutation::take(&mut params)?;
    let track: TrackId = params.required("track")?;
    let source: PathBuf = params.required("source")?;
    let start_tick = params.optional("start_tick")?.unwrap_or(0);
    let label = params.optional("label")?.unwrap_or_else(|| "Import audio clip".into());
    params.finish()?;
    if start_tick < 0 { return Err(invalid("start_tick must be nonnegative")); }
    let project = mutation.snapshot()?;
    let target = project.track(&track).ok_or_else(|| invalid(format!("unknown track {track}")))?;
    let mut clips = verdant_media::read_clips(target).map_err(media_error)?;
    let imported = import_selected(&source)?;
    let clip = AudioClip::new(imported.source, track.clone(), start_tick);
    drop(imported.audio);
    clip.validate().map_err(media_error)?;
    let imported_clip = json!(clip);
    clips.push(clip);
    let batch = clip_batch(track, &clips, label)?;
    let mut result = mutation.apply(batch)?;
    result["clip"] = imported_clip;
    Ok(result)
}

fn relink(mut params: Params) -> Outcome {
    let mut mutation = Mutation::take(&mut params)?;
    let track: TrackId = params.required("track")?;
    let clip_id: uuid::Uuid = params.required("clip_id")?;
    let source: PathBuf = params.required("source")?;
    let label = params.optional("label")?.unwrap_or_else(|| "Relink audio source".into());
    params.finish()?;
    let project = mutation.snapshot()?;
    let target = project.track(&track).ok_or_else(|| invalid(format!("unknown track {track}")))?;
    let mut clips = verdant_media::read_clips(target).map_err(media_error)?;
    let clip = clips.iter_mut().find(|clip| clip.id == clip_id).ok_or_else(|| invalid("unknown clip_id in track"))?;
    let imported = import_selected(&source)?;
    // A relink only changes location. Encoded bytes and every decoded format
    // field must still agree, so trim and automation/history retain meaning.
    clip.source.matches(&imported.audio).map_err(|error| media_error(error)
        .with_details(json!({"track":track,"clip_id":clip_id,"source":clip.source})))?;
    clip.source = imported.source;
    drop(imported.audio);
    clip.validate().map_err(media_error)?;
    let linked = json!(clip);
    let batch = clip_batch(track, &clips, label)?;
    let mut result = mutation.apply(batch)?;
    result["clip"] = linked;
    Ok(result)
}

fn import_selected(path: &Path) -> Result<verdant_media::ImportedAudio, CliError> {
    // A source argument is one explicitly selected file, never authority for
    // its siblings or for any other absolute path saved in the document.
    let path = path.canonicalize().map_err(asset_io)?;
    let policy = AssetPolicy::for_file(&path).map_err(media_error)?;
    MediaImporter::new(policy, ImportLimits::default()).import_file(&path).map_err(media_error)
}

fn clip_batch(track: TrackId, clips: &[AudioClip], label: String) -> Result<CommandBatch, CliError> {
    let value = verdant_media::clips_value(track.clone(), clips).map_err(media_error)?;
    Ok(CommandBatch::single(label, Command::SetTrackData { track, key: verdant_media::AUDIO_CLIPS_KEY.into(), value }))
}

fn inspect(mut params: Params) -> Outcome {
    let selection = Selection::take(&mut params)?;
    params.finish()?;
    let (project, clips) = selection.read()?;
    Ok(json!({"path":selection.path,"project_id":project.project_id,"revision":project.revision,
        "clip_count":clips.len(),"clips":clips,"sources_checked":false}))
}

fn verify(mut params: Params) -> Outcome {
    let selection = Selection::take(&mut params)?;
    let roots: Vec<PathBuf> = params.optional("asset_roots")?.unwrap_or_default();
    let paths: Vec<PathBuf> = params.optional("asset_paths")?.unwrap_or_default();
    params.finish()?;
    let (project, clips) = selection.read()?;
    let mut policy = AssetPolicy::new(roots).map_err(media_error)?;
    for path in paths { policy.allow_file(&path).map_err(media_error)?; }
    let policy = verdant_service::media::project_policy(&selection.path, policy).map_err(media_error)?;
    let importer = MediaImporter::new(policy, ImportLimits::default());
    let mut seen = BTreeSet::new();
    let mut sources = Vec::new();
    let mut failure = None;
    for clip in &clips {
        let source = &clip.source;
        let key = (source.path.as_str(), source.sha256.as_str(), source.byte_len,
            source.sample_rate, source.channels, source.frames);
        if !seen.insert(key) { continue; }
        let checked = importer.load_source(source);
        match checked {
            Ok(audio) => sources.push(json!({"source":source,"verified":true,"duration_seconds":audio.duration_seconds()})),
            Err(error) => {
                let error = media_error(error);
                failure.get_or_insert(error.code);
                sources.push(json!({"source":source,"verified":false,"error":error,"track":clip.track,"clip_id":clip.id}));
            }
        }
    }
    let report = json!({"path":selection.path,"project_id":project.project_id,"revision":project.revision,
        "verified":failure.is_none(),"clip_count":clips.len(),"source_count":sources.len(),"sources":sources});
    if let Some(code) = failure {
        return Err(CliError::new(code, "one or more media sources failed verification").with_details(report));
    }
    Ok(report)
}

fn scan_effects(mut params: Params) -> Outcome {
    let plughost: PathBuf = params.required("plughost")?;
    let cache_dir: PathBuf = params.required("cache_dir")?;
    let paths: Vec<PathBuf> = params.required("paths")?;
    params.finish()?;
    if paths.is_empty() || paths.len() > 256 { return Err(invalid("paths requires 1–256 explicit plugin files or directories")); }
    // Native discovery executes code. Only the ordinary operator CLI accepts
    // these explicit installed-host/library paths; the bot bridge excludes it.
    // Never resolve a host from an uploaded project, PATH or an implicit scan.
    let plughost = plughost.canonicalize().map_err(|error| unsupported_host(error.to_string()))?;
    let metadata = fs::metadata(&plughost).map_err(|error| unsupported_host(error.to_string()))?;
    if !metadata.is_file() { return Err(unsupported_host("plughost must be an installed regular executable")); }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 { return Err(unsupported_host("plughost is not executable")); }
    }
    let mut admitted = Vec::with_capacity(paths.len());
    for path in paths {
        // Preserve the explicitly selected bundle name: native format
        // detection still needs a .clap/.vst3 suffix on a symlinked install.
        let path = std::path::absolute(path).map_err(asset_io)?;
        let metadata = fs::metadata(&path).map_err(asset_io)?;
        if !metadata.is_file() && !metadata.is_dir() { return Err(invalid("plugin paths must be regular files or directories")); }
        if metadata.is_file() && scan::discover_candidates(std::slice::from_ref(&path)).is_empty() {
            return Err(CliError::new("UNSUPPORTED_CAPABILITY", "explicit plugin file has no supported native plugin extension"));
        }
        if !admitted.contains(&path) { admitted.push(path); }
    }
    fs::create_dir_all(&cache_dir).map_err(io_error)?;
    let cache_dir = cache_dir.canonicalize().map_err(io_error)?;
    let summary = scan::scan_paths(&plughost, &cache_dir, &admitted);
    let discovered = summary.discovered.iter().map(|(path, plugin)| json!({"path":path,"descriptor":plugin})).collect::<Vec<_>>();
    let blacklisted = summary.blacklisted.iter().map(|(path, reason)| json!({"path":path,"error":reason})).collect::<Vec<_>>();
    let cached = read_scan_cache(&cache_dir)?;
    let recorded = cached.as_deref().unwrap_or_default();
    let catalog_persisted = summary.discovered.iter().all(|(path, plugin)| recorded.iter().any(|report|
        report.path == *path && report.error.is_none() && report.plugins.contains(plugin)));
    let report = json!({"plughost":plughost,"cache_dir":cache_dir,"paths":admitted,"cached":summary.cached,
        "all_probes_succeeded":blacklisted.is_empty(),"discovered":discovered,"blacklisted":blacklisted,
        "catalog_persisted":catalog_persisted});
    if !catalog_persisted {
        return Err(CliError::new("IO_ERROR", "effect discovery completed but its catalog could not be persisted").with_details(report));
    }
    Ok(report)
}

#[derive(Deserialize)]
struct ScanCache { entries: Vec<ScanReport> }

fn catalog(mut params: Params) -> Outcome {
    let cache_dir: PathBuf = params.required("cache_dir")?;
    params.finish()?;
    let reports = read_scan_cache(&cache_dir)?;
    let found = reports.is_some();
    let reports = reports.unwrap_or_default();
    let mut plugins = Vec::new();
    let mut blacklisted = Vec::new();
    for report in &reports {
        if let Some(error) = &report.error {
            blacklisted.push(json!({"path":report.path,"sha256":report.sha256,"error":error}));
        } else {
            for plugin in &report.plugins {
                plugins.push(json!({"path":report.path,"sha256":report.sha256,"descriptor":plugin}));
            }
        }
    }
    let builtin_effects = std::iter::once(verdant_plugins::BUILTIN_GAIN_ID)
        .chain(verdant_plugins::builtin_dsp::EFFECT_IDS)
        .map(|id| json!({"format":"builtin","id":id,
            "name":verdant_plugins::builtin_dsp::display_name(id).unwrap_or("Verdant Gain"),
            "requires_plughost":true,"native_editor":"gpui"})).collect::<Vec<_>>();
    let builtin_instruments = [verdant_plugins::builtin_dsp::SYNTH_ID, verdant_plugins::builtin_dsp::ONE_SYNTH_ID]
        .map(|id| json!({"format":"builtin","id":id,"name":verdant_plugins::builtin_dsp::instrument_name(id),
            "requires_plughost":false,"native_editor":"gpui"}));
    Ok(json!({"cache_dir":cache_dir,"cache_found":found,"verification":"cached_not_loaded",
        "plugins":plugins,"blacklisted":blacklisted,"reports":reports,
        "builtin_effects":builtin_effects,"builtin_instruments":builtin_instruments}))
}

fn read_scan_cache(cache_dir: &Path) -> Result<Option<Vec<ScanReport>>, CliError> {
    let path = cache_dir.join("plugin-scan.json");
    let file = match File::open(&path) {
        Ok(file) => Some(file),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io_error(error)),
    };
    if let Some(file) = file {
        if !file.metadata().map_err(io_error)?.is_file() { return Err(invalid("plugin scan cache must be a regular file")); }
        let mut bytes = Vec::new();
        file.take(MAX_SCAN_CACHE_BYTES + 1).read_to_end(&mut bytes).map_err(io_error)?;
        if bytes.len() as u64 > MAX_SCAN_CACHE_BYTES {
            return Err(CliError::new("RESOURCE_EXHAUSTED", "plugin scan cache exceeds 16 MiB"));
        }
        let cache = serde_json::from_slice::<ScanCache>(&bytes)
            .map_err(|error| CliError::new("ARTIFACT_INVALID", format!("plugin scan cache: {error}")))?;
        Ok(Some(cache.entries))
    } else { Ok(None) }
}

fn effect_status(mut params: Params) -> Outcome {
    let path: PathBuf = params.required("path")?;
    let recovery: Option<String> = params.optional("recovery")?;
    let allowed_creator: Option<String> = params.optional("allowed_creator")?;
    let owner: Option<AutomationOwner> = params.optional("owner")?;
    let instance_id: Option<String> = params.optional("instance_id")?;
    params.finish()?;
    let project = crate::project::read_snapshot(&path, recovery.as_deref(), allowed_creator.as_deref())?;
    if let Some(AutomationOwner::Track { track_id }) = &owner {
        if project.track(track_id).is_none() { return Err(invalid(format!("unknown track {track_id}"))); }
    }
    if instance_id.as_ref().is_some_and(String::is_empty) { return Err(invalid("instance_id must be nonempty")); }
    let mut inserts = Vec::new();
    for track in &project.tracks {
        let track_owner = AutomationOwner::Track { track_id: track.id.clone() };
        if owner.as_ref().is_some_and(|owner| owner != &track_owner) { continue; }
        collect_insert_status(&mut inserts, &track_owner, track.unknown.get(verdant_plugins::project::INSERTS_KEY), instance_id.as_deref())?;
    }
    if owner.as_ref().is_none_or(|owner| owner == &AutomationOwner::Master) {
        collect_insert_status(&mut inserts, &AutomationOwner::Master, project.extensions.get(MASTER_INSERTS_KEY), instance_id.as_deref())?;
    }
    if instance_id.is_some() && inserts.is_empty() { return Err(invalid("unknown instance_id in the selected owners")); }
    let automation = AutomationDocument::from_project(&project).map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
    let automation = automation.lanes.into_iter().filter(|lane| {
        let verdant_domain::automation::AutomationTarget::PluginParameter { owner: lane_owner, instance_id: lane_id, .. } = &lane.target else { return false; };
        owner.as_ref().is_none_or(|owner| owner == lane_owner) && instance_id.as_ref().is_none_or(|id| id == lane_id)
    }).collect::<Vec<_>>();
    Ok(json!({"path":path,"project_id":project.project_id,"revision":project.revision,"native_code_loaded":false,
        "inserts":inserts,"automation":automation}))
}

fn collect_insert_status(output: &mut Vec<Value>, owner: &AutomationOwner, value: Option<&Value>, instance_id: Option<&str>) -> Result<(), CliError> {
    let Some(value) = value else { return Ok(()); };
    let specs = Vec::<InsertSpec>::deserialize(value)
        .map_err(|error| CliError::new("INVALID_PROJECT", format!("persisted effect inserts: {error}")))?;
    for (slot, spec) in specs.into_iter().enumerate() {
        if instance_id.is_some_and(|id| id != spec.instance_id) { continue; }
        output.push(json!({"owner":owner,"slot":slot,"spec":spec,"runtime_status":"not_loaded"}));
    }
    Ok(())
}

pub(crate) fn media_error(error: MediaError) -> CliError {
    let code = match &error {
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => "MISSING_ASSET",
        MediaError::Io(_) => "IO_ERROR",
        MediaError::UnauthorizedPath => "DECISION_REQUIRED",
        MediaError::SourceChanged | MediaError::Decode(_) => "ARTIFACT_INVALID",
        MediaError::Limit(_) => "RESOURCE_EXHAUSTED",
        MediaError::Channels(_) => "UNSUPPORTED_CAPABILITY",
        MediaError::Json(_) => "INVALID_PROJECT",
        MediaError::InvalidPath | MediaError::NonUnicodePath | MediaError::Invalid(_) => "INVALID_ARGUMENT",
    };
    let mut failure = CliError::new(code, error.to_string());
    if matches!(error, MediaError::UnauthorizedPath) {
        failure = failure.with_details(json!({"suggested_action":"authorize the selected sources with asset_paths or asset_roots; portable relative sources resolve within the project directory"}));
    }
    failure
}

fn invalid(message: impl Into<String>) -> CliError { CliError::new("INVALID_ARGUMENT", message) }
fn io_error(error: std::io::Error) -> CliError { CliError::new("IO_ERROR", error.to_string()) }
fn asset_io(error: std::io::Error) -> CliError {
    CliError::new(if error.kind() == std::io::ErrorKind::NotFound { "MISSING_ASSET" } else { "IO_ERROR" }, error.to_string())
}
fn unsupported_host(message: impl Into<String>) -> CliError { CliError::new("UNSUPPORTED_CAPABILITY", message) }

pub fn dispatch(op: &str, params: Value) -> Option<Result<Value, crate::CliError>> {
    let operation: fn(Params) -> Outcome = match op {
        "media.import" => import,
        "media.relink" => relink,
        "media.inspect" => inspect,
        "media.verify" => verify,
        "effect.scan" => scan_effects,
        "effect.catalog" => catalog,
        "effect.status" => effect_status,
        _ => return None,
    };
    Some(Params::new(params).and_then(operation))
}
