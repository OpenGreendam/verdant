//! Complete inbound admission is independent of the editor's serialization subset.
//! The checked-in public schema supplies structural constraints; the relational
//! checks below preserve the worker's clocks, voicing and route restrictions.
use std::collections::BTreeSet;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use crate::{Config, runtime};

fn equal(a: &Value,b: &Value) -> bool {
    match (a,b) {
        (Value::Number(a),Value::Number(b)) => a.as_f64()==b.as_f64(),
        (Value::Array(a),Value::Array(b)) => a.len()==b.len() && a.iter().zip(b).all(|(a,b)|equal(a,b)),
        (Value::Object(a),Value::Object(b)) => a.len()==b.len() && a.iter().all(|(k,a)|b.get(k).is_some_and(|b|equal(a,b))),
        _ => a==b,
    }
}
fn validate(value: &Value, schema: &Value, root: &Value, path: &str) -> Result<()> {
    if let Some(rule) = schema.as_bool() { ensure!(rule,"{path}: value is not admitted"); return Ok(()); }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let target = root.pointer(reference.strip_prefix('#').context("only local schema references are admitted")?).context("unresolved request schema reference")?;
        validate(value,target,root,path)?;
    }
    if let Some(types)=schema.get("type") {
        let check=|t:&str|match t {
            "object"=>value.is_object(),"array"=>value.is_array(),"string"=>value.is_string(),"boolean"=>value.is_boolean(),"null"=>value.is_null(),
            "number"=>value.as_f64().is_some_and(f64::is_finite),"integer"=>value.as_f64().is_some_and(|v|v.is_finite()&&v.fract()==0.),_=>false,
        };
        let valid = if let Some(t)=types.as_str(){check(t)}else{types.as_array().context("schema types")?.iter().any(|t|t.as_str().is_some_and(check))};
        ensure!(valid,"{path}: expected {types}");
    }
    if let Some(expected)=schema.get("const"){ensure!(equal(value,expected),"{path}: expected {expected}");}
    if let Some(values)=schema.get("enum").and_then(Value::as_array){ensure!(values.iter().any(|e|equal(value,e)),"{path}: unsupported value {value}");}
    if let Some(rules)=schema.get("allOf").and_then(Value::as_array){for rule in rules{validate(value,rule,root,path)?;}}
    if let Some(rules)=schema.get("anyOf").and_then(Value::as_array){ensure!(rules.iter().any(|rule|validate(value,rule,root,path).is_ok()),"{path}: no admitted configuration matches");}
    if let Some(rules)=schema.get("oneOf").and_then(Value::as_array){ensure!(rules.iter().filter(|rule|validate(value,rule,root,path).is_ok()).count()==1,"{path}: must match exactly one admitted configuration");}
    if let Some(rule)=schema.get("not"){ensure!(validate(value,rule,root,path).is_err(),"{path}: incompatible configuration");}
    if let Some(condition)=schema.get("if") {
        let branch=if validate(value,condition,root,path).is_ok(){"then"}else{"else"};
        if let Some(rule)=schema.get(branch){validate(value,rule,root,path)?;}
    }
    if let Some(object)=value.as_object(){
        if let Some(required)=schema.get("required").and_then(Value::as_array){for key in required{let key=key.as_str().unwrap();ensure!(object.contains_key(key),"{path}: missing {key}");}}
        if let Some(min)=schema.get("minProperties").and_then(Value::as_u64){ensure!(object.len()>=min as usize,"{path}: too few properties");}
        let properties=schema.get("properties").and_then(Value::as_object);
        for (key,element) in object {
            if let Some(rule)=properties.and_then(|p|p.get(key)){validate(element,rule,root,&format!("{path}.{key}"))?;}
            else if let Some(rule)=schema.get("additionalProperties"){
                ensure!(rule!=&Value::Bool(false),"{path}: unknown field {key}");
                if rule.is_object(){validate(element,rule,root,&format!("{path}.{key}"))?;}
            }
        }
    }
    if let Some(array)=value.as_array(){
        for (key,minimum) in [("minItems",true),("maxItems",false)] {
            if let Some(bound)=schema.get(key).and_then(Value::as_u64){ensure!(if minimum{array.len()>=bound as usize}else{array.len()<=bound as usize},"{path}: violates {key}={bound}");}
        }
        if schema.get("uniqueItems")==Some(&Value::Bool(true)){for (i,a) in array.iter().enumerate(){ensure!(!array[..i].iter().any(|b|equal(a,b)),"{path}: duplicate array value");}}
        let prefix=schema.get("prefixItems").and_then(Value::as_array);
        for (i,element) in array.iter().enumerate(){
            if let Some(rule)=prefix.and_then(|p|p.get(i)).or_else(||schema.get("items")){validate(element,rule,root,&format!("{path}[{i}]"))?;}
        }
    }
    if let Some(text)=value.as_str(){
        for (key,minimum) in [("minLength",true),("maxLength",false)]{
            if let Some(bound)=schema.get(key).and_then(Value::as_u64){let len=text.chars().count();ensure!(if minimum{len>=bound as usize}else{len<=bound as usize},"{path}: violates {key}={bound}");}
        }
        if let Some(pattern)=schema.get("pattern").and_then(Value::as_str){
            ensure!(pattern=="^[A-Za-z0-9_-]{1,64}$","unrecognized compiled request pattern");
            ensure!(job_id(text),"{path}: requires 1..64 ASCII letters/digits/_/-");
        }
    }
    if let Some(number)=value.as_f64(){
        ensure!(number.is_finite(),"{path}: nonfinite number");
        for (key,op) in [("minimum",0),("maximum",1),("exclusiveMinimum",2),("exclusiveMaximum",3)]{
            if let Some(bound)=schema.get(key).and_then(Value::as_f64){ensure!(match op{0=>number>=bound,1=>number<=bound,2=>number>bound,_=>number<bound},"{path}: violates {key}={bound}");}
        }
    }
    Ok(())
}
fn job_id(id:&str)->bool{!id.is_empty()&&id.len()<=64&&id.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'_'|b'-'))}
pub fn route(request: &Value) -> Result<()> {
    let original = request["voice_ref"].as_str() == Some(verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE);
    match request["route"].as_str() {
        Some("verdant-voice") => ensure!(!original, "the unlocked original voice requires its restricted original route"),
        Some("C0") => ensure!(original, "C0 is retired for ordinary voices; original synthesis requires the explicitly unlocked original voice"),
        Some("C1" | "A") => anyhow::bail!("retired render route: {}", request["route"]),
        _ => anyhow::bail!("unsupported render route: {}", request["route"]),
    }
    Ok(())
}

pub fn request(request:&Value)->Result<()> {
    if request["op"] == "render" { route(request)?; }
    static SCHEMA:std::sync::OnceLock<Value>=std::sync::OnceLock::new();
    let schema=SCHEMA.get_or_init(||serde_json::from_str(include_str!("../../../../backend/worker-request.schema.json")).expect("bundled request schema"));
    validate(request,schema,schema,"request")?;
    if request["op"]=="render"{
        ensure!(request["revision"].as_u64().is_some(),"revision must be a nonnegative integer");
        authored_pitch(&request["payload"],request["route"].as_str().unwrap())?;
    }
    Ok(())
}
// Neumaier compensated accumulation preserves endpoint admission near 30/120 s.
fn sum(values:impl IntoIterator<Item=f64>)->f64{
    let(mut total,mut correction)=(0.0_f64,0.0_f64);
    for x in values{let next=total+x;correction+=if total.abs()>=x.abs(){(total-next)+x}else{(x-next)+total};total=next;}total+correction
}
fn number(value:&Value)->f64{value.as_f64().unwrap()}
fn phoneme_timing(score:&Value)->Result<()> {
    let Some(trace)=score.get("phoneme_timing") else{return Ok(());};
    let segments=score["segments"].as_array().unwrap();
    let(mut starts,mut ends,mut terms)=(Vec::new(),Vec::new(),Vec::new());
    for segment in segments{starts.push(sum(terms.iter().copied()));terms.push(number(&segment["duration_seconds"]));ends.push(sum(terms.iter().copied()));terms.push(segment.get("gap_after_seconds").map_or(0.,number));}
    let mut expected=0;
    let mut ids=BTreeSet::new();
    for note in trace["notes"].as_array().unwrap(){
        ensure!(ids.insert(note["note_id"].as_str().unwrap()),"phoneme_timing note IDs must be unique");
        let first=note["first_segment_index"].as_u64().context("phoneme timing first index must be an integer")? as usize;
        let nucleus=note["nucleus_segment_index"].as_u64().context("phoneme timing nucleus index must be an integer")? as usize;
        let end=note["end_segment_index"].as_u64().context("phoneme timing end index must be an integer")? as usize;
        ensure!(first==expected&&first<=nucleus&&nucleus<end&&end<=segments.len(),"phoneme_timing ranges must cover each segment once in order with an in-range nucleus");
        let(onset,note_end,prefix,tail)=(number(&note["note_onset_seconds"]),number(&note["note_end_seconds"]),number(&note["prefix_seconds"]),number(&note["tail_trim_seconds"]));
        ensure!(note_end>onset&&tail<note_end-onset,"phoneme_timing requires a positive note and nucleus interval");
        ensure!(segments[first..end-1].iter().all(|s|s.get("gap_after_seconds").map_or(0.,number)==0.),"phoneme_timing does not allow an internal gap within a note");
        for (actual,wanted,label) in [(onset,starts[nucleus],"nucleus anchor"),(prefix,starts[nucleus]-starts[first],"prefix"),(note_end-tail,ends[end-1],"tail trim")] {
            ensure!((actual-wanted).abs()<=1e-9,"phoneme_timing {label} disagrees with scheduled segment boundaries");
        }
        expected=end;
    }
    ensure!(expected==segments.len(),"phoneme_timing must cover every score segment");Ok(())
}
fn expression(value:&Value,maximum:f64)->Result<()> {
    let mut last_end=0.;
    for segment in value["segments"].as_array().unwrap(){
        let(start,end,onset)=(number(&segment["start_seconds"]),number(&segment["end_seconds"]),number(&segment["note_onset_seconds"]));
        ensure!(start>=last_end-1e-10&&start<=onset&&onset<end&&end<=maximum+1e-10,"expression intervals must be ordered, nonoverlapping and own their onset within the score");last_end=end;
        for points in segment["curves"].as_object().unwrap().values(){
            let mut previous=-1.;
            for point in points.as_array().unwrap(){let t=number(&point[0]);ensure!(t>previous&&t<=maximum,"controller points must have increasing times within the score duration");previous=t;}
        }
    }
    Ok(())
}
fn voice_music(payload:&Value,route:&str)->Result<()> {
    let Some(value)=payload.get("voice_music") else{return Ok(());};
    ensure!(route=="verdant-voice"&&payload["kind"]=="native_score","voice_music requires the verdant-voice native_score route");
    let profile:verdant_backend::wire::VoiceMusic=serde_json::from_value(value.clone()).context("invalid voice_music profile")?;
    profile.validate().map_err(anyhow::Error::msg)
}
/// Route-specific authoring bounds are checked before loading a voice or doing
/// work. MIDI conversion and every submitted control anchor are checked exactly;
/// no adapter, phoneme gate or interpolation may hide an out-of-range request.
fn authored_pitch(payload:&Value,route:&str)->Result<()> {
    if payload["kind"]!="native_score"{return Ok(());}
    let range=verdant_synthesis::parameters::NATIVE_F0_RANGE;
    for segment in payload["score"]["segments"].as_array().context("missing score segments")?{
        if let Some(hz)=segment.get("f0_hz"){
            ensure!((range[0]..=range[1]).contains(&number(hz)),
                "{route} segment F0 must be {}..{} Hz",range[0],range[1]);
        }
        if let Some(midi)=segment.get("midi_note"){
            let f0=440.*2.0_f64.powf((number(midi)-69.)/12.);
            ensure!((range[0]..=range[1]).contains(&f0),
                "{route} segment MIDI F0 must be {}..{} Hz",range[0],range[1]);
        }
    }
    if let Some(track)=payload.get("control_track").filter(|v|!v.is_null()){
        let values=track["f0_hz"].as_array().context("missing F0 array")?;
        let uv=track["uv"].as_array().context("missing UV array")?;
        ensure!(values.len()==uv.len(),"control F0/UV length mismatch");
        for(hz,unvoiced)in values.iter().zip(uv){
            let hz=number(hz);
            ensure!(if unvoiced.as_bool().context("control UV must be boolean")?{hz==0.}
                else{(range[0]..=range[1]).contains(&hz)},
                "{route} control F0 must be {}..{} Hz when voiced and zero when UV",range[0],range[1]);
        }
    }
    Ok(())
}
pub fn render(request:&Value,config:&Config)->Result<()> {
    let payload = &request["payload"];
    let route = request["route"].as_str().unwrap();
    ensure!(payload["kind"] == "native_score", "rendering requires a native_score payload");
    voice_music(payload, route)?;
    if route == "verdant-voice" {
        let package = runtime::admitted_voice_package(config)?;
        for key in ["vocal_tract", "model_dynamics", "consonant_dynamics"] {
            ensure!(payload.get(key).is_none_or(Value::is_null), "verdant-voice does not support {key}");
        }
        if let Some(synthesis) = payload.pointer("/score/synthesis") {
            for key in ["source_profile", "gain_mode", "vowel_brightness_db"] {
                ensure!(synthesis.get(key).is_none(), "verdant-voice does not use {key}");
            }
            ensure!(synthesis.get("vowel_balance_db").is_none_or(|v| v.as_f64() == Some(0.))
                && synthesis.get("phonetic_correction").is_none_or(Value::is_null),
                "verdant-voice requires neutral harmonic compensation and phonetic correction");
        }
        let options = serde_json::json!({"onnxruntime_library":runtime::onnxruntime_path(config)?});
        runtime::validate_voice_package(config, &package, &options)?;
    } else {
        runtime::load_original(config)?;
    }
    if let Some(seed) = payload.get("native_seed") {
        ensure!(seed.as_u64().is_some_and(|n| n < 1 << 31), "native_seed must be an integer in [0,2^31)");
    }
    let score = &payload["score"];
    let segments = score["segments"].as_array().unwrap();
    let maximum = if score.get("continuity").is_some() { 120. } else { 30. };
    let duration = sum(segments.iter().flat_map(|s| [number(&s["duration_seconds"]), s.get("gap_after_seconds").map_or(0., number)]));
    ensure!(duration <= maximum + 1e-12, "score exceeds {maximum} seconds; insert a real rest or select a shorter passage");
    phoneme_timing(score)?;
    verdant_synthesis::parameters::validate_score_hierarchy(score)?;
    verdant_synthesis::parameters::validate_score_continuations(score)?;
    if let Some(value) = payload.get("expression") { expression(value, duration)?; }
    if let Some(track)=payload.get("control_track").filter(|v|!v.is_null()){
        let times=track["frame_times_seconds"].as_array().unwrap();let f0=track["f0_hz"].as_array().unwrap();let uv=track["uv"].as_array().unwrap();
        ensure!(times.len()==f0.len()&&f0.len()==uv.len(),"control_track requires equally sized frame_times_seconds, f0_hz and uv arrays");
        let mut previous=-1.;
        for ((t,hz),unvoiced) in times.iter().zip(f0).zip(uv){
            let(t,hz,unvoiced)=(number(t),number(hz),unvoiced.as_bool().unwrap());
            ensure!(t>previous&&t<=maximum+1e-12&&if unvoiced{hz==0.}else{hz>0.},"track times must increase and UV/F0 must agree");
            previous=t;
        }
    }
    Ok(())
}

#[cfg(test)]
mod music_tests {
    use super::*;
    use serde_json::json;

    fn request_value() -> Value {
        json!({"protocol":"ogd-worker/1","request_id":"music-contract","op":"render",
            "job_id":"music-contract","revision":0,"route":"verdant-voice",
            "payload":{"kind":"native_score","native_seed":10008,
                "score":{"segments":[{"vowel":"a","duration_seconds":0.3,"midi_note":64}]}}})
    }

    #[test]
    fn compact_phone_floor_is_voice_only_with_optional_music() {
        for frames in 1..=4 {
            let mut value = request_value();
            value["payload"]["score"]["segments"][0]["duration_seconds"] = json!(frames as f64*0.005);
            request(&value).unwrap();
            value["payload"]["voice_music"] = json!({"method":"verdant-voice-music-v1","sample_rate":48000});
            request(&value).unwrap();
            value["payload"].as_object_mut().unwrap().remove("voice_music");
            value["route"] = "C0".into();
            value["voice_ref"] = verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE.into();
            assert!(request(&value).is_err());
        }
        let mut value = request_value();
        value["payload"]["score"]["segments"][0]["duration_seconds"] = json!(0.005-5e-13);
        request(&value).unwrap();
        value["payload"]["score"]["segments"][0]["duration_seconds"] = json!(0.004999);
        assert!(request(&value).is_err());
    }

    #[test]
    fn voice_music_schema_and_runtime_admission_agree_on_explicit_rates() {
        let raw = request_value();
        request(&raw).unwrap();
        voice_music(&raw["payload"], "verdant-voice").unwrap();
        for sample_rate in [44100,48000] {
            let mut value = raw.clone();
            value["payload"]["voice_music"] = json!({"method":"verdant-voice-music-v1","sample_rate":sample_rate});
            request(&value).unwrap();
            voice_music(&value["payload"], "verdant-voice").unwrap();
            for route in ["C0","C1","A"] {
                value["route"] = route.into();
                if route == "C0" { value["voice_ref"] = verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE.into(); }
                assert!(request(&value).is_err(), "{route} accepted a verdant-voice profile");
                assert!(voice_music(&value["payload"], route).is_err());
            }
        }
    }

    #[test]
    fn voice_music_rejects_unknown_or_incomplete_profiles_before_asset_loading() {
        for profile in [json!(null), json!({}),
            json!({"method":"verdant-voice-music-v1","sample_rate":16000}),
            json!({"method":"verdant-voice-music-v1","sample_rate":48000.5}),
            json!({"method":"future-music","sample_rate":48000}),
            json!({"method":"verdant-voice-music-v1","sample_rate":48000,"strength":0.35})] {
            let mut value = request_value();
            value["payload"]["voice_music"] = profile;
            assert!(request(&value).is_err(), "{value}");
            assert!(voice_music(&value["payload"], "verdant-voice").is_err(), "{value}");
        }
    }

    #[test]
    fn explicit_source_note_ownership_has_the_same_schema_and_planner_type_contract() {
        let mut value = request_value();
        value["payload"]["score"]["segments"][0]["phoneme"] = json!({"canonical_ipa":"ɑ",
            "source_language":"en","input_alphabet":"ipa","native_key":"en:aa","support":"native",
            "source_note_id":"note-a"});
        request(&value).unwrap();
        verdant_synthesis::model::prepare_score_segments(&value["payload"]["score"], &json!({})).unwrap();
        for invalid in [json!(null),json!(""),json!(false),json!(3),json!([]),json!({})] {
            value["payload"]["score"]["segments"][0]["phoneme"]["source_note_id"] = invalid;
            assert!(request(&value).is_err());
            assert!(verdant_synthesis::model::prepare_score_segments(&value["payload"]["score"], &json!({})).is_err());
        }
    }

    #[test]
    fn native_target_pitch_is_admitted_before_io_without_clamping() {
        for route in ["verdant-voice","C0"] {
            for hz in [35.0,2000.0] {
                let mut value=request_value();value["route"]=json!(route);
                if route == "C0" { value["voice_ref"] = verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE.into(); }
                value["payload"]["score"]["segments"][0].as_object_mut().unwrap().remove("midi_note");
                value["payload"]["score"]["segments"][0]["f0_hz"]=json!(hz);
                value["payload"]["control_track"]=json!({"frame_times_seconds":[0.0,0.3],"f0_hz":[hz,0.0],"uv":[false,true]});
                request(&value).unwrap();
            }
            for bad in [34.999,2000.001] {
                let mut value=request_value();value["route"]=json!(route);
                if route == "C0" { value["voice_ref"] = verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE.into(); }
                value["payload"]["control_track"]=json!({"frame_times_seconds":[0.0,0.3],"f0_hz":[bad,0.0],"uv":[false,true]});
                assert!(request(&value).is_err());
            }
            for midi in [96,108] {
                let mut value=request_value();value["route"]=json!(route);
                if route == "C0" { value["voice_ref"] = verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE.into(); }
                value["payload"]["score"]["segments"][0]["midi_note"]=json!(midi);
                assert!(request(&value).is_err());
            }
        }
        let mut value=request_value();value["payload"]["control_track"]=json!({
            "frame_times_seconds":[0.0,0.3],"f0_hz":[440.0,1.0],"uv":[false,true]});
        assert!(request(&value).is_err(),"UV sentinels must remain exactly zero");
    }

}
