use std::{io::Write, process::{Command,Stdio}, path::Path};
use serde_json::{json,Value};

fn request(op:&str,params:Value)->Value{
    let mut child=Command::new(env!("CARGO_BIN_EXE_verdant")).args(["request","--json"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let request=json!({"protocol":"verdant-cli/1","request_id":"native-studio-test","op":op,"params":params});
    child.stdin.take().unwrap().write_all(serde_json::to_string(&request).unwrap().as_bytes()).unwrap();
    let output=child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_|panic!("{}",String::from_utf8_lossy(&output.stderr)))
}
fn ok(op:&str,params:Value)->Value{
    let response=request(op,params);assert_eq!(response["ok"],json!(true),"{op}: {response}");response["result"].clone()
}
fn inspect(path:&Path)->Value{ok("project.inspect",json!({"path":path}))}
#[test]
fn plugin_mixer_preset_workflow_is_revision_checked_and_roundtrips(){
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("混音 project.verp");
    ok("project.new",json!({"output":path}));
    let project=inspect(&path);let track=project["project"]["tracks"][0]["id"].clone();
    let owner=json!({"kind":"track","track_id":track});
    let add=ok("plugin.add",json!({"path":path,"expect_revision":project["revision"],"owner":owner,"device":"verdant.builtin.eq"}));
    let id=add["result"]["instance_id"].clone();let revision=add["state"]["revision"].clone();
    let before=std::fs::read(&path).unwrap();
    let rejected=request("plugin.set",json!({"path":path,"expect_revision":0,"owner":owner,"instance_id":id,"values":{"output":2}}));
    assert_eq!(rejected["error"]["code"],"REVISION_CONFLICT");assert_eq!(std::fs::read(&path).unwrap(),before);
    let rejected=request("mixer.apply",json!({"path":path,"expect_revision":revision,"edits":[
        {"operation":"set","owner":owner,"instance_id":id,"values":{"output":2}},
        {"operation":"mixer","owner":owner,"settings":{"pan":2}}
    ]}));
    assert_eq!(rejected["ok"],false);assert_eq!(std::fs::read(&path).unwrap(),before);
    let dry=ok("plugin.set",json!({"path":path,"expect_revision":revision,"dry_run":true,"owner":owner,"instance_id":id,"values":{"output":2}}));
    assert_eq!(dry["dry_run"],true);assert_eq!(std::fs::read(&path).unwrap(),before);
    let presets=ok("preset.list",json!({"device":"verdant.builtin.eq"}));assert_eq!(presets["count"],20);
    let applied=ok("preset.apply",json!({"path":path,"expect_revision":revision,"owner":owner,"instance_id":id,"preset_id":presets["presets"][0]["id"]}));
    let saved=dir.path().join("Lead preset.vpreset");
    ok("preset.export",json!({"output":saved,"path":path,"owner":owner,"instance_id":id}));
    let p:Value=serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    assert_eq!(p["format"],"verdant-device-preset");assert_eq!(p["parameters"].as_object().unwrap().len(),601);
    let restored=ok("preset.import",json!({"path":path,"expect_revision":applied["state"]["revision"],"owner":owner,"instance_id":id,"source":saved}));
    assert!(restored["state"]["revision"].is_number());
    let state=ok("plugin.inspect",json!({"path":path,"owner":owner,"instance_id":id}));
    assert_eq!(state["plugins"][0]["parameters"],p["parameters"]);
}

#[test]
fn every_native_operation_exposes_a_headless_schema_and_factory_is_complete(){
    let caps=ok("capabilities",json!({}));
    for op in caps["operations"].as_array().unwrap().iter().filter_map(Value::as_str).filter(|op|["plugin.","mixer.","preset.","clip.","analysis."].iter().any(|prefix|op.starts_with(prefix))){
        let schema=ok("schema",json!({"op":op}));assert_eq!(schema["params"]["additionalProperties"],false,"{op}");
    }
    let devices=ok("plugin.list",json!({}));assert_eq!(devices["devices"].as_array().unwrap().len(),14);
    for d in devices["devices"].as_array().unwrap(){assert_eq!(d["factory_preset_count"],20);let desc=ok("plugin.describe",json!({"device":d["id"]}));assert!(!desc["parameters"].as_array().unwrap().is_empty());}
}

#[test]
fn native_analysis_returns_applicable_values_and_rejects_silent_matches(){
    let dir=tempfile::tempdir().unwrap();
    let path=dir.path().join("anti-phase stereo.wav");
    let rate=48_000u32;let frames=8192u32;let bytes=frames*4;
    let mut wav=Vec::new();
    wav.extend_from_slice(b"RIFF");wav.extend_from_slice(&(36+bytes).to_le_bytes());wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());wav.extend_from_slice(&1u16.to_le_bytes());wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());wav.extend_from_slice(&(rate*4).to_le_bytes());wav.extend_from_slice(&4u16.to_le_bytes());wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");wav.extend_from_slice(&bytes.to_le_bytes());
    for frame in 0..frames{
        let x=(6000.*(std::f64::consts::TAU*440.*f64::from(frame)/f64::from(rate)).sin())as i16;
        wav.extend_from_slice(&x.to_le_bytes());wav.extend_from_slice(&(-x).to_le_bytes());
    }
    std::fs::write(&path,&wav).unwrap();
    let before=std::fs::read(&path).unwrap();
    let matched=ok("analysis.eq-match",json!({"source":path,"reference":path,"max_bands":4,"max_windows":4}));
    assert_eq!(matched["fit"]["bands_used"],0);
    assert!(matched["fit"]["rms_error_db"].as_f64().unwrap()<1e-10);
    assert_eq!(matched["source"]["channels"],2);
    let fitted=ok("analysis.eq-fit",json!({"points":[[20.,0.],[200.,0.],[1000.,-3.],[3000.,0.],[20000.,0.]],"max_bands":6}));
    assert!(fitted["values"].as_object().is_some());
    assert!(fitted["fit"]["bands_used"].as_u64().unwrap()>0);
    let project=dir.path().join("fit.verp");ok("project.new",json!({"output":project}));
    let state=inspect(&project);let owner=json!({"kind":"track","track_id":state["project"]["tracks"][0]["id"]});
    let added=ok("plugin.add",json!({"path":project,"expect_revision":state["revision"],"owner":owner,"device":"verdant.builtin.eq"}));
    ok("plugin.set",json!({"path":project,"expect_revision":added["state"]["revision"],"owner":owner,
        "instance_id":added["result"]["instance_id"],"values":fitted["values"]}));
    assert_eq!(std::fs::read(&path).unwrap(),before);
    let silent=dir.path().join("silence.wav");wav[44..].fill(0);std::fs::write(&silent,&wav).unwrap();
    let refused=request("analysis.eq-match",json!({"source":silent,"reference":path}));
    assert_eq!(refused["ok"],false);
    let paste=ok("schema",json!({"op":"clip.paste"}));
    assert!(paste["params"]["required"].as_array().unwrap().contains(&json!("target_track")));
    let target=ok("schema",json!({"op":"mixer.automate"}));
    assert_eq!(target["params"]["properties"]["lane"]["properties"]["target"]["oneOf"].as_array().unwrap().len(),3);
}
