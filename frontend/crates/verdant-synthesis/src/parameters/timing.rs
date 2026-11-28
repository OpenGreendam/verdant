//! Absolute acoustic clock and inventory-neutral scheduling admission.
//! Decimal score intervals accumulate before one shared boundary quantizer.
use std::collections::BTreeSet;
use anyhow::{bail, ensure, Context, Result};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::Serialize;
use serde_json::{json, Value};

const TIE_TOLERANCE_DENOMINATOR: i64 = 1_000_000_000;
pub const CONTINUITY_MAX_SECONDS: f64 = 120.0;
pub const CONTINUITY_MAX_SEGMENTS: usize = 2048;
pub const CONTINUITY_MAX_CONTROL_POINTS: usize = 32768;
/// The legacy per-phone frame floor: one frame for every HMM state, so five
/// frames (25 ms at the native 5 ms hop) for each native phone.
pub const STATE_FLOOR_FRAMES: usize = 5;
/// The compact verdant-voice per-phone frame floor: exactly one acoustic frame
/// (5 ms) for a whole native phone, with the five-state model collapsed onto
/// the frames that are actually available.
pub const COMPACT_FLOOR_FRAMES: usize = 1;

#[derive(Clone, Debug, Serialize)]
pub struct SegmentTiming {
    pub requested_start_seconds: f64,
    pub requested_end_seconds: f64,
    pub requested_gap_end_seconds: f64,
    pub start_frame: usize,
    pub end_frame: usize,
    pub gap_end_frame: usize,
}
impl SegmentTiming {
    pub fn frames(&self) -> usize { self.end_frame - self.start_frame }
    pub fn gap_frames(&self) -> usize { self.gap_end_frame - self.end_frame }
}
#[derive(Clone, Debug)]
pub struct TimingPlan {
    pub segments: Vec<SegmentTiming>,
    pub total_frames: usize,
    pub render_seconds: Vec<f64>,
    pub authored_seconds: Vec<f64>,
}
impl TimingPlan {
    pub(super) fn report(&self, minimum_segment_frames: usize) -> Value {
        let maximum_error = self.render_seconds.iter().zip(&self.authored_seconds)
            .map(|(a,b)| (a-b).abs()).fold(0.0_f64, f64::max);
        json!({
            "policy":"absolute cumulative boundaries; half-frame ties toward the later frame",
            "minimum_segment_frames":minimum_segment_frames,
            "half_frame_tie_tolerance_frames":1e-9,
            "requested_seconds":self.authored_seconds.last(),
            "actual_seconds":self.render_seconds.last(),
            "total_frames":self.total_frames,
            "maximum_boundary_error_seconds":maximum_error,
            "time_map_render_seconds":self.render_seconds,
            "time_map_authored_seconds":self.authored_seconds,
            "mapping_policy":"piecewise linear within intervals; duplicate render anchors are right-continuous zero-frame gaps",
            "segments":self.segments,
        })
    }
}

pub(super) fn finite(value: &Value, name: &str) -> Result<f64> {
    let number = value.as_f64().with_context(|| format!("{name} must be a finite number"))?;
    ensure!(number.is_finite(), "{name} must be a finite number");
    Ok(number)
}
pub(super) fn field(value: &Value, name: &str, default: Option<f64>) -> Result<f64> {
    match value.get(name) {
        Some(v) => finite(v, name),
        None => default.with_context(|| format!("{name} is required")),
    }
}

// Decimal(str(float(value))) is the Python score-clock policy. Parse the
// shortest decimal representation, retaining its exact value during the sum.
fn decimal_seconds(number: f64) -> Result<BigRational> {
    ensure!(number.is_finite() && number >= 0.0, "invalid score interval");
    let text = number.to_string();
    let (mantissa, exponent) = if let Some(index) = text.find(['e','E']) {
        (&text[..index], text[index+1..].parse::<i32>()?)
    } else { (text.as_str(), 0) };
    let fraction = mantissa.find('.').map_or(0, |p| mantissa.len()-p-1) as i32;
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();
    let numerator = BigInt::parse_bytes(digits.as_bytes(), 10).context("invalid decimal interval")?;
    let power = exponent - fraction;
    Ok(if power >= 0 {
        BigRational::from_integer(numerator * BigInt::from(10).pow(power as u32))
    } else {
        BigRational::new(numerator, BigInt::from(10).pow((-power) as u32))
    })
}
fn later_integer(value: &BigRational) -> BigInt {
    // All clock coordinates are nonnegative.
    let twice = value.numer() * 2;
    (twice + value.denom()) / (value.denom() * 2)
}
fn frame_at(time: &BigRational, scale: &BigRational) -> Result<usize> {
    let mut coordinate = time * scale;
    let half = BigRational::new(later_integer(&(&coordinate + &coordinate)), BigInt::from(2));
    let tolerance = BigRational::new(BigInt::one(), BigInt::from(TIE_TOLERANCE_DENOMINATOR));
    if (&coordinate - &half).abs() <= tolerance { coordinate = half; }
    later_integer(&coordinate).to_usize().context("acoustic frame coordinate overflow")
}
pub fn plan_score_timing(segments: &[Value], sample_rate: u32, hop: usize) -> Result<TimingPlan> {
    plan_score_timing_with_floor(segments, sample_rate, hop, STATE_FLOOR_FRAMES)
}
/// One shared absolute clock with an explicit per-segment frame floor.
/// `minimum_frames` is five for the legacy five-state allocation and one for
/// the compact verdant-voice phone clock. The floor is never met by extending or
/// stretching an authored duration.
pub(super) fn plan_score_timing_with_floor(segments: &[Value], sample_rate: u32, hop: usize,
    minimum_frames: usize) -> Result<TimingPlan> {
    ensure!(sample_rate > 0 && hop > 0, "sample_rate and hop must be positive integers");
    ensure!(matches!(minimum_frames, COMPACT_FLOOR_FRAMES | STATE_FLOOR_FRAMES), "unsupported phone frame floor");
    ensure!(!segments.is_empty(), "score has no segments");
    let scale = BigRational::new(BigInt::from(sample_rate), BigInt::from(hop));
    let mut cursor = BigRational::zero();
    let mut rendered = vec![0.0];
    let mut authored = vec![0.0];
    let mut plans = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        ensure!(segment.is_object(), "segment {index} must be an object");
        let duration = field(segment, "duration_seconds", Some(0.8))?;
        let gap = field(segment, "gap_after_seconds", Some(0.0))?;
        ensure!(duration > 0.0 && gap >= 0.0, "segment {index} requires positive duration and nonnegative gap");
        let end = &cursor + decimal_seconds(duration)?;
        let after = &end + decimal_seconds(gap)?;
        let first = frame_at(&cursor, &scale)?;
        let last = frame_at(&end, &scale)?;
        let gap_end = frame_at(&after, &scale)?;
        let floor_note = if minimum_frames == STATE_FLOOR_FRAMES {
            "five states require at least five frames"
        } else {
            "the compact phone clock requires at least one frame"
        };
        ensure!(last >= first && last-first >= minimum_frames,
            "segment {index} has only {} frames on the absolute timeline; {floor_note}; duration is not silently extended", last.saturating_sub(first));
        let start_seconds = cursor.to_f64().context("score start overflow")?;
        let end_seconds = end.to_f64().context("score end overflow")?;
        let after_seconds = after.to_f64().context("score gap overflow")?;
        plans.push(SegmentTiming { requested_start_seconds:start_seconds,
            requested_end_seconds:end_seconds, requested_gap_end_seconds:after_seconds,
            start_frame:first, end_frame:last, gap_end_frame:gap_end });
        rendered.push(last as f64 * hop as f64 / sample_rate as f64);
        authored.push(end_seconds);
        if gap != 0.0 {
            rendered.push(gap_end as f64 * hop as f64 / sample_rate as f64);
            authored.push(after_seconds);
        }
        cursor = after;
    }
    Ok(TimingPlan { total_frames:frame_at(&cursor, &scale)?, segments:plans,
        render_seconds:rendered, authored_seconds:authored })
}

// Like math.fsum, sum the supplied binary64 inputs without sequential-sum
// drift. This validation clock deliberately differs from the decimal planner.
fn accurate_sum(terms: &[f64]) -> Result<f64> {
    let sum = terms.iter().try_fold(BigRational::zero(), |sum, &value| {
        BigRational::from_float(value).map(|v| sum+v).context("nonfinite interval")
    })?;
    let result = sum.to_f64().context("interval sum overflow")?;
    ensure!(result.is_finite(), "interval sum overflow");
    Ok(result)
}
fn nonnegative(value: &Value, name: &str) -> Result<f64> {
    let number = finite(value, name)?;
    ensure!(number >= 0.0, "{name} must be nonnegative");
    Ok(number)
}
fn exact_keys(value: &Value, names: &[&str], name: &str) -> Result<()> {
    let object = value.as_object().with_context(|| format!("{name} must be an object"))?;
    ensure!(object.len() == names.len() && names.iter().all(|key| object.contains_key(*key)),
        "{name} requires exactly {}", names.join(", "));
    Ok(())
}
fn same(actual: f64, expected: f64, name: &str) -> Result<()> {
    ensure!((actual-expected).abs() <= 1e-9,
        "phoneme_timing {name} disagrees with the scheduled segment boundaries");
    Ok(())
}
fn index(value: &Value, key: &str) -> Result<usize> {
    value.get(key).and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok())
        .with_context(|| format!("phoneme_timing {key} must be a nonnegative integer"))
}

fn nucleus_index(note: &Value) -> Result<Option<usize>> {
    match note.get("nucleus_segment_index") {
        Some(Value::Null) => Ok(None),
        _ => index(note, "nucleus_segment_index").map(Some),
    }
}

pub fn validate_score_phoneme_timing(score: &Value) -> Result<Option<Value>> {
    let Some(trace) = score.get("phoneme_timing") else { return Ok(None); };
    exact_keys(trace, &["method","notes"], "phoneme_timing")?;
    ensure!(trace["method"] == "vowel-anchor-v1", "unsupported phoneme_timing method");
    let notes = trace["notes"].as_array().context("phoneme_timing notes must be an array")?;
    let segments = score.get("segments").and_then(Value::as_array).context("phoneme_timing requires score segments")?;
    ensure!(!notes.is_empty() && !segments.is_empty(), "phoneme_timing requires nonempty notes and score segments");
    let mut starts = Vec::with_capacity(segments.len());
    let mut ends = Vec::with_capacity(segments.len());
    let mut accumulated = BigRational::zero();
    for (i, segment) in segments.iter().enumerate() {
        ensure!(segment.is_object(), "phoneme_timing requires segment objects");
        starts.push(accumulated.to_f64().context("score start overflow")?);
        let duration = field(segment, "duration_seconds", None)?;
        ensure!(duration > 0.0, "phoneme_timing segment {i} duration must be positive");
        accumulated += BigRational::from_float(duration).context("nonfinite duration")?;
        ends.push(accumulated.to_f64().context("score end overflow")?);
        let gap = field(segment, "gap_after_seconds", Some(0.0))?;
        ensure!(gap >= 0.0, "phoneme_timing segment {i} gap must be nonnegative");
        accumulated += BigRational::from_float(gap).context("nonfinite gap")?;
    }
    let mut identities = BTreeSet::new();
    let mut expected_first = 0;
    for note in notes {
        const REQUIRED: [&str; 8] = ["note_id", "note_onset_seconds", "note_end_seconds", "first_segment_index",
            "nucleus_segment_index", "end_segment_index", "prefix_seconds", "tail_trim_seconds"];
        let fields = note.as_object().context("phoneme_timing note must be an object")?;
        ensure!(REQUIRED.iter().all(|key| fields.contains_key(*key))
            && fields.len() == REQUIRED.len() + usize::from(fields.contains_key("nucleus_delay_seconds")),
            "phoneme_timing note has unknown or missing fields");
        let identity = note["note_id"].as_str().context("phoneme_timing note ID must be a string")?;
        ensure!(!identity.is_empty() && identities.insert(identity), "phoneme_timing note IDs must be nonempty and unique");
        let first = index(note,"first_segment_index")?;
        let nucleus = nucleus_index(note)?;
        let end = index(note,"end_segment_index")?;
        ensure!(first == expected_first && first < end && end <= segments.len()
            && nucleus.is_none_or(|n| (first..end).contains(&n)),
            "phoneme_timing ranges must cover each segment once in order and contain any declared nucleus");
        let onset = nonnegative(&note["note_onset_seconds"], "note onset")?;
        let note_end = nonnegative(&note["note_end_seconds"], "note end")?;
        let prefix = nonnegative(&note["prefix_seconds"], "prefix")?;
        let tail = nonnegative(&note["tail_trim_seconds"], "tail trim")?;
        let delay = note.get("nucleus_delay_seconds").map(|v| nonnegative(v, "nucleus delay")).transpose()?.unwrap_or(0.0);
        ensure!(note_end > onset && tail < note_end-onset,
            "phoneme_timing requires a positive note interval after tail trim");
        for segment in &segments[first..end-1] {
            ensure!(field(segment,"gap_after_seconds",Some(0.0))? == 0.0,
                "phoneme_timing does not allow an internal gap inside one scheduled note");
        }
        if let Some(n) = nucleus {
            same(onset + delay, starts[n], "nucleus anchor")?;
        } else {
            ensure!(prefix == 0.0 && delay == 0.0 && tail == 0.0,
                "a consonant-only event must own its full note interval on the beat");
        }
        if nucleus.is_none() || delay > 0.0 {
            // These extensions to the original on-beat contract must identify
            // the first real native nucleus, including a deliberate absence.
            let mut first_nucleus = None;
            for (offset, segment) in segments[first..end].iter().enumerate() {
                let identity = crate::model::resolve_segment_identity(segment)?;
                let id = super::metadata_integer(&identity, "phone_id")?.context("extended phoneme_timing requires a native phone identity")?;
                if super::duration::is_nucleus(id) { first_nucleus = Some(first + offset); break; }
            }
            ensure!(first_nucleus == nucleus, "phoneme_timing nucleus must match the first native nucleus");
        }
        same(onset - prefix, starts[first], "prefix")?;
        same(note_end-tail, ends[end-1], "tail trim")?;
        expected_first = end;
    }
    ensure!(expected_first == segments.len(), "phoneme_timing must cover every score segment");
    Ok(Some(trace.clone()))
}
pub fn validate_score_continuity(score: &Value) -> Result<Option<Value>> {
    let Some(marker) = score.get("continuity") else { return Ok(None); };
    ensure!(marker == &json!({"method":"continuous-runs-v1"}),
        "continuity requires exactly method='continuous-runs-v1'");
    ensure!(validate_score_phoneme_timing(score)?.is_some(),
        "continuity requires a validated vowel-anchor-v1 trace");
    let segments = score["segments"].as_array().context("continuity requires score segments")?;
    ensure!((1..=CONTINUITY_MAX_SEGMENTS).contains(&segments.len()),
        "continuity requires 1..2048 score segments");
    let total = score_seconds(segments)?;
    ensure!(total <= CONTINUITY_MAX_SECONDS+1e-12,
        "continuous run exceeds 120 seconds; insert a real rest or select a shorter passage");
    Ok(Some(marker.clone()))
}
pub(super) fn score_seconds(segments: &[Value]) -> Result<f64> {
    let mut terms = Vec::with_capacity(segments.len()*2);
    for segment in segments {
        let duration = field(segment,"duration_seconds",None)?;
        let gap = field(segment,"gap_after_seconds",Some(0.0))?;
        ensure!(duration >= 0.0 && gap >= 0.0, "score intervals must be nonnegative");
        terms.extend([duration,gap]);
    }
    accurate_sum(&terms)
}
pub(super) fn continuity_report(marker: Option<&Value>, runs: usize, timing: &TimingPlan, sample_rate: u32, hop: usize) -> Value {
    if marker.is_none() { return Value::Null; }
    json!({"method":"continuous-runs-v1","whole_request_rendered_together":true,
        "continuous_acoustic_runs":runs,"frame_count":timing.total_frames,
        "sample_rate":sample_rate,"hop_samples":hop,"artificial_job_boundaries":0,
        "endpoint_ramps":"retained only at actual acoustic run endpoints",
        "state_scope":"each acoustic run keeps one MLPG trajectory, pulse phase, noise stream, lattice state and peak calibration",
        "level_policy":"one existing 0.65 peak calibration per complete acoustic run; joining formerly split jobs can change their common gain"})
}
pub(super) fn realized_phoneme_timing(trace: Option<&Value>, timing: &TimingPlan, sample_rate: u32, hop: usize) -> Result<Value> {
    let Some(trace) = trace else { return Ok(Value::Null); };
    let mut notes = Vec::new();
    let mut maximum_error: Option<f64> = None;
    for source in trace["notes"].as_array().context("missing admitted timing notes")? {
        let first = &timing.segments[index(source,"first_segment_index")?];
        let last = &timing.segments[index(source,"end_segment_index")?-1];
        let nucleus_sample = nucleus_index(source)?.map(|n|
            timing.segments[n].start_frame.checked_mul(hop).context("sample offset overflow")).transpose()?;
        let actual = nucleus_sample.map(|sample| sample as f64 / sample_rate as f64);
        let onset = finite(&source["note_onset_seconds"],"note onset")?;
        let error = actual.map(|seconds| seconds - onset);
        if let Some(error) = error { maximum_error = Some(maximum_error.unwrap_or(0.0).max(error.abs())); }
        let mut note = source.clone();
        let object = note.as_object_mut().context("timing note must be an object")?;
        object.insert("actual_first_sample".into(), json!(first.start_frame.checked_mul(hop).context("sample offset overflow")?));
        object.insert("actual_nucleus_sample".into(), json!(nucleus_sample));
        object.insert("actual_end_sample".into(), json!(last.end_frame.checked_mul(hop).context("sample offset overflow")?));
        object.insert("actual_nucleus_seconds".into(), json!(actual));
        object.insert("nucleus_error_seconds".into(), json!(error));
        notes.push(note);
    }
    if notes.is_empty() { bail!("phoneme_timing requires notes"); }
    Ok(json!({"method":"vowel-anchor-v1","sample_rate":sample_rate,"hop_samples":hop,
        "clock":"relative to the rendered phrase's first sample",
        "scope":"planned nucleus/native final boundary; not an estimate of perceptual vowel onset",
        "maximum_nucleus_error_seconds":maximum_error,"notes":notes}))
}
