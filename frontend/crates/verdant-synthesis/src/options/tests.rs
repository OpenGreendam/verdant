use super::*;
use crate::core::{AcousticControlTrack, Arrays};
use std::f64::consts::PI;

fn controls(frames: usize, voiced: bool, class: &str) -> AcousticControlTrack {
    AcousticControlTrack { f0_hz: vec![if voiced { 400. } else { 0. }; frames],
        voiced: vec![voiced; frames], phonetic_classes: vec![class.into(); frames],
        phone_identity: vec![723; frames], segment_identity: vec![0; frames],
        continuity_identity: vec![0; frames], period_seconds: 0.005, origin_seconds: 0. }
}
fn lsf(frames: usize) -> NumericArray {
    let row: Vec<f64> = (0..12).flat_map(|i| {
        let center = 0.14+i as f64*(2.9-0.14)/11.;
        [center-0.006, center+0.006]
    }).collect();
    NumericArray::new(vec![frames, 24], "float64", (0..frames).flat_map(|_| row.iter().copied()).collect()).unwrap()
}
fn segment(language: &str, ipa: &str, key: &str, index: usize) -> Value {
    json!({"phoneme": {"source_language": language, "canonical_ipa": ipa, "native_key": key,
        "source_index": index, "syllable_index": 0}})
}
fn same_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| a.to_bits() == b.to_bits())
}

#[test]
fn explicit_zero_phonetics_bypasses_legacy_trace_and_unvalidated_grid() {
    let source = lsf(1);
    let (output, report) = correct_spectrum(&source, &[json!({"phoneme": {"label": "legacy"}})],
        &[], &[], 0, 0, &json!({"method": "articulatory-v1", "strength": 0.}), f64::NAN).unwrap();
    assert!(same_bits(&source.values, &output.values));
    assert!(report.is_none());
    assert!(validate_phonetic_correction(&json!({"method": "articulatory-v1", "strength": true})).is_err());
    assert!(validate_vocal_tract(&json!({"method": "bilinear-v1", "ratio": 1., "pitch": 1.})).is_err());
}

#[test]
fn japanese_recipe_requires_its_actual_language_and_native_selection() {
    let source = lsf(40);
    let config = json!({"method": "articulatory-v1", "strength": 1.});
    let (chinese, report) = correct_spectrum(&source, &[segment("zh", "u", "zh:u", 0)],
        &[40], &[json!("zh:u")], 16000, 80, &config, 0.994).unwrap();
    assert!(same_bits(&source.values, &chinese.values));
    assert_eq!(report.unwrap()["segments"][0]["status"], "unmodified");
    for forged in [segment("ja", "ɯ", "en:uw", 0), segment("en", "ɯ", "zh:u", 0)] {
        let key = forged["phoneme"]["native_key"].clone();
        assert!(correct_spectrum(&source, &[forged], &[40], &[key], 16000, 80, &config, 0.994).is_err());
    }
    let (japanese, report) = correct_spectrum(&source, &[segment("ja", "ɯ", "zh:u", 0)],
        &[40], &[json!("zh:u")], 16000, 80, &config, 0.994).unwrap();
    let report = report.unwrap();
    assert_eq!(report["profile_sha256"], "914d64a666de32ba38533743387945416cad9ebaab1d15f87669f023d4aaefdf");
    let operation = &report["segments"][0]["operations"][0];
    let below = operation["identity_below_hz"].as_f64().unwrap();
    let above = operation["identity_above_hz"].as_f64().unwrap();
    let mut maximum = 0f64;
    for (&before, &after) in source.values.iter().zip(&japanese.values) {
        let hz = before*8000./PI;
        if hz <= below || hz >= above { assert_eq!(before.to_bits(), after.to_bits()); }
        maximum = maximum.max(after-before);
    }
    assert!(maximum > 0.02);
}

#[test]
fn splitting_one_dynamic_vowel_does_not_create_a_spectral_seam() {
    let mut source = lsf(80);
    for (frame, values) in source.values.chunks_exact_mut(24).enumerate() {
        let scale = 0.85+frame as f64*(1.03-0.85)/79.;
        for value in values { *value *= scale; }
    }
    let config = json!({"method": "articulatory-v1", "strength": 1.});
    let (joined, _) = correct_spectrum(&source, &[segment("ja", "ɯ", "zh:u", 0)],
        &[80], &[json!("zh:u")], 16000, 80, &config, 0.994).unwrap();
    let (split, report) = correct_spectrum(&source,
        &[segment("ja", "ɯ", "zh:u", 0), segment("ja", "ɯ", "zh:u", 1)],
        &[40, 40], &[json!("zh:u"), json!("zh:u")], 16000, 80, &config, 0.994).unwrap();
    assert!(same_bits(&joined.values, &split.values));
    let report = report.unwrap();
    assert_eq!(report["segments"][0]["operations"][0]["landmark"], report["segments"][1]["operations"][0]["landmark"]);
}

#[test]
fn contextual_nasals_and_shared_palatal_helpers_keep_their_source_admission() {
    let config = json!({"method": "articulatory-v1", "strength": 1.});
    let source = lsf(40);
    let mut nasal = segment("ja", "ɴ", "zh:m", 0);
    nasal["phoneme"]["realization"] = json!({"schema_version": "verdant-japanese-realization/1",
        "source_mora": "ん", "source_phonological": "moraic_nasal", "target_ipa": "m", "following_ipa": "p"});
    let (output, report) = correct_spectrum(&source, &[nasal.clone()], &[40], &[json!("zh:m")],
        16000, 80, &config, 0.994).unwrap();
    assert!(same_bits(&source.values, &output.values));
    assert_eq!(report.unwrap()["segments"][0]["trace"]["effective_ipa"], "m");
    nasal["phoneme"]["realization"]["following_ipa"] = json!("k");
    assert!(correct_spectrum(&source, &[nasal], &[40], &[json!("zh:m")], 16000, 80, &config, 0.994).is_err());

    let mut source = lsf(80);
    for value in &mut source.values[20*24..60*24] { *value = *value*0.93+0.015; }
    let base = segment("ja", "kʲ", "zh:g", 0);
    let helper = segment("ja", "kʲ", "en:y", 0);
    let following = segment("ja", "a", "zh:a", 1);
    let (joined, _) = correct_spectrum(&source, &[base.clone(), helper.clone(), following.clone()],
        &[20, 40, 20], &[json!("zh:g"), json!("en:y"), json!("zh:a")], 16000, 80, &config, 0.994).unwrap();
    let (split, report) = correct_spectrum(&source, &[base.clone(), helper.clone(), helper.clone(), following.clone()],
        &[20, 20, 20, 20], &[json!("zh:g"), json!("en:y"), json!("en:y"), json!("zh:a")], 16000, 80, &config, 0.994).unwrap();
    assert!(same_bits(&joined.values, &split.values));
    assert!(same_bits(&split.values[20*24..60*24], &source.values[20*24..60*24]));
    let report = report.unwrap();
    assert_eq!(report["segments"][1]["status"], "reference_only");
    assert_eq!(report["segments"][2]["status"], "reference_only");
    let mut invalid_unit = helper;
    invalid_unit["phoneme"]["source_index"] = json!(true);
    let (unchanged, _) = correct_spectrum(&source, &[base, invalid_unit, following],
        &[20, 40, 20], &[json!("zh:g"), json!("en:y"), json!("zh:a")], 16000, 80, &config, 0.994).unwrap();
    assert!(same_bits(&unchanged.values, &source.values));
}

#[test]
fn anchored_warp_inverts_the_actual_cubic_and_has_independent_knots() {
    let config = json!({"method": "anchored-warp-v1", "ratio": 1.125, "anchor_ratios": [1.03, 1.08]});
    let preimages = warp_frequencies(&[800., 2400.], 16000.,
        &json!({"method": "bilinear-v1", "ratio": 1./1.125})).unwrap();
    let mapped = warp_frequencies(&preimages, 16000., &config).unwrap();
    assert!((mapped[0]-824.).abs() < 2e-11);
    assert!((mapped[1]-2592.).abs() < 2e-11);
    let grid: Vec<f64> = (0..=512).map(|i| i as f64*8000./512.).collect();
    let source = NumericArray::new(vec![grid.len()], "float64", grid.clone()).unwrap();
    let inverse = warp_log_envelope(&source, &grid, 16000., &config).unwrap();
    let recovered = warp_frequencies(&inverse.values, 16000., &config).unwrap();
    assert_eq!(recovered[0], 0.);
    assert_eq!(recovered[512], 8000.);
    assert!(recovered.iter().zip(&grid).all(|(a, b)| (a-b).abs() < 1e-9));
    assert!(inverse.values.windows(2).all(|pair| pair[1] > pair[0]));
}

#[test]
fn neutral_anchors_dispatch_the_identical_bilinear_fit_and_gamma_is_baked_once() {
    let source = lsf(1);
    for ratio in [0.8, 1.125, 1.25] {
        let bilinear = json!({"method": "bilinear-v1", "ratio": ratio});
        let anchored = json!({"method": "anchored-warp-v1", "ratio": ratio, "anchor_ratios": [1., 1.]});
        let (left, report) = transform_lsf(&source, &bilinear, 16000., 0.994).unwrap();
        let (right, _) = transform_lsf(&source, &anchored, 16000., 0.994).unwrap();
        assert!(same_bits(&left.values, &right.values));
        assert_eq!(report.unwrap()["output_bandwidth_gamma"], 1.);
        let mut expanded = crate::dsp::lpc::lsf_to_lpc(&source.values).unwrap();
        for (i, value) in expanded.iter_mut().enumerate() { *value *= 0.994f64.powi(i as i32); }
        let (expected, _, _) = warp_lpc_envelope(&NumericArray::new(vec![1, 25], "float64", expanded).unwrap(), &bilinear, 8192).unwrap();
        let reconstructed = crate::dsp::lpc::lsf_to_lpc(&left.values).unwrap();
        assert!(reconstructed.iter().zip(&expected.values).all(|(a, b)| (a-b).abs() < 1e-8));
    }
    let (unity, report) = transform_lsf(&source,
        &json!({"method": "anchored-warp-v1", "ratio": 1., "anchor_ratios": [1., 1.]}), 16000., 0.994).unwrap();
    assert!(same_bits(&source.values, &unity.values));
    assert_eq!(report.unwrap()["output_bandwidth_gamma"], 0.994);
}

#[test]
fn dynamics_preserve_uv_and_each_opaque_boundary_with_signed_zero() {
    let mut track = controls(20, true, "vowel");
    for i in 0..4 { track.phonetic_classes[i] = "stop_unaspirated".into(); track.voiced[i] = false; track.f0_hz[i] = 0.; }
    for i in 16..20 { track.phonetic_classes[i] = "nasal".into(); }
    let mut source: Vec<f64> = (0..1600).map(|i| ((i as f64*0.1).sin() as f32) as f64).collect();
    source[1] = -0.;
    let logs: Vec<f64> = (0..20).map(|i| 5.-i as f64*10./19.).collect();
    let config = json!({"method": "voiced-envelope-v1", "strength": 0.25, "max_attenuation_db": 6.});
    let result = apply_model_dynamics(&source, &logs, &track, &config, 16000.).unwrap().unwrap();
    assert_eq!(result.sample_gain[320], 1.);
    assert_eq!(result.sample_gain[1279], 1.);
    assert!(result.sample_gain.iter().any(|&gain| gain < 0.9));
    for i in 0..source.len() {
        if !result.sample_eligible[i] { assert_eq!(source[i].to_bits(), result.samples[i].to_bits()); }
    }
    assert_eq!(track.f0_hz[0], 0.);
    assert_eq!(track.f0_hz[4], 400.);
    let mut split = controls(8, true, "vowel");
    split.phone_identity[4..].fill(17);
    let source = vec![0.3; 640];
    let logs = [0., 0., 0., 0., 5., 5., 5., 5.];
    let result = apply_model_dynamics(&source, &logs, &split, &config, 16000.).unwrap().unwrap();
    assert!(same_bits(&source, &result.samples));
    assert_eq!(result.report["regions"][0]["anchor_log_gain"], 0.);
    assert_eq!(result.report["regions"][1]["anchor_log_gain"], 5.);
}

#[test]
fn dynamics_use_complete_fractional_sample_clocks_and_extreme_median_residuals() {
    let mut track = controls(8, true, "vowel");
    track.period_seconds = 0.01;
    track.origin_seconds = 12.125;
    let logs = [0., 1., 2., 3., 4., 5., 6., 7.];
    let config = json!({"method": "voiced-envelope-v1", "strength": 0.25, "max_attenuation_db": 6.});
    let source = vec![0.1; 1764];
    let absolute = apply_model_dynamics(&source, &logs, &track, &config, 22050.).unwrap().unwrap();
    track.origin_seconds = 0.;
    let relative = apply_model_dynamics(&source, &logs, &track, &config, 22050.).unwrap().unwrap();
    assert!(same_bits(&absolute.samples, &relative.samples));
    let low: f64 = 1e308;
    let high = f64::from_bits(low.to_bits()+1);
    let huge = apply_model_dynamics(&vec![1.; 160], &[low, high], &controls(2, true, "vowel"), &config, 16000.).unwrap().unwrap();
    assert!(huge.sample_gain[40] < 0.9);
}

#[test]
fn continuation_keeps_note_pitch_and_dynamics_ownership_without_a_spectral_reattack() {
    let frames = 40;
    let first = segment("ja", "ɯ", "zh:u", 0);
    let mut second = first.clone();
    second["continuation_of"] = json!(0);
    let mut track = controls(frames, true, "vowel");
    track.segment_identity[20..].fill(1);
    track.f0_hz[20..].fill(440.);
    let mut arrays = Arrays::new();
    let source = lsf(frames);
    arrays.insert("lsf_radians".into(), source.clone());
    arrays.insert("generated_log_gain".into(), NumericArray::new(vec![frames], "float64",
        (0..frames).map(|i| if i < 20 { 0. } else { 5. }).collect()).unwrap());
    arrays.insert("segment_index".into(), NumericArray::new(vec![frames], "int64",
        (0..frames).map(|i| if i < 20 { 0. } else { 1. }).collect()).unwrap());
    let metadata = json!({"sample_rate": 16000, "hop_samples": 80,
        "segments": [first.clone(), second], "frame_counts": [20, 20],
        "native_keys": ["zh:u", "zh:u"], "segment_indices": [0, 1],
        "source_continuation_links": [{"segment_index": 1, "continuation_of": 0}]});
    let mut run = RunParameters { start_frame: 0, frames, arrays, controls: track,
        metadata: metadata.clone() };
    let options = json!({"phonetic_correction": {"method": "articulatory-v1", "strength": 1.},
        "model_dynamics": {"method": "voiced-envelope-v1", "strength": 0.25, "max_attenuation_db": 6.}});
    let (joined, _) = correct_spectrum(&source, &[first], &[frames], &[json!("zh:u")],
        16000, 80, &options["phonetic_correction"], 0.994).unwrap();
    apply_spectral_options(&mut run, &options).unwrap();
    assert!(same_bits(&run.arrays["lsf_radians"].values, &joined.values));
    assert_eq!(run.metadata["segments"], metadata["segments"]);
    assert_eq!(run.metadata["source_continuation_links"], metadata["source_continuation_links"]);
    assert_eq!(&run.controls.segment_identity[..20], &[0; 20]);
    assert_eq!(&run.controls.segment_identity[20..], &[1; 20]);
    assert!(run.controls.f0_hz[..20].iter().all(|&f0| f0 == 400.));
    assert!(run.controls.f0_hz[20..].iter().all(|&f0| f0 == 440.));
    let source_wave = vec![0.65; frames*80];
    let mut audio = AudioResult { sample_rate: 16000, samples: source_wave.clone(),
        source_dtype: "float64".into(), arrays: Arrays::new(), metadata: json!({}) };
    apply_waveform_options(&mut audio, &run, &options).unwrap();
    assert!(same_bits(&audio.samples, &source_wave));
    assert_eq!(audio.metadata["model_dynamics"]["regions"].as_array().unwrap().len(), 2);
    assert_eq!(audio.metadata["model_dynamics"]["regions"][0]["anchor_log_gain"], 0.);
    assert_eq!(audio.metadata["model_dynamics"]["regions"][1]["anchor_log_gain"], 5.);
}

#[test]
fn noise_conditioning_interpolates_in_db_and_rejects_invalid_explicit_zero() {
    let config = json!({"method": "unvoiced-model-envelope-v1", "strength": 0.25, "max_attenuation_db": 6.});
    let track = controls(5, false, "stop_unaspirated");
    let result = condition_noise_envelope(&vec![1.; 400], &[-3., -2., 0., 1., 2.], &track, &config, 16000.).unwrap().unwrap();
    assert_eq!(result.sample_gain[0], 1.);
    assert_eq!(result.sample_gain[399], 1.);
    let next_db = -2.*0.25*20./std::f64::consts::LN_10;
    let expected = -6.+(next_db+6.)*(0.5/80.);
    assert!((20.*result.sample_gain[120].log10()-expected).abs() < 2e-14);
    let zero = json!({"method": "unvoiced-model-envelope-v1", "strength": 0., "max_attenuation_db": 6.});
    assert!(condition_noise_envelope(&vec![-1.; 400], &[0.; 5], &track, &zero, 16000.).is_err());
    assert!(condition_noise_envelope(&vec![1.; 399], &[0.; 5], &track, &zero, 16000.).is_err());
}

#[test]
fn contact_detection_uses_the_unchanged_reference_once() {
    let frames = 40;
    let source = lsf(frames);
    let (_, report) = correct_spectrum(&source, &[segment("ja", "ɾ", "en:dx", 0)], &[frames],
        &[json!("en:dx")], 16000, 80, &json!({"method": "articulatory-v1", "strength": 1.}), 0.994).unwrap();
    let candidate: Vec<f64> = (0..frames*80).map(|i| (2.*PI*400.*(i as f64+0.5)/16000.).sin()).collect();
    let mut reference = candidate.clone();
    for (i, sample) in reference.iter_mut().enumerate() {
        if ((i as f64+0.5)/16000.-0.1).abs() < 0.009 { *sample *= 0.05; }
    }
    let run = RunParameters { start_frame: 0, frames, arrays: Arrays::new(), controls: controls(frames, true, "sonorant"),
        metadata: json!({"sample_rate": 16000, "hop_samples": 80, "phonetic_correction": report.unwrap()}) };
    let mut arrays = Arrays::new();
    arrays.insert("calibration_reference_waveform".into(), NumericArray::new(vec![reference.len()], "float64", reference.clone()).unwrap());
    let mut audio = AudioResult { sample_rate: 16000, samples: candidate.clone(), source_dtype: "float64".into(), arrays, metadata: json!({}) };
    apply_phonetic_waveform(&mut audio, &run).unwrap();
    assert!(same_bits(&audio.samples, &candidate));
    assert!(same_bits(&audio.arrays["calibration_reference_waveform"].values, &reference));
    assert_eq!(audio.metadata["dsp"]["phonetic_correction"]["segments"][0]["operations"][0]["applied"], false);
    assert!(audio.arrays["phonetic_correction_gain_frame_min"].values.iter().all(|&gain| gain == 1.));
}

#[test]
fn one_tap_contact_is_shared_across_allocated_segments_and_low_pitch_is_protected() {
    let frames = 80;
    let source = lsf(frames);
    let tap = segment("ja", "ɾ", "en:dx", 0);
    let config = json!({"method": "articulatory-v1", "strength": 1.});
    let (_, joined) = correct_spectrum(&source, &[tap.clone()], &[frames], &[json!("en:dx")], 16000, 80, &config, 0.994).unwrap();
    let (_, split) = correct_spectrum(&source, &[tap.clone(), tap], &[40, 40],
        &[json!("en:dx"), json!("en:dx")], 16000, 80, &config, 0.994).unwrap();
    for hz in [50., 250., 1661.21879, 2000.] {
        let wave: Vec<f64> = (0..frames*80).map(|i| (2.*PI*hz*i as f64/16000.).sin()).collect();
        let f0 = vec![hz; frames];
        let voiced = vec![true; frames];
        let a = correct_waveform(&wave, &f0, &voiced, &mut joined.clone().unwrap(), 16000, 80).unwrap();
        let b = correct_waveform(&wave, &f0, &voiced, &mut split.clone().unwrap(), 16000, 80).unwrap();
        assert!(same_bits(&a, &b));
        if hz == 50. { assert!(a.iter().all(|&gain| gain == 1.)); }
        else {
            let affected: Vec<usize> = a.iter().enumerate().filter_map(|(i, &gain)| (gain < 0.99999).then_some(i)).collect();
            assert!(!affected.is_empty());
            assert!(affected.windows(2).all(|pair| pair[1] == pair[0]+1));
            assert!(a[..2000].iter().all(|&gain| gain == 1.));
        }
    }
}
