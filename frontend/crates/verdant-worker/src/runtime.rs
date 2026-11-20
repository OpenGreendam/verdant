use std::{env, path::{Path,PathBuf}};
use anyhow::{Context,Result,ensure};
use serde::Deserialize;
use verdant_synthesis::model::{self,VoiceModel};
use crate::Config;
pub fn cache_dir()->PathBuf{verdant_backend::platform::cache_dir()}
pub fn absolute(path:&Path)->Result<PathBuf>{
    let path=if let Ok(relative)=path.strip_prefix("~"){verdant_backend::platform::home_dir().context("cannot expand ~ without a user home directory; pass an absolute path")?.join(relative)}else{path.to_owned()};
    Ok(if path.is_absolute(){path}else{env::current_dir()?.join(path)})
}
pub fn voice_runtime(config: &Config) -> Result<&Path> {
    ensure!(config.original.is_none(), "verdant-voice runtime requires a selected voice package");
    config.voice_package.as_deref().context("voice package directory is missing")
}
pub fn validate_voice_package(config: &Config, package: &model::VoicePackage, options: &serde_json::Value) -> Result<()> {
    verdant_synthesis::voice::validate_package(voice_runtime(config)?, package, options)
}
fn original_hashes(config: &Config) -> Result<std::collections::BTreeMap<String, String>> {
    let (token, model) = config.original.as_ref().context("original synthesis requires an unlocked raw pair")?;
    let mut voice = verdant_backend::wire::VoiceEntry { available: true, ..Default::default() };
    let mut hashes = std::collections::BTreeMap::new();
    for (role, path) in [("token", token), ("model", model)] {
        let hash = crate::artifacts::sha(path)?;
        voice.files.insert(role.into(), verdant_backend::wire::FileStatus { available: true, sha256: Some(hash.clone()) });
        hashes.insert(role.into(), hash);
    }
    ensure!(verdant_backend::voices::is_desktop_original(&voice),
        "original synthesis requires the exact original raw token/model bytes");
    Ok(hashes)
}

pub fn load_original(config: &Config) -> Result<VoiceModel> {
    let before = original_hashes(config)?;
    let (token, model) = config.original.as_ref().context("original synthesis requires an unlocked raw pair")?;
    let voice = model::load_raw_voice(token, model)?;
    let sources = voice_sources(&voice)?;
    ensure!(sources.get("sn16k.tkn") == before.get("token") && sources.get("sn16k.bin") == before.get("model")
        && original_hashes(config)? == before, "original bytes changed during model loading");
    Ok(voice)
}

pub fn onnxruntime_path(config:&Config)->Result<PathBuf>{
    if let Some(path)=&config.onnxruntime_library{ensure!(path.is_file(),"ONNX Runtime library missing: {}",path.display());return Ok(path.clone());}
    let exe=env::current_exe()?;let dir=exe.parent().context("worker executable directory")?;
    let name=if cfg!(windows){"onnxruntime.dll"}else if cfg!(target_os="macos"){"libonnxruntime.1.23.2.dylib"}else{"libonnxruntime.so.1.23.2"};
    let packaged=if cfg!(target_os="macos"){dir.join("../Frameworks").join(name)}else if cfg!(windows){dir.join(name)}else{dir.join("../lib").join(name)};
    for path in [packaged,dir.join(name)]{if path.is_file(){return Ok(path.canonicalize()?);}}
    anyhow::bail!("ONNX Runtime 1.23.2 library missing ({name}); install the packaged native library or set VERDANT_ONNXRUNTIME_LIBRARY")
}
pub fn asset_hashes(config: &Config) -> Result<std::collections::BTreeMap<String, String>> {
    if config.original.is_some() { return original_hashes(config); }
    let mut result = if config.bundled_voice.is_some() { crate::bundled_voice::asset_hashes(config)? }
        else { verdant_backend::voices::package_hashes(voice_runtime(config)?)? };
    if let Ok(path) = onnxruntime_path(config) { result.insert("onnxruntime".into(), crate::artifacts::sha(&path)?); }
    if let Some(registered) = config.selection.as_ref().and_then(|selection| selection.get("source_sha256")).and_then(serde_json::Value::as_object) {
        for (key, _) in verdant_backend::voices::PACKAGE_FILES {
            ensure!(registered.get(*key).and_then(serde_json::Value::as_str) == result.get(*key).map(String::as_str),
                "voice input differs from the registered admission: {key}");
        }
    }
    Ok(result)
}
pub fn voice_sources(voice:&VoiceModel)->Result<std::collections::BTreeMap<String,String>>{
    if let Some(object)=voice.provenance.get("source_sha256").and_then(serde_json::Value::as_object){
        return object.iter().map(|(k,v)|Ok((k.clone(),v.as_str().context("source SHA-256 must be string")?.to_owned()))).collect();
    }
    let mut result=std::collections::BTreeMap::new();
    for(key,name)in [("token","sn16k.tkn"),("model","sn16k.bin")]{result.insert(name.into(),voice.provenance[key]["sha256"].as_str().context("raw model provenance lacks source SHA-256")?.to_owned());}Ok(result)
}
pub fn admitted_voice_package(config:&Config)->Result<model::VoicePackage>{
    let package=load_voice_package(config)?;
    validate_voice_receipt(config,&package)?;
    Ok(package)
}
pub fn load_voice_package(config: &Config) -> Result<model::VoicePackage> {
    if config.bundled_voice.is_some() {
        crate::bundled_voice::load_package(config)
    } else {
        model::load_voice_package(&voice_runtime(config)?)
    }
}

pub fn validate_voice_receipt(config:&Config,package:&model::VoicePackage)->Result<()>{
    if config.bundled_voice.is_some(){return crate::bundled_voice::validate_selection(config,package);}
    let selection=config.selection.as_ref().context("versioned voice requires a registered selection")?;
    let originals = voice_sources(&package.voice)?;
    let source_receipt = selection.get("source_receipt").filter(|receipt| !receipt.is_null());
    let package_receipt = selection.get("package_receipt").filter(|receipt| !receipt.is_null());
    let version = &package.voice_version;
    match (source_receipt, package_receipt) {
        (Some(receipt), None) => {
            let receipt = verdant_backend::voices::SourceReceipt::deserialize(receipt)?;
            receipt.validate_for(package.source_manifest_sha256(), &originals)?;
        }
        (None, Some(receipt)) => {
            let receipt = verdant_backend::voices::PackageReceipt::deserialize(receipt)?;
            let bank_sha = package.hashes.get("bank_tensors").context("loaded package lacks its bank digest")?;
            receipt.validate_for(package.source_manifest_sha256(), bank_sha, version)?;
        }
        _ => anyhow::bail!("Install verdant-voice or provide its verified original-source receipt before rendering"),
    }
    let hashes = selection["source_sha256"].as_object().context("voice selection lacks admitted package hashes")?;
    for (role, hash) in &originals {
        ensure!(hashes.get(role).and_then(serde_json::Value::as_str) == Some(hash.as_str()),
            "loaded voice source differs from the selected admission: {role}");
    }
    for (role, _) in verdant_backend::voices::PACKAGE_FILES {
        let expected = package.hashes.get(*role).context("loaded voice omitted a canonical role")?;
        ensure!(hashes.get(*role).and_then(serde_json::Value::as_str) == Some(expected.as_str()),
            "loaded voice differs from the selected admission: {role}");
    }
    ensure!(selection["voice_package"]["name"]=="verdant-voice"&&selection["voice_package"]["version"]==version.as_str(),
        "selected voice package version differs from its admitted manifest");
    let reference=selection["reference"].as_str().context("voice reference is missing")?;
    ensure!(!reference.starts_with("verdant-voice-")||reference==format!("verdant-voice-{version}"),
        "reserved voice reference and manifest version disagree");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap,fs,sync::atomic::{AtomicU64,Ordering}};

    struct Fixture(Config);
    impl Fixture {
        fn new()->Self{
            static NEXT:AtomicU64=AtomicU64::new(0);
            let root=env::temp_dir().join(format!("verdant-voice-runtime-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();
            Self(Config{workspace:root.clone(),project_root:root.clone(),voice_package:Some(root.clone()),
                original:None,voice_registry:None,onnxruntime_library:None,selection:None,
                bundled_voice:None,bundle:None,
                asset_sha256:BTreeMap::new(),request_sha256:String::new()})
        }
    }
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0.workspace);}}

    #[test]
    fn missing_canonical_runtime_inputs_fail_without_selecting_another_layout() {
        let fixture=Fixture::new();
        assert_eq!(voice_runtime(&fixture.0).unwrap(),fixture.0.workspace.as_path());
        assert!(asset_hashes(&fixture.0).is_err());
        fs::write(fixture.0.workspace.join("voice-source.json"),b"source marker").unwrap();
        assert!(asset_hashes(&fixture.0).is_err());
        fs::create_dir(fixture.0.workspace.join("verdant-voice")).unwrap();
        fs::write(fixture.0.workspace.join("verdant-voice/manifest.json"),b"{}\n").unwrap();
        assert!(asset_hashes(&fixture.0).is_err());
        assert!(admitted_voice_package(&fixture.0).is_err());
    }
}
