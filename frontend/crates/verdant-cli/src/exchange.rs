//! In-process score interchange, with explicit admission of every conversion
//! report and fresh destinations. No source tool or synthesis engine is run.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use verdant_domain::{Project, ProjectPersistence, ProjectStore};
use verdant_exchange::{ExportOptions, Format, ImportOptions, Limits};

use crate::CliError;
use crate::project::{Choices, Params, copy_assets, native_output, persistence_error, read_store, require_fresh, stamp_creator, store_json};

#[derive(Clone, Copy)]
enum SourceFormat { Native, Score(Format) }

fn format(value: std::option::Option<&str>, path: &Path) -> Result<SourceFormat, CliError> {
    let extension = path.extension().and_then(|extension| extension.to_str());
    let reject_retired = |name: &str| {
        verdant_domain::io::reject_retired_project_format(name)
            .map_err(|error| CliError::new("INVALID_ARGUMENT", error.to_string()))
    };
    reject_retired(extension.unwrap_or_default())?;
    let name = value.or(extension)
        .ok_or_else(|| CliError::new("INVALID_ARGUMENT", "format is required without a recognized filename extension"))?;
    reject_retired(name)?;
    if ["native", "verp"].iter().any(|known| name.eq_ignore_ascii_case(known)) {
        verdant_domain::io::require_project_path(path)
            .map_err(|error| CliError::new("INVALID_ARGUMENT", error.to_string()))?;
        return Ok(SourceFormat::Native);
    }
    Format::from_extension(name).map(SourceFormat::Score)
        .ok_or_else(|| CliError::new("INVALID_ARGUMENT", format!("unsupported score format {name}")))
}

pub fn format_catalog() -> Value {
    json!({"import":["verp","midi","ust","ustx","svp","vsqx","vpr","ufdata"],
        "export":["verp","midi","ust","ustx","svp","vsqx","vpr","ufdata"],
        "default_loss_policy":"strict","loss_policies":["strict","allow"],"outputs":"fresh paths only"})
}

pub(crate) fn admit_losses(policy: &str, has_losses: bool, report: &Value, dry_run: bool) -> Result<(), CliError> {
    match policy {
        "strict" if has_losses && !dry_run => Err(CliError::new("DECISION_REQUIRED", "conversion has reported losses or unsupported semantics; inspect the report and explicitly choose loss_policy=allow")
            .with_details(json!({"report":report,"required_choice":{"loss_policy":["strict","allow"]}}))),
        "strict" | "allow" => Ok(()),
        _ => Err(CliError::new("INVALID_ARGUMENT", "loss_policy must be strict or allow")),
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, CliError> {
    let file = File::open(path).map_err(|error| CliError::new("IO_ERROR", format!("{}: {error}", path.display())))?;
    let mut bytes = Vec::new();
    file.take((limit as u64).saturating_add(1)).read_to_end(&mut bytes)
        .map_err(|error| CliError::new("IO_ERROR", format!("{}: {error}", path.display())))?;
    if bytes.len() > limit { return Err(CliError::new("RESOURCE_EXHAUSTED", format!("{} exceeds the {limit}-byte input limit", path.display()))); }
    Ok(bytes)
}

fn write_fresh(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| CliError::new("IO_ERROR", format!("{}: {error}", parent.display())))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| CliError::new("IO_ERROR", error.to_string()))?;
    temporary.write_all(bytes).and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| CliError::new("IO_ERROR", format!("{}: {error}", path.display())))?;
    temporary.persist_noclobber(path).map_err(|error| {
        let code = if error.error.kind() == std::io::ErrorKind::AlreadyExists { "OUTPUT_EXISTS" } else { "IO_ERROR" };
        CliError::new(code, format!("{}: {}", path.display(), error.error))
    })?;
    #[cfg(unix)]
    File::open(parent).and_then(|file| file.sync_all()).map_err(|error| CliError::new("IO_ERROR", format!("{}: {error}", parent.display())))?;
    Ok(())
}

fn report_bytes(report: &Value) -> Result<Vec<u8>, CliError> {
    let mut bytes = serde_json::to_vec_pretty(report).map_err(|error| CliError::new("INTERNAL_ERROR", error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn source_name(path: &Path) -> Option<String> {
    path.file_name().and_then(|name| name.to_str()).map(str::to_owned)
}

fn native_import(source: &Path, choices: &Choices) -> Result<(Project, Value, bool), CliError> {
    let (store, recovery) = read_store(source, choices, true)?;
    let has_losses = store.undo_depth() + store.redo_depth() > 0;
    let report = json!({"schema":"verdant-native-import/1","source":source,
        "source_project_id":store.project().project_id,"source_revision":store.revision().get(),
        "saved_revision":recovery.saved_revision,"recovered_revision":recovery.recovered_revision,
        "has_losses":has_losses,"source_unchanged":true,"issues":if has_losses {
            vec![json!({"path":"/history","disposition":"unsupported","detail":"Import establishes a new project identity and omits source undo/redo history. project.save_as preserves history for .verp sources."})]
        } else {Vec::<Value>::new()}});
    let mut project = store.project_for_save();
    project.project_id = uuid::Uuid::new_v4().to_string();
    project.revision = 0;
    Ok((project, report, has_losses))
}

fn import(mut params: Params) -> Result<Value, CliError> {
    let source: PathBuf = params.required("source")?;
    let output = native_output(params.required("output")?)?;
    let selected: Option<String> = params.optional("format")?;
    let options: Option<Value> = params.optional("options")?;
    let policy: String = params.optional("loss_policy")?.unwrap_or_else(|| "strict".into());
    let report_path: Option<PathBuf> = params.optional("report")?;
    let choices = Choices::take(&mut params)?;
    let dry_run: bool = params.optional("dry_run")?.unwrap_or(false);
    params.finish()?;
    require_fresh(&output)?;
    if let Some(path) = &report_path { require_fresh(path)?; }
    if report_path.as_ref() == Some(&output) { return Err(CliError::new("INVALID_ARGUMENT", "output and report must be distinct paths")); }
    let selected = format(selected.as_deref(), &source)?;
    let (mut project, report, has_losses) = match selected {
        SourceFormat::Native => {
            if options.as_ref().is_some_and(|options| options.as_object().is_none_or(|map| !map.is_empty())) {
                return Err(CliError::new("INVALID_ARGUMENT", "native import has no format options"));
            }
            native_import(&source, &choices)?
        },
        SourceFormat::Score(Format::Ust | Format::Ustx) => {
            let mut options: verdant_import::ImportOptions = options.map(serde_json::from_value).transpose()
                .map_err(|error| CliError::new("INVALID_ARGUMENT", format!("/options: {error}")))?.unwrap_or_default();
            if options.source_name.is_none() { options.source_name = source_name(&source); }
            let bytes = read_bounded(&source, Limits::default().max_file_bytes)?;
            let format = match selected { SourceFormat::Score(Format::Ust) => verdant_import::ImportFormat::Ust,
                _ => verdant_import::ImportFormat::Ustx };
            let result = verdant_import::import(format, &bytes, &options)
                .map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
            let has_losses = result.report.unsupported_fields().next().is_some() || !result.report.notices.is_empty();
            let report = serde_json::to_value(&result.report).map_err(|error| CliError::new("INTERNAL_ERROR", error.to_string()))?;
            (result.project, report, has_losses)
        },
        SourceFormat::Score(format) => {
            let mut options: ImportOptions = options.map(serde_json::from_value).transpose()
                .map_err(|error| CliError::new("INVALID_ARGUMENT", format!("/options: {error}")))?.unwrap_or_default();
            if options.source_name.is_none() { options.source_name = source_name(&source); }
            let bytes = read_bounded(&source, options.limits.max_file_bytes)?;
            let result = verdant_exchange::import(format, &bytes, &options)
                .map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
            let has_losses = result.report.has_losses();
            let report = serde_json::to_value(&result.report).map_err(|error| CliError::new("INTERNAL_ERROR", error.to_string()))?;
            (result.project, report, has_losses)
        },
    };
    stamp_creator(&mut project);
    let store = ProjectStore::new(project);
    store.validate_document().map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
    admit_losses(&policy, has_losses, &report, dry_run)?;
    let assets = if matches!(selected, SourceFormat::Native) { copy_assets(&store, &source, &output, dry_run)? } else { Value::Null };
    if dry_run {
        return Ok(json!({"source":source,"output":output,"dry_run":true,"has_losses":has_losses,
            "decision_required":has_losses&&policy=="strict","report":report,"project":store.project(),"assets":assets}));
    }
    let document = ProjectPersistence::create_from_store(&output, store).map_err(persistence_error)?;
    if let Some(path) = &report_path {
        write_fresh(path, &report_bytes(&report)?).map_err(|error| error.with_details(json!({
            "project_written":output,"report_not_completed":path,"report":report,
        })))?;
    }
    Ok(json!({"source":source,"output":output,"dry_run":false,"has_losses":has_losses,
        "state":store_json(document.store()),"report":report,"report_path":report_path,"assets":assets}))
}

fn export(mut params: Params) -> Result<Value, CliError> {
    let path: PathBuf = params.required("path")?;
    let output: PathBuf = params.required("output")?;
    let selected: Option<String> = params.optional("format")?;
    let options: Option<Value> = params.optional("options")?;
    let track_id: Option<verdant_domain::TrackId> = params.optional("track_id")?;
    let report_path: Option<PathBuf> = params.optional("report")?;
    let policy: String = params.optional("loss_policy")?.unwrap_or_else(|| "strict".into());
    let choices = Choices::take(&mut params)?;
    let dry_run: bool = params.optional("dry_run")?.unwrap_or(false);
    params.finish()?;
    require_fresh(&output)?;
    if let Some(path) = &report_path { require_fresh(path)?; }
    if report_path.as_ref() == Some(&output) { return Err(CliError::new("INVALID_ARGUMENT", "output and report must be distinct paths")); }
    let selected = format(selected.as_deref(), &output)?;
    let (store, _) = read_store(&path, &choices, true)?;
    // Native distribution export omits recovery history. Collect assets from
    // this current snapshot, so removed historical media is not exported.
    let snapshot = ProjectStore::new(store.project_for_save());
    let project = snapshot.project();
    let (bytes, report, has_losses) = match selected {
        SourceFormat::Native => {
            if options.as_ref().is_some_and(|options| options.as_object().is_none_or(|map| !map.is_empty())) || track_id.is_some() {
                return Err(CliError::new("INVALID_ARGUMENT", "native export has no format options or single-track selection"));
            }
            let has_losses = store.undo_depth() + store.redo_depth() > 0;
            let report = json!({"schema":"verdant-native-export/1","has_losses":has_losses,
                "history_exported":false,"issues":if has_losses {
                    vec![json!({"path":"/history","disposition":"unsupported","detail":"Distribution export writes the native document without a recovery journal. Use project.save_as to preserve undo/redo history."})]
                } else { Vec::<Value>::new() }});
            (None, report, has_losses)
        },
        SourceFormat::Score(format) => {
            let mut options: ExportOptions = options.map(serde_json::from_value).transpose()
                .map_err(|error| CliError::new("INVALID_ARGUMENT", format!("/options: {error}")))?.unwrap_or_default();
            if let Some(id) = track_id {
                let index = project.tracks.iter().position(|track| track.id == id)
                    .ok_or_else(|| CliError::new("INVALID_ARGUMENT", format!("unknown track ID {id}")))?;
                if options.track.is_some_and(|selected| selected != index) {
                    return Err(CliError::new("INVALID_ARGUMENT", "track_id conflicts with options.track"));
                }
                options.track = Some(index);
            }
            let result = verdant_exchange::export(format, project, &options)
                .map_err(|error| CliError::new("INVALID_PROJECT", error.to_string()))?;
            let has_losses = result.report.has_losses();
            let report = serde_json::to_value(result.report).map_err(|error| CliError::new("INTERNAL_ERROR", error.to_string()))?;
            (Some(result.bytes), report, has_losses)
        },
    };
    admit_losses(&policy, has_losses, &report, dry_run)?;
    let assets = if matches!(selected, SourceFormat::Native) { copy_assets(&snapshot, &path, &output, dry_run)? } else { Value::Null };
    if dry_run {
        return Ok(json!({"output":output,"dry_run":true,"has_losses":has_losses,"decision_required":has_losses&&policy=="strict",
            "byte_length":bytes.as_ref().map(Vec::len),"report":report,"source":store_json(&store),"assets":assets}));
    }
    match bytes.as_deref() {
        Some(bytes) => write_fresh(&output, bytes)?,
        None => ProjectPersistence::export_snapshot(&output, project).map_err(persistence_error)?,
    }
    if let Some(path) = &report_path {
        write_fresh(path, &report_bytes(&report)?).map_err(|error| error.with_details(json!({
            "output_written":output,"report_not_completed":path,"report":report,
        })))?;
    }
    Ok(json!({"output":output,"dry_run":false,"has_losses":has_losses,"report":report,"report_path":report_path,
        "source":store_json(&store),"assets":assets}))
}

pub fn dispatch(op: &str, params: Value) -> Option<Result<Value, CliError>> {
    let operation = match op { "import" => import, "export.project" => export, _ => return None };
    Some(Params::new(params).and_then(operation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_import_export_are_fresh_and_leave_source_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("original.verp");
        let project = Project::new_empty();
        verdant_domain::io::save_project_atomic(&source, &project).unwrap();
        let before = std::fs::read(&source).unwrap();
        let copy = temp.path().join("copy.verp");
        dispatch("export.project", json!({"path":source,"output":copy})).unwrap().unwrap();
        let copy_before = std::fs::read(&copy).unwrap();
        let imported = temp.path().join("imported.verp");
        dispatch("import", json!({"source":copy,"output":imported})).unwrap().unwrap();
        let read = ProjectPersistence::read_saved(&imported).unwrap();
        assert_ne!(read.project_id, project.project_id);
        assert_eq!(read.tracks, project.tracks);
        assert_eq!(read.extensions["verdant.created_with"], env!("CARGO_PKG_VERSION"));
        let error = dispatch("export.project", json!({"path":source,"output":copy})).unwrap().unwrap_err();
        assert_eq!(error.code, "OUTPUT_EXISTS");
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert_eq!(std::fs::read(&copy).unwrap(), copy_before);
        assert!(!verdant_domain::persistence::journal_path(&source).exists());
    }

    #[test]
    fn retired_project_paths_and_explicit_formats_are_rejected_by_exchange() {
        let temp = tempfile::tempdir().unwrap();
        let native = temp.path().join("native.verp");
        let output = temp.path().join("out.verp");
        let report = temp.path().join("report.json");
        for (op, params) in [
            ("import", json!({"source":temp.path().join("old.gdmproj"),"output":output,"report":report})),
            ("import", json!({"source":temp.path().join("old.verdant.JSON"),"format":"midi","output":output,"report":report})),
            ("import", json!({"source":native,"format":".GDMProj","output":output,"report":report})),
            ("export.project", json!({"path":temp.path().join("old.json"),"output":output,"report":report})),
            ("export.project", json!({"path":native,"output":temp.path().join("out.gdmproj"),"report":report})),
            ("export.project", json!({"path":native,"format":"verdant.json","output":output,"report":report})),
        ] {
            let error = dispatch(op, params).unwrap().unwrap_err();
            assert_eq!(error.message, verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE);
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn strict_loss_reports_are_available_before_any_output_is_published() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.verp");
        let mut project = Project::new_empty();
        project.extensions.insert("unsupported-test-control".into(), json!({"value":42}));
        verdant_domain::io::save_project_atomic(&source, &project).unwrap();
        let output = temp.path().join("score.mid");
        let error = dispatch("export.project", json!({"path":source,"output":output})).unwrap().unwrap_err();
        assert_eq!(error.code, "DECISION_REQUIRED");
        assert!(error.details.unwrap()["report"]["issues"].as_array().is_some_and(|issues| !issues.is_empty()));
        let preview = dispatch("export.project", json!({"path":source,"output":output,"dry_run":true})).unwrap().unwrap();
        assert_eq!(preview["decision_required"], true);
        assert!(!output.exists());
        assert!(!verdant_domain::persistence::journal_path(&source).exists());
        assert!(!verdant_domain::persistence::lock_path(&source).exists());
    }
}
