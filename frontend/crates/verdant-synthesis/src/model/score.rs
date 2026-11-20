//! Score context normalization and lossless original-hierarchy audit compaction.
use std::collections::BTreeMap;
use anyhow::{bail, ensure, Context as _, Result};
use serde_json::{json, Map, Value};
use super::{context::{finite_number, original_answer, ContextState, Facts, Hierarchy, HIERARCHY_SCHEMA}, data::{digest, Kind}};

fn provenance(value: &Value) -> Result<&str> {
    let text = value.as_str().context("original_hierarchy.provenance must describe the supplied boundaries")?;
    ensure!(!text.trim().is_empty() && text.chars().count() <= 2048, "original_hierarchy.provenance must contain 1..2048 characters describing the supplied boundaries");
    Ok(text)
}
fn segment_features(segment: &Value) -> Result<&Map<String, Value>> {
    segment.get("context").and_then(|v| v.get("features")).and_then(Value::as_object).context("original_hierarchy requires an explicit phone context for every segment")
}
fn segment_phone(segment: &Value) -> Result<i32> {
    let facts = Facts::new(segment_features(segment)?)?;
    facts.phone("C").context("original_hierarchy segment has no resolved original phone identity")
}
fn validate_answers(segment: &Value, features: &Map<String, Value>) -> Result<()> {
    let facts = Facts::new(features)?;
    if let Some(answers) = segment.get("context").and_then(|v| v.get("answers")) {
        for (question, truth) in answers.as_object().context("context.answers must be an object")? {
            if facts.profiles.questions.get(question).is_some_and(|s| s.kind == Kind::Hierarchy) {
                let truth = truth.as_bool().context("explicit hierarchy answers must be boolean")?;
                if let Some(answer) = original_answer(question, &facts)? {
                    ensure!(answer.truth.is_none_or(|known| known == truth), "explicit {question} contradicts the supplied original hierarchy");
                }
            }
        }
    }
    Ok(())
}
/// Validate caller-owned hierarchy and per-segment node identities without predicting boundaries.
pub fn validate_score_hierarchy(score: &Value) -> Result<Option<Value>> {
    ensure!(score.is_object(), "score must be an object");
    let empty = Vec::new();
    let segments = score.get("segments").map(|v| v.as_array().context("score.segments must be an array")).transpose()?.unwrap_or(&empty);
    let Some(supplied) = score.get("original_hierarchy").filter(|v| !v.is_null()) else {
        for segment in segments {
            if let Some(direct) = segment.get("context").and_then(|v| v.get("features")).and_then(|v| v.get("OriginalHierarchy")).filter(|v| !v.is_null()) {
                Hierarchy::parse(direct, Some(&json!(segment_phone(segment)?)))?;
                provenance(&direct["provenance"])?;
                validate_answers(segment, segment_features(segment)?)?;
            }
        }
        return Ok(None);
    };
    let supplied_object = supplied.as_object().context("original_hierarchy must be an object")?;
    ensure!(supplied["schema_version"] == HIERARCHY_SCHEMA && supplied_object.keys().all(|key| matches!(key.as_str(), "schema_version" | "nodes" | "segment_node_indices" | "provenance")), "original_hierarchy requires schema, nodes, segment_node_indices and provenance");
    let provenance = provenance(&supplied["provenance"])?;
    let indices = supplied["segment_node_indices"].as_array().context("original_hierarchy requires segment_node_indices")?;
    ensure!(!indices.is_empty() && indices.len() == segments.len(), "original_hierarchy requires exactly one node index per segment");
    let indices = indices.iter().map(|v| v.as_i64().context("original_hierarchy segment node indices must be integers")).collect::<Result<Vec<_>>>()?;
    ensure!(indices.windows(2).all(|w| w[0] <= w[1]), "original_hierarchy segment indices must follow the original sequence order");
    let nodes = supplied.get("nodes").context("original_hierarchy requires nodes")?;
    let hierarchy = Hierarchy::parse(&json!({"schema_version": HIERARCHY_SCHEMA, "nodes": nodes, "current_index": indices[0]}), None)?;
    for (segment, &index) in segments.iter().zip(&indices) {
        ensure!(index >= 0 && (index as usize) < hierarchy.nodes.len(), "original_hierarchy segment node index is outside the supplied graph");
        let phone = hierarchy.nodes[index as usize][0];
        ensure!(segment_phone(segment)? == phone, "original_hierarchy node phone conflicts with its mapped segment phone");
        let original = segment_features(segment)?;
        if let Some(direct) = original.get("OriginalHierarchy").filter(|v| !v.is_null()) {
            let direct = Hierarchy::parse(direct, Some(&json!(phone)))?;
            ensure!(direct.nodes == hierarchy.nodes && direct.current == index as usize, "segment OriginalHierarchy conflicts with the score-level mapping");
        }
        let mut features = original.clone();
        features.insert("OriginalHierarchy".into(), json!({"schema_version": HIERARCHY_SCHEMA, "nodes": nodes, "current_index": index}));
        validate_answers(segment, &features)?;
    }
    Ok(Some(json!({"schema_version": HIERARCHY_SCHEMA, "nodes": hierarchy.normalized_nodes(), "segment_node_indices": indices, "provenance": provenance})))
}
fn diagnostic_context(vowel: &str, english: bool) -> Result<ContextState> {
    let (phone_id, proxy) = match vowel { "a" => (3, "aa"), "i" => (13, "iy"), "u" => (29, "uw"), "e" => (8, "eh"), "o" => (26, "ow"), _ => bail!("diagnostic vowels are a, i, u, e, o") };
    let mut value = json!({"features": {"C_Phone": phone_id, "C_Language": 1, "C_YM": vowel,
        "C_Tone": "1", "L_SilSp": "sil", "R_SilSp": "sil"},
        "answers": {"C_EN": false, "C_SM": false, "C_Voiced": true, "C_Vowel": false, "C_YM": true,
            "C_SilSp": false, "C_Nasal": false, "C_Fric": false, "C_Stop": false, "L_SilSp": true, "R_SilSp": true},
        "label": format!("diagnostic_mandarin_{vowel}_phone_{phone_id}"),
        "assumptions": [
            "Explicit isolated native Mandarin vowel context; original text frontend and prosodic labels are not recovered.",
            "Native Mandarin phone IDs and group predicates are used even when this model has no literal question for that vowel.",
            "An anchor is retained only when the original registered literal predicate agrees with the selected phone ID; it may be absent from a particular model tree.",
            "Isolated left/right sil neighbors and tone 1 are explicit diagnostic assumptions, not original frontend output."]});
    let anchor = if english {
        let features = value["features"].as_object_mut().context("diagnostic context features")?;
        for key in ["C_Phone", "C_YM", "C_Tone"] { features.remove(key); }
        features.insert("C_Language".into(), json!(3));
        features.insert("C_Stress".into(), json!(0));
        features.insert("C_Vowel".into(), json!(proxy));
        let answers = value["answers"].as_object_mut().context("diagnostic context answers")?;
        for (key, truth) in [("C_EN", true), ("C_Vowel", true), ("C_YM", false), ("C_Voiced", false)] { answers.insert(key.into(), json!(truth)); }
        value["label"] = json!(format!("diagnostic_english_proxy_{vowel}_via_{proxy}"));
        value["assumptions"] = json!([
            format!("Explicit English vowel proxy {proxy}; not a certified Mandarin /{vowel}/ path."),
            "Synthetic isolated context; current-phone categories follow the original table and unresolved neighboring/prosodic questions are retained."]);
        format!("C_Vowel=={proxy}")
    } else { format!("C_YM=={vowel}") };
    let mut context = ContextState::parse(&value)?;
    if original_answer(&anchor, &Facts::new(&context.features)?)?.is_some_and(|answer| answer.truth == Some(true)) { context.anchor_questions.push(anchor); }
    Ok(context)
}
fn context_for_segment(segment: &Value) -> Result<ContextState> {
    if let Some(context) = segment.get("context") {
        let context = context.as_object().context("segment context must be an object")?;
        // The production score adapter constructs a fresh Context, keeping only caller facts.
        let mut value = Value::Object(context.clone());
        value.as_object_mut().context("context must be an object")?.remove("diagnostic_answers");
        if !context.contains_key("label") { value["label"] = segment.get("label").cloned().unwrap_or_else(|| json!("explicit_context")); }
        return ContextState::parse(&value);
    }
    let mode = segment.get("context_mode").map(|v| v.as_str().context("context_mode must be a string")).transpose()?.unwrap_or("english-proxy");
    let english = match mode { "english-proxy" => true, "observed-chinese" => false, _ => bail!("unknown context mode; supply an explicit context object") };
    diagnostic_context(segment["vowel"].as_str().context("diagnostic segment requires vowel")?, english)
}
/// Resolve the same original phone identity used by selection without loading voice weights.
/// Unprovided identities remain null; contradictory explicit facts are rejected.
pub fn resolve_segment_identity(segment: &Value) -> Result<Value> {
    ensure!(segment.is_object(), "score segment must be an object");
    let context = context_for_segment(segment)?;
    let facts = Facts::new(&context.features)?;
    context.bounds(&facts)?;
    Ok(json!({"phone_id": facts.phone("C"), "language": facts.language("C"), "native_key": facts.native_key()}))
}
fn gap(segment: &Value) -> Result<f64> {
    segment.get("gap_after_seconds").map(|v| finite_number(v).context("gap_after_seconds must be finite and numeric")).unwrap_or(Ok(0.0))
}
/// Prepare all contexts and preserve segment fields before the parameter planner allocates frames.
pub fn prepare_score_segments(score: &Value, options: &Value) -> Result<Vec<Value>> {
    ensure!(score.is_object() && options.is_object(), "score and options must be objects");
    let raw = score["segments"].as_array().context("score.segments must be an array")?;
    ensure!(raw.iter().all(Value::is_object), "each score segment must be an object");
    // The numeric planner also accepts scores directly, without the worker's
    // JSON schema. Preserve explicit frontend ownership as metadata, but do
    // not let malformed values silently become unowned acoustic segments.
    // Other existing trace metadata remains lossless; no note identity is
    // inferred from phoneme indices or from an acoustic model context.
    for segment in raw {
        if let Some(owner) = segment.get("phoneme").and_then(|trace| trace.get("source_note_id")) {
            ensure!(owner.as_str().is_some_and(|owner| !owner.is_empty()),
                "phoneme.source_note_id must be a nonempty explicit frontend note ID");
        }
    }
    let mode = options.get("neighbor_mode").map(|v| v.as_str().context("neighbor_mode must be a string")).transpose()?.unwrap_or("score-adjacent");
    ensure!(matches!(mode, "score-adjacent" | "unspecified-ablation"), "neighbor_mode must be score-adjacent or unspecified-ablation");
    let source_indices = if raw.iter().any(|segment| segment.get("continuation_of").is_some()) {
        Some(crate::parameters::continuation_source_indices(raw)?)
    } else { None };
    let graph = validate_score_hierarchy(score)?;
    let mut contexts = raw.iter().map(context_for_segment).collect::<Result<Vec<_>>>()?;
    if mode == "score-adjacent" {
        let ids = contexts.iter().map(|context| Ok(Facts::new(&context.features)?.phone("C"))).collect::<Result<Vec<_>>>()?;
        let mut first = 0;
        while first < contexts.len() {
            let mut last = first;
            if let Some(indices) = &source_indices {
                while last + 1 < contexts.len() && indices[last + 1] == indices[first] { last += 1; }
            }
            if ids[first..=last].iter().any(Option::is_some) {
                // A marked note continuation is a slice of its anchor phone. Its
                // internal note boundaries do not introduce acoustic neighbors.
                let left = if first == 0 { Some(0) } else if gap(&raw[first - 1])? > 0.0 { Some(2) } else { ids[first - 1] };
                let right = if gap(&raw[last])? > 0.0 { Some(2) } else if last + 1 == contexts.len() { Some(0) } else { ids[last + 1] };
                for index in first..=last {
                    let Some(current) = ids[index] else { continue; };
                    let context = &mut contexts[index];
                    context.features.entry("C_Phone").or_insert_with(|| json!(current.to_string()));
                    for (position, phone) in [("L", left), ("R", right)] {
                        let Some(phone) = phone else { continue; };
                        if !["Phone", "YM", "SM", "Vowel", "Stop", "Nasal", "Fric", "SilSp"].iter().any(|suffix| context.features.contains_key(&format!("{position}_{suffix}"))) {
                            context.features.insert(format!("{position}_Phone"), json!(phone.to_string()));
                        }
                    }
                    context.assumptions.push("Score-adjacent phones supply L/R identity; 0 means absent and 2 means authored short pause. Original neighbor iterators and neighboring tone/stress traversal are not reconstructed.".into());
                    if first != last {
                        context.assumptions.push("Explicit continuation links share one physical source phone for neighbor identity; note segments remain separate.".into());
                    }
                }
            }
            first = last + 1;
        }
    }
    let mut result = Vec::with_capacity(raw.len());
    for (index, (segment, mut context)) in raw.iter().zip(contexts).enumerate() {
        if let Some(graph) = &graph {
            context.features.insert("OriginalHierarchy".into(), json!({"schema_version": HIERARCHY_SCHEMA,
                "nodes": graph["nodes"], "current_index": graph["segment_node_indices"][index], "provenance": graph["provenance"]}));
            context.assumptions.push("Original hierarchy nodes supplied explicitly by caller; original text-to-hierarchy prediction is not executed.".into());
        }
        let mut segment = segment.clone();
        segment["context"] = context.json(&Facts::new(&context.features)?)?;
        result.push(segment);
    }
    Ok(result)
}
fn canonical(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<_, _> = values.iter().map(|(key, value)| (key.clone(), canonical(value))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        _ => value.clone(),
    }
}
fn graph_reference(value: &Value, graphs: &mut BTreeMap<String, Value>) -> Result<Value> {
    Hierarchy::parse(value, None)?;
    let graph = json!({"schema_version": value["schema_version"], "nodes": value["nodes"]});
    let hash = digest(&serde_json::to_vec(&canonical(&graph))?);
    graphs.entry(hash.clone()).or_insert(graph);
    let mut result = json!({"schema_version": HIERARCHY_SCHEMA, "graph_sha256": hash, "current_index": value["current_index"]});
    if let Some(provenance) = value.get("provenance") { result["provenance"] = provenance.clone(); }
    Ok(result)
}
fn compact_visit(value: &Value, graphs: &mut BTreeMap<String, Value>) -> Result<Value> {
    Ok(match value {
        Value::Object(values) => {
            let mut result = Map::new();
            for (key, value) in values {
                result.insert(key.clone(), if key == "OriginalHierarchy" && value.is_object() && value.get("nodes").is_some() { graph_reference(value, graphs)? } else { compact_visit(value, graphs)? });
            }
            Value::Object(result)
        }
        Value::Array(values) => Value::Array(values.iter().map(|value| compact_visit(value, graphs)).collect::<Result<Vec<_>>>()?),
        _ => value.clone(),
    })
}
/// Keep each complete explicit graph once while retaining every current index and provenance.
pub fn compact_hierarchy_report(report: &Value, score_graph: Option<&Value>) -> Result<Value> {
    ensure!(report.is_object(), "hierarchy report must be an object");
    let mut graphs = BTreeMap::new();
    let mut compact = compact_visit(report, &mut graphs)?;
    if let Some(graph) = score_graph {
        let indices = graph["segment_node_indices"].as_array().context("hierarchy graph lacks segment_node_indices")?;
        let first = indices.first().context("hierarchy graph has no segment mapping")?;
        let mut value = graph.clone();
        value["current_index"] = first.clone();
        let reference = graph_reference(&value, &mut graphs)?;
        compact["original_hierarchy_input"] = json!({"schema_version": HIERARCHY_SCHEMA, "graph_sha256": reference["graph_sha256"],
            "segment_node_indices": indices, "provenance": graph["provenance"], "original_text_frontend_executed": false});
    }
    if !graphs.is_empty() { compact["original_hierarchy_graphs"] = serde_json::to_value(graphs)?; }
    Ok(compact)
}
