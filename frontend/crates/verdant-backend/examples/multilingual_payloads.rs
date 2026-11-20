//! Real-capabilities reproduction of editor input -> ProjectStore -> adapter.
//! Usage: multilingual_payloads <worker-capabilities.json> <output-directory>
use std::error::Error;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::reading::parse_phonetic_input;
use verdant_backend::wire::Request;
use verdant_backend::{plan, Capabilities, Route};
use verdant_domain::model::PitchPoint;
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectStore};

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

struct Case {
    name: &'static str,
    notes: Vec<(&'static str, i64, i32)>,
    modulated: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 2 {
        return Err("usage: multilingual_payloads <real-capabilities.json> <output-directory>".into());
    }
    let raw: serde_json::Value = serde_json::from_str(&fs::read_to_string(&arguments[0])?)?;
    let raw_caps = raw.get("capabilities").unwrap_or(&raw);
    if raw_caps.get("test_double").and_then(|v| v.as_bool()) == Some(true) {
        return Err("this example requires real worker capabilities".into());
    }
    let caps: Capabilities = serde_json::from_value(raw_caps.clone())?;
    let output = Path::new(&arguments[1]);
    fs::create_dir_all(output)?;
    let cases = vec![
        Case {
            name: "zh-native",
            modulated: false,
            notes: vec![
                ("上/[zh:ʂ ɑŋ]", 768, 60),
                ("海/[zh:x aɪ]", 768, 62),
                ("好/[zh:x aʊ]", 768, 64),
                ("妈/[zh:m a]", 768, 65),
                ("女/[zh:n y]", 768, 64),
                ("绿/[zh:l y]", 768, 62),
            ],
        },
        Case { name: "en-ipa", modulated: false, notes: vec![("hello/[en:h ə . ˈl oʊ]", 2304, 60)] },
        Case { name: "ja-kana", modulated: false, notes: vec![("桜/[ja/kana:サクラ]", 2880, 64)] },
        Case { name: "ja-long-explicit", modulated: false, notes: vec![("学校/[ja/kana:がっこう]", 2304, 62)] },
        Case { name: "ja-long-common", modulated: false, notes: vec![("学校/[ja/kana-common:がっこう]", 2304, 62)] },
        Case { name: "ja-nasal-context", modulated: false, notes: vec![("散歩/[ja/kana:さんぽ]", 1728, 64), ("簡単/[ja/kana:かんたん]", 2304, 64)] },
        Case { name: "ja-palatal-length", modulated: false, notes: vec![("きゃ/[ja/kana:きゃ]", 576, 62), ("きゃー/[ja/kana:きゃー]", 1152, 64), ("きや/[ja/kana:きや]", 1152, 64)] },
        Case { name: "original-hierarchy-explicit", modulated: false, notes: vec![("a/[zh:a]", 576, 60), ("i/[zh:i]", 576, 60)] },
        Case { name: "en-ipa-pit", modulated: true, notes: vec![("hello/[en:h ə . ˈl oʊ]", 2304, 60)] },
    ];
    let mut requests = Vec::new();
    for case in cases {
        let mut project = Project::new_empty();
        project.tempo_map[0].micro_bpm = 120_000_000;
        let mut store = ProjectStore::new(project);
        let track = store.project().tracks[0].id.clone();
        let mut notes = Vec::new();
        let mut edits = Vec::new();
        let mut tick = 0;
        for (input, duration, midi) in &case.notes {
            let (lyric, pronunciation) = parse_phonetic_input(input)?.ok_or("explicit editor input was not recognized")?;
            let mut note = Note::new(tick, *duration, *midi);
            if case.modulated {
                note.curves.pitch_millicent_points = vec![
                    PitchPoint { id: "pit-start".into(), tick: 0, millicents: -100_000, unknown: Default::default() },
                    PitchPoint { id: "pit-end".into(), tick: duration / 2, millicents: 100_000, unknown: Default::default() },
                ];
            }
            edits.push(Command::SetLyric { track: track.clone(), note_id: note.id.clone(), text: lyric });
            edits.push(Command::SetPronunciation { track: track.clone(), note_id: note.id.clone(), pronunciation });
            notes.push(note);
            tick += duration;
        }
        store.apply(
            store.revision(),
            CommandBatch::single("Insert authored integration notes", Command::InsertNotes { track: track.clone(), notes }),
        )?;
        store.apply(store.revision(), CommandBatch { label: "Compile explicit multilingual editor input".into(), commands: edits })?;
        let project = store.project_for_save();
        write_json(&output.join(format!("{}.project.json", case.name)), &project)?;
        write_json(&output.join(format!("{}.commits.json", case.name)), &store.log())?;
        let route = Route::VerdantVoice;
        let planned = plan(&project, &track, route, &caps);
        write_json(&output.join(format!("{}-{}.plan.json", case.name, route.as_str())), &planned)?;
        let supports_control = caps.native_score.as_ref().is_some_and(|c| c.continuous_f0_control);
        if case.modulated && !supports_control {
            if !planned.has_errors() || !planned.phrases.is_empty() {
                return Err("route without continuous F0 capability unexpectedly accepted an authored pitch curve".into());
            }
            continue;
        }
        if planned.has_errors() || !planned.blocked.is_empty() || planned.phrases.len() != 1 {
            return Err(
                format!("{} / {} did not produce one valid phrase: {:?}", case.name, route.as_str(), planned.diagnostics).into()
            );
        }
        let phrase = &planned.phrases[0];
        let mut payload = payload_for(phrase, route, 10008);
        if case.name == "original-hierarchy-explicit" {
            // Deliberately authored boundary fixture, not linguistic inference.
            std::sync::Arc::make_mut(payload.score.as_mut().ok_or("native score missing")?).original_hierarchy = Some(verdant_backend::wire::OriginalHierarchy {
                schema_version: "ogd-explicit-original-hierarchy/1".into(),
                nodes: vec![
                    verdant_backend::wire::OriginalHierarchyNode { phone: 3, left_level: 5, right_level: 0 },
                    verdant_backend::wire::OriginalHierarchyNode { phone: 13, left_level: 0, right_level: 5 },
                ],
                segment_node_indices: vec![0, 1],
                provenance: "Synthetic two-phone hierarchy fixture; manually supplied levels, not original frontend output".into(),
            });
        }
        let automatic_portamento = !case.modulated && case.notes.windows(2).any(|pair| pair[0].2 != pair[1].2);
        if payload.control_track.is_some() != (supports_control && (case.modulated || automatic_portamento)) {
            return Err(format!("{} / {} lost its expected authored or automatic pitch control", case.name, route.as_str()).into());
        }
        let continuous_control = payload.control_track.is_some();
        let digest = render_digest(&payload, route, &caps);
        let job_id = format!("{}-{}", case.name, route.as_str());
        let request = Request::render(format!("request-{job_id}"), job_id.clone(), store.revision().get(), route, payload);
        let filename = format!("{job_id}.request.json");
        write_json(&output.join(&filename), &request)?;
        requests.push(json!({
            "case":case.name,"route":route.as_str(),"request_file":filename,"job_id":job_id,
            "expected_seconds":phrase.seconds,"render_digest":digest,"modulated":case.modulated,
            "continuous_control":continuous_control,"automatic_portamento":automatic_portamento && supports_control,
            "input":case.notes.iter().map(|n| n.0).collect::<Vec<_>>(),
            "project_file":format!("{}.project.json",case.name),
            "revision":store.revision().get(),
        }));
    }
    let manifest = json!({
        "schema":"verdant-real-multilingual-plans/1",
        "entrypoint":"parse_phonetic_input -> ProjectStore InsertNotes/SetLyric/SetPronunciation -> plan -> payload_for",
        "capabilities_source":arguments[0],"native_version":caps.native_version,
        "requests":requests,
    });
    write_json(&output.join("manifest.json"), &manifest)?;
    println!("{} real-capability requests written to {}", manifest["requests"].as_array().unwrap().len(), output.display());
    Ok(())
}
