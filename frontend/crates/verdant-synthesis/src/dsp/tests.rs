//! Independent numerical contracts. Parent runs these after all owners settle;
//! pinned NumPy/SciPy cross-language comparisons remain an additional gate.
use super::*;
use crate::core::AcousticControlTrack;

#[test]
fn numpy_pcg64_matches_published_upstream_integer_vectors() {
    for csv in [include_str!("fixtures/pcg64-testset-1.csv"),include_str!("fixtures/pcg64-testset-2.csv")] {
        let mut lines = csv.lines();
        let seed = lines.next().unwrap().split(',').nth(1).unwrap().trim();
        let seed = u64::from_str_radix(seed.trim_start_matches("0x"),16).unwrap();
        let mut random = rng::NumpyPcg64::new(seed);
        for (line_number,line) in lines.enumerate() {
            if line.trim().is_empty() {continue;}
            let expected = line.split(',').nth(1).unwrap().trim();
            let expected = u64::from_str_radix(expected.trim_start_matches("0x"),16).unwrap();
            assert_eq!(random.next_u64(),expected,"published seed={seed} vector {line_number}");
        }
    }
}

#[test]
fn seed_sequence_preserves_original_unspawned_42_vector() {
    // NumPy v2.3.5 random/tests/test_seed_sequence.py, test_zero_padding.
    let state = rng::seed_sequence_state(42);
    assert_eq!([state[0] as u32,(state[0]>>32) as u32,state[1] as u32,(state[1]>>32) as u32],
        [3444837047,2669555309,2046530742,3581440988]);
    assert_ne!(state,rng::seed_sequence_state(42_u64<<32));
}

#[test]
fn numpy_normal_stream_is_invariant_to_consumer_chunk_boundaries() {
    for seed in [0,10008,0x56584334,u64::MAX] {
        let whole = rng::NumpyPcg64::new(seed).standard_normal_vec(16384);
        let mut random = rng::NumpyPcg64::new(seed);
        let mut chunks = Vec::new();
        for count in [1,7,128,321,8192,7735] {chunks.extend(random.standard_normal_vec(count));}
        assert_eq!(whole.len(),chunks.len());
        assert!(whole.iter().all(|x|x.is_finite()));
        assert!(whole.iter().zip(chunks).all(|(a,b)|a.to_bits()==b.to_bits()));
    }
}

#[test]
fn synthesis_lattice_matches_independent_constant_coefficient_recurrence() {
    let a = [1.0,-0.7,0.2];
    let reflection = lpc::lpc_to_reflection(&a).unwrap();
    let source = rng::NumpyPcg64::new(1234).standard_normal_vec(640);
    let coefficients:Vec<_> = (0..8).flat_map(|_|reflection.iter().copied()).collect();
    let actual = lpc::lattice_filter(&source,&coefficients,8,2,80).unwrap();
    let mut expected = vec![0.0;source.len()];
    for n in 0..expected.len() {
        expected[n] = source[n];
        for k in 1..=2 {if n >= k {expected[n] -= a[k]*expected[n-k];}}
    }
    assert!(actual.iter().zip(expected).all(|(a,b)|(a-b).abs() <= 2e-14));
    assert!(lpc::lpc_to_reflection(&[1.0,0.0,1.0]).is_err());
    assert!(lpc::lattice_filter(&source,&[1.0;16],8,2,80).is_err());
}

#[test]
fn lsf_lpc_conversion_is_strict_and_roundtrips_resonators() {
    let mut coefficients = vec![1.0];
    for (hz,width) in [(300.0,70.0),(1100.0,100.0),(2600.0,180.0),(6500.0,250.0)] {
        let radius = (-PI*width/16000.0).exp();
        let section = [1.0,-2.0*radius*(2.0*PI*hz/16000.0).cos(),radius*radius];
        let mut next = vec![0.0;coefficients.len()+2];
        for (i,&a) in coefficients.iter().enumerate() {
            for (j,&b) in section.iter().enumerate() {next[i+j] += a*b;}
        }
        coefficients = next;
    }
    coefficients.resize(25,0.0);
    let lsf = lpc::lpc_to_lsf(&coefficients).unwrap();
    let reconstructed = lpc::lsf_to_lpc(&lsf).unwrap();
    assert!(coefficients.iter().zip(reconstructed).all(|(a,b)|(a-b).abs() < 2e-9));
    assert!(lpc::maximum_pole_radius(&coefficients).unwrap() < 1.0);
    assert!(lpc::lsf_to_lpc(&[0.2,0.1]).is_err());
    assert!(lpc::lsf_to_lpc(&[0.0,0.5]).is_err());
    assert!(lpc::lpc_to_lsf(&[1.0,0.0,1.01]).is_err());
}

#[test]
fn periodic_phase_freezes_in_uv_without_resetting_next_voiced_island() {
    let first = 13;
    let gap = 17;
    let last = 40;
    let compact_f0:Vec<_> = (0..first).map(|_|347.0).chain((0..last).map(|_|400.0)).collect();
    let f0:Vec<_> = (0..first).map(|_|347.0).chain((0..gap).map(|_|999.0)).chain((0..last).map(|_|400.0)).collect();
    let voiced:Vec<_> = (0..f0.len()).map(|i|i<first || i>=first+gap).collect();
    for source in ["rosenberg","impulse"] {
        let (with_gap,info) = excitation::bandlimited_excitation(&f0,&voiced,16000,80,source,8,&[0.0],None).unwrap();
        let (compact,_) = excitation::bandlimited_excitation(&compact_f0,&vec![true;compact_f0.len()],16000,80,source,8,&[0.0],None).unwrap();
        for i in (first+gap)*80+160..with_gap.len()-160 {
            assert!((with_gap[i]-compact[i-gap*80]).abs() < 2e-11);
        }
        assert!(with_gap[first*80+65..(first+gap)*80-65].iter().all(|v|*v==0.0));
        assert_eq!(info["fir_group_delay_output_samples"],64.0);
        assert_eq!(info["fir_edge_support_output_samples"],64);
    }
}

#[test]
fn absent_and_zero_harmonic_controls_take_exact_same_source_path() {
    let f0 = vec![347.0;31];
    let mut voiced = vec![true;31];
    voiced[11..20].fill(false);
    let brightness:Vec<_> = (0..31).map(|i|i as f64/5.0).collect();
    for source in ["rosenberg","impulse"] {
        let (base,metadata) = excitation::bandlimited_excitation(&f0,&voiced,16000,80,source,8,&brightness,None).unwrap();
        let (zero,zero_metadata) = excitation::bandlimited_excitation(&f0,&voiced,16000,80,source,8,&brightness,Some(&[[0.0;2];31])).unwrap();
        assert_eq!(metadata,zero_metadata);
        assert!(base.iter().zip(zero).all(|(a,b)|a.to_bits()==b.to_bits()));
        let mut uv_only = [[0.0;2];31];
        uv_only[11..20].fill([18.0,9.0]);
        let (uv,uv_metadata) = excitation::bandlimited_excitation(&f0,&voiced,16000,80,source,8,&brightness,Some(&uv_only)).unwrap();
        assert_eq!(metadata,uv_metadata);
        assert!(base.iter().zip(uv).all(|(a,b)|a.to_bits()==b.to_bits()));
    }
}


#[test]
fn harmonic_balance_admits_only_static_collapsed_monophthong_and_keeps_bounds() {
    let frames = 200;
    let order = 24;
    let one:Vec<f64> = [0.10,0.11,0.19,0.20].into_iter()
        .chain((0..20).map(|i|0.55+i as f64*((3.04-0.55)/19.0))).collect();
    let row:Vec<f64> = lpc::lsf_to_lpc(&one).unwrap().into_iter().enumerate()
        .map(|(i,a)|a*0.994_f64.powi(i as i32)).collect();
    let lsf:Vec<_> = (0..frames).flat_map(|_|one.iter().copied()).collect();
    let lpc:Vec<_> = (0..frames).flat_map(|_|row.iter().copied()).collect();
    let f0 = vec![500.0;frames];
    let model = vec![225.0;frames];
    let voiced = vec![true;frames];
    let ids = vec![29;frames];
    let segments = vec![0;frames];
    let predict = |model:&[f64],voiced:&[bool],ids:&[i64],segments:&[i64],cap:f64| {
        harmonic_balance::vowel_harmonic_gains(&lpc,&lsf,order,&f0,model,voiced,ids,segments,
            16000,80,"impulse",&[0.0],cap).unwrap()
    };
    let (gains,report) = predict(&model,&voiced,&ids,&segments,18.0);
    assert!(gains[100][0] > 10.0 && gains[100][0] > gains[100][1]);
    assert_eq!(report["active_native_keys"],json!(["zh:u"]));
    assert!(gains.iter().flatten().all(|&gain|(0.0..=18.0).contains(&gain)));
    for (model,voiced,ids,cap) in [
        (vec![1.0;frames],voiced.clone(),ids.clone(),18.0),
        (model.clone(),vec![false;frames],ids.clone(),18.0),
        (model.clone(),voiced.clone(),vec![15;frames],18.0),
        (model.clone(),voiced.clone(),ids.clone(),0.0)] {
        let (zero,_) = predict(&model,&voiced,&ids,&segments,cap);
        assert!(zero.iter().flatten().all(|&gain|gain == 0.0));
    }
    let mut split = segments;
    split[100..].fill(1);
    let (split_gains,_) = predict(&model,&voiced,&ids,&split,18.0);
    assert!(split_gains[94..106].iter().flatten().all(|&gain|gain == 0.0));
    assert!(split_gains[50][0] > 0.0 && split_gains[150][0] > 0.0);
}

#[test]
fn phonetic_source_preserves_uv_and_keeps_one_final_long_stop_release() {
    let stop = (0..400).find(|&id|phonetic_source::native_phone_key(id)==Some("zh:k")).unwrap();
    let n = 100;
    let controls = phonetic_source::source_controls(&vec![stop;n],&vec![0;n],&vec![false;n],
        16000,80,4.0,Some(&vec![2;n])).unwrap();
    assert!(controls.periodic_gain.iter().all(|v|*v==0.0));
    assert!(controls.noise_gain[..7000].iter().all(|&v|v==0.008));
    assert!(controls.metadata["segments"][0]["closure_seconds"].as_f64().unwrap() > 0.45);
    assert!(phonetic_source::source_controls(&[99999],&[0],&[true],16000,80,4.0,None).is_err());
    assert!(phonetic_source::source_controls(&[stop,29],&[0,0],&[false,true],16000,80,4.0,None).is_err());
}

#[test]
fn relative_gain_neutral_paths_preserve_signed_zero_and_active_gain_is_bounded() {
    let original = [0.0,-0.0,0.25,-0.5,0.0,-0.0,0.2,-0.2];
    for (mode,strength,cap,logs) in [
        ("off",0.25,6.0,[1.0,7.0]),("relative",0.0,6.0,[1.0,7.0]),
        ("relative",0.25,0.0,[1.0,7.0]),("relative",0.25,6.0,[7.0,7.0])] {
        let mut actual = original;
        let info = relative_gain::apply_relative_model_gain(&mut actual,&logs,4,mode,strength,cap).unwrap();
        assert_eq!(info["applied"],false);
        assert!(original.iter().zip(actual).all(|(a,b)|a.to_bits()==b.to_bits()));
    }
    let mut wave = vec![0.1;30];
    let info = relative_gain::apply_relative_model_gain(&mut wave,&[-1e300,0.0,1e300],10,"relative",0.25,6.0).unwrap();
    assert_eq!(info["frame_control_db_range"],json!([-6.0,6.0]));
    assert!(wave.iter().all(|v|v.is_finite() && *v <= 0.1*10.0_f64.powf(6.0/20.0)+1e-14));
}

fn constant_run(frames:usize, split:Option<usize>, active:bool) -> RunParameters {
    let f0 = vec![if active{300.0}else{0.0};frames];
    let voiced = vec![active;frames];
    let segments:Vec<i64> = (0..frames).map(|i|if split.is_some_and(|boundary|i>=boundary){1}else{0}).collect();
    let mut arrays = Arrays::new();
    let lsf:Vec<_> = (0..frames).flat_map(|_|(1..=24).map(|i|i as f64*PI/25.0)).collect();
    arrays.insert("lsf_radians".into(),NumericArray::new(vec![frames,24],"float64",lsf).unwrap());
    for (key,dtype,values) in [
        ("f0_hz","float64",f0.clone()),("voiced","bool",voiced.iter().map(|&v|if v{1.0}else{0.0}).collect()),
        ("native_phone_id","int64",vec![29.0;frames]),
        ("segment_index","int64",segments.iter().map(|&v|v as f64).collect()),
        ("phonemic_length","int64",vec![1.0;frames]),("generated_log_gain","float64",vec![1.3;frames])] {
        arrays.insert(key.into(),NumericArray::new(vec![frames],dtype,values).unwrap());
    }
    RunParameters {start_frame:0,frames,arrays,
        controls:AcousticControlTrack {f0_hz:f0,voiced,phonetic_classes:vec!["vowel".into();frames],
            phone_identity:vec![29;frames],segment_identity:segments,continuity_identity:vec![0;frames],
            period_seconds:0.005,origin_seconds:0.0},
        metadata:json!({"sample_rate":16000,"hop_samples":80})}
}

fn pcm16(values:&[f64])->Vec<i16> {
    values.iter().map(|v|(v.clamp(-0.999,0.999)*32767.0).round_ties_even() as i16).collect()
}

#[test]
fn split_vowel_preserves_whole_run_default_pcm_phase_rng_and_filter_state() {
    let joined = constant_run(46,None,true);
    let split = constant_run(46,Some(23),true);
    let options = json!({});
    let mut a = synthesize_run(&joined,&options,10008).unwrap();
    let mut b = synthesize_run(&split,&options,10008).unwrap();
    calibrate_run(&mut a,&joined,&options).unwrap();
    calibrate_run(&mut b,&split,&options).unwrap();
    assert_eq!(pcm16(&a.samples),pcm16(&b.samples));
    assert_eq!(a.arrays["lpc_coefficients"].shape,[46,25]);
    assert_eq!(a.arrays["parcor_coefficients"].shape,[46,24]);
    assert_eq!(a.arrays["lpc_coefficients"].dtype,"float64");
    assert!(peak(&a.samples[23*80-80..23*80+80]) > 0.01);
    assert!((peak(&a.samples)-0.65).abs() < 1e-14);
    assert!(calibrate_run(&mut a,&joined,&options).is_err());
}

#[test]
fn all_uv_legacy_sources_keep_the_same_exact_seeded_noise_output() {
    let run = constant_run(20,None,false);
    let mut results = Vec::new();
    for source in ["rosenberg","impulse"] {
        let options = json!({"source_profile":"legacy","source":source});
        let result = synthesize_run(&run,&options,10008).unwrap();
        assert!(result.samples.iter().any(|v|v.abs()>0.1));
        results.push(result.samples);
    }
    assert!(results[0].iter().zip(&results[1]).all(|(a,b)|a.to_bits()==b.to_bits()));
}


#[test]
fn only_explicit_continuation_groups_source_without_erasing_note_identity() {
    let mut run = constant_run(46,Some(23),true);
    let ids = run.controls.phone_identity.clone();
    let authored = run.controls.segment_identity.clone();
    let before = run.arrays["segment_index"].values.clone();
    let ordinary = source_segment_groups(&run,&ids,&authored).unwrap();
    assert_eq!(&*ordinary,&authored);
    run.metadata["source_continuation_links"] = json!([{"segment_index":1,"continuation_of":0}]);
    let grouped = source_segment_groups(&run,&ids,&authored).unwrap();
    assert!(grouped.iter().all(|&id|id == 0));
    assert_eq!(run.controls.segment_identity,authored);
    assert_eq!(run.arrays["segment_index"].values,before);
    let ordinary_run = constant_run(46,None,true);
    let mut continued = synthesize_run(&run,&json!({}),10008).unwrap();
    let mut whole = synthesize_run(&ordinary_run,&json!({}),10008).unwrap();
    calibrate_run(&mut continued,&run,&json!({})).unwrap();
    calibrate_run(&mut whole,&ordinary_run,&json!({})).unwrap();
    assert_eq!(pcm16(&continued.samples),pcm16(&whole.samples));
    let mut invalid = run.clone();
    invalid.metadata["source_continuation_links"] = json!([{"segment_index":1,"continuation_of":999}]);
    assert!(source_segment_groups(&invalid,&ids,&authored).is_err());
    invalid.metadata["source_continuation_links"] = json!([{"segment_index":1,"continuation_of":1}]);
    assert!(source_segment_groups(&invalid,&ids,&authored).is_err());
    let mut changed_ids = ids;
    changed_ids[23..].fill(15);
    assert!(source_segment_groups(&run,&changed_ids,&authored).is_err());
}

#[test]
fn calibration_uses_unchanged_source_reference_instead_of_candidate_peak() {
    let run = constant_run(2,None,true);
    let mut arrays = Arrays::new();
    arrays.insert(CALIBRATION_REFERENCE.into(),NumericArray::new(vec![160],"float64",vec![0.5;160]).unwrap());
    let mut candidate = AudioResult {sample_rate:16000,samples:vec![0.25;160],source_dtype:"float64".into(),
        arrays,metadata:json!({"dsp":{"consonant_dynamics":{"applied":true}}})};
    calibrate_run(&mut candidate,&run,&json!({})).unwrap();
    assert!(candidate.samples.iter().all(|&v|v==0.325));
    assert_eq!(candidate.metadata["run_peak_calibration_gain"],1.3);
    assert!(!candidate.arrays.contains_key(CALIBRATION_REFERENCE));
    assert_eq!(candidate.metadata["consonant_dynamics"]["calibration_reference_peak"],0.5);
}
