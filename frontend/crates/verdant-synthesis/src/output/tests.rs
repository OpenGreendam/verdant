//! Consumer invariants and regressions for the real production processors.
//! Parent validation must additionally compare active waveforms and numeric
//! detector traces against the frozen ce1ce6c oracle with NumPy 2.3.5/SciPy
//! 1.17.0. Exact equality applies to bypass/protected samples and seeded Rust
//! replays; FFT/library arithmetic on active samples needs measured tolerances.
use super::*;
use crate::core::Arrays;
use crate::dsp::rng::NumpyPcg64;

fn controls(count: usize, period: f64, voiced: bool, class: &str) -> AcousticControlTrack {
    AcousticControlTrack {
        f0_hz: vec![if voiced { 120.0 } else { 0.0 }; count], voiced: vec![voiced; count],
        phonetic_classes: vec![class.into(); count], phone_identity: vec![601; count],
        segment_identity: vec![0; count], continuity_identity: vec![0; count],
        period_seconds: period, origin_seconds: 0.0,
    }
}
fn audio(samples: Vec<f64>, rate: u32, dtype: &str) -> AudioResult {
    AudioResult { sample_rate: rate, samples, source_dtype: dtype.into(), arrays: Arrays::new(),
        metadata: json!({"retained_upstream_provenance": {"route": "C1", "source": "untouched"}}) }
}
fn expression(key: &str, value: f64, begin: f64, end: f64) -> Value {
    let mut curves = Map::new();
    curves.insert(key.into(), json!([[0.0, value]]));
    json!({"method": EXPRESSION_METHOD, "segments": [{"start_seconds": begin,
        "end_seconds": end, "note_onset_seconds": begin, "curves": curves}]})
}
fn guard() -> Value {
    json!({"method": TRANSIENT_METHOD, "strength": 1.0, "max_attenuation_db": 6.0})
}
fn bits(samples: &[f64]) -> Vec<u64> { samples.iter().map(|v| v.to_bits()).collect() }
fn harmonic(rate: u32, count: usize) -> Vec<f64> {
    let mut source = vec![0.0; count];
    for (index, sample) in source.iter_mut().enumerate() {
        let time = index as f64 / rate as f64;
        for harmonic in 1..24 {
            let frequency = harmonic as f64 * 120.0;
            let strength = (0.12 + 3.0 * (-0.5 * ((frequency - 720.0) / 110.0).powi(2)).exp()
                + 2.0 * (-0.5 * ((frequency - 1500.0) / 170.0).powi(2)).exp()) /
                (harmonic as f64).powf(0.55);
            *sample += strength * (2.0 * std::f64::consts::PI * frequency * time + 0.13 * harmonic as f64).sin();
        }
    }
    let scale = 0.3 / peak(&source);
    for sample in &mut source { *sample *= scale; }
    source
}
fn transient_fixture(rate: u32) -> (Vec<f64>, AcousticControlTrack) {
    let mut cells = controls(80, 0.005, true, "vowel");
    for index in 0..80 {
        let region = index / 20;
        cells.phone_identity[index] = [9001, 7447, 5555, 9001][region];
        cells.segment_identity[index] = region as i64;
        if region == 1 || region == 2 {
            cells.voiced[index] = false;
            cells.f0_hz[index] = 0.0;
            cells.phonetic_classes[index] = if region == 1 { "stop_unaspirated" } else { "sibilant" }.into();
        } else { cells.f0_hz[index] = 300.0; }
    }
    let count = seconds_to_samples(0.4, rate as f64);
    let mut samples: Vec<f64> = (0..count).map(|index|
        0.1 * (2.0 * std::f64::consts::PI * 300.0 * index as f64 / rate as f64).sin()).collect();
    let mut rng = NumpyPcg64::new(29);
    let begin = seconds_to_samples(0.1, rate as f64);
    let end = seconds_to_samples(0.3, rate as f64);
    for sample in &mut samples[begin..end] { *sample = 0.02 * rng.standard_normal(); }
    for time in [0.149, 0.151, 0.237] { samples[seconds_to_samples(time, rate as f64)] += 0.75; }
    (samples, cells)
}

#[test]
fn neutral_all_controllers_preserve_floating_bits_and_upstream_provenance() {
    let mut source = harmonic(16000, 1600);
    source[0] = -0.0;
    source[800] = 1.25;
    let cells = controls(20, 0.005, true, "vowel");
    for dtype in ["float32", "float64"] {
        let source: Vec<f64> = source.iter().map(|&v| if dtype == "float32" { f64::from(v as f32) } else { v }).collect();
        for (key, value) in [("BRE", 0.0), ("BRI", 64.0), ("CLE", 0.0), ("GEN", 64.0), ("OPE", 127.0), ("GWL", 0.0)] {
            let output = finalize(audio(source.clone(), 16000, dtype), &cells,
                Some(&expression(key, value, 0.0, 0.1)), None, None, 94).unwrap();
            assert_eq!(bits(&output.samples), bits(&source), "{dtype}/{key}");
            assert_eq!(output.source_dtype, dtype);
            let report = &output.metadata["output_pipeline"];
            assert_eq!(report["processors"]["voice_expression"]["exact_bypass"], true);
            assert!(report["processors"].get("expression_headroom").is_none());
            assert_eq!(report["changed_float_samples"], 0);
            assert_eq!(output.metadata["retained_upstream_provenance"]["source"], "untouched");
        }
    }
}

#[test]
fn compiled_curves_retain_collapsed_gap_ownership_and_finite_map_support() {
    let source = harmonic(16000, 5000);
    let cells = controls(63, 0.005, true, "vowel");
    let rendered = [0.0, 0.105, 0.105, 0.21, 0.31];
    let authored = [0.0, 0.1025, 0.105, 0.2075, 0.3105];
    for (start, end, sample_begin, sample_end) in [(0.0, 0.1025, 0, 1680), (0.105, 0.2075, 1680, 3360)] {
        let output = finalize(audio(source.clone(), 16000, "float64"), &cells,
            Some(&expression("BRI", 20.0, start, end)), None, Some((&rendered, &authored)), 94).unwrap();
        let active = &output.arrays["expression_active"].values;
        for index in 0..source.len() {
            let expected = index >= sample_begin && index < sample_end;
            assert_eq!(active[index] != 0.0, expected, "sample {index}");
            if !expected { assert_eq!(output.samples[index].to_bits(), source[index].to_bits()); }
        }
    }
    let output = finalize(audio(source.clone(), 16000, "float64"), &cells,
        Some(&expression("BRI", 20.0, 0.0, 0.32)), None, Some((&rendered, &authored)), 94).unwrap();
    assert!(output.arrays["expression_active"].values[..4960].iter().all(|&v| v == 1.0));
    assert!(output.arrays["expression_active"].values[4960..].iter().all(|&v| v == 0.0));
    assert_eq!(bits(&output.samples[4960..]), bits(&source[4960..]));
}

#[test]
fn malformed_maps_and_unadmitted_fields_fail_even_for_neutral_expression() {
    let cells = controls(1, 0.005, true, "vowel");
    let neutral = expression("BRI", 64.0, 0.0, 0.005);
    for (rendered, authored) in [
        (vec![0.0, 0.1, 0.09], vec![0.0, 0.1, 0.2]),
        (vec![0.0, 0.1], vec![0.0]),
        (vec![0.0, 0.0], vec![0.0, 0.1]),
        (vec![0.0, 0.1], vec![0.0, 0.0]),
        (vec![0.01, 0.1], vec![0.0, 0.1]),
    ] {
        assert!(finalize(audio(vec![0.0; 80], 16000, "float64"), &cells,
            Some(&neutral), None, Some((&rendered, &authored)), 94).is_err());
    }
    let mut invalid = neutral;
    invalid["segments"][0]["curves"]["BRI"][0][1] = json!(true);
    assert!(validate_expression(&invalid).is_err());
    let mut invalid = guard(); invalid["unexpected"] = json!(1);
    assert!(validate_output_transients(&invalid).is_err());
    for value in [-0.1, 127.1] { assert!(validate_expression(&expression("GEN", value, 0.0, 0.005)).is_err()); }
    assert!(validate_expression(&expression("UNSUPPORTED", 1.0, 0.0, 0.005)).is_err());
}

#[test]
fn every_controller_works_at_declared_bounds_and_breath_replays_one_seeded_stream() {
    let source = harmonic(16000, 4096);
    let cells = controls(52, 0.005, true, "vowel");
    let mut altered = Vec::new();
    for (key, default) in [("BRE", 0.0), ("BRI", 64.0), ("CLE", 0.0), ("GEN", 64.0), ("OPE", 127.0), ("GWL", 0.0)] {
        for value in [0.0, 127.0] {
            let output = finalize(audio(source.clone(), 16000, "float64"), &cells,
                Some(&expression(key, value, 0.0, 0.256)), None, None, 94).unwrap();
            assert!(output.samples.iter().all(|v| v.is_finite()));
            if value == default { assert_eq!(bits(&output.samples), bits(&source)); }
            else {
                let error = output.samples.iter().zip(&source).map(|(a, b)| (a - b).powi(2)).sum::<f64>();
                assert!(error > 1e-5, "{key}={value} must not silently bypass");
                if value == 127.0 || key == "OPE" { altered.push((key, output.samples)); }
            }
        }
    }
    for (index, (name, first)) in altered.iter().enumerate() {
        for (other, second) in altered.iter().skip(index + 1) {
            assert!(first.iter().zip(second).any(|(a, b)| (a - b).abs() > 1e-6), "{name}/{other}");
        }
    }
    let track = expression("BRE", 80.0, 0.0, 0.256);
    let one = finalize(audio(source.clone(), 16000, "float64"), &cells, Some(&track), None, None, 94).unwrap();
    let two = finalize(audio(source.clone(), 16000, "float64"), &cells, Some(&track), None, None, 94).unwrap();
    let other = finalize(audio(source, 16000, "float64"), &cells, Some(&track), None, None, 95).unwrap();
    assert_eq!(bits(&one.samples), bits(&two.samples));
    assert_ne!(bits(&one.samples), bits(&other.samples));
}

#[test]
fn float32_source_float64_headroom_target_keeps_its_comparison_slack() {
    // The exact upstream sample and expression-created peak are retained from
    // PR15's failing C1 regression, without mocking the headroom processor.
    let source = vec![-0.0, f64::from(0.5647878050804138_f32), 1.25, 0.0];
    let mut current = source.clone(); current[1] = 1.1384392568966775;
    let active = [false, true, false, false];
    let blend = [0.0, 1.0, 0.0, 0.0];
    let (report, gain) = apply_headroom(&mut current, SampleDtype::Float64, &source,
        SampleDtype::Float32, &active, &blend, 16000).unwrap();
    assert_eq!(report["applied"], true);
    assert!(current[1] > 0.98);
    assert!(current[1] <= f64::from(0.98_f32) + 1e-12);
    assert_eq!(current[0].to_bits(), source[0].to_bits());
    assert_eq!(current[2].to_bits(), source[2].to_bits());
    assert_eq!(gain.unwrap()[2], 1.0);
    assert_eq!(report["outside_active_bit_exact"], true);
    let mut inherited = vec![1.2];
    let (report, gain) = apply_headroom(&mut inherited, SampleDtype::Float64,
        &[1.2], SampleDtype::Float64, &[true], &[0.0], 16000).unwrap();
    assert_eq!(inherited, vec![1.2]);
    assert_eq!(report["applied"], false); assert!(gain.is_none());
}

#[test]
fn transient_guard_is_monotone_bounded_and_protects_voiced_samples_at_all_output_rates() {
    for rate in [16000, 44100, 48000] {
        let (source, cells) = transient_fixture(rate);
        let output = finalize(audio(source.clone(), rate, "float64"), &cells,
            None, Some(&guard()), None, 10008).unwrap();
        let eligible = &output.arrays["transients_sample_eligible"].values;
        let gain = &output.arrays["transients_sample_gain"].values;
        for index in 0..source.len() {
            assert!(output.samples[index].abs() <= source[index].abs());
            assert!(gain[index] >= 10.0_f64.powf(-6.0 / 20.0) - 1e-12);
            if eligible[index] == 0.0 { assert_eq!(output.samples[index].to_bits(), source[index].to_bits()); }
        }
        assert_eq!(output.metadata["output_pipeline"]["processors"]["output_transients"]["applied"], true);
        let peak = seconds_to_samples(0.149, rate as f64);
        assert!(output.samples[peak].abs() < source[peak].abs() * 0.6);
    }
}

#[test]
fn explicit_guard_zero_and_protected_control_classes_are_exact_bypasses() {
    let (source, cells) = transient_fixture(16000);
    for field in ["strength", "max_attenuation_db"] {
        let mut configuration = guard(); configuration[field] = json!(0.0);
        let output = finalize(audio(source.clone(), 16000, "float64"), &cells,
            None, Some(&configuration), None, 10008).unwrap();
        assert_eq!(bits(&output.samples), bits(&source));
    }
    for mode in 0..3 {
        let mut protected = cells.clone();
        if mode == 0 { protected.phonetic_classes.fill("unknown".into()); }
        if mode == 1 { protected.continuity_identity.fill(-1); }
        if mode == 2 { protected.voiced.fill(true); protected.f0_hz.fill(300.0); }
        let output = finalize(audio(source.clone(), 16000, "float64"), &protected,
            None, Some(&guard()), None, 10008).unwrap();
        assert_eq!(bits(&output.samples), bits(&source));
        assert!(output.arrays["transients_sample_eligible"].values.iter().all(|&v| v == 0.0));
    }
}

#[test]
fn transient_anchors_do_not_depend_on_identity_numbers_or_neighboring_phones() {
    let (source, cells) = transient_fixture(16000);
    let mut renamed = cells.clone();
    for value in &mut renamed.phone_identity { *value = *value * 100 + 37; }
    for value in &mut renamed.segment_identity { *value = *value * 7 + 55; }
    for value in &mut renamed.continuity_identity { *value += 222; }
    let first = finalize(audio(source.clone(), 16000, "float64"), &cells, None, Some(&guard()), None, 10008).unwrap();
    let renamed = finalize(audio(source.clone(), 16000, "float64"), &renamed, None, Some(&guard()), None, 10008).unwrap();
    assert_eq!(bits(&first.samples), bits(&renamed.samples));
    let mut quiet_neighbor = source;
    for sample in &mut quiet_neighbor[3200..] { *sample *= 0.01; }
    let quiet = finalize(audio(quiet_neighbor, 16000, "float64"), &cells, None, Some(&guard()), None, 10008).unwrap();
    assert_eq!(bits(&first.samples[..3200]), bits(&quiet.samples[..3200]));
}

#[test]
fn pcm_publication_preserves_untouched_codes_and_guard_only_integer_attenuation() {
    let original = [3276, -3277, 10, -32768];
    let source = [f64::from(0.1_f32), f64::from(-0.1_f32), 10.0 / 32768.0, -1.0];
    let final_float = [source[0], source[1], 20.0 / 32768.0, -1.0];
    assert_eq!(quantize_preserving_pcm16(&original, &source, &final_float, false).unwrap(), original);
    assert_eq!(quantize_preserving_pcm16(&original, &source, &final_float, true).unwrap(), [3276, -3277, 20, -32768]);
    let codes = [1, 2, 3, -2];
    let source = codes.map(|v| v as f64 / 32768.0);
    let ties = [0.5, 1.5, 2.5, -1.5].map(|v| v / 32768.0);
    assert_eq!(quantize_preserving_pcm16(&codes, &source, &ties, true).unwrap(), [0, 2, 2, -2]);
}

#[test]
fn tiny_float32_regions_keep_source_precision_and_clock_support() {
    let mut cells = controls(2, 1.0 / 16000.0, false, "sibilant");
    cells.phonetic_classes[1] = "stop_unaspirated".into();
    cells.phone_identity[1] = 2;
    for samples in [vec![0.0, -0.0], vec![f64::from(0.2_f32), f64::from(-0.3_f32)]] {
        let result = finalize(audio(samples.clone(), 16000, "float32"), &cells,
            None, Some(&guard()), None, 10008).unwrap();
        assert_eq!(result.source_dtype, "float32");
        assert_eq!(bits(&result.samples), bits(&samples));
    }
    let (source, mut shifted) = transient_fixture(16000);
    shifted.origin_seconds = 0.1;
    let output = finalize(audio(source.clone(), 16000, "float64"), &shifted,
        None, Some(&guard()), None, 10008).unwrap();
    assert_eq!(bits(&output.samples[..1600]), bits(&source[..1600]));
    assert!(output.arrays["transients_sample_eligible"].values[..1600].iter().all(|&v| v == 0.0));
}

#[test]
fn expression_headroom_precedes_the_final_guard_without_global_normalization() {
    let (mut source, cells) = transient_fixture(16000);
    source[0] = 1.2; source[5600] = 1.05;
    let result = finalize(audio(source.clone(), 16000, "float64"), &cells,
        Some(&expression("BRI", 127.0, 0.0, 0.1)), Some(&guard()), None, 94).unwrap();
    let report = &result.metadata["output_pipeline"];
    assert_eq!(report["stage_order"], json!(["voice_expression", "expression_headroom", "output_transients"]));
    assert_eq!(report["normalization"], "none");
    assert_eq!(result.samples[0], 1.2);
    assert_eq!(result.samples[5600], 1.05);
    assert_eq!(bits(&result.samples[4800..]), bits(&source[4800..]));
    assert_eq!(report["processors"]["expression_headroom"]["outside_active_bit_exact"], true);
    assert_eq!(report["processors"]["output_transients"]["noneligible_samples_bit_exact"], true);
    assert_eq!(result.arrays["waveform_final"].values, result.samples);
    assert_eq!(result.arrays["sample_rate"].shape, Vec::<usize>::new());
}

#[test]
fn float32_source_uses_original_float64_frame_input_precision() {
    // Frozen apply_voice_expression.frame_input copies source samples into
    // np.zeros(dtype=float64). Identical f32-valued input therefore has the
    // same active waveform whether its declared source dtype is f32 or f64.
    // Compare before local headroom: its source-typed ceiling legitimately
    // differs and has a separate PR15 regression. This exercises actual DSP,
    // including both FFT sizes and seeded breath with GEN/OPE, not source text.
    for rate in [16000, 44100] {
        let count = rate as usize / 5;
        let mut source: Vec<f64> = harmonic(rate, count).into_iter()
            .map(|value| f64::from(value as f32)).collect();
        source[0] = -0.0;
        let mut cells = controls(40, 0.005, true, "vowel");
        cells.voiced[10..15].fill(false);
        cells.f0_hz[10..15].fill(0.0);
        cells.phonetic_classes[10..15].fill("fricative".into());
        cells.continuity_identity[30..35].fill(-1);
        let mut tracks: Vec<Value> = [("BRE", 127.0), ("BRI", 0.0), ("CLE", 127.0),
            ("GEN", 0.0), ("OPE", 0.0), ("GWL", 127.0)].into_iter()
            .map(|(key, value)| expression(key, value, 0.0, 0.2)).collect();
        tracks.push(json!({"method": EXPRESSION_METHOD, "segments": [{
            "start_seconds": 0.0, "end_seconds": 0.2, "note_onset_seconds": 0.0,
            "curves": {"BRE": [[0.0, 80.0]], "BRI": [[0.0, 110.0]],
                "CLE": [[0.0, 80.0]], "GEN": [[0.0, 110.0]],
                "OPE": [[0.0, 30.0]], "GWL": [[0.0, 80.0]]}}]}));
        for track in tracks {
            let promoted = expression::apply(&source, SampleDtype::Float64, rate,
                &cells, &track, 94, None).unwrap();
            let from_float32 = expression::apply(&source, SampleDtype::Float32, rate,
                &cells, &track, 94, None).unwrap();
            assert_eq!(promoted.report["enabled"], true);
            assert_eq!(from_float32.dtype, SampleDtype::Float64);
            assert_eq!(promoted.active, from_float32.active);
            assert_eq!(promoted.blend, from_float32.blend);
            assert_eq!(bits(&from_float32.samples), bits(&promoted.samples),
                "active source promotion at {rate} Hz: {track}");
            assert!(promoted.samples.iter().zip(&source).any(|(a, b)| a != b));
            for (index, &active) in from_float32.active.iter().enumerate() {
                if !active {
                    assert_eq!(from_float32.samples[index].to_bits(), source[index].to_bits());
                }
            }
        }
    }
}
