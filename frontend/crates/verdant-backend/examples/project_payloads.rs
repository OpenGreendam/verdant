//! Export complete real-worker requests through the editor's project adapter.
//! Usage: project_payloads project.json capabilities.json output-dir [--route verdant-voice]
//! This does not launch GPUI or render audio; it refuses every blocked phrase.
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::{Capabilities, Request, Route, plan_all};
use verdant_domain::digest::{project_content_digest, sha256_hex};
use verdant_domain::{Command, CommandBatch, ProjectStore};

// Compile the application's existing dependency-free resampler directly; the
// offline example does not need to open an audio device or link the GUI.
#[path = "../../verdant-audio/src/resample.rs"]
mod application_resampler;

fn field<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value.get(key).and_then(serde_json::Value::as_str).ok_or_else(|| format!("missing string {key}").into())
}

/// Mix a completed worker run through the real artifact decoder, DYN mapping
/// and application resampler. Insert plugins are intentionally unsupported.
fn mix_completed(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    if arguments.len() != 4 {
        return Err("usage: project_payloads --mix plan-dir render-dir output.wav".into());
    }
    let plan = Path::new(&arguments[1]);
    let render = Path::new(&arguments[2]);
    let destination = Path::new(&arguments[3]);
    if destination.exists() {
        return Err("use a fresh mix output path".into());
    }
    let manifest_bytes = fs::read(plan.join("manifest.json"))?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
    let summary_bytes = fs::read(render.join("summary.json"))?;
    let summary: serde_json::Value = serde_json::from_slice(&summary_bytes)?;
    let completed_bytes = fs::read(render.join("completed.json"))?;
    if summary["success"] != true
        || field(&summary, "plan_manifest_sha256")? != sha256_hex(&manifest_bytes)
        || field(&summary, "completed_json_sha256")? != sha256_hex(&completed_bytes)
    {
        return Err("render summary is incomplete or does not match this plan/completion set".into());
    }
    let completed: Vec<serde_json::Value> = serde_json::from_slice(&completed_bytes)?;
    let requests = manifest["requests"].as_array().ok_or("request manifest missing")?;
    if completed.len() != requests.len() || completed.is_empty() {
        return Err("incomplete phrase set".into());
    }
    let project_bytes = fs::read(plan.join(field(&manifest, "project_file")?))?;
    if sha256_hex(&project_bytes) != field(&manifest, "project_sha256")? {
        return Err("project changed after preflight".into());
    }
    let (project, _) = verdant_domain::io::parse_project(std::str::from_utf8(&project_bytes)?)?;
    let tempo = verdant_domain::TempoMap::new(&project.tempo_map, project.ppq)?;
    for track in &project.tracks {
        if let Some(value) = track.unknown.get("inserts") {
            if !matches!(value, serde_json::Value::Array(values) if values.is_empty()) {
                return Err("this listening mixer refuses plugin inserts; use application export instead".into());
            }
        }
    }
    let workspace = render.join("workspace");
    let rate = 48_000u32;
    let mut track_pcm: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut evidence = Vec::new();
    for (entry, result) in requests.iter().zip(&completed) {
        if field(entry, "job_id")? != field(result, "job_id")? || result["terminal"]["event"] != "completed" {
            return Err("completion does not match its planned job".into());
        }
        let request_path = plan.join(field(entry, "request_file")?);
        let request_bytes = fs::read(request_path)?;
        if sha256_hex(&request_bytes) != field(entry, "request_sha256")? {
            return Err("request changed after planning".into());
        }
        let request: serde_json::Value = serde_json::from_slice(&request_bytes)?;
        let phoneme_timing: Option<verdant_backend::wire::PhonemeTimingTrace> = request["payload"]["score"]
            .get("phoneme_timing").filter(|value| !value.is_null())
            .map(|value| serde_json::from_value(value.clone())).transpose()?;
        if project.render_preferences.phoneme_timing.is_some() != phoneme_timing.is_some() {
            return Err("planned phoneme timing does not match the project".into());
        }
        let route = Route::parse(field(entry, "route")?).ok_or("retired or unsupported render route")?;
        let wideband = entry["wideband"].as_bool().ok_or("wideband condition missing")?;
        let artifacts: Vec<verdant_backend::wire::Artifact> = serde_json::from_value(result["terminal"]["artifacts"].clone())?;
        let audio = verdant_backend::artifact::verify_and_decode_with_options(&workspace, route, wideband, &artifacts)?;
        let track_id = field(entry, "track_id")?;
        let track = project.tracks.iter().find(|track| track.id.as_str() == track_id).ok_or("track missing")?;
        let note_ids: Vec<verdant_domain::NoteId> = serde_json::from_value(entry["note_ids"].clone())?;
        let notes: Vec<_> = note_ids
            .iter()
            .map(|id| track.notes.iter().find(|note| &note.id == id).ok_or("note missing from planned phrase"))
            .collect::<Result<_, _>>()?;
        let start = entry["start_seconds"].as_f64().ok_or("phrase time missing")?;
        if !start.is_finite() || start < 0.0 {
            return Err("invalid phrase time".into());
        }
        let mut pcm = application_resampler::resample(&audio.samples, audio.sample_rate, rate);
        if let Some(envelope) =
            verdant_backend::post::dyn_envelope_for_phrase(&notes, &tempo, start, rate, pcm.len(),
                audio.time_map.as_ref(), phoneme_timing.as_ref())?
        {
            for (sample, gain) in pcm.iter_mut().zip(envelope) {
                *sample *= gain;
            }
        }
        let start_frame = (start * f64::from(rate)).round() as usize;
        let end_frame = start_frame.checked_add(pcm.len()).ok_or("mix length overflow")?;
        let bus = track_pcm.entry(track_id.to_owned()).or_default();
        if bus.len() < end_frame {
            bus.resize(end_frame, 0.0);
        }
        for (index, sample) in pcm.iter().enumerate() {
            bus[start_frame + index] += *sample * 0.9;
        }
        evidence.push(json!({"job_id":entry["job_id"],"track_id":track_id,
            "source_wav_sha256":audio.sha256,"sample_rate":audio.sample_rate,
            "start_frame":start_frame,"prepared_samples":pcm.len(),
            "native_time_map_present":audio.time_map.is_some(),
            "dyn_ownership": if phoneme_timing.is_some() { "scheduled phones; prefix holds owning note's first DYN value" }
                else { "legacy score note intervals" }}));
    }
    let frames = track_pcm.values().map(Vec::len).max().ok_or("empty mix")?;
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    let any_solo = project.tracks.iter().any(|track| track.solo);
    for track in &project.tracks {
        let Some(pcm) = track_pcm.get(track.id.as_str()) else { continue };
        let gain = if track.muted || (any_solo && !track.solo) { 0.0 } else { 10f32.powf(track.gain_millidecibels as f32 / 20_000.0) };
        let pan = track.pan_permille as f32 / 1000.0;
        if !gain.is_finite() || !(-1.0..=1.0).contains(&pan) {
            return Err("invalid track gain or pan".into());
        }
        let (gain_l, gain_r) = (gain * (1.0 - pan).min(1.0), gain * (1.0 + pan).min(1.0));
        for (index, sample) in pcm.iter().enumerate() {
            left[index] += *sample * gain_l;
            right[index] += *sample * gain_r;
        }
    }
    let limit = |value: f32| -> Result<f32, Box<dyn Error>> {
        if !value.is_finite() {
            return Err("nonfinite mix samples".into());
        }
        let magnitude = value.abs();
        Ok(if magnitude <= 0.9 { value } else { value.signum() * (0.9 + 0.1 * ((magnitude - 0.9) / 0.1).tanh()) })
    };
    let data_size = u32::try_from(frames.checked_mul(8).ok_or("WAV size overflow")?)?;
    let riff_size = data_size.checked_add(36).ok_or("listening mix exceeds RIFF limit")?;
    if let Some(parent) = destination.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = std::io::BufWriter::new(fs::OpenOptions::new().write(true).create_new(true).open(destination)?);
    file.write_all(b"RIFF")?;
    file.write_all(&riff_size.to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&3u16.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 8).to_le_bytes())?;
    file.write_all(&8u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    let mut peak = 0.0f32;
    for (l, r) in left.into_iter().zip(right) {
        let l = limit(l)?;
        let r = limit(r)?;
        peak = peak.max(l.abs()).max(r.abs());
        file.write_all(&l.to_le_bytes())?;
        file.write_all(&r.to_le_bytes())?;
    }
    file.flush()?;
    file.get_ref().sync_all()?;
    drop(file);
    let report = json!({
        "schema":"verdant-project-listening-mix/1","sample_rate":rate,"channels":2,
        "samples_per_channel":frames,"duration_seconds":frames as f64 / f64::from(rate),"peak":peak,
        "plan_manifest_sha256":sha256_hex(&manifest_bytes),
        "render_summary_sha256":sha256_hex(&summary_bytes),"waveform_sha256":sha256_hex(&fs::read(destination)?),
        "entrypoint":"real artifact verification -> application resampler -> DYN with native time map -> absolute phrase placement -> track gain/pan/mute/solo -> bounded limiter",
        "scope":"Offline listening mix using application numerical functions; no GUI playback or bit-exact application export assertion; no plugin inserts",
        "clip_gain":0.9,"pan_policy":"linear stereo balance, matching current application export",
        "compiled_source_sha256":compiled_sources(),
        "resampler_sha256":sha256_hex(include_bytes!("../../verdant-audio/src/resample.rs")),
        "post_processing_sha256":sha256_hex(include_bytes!("../src/post.rs")),
        "artifact_verifier_sha256":sha256_hex(include_bytes!("../src/artifact.rs")),
        "phrases":evidence,
    });
    write_json(&destination.with_extension("json"), &report)?;
    println!("Mixed {:.3} seconds at 48 kHz to {}", frames as f64 / f64::from(rate), destination.display());
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<String, Box<dyn Error>> {
    let bytes = format!("{}\n", serde_json::to_string_pretty(value)?).into_bytes();
    let hash = sha256_hex(&bytes);
    fs::write(path, bytes)?;
    Ok(hash)
}

/// Compile-time bytes bind this executable's adapter inputs, independent of
/// later source edits or checkout relocation. Worker source hashes are also
/// retained in capabilities and therefore in every render digest.
fn compiled_sources() -> BTreeMap<&'static str, String> {
    [
        ("verdant-backend/examples/project_payloads.rs", include_bytes!("project_payloads.rs").as_slice()),
        ("verdant-backend/src/adapter.rs", include_bytes!("../src/adapter.rs").as_slice()),
        ("verdant-backend/src/wire.rs", include_bytes!("../src/wire.rs").as_slice()),
        ("verdant-backend/src/f0.rs", include_bytes!("../src/f0.rs").as_slice()),
        ("verdant-backend/src/expression.rs", include_bytes!("../src/expression.rs").as_slice()),
        ("verdant-backend/src/params.rs", include_bytes!("../src/params.rs").as_slice()),
        ("verdant-backend/src/pinyin.rs", include_bytes!("../src/pinyin.rs").as_slice()),
        ("verdant-backend/src/phonemes.rs", include_bytes!("../src/phonemes.rs").as_slice()),
        ("verdant-backend/src/phoneme_timing.rs", include_bytes!("../src/phoneme_timing.rs").as_slice()),
        ("verdant-backend/src/post.rs", include_bytes!("../src/post.rs").as_slice()),
        ("verdant-backend/src/kana.rs", include_bytes!("../src/kana.rs").as_slice()),
        ("verdant-backend/src/reading.rs", include_bytes!("../src/reading.rs").as_slice()),
        ("verdant-domain/src/io.rs", include_bytes!("../../verdant-domain/src/io.rs").as_slice()),
        ("verdant-domain/src/model.rs", include_bytes!("../../verdant-domain/src/model.rs").as_slice()),
        ("verdant-domain/src/synthesis.rs", include_bytes!("../../verdant-domain/src/synthesis.rs").as_slice()),
        ("verdant-domain/src/store.rs", include_bytes!("../../verdant-domain/src/store.rs").as_slice()),
        ("verdant-domain/src/command.rs", include_bytes!("../../verdant-domain/src/command.rs").as_slice()),
        ("verdant-domain/src/timing.rs", include_bytes!("../../verdant-domain/src/timing.rs").as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| (path, sha256_hex(bytes)))
    .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|value| value == "--mix") {
        return mix_completed(&arguments);
    }
    if arguments.len() != 3 && arguments.len() != 5 {
        return Err("usage: project_payloads project.json capabilities.json output-dir [--route verdant-voice]".into());
    }
    let route_override = if arguments.len() == 5 {
        if arguments[3] != "--route" {
            return Err("the optional argument must be --route verdant-voice".into());
        }
        Some(Route::parse(&arguments[4]).ok_or("note projects support verdant-voice")?)
    } else {
        None
    };
    let input_bytes = fs::read(&arguments[0])?;
    let (project, load_report) = verdant_domain::io::parse_project(std::str::from_utf8(&input_bytes)?)?;
    let capabilities_bytes = fs::read(&arguments[1])?;
    let raw: serde_json::Value = serde_json::from_slice(&capabilities_bytes)?;
    let raw_caps = raw.get("capabilities").unwrap_or(&raw);
    if raw_caps.get("test_double").and_then(|value| value.as_bool()) == Some(true) {
        return Err("this exporter requires real worker capabilities".into());
    }
    let caps: Capabilities = serde_json::from_value(raw_caps.clone())?;
    let mut store = ProjectStore::new(project);
    if let Some(route) = route_override {
        let mut commands = vec![Command::SetRoute { route: route.as_str().into() }];
        for track in &store.project().tracks {
            let mut header = track.header();
            header.render_route = Some(route.as_str().into());
            commands.push(Command::SetTrackHeader { track: track.id.clone(), header });
        }
        store.apply(store.revision(), CommandBatch { label: "Explicit offline route comparison".into(), commands })?;
    }
    let project = store.project_for_save();
    let first = project.tracks.first().ok_or("project has no tracks")?;
    let plans = plan_all(&project, &first.id, &caps);
    let output = Path::new(&arguments[2]);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output)?; // A previous request set is immutable evidence.
    fs::write(output.join("input-project.json"), &input_bytes)?;
    let project_hash = write_json(&output.join("project.json"), &project)?;
    let caps_hash = write_json(&output.join("capabilities.json"), raw_caps)?;
    let plans_hash = write_json(&output.join("plans.json"), &plans)?;
    write_json(&output.join("route-override-commands.json"), &store.log())?;
    let failures = plans.iter().filter(|plan| plan.has_errors() || !plan.blocked.is_empty()).count();
    let note_count: usize = project.tracks.iter().map(|track| track.notes.len()).sum();
    let planned_note_count: usize = plans.iter().flat_map(|plan| &plan.phrases).map(|phrase| phrase.note_ids.len()).sum();
    let preflight = json!({
        "success": failures == 0 && note_count == planned_note_count && note_count > 0,
        "tracks_with_errors_or_blocked_phrases": failures,
        "project_notes": note_count,
        "planned_notes": planned_note_count,
        "preserved_unknown_keys": load_report.preserved_unknown_keys,
        "route_override": route_override.map(Route::as_str),
        "plans_sha256": plans_hash,
    });
    write_json(&output.join("preflight.json"), &preflight)?;
    if failures != 0 || note_count != planned_note_count || note_count == 0 {
        return Err(format!("project preflight failed; inspect {}/plans.json and preflight.json", output.display()).into());
    }
    let mut requests = Vec::new();
    for (track_index, plan) in plans.iter().enumerate() {
        let track = project.track(&plan.track).ok_or("planned track disappeared")?;
        for phrase in &plan.phrases {
            let payload = payload_for(phrase, plan.route, project.render_preferences.native_seed);
            let digest = render_digest(&payload, plan.route, &caps);
            let wideband = payload.wideband.is_some();
            let job_id = format!("t{track_index:03}_p{:04}_{}", phrase.index, plan.route.as_str());
            let request = Request::render(format!("request_{job_id}"), &job_id, store.revision().get(), plan.route, payload);
            request.to_line()?; // Enforce the actual worker's one-MiB wire limit.
            let filename = format!("{job_id}.request.json");
            let request_hash = write_json(&output.join(&filename), &request)?;
            let notes: Vec<_> = phrase
                .note_ids
                .iter()
                .map(|id| track.notes.iter().find(|note| &note.id == id).ok_or("planned note disappeared"))
                .collect::<Result<_, _>>()?;
            requests.push(json!({
                "request_file": filename, "request_sha256": request_hash,
                "job_id": job_id, "route": plan.route, "wideband": wideband,
                "render_digest": digest, "track_id": track.id, "track_index": track_index,
                "track_name": track.name, "track_header": track.header(),
                "phrase_index": phrase.index, "note_ids": phrase.note_ids, "notes": notes,
                "start_tick": phrase.start_tick, "end_tick": phrase.end_tick,
                "start_seconds": phrase.start_seconds, "expected_seconds": phrase.seconds,
            }));
        }
    }
    let manifest = json!({
        "schema": "verdant-project-worker-requests/1",
        "entrypoint": "parse_project -> ProjectStore -> plan_all -> payload_for -> render_digest -> Request",
        "scope": "Real GPUI backend adapter requests; no GPUI window, playback or listening assertion",
        "input_project_sha256": sha256_hex(&input_bytes),
        "input_capabilities_sha256": sha256_hex(&capabilities_bytes),
        "project_file": "project.json", "project_sha256": project_hash,
        "project_content_digest": project_content_digest(&project),
        "capabilities_file": "capabilities.json", "capabilities_sha256": caps_hash,
        "compiled_source_sha256": compiled_sources(),
        "native_version": caps.native_version,
        "native_seed": project.render_preferences.native_seed,
        "synthesis": project.render_preferences.synthesis,
        "vocal_tract": project.render_preferences.vocal_tract,
        "model_dynamics": project.render_preferences.model_dynamics,
        "route_override": route_override.map(Route::as_str),
        "revision": store.revision().get(),
        "requests": requests,
    });
    write_json(&output.join("manifest.json"), &manifest)?;
    println!("{} complete real-worker requests written to {}", manifest["requests"].as_array().unwrap().len(), output.display());
    Ok(())
}
