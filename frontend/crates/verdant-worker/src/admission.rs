//! Complete inbound admission is independent of the editor's serialization subset.
//! The checked-in public schema supplies structural constraints; the relational
//! checks below preserve the worker's clocks, voicing and route restrictions.
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
fn expression(value:&Value,maximum:f64,score:&Value)->Result<()> {
    let mut last_end=0.;
    for segment in value["segments"].as_array().unwrap(){
        let(start,end,onset)=(number(&segment["start_seconds"]),number(&segment["end_seconds"]),number(&segment["note_onset_seconds"]));
        ensure!(start>=last_end-1e-10&&start<=onset&&onset<end&&end<=maximum+1e-10,"expression intervals must be ordered, nonoverlapping and own their onset within the score");last_end=end;
        let mut latest = 0.0_f64;
        for points in segment["curves"].as_object().unwrap().values(){
            let mut previous=-1.;
            for point in points.as_array().unwrap(){let t=number(&point[0]);ensure!(t>previous,"controller point times must increase");previous=t;}
            latest = latest.max(previous);
        }
        if latest > maximum {
            // Knots use the authored note clock. A point beyond a trimmed
            // phrase endpoint can still determine interpolation before it.
            let matches_note = score.pointer("/phoneme_timing/notes").and_then(Value::as_array).is_some_and(|notes| {
                let index = notes.partition_point(|n| number(&n["note_onset_seconds"]) < onset - 1e-9);
                notes.get(index).is_some_and(|n| {
                    let beat = number(&n["note_onset_seconds"]);
                    let end_beat = number(&n["note_end_seconds"]);
                    (onset - beat).abs() <= 1e-9 && (start - (beat - number(&n["prefix_seconds"]))).abs() <= 1e-9
                        && (end - (end_beat - number(&n["tail_trim_seconds"]))).abs() <= 1e-9 && latest <= end_beat - beat + 1e-10
                })
            });
            ensure!(matches_note, "controller point exceeds the score duration and its authored timing interval");
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
    verdant_synthesis::parameters::validate_score_phoneme_timing(score)?;
    verdant_synthesis::parameters::validate_score_hierarchy(score)?;
    verdant_synthesis::parameters::validate_score_continuations(score)?;
    if let Some(value) = payload.get("expression") { expression(value, duration, score)?; }
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
    fn worker_request_accepts_nullable_nuclei_and_checks_the_optional_delay_type() {
        let mut value = request_value();
        value["payload"]["score"] = serde_json::from_str(include_str!(
            "../../verdant-synthesis/src/parameters/fixtures/constrained-phoneme-timing.json")).unwrap();
        request(&value).unwrap();
        verdant_synthesis::parameters::validate_score_phoneme_timing(&value["payload"]["score"]).unwrap();
        for (field, invalid) in [("nucleus_segment_index", json!(false)), ("nucleus_segment_index", json!(1.5)),
            ("nucleus_delay_seconds", Value::Null), ("nucleus_delay_seconds", json!(-0.001))] {
            let mut bad = value.clone();
            bad["payload"]["score"]["phoneme_timing"]["notes"][1][field] = invalid;
            assert!(request(&bad).is_err(), "{field}");
            assert!(verdant_synthesis::parameters::validate_score_phoneme_timing(&bad["payload"]["score"]).is_err());
        }
        let note = &mut value["payload"]["score"]["phoneme_timing"]["notes"][0];
        note.as_object_mut().unwrap().remove("nucleus_segment_index");
        assert!(request(&value).is_err());
        assert!(verdant_synthesis::parameters::validate_score_phoneme_timing(&value["payload"]["score"]).is_err());
    }

    #[test]
    fn a_curve_knot_after_the_phrase_end_requires_its_own_timing_interval() {
        use verdant_backend::pinyin::{final_spec, initial_spec, parse_token};
        let ma = parse_token("ma1").unwrap();
        let m = initial_spec(&ma, 0.5).unwrap();
        let (_, a) = final_spec(&ma, 0.5);
        let mut value = request_value();
        value["payload"]["score"] = json!({"segments":[
            {"context":m.context,"voiced":m.voiced,"midi_note":60,"duration_seconds":0.05},
            {"context":a,"voiced":true,"midi_note":60,"duration_seconds":0.39}],
            "phoneme_timing":{"method":"vowel-anchor-v1","notes":[{"note_id":"ma","note_onset_seconds":0.05,
                "note_end_seconds":0.55,"first_segment_index":0,"nucleus_segment_index":1,"end_segment_index":2,
                "prefix_seconds":0.05,"tail_trim_seconds":0.11}]}});
        value["payload"]["expression"] = json!({"method":"vocaloid-controls-v1","segments":[{
            "start_seconds":0.0,"end_seconds":0.44,"note_onset_seconds":0.05,"curves":{"BRI":[[0,12],[0.5,108]]}}]});
        let score = &value["payload"]["score"];
        let curve = &value["payload"]["expression"];
        request(&value).unwrap();
        verdant_synthesis::parameters::validate_score_phoneme_timing(score).unwrap();
        expression(curve, 0.44, score).unwrap();

        for (field, different) in [("start_seconds", 0.01), ("note_onset_seconds", 0.06), ("end_seconds", 0.43)] {
            let mut unrelated = curve.clone();
            unrelated["segments"][0][field] = json!(different);
            assert!(expression(&unrelated, 0.44, score).is_err(), "mismatched {field}");
        }
        let mut too_late = curve.clone();
        too_late["segments"][0]["curves"]["BRI"][1][0] = json!(0.501);
        assert!(expression(&too_late, 0.44, score).is_err());
        let mut no_timing = score.clone();
        no_timing.as_object_mut().unwrap().remove("phoneme_timing");
        assert!(expression(curve, 0.44, &no_timing).is_err());
        let mut in_range = curve.clone();
        in_range["segments"][0]["curves"]["BRI"][1][0] = json!(0.44);
        expression(&in_range, 0.44, &no_timing).unwrap();
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
