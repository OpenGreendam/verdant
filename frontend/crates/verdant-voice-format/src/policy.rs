//! Optional artifact-specific installation policy admitted by a caller-supplied
//! reviewed digest. A policy packaged alongside a voice cannot authorize itself.
use std::{collections::{BTreeMap, BTreeSet}, fs, io::Read, path::{Path, PathBuf}};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use crate::{MAX_ARTIFACT_BYTES, NAME, PACKAGE_FILES, parse_json, read_manifest, sha256, valid_sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File { role:String,path:String,bytes:u64,sha256:String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence { path:String,bytes:u64,sha256:String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantedArtifact { role:String,sha256:String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant { id:String,rights_holder:std::option::Option<std::string::String>,release_authority:std::option::Option<std::string::String>,permission:String,
    artifact_specific:bool,evidence:Evidence,artifacts:Vec<GrantedArtifact> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy { schema:String,name:String,version:String,compatible_app_version:String,authorization_status:String,
    files:Vec<File>,grants:Vec<Grant> }

pub struct PolicyAdmission { pub policy_sha256:String,pub manifest_sha256:String }

fn portable(path:&str)->Result<()>{
    ensure!(!path.is_empty()&&!path.starts_with('/')&&!path.contains(['\\',':'])&&!path.chars().any(char::is_control)
        &&path.split('/').all(|part|!matches!(part,""|"."|"..")),"unsafe policy artifact path");Ok(())
}
fn regular(path:&Path)->Result<fs::File>{
    let metadata=fs::symlink_metadata(path)?;
    ensure!(metadata.file_type().is_file()&&!metadata.file_type().is_symlink(),"policy input must be a regular file");
    Ok(fs::File::open(path)?)
}
fn inventory(root:&Path)->Result<BTreeSet<String>>{
    let mut pending=vec![root.to_path_buf()];let mut files=BTreeSet::new();
    while let Some(directory)=pending.pop(){
        ensure!(fs::symlink_metadata(&directory)?.file_type().is_dir(),"policy bundle directory is not regular");
        for entry in fs::read_dir(&directory)?{
            let entry=entry?;let kind=entry.file_type()?;let path=entry.path();
            if kind.is_dir(){ensure!(pending.len()<1024,"policy directory inventory exceeds its bound");pending.push(path);}
            else{ensure!(kind.is_file()&&files.len()<1024,"policy bundle contains a link/special file or excessive inventory");
                let name=path.strip_prefix(root)?.to_str().context("policy bundle member must be UTF-8")?.replace('\\',"/");
                portable(&name)?;ensure!(files.insert(name),"duplicate policy bundle member");}
        }
    }Ok(files)
}
fn digest_file(path:&Path,expected:u64)->Result<String>{
    let mut file=regular(path)?;ensure!(file.metadata()?.len()==expected,"policy artifact byte count differs");
    let mut hash=Sha256::new();let mut bytes=[0;64*1024];let mut total=0u64;
    loop{let count=file.read(&mut bytes)?;if count==0{break;}total+=count as u64;
        ensure!(total<=expected,"policy artifact grew during verification");hash.update(&bytes[..count]);}
    ensure!(total==expected,"policy artifact shortened during verification");Ok(hex::encode(hash.finalize()))
}

pub fn verify(directory:&Path,policy_path:&Path,reviewed_sha256:&str)->Result<PolicyAdmission>{
    ensure!(valid_sha256(reviewed_sha256),"reviewed artifact-policy SHA-256 is required");
    let file=regular(policy_path)?;ensure!(file.metadata()?.len()<=8*1024*1024,"artifact policy exceeds its JSON bound");
    let mut bytes=Vec::new();file.take(8*1024*1024+1).read_to_end(&mut bytes)?;
    ensure!(bytes.len()<=8*1024*1024&&sha256(&bytes)==reviewed_sha256,"artifact policy differs from the independently reviewed digest");
    let policy:Policy=serde_json::from_value(parse_json(&bytes)?)?;
    ensure!(policy.schema=="verdant-release-artifact-policy/1"&&policy.name==NAME&&policy.authorization_status=="granted"
        &&policy.version==policy.compatible_app_version&&semver::Version::parse(&policy.version).is_ok(),
        "artifact policy lacks compatible granted redistribution admission");
    ensure!((1..=1024).contains(&policy.files.len())&&!policy.grants.is_empty(),"invalid artifact policy inventory");
    let mut roles=BTreeMap::new();let mut paths=BTreeSet::new();let(mut licenses,mut notices)=(0,0);
    for file in &policy.files{
        portable(&file.path)?;
        ensure!((1..=MAX_ARTIFACT_BYTES as u64).contains(&file.bytes)&&valid_sha256(&file.sha256)
            &&roles.insert(file.role.as_str(),file).is_none()&&paths.insert(file.path.clone()),"invalid or repeated policy artifact");
        if let Some((_,name))=PACKAGE_FILES.iter().find(|(role,_)|*role==file.role){ensure!(*name==file.path,"policy core artifact uses a noncanonical path");}
        else{let(kind,id)=file.role.split_once(':').context("policy extras must identify license, notice or authorization evidence")?;
            ensure!(!id.is_empty()&&matches!(kind,"license"|"notice"|"authorization")
                &&matches!(Path::new(&file.path).extension().and_then(|value|value.to_str()),Some("txt"|"md"|"json")),"unapproved policy extra role");
            licenses+=usize::from(kind=="license");notices+=usize::from(kind=="notice");}
    }
    ensure!(licenses>0&&notices>0,"policy requires licenses and attribution notices");
    for(role,_)in PACKAGE_FILES{ensure!(roles.contains_key(role),"policy omits required runtime role {role}");}
    let(mut grant_ids,mut covered)=(BTreeSet::new(),BTreeSet::new());
    for grant in &policy.grants{
        let authority=match(&grant.rights_holder,&grant.release_authority){(Some(value),None)|(None,Some(value))=>!value.trim().is_empty(),_=>false};
        ensure!(authority&&!grant.id.trim().is_empty()&&grant_ids.insert(grant.id.as_str())&&grant.permission=="redistribute-bundled-runtime"
            &&grant.artifact_specific&&!grant.artifacts.is_empty(),"policy grant lacks identified artifact-specific redistribution authority");
        let evidence=policy.files.iter().find(|file|file.path==grant.evidence.path).context("policy grant evidence is not packaged")?;
        ensure!(evidence.role.starts_with("authorization:")&&evidence.bytes==grant.evidence.bytes&&evidence.sha256==grant.evidence.sha256,
            "policy grant evidence does not match its exact packaged authorization record");
        let mut unique=BTreeSet::new();for artifact in &grant.artifacts{
            let file=roles.get(artifact.role.as_str()).context("policy grant names an undeclared role")?;
            ensure!(file.sha256==artifact.sha256&&unique.insert(artifact.role.as_str()),"policy grant digest/role coverage differs");
            covered.insert(artifact.role.as_str());}
    }
    for(role,_)in PACKAGE_FILES{ensure!(covered.contains(role),"policy lacks a grant for runtime role {role}");}
    let(manifest,manifest_bytes)=read_manifest(directory)?;
    ensure!(manifest.version==policy.version,"policy and package versions differ");
    let manifest_sha256=sha256(&manifest_bytes);
    ensure!(roles["verdant_voice_manifest"].sha256==manifest_sha256&&roles["verdant_voice_manifest"].bytes==manifest_bytes.len()as u64,
        "policy root manifest binding differs");
    for(role,artifact)in &manifest.artifacts{
        let file=roles.get(role.as_str()).context("manifest artifact is not authorized")?;
        ensure!(file.path==artifact.file&&file.bytes==artifact.bytes&&file.sha256==artifact.sha256,"policy artifact and manifest binding differ");
    }
    for(kind,record)in &manifest.records{
        let category=match kind.as_str(){"conversion_notice"=>"notice","source_use_evidence"=>"authorization",kind=>kind};
        let file=policy.files.iter().find(|file|file.path==record.file&&file.role.split_once(':').is_some_and(|(kind,_)|kind==category))
            .context("manifest record lacks a matching policy evidence role")?;
        ensure!(file.bytes==record.bytes&&file.sha256==record.sha256,"policy record and manifest binding differ");
    }
    let root=directory.canonicalize()?;ensure!(inventory(&root)?==paths,"policy bundle contains missing or undeclared members");
    for file in &policy.files{
        if file.role=="verdant_voice_manifest"{continue;}
        let path:PathBuf=root.join(&file.path);
        ensure!(digest_file(&path,file.bytes)?==file.sha256,"policy artifact changed: {}",file.role);
    }
    ensure!(digest_file(policy_path,bytes.len()as u64)?==reviewed_sha256,"artifact policy changed during verification");
    Ok(PolicyAdmission{policy_sha256:reviewed_sha256.into(),manifest_sha256})
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::{Artifact,Manifest,RECORD_FILES,SCHEMA,STRATEGY};
    use serde_json::{Value,json};
    use std::sync::atomic::{AtomicU64,Ordering};

    struct Fixture(PathBuf);
    impl Fixture{
        fn new()->Self{static NEXT:AtomicU64=AtomicU64::new(0);
            let root=std::env::temp_dir().join(format!("verdant-reviewed-policy-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();Self(root)}
    }
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}

    // Marker bytes test policy/file consistency only. They are deliberately
    // not synthesis models or real source-rights evidence.
    fn fixture(root:&Path)->(PathBuf,PathBuf,Value){
        let package=root.join("package");fs::create_dir(&package).unwrap();
        let mut manifest=Manifest{schema:SCHEMA.into(),name:NAME.into(),version:"0.1.0".into(),strategy:STRATEGY.into(),
            artifacts:BTreeMap::new(),records:BTreeMap::new()};
        let mut files=Vec::new();
        for &(role,name)in &PACKAGE_FILES[1..]{let bytes=format!("policy fixture: {role}").into_bytes();fs::write(package.join(name),&bytes).unwrap();
            let artifact=Artifact{file:name.into(),bytes:bytes.len()as u64,sha256:sha256(&bytes)};
            files.push(json!({"role":role,"path":name,"bytes":artifact.bytes,"sha256":artifact.sha256}));manifest.artifacts.insert(role.into(),artifact);}
        for &(role,name)in &RECORD_FILES[..3]{let bytes=format!("synthetic {role}; no actual license or redistribution grant").into_bytes();fs::write(package.join(name),&bytes).unwrap();
            let artifact=Artifact{file:name.into(),bytes:bytes.len()as u64,sha256:sha256(&bytes)};
            files.push(json!({"role":format!("{role}:fixture"),"path":name,"bytes":artifact.bytes,"sha256":artifact.sha256}));manifest.records.insert(role.into(),artifact);}
        let bytes=serde_json::to_vec(&manifest).unwrap();fs::write(package.join("manifest.json"),&bytes).unwrap();
        files.push(json!({"role":"verdant_voice_manifest","path":"manifest.json","bytes":bytes.len(),"sha256":sha256(&bytes)}));
        let grants=files.iter().filter(|file|PACKAGE_FILES.iter().any(|(role,_)|file["role"]==*role))
            .map(|file|json!({"role":file["role"],"sha256":file["sha256"]})).collect::<Vec<_>>();
        let authorization=&manifest.records["authorization"];
        let value=json!({"schema":"verdant-release-artifact-policy/1","name":NAME,"version":"0.1.0","compatible_app_version":"0.1.0",
            "authorization_status":"granted","files":files,"grants":[{"id":"synthetic-policy-structure","rights_holder":"synthetic fixture only",
                "permission":"redistribute-bundled-runtime","artifact_specific":true,
                "evidence":{"path":"AUTHORIZATION.json","bytes":authorization.bytes,"sha256":authorization.sha256},"artifacts":grants}]});
        (package,root.join("reviewed-policy.json"),value)
    }

    #[test]
    fn reviewed_policy_binds_merged_bank_manifest_records_and_complete_grants(){
        let scratch=Fixture::new();let(package,path,mut policy)=fixture(&scratch.0);
        let bytes=serde_json::to_vec(&policy).unwrap();fs::write(&path,&bytes).unwrap();
        let admission=verify(&package,&path,&sha256(&bytes)).unwrap();
        assert_eq!(admission.manifest_sha256,sha256(&fs::read(package.join("manifest.json")).unwrap()));
        let original_bank=fs::read(package.join("bank.safetensors")).unwrap();fs::write(package.join("bank.safetensors"),b"changed").unwrap();
        assert!(verify(&package,&path,&sha256(&bytes)).is_err());fs::write(package.join("bank.safetensors"),original_bank).unwrap();
        policy["grants"][0]["artifacts"].as_array_mut().unwrap().retain(|artifact|artifact["role"]!="bank_tensors");
        let bytes=serde_json::to_vec(&policy).unwrap();fs::write(&path,&bytes).unwrap();
        assert!(verify(&package,&path,&sha256(&bytes)).is_err(),"even an independently pinned declaration needs complete bank grant coverage");
    }

    #[test]
    fn reviewed_policy_digest_is_required_before_reading_package(){
        assert!(verify(Path::new("missing"),Path::new("missing-policy"),"self-declared").is_err());
        for name in ["../file","/file","a\\b","a//b","a/./b","C:file"]{assert!(portable(name).is_err());}
    }
}
