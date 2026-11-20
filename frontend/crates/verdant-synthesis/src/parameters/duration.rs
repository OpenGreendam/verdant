//! Exact separable-convex duration allocation and declared singing policy.
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use anyhow::{ensure, Context, Result};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use serde_json::{json, Value};

#[derive(Eq, PartialEq)]
struct Marginal { cost: BigRational, state: usize }
impl Ord for Marginal {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is maximal: reverse both the exact cost and tie index.
        other.cost.cmp(&self.cost).then_with(|| other.state.cmp(&self.state))
    }
}
impl PartialOrd for Marginal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

pub fn allocate_state_durations(total: usize, means: &[f64], variances: &[f64]) -> Result<Vec<usize>> {
    ensure!(!means.is_empty() && means.len() == variances.len(),
        "state duration means and variances must be equal-length nonempty vectors");
    ensure!(means.iter().all(|v| v.is_finite() && *v > 0.0),
        "state duration means must be finite and positive");
    ensure!(variances.iter().all(|v| v.is_finite() && *v > 0.0),
        "state duration variances must be finite and positive");
    ensure!(total >= means.len(), "infeasible frame budget: every state needs at least one frame");
    let mut counts = vec![1; means.len()];
    let mut heap = BinaryHeap::with_capacity(means.len());
    let mut increments = Vec::with_capacity(means.len());
    for (state, (&mean, &variance)) in means.iter().zip(variances).enumerate() {
        let mean = BigRational::from_float(mean).context("nonfinite state mean")?;
        let variance = BigRational::from_float(variance).context("nonfinite state variance")?;
        let cost = (BigRational::new(BigInt::from(3), BigInt::from(2))-mean) / &variance;
        increments.push(variance.recip());
        heap.push(Marginal { cost, state });
    }
    // Each next marginal is previous + 1/variance, exactly. No overflowing
    // binary64 division can introduce false ties, including subnormals.
    for _ in means.len()..total {
        let mut entry = heap.pop().context("empty duration allocation heap")?;
        counts[entry.state] += 1;
        entry.cost += &increments[entry.state];
        heap.push(entry);
    }
    Ok(counts)
}

pub(super) fn allocation_metadata() -> Value {
    json!({"algorithm":"constrained_gaussian_maximum_likelihood_integer_state_duration_allocation",
        "objective":"minimize sum((d_i - mean_i)**2 / (2*variance_i)) subject to integer d_i >= 1 and sum(d_i) == requested total frames",
        "method":"exact separable-convex marginal-cost allocation on a binary heap; equal marginal costs resolve deterministically to the lowest state index",
        "reference":"Saino, Zen, Nankaku, Lee, Tokuda, 'HMM-based singing voice synthesis system', Interspeech 2006, section 2.3; https://www.isca-archive.org/interspeech_2006/saino06_interspeech.pdf",
        "unit_assumption":"original state-duration statistics interpreted as frame units; original phone/state duration coupling and units remain unverified"})
}

fn candidates(key: &str) -> (&'static [usize], &'static str) {
    match key {
        "zh:ian"|"zh:iang"|"zh:iao"|"zh:iong"|"zh:iou"|"zh:uai"|"zh:uan"|"zh:uang"|"zh:uei"|"zh:uen"|"zh:ueng"|"zh:van" =>
            (&[2], "medial-plus-ending rime: protect both sides; central state is a declared nucleus approximation"),
        "zh:ia"|"zh:ie"|"zh:io"|"zh:ua"|"zh:uo"|"zh:ve" =>
            (&[2,3], "medial-plus-vowel rime: prefer a later interior state after the initial glide"),
        "zh:ai"|"zh:ao"|"zh:ei"|"zh:ou"|"en:aw"|"en:ay"|"en:ey"|"en:ow"|"en:oy" =>
            (&[1,2], "closing diphthong: retain the late offglide instead of sustaining its terminal states"),
        "zh:an"|"zh:ang"|"zh:en"|"zh:eng"|"zh:in"|"zh:ing"|"zh:ong"|"zh:un"|"zh:vn" =>
            (&[1,2], "nasal rime: preserve the model-sized late tail instead of distributing surplus into nasal closure"),
        _ => (&[1,2,3], "vowel/syllabic nucleus: choose a spectrally steady interior state"),
    }
}
pub(super) fn is_nucleus(phone_id: i64) -> bool {
    // These are the role ranges in the pinned native inventory, not an
    // inference from labels, IPA or a selected HMM state number.
    (3..=42).contains(&phone_id) || (174..=200).contains(&phone_id)
}

pub fn allocate_singing_state_durations(total: usize, means: &[f64], variances: &[f64],
    native_key: Option<&str>, phone_id: Option<i64>, spectral_means: &[f64],
    mode: &str, frame_seconds: f64) -> Result<(Vec<usize>, Value)>
{
    ensure!(matches!(mode, "model"|"singing"), "articulation duration mode must be model or singing");
    ensure!(frame_seconds.is_finite() && frame_seconds > 0.0, "frame_seconds must be finite and positive");
    ensure!(means.len() == 5 && variances.len() == 5, "singing articulation requires the audited five-state model");
    ensure!(spectral_means.len() == 5*3*41 && spectral_means.iter().all(|v| v.is_finite()),
        "spectral_state_means must be finite [5,3,41]");
    let model_counts = allocate_state_durations(total, means, variances)?;
    let mut evidence = json!({"mode":mode,"native_key":native_key,"applied":false,
        "requested_total_frames":total,"model_counts":model_counts,"frame_seconds":frame_seconds,
        "counts":model_counts,"phone_timeline_changed":false,
        "status":"declared singing reconstruction policy; no recovered HMM state has a certified phonetic boundary label"});
    if mode == "model" {
        evidence["reason"] = json!("Unchanged Gaussian maximum-likelihood allocation");
        return Ok((model_counts,evidence));
    }
    let Some(key) = native_key.filter(|_| phone_id.is_some_and(is_nucleus)) else {
        evidence["reason"] = json!("Only identified native vowel/rime nuclei are eligible; consonant and unknown-phone timing remains model allocation");
        return Ok((model_counts,evidence));
    };
    // NumPy's length-five reduction has the same left-to-right order.
    let mean_sum: f64 = means.iter().sum();
    ensure!(mean_sum.is_finite(), "the sum of duration means must be finite");
    let rounded = (mean_sum+0.5).floor();
    let reference_frames = rounded.to_usize().context("model duration reference budget overflow")?.max(5);
    evidence["reference_total_frames"] = json!(reference_frames);
    evidence["reference_budget_policy"] = json!("nearest integer to sum of the selected model's duration means, minimum five; original mean units remain the recorded frame-unit interpretation");
    if total <= reference_frames {
        evidence["reason"] = json!("Requested vowel is no longer than the selected model duration; short-note allocation preserved exactly");
        return Ok((model_counts,evidence));
    }
    let reference = allocate_state_durations(reference_frames, means, variances)?;
    let (eligible, rationale) = candidates(key);
    let mut ranking = [0.0_f64;5];
    for (state, energy) in ranking.iter_mut().enumerate() {
        let offset = state*3*41+41+16;
        // Match NumPy's pairwise reduction for the contiguous length-24
        // vector, including the eight accumulators used by its scalar loop.
        let mut squares = [0.0;24];
        for (target, &value) in squares.iter_mut().zip(&spectral_means[offset..offset+24]) { *target = value*value; }
        *energy = pairwise_sum(&squares) / 24.0;
        ensure!(energy.is_finite(), "temporal-delta LSF energy must remain finite");
    }
    let chosen = *eligible.iter().min_by(|&&a,&&b| {
        ranking[a].total_cmp(&ranking[b]).then_with(|| a.abs_diff(2).cmp(&b.abs_diff(2))).then(a.cmp(&b))
    }).context("no admissible sustain state")?;
    let mut counts = reference.clone();
    let surplus = total-reference_frames;
    counts[chosen] += surplus;
    ensure!(counts.iter().sum::<usize>() == total && counts.iter().all(|&n| n >= 1),
        "articulation changed the requested frame budget");
    let protected: Vec<usize> = (0..5).filter(|&state| state != chosen).collect();
    for &state in &protected { ensure!(counts[state] == reference[state], "articulation stretched a protected transition state"); }
    let object = evidence.as_object_mut().context("invalid articulation evidence")?;
    for (name,value) in json!({"applied":true,
        "reason":"Preserve model-sized transitions; allocate long-note surplus to an interior sustain state",
        "reference_counts":reference,"counts":counts,"sustain_state_index_0based":chosen,
        "candidate_state_indices_0based":eligible,"candidate_policy":rationale,
        "ranking_policy":"Minimum mean squared selected temporal-delta LSF among semantic candidates; exact ties prefer center then earlier state",
        "mean_squared_temporal_delta_lsf_radians":ranking,
        "ranking_measurement_scope":"selected HMM delta means before MLPG/LSF correction; not measured generated-trajectory velocity or a certified vowel-boundary alignment",
        "protected_state_indices_0based":protected,"extra_sustain_frames":surplus,"protected_counts_unchanged":true,
        "model_boundary_state_frames":model_counts[0]+model_counts[4],
        "singing_boundary_state_frames":counts[0]+counts[4],
        "references":["https://www.isca-archive.org/interspeech_2007/saitou07_interspeech.pdf (preserve consonant-vowel boundary duration, extend vowel)",
            "https://www.isca-archive.org/interspeech_2006/saino06_interspeech.pdf (singing timing differs from speech; model-based state duration constraints)"]
    }).as_object().context("invalid articulation details")? { object.insert(name.clone(),value.clone()); }
    Ok((counts,evidence))
}

/// verdant-voice keeps the normal allocation verbatim whenever all five states fit.
/// Below that floor, retain the central observation first, then entry/exit
/// coverage in temporal order. These are representative model observations,
/// not recovered phonetic labels for HMM states.
pub(super) fn allocate_voice_state_durations(total: usize, means: &[f64], variances: &[f64],
    native_key: Option<&str>, phone_id: Option<i64>, spectral_means: &[f64],
    mode: &str, frame_seconds: f64) -> Result<(Vec<usize>, Value)>
{
    if total >= 5 {
        return allocate_singing_state_durations(total,means,variances,native_key,phone_id,spectral_means,mode,frame_seconds);
    }
    ensure!(total > 0, "compact verdant-voice phone requires at least one 5 ms frame");
    // Validate the unchanged selected statistics and options through the
    // normal five-frame path. Its counts are a reference, never extra audio.
    let (reference,_) = allocate_singing_state_durations(5,means,variances,native_key,phone_id,spectral_means,mode,frame_seconds)?;
    let representatives: &[usize] = match total {
        1 => &[2],
        2 => &[1,3],
        3 => &[0,2,4],
        4 => &[0,1,2,4],
        _ => unreachable!(),
    };
    let mut counts = vec![0;5];
    for &state in representatives { counts[state] = 1; }
    let evidence = json!({"mode":mode,"native_key":native_key,"applied":true,
        "compact_phone_states":true,"requested_total_frames":total,"counts":counts,
        "frame_seconds":frame_seconds,"phone_timeline_changed":false,
        "reference_total_frames":5,"reference_counts":reference,
        "representative_state_indices_0based":representatives,
        "reason":"verdant-voice one-to-four-frame phone: select monotonic representative observations without stretching the authored interval",
        "selection_policy":"one: central state 2; two: interior entry/exit states 1,3; three: boundaries 0,4 plus center 2; four: boundaries and center plus earlier interior state 1",
        "zero_occupancy_meaning":"unselected model states intentionally occupy zero frames; no zero-frame audio is emitted",
        "status":"declared compact reconstruction policy; model state numbers are not certified phonetic nucleus/transition labels"});
    Ok((counts,evidence))
}

// NumPy pairwise_sum's contiguous double reduction (PW_BLOCKSIZE=128).
// The grouping matters only for exact sustain-state energy ties.
pub(super) fn pairwise_sum(values: &[f64]) -> f64 {
    if values.len() < 8 {
        return values.iter().fold(-0.0, |sum,&x| sum+x);
    }
    if values.len() <= 128 {
        let mut accumulators = [0.0;8];
        accumulators.copy_from_slice(&values[..8]);
        let mut index = 8;
        while index+8 <= values.len() {
            for lane in 0..8 { accumulators[lane] += values[index+lane]; }
            index += 8;
        }
        let mut result = ((accumulators[0]+accumulators[1])+(accumulators[2]+accumulators[3]))
            + ((accumulators[4]+accumulators[5])+(accumulators[6]+accumulators[7]));
        for &value in &values[index..] { result += value; }
        return result;
    }
    let half = (values.len()/2)/8*8;
    pairwise_sum(&values[..half])+pairwise_sum(&values[half..])
}
