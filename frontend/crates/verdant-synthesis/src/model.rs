//! Immutable production voice loading and original decision-tree selection.
//!
//! Raw .tkn/.bin pairs retain all little-endian F64 records. Modern directories
//! must pass the converter's source, checksum, topology and tensor contracts.
//! The shared source tables contain recovered predicates, never voice weights.
//! Selection copies only the twelve chosen records into the numerical API; it
//! does not clone a model or synthesize substitute leaf values. Unknown facts
//! remain unresolved under strict selection and are audited under diagnostics.
//!
//! select_phone accepts Context fields (features, answers, label,
//! anchor_questions, assumptions, diagnostic_answers), plus policy,
//! first_child_truth and selection_mode. prepare_score_segments supplies the
//! existing explicit/default vowel contexts, score neighbors and caller-owned
//! original hierarchy. Explicit continuation chains use physical source-phone
//! neighbors while retaining every note segment. No linguistic hierarchy or POS
//! is predicted here.
use std::{collections::BTreeMap, path::Path};
use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use crate::SelectedPhone;
mod context;
mod data;
mod loading;
mod score;
mod selection;
mod source_pins;
mod voice_package;
mod merged_bank;
mod current_package;
pub use voice_package::load_source_bank;
pub use current_package::{compiled_voice_bank_sha256, convert_voice_package, load_voice_package, load_voice_package_for_source_import,
    load_voice_package_with_manifest_pin, verify_voice_source, VoicePackage};
#[cfg(test)] mod tests;

pub use score::{compact_hierarchy_report, prepare_score_segments, resolve_segment_identity, validate_score_hierarchy};
/// Resolve the exact same original context facts used by decision-tree selection.
/// None is unresolved; callers must not confuse a declared but invalid identity
/// with an absent neighbor. Conflicting named, numeric and linked facts fail.
pub(crate) fn resolve_context_phones(features: &Value) -> Result<[Option<i32>; 3]> {
    let facts = context::Facts::new(features.as_object().context("original phone features must be an object")?)?;
    Ok([facts.phone("L"), facts.phone("C"), facts.phone("R")])
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeKind { PhoneDuration, StateDuration, Spectrum, Pitch }
impl TreeKind {
    fn parse(kind: &str) -> Result<Self> { Ok(match kind { "phone-dur-tree" => Self::PhoneDuration, "state-dur-tree" => Self::StateDuration, "spectrum-tree" => Self::Spectrum, "pitch-tree" => Self::Pitch, _ => bail!("unsupported original tree kind {kind}") }) }
    fn width(self) -> usize { match self { Self::PhoneDuration => 2, Self::StateDuration => 10, Self::Spectrum => 246, Self::Pitch => 9 } }
    fn name(self) -> &'static str { match self { Self::PhoneDuration => "phone-dur-tree", Self::StateDuration => "state-dur-tree", Self::Spectrum => "spectrum-tree", Self::Pitch => "pitch-tree" } }
}
#[derive(Clone, Copy, Debug)]
enum Node { Branch { question_id: i64, first: usize, second: usize }, Leaf(usize) }
#[derive(Debug)]
struct Tree {
    name: String,
    kind: TreeKind,
    records: Vec<f64>,
    nodes: Vec<Node>,
    parents: Vec<Option<(usize, bool)>>,
    leaf_nodes: Vec<usize>,
    begin: usize,
    end: usize,
}
impl Tree {
    fn record(&self, index: usize) -> &[f64] { let width = self.kind.width(); &self.records[index * width..(index + 1) * width] }
}
/// One admitted immutable model. Share this object by reference/Arc across requests.
/// Its tree/record storage is deliberately private and never cloned per phone.
#[derive(Debug)]
pub struct VoiceModel {
    pub sample_rate: u32,
    pub voice_attributes: Value,
    pub token_attributes: Value,
    pub provenance: Value,
    questions: BTreeMap<i64, String>,
    trees: BTreeMap<String, Tree>,
    tree_order: Vec<String>,
    pulse: Value,
    format: &'static str,
}
pub fn load_voice(path: &Path) -> Result<VoiceModel> { loading::load(path) }
/// Explicit legacy CLI pairs need not share a stem or reside in one directory.
pub fn load_raw_voice(token: &Path, model: &Path) -> Result<VoiceModel> { loading::load_raw(token, model) }
pub(crate) fn parse_bank_source(token: &[u8], model: &[u8]) -> Result<VoiceModel> {
    loading::parse_raw(token, model, Path::new("source.tkn"), Path::new("sn16k.bin"))
}
impl VoiceModel {
    pub fn select_phone(&self, context: &Value) -> Result<SelectedPhone> { selection::select(self, context) }
    /// A mixed raw pair cannot borrow the known token's historical identity.
    pub fn source_identity_confirmed(&self) -> bool {
        let source = &self.provenance;
        (source["token"]["sha256"].as_str() == Some(source_pins::KNOWN_TOKEN_SHA) && source["model"]["sha256"].as_str() == Some(source_pins::KNOWN_MODEL_SHA))
            || (source["source_sha256"]["sn16k.tkn"].as_str() == Some(source_pins::KNOWN_TOKEN_SHA) && source["source_sha256"]["sn16k.bin"].as_str() == Some(source_pins::KNOWN_MODEL_SHA))
    }
    /// Complete, read-only capability/identity metadata for worker and singer registries.
    pub fn metadata(&self) -> Value {
        let trees: Vec<Value> = self.tree_order.iter().map(|name| {
            let tree = &self.trees[name];
            json!({"name": name, "kind": tree.kind.name(), "leaf_count": tree.leaf_nodes.len(), "values_per_leaf": tree.kind.width(), "byte_range": [tree.begin, tree.end], "topology_nodes": tree.nodes.len()})
        }).collect();
        json!({"format": self.format, "sample_rate": self.sample_rate, "questions": self.questions.len(), "trees": trees,
            "render_ready": true, "source_identity_confirmed": self.source_identity_confirmed(),
            "voice_attributes": self.voice_attributes, "token_attributes": self.token_attributes, "pulse": self.pulse,
            "identity": self.provenance.get("identity"), "provenance": self.provenance,
            "context_source_tables": data::profiles().map(|p| &p.manifest).expect("profiles were admitted before model construction")})
    }
}
