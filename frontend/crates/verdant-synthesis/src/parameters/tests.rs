//! Independent immutable recovered-function receipts and planner invariants.
//! These do not invoke Python, a proprietary engine, native DSP or a model.
use super::*;
use serde_json::{json,Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

fn acoustic_fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/original-acoustic-synthetic-fixture.json")).unwrap()
}
fn flattened(value: &Value) -> Vec<f64> {
    match value {
        Value::Array(values) => values.iter().flat_map(flattened).collect(),
        _ => vec![value.as_f64().unwrap()],
    }
}
fn close(actual: &[f64], expected: &[f64], relative: f64, absolute: f64) {
    assert_eq!(actual.len(),expected.len());
    for (index,(&a,&b)) in actual.iter().zip(expected).enumerate() {
        assert!((a-b).abs() <= absolute+relative*b.abs(), "index {index}: {a:?} != {b:?}");
    }
}
#[test]
fn cumulative_140_bpm_oracle_has_5486_frames_and_27_43_seconds() {
    let whole = (60.0_f64/140.0*1e6).round()/1e6;
    let onset = (0.22*whole*1e6).round()/1e6;
    let segments: Vec<Value> = (0..64).flat_map(|_| [onset,whole-onset]).map(|d| json!({"duration_seconds":d})).collect();
    let plan = plan_score_timing(&segments,16000,80).unwrap();
    assert_eq!(plan.total_frames,5486);
    assert_eq!(plan.render_seconds.last(),Some(&27.43));
    for note in 0..64 {
        assert!((plan.segments[note*2+1].end_frame as f64*0.005-(note+1) as f64*whole).abs() <= 0.0025+1e-12);
    }
}
#[test]
fn quantized_gaps_and_binary_residue_keep_one_shared_clock() {
    let plan = plan_score_timing(&[
        json!({"duration_seconds":0.1025,"gap_after_seconds":0.0025}),
        json!({"duration_seconds":0.1025,"gap_after_seconds":0.0175}),
        json!({"duration_seconds":0.1})],16000,80).unwrap();
    assert_eq!(plan.segments.iter().map(|s| (s.start_frame,s.end_frame,s.gap_end_frame)).collect::<Vec<_>>(),
        vec![(0,21,21),(21,42,45),(45,65,65)]);
    assert_eq!(&plan.render_seconds[..3],&[0.0,0.105,0.105]);
    assert_eq!(&plan.authored_seconds[..3],&[0.0,0.1025,0.105]);
    let residue = plan_score_timing(&[0.10250000000000001,2.475,2.5-2.475].map(|d| json!({"duration_seconds":d})),16000,80).unwrap();
    assert_eq!(residue.segments.iter().map(SegmentTiming::frames).collect::<Vec<_>>(),[21,495,5]);
    assert_eq!(residue.total_frames,521);
    assert_eq!(plan_score_timing(&[json!({"duration_seconds":0.1025-1e-8})],16000,80).unwrap().total_frames,20);
    for segments in [vec![],vec![json!({"duration_seconds":0.001})],vec![json!({"duration_seconds":true})],
        vec![json!({"duration_seconds":0.1,"gap_after_seconds":-1})]] {
        assert!(plan_score_timing(&segments,16000,80).is_err());
    }
}
#[test]
fn compact_states_keep_exact_budgets_and_standard_allocations() {
    let spectral = vec![0.0;5*3*41];
    let expected = [[0,0,1,0,0],[0,1,0,1,0],[1,0,1,0,1],[1,1,1,0,1]];
    for mode in ["model","singing"] { for (index,counts) in expected.iter().enumerate() {
        let total = index+1;
        let allocate = || duration::allocate_voice_state_durations(total,&[2.0;5],&[1.0;5],Some("zh:a"),Some(3),&spectral,mode,0.005).unwrap();
        let (actual,evidence) = allocate();
        assert_eq!(actual.as_slice(),counts);
        assert_eq!(allocate(),(actual.clone(),evidence.clone()));
        assert_eq!(actual.iter().sum::<usize>(),total);
        assert_eq!(evidence["compact_phone_states"],true);
        assert!(allocate_singing_state_durations(total,&[2.0;5],&[1.0;5],Some("zh:a"),Some(3),&spectral,mode,0.005).is_err());
        for offset in 0..total {
            assert_eq!(source::state_occupancy_with_compact(&actual,offset,1,true).unwrap().iter().sum::<usize>(),1);
        }
        assert!(source::state_occupancy(&actual,0,total).is_err());
        assert_eq!(voicing::infer_voicing_with_compact(&actual,&[0.9;5],&[3;5],Some(&[1;5]),true).unwrap().0,vec![true;total]);
        assert!(infer_voicing(&actual,&[0.9;5],&[3;5],None).is_err());
    }}
    for total in [5,6,20,400] { for mode in ["model","singing"] {
        assert_eq!(duration::allocate_voice_state_durations(total,&[2.0;5],&[1.0;5],Some("zh:a"),Some(3),&spectral,mode,0.005).unwrap(),
            allocate_singing_state_durations(total,&[2.0;5],&[1.0;5],Some("zh:a"),Some(3),&spectral,mode,0.005).unwrap());
    }}
}

#[test]
fn compact_clock_keeps_boundaries_without_zero_frame_segments() {
    let segments = [0.005,0.01,0.015,0.02].map(|seconds| json!({"duration_seconds":seconds}));
    let plan = timing::plan_score_timing_with_floor(&segments,16000,80,1).unwrap();
    assert_eq!(plan.total_frames,10);
    assert_eq!(plan.segments.iter().map(|s| (s.start_frame,s.end_frame)).collect::<Vec<_>>(),[(0,1),(1,3),(3,6),(6,10)]);
    assert_eq!(plan.authored_seconds,[0.0,0.005,0.015,0.03,0.05]);
    assert!(plan_score_timing(&segments,16000,80).is_err());
    assert!(timing::plan_score_timing_with_floor(&[json!({"duration_seconds":0.001})],16000,80,1).is_err());
}

fn duration_objective(counts: &[usize], means: &[f64], variances: &[f64]) -> BigRational {
    counts.iter().zip(means).zip(variances).fold(BigRational::zero(),|sum,((&count,&mean),&variance)| {
        let delta = BigRational::from_integer(BigInt::from(count))-BigRational::from_float(mean).unwrap();
        sum+&delta*&delta/(BigRational::from_integer(BigInt::from(2))*BigRational::from_float(variance).unwrap())
    })
}
#[test]
fn exact_allocator_matches_independent_exhaustive_gaussian_objective() {
    for (means,variances) in [([2.0,3.0,1.0],[1.0,1.0,1.0]),([0.25,3.5,1.25],[0.125,8.0,2.0]),
        ([8.0,2.0,6.0],[3.0,0.5,20.0])] {
        for total in 3..16 {
            let result = allocate_state_durations(total,&means,&variances).unwrap();
            let objective = duration_objective(&result,&means,&variances);
            for a in 1..total-1 { for b in 1..total-a {
                assert!(objective <= duration_objective(&[a,b,total-a-b],&means,&variances));
            } }
        }
    }
    assert_eq!(allocate_state_durations(7,&[2.0;3],&[1.0;3]).unwrap(),[3,2,2]);
    assert_eq!(allocate_state_durations(3,&[f64::MAX,f64::MAX/2.0],&[f64::from_bits(1);2]).unwrap(),[2,1]);
    assert!(allocate_state_durations(4,&[1.0;5],&[1.0;5]).is_err());
    assert!(allocate_state_durations(5,&[1.0;5],&[0.0;5]).is_err());
    let means = [4.585366,8.5,11.15854,7.47561,3.865854];
    let variances = [9.350274,45.38195,44.87511,14.30819,22.61396];
    assert_eq!(allocate_state_durations(400,&means,&variances).unwrap(),[30,129,131,46,64]);
}
#[test]
fn singing_surplus_preserves_all_protected_transition_counts() {
    let mut spectral = vec![0.0;5*3*41];
    for state in 0..5 { for index in 0..24 { spectral[state*123+41+16+index] = (state+1) as f64; } }
    let means = [4.0,9.0,11.0,8.0,4.0];
    let variances = [12.0,40.0,45.0,15.0,25.0];
    let reference = allocate_state_durations(36,&means,&variances).unwrap();
    let (long,evidence) = allocate_singing_state_durations(400,&means,&variances,Some("zh:ian"),Some(15),&spectral,"singing",0.005).unwrap();
    assert_eq!(evidence["sustain_state_index_0based"],2);
    assert_eq!(long.iter().sum::<usize>(),400);
    for state in [0,1,3,4] { assert_eq!(long[state],reference[state]); }
    let (consonant,_) = allocate_singing_state_durations(400,&means,&variances,Some("zh:s"),Some(58),&spectral,"singing",0.005).unwrap();
    assert_eq!(consonant,allocate_state_durations(400,&means,&variances).unwrap());
}
#[test]
fn banded_normal_equations_match_independent_original_receipt() {
    let fixture = acoustic_fixture();
    let case = &fixture["normal_case"];
    let means = flattened(&case["means"]);
    let variances = flattened(&case["variances"]);
    let (trajectory,evidence) = generate_independent_trajectory(&means,&variances,5,1,true).unwrap();
    close(&trajectory,&flattened(&case["solution"]),2e-14,2e-14);
    assert!(evidence["relative_normal_equation_residual"].as_f64().unwrap() < 1e-12);
    let normal = case["normal"].as_array().unwrap();
    let product: Vec<f64> = normal.iter().map(|row| flattened(row).iter().zip(&trajectory).map(|(a,b)| a*b).sum()).collect();
    close(&product,&flattened(&case["rhs"]),0.0,2e-13);
    let mut changed = means.clone();
    let mut changed_variances = variances.clone();
    for frame in [0,4] { for window in 1..3 {
        changed[frame*3+window] = 1e12; changed_variances[frame*3+window] = 1e-12;
    } }
    let (endpoints,_) = generate_independent_trajectory(&changed,&changed_variances,5,1,true).unwrap();
    assert_eq!(endpoints,trajectory);
}
#[test]
fn original_lsf_float32_three_forward_passes_are_array_exact() {
    let fixture = acoustic_fixture();
    let case = &fixture["lsf_case"];
    let observations = flattened(&case["observations"]);
    let (result,evidence) = original_lsf_correction(&observations,1).unwrap();
    assert_eq!(result,flattened(&case["corrected_lsf"]));
    assert_eq!(evidence["passes"],3);
    assert!(original_lsf_correction(&[0.0;40],1).is_err());
    let mut means = vec![0.0;5*3*41];
    for frame in 0..5 {
        means[frame*123..frame*123+40].copy_from_slice(&observations);
        means[frame*123+40] = 2.3;
    }
    let (spectral,_) = spectral_mlpg(&means,&vec![0.1;means.len()],5,"original").unwrap();
    for frame in 0..5 {
        assert_eq!(&spectral[frame*25..frame*25+24],result.as_slice());
        assert!((spectral[frame*25+24]-2.3).abs() < 2e-14);
    }
    let (legacy,_) = spectral_mlpg(&means,&vec![0.1;means.len()],5,"joint-reconstruction").unwrap();
    assert!(legacy.iter().zip(&spectral).any(|(a,b)| (a-b).abs() > 0.01));
}
#[test]
fn compact_pitch_discards_unvoiced_statistics_and_retains_uv_placeholders() {
    let fixture = acoustic_fixture();
    let means = flattened(&fixture["normal_case"]["means"]);
    let variances = flattened(&fixture["normal_case"]["variances"]);
    let mask: Vec<bool> = fixture["pitch_case"]["mask"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
    let mut triplets = Vec::new();
    for (&mean,&variance) in means.iter().zip(&variances) { triplets.extend([1.0,mean,variance]); }
    let (result,_) = pitch_mlpg(&triplets,&mask,"original").unwrap();
    let selected: Vec<f64> = result.iter().zip(&mask).filter_map(|(&value,&active)| active.then_some(value)).collect();
    close(&selected,&flattened(&fixture["pitch_case"]["voiced_solution"]),0.0,2e-14);
    for frame in 0..5 { if !mask[frame] {
        assert_eq!(result[frame],0.0);
        for window in 0..3 { triplets[frame*9+window*3+1] = 1e10; triplets[frame*9+window*3+2] = 0.0; }
    } }
    assert_eq!(pitch_mlpg(&triplets,&mask,"original").unwrap().0,result);
    assert_eq!(pitch_mlpg(&triplets,&[false;5],"original").unwrap().0,[0.0;5]);
}
#[test]
fn original_voicing_matches_all_immutable_x87_receipt_masks() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/original-voicing-synthetic-fixture.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let counts: Vec<usize> = case["state_frames"].as_array().unwrap().iter().map(|n| n.as_u64().unwrap() as usize).collect();
        let probabilities = flattened(&case["probabilities"]);
        let ids: Vec<i64> = case["phone_ids"].as_array().unwrap().iter().map(|n| n.as_i64().unwrap()).collect();
        let languages: Option<Vec<i64>> = case.get("languages").map(|v| v.as_array().unwrap().iter().map(|n| n.as_i64().unwrap()).collect());
        let expected: Vec<bool> = case["original_mask"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap_or_else(|| v.as_i64().unwrap() != 0)).collect();
        assert_eq!(infer_voicing(&counts,&probabilities,&ids,languages.as_deref()).unwrap().0,expected,"{case}");
    }
}
fn map(render: &[f64], authored: &[f64]) -> TimingPlan {
    TimingPlan { segments:vec![],total_frames:0,render_seconds:render.to_vec(),authored_seconds:authored.to_vec() }
}
#[test]
fn authored_uv_islands_and_collapsed_gaps_never_refit_voiced_anchors() {
    let track = json!({"frame_times_seconds":[0,0.1,0.2,0.3,0.4],
        "f0_hz":[200,0,400,500,0],"uv":[false,true,false,false,true]});
    let (f0,voiced,_) = sample_authored_control(&track,&[0.05,0.1,0.15,0.2,0.25,0.35],&map(&[0.0,0.4],&[0.0,0.4]),20000,30.0).unwrap();
    assert_eq!(voiced,[true,false,false,true,true,true]);
    close(&f0,&[200.0,0.0,0.0,400.0,450.0,500.0],0.0,1e-12);
    let track = json!({"frame_times_seconds":[0,0.1,0.102,0.3],"f0_hz":[200,0,400,0],"uv":[false,true,false,true]});
    let (f0,voiced,_) = sample_authored_control(&track,&[0.099,0.1,0.101],&map(&[0.0,0.1,0.1,0.298],&[0.0,0.1,0.102,0.3]),20000,30.0).unwrap();
    assert_eq!(voiced,[true;3]); assert_eq!(f0,[200.0,400.0,400.0]);
    assert!(sample_authored_control(&json!({"frame_times_seconds":[0,0.1],"f0_hz":[2000.001,0],"uv":[false,true]}),
        &[0.05],&map(&[0.0,0.1],&[0.0,0.1]),20000,30.0).is_err());
    assert_eq!(control::canonical_json(&json!({"utf8":"童声","small":1e-7,"large":1e16,"float":1.0,"neg":-0.0})).unwrap(),
        r#"{"float":1.0,"large":1e+16,"neg":-0.0,"small":1e-07,"utf8":"\u7ae5\u58f0"}"#);
}
#[test]
fn authored_curves_suppress_automatic_glides_and_manual_track_is_exact() {
    let notes = json!([{"start_seconds":0,"duration_seconds":0.5,"midi_note":60},
        {"start_seconds":0.5,"duration_seconds":0.5,"midi_note":64,"portamento_ms":100}]);
    let times = [0.0,0.49,0.5,0.55,0.6,0.9,1.0];
    let (f0,evidence) = plan_note_pitch(&notes,&times,None,true,80.0,None).unwrap();
    assert_eq!(evidence["automatic_transitions"],json!([]));
    assert_eq!(evidence["explicit_transitions"].as_array().unwrap().len(),1);
    assert!((1200.0*(f0[3]/440.0).log2()+6900.0-6200.0).abs() < 1e-9);
    assert_eq!(f0[6],0.0);
    let native = [true,true,false,true,true,true,true];
    let manual = [251.123,290.123,330.123,400.123,500.123,600.123,700.123];
    let (f0,evidence) = plan_note_pitch(&notes,&times,Some(&native),true,80.0,Some(&manual)).unwrap();
    assert_eq!(evidence["automatic_transitions"],json!([]));
    for frame in 0..times.len() { assert_eq!(f0[frame],if native[frame] && times[frame] < 1.0 { manual[frame] } else { 0.0 }); }
}
fn continuous_score(seconds: f64) -> Value {
    json!({"segments":[{"duration_seconds":seconds}],"continuity":{"method":"continuous-runs-v1"},
        "phoneme_timing":{"method":"vowel-anchor-v1","notes":[{"note_id":"long",
            "note_onset_seconds":0,"note_end_seconds":seconds,"first_segment_index":0,
            "nucleus_segment_index":0,"end_segment_index":1,"prefix_seconds":0,"tail_trim_seconds":0}]}})
}
#[test]
fn neutral_trace_and_120_second_continuity_admission_are_strict() {
    let score = continuous_score(120.0);
    assert!(validate_score_phoneme_timing(&score).unwrap().is_some());
    assert!(validate_score_continuity(&score).unwrap().is_some());
    let segments = score["segments"].as_array().unwrap();
    let plan = plan_score_timing(segments,16000,80).unwrap();
    assert_eq!(plan.total_frames,24000);
    assert_eq!(plan.segments.len(),1);
    assert!(validate_score_continuity(&continuous_score(120.000000001)).is_err());
    assert!(validate_score_continuity(&json!({"segments":[{"duration_seconds":120}],"continuity":{"method":"continuous-runs-v1"}})).is_err());
    let mut bad = score.clone();
    bad["phoneme_timing"]["notes"][0]["note_onset_seconds"] = json!(0.1);
    assert!(validate_score_phoneme_timing(&bad).is_err());
    let mut bad = score.clone(); bad["continuity"]["extra"] = json!(true);
    assert!(validate_score_continuity(&bad).is_err());
    assert_eq!(validate_score_phoneme_timing(&json!({})).unwrap(),None);
    assert_eq!(validate_score_continuity(&json!({})).unwrap(),None);
}

#[test]
fn realized_timing_reports_only_actual_nuclei_on_the_native_frame_clock() {
    let score: Value = serde_json::from_str(include_str!("fixtures/constrained-phoneme-timing.json")).unwrap();
    let trace = validate_score_phoneme_timing(&score).unwrap().unwrap();
    let plan = plan_score_timing(score["segments"].as_array().unwrap(), 16000, 80).unwrap();
    let report = timing::realized_phoneme_timing(Some(&trace), &plan, 16000, 80).unwrap();
    let notes = report["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 2);
    let (nasal, syllable) = (&notes[0], &notes[1]);
    assert_eq!(nasal["note_id"], "nasal");
    assert_eq!(nasal["actual_first_sample"], 0);
    assert_eq!(nasal["actual_end_sample"], 800);
    for field in ["actual_nucleus_sample", "actual_nucleus_seconds", "nucleus_error_seconds"] {
        assert_eq!(nasal.get(field), Some(&Value::Null), "{field}");
    }
    assert_eq!(syllable["note_id"], "sa");
    assert_eq!(syllable["actual_first_sample"], 800);
    assert_eq!(syllable["actual_nucleus_sample"], 1600);
    assert_eq!(syllable["actual_nucleus_seconds"], 0.1);
    assert_eq!(syllable["actual_end_sample"], 3200);
    // The 36 ms planned delay and 1 ms of frame rounding both fall after the 63 ms MIDI onset.
    let error = syllable["nucleus_error_seconds"].as_f64().unwrap();
    assert!((error - 0.037).abs() < 1e-12);
    assert_eq!(report["maximum_nucleus_error_seconds"], syllable["nucleus_error_seconds"]);

    let mut nasal_only = score;
    nasal_only["segments"].as_array_mut().unwrap().truncate(1);
    nasal_only["phoneme_timing"]["notes"].as_array_mut().unwrap().truncate(1);
    let trace = validate_score_phoneme_timing(&nasal_only).unwrap().unwrap();
    let plan = plan_score_timing(nasal_only["segments"].as_array().unwrap(), 16000, 80).unwrap();
    let report = timing::realized_phoneme_timing(Some(&trace), &plan, 16000, 80).unwrap();
    assert_eq!(report.get("maximum_nucleus_error_seconds"), Some(&Value::Null));
    assert_eq!(&report["notes"][0], nasal);
}

#[test]
fn timing_admission_rejects_false_nuclei_and_inconsistent_delays() {
    let score: Value = serde_json::from_str(include_str!("fixtures/constrained-phoneme-timing.json")).unwrap();
    for (note, field, error) in [(0, "nucleus_delay_seconds", 0.005), (0, "prefix_seconds", 0.005),
        (1, "nucleus_delay_seconds", 0.035), (1, "prefix_seconds", 0.010)] {
        let mut bad = score.clone();
        bad["phoneme_timing"]["notes"][note][field] = json!(error);
        assert!(validate_score_phoneme_timing(&bad).is_err(), "note {note}: {field}");
    }

    // Keep the boundaries internally consistent so rejection depends on the model phone's role.
    let mut hidden_vowel = score.clone();
    let note = &mut hidden_vowel["phoneme_timing"]["notes"][1];
    note["nucleus_segment_index"] = Value::Null;
    note["note_onset_seconds"] = json!(0.052);
    note["prefix_seconds"] = json!(0.0);
    note.as_object_mut().unwrap().remove("nucleus_delay_seconds");
    assert!(validate_score_phoneme_timing(&hidden_vowel).is_err());

    let mut voiced_consonant = score.clone();
    voiced_consonant["segments"][2]["context"] = score["segments"][0]["context"].clone();
    assert!(validate_score_phoneme_timing(&voiced_consonant).is_err());
}

fn nucleus_segment(phone: i64, continuation: Option<usize>) -> Value {
    let mut segment = json!({"duration_seconds":0.5,"voiced":true,
        "context":{"features":{"C_Phone":phone.to_string(),"C_Language":"3","C_Stress":"0"},
            "answers":{},"anchor_questions":[],"assumptions":[]}});
    if let Some(anchor) = continuation { segment["continuation_of"] = json!(anchor); }
    segment
}
#[test]
fn explicit_continuation_keeps_original_anchor_and_rejects_barriers() {
    let segments = vec![nucleus_segment(174,None),nucleus_segment(174,Some(0)),nucleus_segment(174,Some(0))];
    assert_eq!(continuation_source_indices(&segments).unwrap(),[0,0,0]);
    validate_score_continuations(&json!({"segments":segments})).unwrap();
    let mut alias = segments.clone(); alias[2]["continuation_of"] = json!(1);
    assert!(continuation_source_indices(&alias).is_err());
    let mut disconnected = segments.clone(); disconnected[1].as_object_mut().unwrap().remove("continuation_of");
    assert!(continuation_source_indices(&disconnected).is_err());
    let mut future = segments.clone(); future[1]["continuation_of"] = json!(2);
    assert!(continuation_source_indices(&future).is_err());
    for invalid in [Value::Null,json!(false),json!(-1),json!(0.0)] {
        let mut bad = segments.clone(); bad[1]["continuation_of"] = invalid;
        assert!(continuation_source_indices(&bad).is_err());
    }
    let mut gap = segments.clone(); gap[0]["gap_after_seconds"] = json!(0.001);
    assert!(continuation_source_indices(&gap).is_err()); // even a collapsed gap is a real authored barrier
    let mut uv = segments.clone(); uv[1]["voiced"] = json!(false);
    assert!(continuation_source_indices(&uv).is_err());
    let mut other = segments.clone(); other[1]["context"]["features"]["C_Phone"] = json!("175");
    assert!(validate_score_continuations(&json!({"segments":other})).is_err());
    let mut consonant = segments.clone();
    for segment in &mut consonant { segment["context"]["features"]["C_Phone"] = json!("204"); }
    assert!(validate_score_continuations(&json!({"segments":consonant})).is_err());
    let mut context = segments.clone(); context[1]["context"]["features"]["C_Stress"] = json!("1");
    assert!(validate_score_continuations(&json!({"segments":context})).is_err());
    let ordinary = [nucleus_segment(174,None),nucleus_segment(174,None)];
    assert_eq!(continuation_source_indices(&ordinary).unwrap(),[0,1]); // never infer a link from equal phone IDs
}
#[test]
fn one_source_state_sequence_slices_without_replaying_onset_or_losing_frames() {
    let states = [4,9,375,8,4];
    let budgets = [100,100,200];
    let mut sliced = Vec::new();
    let mut offset = 0;
    let mut summed = [0;5];
    for frames in budgets {
        let occupancy = source::state_occupancy(&states,offset,frames).unwrap();
        assert_eq!(occupancy.iter().sum::<usize>(),frames);
        for (state,&count) in occupancy.iter().enumerate() {
            summed[state] += count;
            sliced.extend(std::iter::repeat(state).take(count));
        }
        offset += frames;
    }
    assert_eq!(source::state_occupancy(&states,100,100).unwrap(),[0,0,100,0,0]);
    assert_eq!(summed,states);
    let original: Vec<usize> = states.iter().enumerate().flat_map(|(state,&count)| std::iter::repeat(state).take(count)).collect();
    assert_eq!(sliced,original);
    assert!(source::state_occupancy(&states,399,2).is_err());
}
#[test]
fn hierarchy_admission_wrapper_preserves_full_model_validator_rejection() {
    validate_score_hierarchy(&json!({"segments":[nucleus_segment(174,None)]})).unwrap();
    assert!(validate_score_hierarchy(&json!({"segments":[nucleus_segment(174,None)],
        "original_hierarchy":{"schema_version":"ogd-explicit-original-hierarchy/1",
            "nodes":[{"phone":175,"left_level":0,"right_level":0}],
            "segment_node_indices":[0],"provenance":"synthetic mismatch"}})).is_err());
}

#[test]
fn authored_pitch_preserves_admitted_anchors_and_rejects_out_of_range_values() {
    let track=json!({"frame_times_seconds":[0.0,0.1,0.2,0.3,0.4],
        "f0_hz":[35.0,2000.0,0.0,440.123456,0.0],"uv":[false,false,true,false,true]});
    let times=[0.0,0.05,0.1,0.15,0.2,0.25,0.3,0.35];
    let timing=map(&[0.0,0.4],&[0.0,0.4]);
    let (f0,voiced,report)=sample_authored_control(&track,&times,&timing,20000,30.0).unwrap();
    close(&f0,&[35.0,1017.5,2000.0,2000.0,0.0,0.0,440.123456,440.123456],0.0,1e-10);
    assert_eq!(voiced,[true,true,true,true,false,false,true,true]);
    assert_eq!(report["native_control_f0_range_hz"],json!([35.0,2000.0]));
    for value in [34.999,2000.001] {
        let mut bad=track.clone();bad["f0_hz"][0]=json!(value);
        assert!(sample_authored_control(&bad,&times,&timing,20000,30.0).is_err());
    }
}

#[test]
fn composed_note_pitch_rejects_overshoot_without_clamping() {
    let times=[0.0025,0.1,0.25,0.49];
    let mut notes=json!([{"start_seconds":0.0,"duration_seconds":0.5,"midi_note":95}]);
    let expected=440.0_f64*2.0_f64.powf((95.0-69.0)/12.0);
    let pitch=plan_note_pitch(&notes,&times,None,true,80.0,None).unwrap();
    close(&pitch.0,&[expected;4],0.0,1e-8);
    notes[0]["detune_cents"]=json!(100.0);
    assert!(plan_note_pitch(&notes,&times,None,true,80.0,None).is_err());
}
