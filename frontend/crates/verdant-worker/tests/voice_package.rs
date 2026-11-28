//! Explicit private-asset integration test. It exercises the real Rust process,
//! original-byte admission and both render branches; smoke weights establish
//! execution only, never voice quality or a legal-rights determination.
use std::{fs,io::{BufRead,BufReader,Write},path::PathBuf,process::{Child,ChildStdin,Command,Stdio},sync::mpsc,time::Duration};
use anyhow::{ensure,Context,Result};
use serde_json::{json,Value};
struct Fixture{root:PathBuf}
impl Fixture{fn new()->Result<Self>{let root=std::env::temp_dir().join(format!("verdant-voice-worker-test-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&root)?;Ok(Self{root})}}
impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.root);}}
struct Worker{child:Child,stdin:ChildStdin,events:mpsc::Receiver<Value>}
impl Worker{
    fn start(f:&Fixture)->Result<Self>{
        let mut command=Command::new(env!("CARGO_BIN_EXE_verdant-worker"));
        command.args(["--workspace",f.root.join("render").to_str().unwrap(),"--voice-registry",f.root.join("voices.json").to_str().unwrap(),"--project-root",f.root.to_str().unwrap()]);
        if let Ok(library)=std::env::var("VERDANT_ONNXRUNTIME_LIBRARY"){command.args(["--onnxruntime-library",&library]);}
        command.env_remove("VERDANT_VOICE_PACKAGE_DIR").env("VERDANT_DATA_DIR", &f.root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
        let mut child=command.spawn()?;let stdin=child.stdin.take().unwrap();let stdout=child.stdout.take().unwrap();let(tx,events)=mpsc::channel();
        std::thread::spawn(move||{for line in BufReader::new(stdout).lines().map_while(Result::ok){if let Ok(value)=serde_json::from_str(&line){if tx.send(value).is_err(){break;}}}});
        let worker=Self{child,stdin,events};ensure!(worker.next()?["event"]=="ready","worker did not become ready");Ok(worker)
    }
    fn send(&mut self,mut value:Value)->Result<()>{value["protocol"]="ogd-worker/1".into();serde_json::to_writer(&mut self.stdin,&value)?;self.stdin.write_all(b"\n")?;self.stdin.flush()?;Ok(())}
    fn next(&self)->Result<Value>{self.events.recv_timeout(Duration::from_secs(90)).context("worker event timed out")}
    fn render(&mut self,id:&str,route:&str,score:&Value)->Result<Value>{
        self.render_payload(id,route,json!({"kind":"native_score","score":score,"native_seed":10008}))
    }
    fn render_payload(&mut self,id:&str,route:&str,payload:Value)->Result<Value>{
        self.render_payload_for("test-voice", id, route, payload)
    }
    fn render_payload_for(&mut self, reference: &str, id: &str, route: &str, payload: Value) -> Result<Value> {
        self.send(json!({"request_id":id,"op":"render","job_id":id,"revision":1,"voice_ref":reference,"route":route,"payload":payload}))?;
        loop{let event=self.next()?;if matches!(event["event"].as_str(),Some("completed"|"failed"|"rejected")){return Ok(event);}}
    }
}
impl Drop for Worker{fn drop(&mut self){let _=self.child.kill();let _=self.child.wait();}}

#[test]
fn desktop_original_requires_available_exact_raw_roles() -> Result<()> {
    use verdant_backend::{voices::is_desktop_original, wire::{FileStatus, VoiceEntry}};
    let mut voice=VoiceEntry{available:true,..Default::default()};
    for(role,hash)in [("token",concat!("821b7cd9e7b489077dd55bf581ba90e1", "dffd3816c6fedcedc320744b498deead")),
        ("model","213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff")]{
        voice.files.insert(role.into(),FileStatus{available:true,sha256:Some(hash.into())});
    }
    ensure!(is_desktop_original(&voice),"exact available raw roles rejected");
    let original=voice.clone();
    voice.available=false;ensure!(!is_desktop_original(&voice),"unavailable entry admitted");
    for role in ["token","model"]{
        voice=original.clone();voice.files.remove(role);ensure!(!is_desktop_original(&voice),"missing {role} admitted");
        voice=original.clone();voice.files.get_mut(role).unwrap().available=false;ensure!(!is_desktop_original(&voice),"unavailable {role} admitted");
        voice=original.clone();voice.files.get_mut(role).unwrap().sha256=Some("0".repeat(64));ensure!(!is_desktop_original(&voice),"wrong {role} admitted");
    }
    for role in verdant_backend::voices::PACKAGE_FILES.iter().chain(verdant_backend::voices::VOICE_FILES)
        .map(|(role,_)|*role).chain(["voice_source_admission","voice_json"]){
        voice=original.clone();voice.files.insert(role.into(),FileStatus{available:true,sha256:Some("0".repeat(64))});
        ensure!(!is_desktop_original(&voice),"mixed {role} entry admitted");
    }
    Ok(())
}

#[test]
fn reserved_original_admission_rejects_named_wrong_bytes_without_mutating_registry() -> Result<()> {
    use verdant_backend::voices::{VoiceRegistry,VoiceAssets,DESKTOP_ORIGINAL_REFERENCE};
    let f=Fixture::new()?;let token=f.root.join("sn16k.tkn");let model=f.root.join("sn16k.bin");
    fs::write(&token,b"not the original token")?;fs::write(&model,b"not the original model")?;
    let assets=VoiceAssets{token:Some(token.clone()),model:Some(model.clone()),..Default::default()};
    let mut registry=VoiceRegistry::new();let path=f.root.join("voices.json");
    let rejected=registry.install_with(&path,DESKTOP_ORIGINAL_REFERENCE,"Original",assets.clone(),|_|panic!("wrong bytes reached loader"));
    ensure!(rejected.is_err()&&registry.references().is_empty()&&!path.exists(),"wrong bytes changed registry");
    let mut worker = Worker::start(&f)?;
    worker.send(json!({"request_id":"unreserved-raw","op":"admit_voice",
        "reference":"research-raw","display_name":"Research","assets":{"token":token,"model":model}}))?;
    let rejected = worker.next()?;
    ensure!(rejected["event"] == "rejected" && !path.exists(),
        "unreserved raw admission changed registry or failed to reject: {rejected}");
    Ok(())
}

#[test]
fn forged_original_registry_cannot_select_other_routes_or_advertise_wrong_bytes() -> Result<()> {
    use sha2::{Digest,Sha256};
    use verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE;
    let f=Fixture::new()?;let token=f.root.join("sn16k.tkn");let model=f.root.join("sn16k.bin");
    fs::write(&token,b"wrong token")?;fs::write(&model,b"wrong model")?;
    let entry=json!({"display_name":"Forged original","assets":{"token":token,"model":model},
        "files":{"token":hex::encode(Sha256::digest(b"wrong token")),"model":hex::encode(Sha256::digest(b"wrong model"))}});
    fs::write(f.root.join("voices.json"),serde_json::to_vec(&json!({"version":1,"voices":{(DESKTOP_ORIGINAL_REFERENCE):entry}}))?)?;
    let mut worker=Worker::start(&f)?;
    worker.send(json!({"request_id":"caps","op":"capabilities"}))?;let caps=worker.next()?;
    ensure!(caps["capabilities"]["voices"]["error"].is_string()
        &&caps["capabilities"]["voices"].get("voices").is_none(),
        "a forged reserved reference must reject registry admission: {caps}");
    for route in ["C0","C1","verdant-voice","A"]{
        let id=format!("reserved-{route}");
        worker.send(json!({"request_id":id,"op":"render","job_id":id,"revision":1,"route":route,"voice_ref":DESKTOP_ORIGINAL_REFERENCE,
            "payload":{"kind":"native_score","score":{"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":64}]}}}))?;
        let rejected=worker.next()?;ensure!(rejected["event"]=="rejected","reserved mode unexpectedly rendered: {rejected}");
        ensure!(!f.root.join("render/jobs").join(id).exists(),"rejected original created a job");
    }
    Ok(())
}

#[test]
fn first_launch_lists_empty_registry_and_requires_a_voice_package()->Result<()>{
    let f=Fixture::new()?;let mut worker=Worker::start(&f)?;
    worker.send(json!({"request_id":"caps","op":"capabilities"}))?;let caps=worker.next()?;
    ensure!(caps["event"]=="capabilities"&&caps["capabilities"]["voices"]["version"]==1&&caps["capabilities"]["voices"]["voices"]==json!({}),"first-launch empty catalog unavailable: {caps}");
    ensure!(caps["capabilities"]["native_score"]["phoneme_timing"]["constrained_nucleus_timing"] == true,
        "worker must advertise support for delayed and absent nuclei");
    ensure!(caps["capabilities"]["voice_package"]["sample_rate"]==48000
        &&caps["capabilities"]["voice_package"]["music"]["sample_rates"]==json!([44100,48000])
        &&caps["capabilities"]["voice_package"]["music"]["default_sample_rate"]==48000,
        "worker must distinguish native decoder rate from available music output rates");
    for(reference,id)in[(None,"no-default"),(Some("voice-sn"),"old-alias"),(Some("not-installed"),"unknown")]{
        let mut request=json!({"request_id":id,"job_id":id,"revision":1,"op":"render","route":"verdant-voice","payload":{"kind":"native_score","score":{"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":64}]}}});
        if let Some(reference)=reference{request["voice_ref"]=reference.into();}worker.send(request)?;let rejected=worker.next()?;
        ensure!(rejected["event"]=="rejected","missing input unexpectedly rendered: {rejected}");let message=rejected["message"].as_str().unwrap_or("");
        if reference==Some("not-installed"){ensure!(message.contains("not installed"),"unknown singer silently used CLI assets: {message}");}
        else{ensure!(message.contains("voice package directory is missing"),"no-package request took an unexpected route: {message}");}
    }
    ensure!(!f.root.join("voices.json").exists(),"read-only first launch wrote an unrequested registry");Ok(())
}

#[test]
#[ignore="requires VERDANT_VOICE_TEST_PACKAGE/TOKEN/MODEL, native ORT, and a worker built with the matching bank pin"]
fn actual_worker_admission_render_and_tamper_rejection()->Result<()>{
    use sha2::{Digest, Sha256};
    use verdant_backend::voices::{PACKAGE_FILES, RECORD_FILES};
    let f=Fixture::new()?;let source=PathBuf::from(std::env::var("VERDANT_VOICE_TEST_PACKAGE")?);let package=f.root.join("package");fs::create_dir(&package)?;
    for (_, name) in PACKAGE_FILES { fs::copy(source.join(name), package.join(name))?; }
    let manifest_bytes = fs::read(package.join("manifest.json"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let records = manifest["records"].as_object().context("expected canonical source records")?;
    for (kind, name) in RECORD_FILES {
        if records.contains_key(*kind) {
            fs::copy(source.join(name), package.join(name))?;
        }
    }
    let mut worker=Worker::start(&f)?;
    let score=json!({"title":"Original integration vowels with a real rest","synthesis":{"prosody_mode":"step"},"segments":[
        {"vowel":"a","duration_seconds":0.30,"midi_note":64,"gap_after_seconds":0.10},
        {"vowel":"i","duration_seconds":0.30,"midi_note":67}]});
    let rejected=worker.render("missing-bank","verdant-voice",&score)?;ensure!(rejected["event"]=="rejected","missing bank did not reject explicitly: {rejected}");
    ensure!(!f.root.join("render/jobs/missing-bank").exists(),"rejected verdant-voice request created a fallback job");
    let mut admission=json!({"request_id":"admit","op":"admit_voice","reference":"test-voice","display_name":"Integration test only","assets":{"voice_package":package},
        "source_evidence":{"token":std::env::var("VERDANT_VOICE_TEST_TOKEN")?,"model":std::env::var("VERDANT_VOICE_TEST_MODEL")?,"rights_attested":false}});
    worker.send(admission.clone())?;ensure!(worker.next()?["event"]=="rejected","unchecked source attestation was accepted");
    admission["source_evidence"]["rights_attested"]=true.into();
    let wrong_original = f.root.join("wrong.tkn");
    fs::write(&wrong_original, b"wrong original with attestation")?;
    let mut wrong = admission.clone();
    wrong["source_evidence"]["token"] = serde_json::to_value(wrong_original)?;
    worker.send(wrong)?;
    ensure!(worker.next()?["event"] == "rejected" && !f.root.join("voices.json").exists(),
        "wrong original source was admitted or changed the registry");
    worker.send(admission)?;
    let admitted=worker.next()?;ensure!(admitted["event"]=="voice_admitted","real source admission failed: {admitted}");
    let registry:Value=serde_json::from_slice(&fs::read(f.root.join("voices.json"))?)?;
    let roles = registry["voices"]["test-voice"]["files"].as_object().context("missing registry asset map")?;
    let root_hash = hex::encode(Sha256::digest(&manifest_bytes));
    ensure!(roles.len() == PACKAGE_FILES.len() && PACKAGE_FILES.iter().all(|(role, _)| roles.contains_key(*role)),
        "registry retained non-runtime source or omitted canonical roles");
    ensure!(roles["verdant_voice_manifest"] == root_hash.as_str()
        && registry["voices"]["test-voice"]["source_receipt"]["source_manifest_sha256"] == root_hash.as_str(),
        "registry receipt is not bound to the actual package root manifest");
    ensure!(registry["voices"]["test-voice"]["source_receipt"]["attestation_is_legal_determination"]==false,"receipt incorrectly makes a legal determination");
    ensure!(registry["voices"]["test-voice"]["assets"].get("token").is_none()&&registry["voices"]["test-voice"]["assets"].get("model").is_none(),"original files became verdant-voice runtime dependencies");
    worker.send(json!({"request_id":"old-alias-after-install","job_id":"old-alias-after-install","revision":1,"op":"render","voice_ref":"voice-sn","route":"C0","payload":{"kind":"native_score","score":score}}))?;
    let old=worker.next()?;ensure!(old["event"]=="rejected"&&old["message"].as_str().is_some_and(|v|v.contains("retired")),"installing another singer made a retired route usable: {old}");
    for(key,value)in [("source_profile",json!("phonetic")),("gain_mode",json!("relative")),("vowel_brightness_db",json!(3.))]{
        let mut unsupported=score.clone();unsupported["synthesis"][key]=value;
        let id=format!("unsupported-{key}");let rejected=worker.render(&id,"verdant-voice",&unsupported)?;
        ensure!(rejected["event"]=="rejected","verdant-voice silently accepted unused {key}: {rejected}");
        ensure!(!f.root.join("render/jobs").join(&id).exists(),"unsupported legacy option created a fallback job");
    }
    // This is the exact neutral synthesis object emitted by the current app
    // when a new verdant-voice project has an explicit settings panel value.
    let mut score=score;score["synthesis"]["vowel_balance_db"]=0.into();
    let event=worker.render("voice","verdant-voice",&score)?;ensure!(event["event"]=="completed","actual verdant-voice render failed: {event}");
    let job=f.root.join("render/jobs/voice");ensure!(job.join("verdant-voice.wav").is_file()&&job.join("verdant-voice.npz").is_file()&&job.join("native.npz").is_file(),"verdant-voice artifacts missing");
    ensure!(!job.join("native.wav").exists()&&!job.join("neural").exists(),"verdant-voice silently invoked a legacy waveform route");
    let report:Value=serde_json::from_slice(&fs::read(job.join("verdant-voice.json"))?)?;
    ensure!(report["route"]=="verdant-voice"&&report["fallback_used"]==false&&report["old_waveform_generated"]==false,"verdant-voice report does not identify its actual route");
    ensure!(report["candidate"]=="verdant-voice"&&report["strategy"]==verdant_synthesis::voice::STRATEGY,
        "voice report does not identify the admitted native graph strategy");
    ensure!(report["controller_reference"]["target_f0_preserved"]==true
        &&report["controller_reference"]["decoder_target_conditions_preserved"]==true,
        "native controller reference displaced authored decoder conditions");
    let(rate,wave)=verdant_backend::artifact::decode_wav(&fs::read(job.join("verdant-voice.wav"))?).map_err(anyhow::Error::msg)?;
    ensure!(rate==48000&&wave.len()==33600&&wave.iter().all(|v|v.is_finite()),"verdant-voice output clock/finite mismatch");
    ensure!(wave[14400..19200].iter().all(|v|*v==0.),"verdant-voice rest is not exactly silent");
    eprintln!("verdant-voice integration-only render: {} samples, peak={} (no normalization)",wave.len(),wave.iter().fold(0_f32,|a,v|a.max(v.abs())));
    for rate in [44100_u32,48000] {
        let id=format!("voice-music-{rate}");
        let event=worker.render_payload(&id,"verdant-voice",json!({"kind":"native_score","score":score,"native_seed":10008,
            "voice_music":{"method":"verdant-voice-music-v1","sample_rate":rate}}))?;
        ensure!(event["event"]=="completed","verdant-voice music {rate} failed: {event}");
        let music_job=f.root.join("render/jobs").join(&id);
        let artifacts:Vec<verdant_backend::wire::Artifact>=serde_json::from_value(event["artifacts"].clone())?;
        let audio=verdant_backend::artifact::verify_and_decode(&f.root.join("render"),verdant_backend::Route::VerdantVoice,&artifacts)
            .map_err(anyhow::Error::msg)?;
        ensure!(audio.sample_rate==rate&&audio.samples.len()==rate as usize*7/10
            &&audio.samples.iter().all(|value|value.is_finite()),"verdant-voice music WAV clock or verified artifact mismatch");
        ensure!(audio.samples[rate as usize*3/10..rate as usize*4/10].iter().all(|value|*value==0.),
            "verdant-voice music {rate} did not retain the authored rest exactly");
        for filename in ["native.npz","verdant-voice-conditions.npz","verdant-voice.npz"] {
            ensure!(fs::read(job.join(filename))?==fs::read(music_job.join(filename))?,
                "verdant-voice music {rate} changed the admitted raw {filename}");
        }
        let music_report:Value=serde_json::from_slice(&fs::read(music_job.join("verdant-voice.json"))?)?;
        ensure!(music_report["sample_rate"]==rate,"verdant-voice report sample_rate does not describe its final WAV");
        ensure!(!music_job.join("native.wav").exists()&&!music_job.join("neural").exists(),
            "verdant-voice music substituted a legacy waveform route");
        eprintln!("verdant-voice music integration-only output: {rate} Hz, {} samples; native conditions and core arrays byte exact",audio.samples.len());
    }
    for route in ["C0", "C1", "A"] {
        let id = format!("retired-{route}");
        let event = worker.render(&id, route, &score)?;
        ensure!(event["event"] == "rejected", "ordinary voice accepted retired route {route}: {event}");
        ensure!(!f.root.join("render/jobs").join(id).exists(), "retired route created a job");
    }
    use verdant_backend::voices::{PackageReceipt, VoiceAdmission, VoiceAssets, VoiceRegistry};
    let registry_path = f.root.join("voices.json");
    let mut installed = VoiceRegistry::load(&registry_path)?;
    installed.install_with(&registry_path, "installed-package", "Package-only test", VoiceAssets {
        voice_package: Some(package.clone()), ..Default::default()
    }, |assets| {
        let loaded = verdant_synthesis::model::load_voice_package(assets.voice_package.as_deref().unwrap())
            .map_err(|error| format!("{error:#}"))?;
        let source_sha256 = serde_json::from_value(loaded.voice.provenance["source_sha256"].clone())
            .map_err(|error| error.to_string())?;
        Ok(VoiceAdmission { source_sha256, source_receipt: None, package_receipt: Some(PackageReceipt::new(
            loaded.source_manifest_sha256().into(), loaded.hashes["bank_tensors"].clone(), loaded.voice_version.clone(),
        )) })
    })?;
    worker.send(json!({"request_id":"installed-caps","op":"capabilities"}))?;
    let caps = worker.next()?;
    ensure!(caps["capabilities"]["voices"]["voices"]["installed-package"]["files"]["voice_source_admission"]["available"] == true,
        "package installation receipt did not enable catalog admission: {caps}");
    let event = worker.render_payload_for("installed-package", "installed-package", "verdant-voice",
        json!({"kind":"native_score","score":score,"native_seed":10008}))?;
    ensure!(event["event"] == "completed", "package with installation receipt failed to render: {event}");
    let execution: Value = serde_json::from_slice(&fs::read(f.root.join("render/jobs/installed-package/execution-result.json"))?)?;
    ensure!(execution["voice_selection"]["source_receipt"].is_null()
        && execution["voice_selection"]["package_receipt"]["manifest_sha256"] == root_hash.as_str(),
        "package-only render did not preserve the installation receipt");
    let bank_path = package.join("bank.safetensors");
    let mut bank = fs::read(&bank_path)?;
    bank[0] ^= 1;
    fs::write(&bank_path, &bank)?;
    let event = worker.render("tampered-bank", "verdant-voice", &score)?;
    ensure!(event["event"] == "rejected", "changed bank did not fail admission: {event}");
    ensure!(!f.root.join("render/jobs/tampered-bank").exists(), "tampered bank created a render job");
    bank[0] ^= 1;
    fs::write(bank_path, bank)?;
    let controller_path=package.join("context-controller.onnx");
    let mut controller=fs::read(&controller_path)?;controller[0]^=1;fs::write(&controller_path,controller)?;
    let event=worker.render("tampered","verdant-voice",&score)?;ensure!(event["event"]=="rejected","changed controller did not fail admission: {event}");
    ensure!(!f.root.join("render/jobs/tampered").exists(),"tampered package fell back to another renderer");
    worker.send(json!({"request_id":"shutdown","op":"shutdown"}))?;ensure!(worker.next()?["event"]=="shutdown","worker shutdown failed");Ok(())
}

#[test]
#[ignore="requires the actual user-supplied originals in VERDANT_VOICE_TEST_TOKEN/MODEL"]
fn actual_original_c0_keeps_expression_and_rechecks_catalog_bytes()->Result<()>{
    use verdant_backend::voices::{DESKTOP_ORIGINAL_REFERENCE,is_desktop_original};
    let f=Fixture::new()?;let token=f.root.join("arbitrary-token-name");let model=f.root.join("arbitrary-model-name");
    fs::copy(std::env::var("VERDANT_VOICE_TEST_TOKEN")?,&token)?;fs::copy(std::env::var("VERDANT_VOICE_TEST_MODEL")?,&model)?;
    let mut worker=Worker::start(&f)?;
    worker.send(json!({"request_id":"original-admit","op":"admit_voice","reference":DESKTOP_ORIGINAL_REFERENCE,
        "display_name":"Original","assets":{"token":token,"model":model}}))?;
    let admitted=worker.next()?;ensure!(admitted["event"]=="voice_admitted","original admission failed: {admitted}");
    worker.send(json!({"request_id":"original-caps","op":"capabilities"}))?;let caps=worker.next()?;
    let voice=serde_json::from_value(caps["capabilities"]["voices"]["voices"][DESKTOP_ORIGINAL_REFERENCE].clone())?;
    ensure!(is_desktop_original(&voice),"actual raw original not available in catalog");
    worker.send(json!({"request_id":"original-expression","op":"render","job_id":"original-expression","revision":1,
        "route":"C0","voice_ref":DESKTOP_ORIGINAL_REFERENCE,"payload":{"kind":"native_score",
        "score":{"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":64}]},
        "expression":{"method":"vocaloid-controls-v1","segments":[{"start_seconds":0.,"end_seconds":0.3,
            "note_onset_seconds":0.,"curves":{"BRI":[[0.,90.]]}}]}}}))?;
    loop{let event=worker.next()?;if matches!(event["event"].as_str(),Some("completed"|"failed"|"rejected")){
        ensure!(event["event"]=="completed","original expression failed: {event}");break;
    }}
    let job=f.root.join("render/jobs/original-expression");
    let output:Value=serde_json::from_slice(&fs::read(job.join("output_pipeline.json"))?)?;
    ensure!(output["route"]=="C0"&&output["processors"]["voice_expression"]["enabled"]==true,
        "C0 original skipped its expression pipeline: {output}");
    fs::write(&token,b"changed after original admission")?;
    worker.send(json!({"request_id":"changed-caps","op":"capabilities"}))?;let caps=worker.next()?;
    ensure!(caps["capabilities"]["voices"]["voices"][DESKTOP_ORIGINAL_REFERENCE]["available"]==false,
        "changed original bytes still advertised available");
    Ok(())
}
