//! Explicit source-phone continuation topology, independent authored note ownership.
use anyhow::{ensure,Context,Result};
use serde_json::{json,Value};
use super::{duration,metadata_integer,metadata_key};
use super::timing::field;

fn voiced(segment: &Value) -> Result<bool> {
    segment.get("voiced").map(|value| value.as_bool().context("segment voiced must be boolean"))
        .transpose().map(|value| value.unwrap_or(true))
}

/// Map each segment to its physical source-phone anchor. A continuation
/// always names the original unmarked nucleus, never an alias or a merely
/// matching phone elsewhere. This helper does not call model/context code;
/// the model adapter uses it to infer physical neighbors around whole chains.
pub fn continuation_source_indices(segments: &[Value]) -> Result<Vec<usize>> {
    let mut sources: Vec<usize> = (0..segments.len()).collect();
    for (index,segment) in segments.iter().enumerate() {
        let Some(value) = segment.get("continuation_of") else { continue; };
        let anchor = value.as_u64().and_then(|n| usize::try_from(n).ok())
            .context("continuation_of must be an earlier original nucleus segment index")?;
        ensure!(anchor < index, "continuation_of must name an earlier original nucleus");
        ensure!(segments[anchor].get("continuation_of").is_none(),
            "continuation_of must name the original nucleus anchor, not another link");
        ensure!(sources[index-1] == anchor,
            "continuation must immediately follow its anchor or another link to that same anchor");
        ensure!(field(&segments[index-1],"gap_after_seconds",Some(0.0))? == 0.0,
            "continuation cannot cross an authored gap, including a gap that quantizes to zero frames");
        ensure!(voiced(&segments[anchor])? && voiced(&segments[index-1])? && voiced(segment)?,
            "continuation cannot cross an unvoiced or muted source segment");
        sources[index] = anchor;
    }
    Ok(sources)
}

pub(super) fn phonemic_length(segment: &Value, native_key: Option<&str>) -> Result<i64> {
    let mut length = 1;
    if let Some(trace) = segment.get("phoneme").filter(|value| value.is_object()) {
        let marked = trace.get("source_language").and_then(Value::as_str) == Some("ja")
            && trace.get("canonical_ipa").and_then(Value::as_str).is_some_and(|value| value.ends_with('ː'))
            && native_key.is_some_and(|key| trace.get("native_key").and_then(Value::as_str) == Some(key));
        if marked {
            if let Some(value) = trace.get("length").and_then(Value::as_u64).filter(|&n| n > 1) {
                length = i64::try_from(value).context("phonemic length overflows int64")?;
            }
        }
    }
    Ok(length)
}

#[derive(Clone,Debug,PartialEq)]
struct Identity { phone: i64, language: Option<i64>, native_key: String, length: i64 }
fn identity(segment: &Value) -> Result<Identity> {
    let resolved = crate::model::resolve_segment_identity(segment)?;
    let phone = metadata_integer(&resolved,"phone_id")?.context("continuation requires an audited native nucleus")?;
    ensure!(duration::is_nucleus(phone), "continuation requires a voiced native vowel/rime nucleus");
    let native_key = metadata_key(&resolved)?.context("continuation nucleus has no audited native key")?;
    let language = metadata_integer(&resolved,"language")?;
    let length = phonemic_length(segment,Some(&native_key))?;
    Ok(Identity { phone,language,native_key,length })
}

/// Validate already-prepared contexts without re-running score normalization.
/// Semantic context facts must be copied from the anchor. Labels and audit
/// assumptions may differ; they do not alter model selection.
pub(super) fn validate_prepared_continuations(segments: &[Value], sources: &[usize]) -> Result<()> {
    ensure!(segments.len() == sources.len(), "continuation topology shape mismatch");
    let mut identities: Vec<Option<Identity>> = vec![None;segments.len()];
    for (index,&anchor) in sources.iter().enumerate() {
        if anchor == index { continue; }
        ensure!(anchor < index, "invalid continuation anchor order");
        if identities[anchor].is_none() { identities[anchor] = Some(identity(&segments[anchor])?); }
        let current = identity(&segments[index])?;
        ensure!(identities[anchor].as_ref() == Some(&current),
            "continuation must retain its anchor's audited native nucleus, language and phonemic length");
        let original = &segments[anchor]["context"];
        let context = &segments[index]["context"];
        for key in ["features","answers","anchor_questions"] {
            ensure!(context.get(key) == original.get(key),
                "continuation must copy the original nucleus context {key}; authored note F0 remains separate");
        }
        identities[index] = Some(current);
    }
    Ok(())
}

/// Pure synchronous request admission, without a model file or synthesis.
/// Track/mute scheduling is resolved by the editor before a single-track
/// native score is serialized; explicit gaps/unvoiced barriers are checked
/// here and the original physical nucleus context must survive unchanged.
pub fn validate_score_continuations(score: &Value) -> Result<()> {
    ensure!(score.is_object(), "score must be an object");
    let segments = score.get("segments").and_then(Value::as_array).context("score requires segments")?;
    let sources = continuation_source_indices(segments)?;
    if sources.iter().enumerate().all(|(index,&anchor)| index == anchor) { return Ok(()); }
    let prepared = crate::model::prepare_score_segments(score,&json!({}))?;
    validate_prepared_continuations(&prepared,&sources)
}

/// Occupancy of the one source-phone state sequence inside one authored
/// segment. Zero counts are meaningful: continuation notes never restart
/// state zero just to manufacture five positive per-note state counts.
pub(super) fn state_occupancy(states: &[usize], offset: usize, frames: usize) -> Result<Vec<usize>> {
    state_occupancy_with_compact(states,offset,frames,false)
}
pub(super) fn state_occupancy_with_compact(states: &[usize], offset: usize, frames: usize,
    compact: bool) -> Result<Vec<usize>> {
    ensure!(states.len() == 5 && states.iter().any(|&count| count > 0)
        && (compact || states.iter().all(|&count| count > 0)),
        "source phone requires five positive state counts");
    let end = offset.checked_add(frames).context("continuation frame range overflow")?;
    let total = states.iter().try_fold(0usize,|a,&b| a.checked_add(b)).context("source frame budget overflow")?;
    ensure!(end <= total, "authored segment escaped its source phone budget");
    let mut cursor = 0;
    let mut occupancy = Vec::with_capacity(5);
    for &count in states {
        let next = cursor+count;
        occupancy.push(next.min(end).saturating_sub(cursor.max(offset)));
        cursor = next;
    }
    ensure!(occupancy.iter().sum::<usize>() == frames, "continuation changed an authored frame budget");
    Ok(occupancy)
}
