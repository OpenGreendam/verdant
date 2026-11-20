//! Recovered state-probability UV, including asymmetric x87 stores.
use anyhow::{ensure, Context, Result};
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json,Value};

fn power_of_two(exponent: i64) -> BigRational {
    if exponent >= 0 { BigRational::from_integer(BigInt::one() << exponent as usize) }
    else { BigRational::new(BigInt::one(),BigInt::one() << (-exponent) as usize) }
}
fn extended(value: BigRational) -> BigRational {
    if value.is_zero() { return value; }
    let negative = value.numer().sign() == Sign::Minus;
    let value = value.abs();
    let mut exponent = value.numer().bits() as i64-value.denom().bits() as i64;
    if value < power_of_two(exponent) { exponent -= 1; }
    let unit = power_of_two(exponent-63);
    let scaled = &value / &unit;
    let mut whole = scaled.numer()/scaled.denom();
    let remainder = scaled.numer()%scaled.denom();
    let doubled = remainder*2;
    if doubled > *scaled.denom() || (doubled == *scaled.denom() && (&whole%2) != BigInt::zero()) {
        whole += 1;
    }
    let rounded = BigRational::from_integer(whole)*unit;
    if negative { -rounded } else { rounded }
}
fn audited_phone(id: i64) -> bool { matches!(id,1|2|3..=63|174..=235) }
fn class_one(id: i64) -> bool { (3..=42).contains(&id) || matches!(id,52|53|54|57) }

pub fn infer_voicing(counts: &[usize], probabilities: &[f64], phone_ids: &[i64],
    languages: Option<&[i64]>) -> Result<(Vec<bool>,Value)>
{
    infer_voicing_with_compact(counts,probabilities,phone_ids,languages,false)
}
pub(super) fn infer_voicing_with_compact(counts: &[usize], probabilities: &[f64], phone_ids: &[i64],
    languages: Option<&[i64]>, compact: bool) -> Result<(Vec<bool>,Value)>
{
    ensure!(!counts.is_empty() && counts.len() == probabilities.len() && counts.len() == phone_ids.len(),
        "voicing requires equal nonempty one-dimensional state counts, probabilities and phone IDs");
    if compact {
        ensure!(counts.iter().any(|&n| n > 0), "compact verdant-voice occupancy must contain at least one frame");
    } else {
        ensure!(counts.iter().all(|&n| n > 0), "state frame counts must be positive integers");
    }
    ensure!(phone_ids.iter().all(|&id| audited_phone(id)), "voicing requires audited Mandarin/English original phone IDs");
    ensure!(probabilities.iter().all(|&p| p.is_finite() && (0.0..=1.0).contains(&p)),
        "static pitch probabilities must be finite values in [0,1]");
    if let Some(languages) = languages {
        ensure!(languages.len() == counts.len(), "state languages must be aligned integer original language codes");
    }
    let total = counts.iter().try_fold(0usize,|a,&b| a.checked_add(b)).context("voicing frame count overflow")?;
    let mut mask = vec![false;total];
    let mut continuity = vec![false;total];
    let mut offset = 0;
    for (index,(&count,&probability)) in counts.iter().zip(probabilities).enumerate() {
        let previous = if index == 0 { 0.0 } else { probabilities[index-1] };
        let following = probabilities.get(index+1).copied().unwrap_or(0.0);
        let language = languages.map(|v| v[index]).unwrap_or(if phone_ids[index] >= 174 { 3 } else { 1 });
        continuity[offset..offset+count].fill(language == 1 && class_one(phone_ids[index]));
        let whole = probability > 0.5 || (previous >= 0.5 && following >= 0.5 && probability >= 0.25);
        if whole { mask[offset..offset+count].fill(true); }
        else {
            let current = BigRational::from_float(probability).context("nonfinite voicing probability")?;
            let left_sum = extended(BigRational::from_float(previous).context("nonfinite previous probability")?+&current);
            let right_sum = BigRational::from_float(probability+following).context("nonfinite right probability sum")?;
            let one = BigRational::from_integer(BigInt::one());
            let count_rational = BigRational::from_integer(BigInt::from(count));
            let left_threshold = extended(extended(left_sum-&one)*&count_rational);
            let right_threshold = extended(extended(right_sum-&one)*count_rational);
            // For an integer k, k < rational r iff k < ceil(r). Round
            // once in exact arithmetic, rather than allocating per frame.
            let left_edge = left_threshold.ceil().to_integer().to_i128().context("left UV threshold overflow")?;
            let right_edge = right_threshold.ceil().to_integer().to_i128().context("right UV threshold overflow")?;
            for frame in 0..count {
                mask[offset+frame] = (frame as i128+1) < left_edge || ((count-frame) as i128) < right_edge;
            }
        }
        offset += count;
    }
    let before = mask.iter().filter(|&&v| v).count();
    let mut seen = false;
    for (voiced,&eligible) in mask.iter_mut().zip(&continuity) {
        if !eligible { seen = false; }
        else if seen { *voiced = true; }
        else if *voiced { seen = true; }
    }
    let voiced = mask.iter().filter(|&&v| v).count();
    let continuity_ids: Vec<i64> = (3..=42).chain([52,53,54,57]).collect();
    Ok((mask,json!({"mode":"original-model","scope":"recovered per-state probability rules; opt-in, not perceptually certified",
        "original_fragment_start_rva":"0x5a2ce","original_fragment_stop_rva":"0x5a803",
        "static_pitch_probability_float64_index":0,"pitch_record_float64_count":9,"pitch_stream_triplet_stride":3,
        "neighbor_rule":"previous/next HMM state in this contiguous run; zero probability beyond run boundaries",
        "whole_state_rule":"p>0.5 or (previous>=0.5 and next>=0.5 and p>=0.25)",
        "partial_state_rule":"k+1<N*(previous+p-1) or N-k<N*(p+next-1), k is zero-based; strict comparisons",
        "continuity_rule":"once voiced within consecutive language1 and original first_class1 frames, remain voiced until either condition fails",
        "continuity_phone_ids":continuity_ids,
        "continuity_phone_group_evidence":"original table RVA0x117f90+(phoneID-3)*80; includes l52,m53,n54,r57",
        "language_policy":if languages.is_some() { "explicit aligned original state language codes" }
            else { "inferred from audited Mandarin/English inventory IDs (1/3)" },
        "numeric_policy":"x87 64-significand-bit rounding; right sum stored/reloaded as binary64; probe control word0x037f",
        "states":counts.len(),"frames":total,"voiced_frames":voiced,"continuity_added_voiced_frames":voiced-before})))
}
