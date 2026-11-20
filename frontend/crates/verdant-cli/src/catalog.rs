use serde_json::{Value, json};

use crate::{Outcome, invalid, protocol::{MAX_REQUEST_BYTES, PROTOCOL}};

pub const FAMILIES: &[&str] = &["project", "history", "export", "job", "cache", "voice", "effect", "media"];
const FORMATS: &[&str] = &["ust", "ustx", "ufdata", "midi", "svp", "vsqx", "vpr"];
const OPERATIONS: &[&str] = &[
    "capabilities", "schema", "doctor", "project.new", "project.inspect", "project.validate",
    "project.apply", "project.save-as", "project.recover", "project.migrate", "project.pack", "project.unpack",
    "history.list", "history.undo", "history.redo", "import", "export.project", "render.plan", "render", "export.audio",
    "job.list", "job.inspect", "job.cancel", "job.retry", "job.verify", "cache.list", "cache.verify", "cache.clean",
    "voice.list", "voice.verify", "voice.convert", "voice.install", "voice.remove", "voice.set-default", "voice.import-originals", "batch",
    "media.import", "media.relink", "media.inspect", "media.verify", "effect.scan", "effect.catalog", "effect.status",
];

pub fn runtime_features() -> Value {
    json!({"schema":"verdant-runtime-features/1","role":"cli","version":env!("CARGO_PKG_VERSION"),
        "protocol":PROTOCOL,"headless":true,"commands":["request","serve","render","project","voice"]})
}

pub fn capabilities(params: Value) -> Outcome {
    if params.as_object().is_none_or(|params| !params.is_empty()) {
        return Err(invalid("capabilities takes no parameters"));
    }
    Ok(json!({"protocol":PROTOCOL,"version":env!("CARGO_PKG_VERSION"),"worker_protocol":verdant_backend::wire::PROTOCOL,
        "operations":OPERATIONS,"formats":FORMATS,"format_capabilities":crate::exchange::format_catalog(),
        "project_operations":crate::project::operation_catalog(),"max_request_bytes":MAX_REQUEST_BYTES,
        "audio":{"sample_rates":[44100,48000,88200,96000,192000],"channels":[1,2],"encodings":["Float32","Pcm16","Pcm24"]},
        "device":{"playback":false,"capture":false,"midi":false},"stdio":{"max_active_requests":1}}))
}

pub fn schema(params: Value) -> Outcome {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query { op: Option<String> }
    let query: Query = serde_json::from_value(params).map_err(|error| invalid(error.to_string()))?;
    match query.op {
        Some(op) if OPERATIONS.contains(&op.as_str()) => Ok(operation_schema(&op)),
        Some(op) => Err(invalid(format!("unknown operation {op}"))),
        None => Ok(json!({"protocol":PROTOCOL,"request":{"type":"object","additionalProperties":false,
            "required":["protocol","request_id","op","params"],"properties":{
                "protocol":{"const":PROTOCOL},"request_id":{"type":"string","minLength":1,"maxLength":128},
                "op":{"enum":OPERATIONS},"params":{"type":"object"}}},
                "operations":OPERATIONS.iter().map(|op| operation_schema(op)).collect::<Vec<_>>() })),
    }
}

fn operation_schema(op: &str) -> Value {
    let required: &[&str] = match op {
        "project.new" => &["output"],
        "project.apply" => &["path", "expect_revision", "operations"],
        "project.recover" => &["path", "expect_revision", "recovery"],
        "history.undo" | "history.redo" => &["path", "expect_revision"],
        "project.save-as" | "project.migrate" => &["path", "output", "expect_revision"],
        "project.inspect" | "project.validate" | "history.list" | "render.plan" => &["path"],
        "project.pack" | "project.unpack" | "export.project" | "render" | "export.audio" => &["path", "output"],
        "import" => &["source", "output"],
        "voice.convert" => &["source", "destination", "source_token", "source_model"],
        "voice.install" => &["package"],
        "voice.remove" | "voice.set-default" => &["reference"],
        "voice.import-originals" => &["token", "model"],
        "job.inspect" | "job.cancel" | "job.retry" | "job.verify" => &["job_id"],
        "media.import" => &["path", "expect_revision", "track", "source"],
        "media.relink" => &["path", "expect_revision", "track", "source", "clip_id"],
        "media.inspect" | "media.verify" | "effect.status" => &["path"],
        "effect.scan" => &["plughost", "cache_dir", "paths"],
        "effect.catalog" => &["cache_dir"],
        "batch" => &["requests"],
        _ => &[],
    };
    let optional: &[&str] = match op {
        "schema" => &["op"],
        "doctor" | "voice.verify" => &["worker", "voice_registry", "voice_package", "project_root", "onnxruntime_library", "timeout_seconds"],
        "voice.install" => &["registry", "set_default", "artifact_policy", "artifact_policy_sha256"],
        "voice.list" | "voice.remove" | "voice.set-default" | "voice.import-originals" => &["registry"],
        "project.new" => &["project", "ppq", "operations", "label", "dry_run", "allowed_creator"],
        "project.apply" => &["recovery", "allowed_creator", "expect_project_id", "label", "dry_run"],
        "project.inspect" | "project.validate" | "history.list" => &["recovery", "allowed_creator"],
        "project.recover" => &["allowed_creator", "expect_project_id", "dry_run"],
        "project.migrate" => &["recovery", "allowed_creator", "expect_project_id", "dry_run", "loss_policy"],
        "project.save-as" | "history.undo" | "history.redo" =>
            &["recovery", "allowed_creator", "expect_project_id", "dry_run"],
        "project.pack" => &["recovery", "allowed_creator", "allow_external_media"],
        "project.unpack" => &["max_expanded_bytes", "max_archive_bytes"],
        "media.import" => &["recovery", "allowed_creator", "expect_project_id", "dry_run", "start_tick", "label"],
        "media.relink" => &["recovery", "allowed_creator", "expect_project_id", "dry_run", "label"],
        "media.inspect" => &["track", "clip_id", "recovery", "allowed_creator"],
        "media.verify" => &["track", "clip_id", "recovery", "allowed_creator", "asset_roots", "asset_paths"],
        "effect.status" => &["owner", "instance_id", "recovery", "allowed_creator"],
        "import" => &["format", "options", "loss_policy", "report", "dry_run", "recovery", "allowed_creator"],
        "export.project" => &["format", "options", "loss_policy", "track_id", "recovery", "allowed_creator", "report", "dry_run"],
        "render.plan" | "render" | "export.audio" => &["workspace", "report", "worker", "voice_registry", "voice_package",
            "project_root", "onnxruntime_library", "timeout_seconds", "sample_rate", "channels", "encoding", "scope", "track",
            "start_tick", "end_tick", "effects", "seed", "idempotency_key", "idempotency_scope", "overwrite", "plughost", "asset_roots", "max_duration_seconds",
            "allowed_creator", "recovery", "expect_revision"],
        "job.list" => &["workspace", "limit"],
        "job.inspect" | "job.cancel" | "job.verify" => &["workspace"],
        "job.retry" => &["workspace", "output", "report"],
        "cache.list" | "cache.verify" | "cache.clean" => &["workspace", "digest"],
        "batch" => &["stop_on_error"],
        _ => &[],
    };
    let fields = required.iter().chain(optional).map(|&key|(key.to_owned(), field_schema(key))).collect::<serde_json::Map<_, _>>();
    let mut schema = json!({"op":op,"params":{"type":"object","required":required,"properties":fields,
        "additionalProperties":false}});
    if op == "project.apply" { schema["project_operations"] = crate::project::operation_catalog(); }
    schema
}

fn field_schema(key: &str) -> Value {
    match key {
        "recovery" => json!({"enum":["saved","recovered"]}),
        "loss_policy" => json!({"enum":["strict","allow"]}),
        "encoding" => json!({"enum":["Float32","Pcm16","Pcm24"]}),
        "scope" => json!({"enum":["Mix","ActiveTrack","Stems"]}),
        "sample_rate" => json!({"enum":[44100,48000,88200,96000,192000]}),
        "channels" => json!({"enum":[1,2]}),
        "ppq" | "expect_revision" | "start_tick" | "end_tick" | "seed" | "timeout_seconds" | "limit" | "max_duration_seconds"
        | "max_expanded_bytes" | "max_archive_bytes" => json!({"type":"integer"}),
        "effects" | "dry_run" | "overwrite" | "stop_on_error" | "allow_external_media" | "set_default" => json!({"type":"boolean"}),
        "asset_roots" | "asset_paths" | "paths" => json!({"type":"array","items":{"type":"string"}}),
        "operations" => json!({"type":"array","items":{"type":"object","required":["op"],"properties":{"op":{"type":"string"}}}}),
        "requests" => json!({"type":"array","items":{"type":"object"}}),
        "options" | "project" | "owner" => json!({"type":"object"}),
        _ => json!({"type":"string"}),
    }
}

pub fn positionals(op: &str) -> &'static [&'static str] {
    match op {
        "schema" => &["op"],
        "import" => &["source"],
        "project.new" => &["output"],
        "project.save-as" | "project.save_as" | "project.migrate" | "project.pack" | "project.unpack" => &["path", "output"],
        "project.inspect" | "project.validate" | "project.apply" | "project.recover" | "history.list" | "history.undo" | "history.redo"
        | "export.project" | "export.audio" | "render" | "render.plan" => &["path"],
        "job.inspect" | "job.cancel" | "job.retry" | "job.verify" => &["job_id"],
        "cache.verify" | "cache.clean" => &["digest"],
        "voice.convert" => &["source", "destination"],
        "voice.install" => &["package"],
        "voice.remove" | "voice.set-default" => &["reference"],
        "media.import" | "media.relink" => &["path", "source"],
        "media.inspect" | "media.verify" | "effect.status" => &["path"],
        _ => &[],
    }
}
