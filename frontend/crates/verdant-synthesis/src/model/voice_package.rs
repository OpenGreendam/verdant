//! verdant-voice exact-bank and complete-package admission.
//!
//! Self-reported file hashes are insufficient: the complete F64 tensor regions
//! are rebuilt into the original binary, pinned to its source digest, and the
//! preserved original token is parsed again. All decisions, question names,
//! identities and inventory fields must agree with that parse. ONNX execution
//! and graph ABI checks are deliberately the responsibility of the caller.
use super::{
    Node, TreeKind, VoiceModel,
    data::digest,
    parse_bank_source,
    source_pins::{KNOWN_MODEL_SHA, KNOWN_TOKEN_SHA},
};
use anyhow::{Context, Result, ensure};
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::Read,
    path::Path,
};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum PeriodicFilterPhase { ZeroPhase, MinimumPhaseRealCepstrum }
impl PeriodicFilterPhase {
    /// Only absence selects the legacy behavior. A present null/wrong type is
    /// never a default, and a harmonic decoder cannot claim a filter phase.
    pub(crate) fn from_declaration(value:&Value,source_filter:bool)->Result<Self>{
        let phase=match value.get("periodic_filter_phase"){
            None=>Self::ZeroPhase,
            Some(Value::String(value))if value=="zero-phase-v1"=>Self::ZeroPhase,
            Some(Value::String(value))if value=="minimum-phase-real-cepstrum-v1"=>Self::MinimumPhaseRealCepstrum,
            _=>anyhow::bail!("Verdant ONNX periodic_filter_phase must be zero-phase-v1 or minimum-phase-real-cepstrum-v1"),
        };
        ensure!(source_filter||phase==Self::ZeroPhase,"minimum periodic_filter_phase is only supported by source-filter");
        Ok(phase)
    }
    pub(crate) fn name(self)->&'static str{match self{Self::ZeroPhase=>"zero-phase-v1",Self::MinimumPhaseRealCepstrum=>"minimum-phase-real-cepstrum-v1"}}
}

const MAX_JSON: usize = 8 * 1024 * 1024;
const MAX_TOKEN: usize = 2 * 1024 * 1024;
const MAX_BANK: usize = 64 * 1024 * 1024;
const MAX_ONNX: usize = 256 * 1024 * 1024;
const MAX_HEADER: usize = 1024 * 1024;
const SUPPORTED_CONTRACTS: &[u8] = include_bytes!("contracts/contracts.json");
const SOURCE_FILTER_CONTRACTS: &[u8] =
    include_bytes!("contracts/contracts-source-filter.json");

/// Exact source statistics and admitted immutable source-package provenance.
/// Source graph bytes are verified and released; execution uses the separately
/// pinned native runtime graphs. Integrity admission is not a perceptual test.
#[derive(Debug)]
pub(super) struct LegacyVoicePackage {
    pub voice: VoiceModel,
    pub manifest: Value,
    pub export: Value,
    pub contracts: Value,
    pub voice_version: String,
    pub runtime_manifest_sha256: String,
    /// Stable roles: voice_source_manifest, bank_json, bank_tensors, bank_topology,
    /// bank_token, source_controller, source_decoder, source_rights,
    /// source_export, source_contracts.
    pub hashes: BTreeMap<String, String>,
}

impl LegacyVoicePackage {
    pub fn source_manifest_sha256(&self) -> &str {
        &self.hashes["voice_source_manifest"]
    }
}

/// serde_json::Value normally accepts duplicate object names. This visitor
/// rejects them at every nesting level before interpreting any manifest.
struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                Number::from_f64(v).map(|n| UniqueValue(Value::Number(n))).ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some((key, UniqueValue(value))) = map.next_entry::<String, UniqueValue>()? {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn json_bytes(bytes: &[u8], label: &str) -> Result<Value> {
    ensure!(bytes.len() <= MAX_JSON, "{label}: JSON exceeds the size bound");
    let UniqueValue(value) = serde_json::from_slice(bytes).with_context(|| format!("invalid strict JSON: {label}"))?;
    Ok(value)
}
fn keys(value: &Value, expected: &[&str], label: &str) -> Result<()> {
    let object = value.as_object().with_context(|| format!("{label}: expected an object"))?;
    ensure!(
        object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key)),
        "{label}: unexpected or missing fields"
    );
    Ok(())
}
fn integer(value: &Value, minimum: usize, maximum: usize, label: &str) -> Result<usize> {
    let number = usize::try_from(value.as_u64().with_context(|| format!("{label}: expected a nonnegative integer"))?)?;
    ensure!((minimum..=maximum).contains(&number), "{label}: integer outside the bounded contract");
    Ok(number)
}
fn sha<'a>(value: &'a Value, label: &str) -> Result<&'a str> {
    let text = value.as_str().with_context(|| format!("{label}: expected SHA-256 string"))?;
    ensure!(
        text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{label}: expected lowercase SHA-256"
    );
    Ok(text)
}
fn bounded_file(directory: &Path, name: &str, maximum: usize) -> Result<Vec<u8>> {
    ensure!(
        !name.is_empty() && !matches!(name, "." | "..") && !name.contains(['/', '\\', '\0']),
        "artifact must have a fixed package-local filename"
    );
    let root = directory.canonicalize().context("Verdant ONNX package directory is unavailable")?;
    let path = root.join(name);
    let metadata = fs::symlink_metadata(&path).with_context(|| format!("missing Verdant ONNX artifact {name}"))?;
    ensure!(metadata.file_type().is_file() && !metadata.file_type().is_symlink(), "{name}: expected a regular file, not a symlink");
    ensure!(metadata.len() <= maximum as u64, "{name}: artifact exceeds the size bound");
    ensure!(path.canonicalize()?.parent() == Some(root.as_path()), "{name}: artifact escapes package directory");
    let mut bytes = Vec::new();
    fs::File::open(&path)?.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= maximum, "{name}: artifact exceeds the size bound");
    Ok(bytes)
}
fn artifact(directory: &Path, value: &Value, filename: &str, maximum: usize, abi: Option<&str>) -> Result<Vec<u8>> {
    let expected = if abi.is_some() { vec!["file", "sha256", "bytes", "abi"] } else { vec!["file", "sha256", "bytes"] };
    keys(value, &expected, filename)?;
    ensure!(value["file"].as_str() == Some(filename), "expected fixed Verdant ONNX artifact name {filename}");
    if let Some(abi) = abi {
        ensure!(value["abi"].as_str() == Some(abi), "{filename}: unsupported ABI identifier");
    }
    let size = integer(&value["bytes"], 1, maximum, filename)?;
    let expected_hash = sha(&value["sha256"], filename)?;
    let bytes = bounded_file(directory, filename, maximum)?;
    ensure!(bytes.len() == size && digest(&bytes) == expected_hash, "{filename}: size or SHA-256 mismatch");
    Ok(bytes)
}

struct Tensor<'a> {
    shape: [usize; 2],
    data: &'a [u8],
}
fn tensor_records(bytes: &[u8]) -> Result<BTreeMap<String, Tensor<'_>>> {
    ensure!(bytes.len() >= 8, "truncated exact-bank tensor header");
    let size = usize::try_from(u64::from_le_bytes(bytes[..8].try_into()?))?;
    ensure!((2..=MAX_HEADER).contains(&size) && size <= bytes.len() - 8, "bank tensor header length out of bounds");
    let header = json_bytes(&bytes[8..8 + size], "bank tensor header")?;
    let header = header.as_object().context("bank tensor header must be an object")?;
    ensure!((1..=128).contains(&header.len()), "invalid bank tensor inventory");
    let body = &bytes[8 + size..];
    let mut intervals = Vec::new();
    let mut tensors = BTreeMap::new();
    for (name, entry) in header {
        ensure!(
            name.strip_suffix("_raw")
                .is_some_and(|prefix| !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')),
            "only original raw tensors are allowed in the exact bank"
        );
        keys(entry, &["dtype", "shape", "data_offsets"], name)?;
        ensure!(entry["dtype"] == "F64", "exact bank must retain little-endian F64");
        let shape = entry["shape"].as_array().context("tensor shape must be an array")?;
        ensure!(shape.len() == 2, "original record matrix must have rank two");
        let shape = [integer(&shape[0], 1, MAX_BANK / 8, "tensor rows")?, integer(&shape[1], 1, MAX_BANK / 8, "tensor columns")?];
        let needed = shape[0].checked_mul(shape[1]).and_then(|n| n.checked_mul(8)).context("tensor size overflow")?;
        let offsets = entry["data_offsets"].as_array().context("tensor data offsets must be an array")?;
        ensure!(offsets.len() == 2, "expected two tensor offsets");
        let start = integer(&offsets[0], 0, body.len(), "tensor start")?;
        let end = integer(&offsets[1], 0, body.len(), "tensor end")?;
        ensure!(end >= start && end - start == needed, "tensor shape does not match payload length");
        intervals.push((start, end));
        tensors.insert(name.clone(), Tensor { shape, data: &body[start..end] });
    }
    intervals.sort_unstable();
    let mut cursor = 0;
    for (start, end) in intervals {
        ensure!(start == cursor && end > start, "overlapping or uncovered tensor bytes");
        cursor = end;
    }
    ensure!(cursor == body.len(), "uncovered trailing tensor bytes");
    Ok(tensors)
}

fn source_topology(voice: &VoiceModel, token_sha: &str) -> Value {
    let questions: BTreeMap<String, &String> = voice.questions.iter().map(|(id, text)| (id.to_string(), text)).collect();
    let trees: BTreeMap<String, Vec<i64>> = voice
        .tree_order
        .iter()
        .map(|name| {
            let tree = &voice.trees[name];
            let preorder = tree
                .nodes
                .iter()
                .map(|node| match *node {
                    Node::Branch { question_id, .. } => -question_id,
                    Node::Leaf(index) => (index * tree.kind.width() + 1) as i64,
                })
                .collect();
            (name.clone(), preorder)
        })
        .collect();
    json!({"format": "verdant-cfinal-topology", "format_version": 1,
        "source_token_sha256": token_sha, "branch_polarity": "first-false", "questions": questions, "trees": trees})
}
fn source_tree_entries(voice: &VoiceModel, original_binary: &[u8]) -> Value {
    Value::Array(
        voice
            .tree_order
            .iter()
            .map(|name| {
                let tree = &voice.trees[name];
                json!({"name": name, "kind": tree.kind.name(), "leaf_count": tree.leaf_nodes.len(),
            "values_per_leaf": tree.kind.width(), "byte_range": [tree.begin, tree.end],
            "raw_tensor": format!("{}_raw", name.replace('-', "_")),
            "raw_sha256": digest(&original_binary[tree.begin..tree.end])})
            })
            .collect(),
    )
}
fn source_inventory(voice: &VoiceModel) -> Value {
    json!({"questions": voice.questions.len(), "trees": voice.trees.len(),
        "leaves": voice.trees.values().map(|t| t.leaf_nodes.len()).sum::<usize>(),
        "internal_nodes": voice.trees.values().map(|t| t.nodes.iter().filter(|n| matches!(n, Node::Branch { .. })).count()).sum::<usize>(),
        "nodes": voice.trees.values().map(|t| t.nodes.len()).sum::<usize>(),
        "f64_values": voice.trees.values().map(|t| t.records.len()).sum::<usize>(),
        "pulse_entries": voice.pulse["entries"].as_u64().unwrap_or(0)})
}
#[derive(Debug)]
struct VerifiedBank {
    voice: VoiceModel,
    hashes: BTreeMap<String, String>,
}
fn verify_bank(directory: &Path, token_pin: &str, binary_pin: &str) -> Result<VerifiedBank> {
    let manifest_bytes = bounded_file(directory, "bank.json", MAX_JSON)?;
    let manifest = json_bytes(&manifest_bytes, "bank.json")?;
    keys(&manifest, &["format", "format_version", "source", "identity", "sample_rate", "artifacts", "trees", "inventory"], "bank.json")?;
    ensure!(
        manifest["format"] == "verdant-cfinal-bank" && manifest["format_version"].as_u64() == Some(1),
        "unsupported exact-bank format/version"
    );
    keys(&manifest["source"], &["token", "model"], "bank source")?;
    let token = artifact(directory, &manifest["source"]["token"], "source.tkn", MAX_TOKEN, None)?;
    keys(&manifest["source"]["model"], &["sha256", "bytes"], "source binary")?;
    let binary_size = integer(&manifest["source"]["model"]["bytes"], 8, MAX_BANK, "source binary bytes")?;
    let binary_hash = sha(&manifest["source"]["model"]["sha256"], "source binary")?;
    keys(&manifest["artifacts"], &["tensors", "topology"], "bank artifacts")?;
    let tensors_bytes = artifact(directory, &manifest["artifacts"]["tensors"], "bank.safetensors", MAX_BANK, None)?;
    let topology_bytes = artifact(directory, &manifest["artifacts"]["topology"], "topology.json", MAX_JSON, None)?;
    let topology = json_bytes(&topology_bytes, "topology.json")?;
    let tensors = tensor_records(&tensors_bytes)?;
    let trees = manifest["trees"].as_array().context("bank tree inventory must be an array")?;
    ensure!((1..=128).contains(&trees.len()), "invalid bank tree inventory");
    let mut original_binary = Vec::with_capacity(binary_size);
    let mut used = BTreeSet::new();
    for tree in trees {
        keys(tree, &["name", "kind", "leaf_count", "values_per_leaf", "byte_range", "raw_tensor", "raw_sha256"], "bank tree")?;
        let kind = TreeKind::parse(tree["kind"].as_str().context("tree kind must be a string")?)?;
        let width = kind.width();
        ensure!(tree["values_per_leaf"].as_u64() == Some(width as u64), "unsupported exact-bank leaf layout");
        let count = integer(&tree["leaf_count"], 1, MAX_BANK / (8 * width), "leaf count")?;
        let name = tree["raw_tensor"].as_str().context("raw tensor name must be a string")?;
        ensure!(used.insert(name), "duplicate raw leaf tensor");
        let tensor = tensors.get(name).context("missing raw leaf tensor")?;
        let cursor = original_binary.len();
        let end = cursor.checked_add(count * width * 8).context("binary size overflow")?;
        ensure!(
            end <= binary_size && tensor.shape == [count, width] && tree["byte_range"] == json!([cursor, end]),
            "original binary regions must be contiguous, ordered and exactly shaped"
        );
        ensure!(digest(tensor.data) == sha(&tree["raw_sha256"], "raw leaf region")?, "raw leaf region SHA-256 mismatch");
        original_binary.extend_from_slice(tensor.data);
    }
    ensure!(used.len() == tensors.len(), "extra tensors outside original tree inventory");
    let token_hash = digest(&token);
    let actual_binary_hash = digest(&original_binary);
    ensure!(
        original_binary.len() == binary_size && actual_binary_hash == binary_hash,
        "reconstructed original binary does not match source claim"
    );
    ensure!(token_hash == token_pin && actual_binary_hash == binary_pin, "actual source bytes do not match audited sn16k fingerprints");
    let mut voice = parse_bank_source(&token, &original_binary)?;
    ensure!(topology == source_topology(&voice, &token_hash), "topology/questions differ from preserved original token");
    ensure!(manifest["trees"] == source_tree_entries(&voice, &original_binary), "tree metadata differ from original source");
    ensure!(
        manifest["identity"] == json!({"voice_attributes": voice.voice_attributes, "token_attributes": voice.token_attributes}),
        "identity differs from original source"
    );
    ensure!(manifest["inventory"] == source_inventory(&voice), "inventory differs from actual source");
    ensure!(manifest["sample_rate"].as_u64() == Some(16000) && voice.sample_rate == 16000, "unsupported source sample rate");
    let hashes = BTreeMap::from([
        ("bank_json".into(), digest(&manifest_bytes)),
        ("bank_tensors".into(), digest(&tensors_bytes)),
        ("bank_topology".into(), digest(&topology_bytes)),
        ("bank_token".into(), token_hash.clone()),
    ]);
    voice.format = "verdant-cfinal-bank";
    voice.provenance = json!({"kind": "cfinal-exact-bank", "exact_leaf_bank_used": true,
        "leaf_table_selection": true, "requires_original_bin_at_runtime": false,
        "source_sha256": {"sn16k.tkn": token_hash, "sn16k.bin": actual_binary_hash},
        "bank_manifest_sha256": hashes["bank_json"], "exact_binary_reconstruction": true,
        "all_tree_topology_preserved": true, "source_tkn_bytes_preserved": true,
        "voice_attributes": voice.voice_attributes, "token_attributes": voice.token_attributes,
        "original_context_reachability_proven": false, "neural_generalization_or_quality_proven": false});
    Ok(VerifiedBank { voice, hashes })
}

/// Verify and load every original record/topology without the original .bin.
pub fn load_source_bank(directory: &Path) -> Result<VoiceModel> {
    Ok(verify_bank(directory, KNOWN_TOKEN_SHA, KNOWN_MODEL_SHA)?.voice)
}

fn utc_timestamp(text: &str) -> bool {
    let Some(base) = text.strip_suffix('Z') else {
        return false;
    };
    let (date, fraction) = base.split_once('.').map_or((base, None), |(a, b)| (a, Some(b)));
    if date.len() != 19
        || !date.is_ascii()
        || &date[4..5] != "-"
        || &date[7..8] != "-"
        || &date[10..11] != "T"
        || &date[13..14] != ":"
        || &date[16..17] != ":"
    {
        return false;
    }
    if fraction.is_some_and(|f| !(1..=6).contains(&f.len()) || !f.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let parts: Option<Vec<u32>> = [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
        .into_iter()
        .map(|(a, b)| {
            let s = &date[a..b];
            if s.bytes().all(|v| v.is_ascii_digit()) { s.parse().ok() } else { None }
        })
        .collect();
    let Some(p) = parts else {
        return false;
    };
    let (year, month, day, hour, minute, second) = (p[0], p[1], p[2], p[3], p[4], p[5]);
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    (1..=days[(month - 1) as usize]).contains(&day)
}
fn rights_attestation(value: &Value, binding: &Value) -> Result<()> {
    keys(value, &["format", "format_version", "source_binding", "attested_by", "attested_at_utc", "statement"], "rights.json")?;
    ensure!(
        value["format"] == "verdant-cfinal-rights-attestation" && value["format_version"].as_u64() == Some(1),
        "unsupported rights-attestation format/version"
    );
    ensure!(&value["source_binding"] == binding, "rights statement is bound to different source bytes");
    for (name, maximum) in [("attested_by", 256), ("statement", 16384)] {
        let text = value[name].as_str().context("rights attestation requires explicit text")?;
        ensure!(!text.trim().is_empty() && !text.contains('\0') && text.chars().count() <= maximum, "rights {name}: invalid bounded text");
    }
    ensure!(value["attested_at_utc"].as_str().is_some_and(utc_timestamp), "rights attestation requires a valid ISO UTC timestamp");
    Ok(())
}
fn candidate_abis(value: &Value) -> Result<(&'static str, &'static str)> {
    match value["decoder_candidate"].as_str() {
        Some("harmonic") => Ok(("cfinal-controller-v1", "cfinal-decoder-v1")),
        Some("source-filter") => Ok(("cfinal-source-filter-controller-v1", "cfinal-source-filter-decoder-v1")),
        _ => anyhow::bail!("unsupported Verdant ONNX decoder candidate"),
    }
}
fn export_training_clock(value: &Value) -> Result<&'static str> {
    // Training provenance only. Do not alter graph contracts or legacy bytes.
    match value.get("training_clock") {
        None => Ok("padded-batch-v1"),
        Some(Value::String(clock)) if clock == "padded-batch-v1" => Ok("padded-batch-v1"),
        Some(Value::String(clock)) if clock == "per-clip-v2" => Ok("per-clip-v2"),
        _ => anyhow::bail!("export training_clock must be padded-batch-v1 or per-clip-v2"),
    }
}
fn export_training_noise_policy(value: &Value) -> Result<&'static str> {
    match value.get("training_noise_policy") {
        None => Ok("overlapping-counter-windows-v1"),
        Some(Value::String(policy)) if policy == "overlapping-counter-windows-v1" => Ok("overlapping-counter-windows-v1"),
        Some(Value::String(policy)) if policy == "disjoint-counter-windows-v2" => Ok("disjoint-counter-windows-v2"),
        _ => anyhow::bail!("export training_noise_policy must be overlapping-counter-windows-v1 or disjoint-counter-windows-v2"),
    }
}
fn export_contract(
    value: &Value,
    contracts: &Value,
    controller: &[u8],
    decoder: &[u8],
    contracts_bytes: &[u8],
    training: &Value,
    binding: &Value,
) -> Result<()> {
    let (controller_abi, decoder_abi) = candidate_abis(value)?;
    let source_filter = value["decoder_candidate"] == "source-filter";
    let phase = PeriodicFilterPhase::from_declaration(contracts, source_filter)?;
    ensure!(PeriodicFilterPhase::from_declaration(value, source_filter)? == phase, "export/contract periodic_filter_phase mismatch");
    // Strictly validate this optional semantic field before removing it from
    // the frozen tensor-contract comparison. Every other field remains exact.
    let mut tensor_contracts = contracts.clone();
    tensor_contracts.as_object_mut().context("Verdant ONNX contracts must be an object")?.remove("periodic_filter_phase");
    let supported = json_bytes(if source_filter { SOURCE_FILTER_CONTRACTS } else { SUPPORTED_CONTRACTS }, "supported Verdant ONNX contracts")?;
    ensure!(supported["sample_interpolation"] == "integer-sample-grid-v1", "compiled Verdant ONNX contracts lack the reviewed sample grid");
    let mut historical = supported.clone();
    historical.as_object_mut().context("supported contracts must be an object")?.remove("sample_interpolation");
    let mut resolved = supported.clone();
    resolved["neighbor_encoding"] = json!("resolved-context-phone-v2");
    ensure!(
        tensor_contracts == supported || tensor_contracts == resolved || (!source_filter && tensor_contracts == historical),
        "contracts.json differs from supported frozen Verdant ONNX tensor semantics"
    );
    let integer_grid = contracts.get("sample_interpolation").is_some();
    let resolved_neighbors = contracts.get("neighbor_encoding").and_then(Value::as_str) == Some("resolved-context-phone-v2");
    let mut export_keys = vec![
        "schema",
        "controller_abi",
        "decoder_abi",
        "sample_rate",
        "hop_samples",
        "decoder_candidate",
        "trained_steps",
        "plan_sha256",
        "corpus_sha256",
        "checkpoint_sha256",
        "quality_admission",
        "source_bank",
        "source_code",
        "files",
        "graphs",
        "torch_version",
        "onnx_version",
        "onnxruntime_version",
        "opset",
        "requires_native_statistics_for_conditions",
        "requires_reference_waveform_at_inference",
    ];
    if integer_grid {
        export_keys.push("numerical_patch");
    }
    if resolved_neighbors {
        export_keys.push("neighbor_encoding");
    }
    if value.get("training_clock").is_some() {
        export_keys.push("training_clock");
    }
    if value.get("training_noise_policy").is_some() {
        export_keys.push("training_noise_policy");
    }
    if value.get("periodic_filter_phase").is_some() {
        export_keys.push("periodic_filter_phase");
    }
    keys(value, &export_keys, "export.json")?;
    export_training_clock(value)?;
    export_training_noise_policy(value)?;
    ensure!(!resolved_neighbors || value["neighbor_encoding"] == contracts["neighbor_encoding"], "export/contract neighbor encoding mismatch");
    ensure!(
        value["schema"] == "cfinal-onnx-export/1" && value["controller_abi"] == controller_abi && value["decoder_abi"] == decoder_abi,
        "unsupported Verdant ONNX exporter schema, ABI or decoder candidate"
    );
    ensure!(
        value["sample_rate"].as_u64() == Some(16000) && value["hop_samples"].as_u64() == Some(80) && value["opset"].as_u64() == Some(17),
        "export.json has an unsupported audio clock or ONNX opset"
    );
    integer(&value["trained_steps"], 1, 1_000_000_000, "trained_steps")?;
    ensure!(value["quality_admission"].is_boolean(), "export quality_admission must be a boolean declaration");
    ensure!(
        value["requires_native_statistics_for_conditions"] == true && value["requires_reference_waveform_at_inference"] == false,
        "export inference requirements contradict Verdant ONNX contract"
    );
    for key in ["plan_sha256", "corpus_sha256", "checkpoint_sha256"] {
        sha(&value[key], key)?;
    }
    ensure!(
        value["plan_sha256"] == training["manifest_sha256"] && value["corpus_sha256"] == training["dataset_sha256"],
        "export training plan or corpus differs from source-manifest training binding"
    );
    ensure!(&value["source_bank"] == binding, "export controller is bound to different original source bank bytes");
    ensure!(
        value["files"]
            == json!({"controller.onnx": digest(controller), "decoder.onnx": digest(decoder), "contracts.json": digest(contracts_bytes)}),
        "export file hashes differ from actual graphs or contracts.json"
    );
    keys(&value["graphs"], &["controller.onnx", "decoder.onnx"], "export graph inventory")?;
    for (name, bytes) in [("controller.onnx", controller), ("decoder.onnx", decoder)] {
        let graph = &value["graphs"][name];
        keys(graph, &["operators", "nodes", "bytes"], name)?;
        integer(&graph["nodes"], 1, 10_000_000, "export graph nodes")?;
        ensure!(graph["bytes"].as_u64() == Some(bytes.len() as u64), "export graph byte count differs from actual graph");
        let operators = graph["operators"].as_array().context("export graph operators must be an array")?;
        ensure!((1..=256).contains(&operators.len()), "invalid export graph operator count");
        let mut unique = BTreeSet::new();
        for operator in operators {
            let text = operator.as_str().context("operator must be a name")?;
            ensure!(
                (1..=128).contains(&text.len())
                    && text.as_bytes()[0].is_ascii_alphabetic()
                    && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && unique.insert(text),
                "export graph operators must be a bounded unique name list"
            );
        }
    }
    let sources = value["source_code"].as_object().context("source-code digest inventory must be an object")?;
    ensure!(
        (1..=128).contains(&sources.len())
            && ["export.py", "train.py", "model.py", "runtime.py", "conditions.py", "prepare.py", "noise.py", "__init__.py"]
                .iter()
                .all(|name| sources.contains_key(*name)),
        "export source-code digest inventory is incomplete"
    );
    ensure!(
        !resolved_neighbors || ["planner.py", "neighbor.py", "abi.py"].iter().all(|name| sources.contains_key(*name)),
        "resolved-neighbor export source-code digest inventory is incomplete"
    );
    ensure!(phase != PeriodicFilterPhase::MinimumPhaseRealCepstrum || sources.contains_key("filter_phase.py"),
        "minimum periodic_filter_phase requires filter_phase.py in the export source-code digest inventory");
    for (name, checksum) in sources {
        ensure!(
            name.strip_suffix(".py").is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')),
            "export source-code names must be package-local Python filenames"
        );
        sha(checksum, name)?;
    }
    if integer_grid {
        let patch = &value["numerical_patch"];
        keys(
            patch,
            &["id", "frozen_training_code_changed", "training_decoder_source_sha256", "export_decoder_source_sha256"],
            "numerical patch",
        )?;
        ensure!(
            patch["id"] == "integer-sample-grid-v1" && patch["frozen_training_code_changed"] == false,
            "numerical patch must identify the separately versioned integer sample grid without rewriting frozen training code"
        );
        sha(&patch["training_decoder_source_sha256"], "training decoder source")?;
        sha(&patch["export_decoder_source_sha256"], "export decoder source")?;
        ensure!(
            patch["training_decoder_source_sha256"] == value["source_code"]["model.py"]
                && patch["export_decoder_source_sha256"] == value["source_code"]["inference_model.py"],
            "numerical patch source hashes differ from the bound training or inference decoder"
        );
    }
    for key in ["torch_version", "onnx_version", "onnxruntime_version"] {
        let text = value[key].as_str().context("export tool versions must be strings")?;
        ensure!(
            !text.trim().is_empty() && text.chars().count() <= 128 && !text.contains('\0'),
            "export tool versions must be bounded nonempty strings"
        );
    }
    Ok(())
}
pub(super) struct LegacySourcePackage {
    pub voice: VoiceModel,
    pub manifest: Value,
    pub hashes: BTreeMap<String, String>,
}

fn load_source_package_with_pins(directory: &Path, manifest_name: &str, token_pin: &str, binary_pin: &str)
    -> Result<(LegacySourcePackage, Value, Value)>
{
    let bytes = bounded_file(directory, manifest_name, MAX_JSON)?;
    let manifest = json_bytes(&bytes, manifest_name)?;
    keys(
        &manifest,
        &["format", "format_version", "kind", "bank", "audio", "models", "artifacts", "source_binding", "rights", "training"],
        manifest_name,
    )?;
    ensure!(
        manifest["format"] == "verdant-cfinal-voicebank"
            && manifest["format_version"].as_u64() == Some(1)
            && manifest["kind"] == "cfinal-ddsp-onnx",
        "unsupported Verdant ONNX package format/version/kind"
    );
    artifact(directory, &manifest["bank"], "bank.json", MAX_JSON, None)?;
    let VerifiedBank { mut voice, mut hashes } = verify_bank(directory, token_pin, binary_pin)?;
    ensure!(manifest["audio"] == json!({"sample_rate": 16000, "hop": 80}), "unsupported Verdant ONNX audio clock");
    let binding = json!({"token_sha256": token_pin, "model_sha256": binary_pin});
    ensure!(manifest["source_binding"] == binding, "Verdant ONNX ONNX declaration refers to a different source bank");
    keys(&manifest["artifacts"], &["export", "contracts"], "Verdant ONNX evidence artifacts")?;
    let export_bytes = artifact(directory, &manifest["artifacts"]["export"], "export.json", MAX_JSON, None)?;
    let contracts_bytes = artifact(directory, &manifest["artifacts"]["contracts"], "contracts.json", MAX_JSON, None)?;
    let export = json_bytes(&export_bytes, "export.json")?;
    let contracts = json_bytes(&contracts_bytes, "contracts.json")?;
    let (controller_abi, decoder_abi) = candidate_abis(&export)?;
    keys(&manifest["models"], &["controller", "decoder"], "ONNX models")?;
    let controller = artifact(directory, &manifest["models"]["controller"], "controller.onnx", MAX_ONNX, Some(controller_abi))?;
    let decoder = artifact(directory, &manifest["models"]["decoder"], "decoder.onnx", MAX_ONNX, Some(decoder_abi))?;
    let rights_bytes = artifact(directory, &manifest["rights"], "rights.json", MAX_JSON, None)?;
    rights_attestation(&json_bytes(&rights_bytes, "rights.json")?, &binding)?;
    keys(&manifest["training"], &["manifest_sha256", "dataset_sha256", "recipe_id"], "training provenance")?;
    sha(&manifest["training"]["manifest_sha256"], "training manifest")?;
    sha(&manifest["training"]["dataset_sha256"], "training dataset")?;
    let recipe = manifest["training"]["recipe_id"].as_str().context("training recipe_id must be a string")?;
    ensure!(!recipe.is_empty() && recipe.chars().count() <= 128 && !recipe.contains('\0'), "invalid training recipe_id");
    export_contract(&export, &contracts, &controller, &decoder, &contracts_bytes, &manifest["training"], &binding)?;
    hashes.extend([
        ("voice_source_manifest".into(), digest(&bytes)),
        ("source_controller".into(), digest(&controller)),
        ("source_decoder".into(), digest(&decoder)),
        ("source_rights".into(), digest(&rights_bytes)),
        ("source_export".into(), digest(&export_bytes)),
        ("source_contracts".into(), digest(&contracts_bytes)),
    ]);
    voice.format = "verdant-voice";
    voice.provenance["kind"] = json!("verdant-voice-source-package");
    voice.provenance["source_manifest_sha256"] = json!(hashes["voice_source_manifest"]);
    voice.provenance["artifact_hashes"] = json!(hashes);
    voice.provenance["rights_attestation_present"] = json!(true);
    voice.provenance["rights_attestation_is_legal_determination"] = json!(false);
    voice.provenance["onnx_artifacts_verified"] = json!(true);
    voice.provenance["onnx_graph_abi_verified"] = json!(false);
    voice.provenance["training_history_independently_verified"] = json!(false);
    voice.provenance["decoder_candidate"] = export["decoder_candidate"].clone();
    voice.provenance["neighbor_encoding"] =
        contracts.get("neighbor_encoding").cloned().unwrap_or_else(|| json!("legacy-explicit-phone-v1"));
    voice.provenance["declared_sample_interpolation"] =
        contracts.get("sample_interpolation").cloned().unwrap_or_else(|| json!("historical-float32-position"));
    let phase = PeriodicFilterPhase::from_declaration(&contracts, export["decoder_candidate"] == "source-filter")?.name();
    voice.provenance["periodic_filter_phase"] = json!(phase);
    voice.provenance["export_provenance"] = json!({"trained_steps": export["trained_steps"], "checkpoint_sha256": export["checkpoint_sha256"], "quality_admission": export["quality_admission"], "training_clock": export_training_clock(&export)?, "training_noise_policy": export_training_noise_policy(&export)?, "periodic_filter_phase":phase});
    voice.provenance["quality_admission_independently_verified"] = json!(false);
    Ok((LegacySourcePackage { voice, manifest, hashes }, export, contracts))
}

fn load_package_with_pins(directory: &Path, token_pin: &str, binary_pin: &str) -> Result<LegacyVoicePackage> {
    let (LegacySourcePackage { voice, manifest, hashes }, export, contracts) =
        load_source_package_with_pins(directory, "voice-source.json", token_pin, binary_pin)?;
    let (runtime_manifest_sha256, voice_version) = crate::voice::manifest(&directory.join("verdant-voice"), &hashes["voice_source_manifest"])?;
    Ok(LegacyVoicePackage { voice, manifest, export, contracts, voice_version, runtime_manifest_sha256, hashes })
}

/// Explicit migration admission. Every legacy source check still runs; the
/// old extension layout is independently pinned by the converter afterward.
pub(super) fn load_legacy_source_package(directory: &Path) -> Result<LegacySourcePackage> {
    let canonical = directory.join("voice-source.json").try_exists()?;
    let historical = directory.join("cfinal.json").try_exists()?;
    ensure!(canonical != historical, "migration requires exactly one historical source manifest");
    let name = if canonical { "voice-source.json" } else { "cfinal.json" };
    Ok(load_source_package_with_pins(directory, name, KNOWN_TOKEN_SHA, KNOWN_MODEL_SHA)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Fixture {
        directory: PathBuf,
        token: Vec<u8>,
        binary: Vec<u8>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }
    fn write_json(path: &Path, value: &Value) {
        let bytes = serde_json::to_vec_pretty(value).unwrap();
        fs::write(path, &bytes).unwrap();
        if path.file_name().is_some_and(|name|name == "voice-source.json") {
            let runtime = path.parent().unwrap().join("verdant-voice");
            fs::create_dir_all(&runtime).unwrap();
            let declaration = json!({"schema":"verdant-voice-runtime/1","name":"verdant-voice",
                "version":env!("CARGO_PKG_VERSION"),"strategy":crate::voice::STRATEGY,
                "source_manifest_sha256":digest(&bytes)});
            fs::write(runtime.join("manifest.json"),serde_json::to_vec(&declaration).unwrap()).unwrap();
        }
    }
    fn descriptor(name: &str, bytes: &[u8]) -> Value {
        json!({"file": name, "sha256": digest(bytes), "bytes": bytes.len()})
    }
    fn fixture() -> Fixture {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory =
            std::env::temp_dir().join(format!("verdant-cfinal-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&directory).unwrap();
        let mut kinds =
            vec![("phone-dur-tree".to_owned(), TreeKind::PhoneDuration), ("state-dur-tree".to_owned(), TreeKind::StateDuration)];
        for i in 0..5 {
            kinds.push((format!("cep{i}"), TreeKind::Spectrum));
        }
        for i in 0..5 {
            kinds.push((format!("pitch{i}"), TreeKind::Pitch));
        }
        let (mut binary, mut cdata, mut items) = (Vec::new(), String::new(), String::new());
        for (name, kind) in kinds {
            let width = kind.width();
            let start = cdata.len();
            let begin = binary.len();
            cdata.push_str(&format!("-1 1 {}\n", width + 1));
            for i in 0..width * 2 {
                let v = if i == 0 {
                    -0.0f64
                } else if i == width * 2 - 1 {
                    1e-250
                } else {
                    i as f64 / 7.0
                };
                binary.extend(v.to_le_bytes());
            }
            items.push_str(&format!(
                "<{} name=\"{name}\" mbegin=\"{begin}\" mend=\"{}\">{start}-{}</{}>",
                kind.name(),
                binary.len(),
                cdata.len() - 1,
                kind.name()
            ));
        }
        let start = cdata.len();
        cdata.push_str("0.125 0.500 0.750 1.000\n");
        items.push_str(&format!("<pulse>{start}-{}</pulse>", cdata.len() - 1));
        let token = format!("<voices><voice name=\"synthetic\" vid=\"0\"><token sample-rate=\"16000\" lsf_coef_order=\"24\" lsf_diff_order=\"16\"><qstable><queset name=\"unrecovered\">1</queset></qstable>{items}</token></voice><![CDATA[{cdata}]]></voices>").into_bytes();
        let voice = parse_bank_source(&token, &binary).unwrap();
        let mut header = Map::new();
        let mut body: Vec<u8> = Vec::new();
        // Deliberately store tensors in a different order from the binary.
        for (name, tree) in &voice.trees {
            let a = body.len();
            body.extend(&binary[tree.begin..tree.end]);
            header.insert(
                format!("{}_raw", name.replace('-', "_")),
                json!({"dtype": "F64", "shape": [tree.leaf_nodes.len(), tree.kind.width()], "data_offsets": [a, body.len()]}),
            );
        }
        let mut hb = serde_json::to_vec(&header).unwrap();
        while hb.len() % 8 != 0 {
            hb.push(b' ');
        }
        let mut tensors = (hb.len() as u64).to_le_bytes().to_vec();
        tensors.extend(hb);
        tensors.extend(body);
        let topo = serde_json::to_vec_pretty(&source_topology(&voice, &digest(&token))).unwrap();
        fs::write(directory.join("source.tkn"), &token).unwrap();
        fs::write(directory.join("bank.safetensors"), &tensors).unwrap();
        fs::write(directory.join("topology.json"), &topo).unwrap();
        let manifest = json!({"format": "verdant-cfinal-bank", "format_version": 1,
            "source": {"token": descriptor("source.tkn", &token), "model": {"sha256": digest(&binary), "bytes": binary.len()}},
            "identity": {"voice_attributes": voice.voice_attributes, "token_attributes": voice.token_attributes}, "sample_rate": 16000,
            "artifacts": {"tensors": descriptor("bank.safetensors", &tensors), "topology": descriptor("topology.json", &topo)},
            "trees": source_tree_entries(&voice, &binary), "inventory": source_inventory(&voice)});
        write_json(&directory.join("bank.json"), &manifest);
        Fixture { directory, token, binary }
    }
    fn verify(f: &Fixture) -> Result<VerifiedBank> {
        verify_bank(&f.directory, &digest(&f.token), &digest(&f.binary))
    }
    fn json_file(f: &Fixture, name: &str) -> Value {
        serde_json::from_slice(&fs::read(f.directory.join(name)).unwrap()).unwrap()
    }
    fn resign(f: &Fixture, role: &str, filename: &str) {
        let mut m = json_file(f, "bank.json");
        m["artifacts"][role] = descriptor(filename, &fs::read(f.directory.join(filename)).unwrap());
        write_json(&f.directory.join("bank.json"), &m);
    }
    fn final_package(f: &Fixture) {
        final_package_candidate(f, "harmonic");
    }
    fn final_package_candidate(f: &Fixture, candidate: &str) {
        let (controller_abi, decoder_abi) = candidate_abis(&json!({"decoder_candidate": candidate})).unwrap();
        let binding = json!({"token_sha256": digest(&f.token), "model_sha256": digest(&f.binary)});
        let rights = json!({"format": "verdant-cfinal-rights-attestation", "format_version": 1, "source_binding": binding,
            "attested_by": "synthetic test fixture", "attested_at_utc": "2026-11-17T00:00:00Z", "statement": "Test data only; not a legal attestation for real source assets."});
        write_json(&f.directory.join("rights.json"), &rights);
        fs::write(f.directory.join("controller.onnx"), b"synthetic artifact; not executable ONNX").unwrap();
        fs::write(f.directory.join("decoder.onnx"), b"synthetic second artifact; ABI unverified").unwrap();
        fs::write(
            f.directory.join("contracts.json"),
            if candidate == "source-filter" { SOURCE_FILTER_CONTRACTS } else { SUPPORTED_CONTRACTS },
        )
        .unwrap();
        let files: BTreeMap<&str, String> = ["controller.onnx", "decoder.onnx", "contracts.json"]
            .into_iter()
            .map(|name| (name, digest(&fs::read(f.directory.join(name)).unwrap())))
            .collect();
        let graphs: BTreeMap<&str, Value> = ["controller.onnx", "decoder.onnx"]
            .into_iter()
            .map(|name| (name, json!({"operators": ["Add"], "nodes": 1, "bytes": fs::metadata(f.directory.join(name)).unwrap().len()})))
            .collect();
        let sources: BTreeMap<&str, String> = [
            "export.py",
            "train.py",
            "model.py",
            "inference_model.py",
            "runtime.py",
            "conditions.py",
            "prepare.py",
            "noise.py",
            "__init__.py",
        ]
        .into_iter()
        .map(|name| (name, "4".repeat(64)))
        .collect();
        let export = json!({"schema": "cfinal-onnx-export/1", "controller_abi": controller_abi, "decoder_abi": decoder_abi,
            "sample_rate": 16000, "hop_samples": 80, "decoder_candidate": candidate, "trained_steps": 8,
            "plan_sha256": "1".repeat(64), "corpus_sha256": "2".repeat(64), "checkpoint_sha256": "3".repeat(64), "quality_admission": false,
            "source_bank": binding,
            "numerical_patch": {"id": "integer-sample-grid-v1", "frozen_training_code_changed": false,
                "training_decoder_source_sha256": "4".repeat(64), "export_decoder_source_sha256": "4".repeat(64)},
            "source_code": sources, "files": files, "graphs": graphs,
            "torch_version": "synthetic-test", "onnx_version": "synthetic-test", "onnxruntime_version": "synthetic-test", "opset": 17,
            "requires_native_statistics_for_conditions": true, "requires_reference_waveform_at_inference": false});
        write_json(&f.directory.join("export.json"), &export);
        let desc = |name: &str| descriptor(name, &fs::read(f.directory.join(name)).unwrap());
        let mut c = desc("controller.onnx");
        c["abi"] = json!(controller_abi);
        let mut d = desc("decoder.onnx");
        d["abi"] = json!(decoder_abi);
        let manifest = json!({"format": "verdant-cfinal-voicebank", "format_version": 1, "kind": "cfinal-ddsp-onnx",
            "bank": desc("bank.json"), "audio": {"sample_rate": 16000, "hop": 80}, "models": {"controller": c, "decoder": d},
            "artifacts": {"export": desc("export.json"), "contracts": desc("contracts.json")},
            "source_binding": binding, "rights": desc("rights.json"),
            "training": {"manifest_sha256": "1".repeat(64), "dataset_sha256": "2".repeat(64), "recipe_id": "synthetic-fixture-only"}});
        write_json(&f.directory.join("voice-source.json"), &manifest);
    }
    fn rewrite_package_semantics(f: &Fixture, contracts: &Value, export: &mut Value) {
        write_json(&f.directory.join("contracts.json"), contracts);
        let bytes = fs::read(f.directory.join("contracts.json")).unwrap();
        export["files"]["contracts.json"] = json!(digest(&bytes));
        write_json(&f.directory.join("export.json"), export);
        let mut manifest = json_file(f, "voice-source.json");
        manifest["artifacts"]["contracts"] = descriptor("contracts.json", &bytes);
        manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
        write_json(&f.directory.join("voice-source.json"), &manifest);
    }
    #[test]
    fn periodic_phase_defaults_accept_legacy_and_explicit_zero_without_rewriting_bytes() {
        for candidate in ["harmonic", "source-filter"] {
            for (contract_declared, export_declared) in [(false,false),(true,false),(false,true),(true,true)] {
                let f = fixture(); final_package_candidate(&f, candidate);
                let mut contracts = json_file(&f, "contracts.json"); let mut export = json_file(&f, "export.json");
                if contract_declared { contracts["periodic_filter_phase"] = json!("zero-phase-v1"); }
                if export_declared { export["periodic_filter_phase"] = json!("zero-phase-v1"); }
                if contract_declared || export_declared { rewrite_package_semantics(&f, &contracts, &mut export); }
                let original = fs::read(f.directory.join("contracts.json")).unwrap();
                let package = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
                assert_eq!(package.voice.provenance["periodic_filter_phase"], "zero-phase-v1");
                assert_eq!(package.voice.provenance["export_provenance"]["periodic_filter_phase"], "zero-phase-v1");
                assert_eq!(fs::read(f.directory.join("contracts.json")).unwrap(), original);
                if !contract_declared { assert!(package.contracts.get("periodic_filter_phase").is_none()); }
            }
        }
    }
    #[test]
    fn minimum_phase_requires_bound_declarations_source_file_and_unchanged_frozen_semantics() {
        for alteration in 0..7 {
            let f = fixture(); final_package_candidate(&f, "source-filter");
            let mut contracts = json_file(&f, "contracts.json"); let mut export = json_file(&f, "export.json");
            contracts["periodic_filter_phase"] = json!("minimum-phase-real-cepstrum-v1");
            export["periodic_filter_phase"] = contracts["periodic_filter_phase"].clone();
            export["source_code"]["filter_phase.py"] = json!("4".repeat(64));
            match alteration {
                1 => { contracts.as_object_mut().unwrap().remove("periodic_filter_phase"); },
                2 => { export.as_object_mut().unwrap().remove("periodic_filter_phase"); },
                3 => export["periodic_filter_phase"] = json!("zero-phase-v1"),
                4 => { export["source_code"].as_object_mut().unwrap().remove("filter_phase.py"); },
                5 => contracts["periodic_parameterization"] = json!("unreviewed-filter-v2"),
                6 => export["periodic_filter_phase_extra"] = json!("not-an-admitted-field"),
                _ => {},
            }
            rewrite_package_semantics(&f, &contracts, &mut export);
            let result = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary));
            if alteration == 0 {
                let package = result.unwrap();
                assert_eq!(package.voice.provenance["periodic_filter_phase"], "minimum-phase-real-cepstrum-v1");
                // This synthetic package deliberately has non-executable graph
                // bytes: bank integrity is not runtime metadata/ABI admission.
                assert_eq!(package.voice.provenance["onnx_graph_abi_verified"], false);
            } else { assert!(result.is_err(), "rehashed phase alteration {alteration} was accepted"); }
        }
    }
    #[test]
    fn rehashed_null_wrong_type_unknown_phase_and_harmonic_minimum_are_rejected() {
        for invalid in [Value::Null,json!(true),json!(0),json!([]),json!({}),json!(""),json!("minimum-phase-v2")] {
            for in_contract in [false,true] {
                let f = fixture(); final_package_candidate(&f, "source-filter");
                let mut contracts = json_file(&f, "contracts.json"); let mut export = json_file(&f, "export.json");
                if in_contract { contracts["periodic_filter_phase"] = invalid.clone(); } else { export["periodic_filter_phase"] = invalid.clone(); }
                rewrite_package_semantics(&f, &contracts, &mut export);
                let error = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap_err();
                assert!(error.to_string().contains("periodic_filter_phase"), "unexpected error: {error:#}");
            }
        }
        let f = fixture(); final_package(&f);
        let mut contracts = json_file(&f, "contracts.json"); let mut export = json_file(&f, "export.json");
        contracts["periodic_filter_phase"] = json!("minimum-phase-real-cepstrum-v1");
        export["periodic_filter_phase"] = contracts["periodic_filter_phase"].clone(); export["source_code"]["filter_phase.py"] = json!("4".repeat(64));
        rewrite_package_semantics(&f, &contracts, &mut export);
        assert!(load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap_err().to_string().contains("only supported by source-filter"));
    }
    #[test]
    fn exact_binary64_and_all_token_material_survive() {
        let f = fixture();
        let admitted = verify(&f).unwrap();
        let mut bytes = Vec::new();
        for name in &admitted.voice.tree_order {
            for value in &admitted.voice.trees[name].records {
                bytes.extend(value.to_le_bytes());
            }
        }
        assert_eq!(bytes, f.binary);
        assert_eq!(admitted.voice.pulse["entries"], 4);
        assert_eq!(admitted.voice.provenance["leaf_table_selection"], true);
        assert_eq!(admitted.voice.provenance["requires_original_bin_at_runtime"], false);
        assert!(load_source_bank(&f.directory).is_err(), "production pins cannot accept a synthetic voice");
    }
    #[test]
    fn a_rehashed_modified_leaf_cannot_forge_original_source() {
        let f = fixture();
        let mut tensors = fs::read(f.directory.join("bank.safetensors")).unwrap();
        let hs = u64::from_le_bytes(tensors[..8].try_into().unwrap()) as usize;
        tensors[8 + hs..16 + hs].copy_from_slice(&1234.0f64.to_le_bytes());
        fs::write(f.directory.join("bank.safetensors"), &tensors).unwrap();
        resign(&f, "tensors", "bank.safetensors");
        let parsed = tensor_records(&tensors).unwrap();
        let mut m = json_file(&f, "bank.json");
        let mut binary = Vec::new();
        for entry in m["trees"].as_array_mut().unwrap() {
            let tensor = &parsed[entry["raw_tensor"].as_str().unwrap()];
            entry["raw_sha256"] = json!(digest(tensor.data));
            binary.extend(tensor.data);
        }
        m["source"]["model"]["sha256"] = json!(digest(&binary));
        write_json(&f.directory.join("bank.json"), &m);
        assert!(verify(&f).unwrap_err().to_string().contains("audited sn16k"));
    }
    #[test]
    fn rehashed_topology_question_and_pulse_changes_are_rejected() {
        for alteration in 0..3 {
            let f = fixture();
            if alteration < 2 {
                let mut t = json_file(&f, "topology.json");
                if alteration == 0 {
                    t["trees"]["phone-dur-tree"] = json!([-1, 3, 1]);
                } else {
                    t["questions"]["1"] = json!("a different predicate");
                }
                write_json(&f.directory.join("topology.json"), &t);
                resign(&f, "topology", "topology.json");
            } else {
                let token = String::from_utf8(f.token.clone()).unwrap().replace("0.125", "0.250");
                fs::write(f.directory.join("source.tkn"), token.as_bytes()).unwrap();
                let mut m = json_file(&f, "bank.json");
                m["source"]["token"] = descriptor("source.tkn", token.as_bytes());
                write_json(&f.directory.join("bank.json"), &m);
            }
            assert!(verify(&f).is_err());
        }
    }
    #[test]
    fn duplicate_nested_json_and_overlapping_tensor_ranges_are_rejected() {
        assert!(json_bytes(br#"{"outer":{"x":1,"x":2}}"#, "test").is_err());
        let f = fixture();
        let bytes = fs::read(f.directory.join("bank.safetensors")).unwrap();
        let hs = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        let mut h: Value = serde_json::from_slice(&bytes[8..8 + hs]).unwrap();
        h["cep1_raw"]["data_offsets"] = h["cep0_raw"]["data_offsets"].clone();
        let hb = serde_json::to_vec(&h).unwrap();
        let mut altered = (hb.len() as u64).to_le_bytes().to_vec();
        altered.extend(hb);
        altered.extend(&bytes[8 + hs..]);
        assert!(tensor_records(&altered).is_err());
    }
    #[test]
    fn package_binds_graphs_and_attestation_but_does_not_claim_abi_or_rights() {
        let f = fixture();
        assert!(load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).is_err());
        final_package(&f);
        let package = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
        assert_eq!(package.hashes.len(), 10);
        assert_eq!(package.voice.provenance["onnx_graph_abi_verified"], false);
        assert_eq!(package.voice.provenance["rights_attestation_is_legal_determination"], false);
        assert_eq!(package.export["quality_admission"], false);
        assert_eq!(package.voice.provenance["quality_admission_independently_verified"], false);
        let mut rights = json_file(&f, "rights.json");
        rights["source_binding"]["token_sha256"] = json!("f".repeat(64));
        write_json(&f.directory.join("rights.json"), &rights);
        let mut m = json_file(&f, "voice-source.json");
        m["rights"] = descriptor("rights.json", &fs::read(f.directory.join("rights.json")).unwrap());
        write_json(&f.directory.join("voice-source.json"), &m);
        assert!(
            load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary))
                .unwrap_err()
                .to_string()
                .contains("different source bytes")
        );
    }
    #[test]
    fn canonical_source_manifest_preserves_bytes_and_requires_a_bound_runtime() {
        let f = fixture(); final_package(&f);
        let bytes = fs::read(f.directory.join("voice-source.json")).unwrap();
        let package = load_package_with_pins(&f.directory,&digest(&f.token),&digest(&f.binary)).unwrap();
        assert_eq!(package.voice_version,env!("CARGO_PKG_VERSION"));
        assert_eq!(package.source_manifest_sha256(),digest(&bytes));
        assert_eq!(fs::read(f.directory.join("voice-source.json")).unwrap(),bytes);
        let runtime=f.directory.join("verdant-voice").join("manifest.json");
        let mut declaration:Value=serde_json::from_slice(&fs::read(&runtime).unwrap()).unwrap();
        declaration["source_manifest_sha256"]=json!("f".repeat(64));
        write_json(&runtime,&declaration);
        assert!(load_package_with_pins(&f.directory,&digest(&f.token),&digest(&f.binary)).is_err());
        fs::remove_file(&runtime).unwrap();
        assert!(load_package_with_pins(&f.directory,&digest(&f.token),&digest(&f.binary)).is_err());
        fs::rename(f.directory.join("voice-source.json"),f.directory.join("old-source.json")).unwrap();
        assert!(load_package_with_pins(&f.directory,&digest(&f.token),&digest(&f.binary)).is_err());
    }
    #[test]
    fn rehashed_export_contract_and_training_contradictions_are_rejected() {
        for alteration in 0..5 {
            let f = fixture();
            final_package(&f);
            let mut export = json_file(&f, "export.json");
            let mut manifest = json_file(&f, "voice-source.json");
            let expected_error = match alteration {
                0 => {
                    export["files"]["controller.onnx"] = json!("e".repeat(64));
                    "export file hashes"
                }
                1 => {
                    manifest["training"]["manifest_sha256"] = json!("a".repeat(64));
                    "training plan or corpus"
                }
                2 => {
                    let mut contracts = json_file(&f, "contracts.json");
                    contracts["controller_inputs"]["languages"]["meaning"] = json!("silently replace the frozen language enum");
                    write_json(&f.directory.join("contracts.json"), &contracts);
                    let bytes = fs::read(f.directory.join("contracts.json")).unwrap();
                    export["files"]["contracts.json"] = json!(digest(&bytes));
                    manifest["artifacts"]["contracts"] = descriptor("contracts.json", &bytes);
                    "frozen Verdant ONNX tensor semantics"
                }
                3 => {
                    export["quality_admission"] = json!("passed");
                    "boolean declaration"
                }
                _ => {
                    export["source_bank"]["model_sha256"] = json!("a".repeat(64));
                    "different original source bank bytes"
                }
            };
            write_json(&f.directory.join("export.json"), &export);
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let error = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap_err();
            assert!(error.to_string().contains(expected_error), "unexpected error for case {alteration}: {error:#}");
        }
    }
    #[test]
    fn attestation_time_checks_calendar_and_explicit_utc() {
        assert!(utc_timestamp("2024-02-29T23:59:59.123456Z"));
        for invalid in
            ["2026-02-29T00:00:00Z", "2026-11-31T00:00:00Z", "0000-01-01T00:00:00Z", "2026-01-01T00:00:00", "2026-01-01T00:00:60Z"]
        {
            assert!(!utc_timestamp(invalid));
        }
    }
    #[test]
    fn historical_contract_retains_its_semantics_and_unknown_grid_is_rejected() {
        let f = fixture();
        final_package(&f);
        let mut contracts = json_file(&f, "contracts.json");
        contracts.as_object_mut().unwrap().remove("sample_interpolation");
        let mut export = json_file(&f, "export.json");
        export.as_object_mut().unwrap().remove("numerical_patch");
        export["source_code"].as_object_mut().unwrap().remove("inference_model.py");
        for unknown in [false, true] {
            if unknown {
                contracts["sample_interpolation"] = json!("unreviewed-grid");
            }
            write_json(&f.directory.join("contracts.json"), &contracts);
            let contract_bytes = fs::read(f.directory.join("contracts.json")).unwrap();
            export["files"]["contracts.json"] = json!(digest(&contract_bytes));
            write_json(&f.directory.join("export.json"), &export);
            let mut manifest = json_file(&f, "voice-source.json");
            manifest["artifacts"]["contracts"] = descriptor("contracts.json", &contract_bytes);
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let loaded = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary));
            if unknown {
                assert!(loaded.unwrap_err().to_string().contains("frozen Verdant ONNX tensor semantics"));
            } else {
                assert_eq!(loaded.unwrap().voice.provenance["declared_sample_interpolation"], "historical-float32-position");
            }
        }
    }
    #[test]
    fn integer_grid_numerical_patch_is_bound_to_training_and_export_sources() {
        let f = fixture();
        final_package(&f);
        let admitted = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
        assert_eq!(admitted.voice.provenance["declared_sample_interpolation"], "integer-sample-grid-v1");
        let mut export = json_file(&f, "export.json");
        export["numerical_patch"]["export_decoder_source_sha256"] = json!("e".repeat(64));
        write_json(&f.directory.join("export.json"), &export);
        let mut manifest = json_file(&f, "voice-source.json");
        manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
        write_json(&f.directory.join("voice-source.json"), &manifest);
        assert!(
            load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary))
                .unwrap_err()
                .to_string()
                .contains("numerical patch source hashes")
        );
    }
    #[test]
    fn source_filter_requires_its_specific_contract_and_both_abis() {
        for alteration in 0..7 {
            let f = fixture();
            final_package_candidate(&f, "source-filter");
            let admitted = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
            assert_eq!(admitted.voice.provenance["decoder_candidate"], "source-filter");
            assert_eq!(admitted.manifest["models"]["controller"]["abi"], "cfinal-source-filter-controller-v1");
            assert_eq!(admitted.manifest["models"]["decoder"]["abi"], "cfinal-source-filter-decoder-v1");
            let mut export = json_file(&f, "export.json");
            let mut manifest = json_file(&f, "voice-source.json");
            match alteration {
                0 => export["controller_abi"] = json!("cfinal-controller-v1"),
                1 => export["decoder_abi"] = json!("cfinal-decoder-v1"),
                2 => manifest["models"]["decoder"]["abi"] = json!("cfinal-decoder-v1"),
                3 => export["decoder_candidate"] = json!("harmonic"),
                4 => fs::write(f.directory.join("contracts.json"), SUPPORTED_CONTRACTS).unwrap(),
                5 => {
                    let mut contracts = json_file(&f, "contracts.json");
                    contracts.as_object_mut().unwrap().remove("sample_interpolation");
                    write_json(&f.directory.join("contracts.json"), &contracts);
                }
                _ => export["decoder_candidate"] = json!("arbitrary-unknown-model"),
            }
            let bytes = fs::read(f.directory.join("contracts.json")).unwrap();
            export["files"]["contracts.json"] = json!(digest(&bytes));
            manifest["artifacts"]["contracts"] = descriptor("contracts.json", &bytes);
            write_json(&f.directory.join("export.json"), &export);
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            assert!(
                load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).is_err(),
                "mismatched candidate/ABI/contract {alteration} passed"
            );
        }
    }

    #[test]
    fn resolved_neighbor_contract_requires_a_matching_export_declaration() {
        for alteration in 0..5 {
            let f = fixture();
            final_package_candidate(&f, "source-filter");
            let mut contracts = json_file(&f, "contracts.json");
            let mut export = json_file(&f, "export.json");
            let mut manifest = json_file(&f, "voice-source.json");
            contracts["neighbor_encoding"] = json!("resolved-context-phone-v2");
            export["neighbor_encoding"] = json!("resolved-context-phone-v2");
            for name in ["planner.py", "neighbor.py", "abi.py"] {
                export["source_code"][name] = json!("4".repeat(64));
            }
            match alteration {
                1 => { export.as_object_mut().unwrap().remove("neighbor_encoding"); },
                2 => export["neighbor_encoding"] = json!("legacy-explicit-phone-v1"),
                3 => {
                    contracts["neighbor_encoding"] = json!("unknown-v3");
                    export["neighbor_encoding"] = json!("unknown-v3");
                },
                4 => export["neighbor_encoding"] = json!(null),
                _ => {},
            }
            write_json(&f.directory.join("contracts.json"), &contracts);
            let bytes = fs::read(f.directory.join("contracts.json")).unwrap();
            export["files"]["contracts.json"] = json!(digest(&bytes));
            manifest["artifacts"]["contracts"] = descriptor("contracts.json", &bytes);
            write_json(&f.directory.join("export.json"), &export);
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let result = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary));
            if alteration == 0 {
                assert_eq!(result.unwrap().voice.provenance["neighbor_encoding"], "resolved-context-phone-v2");
            } else {
                assert!(result.is_err(), "neighbor encoding alteration {alteration} passed");
            }
        }
    }
    #[test]
    fn training_clock_defaults_to_legacy_and_accepts_both_explicit_versions() {
        for candidate in ["harmonic", "source-filter"] {
            for clock in [None, Some("padded-batch-v1"), Some("per-clip-v2")] {
                let f = fixture();
                final_package_candidate(&f, candidate);
                let original_contracts = fs::read(f.directory.join("contracts.json")).unwrap();
                if let Some(clock) = clock {
                    let mut export = json_file(&f, "export.json");
                    export["training_clock"] = json!(clock);
                    write_json(&f.directory.join("export.json"), &export);
                    let mut manifest = json_file(&f, "voice-source.json");
                    manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
                    write_json(&f.directory.join("voice-source.json"), &manifest);
                }
                let package = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
                assert_eq!(package.voice.provenance["export_provenance"]["training_clock"], clock.unwrap_or("padded-batch-v1"));
                assert_eq!(package.voice.provenance["export_provenance"]["training_noise_policy"], "overlapping-counter-windows-v1");
                assert_eq!(fs::read(f.directory.join("contracts.json")).unwrap(), original_contracts);
                assert!(package.contracts.get("training_clock").is_none(), "training provenance is not a graph ABI field");
            }
        }
    }
    #[test]
    fn rehashed_invalid_training_clock_is_rejected() {
        for invalid in [json!("unknown-clock"), json!(""), json!(null), json!(2), json!(true), json!([]), json!({})] {
            let f = fixture();
            final_package_candidate(&f, "source-filter");
            let mut export = json_file(&f, "export.json");
            export["training_clock"] = invalid;
            write_json(&f.directory.join("export.json"), &export);
            let mut manifest = json_file(&f, "voice-source.json");
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let error = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap_err();
            assert!(error.to_string().contains("training_clock"), "unexpected error: {error:#}");
        }
    }
    #[test]
    fn training_clock_does_not_relax_the_export_field_inventory() {
        let f = fixture();
        final_package(&f);
        let mut export = json_file(&f, "export.json");
        export["training_clock"] = json!("padded-batch-v1");
        export["training_clock_extra"] = json!("not an admitted field");
        write_json(&f.directory.join("export.json"), &export);
        let mut manifest = json_file(&f, "voice-source.json");
        manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
        write_json(&f.directory.join("voice-source.json"), &manifest);
        assert!(load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).is_err());
    }
    #[test]
    fn training_noise_policy_defaults_to_legacy_and_accepts_both_explicit_versions() {
        for policy in [None, Some("overlapping-counter-windows-v1"), Some("disjoint-counter-windows-v2")] {
            let f = fixture();
            final_package_candidate(&f, "source-filter");
            let original_contracts = fs::read(f.directory.join("contracts.json")).unwrap();
            let mut export = json_file(&f, "export.json");
            export["training_clock"] = json!("per-clip-v2");
            if let Some(policy) = policy {
                export["training_noise_policy"] = json!(policy);
            }
            write_json(&f.directory.join("export.json"), &export);
            let mut manifest = json_file(&f, "voice-source.json");
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let package = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap();
            assert_eq!(package.voice.provenance["export_provenance"]["training_noise_policy"], policy.unwrap_or("overlapping-counter-windows-v1"));
            assert_eq!(package.voice.provenance["export_provenance"]["training_clock"], "per-clip-v2");
            assert_eq!(fs::read(f.directory.join("contracts.json")).unwrap(), original_contracts);
            assert!(package.contracts.get("training_noise_policy").is_none());
        }
    }
    #[test]
    fn rehashed_invalid_training_noise_policy_is_rejected() {
        for invalid in [json!("unknown-policy"), json!(""), json!(null), json!(2), json!(true), json!([]), json!({})] {
            let f = fixture();
            final_package_candidate(&f, "source-filter");
            let mut export = json_file(&f, "export.json");
            export["training_noise_policy"] = invalid;
            write_json(&f.directory.join("export.json"), &export);
            let mut manifest = json_file(&f, "voice-source.json");
            manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
            write_json(&f.directory.join("voice-source.json"), &manifest);
            let error = load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary)).unwrap_err();
            assert!(error.to_string().contains("training_noise_policy"), "unexpected error: {error:#}");
        }
    }
    #[test]
    fn harmonic_cannot_relabel_source_filter_semantics_as_its_old_abi() {
        let f = fixture();
        final_package(&f);
        fs::write(f.directory.join("contracts.json"), SOURCE_FILTER_CONTRACTS).unwrap();
        let mut export = json_file(&f, "export.json");
        let mut manifest = json_file(&f, "voice-source.json");
        export["files"]["contracts.json"] = json!(digest(SOURCE_FILTER_CONTRACTS));
        manifest["artifacts"]["contracts"] = descriptor("contracts.json", SOURCE_FILTER_CONTRACTS);
        write_json(&f.directory.join("export.json"), &export);
        manifest["artifacts"]["export"] = descriptor("export.json", &fs::read(f.directory.join("export.json")).unwrap());
        write_json(&f.directory.join("voice-source.json"), &manifest);
        assert!(
            load_package_with_pins(&f.directory, &digest(&f.token), &digest(&f.binary))
                .unwrap_err()
                .to_string()
                .contains("frozen Verdant ONNX tensor semantics")
        );
    }
    #[test]
    #[ignore = "requires a local source-preserving bank; no original weights are distributed"]
    fn local_actual_bank_roundtrip() {
        let path = std::env::var_os("VERDANT_SOURCE_BANK_TEST_DIR").expect("set explicit local bank directory");
        let voice = load_source_bank(Path::new(&path)).unwrap();
        assert_eq!(
            source_inventory(&voice),
            json!({"questions":584,"trees":12,"leaves":11302,"internal_nodes":11290,"nodes":22592,"f64_values":1058828,"pulse_entries":8192})
        );
        assert!(voice.source_identity_confirmed());
    }
}
