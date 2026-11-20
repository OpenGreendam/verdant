//! Fixed contrast probes through the real editor frontend and score adapter.
//! Usage: articulation_payloads <real-capabilities.json> <new-output-dir>
//!        [verdant-voice] [admitted-voice-reference]
//!
//! This writes requests and native .verp projects; it does not synthesize audio
//! or infer perceptual scores. Japanese inputs retain their approximation and
//! kana realization provenance. Optional core requests provide a controlled
//! verdant-voice comparison without changing the score or learned condition input.
use std::{collections::BTreeMap, error::Error, fs, path::Path};

use serde::Serialize;
use serde_json::{Value, json};
use verdant_backend::{Capabilities, Route, adapter::{payload_for, render_digest},
    reading::{parse_phonetic_input, validate_pronunciation}, wire::Request};
use verdant_domain::{Command, CommandBatch, Note, NoteId, Project, ProjectStore, TrackId,
    digest::sha256_hex};

const SEED: u32 = 10008;
const PITCHES: [i32; 2] = [60, 72];
const UNIT_TICKS: [i64; 2] = [384, 768]; // 200 / 400 ms at 120 BPM, PPQ 960.
const GAP_TICKS: i64 = 240;

struct Contrast {
    id: &'static str,
    language: &'static str,
    purpose: &'static str,
    // Explicit readings and their authored syllable/mora counts. The count is
    // duration policy for this probe, not text G2P or a recognition target.
    inputs: &'static [(&'static str, u32)],
}

fn contrasts() -> Vec<Contrast> {
    vec![
        Contrast { id:"zh-sibilants", language:"zh", purpose:"Alveolar versus retroflex frication and affrication",
            inputs:&[("撒/[zh:s a]",1),("沙/[zh:ʂ a]",1),("咂/[zh:t͡s a]",1),("渣/[zh:t͡ʂ a]",1)] },
        Contrast { id:"zh-aspiration", language:"zh", purpose:"Matched vowel with three explicit stop aspiration contrasts",
            inputs:&[("巴/[zh:p a]",1),("趴/[zh:pʰ a]",1),("搭/[zh:t a]",1),("他/[zh:tʰ a]",1),
                ("嘎/[zh:k a]",1),("咖/[zh:kʰ a]",1)] },
        Contrast { id:"en-sibilants", language:"en", purpose:"Place and voicing of English sibilants before the same vowel",
            inputs:&[("see/[en:s i]",1),("she/[en:ʃ i]",1),("zee/[en:z i]",1),("zhee/[en:ʒ i]",1)] },
        Contrast { id:"en-fricatives", language:"en", purpose:"Dental versus labiodental fricatives, voiced and unvoiced",
            inputs:&[("thee-unvoiced/[en:θ i]",1),("fee/[en:f i]",1),("thee/[en:ð i]",1),("vee/[en:v i]",1)] },
        Contrast { id:"en-vowels", language:"en", purpose:"Close vowel contrasts with an identical onset and duration",
            inputs:&[("bee/[en:b i]",1),("bih/[en:b ɪ]",1),("boo/[en:b u]",1),("buh-close/[en:b ʊ]",1)] },
        Contrast { id:"ja-vowels", language:"ja", purpose:"Existing unrounded-u and mid-e approximations against other vowels",
            inputs:&[("う/[ja/kana:う]",1),("え/[ja/kana:え]",1),("お/[ja/kana:お]",1),("い/[ja/kana:い]",1)] },
        Contrast { id:"ja-tap", language:"ja", purpose:"Tap articulation versus stop and nasal with the same vowel",
            inputs:&[("ら/[ja/kana:ら]",1),("だ/[ja/kana:だ]",1),("な/[ja/kana:な]",1)] },
        Contrast { id:"ja-palatal", language:"ja", purpose:"One-mora palatal gesture versus a two-mora vowel/glide sequence",
            inputs:&[("きゃ/[ja/kana:きゃ]",1),("きや/[ja/kana:きや]",2)] },
        Contrast { id:"ja-length", language:"ja", purpose:"Short/long vowel and singleton/geminate consonant timing at each tempo",
            inputs:&[("か/[ja/kana:か]",1),("かー/[ja/kana:かー]",2),
                ("かこ/[ja/kana:かこ]",2),("かっこ/[ja/kana:かっこ]",3)] },
        Contrast { id:"ja-nasal", language:"ja", purpose:"Provenance-restricted mora nasal place and a written-word boundary control",
            inputs:&[("さんぽ/[ja/kana:さんぽ]",3),("かんたん/[ja/kana:かんたん]",4),
                ("さんか/[ja/kana:さんか]",3),("さん ぽ/[ja/kana:さん ぽ]",3)] },
    ]
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<String, Box<dyn Error>> {
    let bytes = format!("{}\n", serde_json::to_string_pretty(value)?).into_bytes();
    fs::write(path, &bytes)?;
    Ok(sha256_hex(&bytes))
}

fn compiled_sources() -> BTreeMap<&'static str, String> {
    [
        ("examples/articulation_payloads.rs", include_bytes!("articulation_payloads.rs").as_slice()),
        ("src/adapter.rs", include_bytes!("../src/adapter.rs").as_slice()),
        ("src/wire.rs", include_bytes!("../src/wire.rs").as_slice()),
        ("src/reading.rs", include_bytes!("../src/reading.rs").as_slice()),
        ("src/kana.rs", include_bytes!("../src/kana.rs").as_slice()),
        ("src/phonemes.rs", include_bytes!("../src/phonemes.rs").as_slice()),
        ("src/phonemizer/resolution.rs", include_bytes!("../src/phonemizer/resolution.rs").as_slice()),
        ("src/data/phoneme_inventory.json", include_bytes!("../src/data/phoneme_inventory.json").as_slice()),
    ].into_iter().map(|(path, bytes)| (path, sha256_hex(bytes))).collect()
}

fn validate_inputs() -> Result<(), Box<dyn Error>> {
    for contrast in contrasts() {
        for &(input, units) in contrast.inputs {
            let (_, reading) = parse_phonetic_input(input)?.ok_or("probe needs an explicit reading")?;
            validate_pronunciation(&reading)?;
            if reading.language.as_deref() != Some(contrast.language) {
                return Err(format!("{} has an inconsistent declared language", contrast.id).into());
            }
            if contrast.language == "ja" && reading.mora_weights.as_ref().map(|weights| weights.iter().sum::<u32>()) != Some(units) {
                return Err(format!("{input} does not have the declared {units} written moras").into());
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=4).contains(&arguments.len()) {
        return Err("usage: articulation_payloads real-capabilities.json new-output-dir [verdant-voice] [admitted-voice-reference]".into());
    }
    let route = Route::parse(arguments.get(2).map(String::as_str).unwrap_or("verdant-voice"))
        .ok_or("articulation score probes require verdant-voice")?;
    let capabilities_bytes = fs::read(&arguments[0])?;
    let raw: Value = serde_json::from_slice(&capabilities_bytes)?;
    let raw_caps = raw.get("capabilities").unwrap_or(&raw);
    let caps: Capabilities = serde_json::from_value(raw_caps.clone())?;
    if caps.test_double { return Err("articulation probes require real worker capabilities".into()); }
    if !caps.supports_route(route) { return Err("worker does not advertise the requested route".into()); }
    validate_inputs()?;
    let voice_ref = arguments.get(3).cloned().or_else(|| caps.voices.as_ref().and_then(|catalog| catalog.default_voice.clone()));
    let output = Path::new(&arguments[1]);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) { fs::create_dir_all(parent)?; }
    fs::create_dir(output)?; // Never mix a new run with old probe evidence.
    write_json(&output.join("capabilities.json"), raw_caps)?;
    let mut requests = Vec::new();
    for contrast in contrasts() {
        for midi in PITCHES {
            for unit_ticks in UNIT_TICKS {
                let name = format!("{}-p{midi}-t{unit_ticks}", contrast.id);
                let mut project = Project::new_empty();
                project.project_id = format!("verdant-articulation-{name}");
                project.render_preferences.route = route.as_str().into();
                project.render_preferences.native_seed = SEED;
                project.tempo_map[0].micro_bpm = 120_000_000;
                project.tracks[0].id = TrackId(format!("track-{name}"));
                project.tracks[0].name = name.clone();
                if let Some(reference) = &voice_ref {
                    project.voice_refs.clear();
                    project.tracks[0].voice_ref = Some(reference.clone());
                    let (_, voice) = caps.voice_for_track(&project, &project.tracks[0])?.ok_or("requested singer is not in the admitted catalog")?;
                    project.tracks[0].unknown.insert("voice_selection".into(), json!({"schema":"verdant-track-voice/1",
                        "reference":reference,"cache_identity":voice.cache_identity,"source_sha256":voice.source_sha256}));
                }
                project.extensions.insert("verdant.articulation_probe".into(), json!({"schema":"verdant-articulation-probe/1",
                    "contrast":contrast.id,"language":contrast.language,"unit_ticks":unit_ticks,"seed":SEED}));
                let mut store = ProjectStore::new(project);
                let track = store.project().tracks[0].id.clone();
                let mut notes = Vec::new();
                let mut edits = Vec::new();
                let mut tick = 0;
                for (index, &(input, units)) in contrast.inputs.iter().enumerate() {
                    let (lyric, pronunciation) = parse_phonetic_input(input)?.ok_or("explicit probe input was not recognized")?;
                    let mut note = Note::new(tick, unit_ticks * i64::from(units), midi);
                    note.id = NoteId(format!("{name}-n{index}"));
                    edits.push(Command::SetLyric { track:track.clone(), note_id:note.id.clone(), text:lyric });
                    edits.push(Command::SetPronunciation { track:track.clone(), note_id:note.id.clone(), pronunciation });
                    tick += note.duration_tick + GAP_TICKS;
                    notes.push(note);
                }
                store.apply(store.revision(), CommandBatch::single("Insert articulation comparison notes", Command::InsertNotes { track:track.clone(), notes }))?;
                store.apply(store.revision(), CommandBatch { label:"Compile explicit contrast readings".into(), commands:edits })?;
                let project = store.project_for_save();
                let project_file = format!("{name}.verp");
                verdant_domain::io::save_project_atomic(&output.join(&project_file), &project)?;
                let planned = verdant_backend::plan(&project, &track, route, &caps);
                let plan_file = format!("{name}.plan.json");
                let plan_sha256 = write_json(&output.join(&plan_file), &planned)?;
                if planned.has_errors() || !planned.blocked.is_empty() || planned.phrases.len() != 1 {
                    return Err(format!("{name} did not produce one admitted phrase; inspect {plan_file}: {:?}", planned.diagnostics).into());
                }
                let phrase = &planned.phrases[0];
                let selected = caps.voice_for_track(&project, &project.tracks[0])?;
                let selected_caps = selected.map(|(_, voice)| caps.for_voice(voice)).unwrap_or_else(|| caps.clone());
                let payload = payload_for(phrase, route, SEED);
                let digest = render_digest(&payload, route, &selected_caps);
                let profile = payload.voice_music.clone();
                let score_file = format!("{name}.score.json");
                let score_sha256 = write_json(&output.join(&score_file), &phrase.score)?;
                let mut request = Request::render(format!("request-{name}"), &name, store.revision().get(), route, payload);
                request.voice_ref = selected.map(|(reference, _)| reference.to_owned());
                request.to_line()?;
                let request_file = format!("{name}.request.json");
                let request_sha256 = write_json(&output.join(&request_file), &request)?;
                let mut baseline = Value::Null;
                if profile.is_some() {
                    let mut payload = request.payload.clone().ok_or("render payload missing")?;
                    payload.voice_music = None;
                    let baseline_digest = render_digest(&payload, route, &selected_caps);
                    let job_id = format!("{name}-core");
                    let mut core = Request::render(format!("request-{job_id}"), &job_id, store.revision().get(), route, payload);
                    core.voice_ref = request.voice_ref.clone();
                    core.to_line()?;
                    let filename = format!("{name}.core.request.json");
                    let sha256 = write_json(&output.join(&filename), &core)?;
                    baseline = json!({"request_file":filename,"request_sha256":sha256,"job_id":job_id,
                        "render_digest":baseline_digest,"same_score_and_seed":true,"voice_music_omitted":true});
                }
                requests.push(json!({"case":name,"contrast":contrast.id,"language":contrast.language,
                    "purpose":contrast.purpose,"midi_note":midi,"unit_ticks":unit_ticks,
                    "unit_seconds":unit_ticks as f64 / 1920.,"gap_seconds":GAP_TICKS as f64 / 1920.,
                    "seed":SEED,"route":route.as_str(),"voice_music":profile,"job_id":name,
                    "expected_seconds":phrase.seconds,"render_digest":digest,
                    "request_file":request_file,"request_sha256":request_sha256,
                    "project_file":project_file,"project_sha256":sha256_hex(&fs::read(output.join(&project_file))?),
                    "plan_file":plan_file,"plan_sha256":plan_sha256,"score_file":score_file,"score_sha256":score_sha256,
                    "inputs":contrast.inputs.iter().map(|(input, units)| json!({"input":input,"duration_units":units})).collect::<Vec<_>>(),
                    "segments":phrase.score.segments.iter().enumerate().map(|(index, segment)| json!({
                        "segment_index":index,"label":segment.label,"duration_seconds":segment.duration_seconds,
                        "gap_after_seconds":segment.gap_after_seconds,"voiced":segment.voiced,"phoneme":segment.phoneme})).collect::<Vec<_>>(),
                    "baseline":baseline}));
            }
        }
    }
    let manifest = json!({"schema":"verdant-articulation-requests/1",
        "entrypoint":"parse_phonetic_input -> ProjectStore -> adapter plan -> payload_for -> Request",
        "scope":"Self-authored finite pronunciation probes; mapping/timing evidence only until real audio is evaluated",
        "input_capabilities_sha256":sha256_hex(&capabilities_bytes),"compiled_source_sha256":compiled_sources(),
        "voice_ref":voice_ref,"native_version":caps.native_version,"seed":SEED,"requests":requests,
        "evaluations":{"native_listener_preference":null,"phoneme_recognition":null,"MOS":null,
            "cross_language_training_improvement":null},
        "limitations":["Japanese acoustic mappings remain approximate.",
            "An advertised language describes input mapping, not learned multilingual pronunciation quality.",
            "High-rate output alone does not establish usable high-frequency content.",
            "Generated requests and .verp projects are not evidence of completed renders or listening."]});
    write_json(&output.join("manifest.json"), &manifest)?;
    println!("{} fixed contrast requests written to {}", manifest["requests"].as_array().unwrap().len(), output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_contrasts_use_admitted_explicit_readings_and_written_mora_counts() {
        validate_inputs().unwrap();
        assert_eq!(contrasts().len() * PITCHES.len() * UNIT_TICKS.len(), 40);
    }
}
