//! Exact raw-token, typed-index and F64 leaf transport in one safetensors file.
//! The token and reconstructed parameter stream are checked against independent
//! source pins. Metadata declarations cannot authenticate themselves.
use std::collections::{BTreeMap, BTreeSet};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use verdant_voice_format::{self as format, tensors::{self, Data, Dtype, Input, Tensor, Tensors}};
use super::{Node, VoiceModel, source_pins::{KNOWN_TOKEN_SHA, KNOWN_MODEL_SHA}};

const METADATA_KEYS: &[&str] = &["schema", "merge.schema_version", "source.token_sha256", "source.model_sha256",
    "source.token_bytes", "source.model_bytes", "tree_order", "source.bin_distribution"];
const BIN_POLICY: &str = "not-distributed; reconstructed binary is verification data, not an original installation file";

fn raw_name(name: &str) -> String { format!("{}_raw", name.replace('-', "_")) }
fn preorder_name(name: &str) -> String { format!("token.{name}.preorder") }
fn preorder(voice: &VoiceModel, name: &str) -> Vec<i64> {
    let tree = &voice.trees[name];
    tree.nodes.iter().map(|node| match *node {
        Node::Branch { question_id, .. } => -question_id,
        Node::Leaf(index) => (index * tree.kind.width() + 1) as i64,
    }).collect()
}

fn question_index(voice: &VoiceModel) -> Result<(Vec<i64>, Vec<i64>, Vec<u8>)> {
    let mut ids = Vec::with_capacity(voice.questions.len());
    let mut offsets = vec![0];
    let mut names = Vec::new();
    for (&id, name) in &voice.questions {
        ensure!(id > 0 && !name.is_empty() && !name.contains('\0'), "invalid source question index");
        ids.push(id); names.extend_from_slice(name.as_bytes());
        offsets.push(i64::try_from(names.len())?);
    }
    Ok((ids, offsets, names))
}

fn phone_index(phones: &[u8]) -> Result<(Vec<i64>, Vec<u8>, Value)> {
    let value = format::parse_json(phones)?;
    let rows = value.as_array().context("phone table must be an array")?;
    ensure!(!rows.is_empty() && rows.len() <= 256, "invalid phone table inventory");
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(rows.len());
    let mut flags = Vec::with_capacity(rows.len() * 2);
    for row in rows {
        let id = row["id"].as_u64().filter(|id| *id < 256).context("invalid native phone ID")?;
        ensure!(seen.insert(id), "duplicate native phone ID");
        ids.push(i64::try_from(id)?);
        flags.push(u8::from(row["fricative"].as_bool().context("phone fricative flag must be boolean")?));
        flags.push(u8::from(row["affricate"].as_bool().context("phone affricate flag must be boolean")?));
    }
    Ok((ids, flags, value))
}

pub(super) fn encode(voice: &VoiceModel, token: &[u8], phones: &[u8]) -> Result<Vec<u8>> {
    super::loading::validate_inventory(voice)?;
    ensure!(!token.is_empty() && token.len() <= 2 * 1024 * 1024, "token exceeds its bound");
    let token_sha = format::sha256(token);
    let source = voice.provenance.get("source_sha256");
    let declared_token = source.and_then(|value| value["sn16k.tkn"].as_str())
        .or_else(|| voice.provenance["token"]["sha256"].as_str()).context("source token digest missing")?;
    let model_sha = source.and_then(|value| value["sn16k.bin"].as_str())
        .or_else(|| voice.provenance["model"]["sha256"].as_str()).context("source model digest missing")?;
    ensure!(token_sha == declared_token, "token bytes differ from the parsed voice source");
    let (question_ids, question_offsets, question_names) = question_index(voice)?;
    let (phone_ids, phone_flags, _) = phone_index(phones)?;
    let raw_names: Vec<_> = voice.tree_order.iter().map(|name| raw_name(name)).collect();
    let node_names: Vec<_> = voice.tree_order.iter().map(|name| preorder_name(name)).collect();
    let nodes: Vec<_> = voice.tree_order.iter().map(|name| preorder(voice, name)).collect();
    let mut inputs = Vec::with_capacity(31);
    let mut cursor = 0;
    for (index, name) in voice.tree_order.iter().enumerate() {
        let tree = &voice.trees[name];
        ensure!(tree.begin == cursor && tree.end <= 64 * 1024 * 1024, "source model regions are not contiguous and bounded");
        cursor = tree.end;
        inputs.push(Input { name: &raw_names[index], shape:vec![tree.leaf_nodes.len(), tree.kind.width()], data:Data::F64(&tree.records) });
        inputs.push(Input { name: &node_names[index], shape:vec![nodes[index].len()], data:Data::I64(&nodes[index]) });
    }
    inputs.extend([
        Input { name:"token.bytes",shape:vec![token.len()],data:Data::U8(token) },
        Input { name:"token.question_ids",shape:vec![question_ids.len()],data:Data::I64(&question_ids) },
        Input { name:"token.question_name_offsets",shape:vec![question_offsets.len()],data:Data::I64(&question_offsets) },
        Input { name:"token.question_name_utf8",shape:vec![question_names.len()],data:Data::U8(&question_names) },
        Input { name:"phone.table_utf8",shape:vec![phones.len()],data:Data::U8(phones) },
        Input { name:"phone.ids",shape:vec![phone_ids.len()],data:Data::I64(&phone_ids) },
        Input { name:"phone.flags",shape:vec![phone_ids.len(),2],data:Data::U8(&phone_flags) },
    ]);
    let metadata = BTreeMap::from([
        ("schema".into(), format::BANK_SCHEMA.into()), ("merge.schema_version".into(), "1".into()),
        ("source.token_sha256".into(), token_sha), ("source.model_sha256".into(), model_sha.into()),
        ("source.token_bytes".into(), token.len().to_string()), ("source.model_bytes".into(), cursor.to_string()),
        ("tree_order".into(), serde_json::to_string(&voice.tree_order)?), ("source.bin_distribution".into(), BIN_POLICY.into()),
    ]);
    tensors::encode(&metadata, &inputs)
}

fn entry<'a, 'b>(bank: &'a Tensors<'b>, name: &str, dtype: Dtype, shape: &[usize]) -> Result<&'a Tensor<'b>> {
    let tensor = bank.tensors.get(name).with_context(|| format!("merged bank lacks {name}"))?;
    ensure!(tensor.dtype == dtype && tensor.shape == shape, "merged bank tensor type/shape differs: {name}");
    Ok(tensor)
}
fn require_i64(bank: &Tensors<'_>, name: &str, expected: &[i64]) -> Result<()> {
    let tensor = entry(bank, name, Dtype::I64, &[expected.len()])?;
    ensure!(tensor.bytes.chunks_exact(8).zip(expected).all(|(bytes, value)| i64::from_le_bytes(bytes.try_into().expect("checked I64 width")) == *value),
        "merged token index differs from the actual token parse: {name}");
    Ok(())
}
fn require_u8(bank: &Tensors<'_>, name: &str, shape: &[usize], expected: &[u8]) -> Result<()> {
    ensure!(entry(bank, name, Dtype::U8, shape)?.bytes == expected, "merged byte tensor differs from its parsed source: {name}");
    Ok(())
}

fn decode_with_pins(bytes: &[u8], token_pin: &str, model_pin: &str, phone_pin: (&str, usize, &str)) -> Result<(VoiceModel, Value)> {
    let bank = tensors::decode(bytes)?;
    ensure!(bank.metadata.len() == METADATA_KEYS.len() && METADATA_KEYS.iter().all(|key| bank.metadata.contains_key(*key)),
        "merged bank metadata is incomplete or contains unknown fields");
    ensure!(bank.metadata["schema"] == format::BANK_SCHEMA && bank.metadata["merge.schema_version"] == "1"
        && bank.metadata["source.bin_distribution"] == BIN_POLICY, "unsupported merged bank schema or distribution declaration");
    ensure!(bank.metadata["source.token_sha256"] == token_pin && bank.metadata["source.model_sha256"] == model_pin,
        "merged bank metadata source pins disagree with the independently admitted source");
    let token_len: usize = bank.metadata["source.token_bytes"].parse()?;
    let model_len: usize = bank.metadata["source.model_bytes"].parse()?;
    ensure!((1..=2 * 1024 * 1024).contains(&token_len) && (8..=64 * 1024 * 1024).contains(&model_len)
        && token_len.to_string() == bank.metadata["source.token_bytes"] && model_len.to_string() == bank.metadata["source.model_bytes"],
        "merged source byte lengths are invalid");
    let token = entry(&bank, "token.bytes", Dtype::U8, &[token_len])?.bytes;
    ensure!(format::sha256(token) == token_pin, "actual token tensor SHA-256 differs from the independent source pin");
    let order: Vec<String> = serde_json::from_value(format::parse_json(bank.metadata["tree_order"].as_bytes())?)?;
    ensure!(order.len() == 12, "merged bank must contain twelve trees");
    let mut binary = Vec::with_capacity(model_len);
    let mut used = BTreeSet::new();
    for name in &order {
        let raw = raw_name(name);
        ensure!(used.insert(raw.clone()), "duplicate source tree in merged bank");
        let tensor = bank.tensors.get(&raw).context("source leaf tensor is missing")?;
        ensure!(tensor.dtype == Dtype::F64 && tensor.shape.len() == 2 && tensor.bytes.len() <= model_len - binary.len(),
            "invalid source F64 region");
        binary.extend_from_slice(tensor.bytes);
    }
    ensure!(binary.len() == model_len && format::sha256(&binary) == model_pin,
        "reconstructed model parameter stream differs from the independent audited digest");
    let mut voice = super::parse_bank_source(token, &binary)?;
    ensure!(voice.tree_order == order, "merged tree order differs from the actual token parse");
    for name in &voice.tree_order {
        let tree = &voice.trees[name];
        ensure!(entry(&bank, &raw_name(name), Dtype::F64, &[tree.leaf_nodes.len(),tree.kind.width()])?.bytes == &binary[tree.begin..tree.end],
            "merged leaf tensor differs from the actual source tree: {name}");
        let node_name = preorder_name(name);
        require_i64(&bank, &node_name, &preorder(&voice, name))?;
        used.insert(node_name);
    }
    let (ids, offsets, names) = question_index(&voice)?;
    require_i64(&bank, "token.question_ids", &ids)?;
    require_i64(&bank, "token.question_name_offsets", &offsets)?;
    require_u8(&bank, "token.question_name_utf8", &[names.len()], &names)?;
    let phones = entry(&bank, "phone.table_utf8", Dtype::U8, &[phone_pin.1])?.bytes;
    format::verify_base(phones, phone_pin)?;
    let (ids, flags, phone_table) = phone_index(phones)?;
    require_i64(&bank, "phone.ids", &ids)?;
    require_u8(&bank, "phone.flags", &[ids.len(),2], &flags)?;
    used.extend(["token.bytes","token.question_ids","token.question_name_offsets","token.question_name_utf8",
        "phone.table_utf8","phone.ids","phone.flags"].into_iter().map(str::to_owned));
    ensure!(used.len() == bank.tensors.len() && bank.tensors.keys().all(|name| used.contains(name)), "extra or mixed merged-bank tensor roles");
    voice.format = "verdant-voice-bank/1";
    voice.provenance = json!({"kind":"verdant-voice-merged-bank","bank_schema":format::BANK_SCHEMA,"bank_sha256":format::sha256(bytes),
        "source_sha256":{"sn16k.tkn":token_pin,"sn16k.bin":model_pin},"source_token_bytes":token_len,
        "token_storage":"bank.safetensors:token.bytes","source_tkn_bytes_preserved":true,
        "exact_leaf_bank_used":true,"all_tree_topology_preserved":true,"exact_binary_reconstruction":true,
        "requires_original_bin_at_runtime":false,"requires_standalone_token_at_runtime":false,
        "reconstructed_binary_is_original_installation_file":false,"reconstructed_binary_file_distributed":false,
        "original_context_reachability_proven":false,"neural_generalization_or_quality_proven":false});
    Ok((voice, phone_table))
}

pub(super) fn decode(bytes: &[u8]) -> Result<(VoiceModel, Value)> {
    decode_with_pins(bytes, KNOWN_TOKEN_SHA, KNOWN_MODEL_SHA, format::PHONE_TABLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TreeKind;

    fn fixture() -> (VoiceModel, Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut kinds = vec![("phone-dur-tree".to_owned(),TreeKind::PhoneDuration),("state-dur-tree".to_owned(),TreeKind::StateDuration)];
        kinds.extend((0..5).map(|i|(format!("cep{i}"),TreeKind::Spectrum)));
        kinds.extend((0..5).map(|i|(format!("pitch{i}"),TreeKind::Pitch)));
        let (mut binary,mut text,mut items) = (Vec::new(),String::new(),String::new());
        for (name,kind) in kinds {
            let start=text.len(); text.push_str("1\n"); let begin=binary.len();
            let mut values=vec![1_f64;kind.width()];
            if kind==TreeKind::Spectrum {
                values[0]=-0.; for(i,value)in values[16..40].iter_mut().enumerate(){*value=(i+1)as f64*0.12;}
                values[40]=-4.; values[122]=1e-250;
            } else if kind==TreeKind::Pitch {values[1]=220_f64.ln();}
            for value in values {binary.extend_from_slice(&value.to_le_bytes());}
            items.push_str(&format!("<{} name=\"{name}\" mbegin=\"{begin}\" mend=\"{}\">{start}-{}</{}>",kind.name(),binary.len(),text.len()-1,kind.name()));
        }
        let start=text.len();text.push_str("0.125\n");items.push_str(&format!("<pulse>{start}-{}</pulse>",text.len()-1));
        let token=format!("<voices><voice name=\"synthetic\"><token sample-rate=\"16000\" lsf_coef_order=\"24\" lsf_diff_order=\"16\"><qstable><queset name=\"C_Tone&lt;=2\">1</queset></qstable>{items}</token></voice><![CDATA[{text}]]></voices>").into_bytes();
        let voice=super::super::parse_bank_source(&token,&binary).unwrap();
        let phones=br#"[{"id":174,"fricative":false,"affricate":false}]"#.to_vec();
        (voice,token,binary,phones)
    }
    fn decode_test(bytes:&[u8],token:&[u8],binary:&[u8],phones:&[u8])->Result<VoiceModel>{
        Ok(decode_with_pins(bytes,&format::sha256(token),&format::sha256(binary),("test phone table",phones.len(),&format::sha256(phones)))?.0)
    }

    #[test]
    fn merged_voice_bank_preserves_token_f64_bits_and_parsed_indexes() {
        let(voice,token,binary,phones)=fixture();let bytes=encode(&voice,&token,&phones).unwrap();
        let decoded=decode_test(&bytes,&token,&binary,&phones).unwrap();
        super::super::current_package::same_planner(&voice,&decoded).unwrap();
        assert_eq!(decoded.trees["cep0"].records[0].to_bits(),(-0_f64).to_bits());
        assert_eq!(decoded.trees["cep0"].records[122].to_bits(),1e-250_f64.to_bits());
        assert_eq!(tensors::decode(&bytes).unwrap().tensors["token.bytes"].bytes,token);
        assert!(decode(&bytes).is_err(),"synthetic tokens cannot satisfy production source pins");
    }

    #[test]
    fn merged_voice_bank_preserves_target_and_reference_planning_arrays() {
        let(voice,token,binary,phones)=fixture();let bytes=encode(&voice,&token,&phones).unwrap();
        let decoded=decode_test(&bytes,&token,&binary,&phones).unwrap();
        let score=json!({"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":60,"gap_after_seconds":0.01},
            {"vowel":"i","duration_seconds":0.2,"midi_note":64}]});
        for plan in [crate::parameters::plan_voice_score,crate::parameters::plan_native_reference_score]{
            let before=plan(&voice,&score,None,&json!({})).unwrap();let after=plan(&decoded,&score,None,&json!({})).unwrap();
            assert_eq!((before.sample_rate,before.hop_samples,before.duration_frames),(after.sample_rate,after.hop_samples,after.duration_frames));
            assert_eq!(before.runs.len(),after.runs.len());
            for(left,right)in before.runs.iter().zip(&after.runs){
                assert_eq!((left.start_frame,left.frames),(right.start_frame,right.frames));
                assert_eq!(left.controls.f0_hz,right.controls.f0_hz);assert_eq!(left.controls.voiced,right.controls.voiced);
                assert_eq!(left.arrays.len(),right.arrays.len());
                for(name,array)in &left.arrays{let other=&right.arrays[name];assert_eq!((&array.shape,&array.dtype),(&other.shape,&other.dtype));
                    assert_eq!(array.values.len(),other.values.len());assert!(array.values.iter().zip(&other.values).all(|(a,b)|a.to_bits()==b.to_bits()),"{name}");}
            }
        }
    }

    #[test]
    fn merged_voice_bank_rejects_changed_raw_token_model_and_indexes() {
        let(voice,token,binary,phones)=fixture();let original=encode(&voice,&token,&phones).unwrap();
        for name in ["token.bytes","cep0_raw","token.question_ids","token.cep0.preorder","phone.ids"] {
            let decoded=tensors::decode(&original).unwrap();let tensor=&decoded.tensors[name];
            let offset=tensor.bytes.as_ptr()as usize-original.as_ptr()as usize;
            let mut changed=original.clone();changed[offset]^=1;
            assert!(decode_test(&changed,&token,&binary,&phones).is_err(),"{name}");
        }
    }
}
