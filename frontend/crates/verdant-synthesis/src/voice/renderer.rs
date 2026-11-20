//! Separately planned native acoustic/source-pitch reference and narrow
//! controller category sharing for English /ɑ/. The decoder always receives
//! the original target conditions, semantic features and authored clock.
use std::{fs::File, io::Read, path::Path};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{Arrays, AudioResult, NumericArray, model::VoicePackage};
use super::{RESOLVED_NEIGHBOR_ENCODING, require_condition_neighbor_encoding, validate_conditions};

pub const CANDIDATE_ID: &str = "verdant-voice";
pub const STRATEGY: &str = "native-mlpg-reference-en-aa-boundary-sustain-v2";
const ACOUSTIC_REFERENCE_FIELDS:[&str;3]=["latent25","states","prosody"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoiceManifest {
    schema: String, name: String, version: String, strategy: String,
    source_manifest_sha256: String,
}

fn validate_voice_declaration(value: &VoiceManifest, source_hash: &str) -> Result<()> {
    ensure!(value.schema == "verdant-voice-runtime/1" && value.name == CANDIDATE_ID
        && value.strategy == STRATEGY && semver::Version::parse(&value.version).is_ok(), "unsupported verdant-voice runtime declaration");
    ensure!(value.source_manifest_sha256 == source_hash, "voice runtime is bound to a different source manifest");
    Ok(())
}

pub(crate) fn manifest(directory: &Path, source_hash: &str) -> Result<(String, String)> {
    let path = directory.join("manifest.json");
    let file = File::open(&path).with_context(|| format!("opening voice declaration {}", path.display()))?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file() && metadata.len() <= 16_384, "voice runtime manifest exceeds its bounded schema");
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(16_385).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 16_384, "voice runtime manifest exceeds its bounded schema");
    let value: VoiceManifest = serde_json::from_slice(&bytes).context("invalid voice runtime manifest")?;
    validate_voice_declaration(&value, source_hash)?;
    Ok((hex::encode(Sha256::digest(&bytes)), value.version))
}

pub fn validate_package(directory: &Path, source: &VoicePackage, options: &Value) -> Result<()> {
    super::native_voice::validate_package(directory, source, options)
}

fn same_array(left:&NumericArray,right:&NumericArray)->bool {
    left.shape==right.shape && left.dtype==right.dtype && left.values==right.values
}

/// All changes apply to an owned clone; the decoder receives `target` itself.
/// Reference fields outside the declared acoustic subset must match exactly.
pub fn controller_conditions(target:&Arrays,reference:&NumericArray,
    acoustic_reference:Option<&Arrays>)->Result<(Arrays,Value)> {
    require_condition_neighbor_encoding(target, RESOLVED_NEIGHBOR_ENCODING)?;
    let frames = validate_conditions(target)?;
    let source=acoustic_reference.unwrap_or(target);
    if acoustic_reference.is_some() {
        require_condition_neighbor_encoding(source,RESOLVED_NEIGHBOR_ENCODING)?;
        ensure!(validate_conditions(source)?==frames,"native acoustic reference changed the original target frame clock");
        ensure!(source.len()==target.len() && target.keys().all(|key|source.contains_key(key)),
            "native acoustic reference field identities differ from original target");
        for (key,original) in target {
            if !ACOUSTIC_REFERENCE_FIELDS.contains(&key.as_str()) {
                ensure!(same_array(&source[key],original),
                    "native acoustic reference changed original target clock/identity/mask: {key}");
            }
        }
        for frame in 0..frames {
            for column in [0,1,6,7] {
                ensure!(source["prosody"].values[frame*8+column]==target["prosody"].values[frame*8+column],
                    "native acoustic reference may change only native duration/progress prosody columns 2..5");
            }
        }
    }
    ensure!(reference.shape == [frames] && reference.dtype == "float32"
        && reference.values.len() == frames && reference.values.iter().all(|value| value.is_finite()),
        "native reference must be finite float32 on the target frame clock");
    let changed_fields=ACOUSTIC_REFERENCE_FIELDS.iter().copied()
        .filter(|key|!same_array(&source[*key],&target[*key])).collect::<Vec<_>>();
    let mut controller = source.clone();
    let mut shared = 0;
    let mut shared_neighbors=0;
    for frame in 0..frames {
        let voiced = target["voiced"].values[frame] == 1.;
        let source = reference.values[frame];
        ensure!(if voiced { (45. ..=800.).contains(&source) } else { source == 0. },
            "native reference must be 45..800 Hz when voiced and exact zero on UV/rest; no clipping");
        ensure!(source == source as f32 as f64, "native reference values do not retain float32 rounding");
        let expected = if voiced { ((target["f0_hz"].values[frame] as f32 / 300.0_f32).ln()) as f64 } else { 0. };
        ensure!((target["prosody"].values[frame * 8] - expected).abs() <= 2e-6,
            "native voice target F0 and prosody encoding disagree");
        controller.get_mut("f0_hz").expect("validated F0").values[frame] = source;
        controller.get_mut("prosody").expect("validated prosody").values[frame * 8] =
            if voiced { ((source as f32 / 300.0_f32).ln()) as f64 } else { 0. };
        if target["active"].values[frame] == 1. && target["phone_ids"].values[frame * 3 + 1] == 174.
            && target["languages"].values[frame] == 2. {
            controller.get_mut("languages").expect("validated languages").values[frame] = 1.;
            shared += 1;
            for column in [0,2] {
                if target["phone_ids"].values[frame*3+column]==2. {
                    controller.get_mut("phone_ids").expect("validated phone IDs").values[frame*3+column]=0.;
                    shared_neighbors+=1;
                }
            }
        }
    }
    validate_conditions(&controller)?;
    Ok((controller,json!({"candidate":CANDIDATE_ID,"shared_language_frames":shared,
        "shared_pause_neighbor_slots":shared_neighbors,"acoustic_reference_supplied":acoustic_reference.is_some(),
        "acoustic_reference_changed_fields":changed_fields,"target_conditions_changed":false,
        "current_native_phone_ids_changed":false,"training_updates":0,"quality_admission":false})))
}

pub fn render(directory: &Path, source: &VoicePackage, target: &Arrays,
    native_reference:&NumericArray,acoustic_reference:&Arrays,
    semantic: Vec<f32>, options: &Value, seed: u64) -> Result<AudioResult> {
    let (controller,reference_report) = controller_conditions(target,native_reference,Some(acoustic_reference))?;
    let mut audio = super::native_voice::render_with_controller(directory, source, target, &controller, semantic, options, seed)?;
    audio.metadata["candidate"] = CANDIDATE_ID.into();
    audio.metadata["name"] = CANDIDATE_ID.into();
    audio.metadata["voice_version"] = source.voice_version.clone().into();
    if let Some(parent) = audio.metadata.as_object_mut().and_then(|metadata| metadata.remove("release_source_commit")) {
        audio.metadata["parent_release_source_commit"] = parent;
    }
    audio.metadata["candidate_id"] = CANDIDATE_ID.into();
    audio.metadata["kind"] = "verdant-voice-native-boundary-sustain-reference-onnx".into();
    audio.metadata["strategy"] = STRATEGY.into();
    audio.metadata["strategy_sha256"] = source.runtime_manifest_sha256.clone().into();
    audio.metadata["source_package_sha256"] = source.source_manifest_sha256().into();
    audio.metadata["controller_reference"] = json!({"source":"separate-native-reference-plan-unclipped-mlpg",
        "policy":crate::parameters::TERMINAL_NUCLEUS_REFERENCE_POLICY,"report":reference_report,
        "f0_hz_range":[45.,800.],"clipping":false,"target_f0_preserved":true,
        "decoder_target_conditions_preserved":true});
    audio.metadata["controller_language_alias"] = json!({"phone_id":174,"source_language":2,
        "controller_language":1,"applied_frames":reference_report["shared_language_frames"],"decoder_phone_and_language_preserved":true});
    audio.metadata["controller_pause_neighbor_alias"]=json!({"phone_id":174,"source_language":2,
        "source_neighbor":2,"controller_neighbor":0,"columns":[0,2],
        "applied_slots":reference_report["shared_pause_neighbor_slots"],"decoder_neighbors_preserved":true});
    audio.metadata["human_listening_accepted"] = false.into();
    for key in ["f0_hz", "languages","phone_ids","latent25","states","prosody"] {
        audio.arrays.insert(format!("controller_{key}"), controller[key].clone());
    }
    audio.arrays.insert("native_reference_f0_hz".into(),native_reference.clone());
    Ok(audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> Arrays {
        let frames = 6;
        let f0 = vec![220., 110., 440., 330., 0., 0.];
        let voiced = vec![1., 1., 1., 1., 0., 0.];
        let mut prosody = vec![0.; frames * 8];
        for frame in 0..frames {
            prosody[frame * 8] = if voiced[frame] == 1. { ((f0[frame] as f32 / 300.0_f32).ln()) as f64 } else { 0. };
            prosody[frame * 8 + 1] = voiced[frame];
        }
        let mut arrays = Arrays::new();
        for (key, shape, dtype, values) in [
            ("latent25", vec![frames, 25], "float32", vec![0.; frames * 25]),
            ("phone_ids", vec![frames, 3], "int64", vec![2.,174.,2.,174.,174.,3.,2.,180.,174.,2.,174.,2.,2.,174.,2.,2.,174.,2.]),
            ("states", vec![frames], "int64", vec![0.,1.,2.,3.,4.,-1.]),
            ("languages", vec![frames], "int64", vec![2.,2.,2.,3.,2.,2.]),
            ("active", vec![frames], "bool", vec![1.,1.,1.,1.,1.,0.]),
            ("voiced", vec![frames], "bool", voiced),
            ("f0_hz", vec![frames], "float32", f0),
            ("prosody", vec![frames,8], "float32", prosody),
            ("frame_segment_index", vec![frames], "int64", vec![0.,0.,1.,1.,2.,-1.]),
            ("neighbor_encoding_version", vec![1], "int64", vec![2.]),
        ] { arrays.insert(key.into(), NumericArray::new(shape, dtype, values).unwrap()); }
        arrays
    }
    fn reference()->NumericArray {
        NumericArray::new(vec![6],"float32",vec![333.,287.,290.,310.,0.,0.]).unwrap()
    }

    #[test]
    fn shares_only_active_english_aa_categories_and_preserves_original_target() -> Result<()> {
        let target = target();
        let before = serde_json::to_value(&target)?;
        let reference=reference();
        let (controller, report) = controller_conditions(&target,&reference,None)?;
        assert_eq!(report["shared_language_frames"],3);
        assert_eq!(report["shared_pause_neighbor_slots"],4);
        assert_eq!(controller["languages"].values, [1.,1.,2.,3.,1.,2.]);
        assert_eq!(controller["phone_ids"].values,
            [0.,174.,0.,174.,174.,3.,2.,180.,174.,2.,174.,2.,0.,174.,0.,2.,174.,2.]);
        assert_eq!(controller["f0_hz"].values, reference.values);
        for key in ["latent25","states","active","voiced","frame_segment_index"] {
            assert!(same_array(&controller[key],&target[key]),"{key}");
        }
        for frame in 0..6 { assert_eq!(controller["prosody"].values[frame*8+1..frame*8+8],target["prosody"].values[frame*8+1..frame*8+8]); }
        assert_eq!(serde_json::to_value(&target)?, before);
        Ok(())
    }

    #[test]
    fn native_acoustic_reference_admits_only_declared_trajectory_fields() -> Result<()> {
        let target=target();let reference=reference();
        let before=serde_json::to_value(&target)?;
        let mut source=target.clone();
        source.get_mut("latent25").unwrap().values[24]=6.25;
        source.get_mut("states").unwrap().values[0]=1.;
        for column in 2..6 {source.get_mut("prosody").unwrap().values[column]=0.5;}
        let (controller,report)=controller_conditions(&target,&reference,Some(&source))?;
        assert_eq!(report["acoustic_reference_changed_fields"],json!(["latent25","states","prosody"]));
        assert!(same_array(&controller["latent25"],&source["latent25"]));
        assert!(same_array(&controller["states"],&source["states"]));
        assert_eq!(controller["prosody"].values[2..6],source["prosody"].values[2..6]);
        for key in ["phone_ids","languages","frame_segment_index","f0_hz"] {
            let mut bad=source.clone();bad.get_mut(key).unwrap().values[0]+=1.;
            assert!(controller_conditions(&target,&reference,Some(&bad)).is_err(),"{key}");
        }
        for column in [0,1,6,7] {
            let mut bad=source.clone();bad.get_mut("prosody").unwrap().values[column]+=0.5;
            assert!(controller_conditions(&target,&reference,Some(&bad)).is_err(),"prosody {column}");
        }
        let mut bad=source.clone();bad.remove("frame_segment_index");
        assert!(controller_conditions(&target,&reference,Some(&bad)).is_err());
        let mut bad=source;bad.insert("extra".into(),reference.clone());
        assert!(controller_conditions(&target,&reference,Some(&bad)).is_err());
        assert_eq!(serde_json::to_value(&target)?,before);
        Ok(())
    }

    #[test]
    fn reference_rejects_wrong_clock_bounds_uv_and_pitch_contradictions() {
        for value in [0., 44., 801., f64::NAN,333.1] {
            let mut bad=reference();bad.values[0]=value;
            assert!(controller_conditions(&target(),&bad,None).is_err());
        }
        for frame in [4,5] {
            let mut bad=reference();bad.values[frame]=220.;
            assert!(controller_conditions(&target(),&bad,None).is_err());
        }
        let mut bad=reference();bad.shape=vec![2,3];assert!(controller_conditions(&target(),&bad,None).is_err());
        let mut bad=reference();bad.dtype="float64".into();assert!(controller_conditions(&target(),&bad,None).is_err());
        let mut bad=target();bad.get_mut("prosody").unwrap().values[0]+=1.;assert!(controller_conditions(&bad,&reference(),None).is_err());
        for value in [45.,800.] {
            let mut valid=reference();valid.values[0]=value;
            assert_eq!(controller_conditions(&target(),&valid,None).unwrap().0["f0_hz"].values[0],value);
        }
    }

    #[test]
    fn canonical_declaration_is_exact_and_retains_compatible_persisted_versions() -> Result<()> {
        let declaration=json!({"schema":"verdant-voice-runtime/1","name":"verdant-voice","version":"0.1.0",
            "strategy":STRATEGY,"source_manifest_sha256":"a".repeat(64)});
        validate_voice_declaration(&serde_json::from_value(declaration.clone())?,&"a".repeat(64))?;
        for version in ["0.1.1","0.1.0-rc.1+local.01"] {
            let mut compatible=declaration.clone();compatible["version"]=version.into();
            validate_voice_declaration(&serde_json::from_value(compatible)?,&"a".repeat(64))?;
        }
        for (key,value) in [("schema",json!("unknown-runtime/1")),("name",json!("other-voice")),
            ("version",json!("01.0.0")),("version",json!("0.1.0-01")),("version",json!("0.1.0+")),
            ("strategy",json!("other")),("source_manifest_sha256",json!("b".repeat(64)))] {
            let mut bad=declaration.clone();bad[key]=value;
            assert!(validate_voice_declaration(&serde_json::from_value(bad)?,&"a".repeat(64)).is_err(),"{key}");
        }
        let mut extra=declaration.clone();extra["training_updates"]=json!(0);
        assert!(serde_json::from_value::<VoiceManifest>(extra).is_err());
        for key in ["schema","name","version","strategy","source_manifest_sha256"] {
            let mut missing=declaration.clone();missing.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<VoiceManifest>(missing).is_err(),"{key}");
        }
        let duplicate=format!("{{\"schema\":\"verdant-voice-runtime/1\",\"name\":\"verdant-voice\",\"version\":\"0.1.0\",\"version\":\"0.0.1\",\"strategy\":\"{STRATEGY}\",\"source_manifest_sha256\":\"{}\"}}","a".repeat(64));
        assert!(serde_json::from_str::<VoiceManifest>(&duplicate).is_err());
        let target=target();let reference=reference();
        let (current,current_report)=controller_conditions(&target,&reference,None)?;
        assert_eq!(current["f0_hz"].values,reference.values);
        assert_eq!(current_report["candidate"],"verdant-voice");
        Ok(())
    }
}
