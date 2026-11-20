//! Current verdant-voice conditions and native 48 kHz graph execution.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use crate::{Arrays, NumericArray, graph::{GraphTensor, TensorData}};
use crate::parameters::NATIVE_F0_RANGE;
mod conditions;
pub mod music;
pub mod native_voice;
mod renderer;
pub use renderer::{CANDIDATE_ID, STRATEGY, validate_package, controller_conditions, render};
pub(crate) use renderer::manifest;
pub use conditions::{conditions_from_plan, conditions_for_package, native_reference_from_plan,
    neighbor_encoding_from_contracts, RESOLVED_NEIGHBOR_ENCODING};
const CORE: usize = 64;
const CONTROLS: [(&str, usize); 3] = [("harmonic_logits",129),("noise_log_gains",65),("log_amplitude",1)];

fn require_condition_neighbor_encoding(arrays: &Arrays, expected: &str) -> Result<()> {
    let marker = arrays.get("neighbor_encoding_version").context("missing resolved neighbor encoding marker")?;
    ensure!(expected == RESOLVED_NEIGHBOR_ENCODING && marker.shape == [1] && marker.dtype == "int64"
        && marker.values == [2.], "verdant-voice requires resolved-context-phone-v2 conditions");
    Ok(())
}

pub fn validate_conditions(arrays:&Arrays)->Result<usize>{
    require_condition_neighbor_encoding(arrays, RESOLVED_NEIGHBOR_ENCODING)?;
    let n=arrays.get("states").context("missing Verdant ONNX states")?.values.len();ensure!((1..=24000).contains(&n),"Verdant ONNX supports 1..24000 frames");
    for(key,width,dtype)in[("latent25",25,"float32"),("phone_ids",3,"int64"),("states",1,"int64"),("languages",1,"int64"),("prosody",8,"float32"),("f0_hz",1,"float32"),("voiced",1,"bool"),("active",1,"bool")]{
        let a=arrays.get(key).with_context(||format!("missing Verdant ONNX condition {key}"))?;let shape=if width==1{vec![n]}else{vec![n,width]};
        ensure!(a.shape==shape&&a.values.len()==n*width&&a.dtype==dtype&&a.values.iter().all(|x|x.is_finite()),"Verdant ONNX {key} shape/type/finite contract failed");
        if dtype=="float32"{ensure!(a.values.iter().all(|&x|(x as f32).is_finite()),"Verdant ONNX {key} exceeds float32");}
        if dtype=="int64"{ensure!(a.values.iter().all(|x|x.fract()==0.),"Verdant ONNX {key} must contain integers");}
    }
    ensure!(arrays["phone_ids"].values.iter().all(|v|(0. ..=255.).contains(v)),"Verdant ONNX phone IDs exceed native ABI");
    for i in 0..n{
        let state=arrays["states"].values[i];let active=arrays["active"].values[i];let voiced=arrays["voiced"].values[i];let f0=arrays["f0_hz"].values[i];let p=&arrays["prosody"].values[i*8..(i+1)*8];
        ensure!((-1. ..=4.).contains(&state)&&(0. ..=3.).contains(&arrays["languages"].values[i])&&[0.,1.].contains(&active)&&[0.,1.].contains(&voiced),"Verdant ONNX state/language/masks outside ABI");
        ensure!((active==1.)==(state>=0.)&&(voiced==0.||active==1.)&&if voiced==1.{(NATIVE_F0_RANGE[0]..=NATIVE_F0_RANGE[1]).contains(&f0)}else{f0==0.},"Verdant ONNX F0/UV/active conflict");
        ensure!(p[1]==voiced&&((active==0.&&p.iter().all(|v|*v==0.))||(active==1.&&(0. ..=1.).contains(&p[2])&&(0. ..=1.).contains(&p[3]))),"Verdant ONNX prosody masks/progress conflict");
        if active==1.{ensure!(arrays["phone_ids"].values[i*3+1]>0.,"Verdant ONNX active current phone is unresolved");}
    }Ok(n)
}
/// Target pitch and controller reference use independent, explicit ranges.
pub fn authored_pitch_report(conditions: &Arrays) -> Value {
    let mut minimum: Option<f64> = None;
    let mut maximum: Option<f64> = None;
    for (&hz, &voiced) in conditions["f0_hz"].values.iter().zip(&conditions["voiced"].values) {
        if voiced == 1. {
            minimum = Some(minimum.map_or(hz, |value| value.min(hz)));
            maximum = Some(maximum.map_or(hz, |value| value.max(hz)));
        }
    }
    json!({"policy":"native-target-reference-v1","host_f0_hz_range":NATIVE_F0_RANGE,
        "controller_reference_f0_hz_range":[45.,800.],
        "minimum_rendered_f0_hz":minimum,"maximum_rendered_f0_hz":maximum,
        "learned_weights_changed":false,"pitch_clamped":false,"high_register_quality_admitted":false})
}
fn slice(array:&NumericArray,start:usize,end:usize)->Result<GraphTensor>{
    ensure!(!array.shape.is_empty()&&start<end&&end<=array.shape[0],"Verdant ONNX tensor slice outside clock");let row=array.shape[1..].iter().product::<usize>();let v=&array.values[start*row..end*row];
    let mut shape=vec![1,(end-start)as i64];shape.extend(array.shape[1..].iter().map(|v|*v as i64));
    let data=match array.dtype.as_str(){"float32"=>TensorData::F32(v.iter().map(|v|*v as f32).collect()),"int64"=>TensorData::I64(v.iter().map(|v|*v as i64).collect()),"bool"=>TensorData::Bool(v.iter().map(|v|u8::from(*v!=0.)).collect()),_=>anyhow::bail!("unsupported Verdant ONNX condition dtype")};GraphTensor::new(shape,data)
}
fn splitmix(mut value:u64)->u64{value=value.wrapping_add(0x9e3779b97f4a7c15);value=(value^(value>>30)).wrapping_mul(0xbf58476d1ce4e5b9);value=(value^(value>>27)).wrapping_mul(0x94d049bb133111eb);value^(value>>31)}
fn white_noise(seed:u64,samples:usize,offset:usize)->Vec<f32>{
    (offset..offset+samples).map(|n|{let index=(n as u64).wrapping_mul(2);let a=splitmix(seed.wrapping_add(index));let b=splitmix(seed.wrapping_add(index).wrapping_add(1));
        let first=((a>>11)as f64+0.5)*2_f64.powi(-53);let second=((b>>11)as f64+0.5)*2_f64.powi(-53);((-2.*first.ln()).sqrt()*(std::f64::consts::TAU*second).cos())as f32}).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_marker_requires_the_current_encoding() {
        let mut arrays = Arrays::new();
        assert!(require_condition_neighbor_encoding(&arrays, RESOLVED_NEIGHBOR_ENCODING).is_err());
        arrays.insert("neighbor_encoding_version".into(), NumericArray::new(vec![1],"int64",vec![2.]).unwrap());
        require_condition_neighbor_encoding(&arrays, RESOLVED_NEIGHBOR_ENCODING).unwrap();
        for code in [0.,1.,3.] {
            arrays.get_mut("neighbor_encoding_version").unwrap().values[0] = code;
            assert!(require_condition_neighbor_encoding(&arrays, RESOLVED_NEIGHBOR_ENCODING).is_err());
        }
    }
}
