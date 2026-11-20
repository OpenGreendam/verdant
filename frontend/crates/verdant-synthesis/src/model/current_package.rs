//! Current Verdant Voice package loading and explicit source conversion.
//! The native ONNX graphs remain byte-identical; token bytes and their parsed
//! indexes share bank.safetensors with the original F64 parameter records.
use std::{collections::BTreeMap, fs::{self, OpenOptions}, io::{Read, Write}, path::{Path,PathBuf}};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use verdant_voice_format::{self as format, Artifact, Manifest, PACKAGE_FILES};
use super::{Node, VoiceModel, merged_bank, source_pins::{KNOWN_TOKEN_SHA, KNOWN_MODEL_SHA}, voice_package};

const MAX_RECORD: usize = 8 * 1024 * 1024;
const MAX_MODEL: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub struct VoicePackage {
    pub voice: VoiceModel,
    pub manifest: Value,
    pub contracts: Value,
    pub voice_version: String,
    pub runtime_manifest_sha256: String,
    pub hashes: BTreeMap<String, String>,
    pub(crate) context_graph: Vec<u8>,
    pub(crate) local_graph: Vec<u8>,
    pub(crate) decoder_graph: Vec<u8>,
    pub(crate) phone_table: Value,
    pub(crate) directory: PathBuf,
}
impl VoicePackage {
    pub fn source_manifest_sha256(&self) -> &str { &self.hashes["verdant_voice_manifest"] }
}

fn preorder(voice: &VoiceModel, name: &str) -> Vec<i64> {
    let tree=&voice.trees[name];
    tree.nodes.iter().map(|node| match *node {
        Node::Branch{question_id,..}=>-question_id,
        Node::Leaf(index)=>(index*tree.kind.width()+1)as i64,
    }).collect()
}

pub(super) fn same_planner(expected: &VoiceModel, actual: &VoiceModel) -> Result<()> {
    ensure!(expected.sample_rate==actual.sample_rate && expected.questions==actual.questions
        && expected.tree_order==actual.tree_order && expected.pulse==actual.pulse && expected.trees.len()==actual.trees.len(),
        "merged voice clock, questions, tree order or pulse audit differs from the source");
    for name in &expected.tree_order {
        let left=&expected.trees[name];let right=actual.trees.get(name).context("merged bank lost a source tree")?;
        ensure!(left.kind==right.kind && left.begin==right.begin && left.end==right.end && left.parents==right.parents
            && left.leaf_nodes==right.leaf_nodes && preorder(expected,name)==preorder(actual,name)
            && left.records.len()==right.records.len()
            && left.records.iter().zip(&right.records).all(|(a,b)|a.to_bits()==b.to_bits()),
            "merged voice topology or exact F64 records differ from the source: {name}");
    }
    Ok(())
}

fn verified_source_pair(token: &Path, model: &Path) -> Result<VoiceModel> {
    let token_metadata=fs::metadata(token)?;let model_metadata=fs::metadata(model)?;
    ensure!(token_metadata.is_file()&&model_metadata.is_file()&&token_metadata.len()>0&&model_metadata.len()>0
        &&token_metadata.len()<=2*1024*1024&&model_metadata.len()<=MAX_MODEL as u64,
        "source verification input exceeds its bounded contract");
    let voice=super::load_raw_voice(token,model)?;
    ensure!(voice.provenance["token"]["sha256"]==KNOWN_TOKEN_SHA && voice.provenance["model"]["sha256"]==KNOWN_MODEL_SHA,
        "source token/model bytes do not match both independently audited fingerprints");
    Ok(voice)
}

/// Check a user-held pair against the merged data. Equal bytes authenticate
/// source data; they do not establish that a reconstructed file is an original
/// installation artifact or grant redistribution permission.
pub fn verify_voice_source(package:&VoicePackage,token:&Path,model:&Path)->Result<()> {
    let source=verified_source_pair(token,model)?;
    ensure!(source.provenance["token"]["bytes"]==package.voice.provenance["source_token_bytes"],
        "merged token length differs from the supplied verification input");
    same_planner(&source,&package.voice)
}

pub fn compiled_voice_bank_sha256()->Option<&'static str>{option_env!("VERDANT_VOICE_BANK_SHA256")}

/// Ordinary rendering requires a bank identity compiled into the runtime.
pub fn load_voice_package(directory:&Path)->Result<VoicePackage>{
    let pin=compiled_voice_bank_sha256().context("Verdant Voice requires a compiled VERDANT_VOICE_BANK_SHA256 or independently verified artifact policy")?;
    ensure!(format::valid_sha256(pin),"compiled voice bank pin is invalid");
    let mut package=load_package_data(directory)?;
    ensure!(package.hashes["bank_tensors"]==pin,"merged bank differs from the independently compiled identity");
    package.voice.provenance["source_authentication"]="compiled-bank-sha256-and-audited-source-pins".into();
    Ok(package)
}

/// The root manifest digest must come from an independently authenticated
/// policy, never from the package currently being admitted. It binds the bank,
/// all three unchanged graphs, version, and attribution records together.
pub fn load_voice_package_with_manifest_pin(directory:&Path,trusted_manifest_sha256:&str)->Result<VoicePackage>{
    ensure!(format::valid_sha256(trusted_manifest_sha256),"independent manifest pin is invalid");
    let mut package=load_package_data(directory)?;
    ensure!(package.source_manifest_sha256()==trusted_manifest_sha256,"voice manifest differs from independent artifact policy");
    package.voice.provenance["source_authentication"]="independently-pinned-manifest-and-audited-source-pins".into();
    Ok(package)
}

/// Explicit source comparison is available to conversion/import tools without
/// a compiled bank pin. Normal render callers must use a pinned loader above.
pub fn load_voice_package_for_source_import(directory:&Path,token:&Path,model:&Path)->Result<VoicePackage>{
    let mut package=load_package_data(directory)?;
    verify_voice_source(&package,token,model)?;
    package.voice.provenance["source_authentication"]="supplied-source-pair-compared-with-merged-bank".into();
    Ok(package)
}

fn load_package_data(directory:&Path)->Result<VoicePackage>{
    let(declaration,manifest_bytes)=format::read_manifest(directory)?;
    for name in ["source.tkn","voice-source.json","cfinal.json","bank.json","topology.json","controller.onnx","decoder.onnx","rights.json"]{
        match fs::symlink_metadata(directory.join(name)){
            Ok(_)=>anyhow::bail!("retired standalone source artifact is not allowed in a current runtime package: {name}"),
            Err(error)if error.kind()==std::io::ErrorKind::NotFound=>{},Err(error)=>return Err(error.into()),
        }
    }
    for record in declaration.records.values(){format::read_artifact(directory,record)?;}
    let bank=format::read_artifact(directory,&declaration.artifacts["bank_tensors"])?;
    let(mut voice,phone_table)=merged_bank::decode(&bank)?;
    let context_graph=format::read_artifact(directory,&declaration.artifacts["verdant_voice_context_controller"])?;
    format::verify_base(&context_graph,format::CONTEXT_CONTROLLER)?;
    let local_graph=format::read_artifact(directory,&declaration.artifacts["verdant_voice_local_controller"])?;
    let decoder_graph=format::read_artifact(directory,&declaration.artifacts["verdant_voice_decoder"])?;
    format::verify_base(&local_graph,format::LOCAL_CONTROLLER)?;format::verify_base(&decoder_graph,format::DECODER)?;
    let runtime_manifest_sha256=format::sha256(&manifest_bytes);
    let mut hashes=BTreeMap::from([("verdant_voice_manifest".into(),runtime_manifest_sha256.clone())]);
    hashes.extend(declaration.artifacts.iter().map(|(role,artifact)|(role.clone(),artifact.sha256.clone())));
    voice.provenance["artifact_hashes"]=json!(hashes);
    voice.provenance["voice_manifest_sha256"]=runtime_manifest_sha256.clone().into();
    voice.provenance["graphs_distributed"]=true.into();voice.provenance["onnx_artifacts_verified"]=true.into();
    voice.provenance["onnx_graph_abi_verified"]=false.into();voice.provenance["rights_attestation_is_legal_determination"]=false.into();
    let contracts=json!({"schema":"verdant-voice-conditions/1","neighbor_encoding":crate::voice::RESOLVED_NEIGHBOR_ENCODING,
        "sample_rate":16000,"hop_samples":80,"output_sample_rate":48000,"output_hop_samples":240});
    let manifest=serde_json::to_value(&declaration)?;
    Ok(VoicePackage{voice,manifest,contracts,voice_version:declaration.version,runtime_manifest_sha256,hashes,
        context_graph,local_graph,decoder_graph,phone_table,directory:directory.canonicalize()?})
}

fn read_legacy(directory:&Path,name:&str,maximum:usize)->Result<Vec<u8>>{
    let path=directory.join(name);let metadata=fs::symlink_metadata(&path)?;
    ensure!(metadata.file_type().is_file()&&!metadata.file_type().is_symlink()&&metadata.len()<=maximum as u64,
        "conversion input must be a bounded regular file: {name}");
    let mut bytes=Vec::with_capacity(metadata.len()as usize);fs::File::open(path)?.take(maximum as u64+1).read_to_end(&mut bytes)?;
    ensure!(bytes.len()<=maximum,"conversion input exceeds its bound");Ok(bytes)
}
fn descriptor(name:&str,bytes:&[u8])->Artifact{Artifact{file:name.into(),bytes:bytes.len()as u64,sha256:format::sha256(bytes)}}
fn write_new(directory:&Path,name:&str,bytes:&[u8])->Result<()>{
    let mut file=OpenOptions::new().write(true).create_new(true).open(directory.join(name))?;file.write_all(bytes)?;file.sync_all()?;Ok(())
}
fn optional_record(directory:&Path,names:&[&str])->Result<Option<Vec<u8>>>{
    let mut record=None;
    for name in names{match fs::symlink_metadata(directory.join(name)){
        Ok(_)=>{let bytes=read_legacy(directory,name,MAX_RECORD)?;ensure!(record.as_ref().is_none_or(|prior|prior==&bytes),
            "multiple distinct source records require explicit reconciliation before conversion");record=Some(bytes);},
        Err(error)if error.kind()==std::io::ErrorKind::NotFound=>{},Err(error)=>return Err(error.into()),
    }}Ok(record)
}

/// Produce a current package from an explicitly selected historical package
/// and a supplied token/model verification pair. A model stream reconstructed
/// from retained F64 tensors is valid comparison material after its fixed hash
/// check, but is not represented as an original installation file or shipped.
pub fn convert_voice_package(source:&Path,destination:&Path,token_input:&Path,model_verification_input:&Path)->Result<Value>{
    let source=source.canonicalize()?;
    let parent=destination.parent().filter(|parent|!parent.as_os_str().is_empty()).unwrap_or_else(||Path::new("."));
    let destination_root=parent.canonicalize()?.join(destination.file_name().context("conversion destination needs a directory name")?);
    let destination=destination_root.as_path();
    ensure!(!destination.starts_with(&source)&&!destination.try_exists()?,"conversion destination must be absent and outside the unchanged source");
    let legacy=voice_package::load_legacy_source_package(&source)?;
    let supplied=verified_source_pair(token_input,model_verification_input)?;
    same_planner(&supplied,&legacy.voice)?;
    let mut layout=None;
    for(name,context,decoder)in[("verdant-voice","context-controller.onnx","decoder-48k.onnx"),
        ("r14","D-controller.onnx","r10-guard-decoder48.onnx"),("r13b","D-controller.onnx","r10-guard-decoder48.onnx")]{
        if source.join(name).try_exists()?{ensure!(layout.is_none(),"conversion source contains mixed native layouts");
            let directory=source.join(name);ensure!(fs::symlink_metadata(&directory)?.file_type().is_dir(),"native layout must be a regular directory");
            layout=Some((directory,context,decoder));}
    }
    let(runtime,context_name,decoder_name)=layout.context("source lacks the current native graph set")?;
    if runtime.file_name().is_some_and(|name|name=="verdant-voice"){crate::voice::manifest(&runtime,&legacy.hashes["voice_source_manifest"])?;}
    let context=read_legacy(&runtime,context_name,format::MAX_ARTIFACT_BYTES)?;
    let local=read_legacy(&runtime,"local-controller.onnx",format::MAX_ARTIFACT_BYTES)?;
    let decoder=read_legacy(&runtime,decoder_name,format::MAX_ARTIFACT_BYTES)?;
    let phones=read_legacy(&runtime,"native-phone-table.json",MAX_RECORD)?;
    format::verify_base(&context,format::CONTEXT_CONTROLLER)?;format::verify_base(&local,format::LOCAL_CONTROLLER)?;
    format::verify_base(&decoder,format::DECODER)?;format::verify_base(&phones,format::PHONE_TABLE)?;
    let token=read_legacy(&source,"source.tkn",2*1024*1024)?;
    ensure!(format::sha256(&token)==KNOWN_TOKEN_SHA,"source token changed after admission");
    let bank=merged_bank::encode(&legacy.voice,&token,&phones)?;
    let(roundtrip,_)=merged_bank::decode(&bank)?;same_planner(&legacy.voice,&roundtrip)?;same_planner(&supplied,&roundtrip)?;
    let bank_sha256=format::sha256(&bank);
    let mut assets=BTreeMap::from([("bank.safetensors",bank),("context-controller.onnx",context),
        ("local-controller.onnx",local),("decoder-48k.onnx",decoder)]);
    let local_evidence=read_legacy(&source,"rights.json",MAX_RECORD)?;
    ensure!(format::sha256(&local_evidence)==legacy.hashes["source_rights"],"source-use evidence changed during conversion");
    let supplied_authorization=source.join("AUTHORIZATION.json").try_exists()?;
    let authorization=if supplied_authorization{assets.insert("SOURCE-USE-EVIDENCE.json",local_evidence);
        read_legacy(&source,"AUTHORIZATION.json",MAX_RECORD)?}else{local_evidence};
    assets.insert("AUTHORIZATION.json",authorization);
    let evidence_scope=if supplied_authorization{
        "AUTHORIZATION.json preserves supplied evidence verbatim. SOURCE-USE-EVIDENCE.json preserves the original local-use attestation, NOT a redistribution grant."
    }else{"AUTHORIZATION.json preserves the original local-use attestation verbatim. It is local-use evidence, NOT redistribution authorization."};
    let notice=format!("Verdant Voice {}\n\nThis package retains voice data derived from the supplied sn16k token/model and unchanged native learned graphs. The merged bank includes exact token bytes, parsed indexes and F64 model records; it does not erase their historical source or ownership.\nOriginal sn16k.tkn SHA-256: {}\nOriginal sn16k.bin SHA-256: {}\nHistorical package manifest SHA-256: {}\n\nA reconstructed binary is verification input, not an original installation file. No standalone TKN or BIN file is distributed. Hash equality checks bytes and does not establish possession of an original installation artifact.\n\n{} Conversion grants no new rights and never replaces applicable third-party attribution, licenses or permission evidence.\n",
        env!("CARGO_PKG_VERSION"),KNOWN_TOKEN_SHA,KNOWN_MODEL_SHA,legacy.hashes["voice_source_manifest"],evidence_scope).into_bytes();
    if let Some(original)=optional_record(&source,&["NOTICE.txt","NOTICE","NOTICE.md"])?{assets.insert("NOTICE.txt",original);assets.insert("CONVERSION.txt",notice);}
    else{assets.insert("NOTICE.txt",notice);}
    if let Some(license)=optional_record(&source,&["LICENSE.txt","LICENSE","LICENSE.md","COPYING"])?{assets.insert("LICENSE.txt",license);}
    let declaration=Manifest{schema:format::SCHEMA.into(),name:format::NAME.into(),version:env!("CARGO_PKG_VERSION").into(),strategy:format::STRATEGY.into(),
        artifacts:PACKAGE_FILES[1..].iter().map(|(role,name)|((*role).into(),descriptor(name,&assets[name]))).collect(),
        records:format::RECORD_FILES.iter().filter_map(|(role,name)|assets.get(name).map(|bytes|((*role).into(),descriptor(name,bytes)))).collect()};
    declaration.validate()?;
    let after=voice_package::load_legacy_source_package(&source)?;ensure!(after.hashes==legacy.hashes,"legacy source changed during conversion");
    same_planner(&legacy.voice,&after.voice)?;same_planner(&supplied,&verified_source_pair(token_input,model_verification_input)?)?;
    fs::create_dir(destination)?;for(name,bytes)in &assets{write_new(destination,name,bytes)?;}
    let mut manifest_bytes=serde_json::to_vec_pretty(&declaration)?;manifest_bytes.push(b'\n');write_new(destination,"manifest.json",&manifest_bytes)?;
    let readback=load_package_data(destination)?;same_planner(&supplied,&readback.voice)?;
    let members:BTreeMap<_,_>=assets.iter().map(|(name,bytes)|((*name).to_owned(),descriptor(name,bytes)))
        .chain(std::iter::once(("manifest.json".into(),descriptor("manifest.json",&manifest_bytes)))).collect();
    Ok(json!({"schema":"verdant-voice-conversion/2","name":format::NAME,"version":env!("CARGO_PKG_VERSION"),
        "source_directory":source,"source_manifest_sha256":legacy.hashes["voice_source_manifest"],"destination":destination,"members":members,
        "source_sha256":{"sn16k.tkn":KNOWN_TOKEN_SHA,"sn16k.bin":KNOWN_MODEL_SHA},"bank_sha256":bank_sha256,"bank_schema":format::BANK_SCHEMA,
        "token_verification_input":token_input,"model_verification_input":model_verification_input,
        "merged_tensor_roundtrip_compared":true,"token_raw_bytes_compared":true,"f64_bits_and_topology_compared":true,
        "onnx_model_files_modified":false,"audio_equivalence_executed":false,
        "reconstructed_binary_is_original_installation_file":false,"model_verification_input_is_distributed":false,
        "authorization_record_scope":if supplied_authorization{"preserved-supplied-evidence"}else{"local-source-use-evidence-not-redistribution-grant"},
        "redistribution_permission_granted_by_conversion":false}))
}
