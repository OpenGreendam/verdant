use std::{env, path::{Path, PathBuf}};
use serde::Deserialize;
use serde_json::{Value, json};
use verdant_agent_skills::{Bundle, Error};
use crate::{CliError, Outcome, invalid};

pub(crate) const OPERATIONS: &[&str] = &["skill.list", "skill.show", "skill.install", "skill.uninstall"];

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    name: Option<String>,
    #[serde(default)] all: bool,
    bundle: Option<PathBuf>,
    agent: Option<String>,
    scope: Option<String>,
    project_root: Option<PathBuf>,
    dest: Option<PathBuf>,
    #[serde(default)] dry_run: bool,
}

fn error(error: Error) -> CliError { CliError::new(error.code, error.message) }

fn bundle_path(explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit { return Ok(path.to_owned()); }
    if let Some(path) = env::var_os("VERDANT_SKILLS_DIR") { return Ok(path.into()); }
    let packaged = verdant_backend::platform::resource_dir().map(|root| root.join("skills"));
    if let Some(root) = &packaged {
        if root.join("manifest.json").is_file() { return Ok(root.clone()); }
    }
    #[cfg(debug_assertions)] {
        let development = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills");
        if development.join("manifest.json").is_file() {
            return development.canonicalize().map_err(|error| CliError::new("IO_ERROR", error.to_string()));
        }
    }
    Err(CliError::new("SKILL_BUNDLE_UNAVAILABLE",
        format!("offline skill bundle not found at {}; set VERDANT_SKILLS_DIR or use --bundle DIR", packaged.as_ref().map(|path| path.display().to_string()).unwrap_or_else(|| "share/verdant/skills".into()))))
}

fn destination(params: &Params) -> Result<PathBuf, CliError> {
    if let Some(path) = &params.dest {
        if params.agent.is_some() || params.scope.is_some() || params.project_root.is_some() {
            return Err(invalid("--dest is an explicit skills directory; do not combine it with --agent, --scope or --project-root"));
        }
        return if path.is_absolute() { Ok(path.clone()) } else {
            env::current_dir().map(|root| root.join(path)).map_err(|error| CliError::new("IO_ERROR", error.to_string()))
        };
    }
    let directory = match params.agent.as_deref().unwrap_or("generic") {
        "generic" => ".agents/skills",
        "codex" => ".codex/skills",
        "claude" => ".claude/skills",
        _ => return Err(invalid("--agent must be generic, codex or claude")),
    };
    let root = match params.scope.as_deref().unwrap_or("project") {
        "project" => params.project_root.clone().map(Ok).unwrap_or_else(env::current_dir)
            .map_err(|error| CliError::new("IO_ERROR", error.to_string()))?,
        "user" | "global" => {
            if params.project_root.is_some() { return Err(invalid("--project-root requires project scope")); }
            #[cfg(windows)] let home = env::var_os("USERPROFILE");
            #[cfg(not(windows))] let home = env::var_os("HOME");
            home.map(PathBuf::from).ok_or_else(|| invalid("home directory is unavailable; use --dest"))?
        },
        _ => return Err(invalid("--scope must be project or user (global is an alias for user)")),
    };
    if !root.is_dir() { return Err(invalid(format!("project/home root is not a directory: {}", root.display()))); }
    let root = if root.is_absolute() { root } else {
        env::current_dir().map_err(|error| CliError::new("IO_ERROR", error.to_string()))?.join(root)
    };
    Ok(root.join(directory))
}

fn names(params: &Params, bundle: &Bundle, required: bool) -> Result<Vec<String>, CliError> {
    if params.all && params.name.is_some() { return Err(invalid("choose a skill name or --all")); }
    if let Some(name) = &params.name {
        if !bundle.manifest().skills.iter().any(|skill| &skill.name == name) {
            return Err(invalid(format!("unknown bundled skill: {name}")));
        }
        return Ok(vec![name.clone()]);
    }
    if required && !params.all { return Err(invalid("choose a skill name or --all")); }
    Ok(bundle.manifest().skills.iter().map(|skill| skill.name.clone()).collect())
}

pub(crate) fn dispatch(op: &str, value: Value) -> Outcome {
    let params: Params = serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;
    if params.name.is_some() && params.all { return Err(invalid("choose a skill name or --all")); }
    if op == "skill.uninstall" {
        if params.bundle.is_some() { return Err(invalid("uninstall uses managed receipts and does not take --bundle")); }
        if !params.all && params.name.is_none() { return Err(invalid("choose a skill name or --all")); }
        let destination = destination(&params)?;
        let selected = params.name.as_ref().map(|name| vec![name.clone()]);
        let actions = verdant_agent_skills::uninstall(&destination, selected.as_deref(), params.dry_run).map_err(error)?;
        return Ok(json!({"destination":destination,"dry_run":params.dry_run,"skills":actions}));
    }
    if !matches!(op, "skill.list" | "skill.show" | "skill.install") { return Err(invalid("unknown skill operation")); }
    if op != "skill.install" && (params.dest.is_some() || params.agent.is_some() || params.scope.is_some()
        || params.project_root.is_some() || params.dry_run) {
        return Err(invalid("destination and dry-run options only apply to install/uninstall"));
    }
    let source = bundle_path(params.bundle.as_deref())?;
    let bundle = Bundle::load(&source).map_err(error)?;
    let selected = names(&params, &bundle, op != "skill.list")?;
    if op == "skill.install" {
        let destination = destination(&params)?;
        let actions = bundle.install(&destination, &selected, params.dry_run).map_err(error)?;
        return Ok(json!({"version":bundle.manifest().version,"bundle_sha256":bundle.manifest_sha256(),
            "destination":destination,"dry_run":params.dry_run,"skills":actions}));
    }
    let skills = selected.iter().map(|name| {
        let skill = bundle.manifest().skills.iter().find(|skill| &skill.name == name).expect("validated name");
        let mut value = json!({"name":skill.name,"description":skill.description,"files":skill.files});
        if op == "skill.show" {
            value["content"] = json!(std::str::from_utf8(bundle.read(name, "SKILL.md").expect("validated bundle")).expect("validated UTF-8"));
        }
        value
    }).collect::<Vec<_>>();
    Ok(json!({"schema":bundle.manifest().schema,"version":bundle.manifest().version,
        "bundle_sha256":bundle.manifest_sha256(),"source":source,"skills":skills}))
}

pub(crate) fn schema(op: &str) -> Option<Value> {
    if !OPERATIONS.contains(&op) { return None; }
    let mut fields = serde_json::Map::from_iter([
        ("name".into(), json!({"type":"string","pattern":"^[a-z0-9]+(-[a-z0-9]+)*$","maxLength":64})),
        ("all".into(), json!({"type":"boolean"})),
    ]);
    if op != "skill.uninstall" { fields.insert("bundle".into(), json!({"type":"string","description":"Explicit offline bundle directory"})); }
    if matches!(op, "skill.install" | "skill.uninstall") {
        fields.extend([
            ("agent".into(), json!({"enum":["generic","codex","claude"]})),
            ("scope".into(), json!({"enum":["project","user","global"]})),
            ("project_root".into(), json!({"type":"string"})),
            ("dest".into(), json!({"type":"string"})),
            ("dry_run".into(), json!({"type":"boolean"})),
        ]);
    }
    let mut schema = json!({"op":op,"params":{"type":"object","additionalProperties":false,"properties":fields}});
    if op != "skill.list" {
        schema["params"]["oneOf"] = json!([
            {"required":["name"],"not":{"required":["all"]}},
            {"required":["all"],"properties":{"all":{"const":true}},"not":{"required":["name"]}},
        ]);
    }
    Some(schema)
}

pub(crate) fn positionals(op: &str) -> Option<&'static [&'static str]> {
    OPERATIONS.contains(&op).then_some(&["name"][..])
}
