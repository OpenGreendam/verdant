use std::{fs,path::Path};
use anyhow::{Context,Result,bail,ensure};
use serde_json::{Value,json};
use verdant_synthesis::{AcousticControlTrack,Arrays,AudioResult,NativePlan,NumericArray,
    dsp,options,output,parameters};
use verdant_synthesis::voice::{self,native_voice};
use crate::{Config,artifacts::{self,array,scalar},capabilities,runtime};

fn stage(request:&Value,name:&str,phase:&str)->Result<()>{crate::emit("progress",Some(request),json!({"stage":name,"phase":phase,"progress_kind":"stage_boundary","fraction":null}))}
fn options(payload:&Value)->Value{
    let mut value=payload.pointer("/score/synthesis").and_then(Value::as_object).cloned().unwrap_or_default();
    for name in ["vocal_tract","model_dynamics","consonant_dynamics","expression","output_transients"]{if let Some(option)=payload.get(name){value.insert(name.into(),option.clone());}}
    value.insert("threads".into(),3.into());Value::Object(value)
}
fn validate_audio(audio:&AudioResult)->Result<()>{ensure!(audio.sample_rate>0&&!audio.samples.is_empty()&&audio.samples.iter().all(|x|x.is_finite()),"stage returned empty or nonfinite audio");Ok(())}
fn maps(plan:&NativePlan)->Result<(Vec<f64>,Vec<f64>)>{
    let get=|key:&str|->Result<Vec<f64>>{plan.metadata[key].as_array().context(format!("native plan missing {key}"))?.iter().map(|n|n.as_f64().context("time-map number")).collect()};
    let(render,authored)=(get("time_map_render_seconds")?,get("time_map_authored_seconds")?);
    ensure!(render.len()>=2&&render.len()==authored.len()&&render[0]==0.&&authored[0]==0.&&render.windows(2).all(|p|p[1]>=p[0])&&authored.windows(2).all(|p|p[1]>=p[0]),"invalid native time map");Ok((render,authored))
}
fn merge_arrays(destination:&mut Arrays,source:Arrays,start_frame:usize,frames:usize,duration_frames:usize,hop_samples:usize)->Result<()>{
    for (key,value) in source{
        if matches!(key.as_str(),"calibration_reference"|"calibration_reference_waveform")||key.starts_with('_'){continue;}
        ensure!(!value.shape.is_empty(),"run array {key} must have a leading time axis");
        let scale=if value.shape[0]==frames{1}else if value.shape[0]==frames*hop_samples{hop_samples}else{bail!("run array {key} has an unrecognized clock")};
        let row=value.shape[1..].iter().product::<usize>();
        let entry=destination.entry(key.clone()).or_insert_with(||{
            let mut shape=value.shape.clone();shape[0]=duration_frames*scale;
            let initial=if key=="uv"||key=="model_dynamics_sample_gain"{1.}else if matches!(key.as_str(),"state_index"|"segment_index"|"run_index"){-1.}else{0.};
            NumericArray{values:vec![initial;shape.iter().product()],shape,dtype:value.dtype.clone()}
        });
        ensure!(entry.shape[1..]==value.shape[1..]&&entry.dtype==value.dtype&&entry.shape[0]==duration_frames*scale,"run array {key} changed shape/dtype");
        let start=start_frame*scale*row;let end=start+value.values.len();ensure!(end<=entry.values.len(),"run array {key} exceeds phrase timeline");entry.values[start..end].copy_from_slice(&value.values);
    }Ok(())
}
fn fill_gaps(arrays:&mut Arrays,plan:&NativePlan)->Result<()>{
    for (i,run) in plan.runs.iter().enumerate(){
        let start=run.start_frame+run.frames;let end=plan.runs.get(i+1).map_or(plan.duration_frames,|r|r.start_frame);
        ensure!(end>=start,"overlapping native runs");if start==end{continue;}
        for (key,source_key) in [("lsf_radians","lsf_radians"),("lsf_radians_before_projection","lsf_radians"),
            ("lsf_radians_before_phonetic_correction","lsf_radians_before_phonetic_correction"),("lsf_radians_before_vocal_tract","lsf_radians_before_vocal_tract"),
            ("lpc_coefficients","lpc_coefficients"),("parcor_coefficients","parcor_coefficients")]{
            let Some(source)=arrays.get(source_key)else{continue;};let row=source.shape[1..].iter().product::<usize>();let held=source.values[(start-1)*row..start*row].to_vec();
            if let Some(array)=arrays.get_mut(key){ensure!(array.shape[0]==plan.duration_frames,"held gap array is not frame-aligned");for target in array.values[start*row..end*row].chunks_exact_mut(row){target.copy_from_slice(&held);}}
        }
    }Ok(())
}
fn controls(plan:&NativePlan)->Result<AcousticControlTrack>{
    let n=plan.duration_frames;
    let mut track=AcousticControlTrack{f0_hz:vec![0.;n],voiced:vec![false;n],phonetic_classes:vec!["silence".into();n],phone_identity:vec![0;n],segment_identity:vec![-1;n],continuity_identity:vec![-1;n],period_seconds:plan.hop_samples as f64/plan.sample_rate as f64,origin_seconds:0.};
    for run in &plan.runs{
        let start=run.start_frame;let end=start+run.frames;ensure!(end<=n&&run.controls.f0_hz.len()==run.frames&&run.controls.voiced.len()==run.frames&&run.controls.phonetic_classes.len()==run.frames&&run.controls.phone_identity.len()==run.frames&&run.controls.segment_identity.len()==run.frames&&run.controls.continuity_identity.len()==run.frames,"native run control length mismatch");
        track.f0_hz[start..end].copy_from_slice(&run.controls.f0_hz);track.voiced[start..end].copy_from_slice(&run.controls.voiced);
        track.phonetic_classes[start..end].clone_from_slice(&run.controls.phonetic_classes);track.phone_identity[start..end].copy_from_slice(&run.controls.phone_identity);
        track.segment_identity[start..end].copy_from_slice(&run.controls.segment_identity);track.continuity_identity[start..end].copy_from_slice(&run.controls.continuity_identity);
    }Ok(track)
}
fn native(config:&Config,request:&Value,folder:&Path,opts:&Value)->Result<(AudioResult,AcousticControlTrack,(Vec<f64>,Vec<f64>))>{
    let payload=&request["payload"];let seed=payload.get("native_seed").and_then(Value::as_u64).unwrap_or(10008);
    let voice=runtime::load_original(config)?;
    let mut plan=parameters::plan_score(&voice,&payload["score"],payload.get("control_track").filter(|v|!v.is_null()),opts)?;
    ensure!(plan.sample_rate>0&&plan.hop_samples>0&&!plan.runs.is_empty()&&plan.duration_frames>0,"native plan has no acoustic frames");
    let mapping=maps(&plan)?;let control=controls(&plan)?;
    let mut samples=vec![0.;plan.duration_frames.checked_mul(plan.hop_samples).context("native sample count overflow")?];
    let mut arrays=Arrays::new();let mut reports=Vec::new();
    for i in 0..plan.runs.len(){
        let run=&mut plan.runs[i];
        options::apply_spectral_options(run,opts)?;
        let first=run.metadata["first_segment_index"].as_u64().context("run missing first_segment_index seed provenance")?;
        let mut rendered=dsp::synthesize_run(run,opts,seed.checked_add(first).context("run seed overflow")?)?;
        options::apply_phonetic_waveform(&mut rendered,run)?;
        dsp::calibrate_run(&mut rendered,run,opts)?;
        options::apply_waveform_options(&mut rendered,run,opts)?;
        validate_audio(&rendered)?;
        ensure!(rendered.sample_rate==plan.sample_rate&&rendered.samples.len()==run.frames*plan.hop_samples,"DSP changed native run dimensions");
        let start=run.start_frame*plan.hop_samples;let end=start+rendered.samples.len();ensure!(end<=samples.len(),"native run extends past frame plan");samples[start..end].copy_from_slice(&rendered.samples);
        let mut run_arrays=std::mem::take(&mut run.arrays);for(key,value)in rendered.arrays{run_arrays.insert(key,value);}
        let mut report=run.metadata.clone();report["first_frame"]=run.start_frame.into();report["frame_count"]=run.frames.into();report["seed"]=(seed+first).into();if let Some(metadata)=rendered.metadata.as_object(){for(key,value)in metadata{report[key]=value.clone();}}
        reports.push(report);
        // Borrow the immutable timing fields only after the mutable run scope ends.
        let run=&plan.runs[i];merge_arrays(&mut arrays,run_arrays,run.start_frame,run.frames,plan.duration_frames,plan.hop_samples)?;
    }
    fill_gaps(&mut arrays,&plan)?;
    let n=plan.duration_frames;
    arrays.insert("frame_times_seconds".into(),array(vec![n],"float64",(0..n).map(|i|(i as f64+0.5)*plan.hop_samples as f64/plan.sample_rate as f64).collect())?);
    arrays.insert("times_seconds".into(),arrays["frame_times_seconds"].clone());
    arrays.insert("sample_rate".into(),scalar(plan.sample_rate as f64,"int64")?);arrays.insert("hop_samples".into(),scalar(plan.hop_samples as f64,"int64")?);arrays.insert("seed".into(),scalar(seed as f64,"int64")?);
    arrays.insert("time_map_render_seconds".into(),array(vec![mapping.0.len()],"float64",mapping.0.clone())?);arrays.insert("time_map_authored_seconds".into(),array(vec![mapping.1.len()],"float64",mapping.1.clone())?);
    // Required archive names are never synthesized from renamed diagnostics.
    for key in ["f0_hz","voiced","uv","native_phone_id","segment_index","run_index","lsf_radians","lpc_coefficients","parcor_coefficients"]{ensure!(arrays.contains_key(key),"native stage did not produce required {key}");}
    let pcm=artifacts::quantize_native(&samples)?;
    artifacts::write_pcm16(&folder.join("native.wav"),plan.sample_rate,&pcm)?;artifacts::write_npz(&folder.join("native.npz"),&arrays)?;
    let mut report=plan.metadata;
    report["format"]="greendam-native-diagnostic-v1".into();report["title"]=payload["score"].get("title").cloned().unwrap_or_else(||"native".into());
    report["status"]="actually_rendered_approximate_native_parameter_synthesis".into();report["sample_rate"]=plan.sample_rate.into();report["hop_samples"]=plan.hop_samples.into();report["hop_seconds"]=control.period_seconds.into();report["seed"]=seed.into();report["runs"]=reports.into();
    report["provenance"]=voice.provenance.clone();report["source_identity_confirmed"]=voice.source_identity_confirmed().into();report["voice_selection"]=config.selection.clone().unwrap_or(Value::Null);
    report["implementation"]=json!({"language":"Rust","version":env!("CARGO_PKG_VERSION"),"native_contract_version":env!("CARGO_PKG_VERSION"),"source_sha256":capabilities::source_hashes()});
    report["audio_metrics"]=artifacts::metrics(&samples);report["audio_metrics"]["duration_seconds"]=(samples.len() as f64/plan.sample_rate as f64).into();report["audio_metrics"]["frames"]=n.into();
    report["output_sha256"]=json!({"wav":artifacts::sha(&folder.join("native.wav"))?,"npz":artifacts::sha(&folder.join("native.npz"))?});report["files"]=json!({"wav":"native.wav","parameters":"native.npz"});
    artifacts::write_json(&folder.join("native.json"),&report)?;
    // Original output processing starts from the published PCM codes.
    let audio=AudioResult{sample_rate:plan.sample_rate,samples:pcm.into_iter().map(|v|v as f64/32768.).collect(),source_dtype:"int16".into(),arrays,metadata:report};
    Ok((audio,control,mapping))
}

/// Decode the admitted voice graph from the original statistical trajectories.
fn render_voice(config:&Config,request:&Value,folder:&Path,opts:&Value)->Result<()>{
    let payload=&request["payload"];let package=runtime::admitted_voice_package(config)?;
    let (report_file,wav_file,archive_file,conditions_file,before_music,before_output)=
        ("verdant-voice.json","verdant-voice.wav","verdant-voice.npz","verdant-voice-conditions.npz","verdant-voice-before-music.wav","verdant-voice-before-output.wav");
    let (reference_conditions_file,reference_plan_file,reference_key)=
        ("native-reference-conditions.npz","native-reference-plan.json","native_reference");
    let conditions_stage="voice_conditions";
    let graph_stage="voice_onnx";
    let music_stage="voice_music";
    let seed=payload.get("native_seed").and_then(Value::as_u64).unwrap_or(10008);
    artifacts::write_json(&folder.join("score.json"),&payload["score"])?;
    stage(request,conditions_stage,"stage_started")?;
    let mut plan=parameters::plan_voice_score(&package.voice,&payload["score"],payload.get("control_track").filter(|v|!v.is_null()),opts)?;
    let mapping=maps(&plan)?;let control=controls(&plan)?;
    let directory=runtime::voice_runtime(config)?;
    let conditions=verdant_synthesis::voice::conditions_for_package(&plan,&package)?;
    let (reference_conditions,reference_f0)={
        let mut reference_plan=parameters::plan_native_reference_score(&package.voice,&payload["score"],
            payload.get("control_track").filter(|v|!v.is_null()),opts)?;
        ensure!(reference_plan.duration_frames==plan.duration_frames && maps(&reference_plan)?==mapping,
            "native reference changed the original authored/output time map");
        let reference_conditions=verdant_synthesis::voice::conditions_for_package(&reference_plan,&package)?;
        let reference_f0=verdant_synthesis::voice::native_reference_from_plan(&reference_plan)?;
        let (_,adapter_report)=voice::controller_conditions(&conditions,&reference_f0,Some(&reference_conditions))?;
        let mut archive=reference_conditions.clone();
        archive.insert("native_reference_f0_hz".into(),reference_f0.clone());
        artifacts::write_npz(&folder.join(reference_conditions_file),&archive)?;
        for run in &mut reference_plan.runs {
            run.metadata["gain_policy"]="conditions only; no legacy peak calibration or waveform synthesis executed".into();
        }
        let mut reference_report=reference_plan.metadata;
        reference_report["runs"]=reference_plan.runs.iter().map(|run|run.metadata.clone()).collect::<Vec<_>>().into();
        reference_report["format"]="verdant-voice-native-reference-plan/1".into();
        reference_report["candidate"]=voice::CANDIDATE_ID.into();reference_report["strategy"]=voice::STRATEGY.into();
        reference_report["name"]="verdant-voice".into();reference_report["voice_version"]=package.voice_version.clone().into();
        reference_report["source_package_sha256"]=package.source_manifest_sha256().into();
        reference_report["controller_adapter"]=adapter_report;
        reference_report["conditions_sha256"]=artifacts::sha(&folder.join(reference_conditions_file))?.into();
        reference_report["original_decoder_target_changed"]=false.into();
        reference_report["old_waveform_generated"]=false.into();
        reference_report["legacy_peak_calibration_executed"]=false.into();
        artifacts::write_json(&folder.join(reference_plan_file),&reference_report)?;
        (reference_conditions,reference_f0)
    };
    let semantic=native_voice::features(&conditions,&plan)?;
    let music_configuration=payload.get("voice_music").map(verdant_synthesis::voice::music::Configuration::parse).transpose()?;
    let mut native_arrays=Arrays::new();
    for run in &mut plan.runs{
        run.metadata["gain_policy"]="no legacy peak calibration executed; verdant-voice controller predicts amplitude".into();
        let arrays=std::mem::take(&mut run.arrays);
        merge_arrays(&mut native_arrays,arrays,run.start_frame,run.frames,plan.duration_frames,plan.hop_samples)?;
    }
    fill_gaps(&mut native_arrays,&plan)?;
    let n=plan.duration_frames;
    native_arrays.insert("frame_times_seconds".into(),array(vec![n],"float64",(0..n).map(|i|(i as f64+0.5)*0.005).collect())?);
    native_arrays.insert("times_seconds".into(),native_arrays["frame_times_seconds"].clone());
    native_arrays.insert("frame_period_seconds".into(),array(vec![1],"float64",vec![0.005])?);
    native_arrays.insert("sample_rate".into(),scalar(16000.,"int64")?);native_arrays.insert("hop_samples".into(),scalar(80.,"int64")?);
    native_arrays.insert("time_map_render_seconds".into(),array(vec![mapping.0.len()],"float64",mapping.0.clone())?);
    native_arrays.insert("time_map_authored_seconds".into(),array(vec![mapping.1.len()],"float64",mapping.1.clone())?);
    artifacts::write_npz(&folder.join("native.npz"),&native_arrays)?;
    artifacts::write_npz(&folder.join(conditions_file),&conditions)?;
    let mut native_report=plan.metadata;
    native_report["runs"]=plan.runs.iter().map(|r|r.metadata.clone()).collect::<Vec<_>>().into();
    native_report["format"]="verdant-voice-native-conditions-v1".into();native_report["route"]="verdant-voice".into();
    native_report["candidate"]="verdant-voice".into();native_report["name"]="verdant-voice".into();native_report["voice_version"]=package.voice_version.clone().into();
    native_report["neighbor_encoding"]=verdant_synthesis::voice::neighbor_encoding_from_contracts(&package.contracts)?.into();
    native_report["old_waveform_generated"]=false.into();native_report["statistical_dependency"]="exact retained original decision trees and leaf records; original MLPG/phone clock".into();
    native_report["source_provenance"]=package.voice.provenance.clone();native_report["source_receipt"]=config.selection.as_ref().map(|s|s["source_receipt"].clone()).unwrap_or(Value::Null);
    if let Some(receipt) = config.selection.as_ref().and_then(|selection| selection.get("package_receipt")).filter(|receipt| !receipt.is_null()) {
        native_report["package_receipt"] = receipt.clone();
    }
    native_report["conditions_sha256"]=artifacts::sha(&folder.join(conditions_file))?.into();native_report["native_control_sha256"]=artifacts::sha(&folder.join("native.npz"))?.into();
    {
        native_report[reference_key]=json!({"candidate":voice::CANDIDATE_ID,"strategy":voice::STRATEGY,
            "conditions":reference_conditions_file,
            "conditions_sha256":artifacts::sha(&folder.join(reference_conditions_file))?,
            "plan":reference_plan_file,"plan_sha256":artifacts::sha(&folder.join(reference_plan_file))?,
            "original_decoder_target_conditions_preserved":true});
    }
    artifacts::write_json(&folder.join("native.json"),&native_report)?;
    stage(request,conditions_stage,"stage_completed")?;stage(request,graph_stage,"stage_started")?;
    let audio=voice::render(directory,&package,&conditions,&reference_f0,&reference_conditions,semantic,opts,seed)?;
    validate_audio(&audio)?;
    let(decoder_rate,decoder_hop)=(48000,240);
    ensure!(audio.sample_rate==decoder_rate&&audio.samples.len()==n*decoder_hop,"Verdant ONNX changed the native clock");
    artifacts::write_npz(&folder.join(archive_file),&audio.arrays)?;
    let mut report=audio.metadata;report["voice_selection"]=config.selection.clone().unwrap_or(Value::Null);report["seed"]=seed.into();
    report["conditions_sha256"]=native_report["conditions_sha256"].clone();report["native_control_sha256"]=native_report["native_control_sha256"].clone();
    report["native_control_clock"]=json!({"sample_rate":16000,"hop_samples":80,"frame_period_seconds":0.005,"frames":n});
    report[reference_key]=native_report[reference_key].clone();
    report["raw_float_archive_sha256"]=artifacts::sha(&folder.join(archive_file))?.into();report["raw_float_archive_scope"]="unchanged native 48 kHz decoded verdant-voice before music, authored expression and output processing".into();
    report["processor_source_sha256"]=serde_json::to_value(capabilities::source_hashes())?;
    report["audio_metrics_before_output"]=artifacts::metrics(&audio.samples);
    let final_requested=payload.get("expression").is_some()||payload.get("output_transients").is_some();
    let mut samples=audio.samples;
    let mut output_rate=decoder_rate;
    let mut source_dtype="float32";
    if let Some(configuration)=music_configuration{
        stage(request,music_stage,"stage_started")?;
        artifacts::write_float32(&folder.join(before_music),decoder_rate,&samples)?;
        report["decoded_audio_metrics"]=report["audio_metrics_before_output"].clone();
        let(rendered,music)=verdant_synthesis::voice::music::render(samples,&conditions["active"].values,configuration)?;
        samples=rendered;output_rate=configuration.sample_rate;source_dtype="float64";
        report["voice_music"]=music;
        report["normalization"]="no upward normalization; explicit attenuation-only music headroom".into();
        report["decoder_sample_rate"]=decoder_rate.into();report["decoder_hop_samples"]=decoder_hop.into();
        report["sample_rate"]=output_rate.into();report["output_sample_rate"]=output_rate.into();
        // 44.1 kHz has 220.5 samples per 5 ms control cell. Do not publish an
        // integer audio hop that would misdescribe either the archive or WAV.
        report.as_object_mut().unwrap().remove("hop_samples");
        report["frame_period_seconds"]=0.005.into();
        report["audio_metrics_before_output"]=artifacts::metrics(&samples);
        stage(request,music_stage,"stage_completed")?;
    }
    let output_samples=samples.len();
    if final_requested{
        artifacts::write_float32(&folder.join(before_output),output_rate,&samples)?;
        let finalized=output::finalize(AudioResult{sample_rate:output_rate,samples,source_dtype:source_dtype.into(),arrays:Arrays::new(),metadata:json!({})},&control,payload.get("expression"),payload.get("output_transients"),Some((&mapping.0,&mapping.1)),seed)?;
        validate_audio(&finalized)?;ensure!(finalized.sample_rate==output_rate&&finalized.samples.len()==output_samples,"Verdant ONNX output processors changed its clock");
        artifacts::write_npz(&folder.join("output_pipeline.npz"),&finalized.arrays)?;
        report["output_pipeline"]=finalized.metadata;report["output_pipeline_trace_sha256"]=artifacts::sha(&folder.join("output_pipeline.npz"))?.into();
        report["output_pipeline"]["output_pipeline"]["quantization"]="floating-point output at the reported source/output precision; final WAV is IEEE float32, without PCM16 conversion".into();
        samples=finalized.samples;
    }
    if music_configuration.is_some(){
        report["voice_music"]["final_authored_rests"]=verdant_synthesis::voice::music::enforce_rests(&mut samples,output_rate,&conditions["active"].values)?;
    }else{
        ensure!(samples.iter().enumerate().all(|(i,v)|conditions["active"].values[i/decoder_hop]!=0.||*v==0.),"Verdant ONNX final output is not silent in an authored rest");
    }
    // Float WAV preserves model energy without hidden clipping or per-clip
    // gain. Full-scale excursions are measured and remain explicit metadata.
    artifacts::write_float32(&folder.join(wav_file),output_rate,&samples)?;
    report["audio_metrics"]=artifacts::metrics(&samples);report["waveform_sha256"]=artifacts::sha(&folder.join(wav_file))?.into();
    report["files"]=json!({"wav":wav_file,"raw_float_archive":archive_file,"conditions":conditions_file,"native_control":"native.npz"});
    report["files"]["native_reference_conditions"]=reference_conditions_file.into();report["files"]["native_reference_plan"]=reference_plan_file.into();
    if music_configuration.is_some(){report["files"]["before_music"]=before_music.into();}
    if final_requested{report["files"]["before_output"]=before_output.into();}
    artifacts::write_json(&folder.join(report_file),&report)?;stage(request,graph_stage,"stage_completed")?;Ok(())
}
fn finalize_original(folder: &Path, request: &Value, audio: AudioResult, control: &AcousticControlTrack, mapping: &(Vec<f64>, Vec<f64>)) -> Result<()> {
    let payload = &request["payload"];
    let request_path = folder.join("request.json");
    let request_hash = artifacts::sha(&request_path)?;
    let output_path = folder.join("native.wav");
    let before = artifacts::sha(&output_path)?;
    let control_path = folder.join("native.npz");
    let control_hash = artifacts::sha(&control_path)?;
    let (rate, pcm) = artifacts::pcm16_codes(&fs::read(&output_path)?)?;
    ensure!(rate == audio.sample_rate && pcm.len() == audio.samples.len(), "original PCM changed waveform dimensions");
    let duration = control.f0_hz.len() as f64 * control.period_seconds;
    ensure!((pcm.len() as f64 / rate as f64 - duration).abs() <= 1. / rate as f64 + 1e-9,
        "original output and native control durations disagree");
    let seed = payload.get("native_seed").and_then(Value::as_u64).unwrap_or(10008);
    let source = audio.samples.clone();
    let mut audio = audio;
    audio.arrays.clear();
    audio.source_dtype = "float64".into();
    let finalized = output::finalize(audio, control, payload.get("expression"), payload.get("output_transients"), Some((&mapping.0, &mapping.1)), seed)?;
    validate_audio(&finalized)?;
    ensure!(finalized.sample_rate == rate && finalized.samples.len() == pcm.len(), "original output processing changed the waveform clock");
    let processors = &finalized.metadata["output_pipeline"]["processors"];
    let expression_enabled = processors["voice_expression"]["enabled"].as_bool().unwrap_or(false);
    let output_pcm = output::quantize_preserving_pcm16(&pcm, &source, &finalized.samples, expression_enabled)?;
    let trace = folder.join("output_pipeline.npz");
    let traces: Arrays = finalized.arrays.into_iter().filter(|(key, _)|
        matches!(key.as_str(), "waveform_final" | "sample_rate" | "changed_samples") || key.starts_with("expression_") || key.starts_with("transients_")).collect();
    for key in ["waveform_final", "sample_rate", "changed_samples"] {
        ensure!(traces.contains_key(key), "final output missing trace {key}");
    }
    ensure!(artifacts::sha(&request_path)? == request_hash && artifacts::sha(&output_path)? == before
        && artifacts::sha(&control_path)? == control_hash, "final-output inputs changed during processing");
    fs::copy(&output_path, folder.join("pre_output_pipeline.wav"))?;
    artifacts::write_npz(&trace, &traces)?;
    if output_pcm != pcm {
        let temporary = folder.join("output_pipeline.tmp.wav");
        artifacts::write_pcm16(&temporary, rate, &output_pcm)?;
        fs::rename(&temporary, &output_path)?;
    }
    let after = artifacts::sha(&output_path)?;
    let metadata_path = folder.join("native.json");
    let upstream_path = folder.join("native.upstream.json");
    let mut metadata: Value = serde_json::from_slice(&fs::read(&metadata_path)?)?;
    fs::copy(&metadata_path, &upstream_path)?;
    metadata["pre_output_pipeline_audio_metrics"] = metadata["audio_metrics"].clone();
    let metrics = artifacts::metrics(&finalized.samples);
    metadata["audio_metrics"].as_object_mut().context("native audio metrics")?.extend(metrics.as_object().unwrap().clone());
    metadata["audio_metrics"]["scope"] = "after_voice_expression_and_output_guard".into();
    metadata["output_sha256"]["wav"] = after.clone().into();
    metadata["output_pipeline"] = json!({"report":"output_pipeline.json","upstream_metadata":"native.upstream.json",
        "upstream_metadata_sha256":artifacts::sha(&upstream_path)?,"pre_output_wav":"pre_output_pipeline.wav","pre_output_wav_sha256":before,
        "measured_f0_scope":"upstream_inference_before_expression_and_output_guard"});
    artifacts::write_json(&metadata_path, &metadata)?;
    let original_metrics = artifacts::metrics(&source);
    let report = json!({"format":"verdant-output-pipeline-v1","route":"C0","request_sha256":request_hash,
        "stage_order":["voice_expression","expression_headroom","output_transients"],"final_wav":"native.wav","final_wav_sha256":after,
        "pre_pipeline_wav":"pre_output_pipeline.wav","pre_pipeline_wav_sha256":before,"trace":"output_pipeline.npz","trace_sha256":artifacts::sha(&trace)?,
        "native_control_sha256":control_hash,"sample_rate":rate,"samples":source.len(),"seed":seed,
        "input_float_archive":null,"input_float_archive_sha256":null,"float_input_scope":"decoded final Native PCM16",
        "changed_float_samples":source.iter().zip(&finalized.samples).filter(|(a,b)|a!=b).count(),
        "changed_pcm_samples":pcm.iter().zip(&output_pcm).filter(|(a,b)|a!=b).count(),
        "unchanged_float_pcm_preserved":source.iter().zip(&finalized.samples).zip(pcm.iter().zip(&output_pcm)).all(|((a,b),(p,q))|a!=b||p==q),
        "peak_before":original_metrics["peak"],"peak_after":metrics["peak"],"rms_before":original_metrics["rms"],"rms_after":metrics["rms"],
        "samples_outside_pcm_full_scale":metrics["clipped_samples"],
        "changed_samples_outside_pcm_full_scale":source.iter().zip(&finalized.samples).filter(|(a,b)|a!=b&&b.abs()>=1.).count(),
        "quantization":"PCM16 nearest integer at changed samples only; original codes elsewhere","normalization":"none","upstream_archives_modified":false,
        "upstream_pitch_measurement_scope":"before final voice expression and transient control; not a final-output measurement",
        "updated_metadata":[{"path":"native.json","sha256":artifacts::sha(&metadata_path)?,"upstream_path":"native.upstream.json","upstream_sha256":artifacts::sha(&upstream_path)?}],
        "processor_source_sha256":capabilities::source_hashes(),"processors":processors});
    artifacts::write_json(&folder.join("output_pipeline.json"), &report)
}

pub fn execute(config: &Config, request: &Value, folder: &Path) -> Result<()> {
    ensure!(folder.canonicalize()?.starts_with(&config.workspace), "job directory escapes workspace");
    crate::admission::request(request)?;
    ensure!(artifacts::sha(&folder.join("request.json"))? == config.request_sha256, "request changed after admission");
    let current = config.selected(request)?;
    ensure!(current.selection == config.selection && current.original == config.original && current.voice_package == config.voice_package,
        "voice registry selection changed after admission");
    let assets = runtime::asset_hashes(config)?;
    ensure!(assets == config.asset_sha256, "runtime assets changed after admission");
    let payload = &request["payload"];
    let route = request["route"].as_str().unwrap();
    let mut opts = options(payload);
    opts["route"] = route.into();
    if let Some(control) = payload.get("control_track").filter(|v| !v.is_null()) {
        artifacts::write_json(&folder.join("provided_control.json"), control)?;
    }
    let result = if route == "verdant-voice" {
        opts["onnxruntime_library"] = runtime::onnxruntime_path(config)?.to_string_lossy().into_owned().into();
        render_voice(config, request, folder, &opts)?;
        let mut result = json!({"status":"completed","route":route,"fallback_used":false,"voice_selection":config.selection});
        if let Some(package) = config.selection.as_ref().and_then(|selection| selection.get("voice_package")) {
            result["candidate"] = "verdant-voice".into();
            result["name"] = package["name"].clone();
            result["voice_version"] = package["version"].clone();
        }
        result
    } else {
        artifacts::write_json(&folder.join("score.json"), &payload["score"])?;
        stage(request, "native", "stage_started")?;
        let (audio, control, mapping) = native(config, request, folder, &opts)?;
        stage(request, "native", "stage_completed")?;
        if payload.get("expression").is_some() || payload.get("output_transients").is_some() {
            stage(request, "output", "stage_started")?;
            finalize_original(folder, request, audio, &control, &mapping)?;
            stage(request, "output", "stage_completed")?;
        }
        json!({"status":"completed","route":route,"voice_selection":config.selection,
            "identity_status":"voice identity is bound to recorded model/registry provenance; engine/frontend fidelity unconfirmed"})
    };
    ensure!(runtime::asset_hashes(config)? == assets, "runtime assets changed during synthesis");
    ensure!(artifacts::sha(&folder.join("request.json"))? == config.request_sha256, "request changed during synthesis");
    artifacts::write_json(&folder.join("execution-result.json"), &result)
}
