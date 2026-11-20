//! Recovered original predicates and finite numeric constraints; absent facts stay unknown.
use std::{cell::RefCell, collections::BTreeMap};
use anyhow::{bail, ensure, Context as _, Result};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use super::data::{profiles, Kind, Profiles, Spec};

pub(super) const CONSTRAINT_SEMANTICS: &str = "original current/neighbor phone groups on explicit linked nodes; explicit original POS labels; resolved tone/stress fields; original hierarchy predicates; finite numeric <= intervals with recovered -1 sentinels; text-to-POS and text-to-hierarchy prediction remain unresolved";
pub(super) const HIERARCHY_SCHEMA: &str = "ogd-explicit-original-hierarchy/1";

pub(super) fn finite_number(value: &Value) -> Option<f64> {
    let value = match value {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    value.is_finite().then_some(value)
}
pub(super) fn integer(value: &Value) -> Option<i32> {
    let number = finite_number(value)?;
    (number.fract() == 0.0 && number >= i32::MIN as f64 && number <= i32::MAX as f64).then_some(number as i32)
}
fn required_integer(value: &Value, field: &str) -> Result<i32> {
    integer(value).with_context(|| format!("{field} must be a signed 32-bit integer"))
}
fn python_string(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hierarchy {
    pub nodes: Vec<[i32; 3]>,
    pub current: usize,
}
impl Hierarchy {
    pub fn parse(value: &Value, current_phone: Option<&Value>) -> Result<Self> {
        ensure!(value.is_object() && value["schema_version"] == HIERARCHY_SCHEMA, "OriginalHierarchy requires schema {HIERARCHY_SCHEMA}");
        let raw = value["nodes"].as_array().context("OriginalHierarchy.nodes must be an array")?;
        ensure!((1..=4096).contains(&raw.len()), "OriginalHierarchy.nodes must contain 1..4096 explicit original nodes");
        let current = required_integer(&value["current_index"], "OriginalHierarchy.current_index")?;
        ensure!(current >= 0 && (current as usize) < raw.len(), "OriginalHierarchy.current_index is outside the supplied sequence");
        let mut nodes = Vec::with_capacity(raw.len());
        for item in raw {
            ensure!(item.is_object(), "each OriginalHierarchy node requires phone, left_level and right_level");
            let phone = required_integer(&item["phone"], "OriginalHierarchy.phone")?;
            ensure!((1..=235).contains(&phone), "OriginalHierarchy.phone must be an original non-absent phone ID 1..235");
            nodes.push([phone, required_integer(&item["left_level"], "OriginalHierarchy.left_level")?, required_integer(&item["right_level"], "OriginalHierarchy.right_level")?]);
        }
        if let Some(phone) = current_phone.filter(|v| !v.is_null()) {
            ensure!(required_integer(phone, "C_Phone")? == nodes[current as usize][0], "OriginalHierarchy current phone conflicts with C_Phone");
        }
        Ok(Self { nodes, current: current as usize })
    }
    pub fn normalized_nodes(&self) -> Value {
        Value::Array(self.nodes.iter().map(|n| json!({"phone": n[0], "left_level": n[1], "right_level": n[2]})).collect())
    }
    fn value(&self, family: &str, inner: i32, outer: i32) -> Result<i32> {
        let nodes = &self.nodes;
        let mut current = self.current;
        let phone = nodes[current][0];
        match family {
            "C_LSLevel" => {
                for node in nodes[..=current].iter().rev() { if node[1] > -2 { return Ok(node[1]); } }
                bail!("C_LSLevel requires a preceding original boundary > -2");
            }
            "C_RSLevel" => {
                for node in &nodes[current..] {
                    if matches!(node[0], 1 | 2) { return Ok(5); }
                    if node[2] > -2 { return Ok(node[2]); }
                }
                bail!("C_RSLevel requires a following original boundary > -2 or silence");
            }
            "C_LWLevel" => {
                for node in nodes[..=current].iter().rev() { if node[1] >= 1 { return Ok(node[1]); } }
                bail!("C_LWLevel requires a terminating original boundary >= 1");
            }
            "C_RWLevel" => {
                for node in &nodes[current..] { if node[2] >= 1 { return Ok(node[2]); } }
                bail!("C_RWLevel requires a terminating original boundary >= 1");
            }
            "C_AN" | "C_AHP" | "C_ATP" | "C_RP" => {}
            _ => bail!("unsupported original hierarchy family {family}"),
        }
        if phone == 1 { return Ok(if matches!(family, "C_AHP" | "C_ATP") { 0 } else { 999 }); }
        // Exactly one left step for sp; do not recursively apply the silence wrapper.
        if phone == 2 { current = current.saturating_sub(1); }
        if family == "C_RP" {
            let mut first = current;
            while nodes[first][1] < inner { first = first.checked_sub(1).context("C_RP requires a preceding original inner boundary")?; }
            let mut last = first;
            while nodes[last][2] < inner {
                last += 1;
                ensure!(last < nodes.len(), "C_RP requires a following original inner boundary");
            }
            let start = nodes[first][1] >= outer;
            let end = nodes[last][2] >= outer;
            return Ok(if end { if start { 0 } else { 3 } } else if start { 1 } else { 2 });
        }
        let count = |left: bool| {
            let mut result = 0;
            let mut index = current;
            loop {
                let node = nodes[index];
                let level = node[if left { 1 } else { 2 }];
                if !matches!(node[0], 1 | 2) && level >= inner { result += 1; }
                if level >= outer { break; }
                if left { if index == 0 { break; } index -= 1; }
                else { index += 1; if index == nodes.len() { break; } }
            }
            result
        };
        Ok(match family { "C_AHP" => count(true), "C_ATP" => count(false), _ => count(true) + count(false) - 1 })
    }
}
/// Context facts are resolved once per phone, with hierarchy calculations shared across tree visits.
pub(super) struct Facts<'a> {
    pub features: &'a Map<String, Value>,
    pub profiles: &'static Profiles,
    pub hierarchy: Option<Hierarchy>,
    phones: [Option<i32>; 3],
    languages: [Option<i32>; 3],
    hierarchy_cache: RefCell<BTreeMap<(&'static str, i32, i32), i32>>,
}
fn position_index(position: &str) -> usize { match position { "L" => 0, "C" => 1, "R" => 2, _ => unreachable!("validated original position") } }
impl<'a> Facts<'a> {
    pub fn new(features: &'a Map<String, Value>) -> Result<Self> {
        let profiles = profiles()?;
        let hierarchy = features.get("OriginalHierarchy").map(|value| Hierarchy::parse(value, features.get("C_Phone"))).transpose()?;
        let mut facts = Self { features, profiles, hierarchy, phones: [None; 3], languages: [None; 3], hierarchy_cache: RefCell::new(BTreeMap::new()) };
        for position in ["L", "C", "R"] {
            let index = position_index(position);
            facts.phones[index] = facts.resolve_phone(position)?;
            let key = ["L_Language", "C_Language", "R_Language"][index];
            facts.languages[index] = if let Some(value) = features.get(key) {
                Some(required_integer(value, &key)?)
            } else {
                facts.phones[index].filter(|id| profiles.phones.contains_key(id)).map(|id| if (174..=235).contains(&id) { 3 } else { 1 })
            };
        }
        Ok(facts)
    }
    pub fn phone(&self, position: &str) -> Option<i32> { self.phones[position_index(position)] }
    pub fn language(&self, position: &str) -> Option<i32> { self.languages[position_index(position)] }
    pub fn native_key(&self) -> Option<&'static str> { self.phone("C").and_then(|id| self.profiles.native_phones.get(&id)).map(|(key, _)| key.as_str()) }
    fn resolve_phone(&self, position: &str) -> Result<Option<i32>> {
        let mut selected = None;
        let mut add = |phone: i32| -> Result<()> {
            ensure!(selected.is_none_or(|old| old == phone), "conflicting {position} phone fields in original context profile");
            selected = Some(phone);
            Ok(())
        };
        let index = position_index(position);
        let explicit = ["L_Phone", "C_Phone", "R_Phone"][index];
        if let Some(value) = self.features.get(explicit) {
            let phone = integer(value).with_context(|| format!("{explicit} must be an audited original phone ID"))?;
            ensure!(self.profiles.phones.contains_key(&phone) || matches!(phone, 1 | 2) || (position != "C" && phone == 0), "{explicit} must be an audited original phone ID; zero is only an absent L/R neighbor");
            add(phone)?;
        }
        if let Some(hierarchy) = &self.hierarchy {
            let index = hierarchy.current as isize + match position { "L" => -1, "C" => 0, _ => 1 };
            add(if index >= 0 && (index as usize) < hierarchy.nodes.len() { hierarchy.nodes[index as usize][0] } else { 0 })?;
        }
        for (fields, low, high) in [(["L_YM", "C_YM", "R_YM"], 3, 42), (["L_SM", "C_SM", "R_SM"], 43, 63)] {
            if let Some(value) = self.features.get(fields[index]) {
                let Some(&phone) = self.profiles.mandarin.get(&python_string(value)) else { return Ok(None); };
                if !(low..=high).contains(&phone) { return Ok(None); }
                add(phone)?;
            }
        }
        for fields in [["L_Vowel", "C_Vowel", "R_Vowel"], ["L_Stop", "C_Stop", "R_Stop"], ["L_Nasal", "C_Nasal", "R_Nasal"], ["L_Fric", "C_Fric", "R_Fric"]] {
            if let Some(value) = self.features.get(fields[index]) {
                let Some(&phone) = self.profiles.english.get(&python_string(value)) else { return Ok(None); };
                add(phone)?;
            }
        }
        match self.features.get(["L_SilSp", "C_SilSp", "R_SilSp"][index]).and_then(Value::as_str) {
            Some("sil") => add(1)?, Some("sp") => add(2)?, Some("XX") if position != "C" => add(0)?, _ => {}
        }
        Ok(selected)
    }
    fn tone(&self, position: &str) -> Result<Option<i32>> {
        let raw = self.features.get(["L_Tone", "C_Tone", "R_Tone"][position_index(position)]).and_then(integer);
        if position == "L" && self.phone("L") == Some(0) {
            ensure!(raw.is_none_or(|value| value == -1), "an absent original left neighbor has L_Tone=-1");
            return Ok(Some(-1));
        }
        if position == "C" && self.language("C") == Some(3) {
            let stress = self.features.get("C_Stress").and_then(integer);
            ensure!(!matches!((raw, stress), (Some(a), Some(b)) if a != b), "English C_Tone and C_Stress share original node+0x6c and must agree");
            return Ok(raw.or(stress));
        }
        Ok(raw)
    }
    fn hierarchy_value(&self, spec: &'static Spec) -> Result<Option<i32>> {
        let direct = self.features.get(&spec.feature_key).map(|v| required_integer(v, &spec.feature_key)).transpose()?;
        let computed = if let Some(hierarchy) = &self.hierarchy {
            let key = (spec.family.as_str(), spec.argument1, spec.argument2);
            let cached = self.hierarchy_cache.borrow().get(&key).copied();
            Some(match cached { Some(value) => value, None => {
                let value = hierarchy.value(&spec.family, spec.argument1, spec.argument2)?;
                self.hierarchy_cache.borrow_mut().insert(key, value);
                value
            } })
        } else { None };
        ensure!(!matches!((direct, computed), (Some(a), Some(b)) if a != b), "explicit {} conflicts with the supplied original hierarchy", spec.feature_key);
        Ok(computed.or(direct))
    }
    fn numeric_feature(&self, question: &str) -> Result<Option<f64>> {
        let Some(spec) = self.profiles.questions.get(question) else { return Ok(None); };
        if spec.kind == Kind::Hierarchy && spec.comparison_mode == 4 { return Ok(self.hierarchy_value(spec)?.map(f64::from)); }
        if tone_sentinel(spec) { return Ok(self.tone(&spec.position)?.map(f64::from)); }
        Ok(None)
    }
}
fn tone_sentinel(spec: &Spec) -> bool { spec.kind == Kind::Tone && spec.comparison_mode == 4 && spec.argument1 == -1 && spec.argument2 == -1 }
fn numeric_sentinel(spec: &Spec) -> bool { tone_sentinel(spec) || (spec.kind == Kind::Hierarchy && spec.comparison_mode == 4) }
fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::PhoneId => "phone_id", Kind::PhoneGroup => "phone_group", Kind::Silence => "silence", Kind::MandarinVoiced => "mandarin_voiced", Kind::Language => "language", Kind::Tone => "tone", Kind::Stress => "stress", Kind::OnsetCoda => "onset_coda", Kind::PronunciationStyle => "pronunciation_style", Kind::Hierarchy => "hierarchy", Kind::PosLiteral => "pos_literal", Kind::PosGroup => "pos_group", Kind::PosAmbiguous => "pos_ambiguous",
    }
}
#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    Explicit, Shared, Category(&'a str), Original(&'static Spec), Fixed(&'static str),
    MissingHierarchy(&'static str), MissingTone(&'static str), MissingStress(&'static str), MissingPhone(&'static str),
    Numeric(&'a str, bool), Feature(&'a str, bool),
}
impl Source<'_> {
    pub fn diagnostic(self) -> bool { matches!(self, Self::Shared | Self::Numeric(_, true)) }
    pub fn text(self) -> String {
        match self {
            Self::Explicit => "explicit_question_answer".into(),
            Self::Shared => "shared_diagnostic_context_completion".into(),
            Self::Category(key) => format!("declared_category_nonmembership:{key}"),
            Self::Original(spec) => {
                let scope = match spec.kind { Kind::PosLiteral | Kind::PosGroup | Kind::PosAmbiguous => "explicit_original_POS", Kind::Hierarchy => "explicit_original_hierarchy", _ if spec.position == "C" => "current_context", _ => "explicit_resolved_neighbor_context" };
                format!("original_binary_{scope}:{}:rva={:x}", kind_name(spec.kind), spec.function_rva)
            }
            Self::Fixed(value) => value.into(),
            Self::MissingHierarchy(key) => format!("original_context_requires_explicit_or_resolved_hierarchy:{key}"),
            Self::MissingTone(position) => format!("original_context_requires_explicit_resolved_{position}_Tone"),
            Self::MissingStress(position) => format!("original_context_requires_explicit_resolved_{position}_Stress"),
            Self::MissingPhone(position) => format!("original_context_requires_explicit_{position}_phone"),
            Self::Numeric(key, diagnostic) => format!("{}_numeric_threshold_implication:{key}", if diagnostic { "diagnostic" } else { "explicit" }),
            Self::Feature(key, numeric) => format!("explicit_{}feature:{key}", if numeric { "numeric_" } else { "" }),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Answer<'a> { pub truth: Option<bool>, pub source: Source<'a> }
impl<'a> Answer<'a> {
    fn missing(source: Source<'a>) -> Self { Self { truth: None, source } }
    fn known(truth: bool, source: Source<'a>) -> Self { Self { truth: Some(truth), source } }
}
fn pos_value(spec: &Spec, facts: &Facts<'_>) -> Result<Option<i32>> {
    let Some(value) = facts.features.get("C_POS") else { return Ok(None); };
    let tag = value.as_str().context("C_POS requires an explicit original ASCII POS tag")?;
    ensure!((1..=63).contains(&tag.len()) && tag.is_ascii() && !tag.contains('\0'), "C_POS requires an explicit original ASCII POS tag of 1..63 non-NUL characters");
    Ok(match spec.kind {
        Kind::PosAmbiguous => None,
        Kind::PosLiteral => Some(if tag.len() == 1 { i32::from(tag.as_bytes()[0]) - 0x60 } else if tag.starts_with("ng") { if tag.as_bytes().get(2) == Some(&b'p') { 28 } else { 27 } } else if tag.starts_with("sg") { 29 } else { -1 }),
        Kind::PosGroup => {
            ensure!((1..=11).contains(&spec.argument1), "only the eleven recovered current-POS groups are supported");
            let members = facts.profiles.pos_groups.get(&spec.argument1).context("missing original POS group")?;
            Some(i32::from(members.iter().any(|member| tag.as_bytes()[0] == member.as_bytes()[0] && (tag.as_bytes().get(1) != Some(&b'n') || tag == member))))
        }
        _ => bail!("unsupported original POS predicate"),
    })
}
pub(super) fn original_answer(question: &str, facts: &Facts<'_>) -> Result<Option<Answer<'static>>> {
    let Some(spec) = facts.profiles.questions.get(question) else { return Ok(None); };
    let position = spec.position.as_str();
    let value = match spec.kind {
        Kind::PosLiteral | Kind::PosGroup | Kind::PosAmbiguous => {
            let Some(value) = pos_value(spec, facts)? else { return Ok(Some(Answer::missing(Source::Fixed(if spec.kind == Kind::PosAmbiguous { "original_POS_duplicate_registration_unresolved" } else { "original_context_requires_explicit_original_C_POS" })))); };
            value
        }
        Kind::Hierarchy => {
            let Some(value) = facts.hierarchy_value(spec)? else { return Ok(Some(Answer::missing(Source::MissingHierarchy(&spec.feature_key)))); };
            value
        }
        Kind::Tone => {
            let Some(tone) = facts.tone(position)? else { return Ok(Some(Answer::missing(Source::MissingTone(position)))); };
            if (spec.argument1 == -1 && spec.argument2 == -1) || (position == "L" && facts.phone("L") == Some(0)) { tone } else { i32::from(tone == spec.argument1 || tone == spec.argument2) }
        }
        Kind::PronunciationStyle => {
            let Some(value) = facts.features.get("ZC_PronStyle").and_then(integer) else { return Ok(Some(Answer::missing(Source::Fixed("original_context_requires_explicit_ZC_PronStyle")))); };
            value
        }
        Kind::Stress if position != "C" => {
            let Some(value) = facts.features.get(["L_Stress", "C_Stress", "R_Stress"][position_index(position)]).and_then(integer) else { return Ok(Some(Answer::missing(Source::MissingStress(position)))); };
            value
        }
        Kind::Language => {
            let Some(language) = facts.language(position) else { return Ok(Some(Answer::missing(Source::Fixed("original_context_requires_explicit_language_id")))); };
            i32::from(language == spec.argument1)
        }
        kind => {
            let Some(phone) = facts.phone(position) else { return Ok(Some(Answer::missing(Source::MissingPhone(position)))); };
            let row = facts.profiles.phones.get(&phone);
            let language = facts.language(position);
            match kind {
                Kind::PhoneId => phone,
                Kind::PhoneGroup => i32::from(row.is_some_and(|row| row.groups.contains(&spec.argument1))),
                Kind::Silence => i32::from(matches!(phone, 1 | 2) || (position != "C" && phone == 0)),
                Kind::MandarinVoiced => i32::from(language != Some(3) && row.is_some_and(|row| row.first_class == 1)),
                Kind::Stress => {
                    if matches!(phone, 1 | 2) { -1 } else if language != Some(3) { 3 } else {
                        let Some(value) = facts.tone("C")? else { return Ok(Some(Answer::missing(Source::Fixed("original_context_requires_explicit_C_Stress")))); };
                        value
                    }
                }
                Kind::OnsetCoda => {
                    let Some(language) = language else { return Ok(Some(Answer::missing(Source::Fixed("original_context_requires_explicit_language_id")))); };
                    if language != 3 { 2 } else {
                        let Some(value) = facts.features.get("C_OnsetCoda").and_then(integer) else { return Ok(Some(Answer::missing(Source::Fixed("original_context_requires_explicit_resolved_C_OnsetCoda")))); };
                        ensure!(matches!(value, 0 | 1), "original English C_OnsetCoda is 0 or 1 for valid nodes");
                        value
                    }
                }
                _ => bail!("unsupported original predicate kind"),
            }
        }
    };
    let truth = match spec.comparison_mode { 0 => value == spec.rhs, 4 => value != -1 && value <= spec.rhs, _ => bail!("unverified original comparison mode") };
    Ok(Some(Answer::known(truth, Source::Original(spec))))
}
fn predicate(question: &str) -> Option<(&str, &str, &str)> {
    let found = match (question.find("=="), question.find("<=")) { (Some(a), Some(b)) => a.min(b), (Some(a), None) | (None, Some(a)) => a, _ => return None };
    if found == 0 || found + 2 == question.len() { return None; }
    Some((&question[..found], &question[found..found + 2], &question[found + 2..]))
}
fn numeric_question(question: &str) -> Option<(&str, f64)> {
    let (key, threshold) = question.split_once("<=")?;
    if key.is_empty() || threshold.is_empty() { return None; }
    let threshold: f64 = threshold.trim().parse().ok()?;
    threshold.is_finite().then_some((key, threshold))
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Interval { lower: f64, upper: f64, lower_diagnostic: bool, upper_diagnostic: bool }
impl Default for Interval { fn default() -> Self { Self { lower: f64::NEG_INFINITY, upper: f64::INFINITY, lower_diagnostic: false, upper_diagnostic: false } } }
#[derive(Debug)]
pub(super) struct ConstraintError(pub String);
impl std::fmt::Display for ConstraintError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) } }
impl std::error::Error for ConstraintError {}
pub(super) struct Bounds { intervals: BTreeMap<String, Interval> }
pub(super) struct Undo<'a> { key: &'a str, previous: Option<Interval> }
impl Bounds {
    fn from_context(context: &ContextState, facts: &Facts<'_>) -> Result<Self> {
        let mut state = Self { intervals: BTreeMap::new() };
        for (diagnostic, answers) in [(false, &context.answers), (true, &context.diagnostic_answers)] {
            for (question, &truth) in answers {
                ensure!(!diagnostic || context.answers.get(question).is_none_or(|&known| known == truth), "conflicting explicit/diagnostic answer: {question}");
                if facts.profiles.questions.get(question).is_some_and(|s| matches!(s.kind, Kind::Hierarchy | Kind::PosLiteral | Kind::PosGroup | Kind::PosAmbiguous)) {
                    if let Some(Answer { truth: Some(known), .. }) = original_answer(question, facts)? { ensure!(known == truth, "{question}={truth} contradicts supplied original context facts"); }
                }
                state.apply(question, truth, diagnostic, facts)?;
            }
        }
        Ok(state)
    }
    pub fn apply<'q>(&mut self, question: &'q str, truth: bool, diagnostic: bool, facts: &Facts<'_>) -> Result<Option<Undo<'q>>> {
        let Some((key, threshold)) = numeric_question(question) else { return Ok(None); };
        let mut value = facts.features.get(key).and_then(finite_number);
        if let Some(recovered) = facts.numeric_feature(question)? { value = Some(recovered); }
        if value == Some(-1.0) && facts.profiles.questions.get(question).is_some_and(numeric_sentinel) {
            if truth { bail!(ConstraintError(format!("{question}=True contradicts the original numeric -1 sentinel"))); }
            return Ok(None);
        }
        if let Some(value) = value { if (value <= threshold) != truth { bail!(ConstraintError(format!("{question}={truth} contradicts explicit numeric feature {key}"))); } }
        let previous = self.intervals.get(key).copied();
        let mut next = previous.unwrap_or_default();
        if truth && threshold <= next.upper {
            next.upper_diagnostic = if threshold < next.upper { diagnostic } else { next.upper_diagnostic && diagnostic };
            next.upper = threshold;
        } else if !truth && threshold >= next.lower {
            next.lower_diagnostic = if threshold > next.lower { diagnostic } else { next.lower_diagnostic && diagnostic };
            next.lower = threshold;
        }
        if next.lower >= next.upper { bail!(ConstraintError(format!("inconsistent numeric context for {key}: requires x > {} and x <= {}", next.lower, next.upper))); }
        if previous == Some(next) { return Ok(None); }
        if let Some(value) = self.intervals.get_mut(key) { *value = next; } else { self.intervals.insert(key.to_owned(), next); }
        Ok(Some(Undo { key, previous }))
    }
    pub fn restore(&mut self, undo: Option<Undo<'_>>) {
        if let Some(undo) = undo {
            if let Some(previous) = undo.previous { *self.intervals.get_mut(undo.key).expect("live numeric bound") = previous; }
            else { self.intervals.remove(undo.key); }
        }
    }
    fn answer<'q>(&self, question: &'q str) -> Option<Answer<'q>> {
        let (key, threshold) = numeric_question(question)?;
        let interval = self.intervals.get(key)?;
        if interval.upper <= threshold { Some(Answer::known(true, Source::Numeric(key, interval.upper_diagnostic))) }
        else if interval.lower >= threshold { Some(Answer::known(false, Source::Numeric(key, interval.lower_diagnostic))) }
        else { None }
    }
    pub fn json(&self) -> Value {
        Value::Object(self.intervals.iter().map(|(key, bound)| (key.clone(), json!({
            "lower_exclusive": if bound.lower.is_finite() { Some(bound.lower) } else { None },
            "upper_inclusive": if bound.upper.is_finite() { Some(bound.upper) } else { None },
            "lower_from_diagnostic_answer": bound.lower_diagnostic, "upper_from_diagnostic_answer": bound.upper_diagnostic,
        }))).collect())
    }
}
#[derive(Clone, Deserialize)]
pub(super) struct ContextState {
    #[serde(default)] pub features: Map<String, Value>,
    #[serde(default)] pub answers: BTreeMap<String, bool>,
    #[serde(default = "default_label")] pub label: String,
    #[serde(default)] pub anchor_questions: Vec<String>,
    #[serde(default)] pub assumptions: Vec<String>,
    #[serde(default)] pub diagnostic_answers: BTreeMap<String, bool>,
}
fn default_label() -> String { "caller_supplied_context".into() }
impl ContextState {
    pub fn parse(value: &Value) -> Result<Self> {
        ensure!(value.is_object(), "context must be a JSON object");
        serde_json::from_value(value.clone()).context("invalid context fields or non-boolean question answers")
    }
    pub fn bounds(&self, facts: &Facts<'_>) -> Result<Bounds> { Bounds::from_context(self, facts) }
    pub fn answer<'q>(&self, question: &'q str, facts: &Facts<'_>, constraints: &Bounds) -> Result<Answer<'q>> {
        if let Some(&truth) = self.answers.get(question) { return Ok(Answer::known(truth, Source::Explicit)); }
        if let Some(&truth) = self.diagnostic_answers.get(question) { return Ok(Answer::known(truth, Source::Shared)); }
        let parsed = predicate(question);
        if let Some((key, "==", _)) = parsed {
            if matches!(key, "C_Nasal" | "C_Stop" | "C_Fric" | "C_Vowel" | "C_SM" | "C_SilSp") && self.answers.get(key) == Some(&false) { return Ok(Answer::known(false, Source::Category(key))); }
        }
        if let Some(recovered) = original_answer(question, facts)? {
            if recovered.truth.is_none() { if let Some(implied) = constraints.answer(question) { return Ok(implied); } }
            return Ok(recovered);
        }
        let Some((key, operator, rhs)) = parsed else { return Ok(Answer::missing(Source::Fixed("unknown_preserved"))); };
        let Some(lhs) = self.features.get(key) else { return Ok(constraints.answer(question).unwrap_or_else(|| Answer::missing(Source::Fixed("unknown_preserved")))); };
        if operator == "==" { return Ok(Answer::known(python_string(lhs) == rhs, Source::Feature(key, false))); }
        let rhs_number = rhs.trim().parse::<f64>().ok().filter(|value| value.is_finite());
        let (Some(lhs), Some(rhs)) = (finite_number(lhs), rhs_number) else { return Ok(constraints.answer(question).unwrap_or_else(|| Answer::missing(Source::Fixed("unknown_non_numeric_comparison")))); };
        Ok(Answer::known(lhs <= rhs, Source::Feature(key, true)))
    }
    pub fn json(&self, facts: &Facts<'_>) -> Result<Value> {
        Ok(json!({"label": self.label, "features": self.features, "answers": self.answers,
            "anchor_questions": self.anchor_questions, "assumptions": self.assumptions,
            "diagnostic_answers": self.diagnostic_answers, "numeric_constraints": self.bounds(facts)?.json(),
            "constraint_semantics": CONSTRAINT_SEMANTICS}))
    }
}
