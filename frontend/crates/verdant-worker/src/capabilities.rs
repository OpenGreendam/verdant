use std::{collections::BTreeMap,path::Path};
use anyhow::Result;
use serde_json::{Value,json};
use crate::{Config,PROTOCOL,MAX_REQUEST_BYTES,artifacts,runtime};
include!(concat!(env!("OUT_DIR"),"/sources.rs"));
pub fn source_hashes()->BTreeMap<String,String>{SOURCE_FILES.iter().map(|(path,hash)|(path.to_string(),hash.to_string())).collect()}
pub fn source_files()->BTreeMap<String,Value>{SOURCE_FILES.iter().map(|(path,hash)|(format!("synthesis:{path}"),json!({"available":true,"sha256":hash}))).collect()}
fn file(path:&Path,usable:bool)->Result<Value>{
    let hash = path.is_file().then(|| artifacts::sha(path)).transpose()?;
    Ok(json!({"available":usable && hash.is_some(), "sha256":hash}))
}
pub fn describe(config:&Config)->Result<Value>{
    let package = runtime::load_voice_package(config);
    let loaded = package.as_ref().ok();
    let admitted = loaded.filter(|package| runtime::validate_voice_receipt(config, package).is_ok());
    let confirmed = loaded.is_some_and(|package| package.voice.source_identity_confirmed());
    let model_metadata = loaded.map(|package| package.voice.metadata());
    let voice_version = loaded.map(|package| package.voice_version.as_str());
    let native_f0_range = verdant_synthesis::parameters::NATIVE_F0_RANGE;
    let catalog = config.voice_registry.as_ref().map(|path| match verdant_backend::voices::VoiceRegistry::load_or_empty(path) {
        Ok(registry) => registry.capabilities(),
        Err(error) => json!({"error":error.to_string()}),
    });
    let original = catalog.as_ref().and_then(|catalog| catalog["voices"].get(verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE))
        .and_then(|entry| serde_json::from_value::<verdant_backend::wire::VoiceEntry>(entry.clone()).ok())
        .is_some_and(|entry| verdant_backend::voices::is_desktop_original(&entry));
    let mut routes = vec!["verdant-voice"];
    let original_routes: &[&str] = if original {
        routes.push("C0");
        &["C0"]
    } else { &[] };
    let mut files = source_files();
    let registered = config.selection.as_ref().and_then(|selection| selection.get("source_sha256")).and_then(Value::as_object);
    for (role, _) in verdant_backend::voices::PACKAGE_FILES {
        let hash = loaded.and_then(|package| package.hashes.get(*role)).map(String::as_str);
        let available = hash.is_some() && registered.is_none_or(|registered| registered.get(*role).and_then(Value::as_str) == hash);
        files.insert((*role).into(), json!({"available":available,"sha256":hash}));
    }
    files.insert("voice_source_admission".into(), json!({"available":admitted.is_some(),"sha256":admitted.map(|p|p.source_manifest_sha256())}));
    files.insert("worker_binary".into(), file(&std::env::current_exe()?, true)?);
    if let Ok(path) = runtime::onnxruntime_path(config) { files.insert("onnxruntime".into(), file(&path, true)?); }
    else { files.insert("onnxruntime".into(), json!({"available":false,"sha256":null})); }
    let mut inventory:Value=serde_json::from_str(include_str!("../../verdant-backend/src/data/pinyin_inventory.json"))?;
    let mut syllables=if let Some(map)=inventory["syllables"].as_object(){map.keys().cloned().collect::<Vec<_>>()}else{inventory["syllables"].as_array_mut().unwrap().iter().map(|s|s.as_str().unwrap().to_owned()).collect()};syllables.sort();
    let phonemes:Value=serde_json::from_str(include_str!("../../verdant-backend/src/data/phoneme_inventory.json"))?;
    let languages=phonemes["languages"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),v["support"].clone())).collect::<serde_json::Map<_,_>>();
    let mut result=json!({
        "engine":"Verdant Rust synthesis worker","protocol":PROTOCOL,"native_version":env!("CARGO_PKG_VERSION"),"worker_version":env!("CARGO_PKG_VERSION"),
        "routes":routes,"max_active_jobs":crate::max_active_jobs(),"max_phrase_seconds":30.,"max_request_bytes":MAX_REQUEST_BYTES,
        "timing_roundoff_tolerance_seconds":1e-12,"audio_device_required_for_render":false,"wine_required":false,
        "dependency_mode":"native_rust","production_python_required":false,
        "native_score":{
            "explicit_original_hierarchy":{"schema":"ogd-explicit-original-hierarchy/1","requires_segment_node_indices":true,"requires_provenance":true,"original_text_frontend":false},
            "inventory":syllables,"inventory_note":"explicit pinyin sequencing helper; no general Hanzi frontend",
            "phoneme_frontend":{"schema_version":phonemes["schema_version"],"alphabets":phonemes["supported_alphabets"],"languages":languages,"canonical_alphabet":"ipa","text_g2p":false,
                "approximation_policy":"explicit opt-in; retain target IPA and every selected native phone"},
            "parameter_generation":"verified original scalar MLPG and LSF correction","neighbor_context":"authored immediate score neighbors; original iterators remain unrecovered",
            "sub_syllable_timing":true,"phoneme_duration_control":true,
            "compact_phone_states":true,
            "compact_phone_states_scope":"verdant-voice uses a fixed 5 ms acoustic clock and permits one frame per phone",
            "phoneme_timing":{"methods":["vowel-anchor-v1"],"routes":routes,"timing_resolution_seconds":0.005,
                "continuity":{"method":"continuous-runs-v1","max_seconds":120.,"max_segments":2048,"max_control_points":32768,"minimum_silence_seconds":0.005,
                    "requires_phoneme_timing":"vowel-anchor-v1","execution":"whole connected run, one frame plan and MLPG; no artificial split"}},
            "absolute_boundary_timing":true,"duration_roundoff_tolerance_seconds":1e-12,
            "timing_policy":"shared absolute frame boundaries; half-frame ties toward the later frame; no independent per-segment rounding",
            "time_map_fields":["time_map_render_seconds","time_map_authored_seconds"],"time_map_policy":"native rendered time to authored score time; right-continuous across zero-frame gaps",
            "english_vowel_proxies":["a","i","u","e","o"],"f0":"continuous authored control, automatic MIDI connections, per-segment or model-derived",
            "explicit_f0_hz_range":native_f0_range,"continuous_f0_control":true,"coarticulation_recovered":false,
            "high_f0_status":"bounded engineering range; high-register quality requires listening; original model pitch range is unchanged",
            "singing_defaults":{"duration_mode":"singing","source_profile":"phonetic","prosody_mode":"auto","automatic_portamento_ms_max":80,"vowel_brightness_db":4.,"gain_mode":"off"},
            "state_duration_modes":["singing","model"],"source_profiles":["phonetic","legacy"],"gain_modes":["off","relative"],"vowel_balance_db_range":[0,18],
            "phonetic_correction":{"methods":["articulatory-v1"],"strength_range":[0,1],"routes":original_routes,"requires_explicit_phoneme_trace":true,"preserves_authored_f0":true,"preserves_note_timing":true,"unmatched_targets":"reported without acoustic correction"},
            "vocal_tract":{"methods":["bilinear-v1","anchored-warp-v1"],"ratio_range":[0.8,1.25],"default_ratio":1.,"default_enabled":false,"anchor_ratio_range":[0.8,1.25],"anchor_fractions":[0.1,0.3],"default_anchor_ratios":[1.,1.],"anchor_method":"anchored-warp-v1","routes":original_routes},
            "model_dynamics":{"methods":["voiced-envelope-v1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.,"routes":original_routes},
            "consonant_dynamics":{"methods":["unvoiced-model-envelope-v1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.,"routes":original_routes},
            "expression":{"methods":["vocaloid-controls-v1"],"parameters":["BRE","BRI","CLE","GEN","OPE","GWL"],"range":[0,127],"defaults":{"BRE":0.,"BRI":64.,"CLE":0.,"GEN":64.,"OPE":127.,"GWL":0.},"routes":routes,"maximum_compiled_curve_points":32768,"maximum_segments":2048,"point_scope":"compiled note-relative seconds, including exact tempo-boundary anchors","stage":"after_reconstruction_before_output_transients","implementation":"Verdant functional reconstruction; not VOCALOID binary equivalence"},
            "output_transients":{"methods":["phoneme-output-envelope-v1"],"minimum_strength":0.,"maximum_strength":1.,"minimum_attenuation_db":0.,"maximum_attenuation_db":12.,"routes":routes,"stage":"final waveform after reconstruction and voice expression","eligible_classes":["affricate","aspiration","fricative","sibilant","stop_aspirated","stop_unaspirated","stop_voiced"],"sample_peak_monotone":true,"voiced_samples_bit_exact":true},
            "generated_high_band_default":false
        },
        "voice_package":{"name":"verdant-voice","voice_version":voice_version,
            "display_name":voice_version.map(|version|format!("verdant-voice {version}")).unwrap_or_else(||"verdant-voice".into()),
            "strategy":verdant_synthesis::voice::STRATEGY,
            "mainline_decoder_candidate":"decoder-48k",
            "authored_f0_hz_range":native_f0_range,"authored_f0_policy":"native-target-reference-v1",
            "authored_f0_scope":"native target decoder range; native reference is separately admitted",
            "controller_reference":{"source":"separate-native-reference-plan-unclipped-mlpg","f0_hz_range":[45.,800.],
                "native_acoustic_policy":verdant_synthesis::parameters::TERMINAL_NUCLEUS_REFERENCE_POLICY,
                "target_f0_preserved":true,"decoder_target_conditions_preserved":true},
            "sample_rate":48000,"hop_samples":240,"source_evidence_required":false,"exact_leaf_bank_used":true,
            "original_statistics_and_mlpg_required":true,
            "music":{"methods":["verdant-voice-music-v1"],"routes":["verdant-voice"],"sample_rates":[44100,48000],"default_sample_rate":48000,
                "preserves_learned_conditions":true,"generated_highband":false,"trained_highband":false,
                "stage":"after native decoding, before voice expression and final output processors",
                "articulation_scope":"native semantic model; music selects output rate and attenuation-only headroom",
                "omitted_request_policy":"unchanged 48000 Hz verdant-voice decoded output"},
            "learned_controller":true,"streaming":false,"decoder_core_frames":64,"decoder_context_frames":16,
            "fallback_allowed":false,"normalization":"none",
            "source_evidence_scope":"installation receipt for an independently pinned bank, or an explicit original-source admission with verified bytes and user attestation",
            "supported_languages":{"zh":"native","en":"native","ja":"approximate"},
            "language_scope":"retained frontend mapping support; neural quality must be evaluated per language",
            "legacy_processor_policy":"harmonic compensation, phonetic waveform correction, vocal-tract warp, generated highband and model/consonant gain processors are rejected"},
        "voice_source":"verdant-voice","model_metadata":model_metadata,"files":files,
        "progress":"real stage boundaries only; no fabricated percentage",
        "cancellation":"supervised Rust process group, TERM then KILL after two seconds; partial outputs invalid"
    });
    if let Some(catalog) = catalog { result["voices"] = catalog; }
    crate::bundled_voice::extend_capabilities(config,&mut result)?;
    if let Some(provenance)=&config.bundled_voice{
        result["voice_package"]["source_evidence_scope"]="hash-pinned artifact-specific redistribution policy; package integrity and live graph ABI admission; no local original-source attestation".into();
        result["voice_package"]["bundled_voice_admission"]=serde_json::to_value(provenance)?;
    }
    if confirmed {
        let source_voice=json!({"name":"sn","display_name":"楠楠","vid":10008,"historical_identity_confirmed":true,
            "identity_confirmed_by":"owner_2026-11-13","target_preset":"童声01","confirmation_ref":"IDENTITY-CONFIRMATION-2026-11-13",
            "technical_limits":"diagnostic parameter reconstruction; coarticulation/frontend unrecovered"});
            result["source_voice"]=source_voice;
            result["candidate_voice"]=json!({"name":"verdant-voice","voice_version":voice_version,
                "display_name":voice_version.map(|version|format!("verdant-voice {version}")).unwrap_or_else(||"verdant-voice".into()),
                "historical_identity_confirmed":true});
    }
    Ok(result)
}
