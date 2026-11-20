//! Read-only raw-token and checksummed modern-directory admission.
use std::{collections::{BTreeMap, BTreeSet}, fs, path::{Path, PathBuf}};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use super::{data::{digest, profiles}, source_pins::{KNOWN_MODEL_SHA, KNOWN_TOKEN_SHA}, Node, Tree, TreeKind, VoiceModel};

const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;
const MODEL_FILES: [&str; 4] = ["voice.json", "tensors.safetensors", "topology.json", "PROVENANCE.json"];

pub(super) fn load(path: &Path) -> Result<VoiceModel> {
    if path.is_dir() {
        if path.join("voice.json").is_file() { return load_modern(path); }
        let mut tokens = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && entry.path().extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("tkn")) {
                tokens.push(entry.path());
            }
        }
        ensure!(tokens.len() == 1, "raw model directory must contain exactly one .tkn file; use load_raw_voice for explicit pairs");
        let model = tokens[0].with_extension("bin");
        return load_raw(&tokens[0], &model);
    }
    if path.file_name().is_some_and(|s| s == "voice.json") { return load_modern(path.parent().context("voice.json has no parent directory")?); }
    match path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("tkn") => load_raw(path, &path.with_extension("bin")),
        Some("bin") => load_raw(&path.with_extension("tkn"), path),
        _ => bail!("expected a modern model directory, voice.json, or a raw .tkn/.bin pair"),
    }
}
pub(super) fn load_raw(token: &Path, model: &Path) -> Result<VoiceModel> {
    let token_bytes = fs::read(token).with_context(|| format!("reading token {}", token.display()))?;
    let model_bytes = fs::read(model).with_context(|| format!("reading model {}", model.display()))?;
    parse_raw(&token_bytes, &model_bytes, token, model)
}
fn file_name(path: &Path) -> Result<&str> {
    path.file_name().and_then(|p| p.to_str()).context("model file name must be UTF-8")
}
fn encoding_label(bytes: &[u8]) -> Option<&[u8]> {
    let head = &bytes[..bytes.len().min(256)];
    let start = head.windows(9).position(|w| w == b"encoding=")? + 9;
    let quote = *head.get(start)?;
    if !matches!(quote, b'\'' | b'"') { return None; }
    let end = head[start + 1..].iter().position(|&v| v == quote)? + start + 1;
    Some(&head[start + 1..end])
}
fn decode_token(bytes: &[u8]) -> Result<String> {
    let label = encoding_label(bytes).unwrap_or(b"utf-8");
    ensure!(label.is_ascii(), "token encoding name must be ASCII");
    if label.eq_ignore_ascii_case(b"ascii") || label.eq_ignore_ascii_case(b"us-ascii") {
        ensure!(bytes.is_ascii(), "non-ASCII bytes in ASCII token");
        return Ok(std::str::from_utf8(bytes)?.to_owned());
    }
    let encoding = encoding_rs::Encoding::for_label(label).context("unsupported token character encoding")?;
    let (text, _, invalid) = encoding.decode(bytes);
    ensure!(!invalid, "token contains invalid bytes for its declared encoding");
    Ok(text.into_owned())
}
fn attributes(node: roxmltree::Node<'_, '_>) -> Value {
    Value::Object(node.attributes().map(|a| (a.name().to_owned(), Value::String(a.value().to_owned()))).collect())
}
fn parse_usize(value: &str, what: &str) -> Result<usize> {
    value.parse().with_context(|| format!("invalid {what}"))
}
fn json_usize(value: &Value, what: &str) -> Result<usize> {
    usize::try_from(value.as_u64().with_context(|| format!("{what} must be a nonnegative integer"))?).with_context(|| format!("{what} is too large"))
}
pub(super) fn records(bytes: &[u8], width: usize) -> Result<Vec<f64>> {
    ensure!(!bytes.is_empty() && bytes.len() % (width * 8) == 0, "model region length does not match leaf width");
    let mut result = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let value = f64::from_le_bytes(chunk.try_into()?);
        ensure!(value.is_finite(), "nonfinite original model value");
        result.push(value);
    }
    Ok(result)
}
/// Iterative construction admits complete binary topology without recursion or a depth assumption.
pub(super) fn topology(numbers: &[i64], width: usize, count: usize, questions: &BTreeMap<i64, String>) -> Result<(Vec<Node>, Vec<Option<(usize, bool)>>, Vec<usize>)> {
    ensure!(!numbers.is_empty() && count > 0, "topology nodes and leaf records must be nonempty");
    let mut nodes: Vec<Node> = Vec::with_capacity(numbers.len());
    let mut parents = Vec::with_capacity(numbers.len());
    let mut leaf_nodes = vec![usize::MAX; count];
    let mut pending: Vec<Option<(usize, bool)>> = vec![None];
    for &number in numbers {
        let parent = pending.pop().context("invalid or trailing preorder node")?;
        let index = nodes.len();
        if let Some((parent_index, first)) = parent {
            let Node::Branch { first: a, second: b, .. } = &mut nodes[parent_index] else { bail!("leaf cannot have a child"); };
            if first { *a = index; } else { *b = index; }
        }
        parents.push(parent);
        if number < 0 {
            let question_id = number.checked_neg().context("invalid question ID")?;
            ensure!(questions.contains_key(&question_id), "undefined question {question_id}");
            nodes.push(Node::Branch { question_id, first: usize::MAX, second: usize::MAX });
            pending.push(Some((index, false)));
            pending.push(Some((index, true)));
        } else {
            let offset = usize::try_from(number).context("leaf offset overflow")?;
            ensure!(offset > 0 && (offset - 1) % width == 0, "invalid 1-based leaf vector offset");
            let leaf = (offset - 1) / width;
            ensure!(leaf < count, "leaf beyond model region");
            ensure!(leaf_nodes[leaf] == usize::MAX, "duplicate leaf in topology");
            leaf_nodes[leaf] = index;
            nodes.push(Node::Leaf(leaf));
        }
    }
    ensure!(pending.is_empty() && leaf_nodes.iter().all(|&v| v != usize::MAX), "tree/leaf coverage is not one-to-one and complete");
    Ok((nodes, parents, leaf_nodes))
}
pub(super) fn build_tree(name: String, kind: TreeKind, values: Vec<f64>, numbers: &[i64], begin: usize, end: usize, questions: &BTreeMap<i64, String>) -> Result<Tree> {
    let width = kind.width();
    let (nodes, parents, leaf_nodes) = topology(numbers, width, values.len() / width, questions)?;
    Ok(Tree { name, kind, records: values, nodes, parents, leaf_nodes, begin, end })
}
pub(super) fn validate_inventory(model: &VoiceModel) -> Result<()> {
    ensure!(model.sample_rate > 0, "model sample rate must be positive");
    ensure!(model.trees.len() == 12, "the audited model requires exactly 12 trees");
    for (name, kind) in [("phone-dur-tree", TreeKind::PhoneDuration), ("state-dur-tree", TreeKind::StateDuration)] {
        ensure!(model.trees.get(name).is_some_and(|t| t.kind == kind), "missing or misnamed audited tree {name}");
    }
    for state in 0..5 {
        for (prefix, kind) in [("cep", TreeKind::Spectrum), ("pitch", TreeKind::Pitch)] {
            let name = format!("{prefix}{state}");
            ensure!(model.trees.get(&name).is_some_and(|t| t.kind == kind), "missing or misnamed audited tree {name}");
        }
    }
    Ok(())
}
pub(super) fn parse_raw(token_bytes: &[u8], model_bytes: &[u8], token_path: &Path, model_path: &Path) -> Result<VoiceModel> {
    profiles()?;
    let text = decode_token(token_bytes)?;
    let upper = text.to_ascii_uppercase();
    ensure!(!upper.contains("<!DOCTYPE") && !upper.contains("<!ENTITY"), "DTD/entity declarations are not supported");
    let start = text.find("<![CDATA[").context("expected one original CDATA region")? + 9;
    let end = text[start..].find("]]>").context("unterminated original CDATA region")? + start;
    ensure!(!text[end + 3..].contains("<![CDATA["), "expected one original CDATA region");
    let cdata = &text[start..end];
    let offsets = if cdata.is_ascii() { None } else { Some(cdata.char_indices().map(|(i, _)| i).chain(std::iter::once(cdata.len())).collect::<Vec<_>>()) };
    let chars = offsets.as_ref().map_or(cdata.len(), |v| v.len() - 1);
    let mut xml = text.as_str();
    let stripped;
    if let Some(declaration) = text.find("<?xml") {
        let stop = text[declaration..].find("?>").context("unterminated XML declaration")? + declaration + 2;
        stripped = format!("{}{}", &text[..declaration], &text[stop..]);
        xml = &stripped;
    }
    let document = roxmltree::Document::parse(xml).context("invalid original token XML")?;
    let voices: Vec<_> = document.root_element().children().filter(|n| n.has_tag_name("voice")).collect();
    ensure!(voices.len() == 1, "expected one voice");
    let voice = voices[0];
    let tokens: Vec<_> = voice.children().filter(|n| n.has_tag_name("token")).collect();
    ensure!(tokens.len() == 1, "expected one voice token");
    let token = tokens[0];
    ensure!(token.attribute("lsf_coef_order") == Some("24") && token.attribute("lsf_diff_order") == Some("16"), "only the audited 24+16 layout is supported");
    let sample_rate: u32 = token.attribute("sample-rate").context("token lacks sample-rate")?.parse().context("invalid sample rate")?;
    let mut questions = BTreeMap::new();
    for table in token.children().filter(|n| n.has_tag_name("qstable")) {
        for item in table.children().filter(|n| n.has_tag_name("queset")) {
            let id: i64 = item.text().context("question has no ID")?.trim().parse().context("invalid question ID")?;
                        let name = item.attribute("name").context("question has no name")?.to_owned();
            ensure!(questions.insert(id, name).is_none(), "duplicate question IDs");
        }
    }
    let (mut text_cursor, mut model_cursor) = (0, 0);
    let mut trees = BTreeMap::new();
    let mut tree_order = Vec::with_capacity(12);
    let mut pulse = Value::Null;
    for item in token.children().filter(|n| n.is_element()) {
        let tag = item.tag_name().name();
        if tag == "qstable" { continue; }
        let span = item.text().unwrap_or("").trim();
        let (first, last) = span.split_once('-').context("bad CDATA range")?;
        ensure!(!first.is_empty() && !last.is_empty() && first.bytes().all(|b| b.is_ascii_digit()) && last.bytes().all(|b| b.is_ascii_digit()), "bad CDATA range");
        let first = parse_usize(first, "CDATA start")?;
        let last = parse_usize(last, "CDATA end")?;
        ensure!(first == text_cursor && first <= last && last < chars, "noncontiguous/out-of-bounds CDATA region");
        let raw = if let Some(ref indices) = offsets { &cdata[indices[first]..indices[last + 1]] } else { &cdata[first..last + 1] };
        text_cursor = last + 1;
        if tag == "pulse" {
            ensure!(pulse.is_null(), "duplicate pulse region");
            let parts: Vec<_> = raw.split_whitespace().collect();
            ensure!(parts.iter().all(|p| p.split_once('.').is_some_and(|(a,b)| !a.is_empty() && !b.is_empty() && a.bytes().all(|v| v.is_ascii_digit()) && b.bytes().all(|v| v.is_ascii_digit()))), "unexpected pulse encoding");
            pulse = json!({"entries": parts.len(), "status": "opaque_not_used", "normalized_sha256": digest(parts.join(" ").as_bytes())});
            continue;
        }
        let begin = parse_usize(item.attribute("mbegin").context("tree lacks mbegin")?, "binary start")?;
        let end = parse_usize(item.attribute("mend").context("tree lacks mend")?, "binary end")?;
        ensure!(begin == model_cursor && begin < end && end <= model_bytes.len() && (end - begin) % 8 == 0, "invalid/noncontiguous binary region");
        model_cursor = end;
        let kind = TreeKind::parse(tag)?;
        let values = records(&model_bytes[begin..end], kind.width())?;
        let numbers = raw.split_whitespace().map(str::parse).collect::<std::result::Result<Vec<i64>, _>>().context("non-integer preorder node")?;
        let name = item.attribute("name").unwrap_or(tag).to_owned();
        let tree = build_tree(name.clone(), kind, values, &numbers, begin, end, &questions)?;
        ensure!(trees.insert(name.clone(), tree).is_none(), "duplicate original tree name");
        tree_order.push(name);
    }
    ensure!(text_cursor == chars && model_cursor == model_bytes.len(), "uncovered original data");
    let voice_attributes = attributes(voice);
    let token_attributes = attributes(token);
    let token_hash = digest(token_bytes);
    let provenance = json!({
        "token": {"name": file_name(token_path)?, "bytes": token_bytes.len(), "sha256": token_hash},
        "model": {"name": file_name(model_path)?, "bytes": model_bytes.len(), "sha256": digest(model_bytes)},
        "voice_attributes": voice_attributes, "token_attributes": token_attributes,
        "identity_status": if token_hash == KNOWN_TOKEN_SHA { "owner-confirmed sn/楠楠 VID10008 as target 童声01/绿坝娘; voice identity does not establish original-engine synthesis fidelity" } else { "input voice parsed under the audited 24+16 layout; voice identity not verified" },
    });
    let model = VoiceModel { sample_rate, voice_attributes, token_attributes, provenance, questions, trees, tree_order, pulse, format: "original-token-model" };
    validate_inventory(&model)?;
    Ok(model)
}
fn bounded_file(path: &Path, max: u64) -> Result<Vec<u8>> {
    ensure!(fs::metadata(path).with_context(|| format!("missing model file {}", path.display()))?.len() <= max, "model JSON/header exceeds {max} bytes: {}", path.display());
    let bytes = fs::read(path)?;
    ensure!(bytes.len() as u64 <= max, "model JSON/header exceeds {max} bytes");
    Ok(bytes)
}
fn json_file(bytes: &[u8], label: &str) -> Result<Value> {
    let value: Value = serde_json::from_slice(bytes).with_context(|| format!("invalid JSON in {label}"))?;
    ensure!(value.is_object(), "{label} must be a JSON object");
    Ok(value)
}
fn verify_checksums(manifest: &[u8], files: &BTreeMap<&str, Vec<u8>>) -> Result<()> {
    let sums = std::str::from_utf8(manifest).context("SHA256SUMS must be UTF-8")?;
    let mut checked = BTreeSet::new();
    for line in sums.lines() {
        let (expected, name) = line.split_once("  ").context("invalid checksum manifest entry")?;
        ensure!(MODEL_FILES.contains(&name) && checked.insert(name) && expected.len() == 64 && expected.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "invalid checksum manifest entry");
        ensure!(digest(files.get(name).context("checksum file is missing")?) == expected, "checksum mismatch: {name}");
    }
    ensure!(checked.len() == MODEL_FILES.len(), "checksum manifest does not cover all required model files");
    Ok(())
}
struct Tensor<'a> { shape: Vec<usize>, data: &'a [u8] }
fn safetensors(bytes: &[u8]) -> Result<BTreeMap<String, Tensor<'_>>> {
    ensure!(bytes.len() >= 8, "tensors.safetensors missing or too small");
    let header_length = usize::try_from(u64::from_le_bytes(bytes[..8].try_into()?)).context("safetensors header length overflow")?;
    ensure!(header_length as u64 <= MAX_JSON_BYTES && header_length <= bytes.len() - 8, "safetensors header length out of bounds");
    let header = json_file(&bytes[8..8 + header_length], "safetensors header")?;
    let body = &bytes[8 + header_length..];
    let mut tensors = BTreeMap::new();
    let mut regions = Vec::new();
    for (name, meta) in header.as_object().context("invalid safetensors header")? {
        if name == "__metadata__" { continue; }
        ensure!(meta.is_object() && meta["dtype"] == "F64", "tensor {name}: only dtype F64 is supported");
        let shape_json = meta["shape"].as_array().context("tensor shape must be an array")?;
        ensure!(!shape_json.is_empty(), "tensor {name}: invalid shape");
        let mut shape = Vec::with_capacity(shape_json.len());
        let mut size = 8usize;
        for value in shape_json {
            let dimension = json_usize(value, "tensor dimension")?;
            ensure!(dimension > 0 && dimension <= (1usize << 31), "tensor {name}: invalid shape");
            size = size.checked_mul(dimension).context("tensor size overflow")?;
            shape.push(dimension);
        }
        let offsets = meta["data_offsets"].as_array().context("tensor data_offsets must be an array")?;
        ensure!(offsets.len() == 2, "tensor {name}: invalid data_offsets");
        let (a, b) = (json_usize(&offsets[0], "tensor start")?, json_usize(&offsets[1], "tensor end")?);
        ensure!(a <= b && b <= body.len() && b - a == size, "tensor {name}: invalid data_offsets or byte size");
        regions.push((a, b));
        tensors.insert(name.clone(), Tensor { shape, data: &body[a..b] });
    }
    regions.sort_unstable();
    let mut cursor = 0;
    for (a, b) in regions { ensure!(a == cursor, "safetensors contains overlapping or uncovered data"); cursor = b; }
    ensure!(cursor == body.len(), "safetensors contains trailing uncovered data");
    Ok(tensors)
}
fn load_modern(directory: &Path) -> Result<VoiceModel> {
    profiles()?;
    let mut files = BTreeMap::new();
    for name in MODEL_FILES {
        let path: PathBuf = directory.join(name);
        let bytes = if name == "tensors.safetensors" { fs::read(&path).with_context(|| format!("reading {}", path.display()))? } else { bounded_file(&path, MAX_JSON_BYTES)? };
        files.insert(name, bytes);
    }
    verify_checksums(&bounded_file(&directory.join("SHA256SUMS"), MAX_JSON_BYTES)?, &files)?;
    let voice = json_file(&files["voice.json"], "voice.json")?;
    let topology_json = json_file(&files["topology.json"], "topology.json")?;
    json_file(&files["PROVENANCE.json"], "PROVENANCE.json")?;
    ensure!(voice["format"] == "greendam-voice-model" && voice["format_version"].as_u64() == Some(1), "unsupported voice model format/version");
    ensure!(voice["distribution"]["release_asset"].as_bool() == Some(false), "development-only model marker missing");
    ensure!(voice["source_sha256"]["sn16k.tkn"].as_str() == Some(KNOWN_TOKEN_SHA) && voice["source_sha256"]["sn16k.bin"].as_str() == Some(KNOWN_MODEL_SHA), "source fingerprints do not match the confirmed candidate");
    ensure!(voice["lsf_coef_order"].as_u64() == Some(24) && voice["lsf_diff_order"].as_u64() == Some(16), "only the audited 24+16 layout is supported");
    ensure!(topology_json["format"] == "greendam-voice-topology" && topology_json["format_version"].as_u64() == Some(1) && topology_json["source_sha256"] == voice["source_sha256"], "topology format or source fingerprints do not match voice.json");
    let mut questions = BTreeMap::new();
    for (key, value) in voice["questions"].as_object().context("voice.json lacks questions")? {
        let id: i64 = key.parse().context("invalid question ID")?;
        ensure!(id > 0, "question IDs must be positive");
        ensure!(questions.insert(id, value.as_str().context("question name must be a string")?.to_owned()).is_none(), "duplicate question IDs");
    }
    let encoded = topology_json["trees"].as_object().context("topology lacks trees")?;
    let tensors = safetensors(&files["tensors.safetensors"])?;
    let tree_metadata = voice["trees"].as_object().context("voice.json lacks trees")?;
    let mut trees = BTreeMap::new();
    let mut tree_order = Vec::with_capacity(tree_metadata.len());
    for (key, meta) in tree_metadata {
        let kind = TreeKind::parse(meta["kind"].as_str().context("tree lacks kind")?)?;
        let width = json_usize(&meta["values_per_leaf"], "values_per_leaf")?;
        ensure!(width == kind.width(), "tree {key}: unsupported leaf layout");
        let count = json_usize(&meta["leaf_count"], "leaf_count")?;
        ensure!(count > 0, "tree {key}: invalid leaf count");
        let name = meta.get("name").map(|v| v.as_str().context("tree name must be a string")).transpose()?.unwrap_or(key).to_owned();
        let tensor = tensors.get(meta["raw_tensor"].as_str().context("tree lacks raw_tensor")?).context("raw tensor absent from safetensors")?;
        ensure!(tensor.shape == [count, width], "tree {key}: raw tensor shape does not match leaf layout");
        let numbers = encoded.get(&name).context("topology tree is missing")?.as_array().context("topology nodes must be an integer array")?.iter().map(|v| v.as_i64().context("topology nodes must be integers")).collect::<Result<Vec<_>>>()?;
        let range = meta["byte_range"].as_array().context("tree lacks original byte_range")?;
        ensure!(range.len() == 2, "tree byte_range must contain two offsets");
        let (begin, end) = (json_usize(&range[0], "tree byte start")?, json_usize(&range[1], "tree byte end")?);
        ensure!(begin < end && end - begin == tensor.data.len() && begin % 8 == 0, "tree original byte range does not match raw tensor");
        let tree = build_tree(name.clone(), kind, records(tensor.data, width)?, &numbers, begin, end, &questions)?;
        ensure!(trees.insert(name.clone(), tree).is_none(), "duplicate tree name");
        tree_order.push(name);
    }
    ensure!(encoded.len() == trees.len() && encoded.keys().all(|key| trees.contains_key(key)), "topology tree inventory does not match voice.json");
    let mut ranges: Vec<_> = trees.values().map(|t| (t.begin, t.end)).collect();
    ranges.sort_unstable();
    let mut cursor = 0;
    for (begin, end) in ranges { ensure!(begin == cursor, "noncontiguous original model byte ranges"); cursor = end; }
    if let Some(length) = voice.get("source_bytes").and_then(|v| v.get("sn16k.bin")) {
        ensure!(json_usize(length, "source model byte length")? == cursor, "uncovered original model byte ranges");
    }
    let voice_attributes = voice.get("voice_attributes").filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
    let token_attributes = voice.get("token_attributes").filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
    let identity = voice.get("identity").cloned().unwrap_or_else(|| json!({}));
    let provenance = json!({
        "voice_model": {"name": "voice.json", "format": "greendam-voice-model", "format_version": 1, "sha256": digest(&files["voice.json"])},
        "tensors": {"name": "tensors.safetensors", "sha256": digest(&files["tensors.safetensors"])},
        "topology": {"name": "topology.json", "sha256": digest(&files["topology.json"])},
        "source_sha256": voice["source_sha256"], "voice_attributes": voice_attributes, "token_attributes": token_attributes,
        "identity": identity,
        "identity_status": if identity.get("confirmed_by").is_some_and(|v| match v { Value::Null => false, Value::Bool(value) => *value, Value::String(value) => !value.is_empty(), Value::Number(value) => value.as_f64() != Some(0.0), Value::Array(value) => !value.is_empty(), Value::Object(value) => !value.is_empty() }) { "owner-confirmed sn/楠楠 VID 10008 as 童声01 (IDENTITY-CONFIRMATION-2026-11-13)" } else { "voice identity field absent; not owner-confirmed" },
    });
    let sample_rate = u32::try_from(voice["sample_rate"].as_u64().context("sample_rate must be a positive integer")?).context("sample_rate exceeds u32")?;
    let model = VoiceModel { sample_rate, voice_attributes, token_attributes, provenance, questions, trees, tree_order, pulse: voice.get("pulse").cloned().unwrap_or(Value::Null), format: "greendam-voice-model-v1" };
    validate_inventory(&model)?;
    Ok(model)
}
