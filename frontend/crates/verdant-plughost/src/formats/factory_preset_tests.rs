//! Every shipped recipe is admitted by the real processor and renders PCM.
use std::collections::{BTreeMap, BTreeSet};
use sha2::{Digest, Sha256};
use serde_json::json;
use verdant_plugins::{factory_presets as factory, builtin_dsp as dsp};

#[test]
fn factory_library_is_complete_distinct_and_audible() {
    let presets=factory::all_factory_presets();
    assert_eq!(presets.len(),280);
    let mut count=BTreeMap::new();let mut unique=BTreeSet::new();let mut report=Vec::new();let mut failures=Vec::new();
    for preset in presets {
        *count.entry(preset.device.clone()).or_insert(0usize)+=1;
        let state=serde_json::to_vec(&preset.parameters).unwrap();
        assert!(unique.insert((preset.device.clone(),state.clone())),"duplicate sound {}",preset.id);
        let descs=factory::parameter_descriptors(&preset.device).unwrap();
        assert_eq!(descs.len(),preset.parameters.len());
        let instrument=dsp::is_builtin_instrument(&preset.device);
        let mut effect=(!instrument).then(||super::builtin::effect(&preset.device,48000.).unwrap());
        let mut synth=if instrument {
            let values=serde_json::to_value(&preset.parameters).unwrap();
            let params=dsp::SynthParams::from_json_for(&preset.device,Some(&values));
            let mut synth=dsp::RealtimeSynth::for_instrument(&preset.device,&params,48000.);
            synth.note_on(0,60,60.,0.7);
            Some(synth)
        }else{None};
        let latency=if let Some(fx)=&mut effect{
            let actual=fx.params().unwrap();
            assert_eq!(actual.len(),descs.len(),"{} descriptor count",preset.device);
            for (a,b) in actual.iter().zip(&descs) {
                assert_eq!((&a.id,a.min,a.max,a.step_count),(&b.id,b.min,b.max,b.step_count),"{} descriptor grid",preset.device);
            }
            fx.set_state(&state).unwrap_or_else(|e|panic!("{}: {e}",preset.id));
            let restored:serde_json::Value=serde_json::from_slice(&fx.state().unwrap()).unwrap();
            for(k,v)in &preset.parameters{assert_eq!(restored[k].as_f64(),Some(*v),"{} restore {k}",preset.id);}
            fx.set_offline(true).unwrap();fx.latency()
        }else{0};
        let mut peak=0f64;let mut energy=0f64;let mut hash=Sha256::new();let mut rng=42u32;
        let mut il=[0f32;256];let mut ir=[0f32;256];let mut ol=[0f32;256];let mut orr=[0f32;256];
        for block in 0..375 {
            for i in 0..256 {
                let frame=block*256+i;let t=frame as f64/48000.;
                rng^=rng<<13;rng^=rng>>17;rng^=rng<<5;
                let noise=(rng as f64/u32::MAX as f64*2.-1.)*0.012;
                let env=if t<1.0 {0.55+0.45*(-(t%0.25)*18.).exp()}else{0.};
                il[i]=((0.10*(std::f64::consts::TAU*110.*t).sin()+0.04*(std::f64::consts::TAU*1700.*t).sin()+noise)*env)as f32;
                ir[i]=((0.09*(std::f64::consts::TAU*113.*t).sin()+0.035*(std::f64::consts::TAU*6400.*t).sin()-noise)*env)as f32;
            }
            if let Some(fx)=&mut effect{fx.process(&il,&ir,&mut ol,&mut orr).unwrap();}
            if let Some(s)=&mut synth{
                if block==188{s.note_off(0,60);}
                s.process(&mut ol,&mut orr);
            }
            for i in 0..256{for x in [ol[i],orr[i]]{
                assert!(x.is_finite(),"{} produced nonfinite PCM",preset.id);
                let x64=f64::from(x);peak=peak.max(x64.abs());energy+=x64*x64;hash.update(x.to_le_bytes());
            }}
        }
        let rms=(energy/(96000.*2.)).sqrt();
        if rms<=1e-7{failures.push(format!("{} was silent",preset.id));}
        if peak>=1.0{failures.push(format!("{} exceeded calibration headroom: {peak}",preset.id));}
        report.push(json!({"id":preset.id,"device":preset.device,"parameter_count":preset.parameters.len(),
            "sample_rate":48000,"frames":96000,"channels":2,"peak":peak,"rms":rms,
            "latency_frames":latency,"nonfinite_samples":0,"pcm_sha256":hex::encode(hash.finalize())}));
    }
    assert_eq!(count.len(),14);
    assert!(count.values().all(|n|*n==20));
    if let Ok(path)=std::env::var("VERDANT_FACTORY_PRESET_REPORT"){
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"schema":"verdant-factory-preset-validation/1",
            "library_version":factory::LIBRARY_VERSION,"calibration":"1 s stereo multitone/noise then 1 s tail; synth C4 velocity 0.7",
            "presets":report,"failures":failures})).unwrap()).unwrap();
    }
    assert!(failures.is_empty(),"{}",failures.join("\n"));
}
