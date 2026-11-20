//! Export the exact desktop planner and scheduler requests for a saved project.
//! Usage: desktop_project_payloads project.verp capabilities.json fresh-output-dir
use std::{error::Error, fs, path::Path};
use serde_json::{Value, json};
use verdant_backend::{Capabilities, RenderScheduler, Request, adapter};
use verdant_domain::digest::sha256_hex;

fn write(path: &Path, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: desktop_project_payloads project.verp capabilities.json fresh-output-dir".into());
    }
    let project_bytes = fs::read(&args[0])?;
    let (project, _) = verdant_domain::io::parse_project(std::str::from_utf8(&project_bytes)?)?;
    let event: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let raw_caps = event.get("capabilities").unwrap_or(&event);
    if raw_caps.get("test_double").and_then(Value::as_bool) == Some(true) {
        return Err("real worker capabilities are required".into());
    }
    let caps: Capabilities = serde_json::from_value(raw_caps.clone())?;
    let first = &project.tracks.first().ok_or("project has no tracks")?.id;
    let plans = adapter::plan_all_desktop(&project, first, &caps);
    let output = Path::new(&args[2]);
    fs::create_dir(output)?;
    fs::write(output.join("project.verp"), &project_bytes)?;
    write(&output.join("plans.json"), &plans)?;
    let eligible: usize = project.tracks.iter().map(|t| t.notes.iter().filter(|n| !n.is_muted()).count()).sum();
    let planned: usize = plans.iter().flat_map(|p| &p.phrases).map(|p| p.note_ids.len()).sum();
    let errors = plans.iter().any(|p| p.has_errors() || !p.blocked.is_empty());
    write(&output.join("preflight.json"), &json!({
        "success": !errors && planned == eligible && eligible > 0,
        "eligible_notes": eligible, "planned_notes": planned,
        "blocked_phrases": plans.iter().map(|p| p.blocked.len()).sum::<usize>(),
    }))?;
    if errors || eligible == 0 || eligible != planned {
        return Err(format!("desktop preflight failed; inspect {}/plans.json", output.display()).into());
    }
    let mut scheduler = RenderScheduler::new();
    scheduler.capabilities = Some(caps);
    scheduler.set_desktop_project_voices(&project);
    scheduler.set_plans(project.revision, &plans, project.render_preferences.native_seed);
    let mut requests = Vec::new();
    for (i, wanted) in scheduler.wanted().iter().enumerate() {
        let job_id = format!("desktop_p{i:03}");
        let mut request = Request::render(format!("request_{job_id}"), &job_id,
            project.revision, wanted.phrase.route, wanted.payload.clone());
        request.voice_ref = wanted.voice_ref.clone();
        request.to_line()?;
        let filename = format!("{job_id}.request.json");
        write(&output.join(&filename), &request)?;
        let track = project.track(&wanted.phrase.track).ok_or("planned track missing")?;
        let notes: Vec<_> = wanted.phrase.note_ids.iter().map(|id|
            track.notes.iter().find(|n| &n.id == id).ok_or("planned note missing")
        ).collect::<Result<_, _>>()?;
        requests.push(json!({
            "job_id": job_id, "request_file": filename,
            "request_sha256": sha256_hex(&fs::read(output.join(&filename))?),
            "route": wanted.phrase.route, "voice_ref": wanted.voice_ref,
            "track": wanted.phrase.track, "phrase_index": wanted.phrase.index,
            "digest": wanted.digest, "start_seconds": wanted.phrase.start_seconds,
            "seconds": wanted.phrase.seconds, "notes": notes,
        }));
    }
    write(&output.join("manifest.json"), &json!({
        "schema": "verdant-desktop-project-payloads/1",
        "entrypoint": "load project -> plan_all_desktop -> RenderScheduler::set_desktop_project_voices/set_plans -> wanted payload",
        "project_sha256": sha256_hex(&project_bytes),
        "worker_binary_sha256": raw_caps["files"]["worker_binary"]["sha256"],
        "eligible_notes": eligible, "requests": requests,
    }))?;
    println!("{} desktop requests, {} notes", scheduler.wanted().len(), planned);
    Ok(())
}
