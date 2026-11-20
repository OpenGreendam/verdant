//! Preserve the existing frontend/MLPG clock without running legacy waveform DSP.
use anyhow::{ensure,Context,Result};
use serde_json::Value;
use std::borrow::Cow;
use crate::{Arrays,NativePlan,NumericArray};

pub const RESOLVED_NEIGHBOR_ENCODING: &str = "resolved-context-phone-v2";

pub fn neighbor_encoding_from_contracts(contracts: &Value) -> Result<&str> {
    let encoding = contracts.get("neighbor_encoding").and_then(Value::as_str)
        .context("verdant-voice requires an explicit neighbor encoding")?;
    ensure!(encoding == RESOLVED_NEIGHBOR_ENCODING,"verdant-voice requires resolved-context-phone-v2");
    Ok(encoding)
}

fn context_phones(features: &Value) -> Result<[i64; 3]> {
    let resolved = crate::model::resolve_context_phones(features)?;
    let mut result = [0; 3];
    for (index, position) in ["L", "C", "R"].into_iter().enumerate() {
        if let Some(value) = features.get(format!("{position}_SilSp")) {
            ensure!(matches!(value.as_str(), Some("sil" | "sp")) || (index != 1 && value.as_str() == Some("XX")),
                "Verdant ONNX {position}_SilSp must name sil, sp or an absent neighbor");
        }
        let declared = features.get("OriginalHierarchy").is_some() || ["Phone", "YM", "SM", "Vowel", "Stop", "Nasal", "Fric", "SilSp"]
            .iter().any(|suffix| features.get(format!("{position}_{suffix}")).is_some());
        let value = match resolved[index] {
            Some(value) => value,
            None => {
                ensure!(index != 1 && !declared, "Verdant ONNX {position} phone context does not resolve to one original identity");
                0
            }
        };
        ensure!((0..=255).contains(&value) && (index != 1 || value > 0), "Verdant ONNX native phone ID outside ABI");
        result[index] = i64::from(value);
    }
    Ok(result)
}

fn expand_compact_features<'a>(features: &'a Value, metadata: &Value) -> Result<Cow<'a, Value>> {
    let Some(reference) = features.get("OriginalHierarchy") else { return Ok(Cow::Borrowed(features)); };
    let Some(hash) = reference.get("graph_sha256") else { return Ok(Cow::Borrowed(features)); };
    let hash = hash.as_str().context("Verdant ONNX hierarchy graph reference must be a string")?;
    ensure!(reference.get("nodes").is_none(), "Verdant ONNX hierarchy cannot contain both nodes and a compact reference");
    let graph = metadata.get("original_hierarchy_graphs").and_then(|graphs| graphs.get(hash))
        .context("Verdant ONNX compact hierarchy graph is missing")?;
    ensure!(graph["schema_version"] == reference["schema_version"] && graph["nodes"].is_array(),
        "Verdant ONNX compact hierarchy graph has incompatible schema/nodes");
    let mut expanded = features.clone();
    expanded["OriginalHierarchy"]["nodes"] = graph["nodes"].clone();
    Ok(Cow::Owned(expanded))
}

pub fn conditions_for_package(plan: &NativePlan, package: &crate::model::VoicePackage) -> Result<Arrays> {
    neighbor_encoding_from_contracts(&package.contracts)?;
    conditions_from_plan(plan)
}

/// Retain the same native planning run's raw MLPG pitch for an explicitly
/// admitted reference-controller strategy. This is opt-in: historical
/// conditions and their archive keys remain unchanged. The consuming model
/// admits its reference range; no clipping or target-pitch substitution occurs.
pub fn native_reference_from_plan(plan: &NativePlan) -> Result<NumericArray> {
    ensure!(plan.sample_rate == 16000 && plan.hop_samples == 80
        && (1..=24000).contains(&plan.duration_frames),
        "native reference requires the original 16 kHz/hop80 frame clock");
    let mut reference = vec![0.; plan.duration_frames];
    let mut assigned = vec![false; plan.duration_frames];
    for run in &plan.runs {
        let end = run.start_frame.checked_add(run.frames).context("native reference run frame overflow")?;
        ensure!(run.frames > 0 && end <= plan.duration_frames
            && run.controls.voiced.len() == run.frames,
            "native reference run clock differs from target voicing");
        let raw = run.arrays.get("model_f0_hz_assuming_natural_log")
            .context("native reference requires the original unclipped MLPG trajectory")?;
        ensure!(raw.dtype == "float64" && raw.shape == [run.frames]
            && raw.values.len() == run.frames && raw.values.iter().all(|value| value.is_finite()),
            "native MLPG reference must be finite float64 on the run clock");
        for local in 0..run.frames {
            let frame = run.start_frame + local;
            ensure!(!assigned[frame], "overlapping native reference runs");
            assigned[frame] = true;
            if run.controls.voiced[local] {
                let value = raw.values[local] as f32 as f64;
                ensure!(value.is_finite() && value > 0., "native MLPG reference must be positive on voiced frames");
                reference[frame] = value;
            }
        }
    }
    NumericArray::new(vec![plan.duration_frames], "float32", reference)
}

fn put(arrays:&mut Arrays,key:&str,shape:Vec<usize>,dtype:&str,values:Vec<f64>)->Result<()>{
    arrays.insert(key.into(),NumericArray::new(shape,dtype,values)?);Ok(())
}
fn integer(value:&Value,label:&str)->Result<usize>{usize::try_from(value.as_u64().with_context(||format!("Verdant ONNX {label} must be an integer"))?).context("Verdant ONNX frame index overflow")}
pub fn conditions_from_plan(plan:&NativePlan)->Result<Arrays>{
    ensure!(plan.sample_rate==16000&&plan.hop_samples==80&&(1..=24000).contains(&plan.duration_frames),"verdant-voice requires 1..24000 frames at 16 kHz/hop80");
    let n=plan.duration_frames;
    let mut latent=vec![0.;n*25];let mut phones=vec![0.;n*3];let mut states=vec![-1.;n];let mut languages=vec![0.;n];
    let mut prosody=vec![0.;n*8];let mut f0=vec![0.;n];let mut voiced=vec![0.;n];let mut active=vec![0.;n];let mut segment_ids=vec![-1.;n];
    for run in &plan.runs{
        let end=run.start_frame.checked_add(run.frames).context("Verdant ONNX run frame overflow")?;ensure!(end<=n,"Verdant ONNX run extends beyond score");
        let array=|key:&str,width:usize|->Result<&[f64]>{let a=run.arrays.get(key).with_context(||format!("native conditions lack {key}"))?;
            let expected=if width==1{vec![run.frames]}else{vec![run.frames,width]};ensure!(a.shape==expected,"native {key} shape mismatch");Ok(&a.values)};
        let lsf=array("lsf_radians",24)?;let gain=array("generated_log_gain",1)?;let state=array("state_index",1)?;let ids=array("segment_index",1)?;
        ensure!(run.controls.f0_hz.len()==run.frames&&run.controls.voiced.len()==run.frames,"Verdant ONNX native control length mismatch");
        for local in 0..run.frames{
            let frame=run.start_frame+local;ensure!(active[frame]==0.,"overlapping Verdant ONNX acoustic runs");
            for k in 0..24{latent[frame*25+k]=lsf[local*24+k] as f32 as f64;}latent[frame*25+24]=gain[local] as f32 as f64;
            states[frame]=state[local];segment_ids[frame]=ids[local];active[frame]=1.;
            f0[frame]=run.controls.f0_hz[local] as f32 as f64;voiced[frame]=if run.controls.voiced[local]{1.}else{0.};
            prosody[frame*8]=if run.controls.voiced[local]{(f0[frame]/300.).ln() as f32 as f64}else{0.};
            prosody[frame*8+1]=voiced[frame];
        }
    }
    let reports=plan.metadata["segments"].as_array().context("Verdant ONNX plan missing segment provenance")?;
    let mut assigned=vec![false;n];
    for(index,report)in reports.iter().enumerate(){
        let first=integer(&report["first_frame"],"segment start")?;let count=integer(&report["frame_count"],"segment length")?;
        ensure!(count>0&&first.checked_add(count).is_some_and(|end|end<=n),"Verdant ONNX segment exceeds score");
        let features=&report["initial_context"]["features"];
        let features=expand_compact_features(features,&plan.metadata)?;
        let context=context_phones(&features)?;
        let language=report.pointer("/request/phoneme/source_language").and_then(Value::as_str)
            .or_else(||report.pointer("/native_identity/native_language").and_then(Value::as_str));
        let language=match language{Some("zh")=>1.,Some("en")=>2.,Some("ja")=>3.,_=>0.};
        // A continuation is one acoustic phone, not a fresh phone or state
        // clock. Per-note ownership remains in frame_segment_index.
        let source_frames=report.get("acoustic_source_frame_count").map(|v|integer(v,"source frame count")).transpose()?.unwrap_or(count);
        let source_offset=report.get("acoustic_source_frame_offset").map(|v|integer(v,"source frame offset")).transpose()?.unwrap_or(0);
        let counts=report.pointer("/state_duration_allocation/source_counts").unwrap_or(&report["state_frames"]).as_array().context("Verdant ONNX state allocation missing")?;
        ensure!(counts.len()==5,"Verdant ONNX requires original five-state allocation");
        let counts=counts.iter().map(|v|integer(v,"state length")).collect::<Result<Vec<_>>>()?;
        ensure!(counts.iter().sum::<usize>()==source_frames&&source_offset+count<=source_frames,"Verdant ONNX source-phone state budget differs");
        let mut starts=[0usize;5];for s in 1..5{starts[s]=starts[s-1]+counts[s-1];}
        for local in 0..count{
            let frame=first+local;ensure!(active[frame]==1.&&!assigned[frame]&&segment_ids[frame]==index as f64,"Verdant ONNX segment ownership/clock mismatch");assigned[frame]=true;
            let state=states[frame];ensure!(state.fract()==0.&&(0. ..=4.).contains(&state),"Verdant ONNX state outside five-state ABI");let state=state as usize;
            let offset=source_offset+local;ensure!(counts[state]>0&&(starts[state]..starts[state]+counts[state]).contains(&offset),"Verdant ONNX frame disagrees with original state allocation");
            phones[frame*3..frame*3+3].copy_from_slice(&context.map(|v|v as f64));languages[frame]=language;
            prosody[frame*8+2]=((offset as f64+0.5)/source_frames as f64) as f32 as f64;
            prosody[frame*8+3]=(((offset-starts[state]) as f64+0.5)/counts[state] as f64) as f32 as f64;
            prosody[frame*8+4]=(source_frames as f64*0.005/0.3).ln() as f32 as f64;
            prosody[frame*8+5]=(counts[state] as f64*0.005/0.06).ln() as f32 as f64;
        }
    }
    ensure!(assigned.iter().zip(&active).all(|(&a,&b)|a==(b==1.)),"Verdant ONNX active frames lack source conditions");
    // NumericArray retains float32 rounding explicitly, even in its f64 host
    // storage. There is no latent or phoneme-dependent template lookup here.
    for v in &mut prosody{*v=*v as f32 as f64;}
    let mut arrays=Arrays::new();
    for(key,shape,dtype,values)in[
        ("latent25",vec![n,25],"float32",latent),("phone_ids",vec![n,3],"int64",phones),
        ("states",vec![n],"int64",states),("languages",vec![n],"int64",languages),
        ("prosody",vec![n,8],"float32",prosody),("f0_hz",vec![n],"float32",f0),
        ("voiced",vec![n],"bool",voiced),("active",vec![n],"bool",active),
        ("frame_segment_index",vec![n],"int64",segment_ids)]{put(&mut arrays,key,shape,dtype,values)?;}
    put(&mut arrays, "neighbor_encoding_version", vec![1], "int64", vec![2.])?;
    super::validate_conditions(&arrays)?;Ok(arrays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reference_plan() -> NativePlan {
        let mut arrays = Arrays::new();
        arrays.insert("model_f0_hz_assuming_natural_log".into(),
            NumericArray::new(vec![4], "float64", vec![333., 800.01, 155., 300.]).unwrap());
        NativePlan { duration_frames:6, sample_rate:16000, hop_samples:80, metadata:json!({}),
            runs:vec![crate::RunParameters { start_frame:1, frames:4, arrays, metadata:json!({}),
                controls:crate::AcousticControlTrack { f0_hz:vec![220.,110.,0.,0.],
                    voiced:vec![true,true,false,false], phonetic_classes:vec![], phone_identity:vec![],
                    segment_identity:vec![], continuity_identity:vec![], period_seconds:0.005, origin_seconds:0.005 } }] }
    }

    #[test]
    fn native_reference_keeps_raw_mlpg_and_only_intersects_final_uv() -> Result<()> {
        let plan=reference_plan();
        let before=serde_json::to_value(&plan.runs[0].arrays)?;
        let reference=native_reference_from_plan(&plan)?;
        assert_eq!(reference.dtype,"float32");
        assert_eq!(reference.values,[0.,333.,800.01_f32 as f64,0.,0.,0.]);
        assert!(reference.values[2]>800.,"the planner must not clip a model reference");
        assert_eq!(plan.runs[0].controls.f0_hz,[220.,110.,0.,0.]);
        assert_eq!(serde_json::to_value(&plan.runs[0].arrays)?,before);
        Ok(())
    }

    #[test]
    fn native_reference_rejects_missing_raw_source_and_wrong_clocks() {
        let mut missing=reference_plan();missing.runs[0].arrays.clear();assert!(native_reference_from_plan(&missing).is_err());
        let mut overlap=reference_plan();overlap.runs.push(overlap.runs[0].clone());assert!(native_reference_from_plan(&overlap).is_err());
        let mut bad=reference_plan();bad.runs[0].controls.voiced.pop();assert!(native_reference_from_plan(&bad).is_err());
        let mut bad=reference_plan();bad.runs[0].start_frame=4;assert!(native_reference_from_plan(&bad).is_err());
        let mut bad=reference_plan();bad.runs[0].arrays.get_mut("model_f0_hz_assuming_natural_log").unwrap().dtype="float32".into();assert!(native_reference_from_plan(&bad).is_err());
        let mut bad=reference_plan();bad.runs[0].arrays.get_mut("model_f0_hz_assuming_natural_log").unwrap().values[0]=0.;assert!(native_reference_from_plan(&bad).is_err());
    }

    #[test]
    fn resolved_neighbors_match_shared_independent_fixtures() {
        let fixtures: Value = serde_json::from_str(include_str!("fixtures/neighbor-contexts-v2.json")).unwrap();
        for case in fixtures["valid"].as_array().unwrap() {
            let expected: [i64; 3] = serde_json::from_value(case["phones"].clone()).unwrap();
            let features = case["features"].clone();
            assert_eq!(context_phones(&features).unwrap(), expected, "{}", case["name"]);
            assert_eq!(features, case["features"]);
        }
        for case in fixtures["invalid"].as_array().unwrap() {
            assert!(context_phones(&case["features"]).is_err(), "{}", case["name"]);
        }
    }

    #[test]
    fn current_neighbor_encoding_is_required() {
        assert!(neighbor_encoding_from_contracts(&json!({})).is_err());
        assert_eq!(neighbor_encoding_from_contracts(&json!({"neighbor_encoding":RESOLVED_NEIGHBOR_ENCODING})).unwrap(),RESOLVED_NEIGHBOR_ENCODING);
        for invalid in [Value::Null,json!(false),json!("future-unrecognized")] {
            assert!(neighbor_encoding_from_contracts(&json!({"neighbor_encoding":invalid})).is_err());
        }
    }

    #[test]
    fn compact_original_hierarchy_retains_neighbor_ids_and_rejects_missing_graph() {
        let report = json!({"segments": [{"initial_context": {"features": {"C_Phone": 3, "OriginalHierarchy": {
            "schema_version": "ogd-explicit-original-hierarchy/1", "current_index": 1, "nodes": [
                {"phone": 53, "left_level": 5, "right_level": 0},
                {"phone": 3, "left_level": 0, "right_level": 0},
                {"phone": 58, "left_level": 0, "right_level": 5}]}}}}]});
        let compact = crate::model::compact_hierarchy_report(&report, None).unwrap();
        let features = &compact["segments"][0]["initial_context"]["features"];
        assert!(features["OriginalHierarchy"].get("nodes").is_none());
        let expanded = expand_compact_features(features, &compact).unwrap();
        assert_eq!(context_phones(&expanded).unwrap(), [53, 3, 58]);
        assert!(expand_compact_features(features, &json!({})).is_err());
        assert!(features["OriginalHierarchy"].get("nodes").is_none());
    }

    fn ma_score() -> Value {
        json!({"segments": [
            {"context": {"features": {"C_SM": "m", "C_Phone": 53, "C_Tone": 1}},
                "duration_seconds": 0.08, "voiced": true, "midi_note": 60},
            {"context": {"features": {"C_YM": "a", "C_Phone": 3, "L_SM": "m", "C_Tone": 1}},
                "duration_seconds": 0.285, "voiced": true, "midi_note": 60}]})
    }

    #[test]
    fn preparation_preserves_named_overrides_and_continuation_neighbors() {
        let mut score = ma_score();
        let mut continued = score["segments"][1].clone();
        continued["duration_seconds"] = 0.2.into();
        continued["midi_note"] = 64.into();
        continued["continuation_of"] = 1.into();
        score["segments"].as_array_mut().unwrap().push(continued);
        let prepared = crate::model::prepare_score_segments(&score, &json!({})).unwrap();
        for segment in &prepared[1..] {
            assert_eq!(context_phones(&segment["context"]["features"]).unwrap(), [53, 3, 0]);
        }
        for segment in score["segments"].as_array_mut().unwrap().iter_mut().skip(1) {
            segment["context"]["features"]["L_SM"] = "s".into();
        }
        let prepared = crate::model::prepare_score_segments(&score, &json!({})).unwrap();
        for segment in &prepared[1..] {
            assert_eq!(context_phones(&segment["context"]["features"]).unwrap(), [58, 3, 0]);
            assert!(segment["context"]["features"].get("L_Phone").is_none());
        }
    }

    #[test]
    #[ignore = "requires explicit VERDANT_VOICE_TEST_TOKEN and VERDANT_VOICE_TEST_MODEL local source-bank paths"]
    fn actual_bank_native_release_context_keeps_original_target_and_following_note_clock()->Result<()> {
        let voice=crate::model::load_raw_voice(
            &std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_TOKEN")?),
            &std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_MODEL")?))?;
        let options=json!({"prosody_mode":"step","voicing_mode":"authored"});
        let ordinary=ma_score();
        let old=crate::parameters::plan_voice_score(&voice,&ordinary,None,&options)?;
        let reference=crate::parameters::plan_native_reference_score(&voice,&ordinary,None,&options)?;
        assert_eq!(old.metadata,reference.metadata);
        assert_eq!(serde_json::to_value(conditions_from_plan(&old)?)?,
            serde_json::to_value(conditions_from_plan(&reference)?)?);

        let first=json!({"context":{"features":{"C_Phone":174,"C_Vowel":"aa","C_Language":3,"C_Stress":0},
            "answers":{"C_EN":true,"C_SM":false,"C_SilSp":false,"C_Vowel":true,"C_Stop":false,"C_Nasal":false,"C_Fric":false},
            "anchor_questions":["C_Vowel==aa"]},"duration_seconds":0.32,"midi_note":69,"voiced":true,
            "phoneme":{"source_language":"en","native_key":"en:aa","length":1}});
        let mut last=first.clone();last["duration_seconds"]=json!(0.025);last["midi_note"]=json!(57);last["continuation_of"]=json!(0);
        for following in [false,true] {
            let mut score=json!({"segments":[first,last]});
            if following {score["segments"].as_array_mut().unwrap().push(ma_score()["segments"][1].clone());}
            let target_plan=crate::parameters::plan_voice_score(&voice,&score,None,&options)?;
            let target=conditions_from_plan(&target_plan)?;
            let before=serde_json::to_value(&target)?;
            let reference_plan=crate::parameters::plan_native_reference_score(&voice,&score,None,&options)?;
            let acoustic=conditions_from_plan(&reference_plan)?;
            let raw=native_reference_from_plan(&reference_plan)?;
            let (_,report)=super::super::controller_conditions(&target,&raw,Some(&acoustic))?;
            assert_eq!(target_plan.duration_frames,reference_plan.duration_frames);
            assert_eq!(target_plan.metadata["time_map_render_seconds"],reference_plan.metadata["time_map_render_seconds"]);
            assert_eq!(target_plan.metadata["time_map_authored_seconds"],reference_plan.metadata["time_map_authored_seconds"]);
            assert_eq!(target["frame_segment_index"].values,acoustic["frame_segment_index"].values);
            assert_eq!(target["f0_hz"].values,acoustic["f0_hz"].values);
            assert_eq!(serde_json::to_value(&target)?,before);
            if following {
                assert_eq!(acoustic["frame_segment_index"].values[69],2.);
                assert_eq!(reference_plan.metadata["segments"][2]["first_frame"],69);
            } else {
                assert_eq!(target_plan.duration_frames,69);
                assert_eq!(reference_plan.metadata["segments"][1]["acoustic_source_frame_count"],103);
                assert_eq!(reference_plan.metadata["segments"][1]["acoustic_source_emitted_frame_count"],69);
                assert_eq!(reference_plan.metadata["segments"][1]["state_duration_allocation"]["source_counts"],json!([5,62,18,10,8]));
                assert_eq!(report["acoustic_reference_changed_fields"],json!(["latent25","states","prosody"]));
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires explicit VERDANT_VOICE_TEST_TOKEN and VERDANT_VOICE_TEST_MODEL local source-bank paths"]
    fn actual_bank_compact_phones_keep_conditions_voicing_and_music_clock() -> Result<()> {
        let voice = crate::model::load_raw_voice(
            &std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_TOKEN")?),
            &std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_MODEL")?))?;
        for frames in 1..=4 {
            let mut score = ma_score();
            score["segments"][1]["duration_seconds"] = json!(frames as f64*0.005);
            assert!(crate::parameters::plan_score(&voice,&score,None,&json!({})).is_err());
            for mode in ["authored","original-model"] {
                let plan = crate::parameters::plan_voice_score(&voice,&score,None,&json!({"voicing_mode":mode}))?;
                assert_eq!(plan.duration_frames,16+frames);
                assert_eq!(plan.metadata["timing"]["minimum_segment_frames"],1);
                let conditions = conditions_from_plan(&plan)?;
                assert_eq!(super::super::validate_conditions(&conditions)?,16+frames);
                for frame in 16..16+frames {
                    assert_eq!(&conditions["phone_ids"].values[frame*3..frame*3+3],&[53.,3.,0.]);
                }
            }
        }
        let mut chain = ma_score();
        chain["segments"][1]["duration_seconds"] = json!(0.005);
        for _ in 0..3 {
            let mut continued = chain["segments"][1].clone();
            continued["continuation_of"] = json!(1);
            chain["segments"].as_array_mut().unwrap().push(continued);
        }
        let plan = crate::parameters::plan_voice_score(&voice,&chain,None,&json!({}))?;
        let conditions = conditions_from_plan(&plan)?;
        assert_eq!(super::super::validate_conditions(&conditions)?,20);
        assert_eq!(&conditions["frame_segment_index"].values[16..],&[1.,2.,3.,4.]);
        assert_eq!(&conditions["states"].values[16..],&[0.,1.,2.,4.]);
        let score = ma_score();
        let mut legacy = crate::parameters::plan_score(&voice,&score,None,&json!({}))?;
        let compact = crate::parameters::plan_voice_score(&voice,&score,None,&json!({}))?;
        assert_eq!(legacy.metadata["timing"]["minimum_segment_frames"],5);
        legacy.metadata["timing"]["minimum_segment_frames"] = json!(1);
        assert_eq!(legacy.metadata,compact.metadata);
        for (old,new) in legacy.runs.iter().zip(&compact.runs) {
            assert_eq!(old.arrays.len(),new.arrays.len());
            for (key,array) in &old.arrays {
                assert_eq!(array.shape,new.arrays[key].shape);
                assert_eq!(array.dtype,new.arrays[key].dtype);
                assert_eq!(array.values.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),new.arrays[key].values.iter().map(|v|v.to_bits()).collect::<Vec<_>>());
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires explicit VERDANT_VOICE_TEST_TOKEN and VERDANT_VOICE_TEST_MODEL local source-bank paths"]
    fn actual_bank_ma_and_continuation_keep_non_identity_arrays() -> Result<()> {
        let token = std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_TOKEN")?);
        let model = std::path::PathBuf::from(std::env::var("VERDANT_VOICE_TEST_MODEL")?);
        let voice = crate::model::load_raw_voice(&token, &model)?;
        for continuation in [false, true] {
            let mut score = ma_score();
            if continuation {
                let mut continued = score["segments"][1].clone();
                continued["duration_seconds"] = 0.2.into();
                continued["midi_note"] = 64.into();
                continued["continuation_of"] = 1.into();
                score["segments"].as_array_mut().unwrap().push(continued);
            }
            let plan = crate::parameters::plan_score(&voice, &score, None, &json!({}))?;
            let resolved = conditions_from_plan(&plan)?;
            assert_eq!(resolved["neighbor_encoding_version"].values, vec![2.]);
            let mut final_frames = 0;
            for (index, &segment) in resolved["frame_segment_index"].values.iter().enumerate() {
                if segment >= 1. {
                    final_frames += 1;
                    assert_eq!(&resolved["phone_ids"].values[index * 3..index * 3 + 3], &[53., 3., 0.]);
                }
            }
            assert!(final_frames > 0);
        }
        Ok(())
    }
}
