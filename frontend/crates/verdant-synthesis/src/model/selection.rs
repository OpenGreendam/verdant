//! Deterministic unchanged-leaf selection, including shared diagnostic completion across all streams.
use std::collections::BTreeMap;
use anyhow::{bail, ensure, Context as _, Result};
use serde_json::{json, Value};
use crate::{Arrays, NumericArray, SelectedPhone};
use super::{context::{Answer, ConstraintError, ContextState, Facts, Undo}, Node, Tree, TreeKind, VoiceModel};

const BASE_ORDER: [&str; 12] = ["cep2", "cep1", "cep3", "cep0", "cep4", "pitch2", "pitch1", "pitch3", "pitch0", "pitch4", "state-dur-tree", "phone-dur-tree"];
const SEED_POLICY: &str = "first exact-phone-anchor-compatible tree in spectral-priority order; then fixed tree order";
#[derive(Clone, Copy)]
struct Candidate { leaf: usize, anchors: usize }
enum Event<'a> {
    Enter(usize),
    Branch { question: &'a str, child: usize, truth: bool, answer: Answer<'a> },
    Restore { question: &'a str, previous: Option<bool>, bound: Option<Undo<'a>>, hit: usize },
}
fn compatible_leaves(voice: &VoiceModel, tree: &Tree, context: &ContextState, facts: &Facts<'_>, strict: bool, first_child_truth: bool) -> Result<Vec<Candidate>> {
    let mut constraints = context.bounds(facts)?;
    let mut local = BTreeMap::new();
    let mut stack = vec![Event::Enter(0)];
    let mut candidates = Vec::new();
    let mut anchors = 0usize;
    while let Some(event) = stack.pop() {
        match event {
            Event::Enter(index) => match tree.nodes[index] {
                Node::Leaf(leaf) => candidates.push(Candidate { leaf, anchors }),
                Node::Branch { question_id, first, second } => {
                    let question = voice.questions.get(&question_id).context("tree references an undefined question")?.as_str();
                    let answer = context.answer(question, facts, &constraints)?;
                    if strict && answer.truth.is_none() { bail!("{}: unresolved question {question_id} {question}", tree.name); }
                    // Stack reversal preserves the original first-child DFS order.
                    stack.push(Event::Branch { question, child: second, truth: !first_child_truth, answer });
                    stack.push(Event::Branch { question, child: first, truth: first_child_truth, answer });
                }
            },
            Event::Branch { question, child, truth, answer } => {
                if answer.truth.is_some_and(|known| known != truth) || local.get(question).is_some_and(|&known| known != truth) { continue; }
                let bound = match constraints.apply(question, truth, answer.truth.is_none() || answer.source.diagnostic(), facts) {
                    Ok(undo) => undo,
                    Err(error) if error.is::<ConstraintError>() => continue,
                    Err(error) => return Err(error),
                };
                let previous = local.insert(question, truth);
                let hit = usize::from(truth && context.anchor_questions.iter().any(|anchor| anchor == question));
                anchors += hit;
                stack.push(Event::Restore { question, previous, bound, hit });
                stack.push(Event::Enter(child));
            }
            Event::Restore { question, previous, bound, hit } => {
                anchors -= hit;
                constraints.restore(bound);
                if let Some(previous) = previous { local.insert(question, previous); } else { local.remove(question); }
            }
        }
    }
    ensure!(!candidates.is_empty(), "{}: no compatible leaf", tree.name);
    Ok(candidates)
}
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let index = (sorted.len() - 1) as f64 * q;
    let low = index.floor() as usize;
    let high = (low + 1).min(sorted.len() - 1);
    let t = index - low as f64;
    let (a, b) = (sorted[low], sorted[high]);
    // NumPy's linear _lerp uses the upper endpoint for t >= 0.5.
    if t >= 0.5 { b - (b - a) * (1.0 - t) } else { a + (b - a) * t }
}
fn median(sorted: &[f64]) -> f64 {
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 { (sorted[mid - 1] + sorted[mid]) / 2.0 } else { sorted[mid] }
}
fn numpy_sum(values: &[f64]) -> f64 {
    if values.len() < 8 { return values.iter().fold(-0.0, |sum, &value| sum + value); }
    let mut lanes = [0.0; 8];
    lanes.copy_from_slice(&values[..8]);
    let mut index = 8;
    while index + 8 <= values.len() { for lane in 0..8 { lanes[lane] += values[index + lane]; } index += 8; }
    let mut sum = ((lanes[0] + lanes[1]) + (lanes[2] + lanes[3])) + ((lanes[4] + lanes[5]) + (lanes[6] + lanes[7]));
    for &value in &values[index..] { sum += value; }
    sum
}
struct Selection { leaf: usize, anchors: usize, before: usize, after: usize, distance: f64 }
fn choose(voice: &VoiceModel, tree: &Tree, context: &ContextState, facts: &Facts<'_>, strict: bool, first_child_truth: bool) -> Result<Selection> {
    let mut candidates = compatible_leaves(voice, tree, context, facts, strict, first_child_truth)?;
    let before = candidates.len();
    let anchors = candidates.iter().map(|c| c.anchors).max().context("no compatible leaf")?;
    candidates.retain(|c| c.anchors == anchors);
    if tree.kind == TreeKind::Pitch && !strict && candidates.iter().any(|c| tree.record(c.leaf)[0] >= 0.5) {
        candidates.retain(|c| tree.record(c.leaf)[0] >= 0.5);
    }
    let (first, dimensions) = match tree.kind { TreeKind::Spectrum => (16, 24), TreeKind::Pitch => (1, 1), _ => (0, tree.kind.width() / 2) };
    let mut medians = [0.0; 24];
    let mut scales = [0.0; 24];
    let mut column = Vec::with_capacity(candidates.len());
    for dimension in 0..dimensions {
        column.clear();
        column.extend(candidates.iter().map(|c| tree.record(c.leaf)[first + dimension]));
        column.sort_unstable_by(f64::total_cmp);
        medians[dimension] = median(&column);
        scales[dimension] = (quantile(&column, 0.75) - quantile(&column, 0.25)).max(1e-4);
    }
    let mut best: Option<(usize, f64)> = None;
    for candidate in &candidates {
        let record = tree.record(candidate.leaf);
        let mut squares = [0.0; 24];
        for dimension in 0..dimensions {
            let value = (record[first + dimension] - medians[dimension]) / scales[dimension];
            squares[dimension] = value * value;
        }
        let distance = numpy_sum(&squares[..dimensions]) / dimensions as f64;
        ensure!(distance.is_finite(), "{}: nonfinite diagnostic medoid distance", tree.name);
        if best.is_none_or(|(leaf, previous)| distance < previous || (distance == previous && candidate.leaf < leaf)) { best = Some((candidate.leaf, distance)); }
    }
    let (leaf, distance) = best.context("no candidate after selection preferences")?;
    Ok(Selection { leaf, anchors, before, after: candidates.len(), distance })
}
fn evidence(voice: &VoiceModel, tree: &Tree, selection: &Selection, context: &ContextState, facts: &Facts<'_>, policy: &str, first_child_truth: bool) -> Result<Value> {
    let mut indices = Vec::new();
    let mut node = tree.leaf_nodes[selection.leaf];
    while let Some((parent, first)) = tree.parents[node] { indices.push((parent, first)); node = parent; }
    indices.reverse();
    let mut constraints = context.bounds(facts)?;
    let mut path = Vec::with_capacity(indices.len());
    let mut unknown = Vec::new();
    for (index, first) in indices {
        let Node::Branch { question_id, .. } = tree.nodes[index] else { bail!("invalid parent topology"); };
        let question = voice.questions.get(&question_id).context("undefined selected question")?;
        let answer = context.answer(question, facts, &constraints)?;
        let truth = if first { first_child_truth } else { !first_child_truth };
        constraints.apply(question, truth, answer.truth.is_none() || answer.source.diagnostic(), facts)?;
        if answer.truth.is_none() { unknown.push(question); }
        path.push(json!({"question_id": question_id, "question": question, "answer": answer.truth,
            "answer_source": answer.source.text(), "chosen_child": if first { "first" } else { "second" }, "assumed_child_truth": truth}));
    }
    Ok(json!({
        "tree": tree.name, "leaf_index_0based": selection.leaf,
        "leaf_double_offset_1based": selection.leaf * tree.kind.width() + 1,
        "leaf_record_bytes": tree.kind.width() * 8, "context": context.json(facts)?, "policy": policy,
        "branch_polarity": if first_child_truth { "first_true" } else { "first_false" },
        "branch_polarity_status": if first_child_truth { "caller-selected first-true override of original polarity" } else { "verified original parser +4 first / +8 second and evaluator false/true branches; RVA 0x68e5e/0x68e82/0x5832c" },
        "compatible_leaf_count_before_anchor_preference": selection.before,
        "compatible_leaf_count_after_preferences": selection.after,
        "positive_exact_anchor_hits": selection.anchors,
        "numeric_constraints_on_selected_path": constraints.json(), "medoid_distance": selection.distance,
        "unknown_questions_on_selected_path": unknown, "path": path,
        "selection_status": if policy == "strict" { "exact_for_supplied_answers_and_assumed_polarity" } else { "diagnostic selection among unresolved branches; not original frontend output" },
    }))
}
fn option_string<'a>(value: &'a Value, key: &str, default: &'a str) -> Result<&'a str> {
    value.get(key).map(|v| v.as_str().with_context(|| format!("{key} must be a string"))).unwrap_or(Ok(default))
}
pub(super) fn select(voice: &VoiceModel, value: &Value) -> Result<SelectedPhone> {
    let policy = option_string(value, "policy", "diagnostic-medoid")?;
    ensure!(matches!(policy, "strict" | "diagnostic-medoid"), "policy must be strict or diagnostic-medoid");
    let mode = option_string(value, "selection_mode", "coherent")?;
    ensure!(matches!(mode, "coherent" | "independent-ablation"), "selection_mode must be coherent or independent-ablation");
    let first_child_truth = value.get("first_child_truth").map(|v| v.as_bool().context("first_child_truth must be boolean")).transpose()?.unwrap_or(false);
    let mut context = ContextState::parse(value)?;
    let facts = Facts::new(&context.features)?;
    let initial_context = context.json(&facts)?;
    let strict = policy == "strict";
    let mut anchor_probes = Vec::new();
    let mut order: Vec<&str> = if mode == "coherent" { BASE_ORDER.to_vec() } else { voice.tree_order.iter().map(String::as_str).collect() };
    if mode == "coherent" && !strict {
        let mut seed = BASE_ORDER[0];
        let mut found = false;
        for name in BASE_ORDER {
            let tree = voice.trees.get(name).context("missing audited selection tree")?;
            let candidates = compatible_leaves(voice, tree, &context, &facts, false, first_child_truth)?;
            let anchors = candidates.iter().map(|c| c.anchors).max().context("no compatible anchor probe")?;
            anchor_probes.push(json!({"tree": name, "positive_exact_anchor_hits": anchors}));
            if !found && anchors > 0 { seed = name; found = true; }
        }
        order.retain(|&name| name != seed);
        order.insert(0, seed);
    }
    let mut choices = Vec::with_capacity(12);
    let mut records = BTreeMap::new();
    let mut completions = Vec::new();
    let mut shared_truths = BTreeMap::new();
    let mut conflicts = std::collections::BTreeSet::new();
    for &name in &order {
        let tree = voice.trees.get(name).context("missing audited tree")?;
        let selected = choose(voice, tree, &context, &facts, strict, first_child_truth)?;
        let evidence = evidence(voice, tree, &selected, &context, &facts, policy, first_child_truth)?;
        for step in evidence["path"].as_array().context("invalid internal selection path")? {
            let question = step["question"].as_str().context("invalid selected question")?;
            let truth = step["assumed_child_truth"].as_bool().context("invalid selected truth")?;
            if let Some(previous) = shared_truths.insert(question.to_owned(), truth) { if previous != truth { conflicts.insert(question.to_owned()); } }
            if mode == "coherent" && step["answer"].is_null() {
                if let Some(previous) = context.diagnostic_answers.get(question) { ensure!(*previous == truth, "contradictory diagnostic context completion"); }
                else {
                    context.diagnostic_answers.insert(question.to_owned(), truth);
                    completions.push(json!({"question": question, "assumed_answer": truth, "introduced_by_tree": name, "status": "diagnostic assumption, not recovered frontend output"}));
                }
            }
        }
        records.insert(name, tree.record(selected.leaf));
        choices.push(evidence);
    }
    ensure!(mode != "coherent" || conflicts.is_empty(), "cross-tree context contradiction");
    let mut means = Vec::with_capacity(5 * 123);
    let mut variances = Vec::with_capacity(5 * 123);
    let mut pitch = Vec::with_capacity(5 * 9);
    let mut probabilities = Vec::with_capacity(5);
    for state in 0..5 {
        let spectrum = records.get(format!("cep{state}").as_str()).context("missing selected spectrum")?;
        means.extend_from_slice(&spectrum[..123]);
        variances.extend_from_slice(&spectrum[123..]);
        let record = records.get(format!("pitch{state}").as_str()).context("missing selected pitch")?;
        pitch.extend_from_slice(record);
        probabilities.push(record[0]);
    }
    let duration = records["state-dur-tree"];
    let mut arrays = Arrays::new();
    for (key, shape, values) in [
        ("spectral_means", vec![5, 3, 41], means),
        ("spectral_variances", vec![5, 3, 41], variances),
        ("pitch_triplets", vec![5, 3, 3], pitch),
        ("state_duration_means", vec![5], duration[..5].to_vec()),
        ("state_duration_variances", vec![5], duration[5..].to_vec()),
        ("phone_duration", vec![2], records["phone-dur-tree"].to_vec()),
    ] { arrays.insert(key.to_owned(), NumericArray::new(shape, "float64", values)?); }
    let native = facts.phone("C").and_then(|id| facts.profiles.native_phones.get(&id)).map(|(key, phone)| json!({"native_key": key, "phone_id": phone.phone_id, "native_language": phone.native_language, "native_phone": phone.native_phone, "role": phone.role, "voiced": phone.voiced}));
    let metadata = json!({
        "initial_context": initial_context, "context": context.json(&facts)?, "native_key": facts.native_key(),
        "phone_id": facts.phone("C"), "language": facts.language("C"), "native_identity": native,
        "selection_mode": mode, "diagnostic_completion_order": order, "diagnostic_seed_policy": SEED_POLICY,
        "diagnostic_anchor_probes_before_completion": anchor_probes, "shared_diagnostic_completions": completions,
        "cross_tree_question_truth_conflicts": conflicts, "leaf_selections": choices, "state_probabilities": probabilities,
        "phone_duration_record_unused_in_timing": records["phone-dur-tree"], "provenance": voice.provenance,
    });
    Ok(SelectedPhone { arrays, metadata })
}
