//! Synthetic boundary regressions plus unchanged original-x86 hierarchy vectors.
//! No voice weights or recorded synthesis output are embedded in these tests.
use std::{collections::{BTreeMap, BTreeSet}, fs, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};
use serde_json::{json, Value};
use super::{compact_hierarchy_report, load_raw_voice, load_voice, prepare_score_segments, resolve_segment_identity, validate_score_hierarchy, context::{ContextState, Facts}, data::{digest, profiles, Kind}, loading, source_pins};

fn answer(value: Value, question: &str) -> anyhow::Result<(Option<bool>, String)> {
    let context = ContextState::parse(&value)?;
    let facts = Facts::new(&context.features)?;
    let bounds = context.bounds(&facts)?;
    let result = context.answer(question, &facts, &bounds)?;
    Ok((result.truth, result.source.text()))
}

#[test]
fn explicit_phoneme_note_ownership_is_validated_and_does_not_change_contexts() {
    let mut score = json!({"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":60,
        "phoneme":{"canonical_ipa":"ɑ","source_language":"en","input_alphabet":"ipa",
            "native_key":"en:aa","support":"native","source_note_id":"note-a"}}]});
    let with_owner = prepare_score_segments(&score, &json!({})).unwrap();
    assert_eq!(with_owner[0]["phoneme"]["source_note_id"], "note-a");
    score["segments"][0]["phoneme"].as_object_mut().unwrap().remove("source_note_id");
    let without_owner = prepare_score_segments(&score, &json!({})).unwrap();
    let mut stripped = with_owner;
    stripped[0]["phoneme"].as_object_mut().unwrap().remove("source_note_id");
    assert_eq!(stripped, without_owner, "note ownership must not enter model context selection");
    for invalid in [json!(null), json!(""), json!(false), json!(3), json!([]), json!({})] {
        score["segments"][0]["phoneme"]["source_note_id"] = invalid;
        assert!(prepare_score_segments(&score, &json!({})).is_err());
    }
}
#[test]
fn numeric_contexts_have_an_independent_finite_witness() {
    let thresholds = [-2, 0, 3];
    let witnesses = [-3, -2, -1, 0, 1, 3, 4];
    for mask in 0..8 {
        let mut answers = serde_json::Map::new();
        for (index, threshold) in thresholds.iter().enumerate() { answers.insert(format!("value<={threshold}"), json!(mask & (1 << index) != 0)); }
        let feasible = witnesses.iter().any(|value| thresholds.iter().enumerate().all(|(index, threshold)| (value <= threshold) == (mask & (1 << index) != 0)));
        assert_eq!(answer(json!({"answers": answers}), "unrelated").is_ok(), feasible, "mask {mask}");
    }
    let context = json!({"answers": {"count<=11": false, "count<=13": true}});
    assert_eq!(answer(context.clone(), "count<=10").unwrap().0, Some(false));
    assert_eq!(answer(context.clone(), "count<=12").unwrap().0, None);
    assert_eq!(answer(context, "count<=14").unwrap().0, Some(true));
    assert!(answer(json!({"answers": {"x<=02": true, "x<=2.0": false}}), "x<=3").is_err());
    assert!(answer(json!({"features": {"count": 2}, "diagnostic_answers": {"count<=2": false}}), "count<=2").is_err());
    assert!(answer(json!({"answers": {"unknown": true}, "diagnostic_answers": {"unknown": false}}), "unknown").is_err());
    assert!(answer(json!({"answers": {"unknown": 1}}), "unknown").is_err());
    let implied = answer(json!({"diagnostic_answers": {"count<=11": true}}), "count<=13").unwrap();
    assert_eq!(implied, (Some(true), "diagnostic_numeric_threshold_implication:count".into()));
}
#[test]
fn original_sentinels_aliases_and_missing_fields_are_preserved() {
    let sentinel = json!({"features": {"C_Tone": -1}, "answers": {"C_Tone<=2": false}});
    assert_eq!(answer(sentinel, "C_Tone<=1").unwrap().0, Some(false));
    assert!(answer(json!({"features": {"C_Tone": -1}, "answers": {"C_Tone<=2": true}}), "C_Tone<=1").is_err());
    assert_eq!(answer(json!({"features": {"L_Phone": 0}}), "L_Tone==XX").unwrap().0, Some(true));
    assert_eq!(answer(json!({"features": {"C_Phone": 175, "C_Stress": 1}}), "C_Tone==1").unwrap().0, Some(true));
    assert!(answer(json!({"features": {"C_Phone": 175, "C_Tone": 1, "C_Stress": 0}}), "C_Stress==0").is_err());
    // These two shipped literal aliases intentionally do not match their spelling.
    assert_eq!(answer(json!({"features": {"C_Vowel": "eh"}}), "C_Vowel==eh").unwrap().0, Some(false));
    assert_eq!(answer(json!({"features": {"C_Vowel": "ah"}}), "C_Vowel==eh").unwrap().0, Some(true));
    assert_eq!(answer(json!({"features": {"R_Phone": 15}}), "R_YM==iang").unwrap().0, Some(true));
    assert_eq!(answer(json!({"features": {"C_YM": "ai"}}), "L_YM=LS3").unwrap().0, None);
    assert_eq!(answer(json!({"features": {"C_YM": "ai"}}), "C_YM=UNDECODED").unwrap().0, None);
    assert_eq!(answer(json!({"features": {"C_POS": "j"}}), "C_POS==j").unwrap().0, None);
    assert!(answer(json!({"features": {"C_Phone": 217, "C_SM": "m"}}), "C_EN").is_err());
}
#[test]
fn all_hierarchy_comparisons_match_retained_original_x86_vectors() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/original-hierarchy-synthetic-fixture.json")).unwrap();
    let profiles = profiles().unwrap();
    let mut observed = BTreeSet::new();
    for case in fixture["cases"].as_array().unwrap() {
        let hierarchy = json!({"schema_version": "ogd-explicit-original-hierarchy/1", "nodes": case["nodes"], "current_index": case["current_index"]});
        let index = case["current_index"].as_u64().unwrap() as usize;
        let context = ContextState::parse(&json!({"features": {"C_Phone": case["nodes"][index]["phone"], "OriginalHierarchy": hierarchy}})).unwrap();
        let facts = Facts::new(&context.features).unwrap();
        let bounds = context.bounds(&facts).unwrap();
        let mut values = BTreeMap::new();
        for row in case["results"].as_array().unwrap() {
            values.insert((row["family"].as_str().unwrap(), row["argument1"].as_i64().unwrap() as i32, row["argument2"].as_i64().unwrap() as i32), row["original_value"].as_i64().unwrap() as i32);
        }
        for (question, spec) in &profiles.questions {
            if spec.kind != Kind::Hierarchy { continue; }
            if let Some(checked) = case.get("checked_families") { if !checked.as_array().unwrap().iter().any(|family| family.as_str() == Some(spec.family.as_str())) { continue; } }
            let expected = values.get(&(spec.family.as_str(), spec.argument1, spec.argument2)).map(|&value| if spec.comparison_mode == 0 { value == spec.rhs } else { value != -1 && value <= spec.rhs });
            assert_eq!(context.answer(question, &facts, &bounds).unwrap().truth, expected, "{question}; case {case}");
            if expected.is_some() { observed.insert(question.as_str()); }
        }
    }
    assert_eq!(observed.len(), 540);
    assert_eq!(profiles.manifest["files"]["original_context_profile.json"]["sha256"], "9b5226ef835cde71a989858d06ee4b5618d4ae5d93f1ec50f420d106157b8373");
    assert_eq!(profiles.manifest["files"]["original_context_extensions.json"]["sha256"], "beee047f3dfc2e100ab05066fe4b6bf443304406bc935e6e83212a18da44407e");
}
fn tree_kinds() -> Vec<(String, &'static str, usize)> {
    let mut result = vec![("phone-dur-tree".into(), "phone-dur-tree", 2), ("state-dur-tree".into(), "state-dur-tree", 10)];
    for state in 0..5 { result.push((format!("cep{state}"), "spectrum-tree", 246)); }
    for state in 0..5 { result.push((format!("pitch{state}"), "pitch-tree", 9)); }
    result
}
fn leaf_values(kind: &str, width: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(width * 2);
    for leaf in 0..2 {
        for dimension in 0..width {
            let value = if kind == "pitch-tree" && dimension == 0 { if leaf == 0 { 0.4 } else { 0.6 } } else { 1.0 + leaf as f64 + dimension as f64 / 256.0 };
            values.push(value);
        }
    }
    values
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("verdant-model-synthetic-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
fn raw_fixture(directory: &Path) -> (PathBuf, PathBuf) {
    let mut binary = Vec::new();
    let mut cdata = String::new();
    let mut items = String::new();
    for (name, kind, width) in tree_kinds() {
        let start = cdata.len();
        cdata.push_str(&format!(" -1 1 {}\n", width + 1));
        let begin = binary.len();
        for value in leaf_values(kind, width) { binary.extend(value.to_le_bytes()); }
        items.push_str(&format!("<{kind} name=\"{name}\" mbegin=\"{begin}\" mend=\"{}\">{start}-{}</{kind}>", binary.len(), cdata.len() - 1));
    }
    let start = cdata.len();
    cdata.push_str("0.25 0.50\n");
    items.push_str(&format!("<pulse>{start}-{}</pulse>", cdata.len() - 1));
    let xml = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><voices><voice name=\"synthetic\" vid=\"0\"><token sample-rate=\"16000\" lsf_coef_order=\"24\" lsf_diff_order=\"16\"><qstable><queset name=\"unrecovered\">1</queset></qstable>{items}</token></voice><![CDATA[{cdata}]]></voices>");
    let token = directory.join("synthetic.tkn");
    let model = directory.join("independent.bin");
    fs::write(&token, xml).unwrap();
    fs::write(&model, binary).unwrap();
    (token, model)
}
#[test]
fn raw_selection_uses_unchanged_f64_records_and_audited_shared_answers() {
    let scratch = Scratch::new();
    let (token, model) = raw_fixture(&scratch.0);
    let voice = load_raw_voice(&token, &model).unwrap();
    assert!(!voice.source_identity_confirmed());
    let coherent = voice.select_phone(&json!({})).unwrap();
    assert_eq!(coherent.arrays["spectral_means"].shape, [5, 3, 41]);
    assert_eq!(coherent.arrays["pitch_triplets"].shape, [5, 3, 3]);
    assert!(coherent.arrays.values().all(|a| a.dtype == "float64"));
    assert_eq!(coherent.arrays["spectral_means"].values[0].to_bits(), 1.0f64.to_bits());
    assert_eq!(coherent.arrays["pitch_triplets"].values[0].to_bits(), 0.4f64.to_bits());
    assert_eq!(coherent.metadata["cross_tree_question_truth_conflicts"], json!([]));
    assert_eq!(coherent.metadata["shared_diagnostic_completions"].as_array().unwrap().len(), 1);
    assert!(voice.select_phone(&json!({"policy": "strict"})).is_err());
    let strict = voice.select_phone(&json!({"policy": "strict", "answers": {"unrecovered": true}})).unwrap();
    assert_eq!(strict.arrays["spectral_means"].values[0].to_bits(), 2.0f64.to_bits());
    let ablation = voice.select_phone(&json!({"selection_mode": "independent-ablation"})).unwrap();
    assert_eq!(ablation.arrays["pitch_triplets"].values[0].to_bits(), 0.6f64.to_bits());
    assert_eq!(ablation.metadata["cross_tree_question_truth_conflicts"], json!(["unrecovered"]));
}
#[test]
fn topology_rejects_missing_undefined_duplicate_and_trailing_nodes() {
    let questions = BTreeMap::from([(1, "q".into())]);
    for nodes in [vec![-1, 1], vec![-2, 1, 3], vec![-1, 1, 1], vec![-1, 1, 5], vec![-1, 1, 3, 1], vec![0]] {
        assert!(loading::topology(&nodes, 2, 2, &questions).is_err(), "{nodes:?}");
    }
    assert!(loading::topology(&[-1, 1, 3], 2, 2, &questions).is_ok());
}
fn checksums(directory: &Path) {
    let mut sums = String::new();
    for name in ["voice.json", "tensors.safetensors", "topology.json", "PROVENANCE.json"] {
        sums.push_str(&format!("{}  {name}\n", digest(&fs::read(directory.join(name)).unwrap())));
    }
    fs::write(directory.join("SHA256SUMS"), sums).unwrap();
}
fn modern_fixture(directory: &Path) {
    // Synthetic values deliberately exercise the directory protocol; source claims are fixture data.
    let source = json!({"sn16k.tkn": source_pins::KNOWN_TOKEN_SHA, "sn16k.bin": source_pins::KNOWN_MODEL_SHA});
    let mut trees = serde_json::Map::new();
    let mut topology = serde_json::Map::new();
    let mut header = serde_json::Map::new();
    let mut body = Vec::new();
    for (name, kind, width) in tree_kinds() {
        let key = name.replace('-', "_");
        let raw = format!("{key}_raw");
        let start = body.len();
        for value in leaf_values(kind, width) { body.extend(value.to_le_bytes()); }
        let end = body.len();
        trees.insert(key, json!({"name": name, "kind": kind, "leaf_count": 2, "values_per_leaf": width, "byte_range": [start, end], "raw_tensor": raw}));
        topology.insert(name, json!([-1, 1, width + 1]));
        header.insert(raw, json!({"dtype": "F64", "shape": [2, width], "data_offsets": [start, end]}));
    }
    let header = serde_json::to_vec(&header).unwrap();
    let mut tensors = (header.len() as u64).to_le_bytes().to_vec();
    tensors.extend(header);
    tensors.extend(&body);
    fs::write(directory.join("tensors.safetensors"), tensors).unwrap();
    let voice = json!({"format": "greendam-voice-model", "format_version": 1, "distribution": {"release_asset": false},
        "sample_rate": 16000, "lsf_coef_order": 24, "lsf_diff_order": 16, "identity": {"name": "synthetic"},
        "questions": {"1": "unrecovered"}, "source_sha256": source, "source_bytes": {"sn16k.bin": body.len()}, "trees": trees});
    fs::write(directory.join("voice.json"), serde_json::to_vec(&voice).unwrap()).unwrap();
    fs::write(directory.join("topology.json"), serde_json::to_vec(&json!({"format": "greendam-voice-topology", "format_version": 1, "source_sha256": source, "trees": topology})).unwrap()).unwrap();
    fs::write(directory.join("PROVENANCE.json"), b"{\"mode\":\"synthetic-format-test\"}").unwrap();
    checksums(directory);
}
#[test]
fn modern_directory_checksums_source_grafts_and_dtype_are_admitted_before_selection() {
    let scratch = Scratch::new();
    modern_fixture(&scratch.0);
    let voice = load_voice(&scratch.0).unwrap();
    assert_eq!(voice.select_phone(&json!({})).unwrap().arrays["phone_duration"].values, leaf_values("phone-dur-tree", 2)[..2]);
    let topology_path = scratch.0.join("topology.json");
    let original = fs::read(&topology_path).unwrap();
    let mut topology: Value = serde_json::from_slice(&original).unwrap();
    topology["source_sha256"]["sn16k.bin"] = json!("unrelated-source");
    fs::write(&topology_path, serde_json::to_vec(&topology).unwrap()).unwrap();
    checksums(&scratch.0);
    assert!(load_voice(&scratch.0).is_err());
    fs::write(&topology_path, original).unwrap();
    checksums(&scratch.0);
    fs::write(scratch.0.join("SHA256SUMS"), "").unwrap();
    assert!(load_voice(&scratch.0).is_err());
    modern_fixture(&scratch.0);
    let tensor_path = scratch.0.join("tensors.safetensors");
    let mut tensors = fs::read(&tensor_path).unwrap();
    let position = tensors.windows(3).position(|w| w == b"F64").unwrap();
    tensors[position..position + 3].copy_from_slice(b"F32");
    fs::write(tensor_path, tensors).unwrap();
    checksums(&scratch.0);
    assert!(load_voice(&scratch.0).is_err());
}
#[test]
fn score_contexts_and_compacted_hierarchies_preserve_indices_and_caller_data() {
    let score = json!({"segments": [
        {"context": {"features": {"C_Phone": "3"}}, "duration_seconds": 0.15},
        {"context": {"features": {"C_Phone": "13"}}, "duration_seconds": 0.15}],
        "original_hierarchy": {"schema_version": "ogd-explicit-original-hierarchy/1", "nodes": [
            {"phone": 3, "left_level": 5, "right_level": 0}, {"phone": 13, "left_level": 0, "right_level": 5}],
            "segment_node_indices": [0, 1], "provenance": "Synthetic explicit caller boundaries"}});
    let before = score.clone();
    let segments = prepare_score_segments(&score, &json!({})).unwrap();
    let graph = validate_score_hierarchy(&score).unwrap().unwrap();
    let report = compact_hierarchy_report(&json!({"segments": segments}), Some(&graph)).unwrap();
    assert_eq!(score, before);
    assert_eq!(report["original_hierarchy_graphs"].as_object().unwrap().len(), 1);
    let first = &report["segments"][0]["context"]["features"]["OriginalHierarchy"];
    let second = &report["segments"][1]["context"]["features"]["OriginalHierarchy"];
    assert_eq!(first["current_index"], 0);
    assert_eq!(second["current_index"], 1);
    assert_eq!(first["graph_sha256"], second["graph_sha256"]);
    let mut wrong = score;
    wrong["segments"][0]["context"]["answers"] = json!({"C_LSLevel==1": true});
    assert!(prepare_score_segments(&wrong, &json!({})).is_err());
    let proxy = prepare_score_segments(&json!({"segments": [{"vowel": "e"}, {"vowel": "u", "context_mode": "observed-chinese"}]}), &json!({})).unwrap();
    assert_eq!(proxy[0]["context"]["features"]["C_Phone"], "183");
    assert_eq!(proxy[0]["context"]["anchor_questions"], json!([]));
    assert_eq!(proxy[1]["context"]["features"]["C_Phone"], 29);
    assert_eq!(proxy[1]["context"]["anchor_questions"], json!([]));
}

#[test]
fn segment_identity_uses_original_phone_resolution_without_a_model() {
    assert_eq!(resolve_segment_identity(&json!({"context": {"features": {"C_SM": "m"}}})).unwrap(),
        json!({"phone_id": 53, "language": 1, "native_key": "zh:m"}));
    assert_eq!(resolve_segment_identity(&json!({"context": {"features": {"C_Phone": 217}}})).unwrap(),
        json!({"phone_id": 217, "language": 3, "native_key": "en:m"}));
    assert_eq!(resolve_segment_identity(&json!({"vowel": "a"})).unwrap(),
        json!({"phone_id": 174, "language": 3, "native_key": "en:aa"}));
    assert_eq!(resolve_segment_identity(&json!({"context": {}})).unwrap(),
        json!({"phone_id": null, "language": null, "native_key": null}));
    assert!(resolve_segment_identity(&json!({"context": {"features": {"C_Phone": 217, "C_SM": "m"}}})).is_err());
}
#[test]
fn marked_continuations_share_physical_neighbors_and_keep_every_note_segment() {
    let score = json!({"segments": [
        {"context": {"features": {"C_Phone": 53}}, "duration_seconds": 0.1},
        {"context": {"features": {"C_Phone": 3}}, "duration_seconds": 0.2, "label": "anchor"},
        {"context": {"features": {"C_Phone": 3}}, "duration_seconds": 0.3, "continuation_of": 1, "label": "note-two"},
        {"context": {"features": {"C_Phone": 3}}, "duration_seconds": 0.4, "continuation_of": 1, "label": "note-three"},
        {"context": {"features": {"C_Phone": 26}}, "duration_seconds": 0.2}]});
    let before = score.clone();
    let prepared = prepare_score_segments(&score, &json!({})).unwrap();
    assert_eq!(score, before);
    assert_eq!(prepared.len(), 5);
    for index in 1..=3 {
        assert_eq!(prepared[index]["context"]["features"]["L_Phone"], "53");
        assert_eq!(prepared[index]["context"]["features"]["R_Phone"], "26");
        assert_eq!(prepared[index]["duration_seconds"], score["segments"][index]["duration_seconds"]);
        assert_eq!(prepared[index]["label"], score["segments"][index]["label"]);
        assert_eq!(prepared[index].get("continuation_of"), score["segments"][index].get("continuation_of"));
    }
    assert_eq!(prepared[0]["context"]["features"]["R_Phone"], "3");
    assert_eq!(prepared[4]["context"]["features"]["L_Phone"], "3");
    let mut unmarked = score.clone();
    for segment in unmarked["segments"].as_array_mut().unwrap() { segment.as_object_mut().unwrap().remove("continuation_of"); }
    let ordinary = prepare_score_segments(&unmarked, &json!({})).unwrap();
    assert_eq!(ordinary[1]["context"]["features"]["R_Phone"], "3");
    assert_eq!(ordinary[2]["context"]["features"]["L_Phone"], "3");
    assert_eq!(ordinary[2]["context"]["features"]["R_Phone"], "3");
    let mut explicit = score.clone();
    explicit["segments"][2]["context"]["features"]["R_SilSp"] = json!("sp");
    let preserved = prepare_score_segments(&explicit, &json!({})).unwrap();
    assert_eq!(preserved[2]["context"]["features"]["R_SilSp"], "sp");
    assert!(preserved[2]["context"]["features"].get("R_Phone").is_none());
    let mut paused = score.clone();
    paused["segments"][3]["gap_after_seconds"] = json!(0.1);
    let paused = prepare_score_segments(&paused, &json!({})).unwrap();
    for segment in &paused[1..=3] { assert_eq!(segment["context"]["features"]["R_Phone"], "2"); }
    assert_eq!(paused[4]["context"]["features"]["L_Phone"], "2");
    let mut broken = score;
    broken["segments"][1]["gap_after_seconds"] = json!(0.1);
    assert!(prepare_score_segments(&broken, &json!({})).is_err());
}
