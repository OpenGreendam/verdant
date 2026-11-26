//! Materialized vowel/rime anchors without moving MIDI geometry.
//!
//! Existing phone allocation supplies consonant budgets, not recovered OTO.
//! Prefixes precede the first native nucleus at note-on; the next prefix first
//! consumes a rest and then the previous note's vowel tail. Global scheduling
//! precedes phrase splitting, so a job boundary cannot reset these decisions.

use std::sync::Arc;

use verdant_domain::{Command, CommandBatch, Note, NoteId, Project, TempoMap, Track, TrackId};

use crate::adapter::{BlockedPhrase, Diagnostic, NoteSupport, PhrasePlan, Severity,
                     MAX_SCORE_SEGMENTS, PHRASE_BREAK_SECONDS, minimum_segment_seconds, note_support};
use crate::f0::{Preutterance, anchored_control_track, continuous_control_track, snap_seconds_bound};
use crate::wire::{Capabilities, ContinuousRuns, NativeScore, PhonemeTimingNote, PhonemeTimingTrace, Route, Segment};

const EPSILON: f64 = 1e-10;

/// The note-local allocation before optional vowel-anchor scheduling. The UI
/// shows this separately from the materialized plan: neighboring consonants
/// can trim a scheduled vowel without changing its authored allocation.
#[derive(Clone, Debug)]
pub struct PhonemeBudget {
    pub labels: Vec<String>,
    pub durations_seconds: Vec<f64>,
    pub total_seconds: f64,
    pub minimum_segment_seconds: f64,
    /// None when a legal automatic allocation cannot be represented in whole
    /// permille while retaining every admitted segment minimum.
    pub weights_permille: Option<Vec<u32>>,
    pub editable: bool,
}

impl PhonemeBudget {
    pub fn minimum_permille(&self) -> u32 {
        minimum_permille(self.total_seconds, self.minimum_segment_seconds)
    }
}

/// Resolve exactly the native segments addressed by duration overrides. A
/// canonical symbol can expand into several segments; never split by text.
pub fn note_budget(note: &Note, tempo: &TempoMap, caps: &Capabilities, route: Route) -> Result<PhonemeBudget, String> {
    if note.is_continuation() { return Err("延音沿用前一音符的元音，没有独立音素边界".into()); }
    let total = tempo.seconds_at(note.end_tick() as f64) - tempo.seconds_at(note.start_tick as f64);
    let floor = minimum_segment_seconds(route, caps);
    if !total.is_finite() || total < floor - EPSILON {
        return Err(format!("音符须至少 {:.0} ms", floor * 1000.0));
    }
    let explicit = note.pronunciation.phoneme_durations_permille.as_deref();
    let (labels, durations) = match note_support(note, caps).map_err(|d| d.message)? {
        NoteSupport::Phonemes(reading) => {
            let durations = crate::phonemes::allocate_for_note_with_minimum(&reading, total, explicit,
                note.pronunciation.mora_weights.as_deref(), floor)?;
            let labels = reading.phones.iter().map(|phone|
                format!("{} → {}", phone.canonical.symbol, phone.native_key)).collect();
            (labels, durations)
        }
        NoteSupport::Pinyin(token) if !token.initial.is_empty() => {
            let initial = explicit.and_then(|w| w.first()).map_or_else(
                || crate::pinyin::bounded_initial_seconds(&token, total, floor),
                |&w| total * w as f64 / 1000.0);
            (vec![token.initial, token.final_], vec![initial, total - initial])
        }
        NoteSupport::Pinyin(token) => (vec![token.final_], vec![total]),
        NoteSupport::VowelProxy { vowel } => (vec![vowel], vec![total]),
        NoteSupport::Unsupported => return Err("当前后端未解析此音符的音素".into()),
    };
    if durations.iter().any(|d| !d.is_finite() || *d < floor - EPSILON) {
        return Err(format!("每个音素须至少 {:.0} ms；请延长音符", floor * 1000.0));
    }
    let minimum = minimum_permille(total, floor);
    if explicit.is_some_and(|w| w.len() != labels.len() || w.iter().any(|&v| v < minimum)
        || w.iter().map(|&v| u64::from(v)).sum::<u64>() != 1000) {
        return Err(format!("音素时长比例须对应实际音素、总和 1000，且每段至少 {:.0} ms", floor * 1000.0));
    }
    // Quantize cumulative boundaries once. Clamping leaves enough room for
    // every following phone, including automatic allocations at their floor.
    let weights = explicit.map(<[u32]>::to_vec).or_else(|| {
        if minimum as usize * labels.len() > 1000 { return None; }
        let mut weights = Vec::with_capacity(labels.len());
        let (mut seconds, mut previous) = (0.0, 0u32);
        for (index, &duration) in durations.iter().enumerate() {
            seconds += duration;
            let end = if index + 1 == labels.len() { 1000 } else {
                ((seconds / total * 1000.0).round() as u32).clamp(previous + minimum,
                    1000 - minimum * (labels.len() - index - 1) as u32)
            };
            weights.push(end - previous);
            previous = end;
        }
        Some(weights)
    });
    let editable = caps.native_score.as_ref().is_some_and(|ns| ns.phoneme_duration_control)
        && weights.is_some();
    Ok(PhonemeBudget { labels, durations_seconds: durations, total_seconds: total,
        minimum_segment_seconds: floor, weights_permille: weights, editable })
}

/// Smallest whole-permille share whose represented duration reaches the floor.
/// Snap only arithmetic residue at an integer boundary, as duration admission
/// does; genuine fractional permille requirements still round upward.
pub fn minimum_permille(total_seconds: f64, minimum_segment_seconds: f64) -> u32 {
    let required = minimum_segment_seconds / total_seconds * 1000.0;
    let rounded = required.round();
    let required = if (required - rounded).abs() <= 8.0 * f64::EPSILON * required.abs() {
        rounded
    } else { required };
    required.ceil().max(1.0) as u32
}

/// Move an internal boundary (1..phone count), in seconds from note-on.
/// Only its adjacent pair exchanges budget; all other represented durations
/// remain unchanged. Reject out-of-range requests rather than silently clamp
/// the gesture. This prepares one transaction and never mutates the document.
pub fn drag_boundary(project: &Project, track_id: &TrackId, note_id: &NoteId, caps: &Capabilities,
    boundary: usize, position_seconds: f64) -> Result<CommandBatch, String> {
    let track = project.track(track_id).ok_or("音轨不存在")?;
    let note = track.note(note_id).ok_or("音符不存在")?;
    let route = crate::adapter::desktop_route(project, track, Some(caps));
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).map_err(|e| e.to_string())?;
    let settings = crate::phonemizer::settings_from_project(project)?;
    let mut resolved = crate::phonemizer::resolve_track(track, &settings);
    let budget = if let Some(reading) = resolved.notes.remove(note_id) {
        let mut resolved_note = note.clone();
        resolved_note.pronunciation = reading.pronunciation;
        note_budget(&resolved_note, &tempo, caps, route)?
    } else {
        note_budget(note, &tempo, caps, route)?
    };
    if !budget.editable { return Err("当前后端或音符长度不支持逐音素时长编辑".into()); }
    let minimum = budget.minimum_permille();
    let mut weights = budget.weights_permille.ok_or("音素时长不能用千分比表示")?;
    if boundary == 0 || boundary >= weights.len() || !position_seconds.is_finite() {
        return Err("请选择两个音素之间的有效边界".into());
    }
    let left: u32 = weights[..boundary - 1].iter().sum();
    let right: u32 = weights[..=boundary].iter().sum();
    let proposed = position_seconds / budget.total_seconds * 1000.0;
    // A position computed from an exact permille boundary can round by a few
    // binary64 ULPs on its way through seconds. The stored integer still has
    // to satisfy the exact minimum on both sides.
    if proposed < (left + minimum) as f64 - 1e-9 || proposed > (right - minimum) as f64 + 1e-9 {
        return Err(format!("边界不能越过相邻音素；每段须保留至少 {:.0} ms", budget.minimum_segment_seconds * 1000.0));
    }
    let end = proposed.round() as u32;
    weights[boundary - 1] = end - left;
    weights[boundary] = right - end;
    let mut pronunciation = note.pronunciation.clone();
    if pronunciation.alphabet.is_none() { pronunciation.phonemes = budget.labels; }
    pronunciation.phoneme_durations_permille = Some(weights);
    Ok(CommandBatch::single("拖动音素边界", Command::SetPronunciation {
        track: track_id.clone(), note_id: note_id.clone(), pronunciation,
    }))
}

#[derive(Clone)]
struct Scheduled<'a> {
    note: &'a Note,
    note_index: usize,
    segments: Vec<Segment>,
    continuation_anchor: Option<verdant_domain::NoteId>,
    nuclei: Vec<bool>,
    nucleus: usize,
    prefix: f64,
    start: f64,
    onset: f64,
    nominal_end: f64,
    end: f64,
}

fn diagnostic(note: &Note, code: &'static str, message: String, detail: String) -> Diagnostic {
    Diagnostic { code, severity: Severity::Error, note_id: Some(note.id.clone()), message, detail }
}

/// Transform only an explicitly opted-in plan. Legacy expansion remains the
/// source of legal model paths and initial duration budgets; no worker ID is
/// used to guess phonetic boundaries here.
pub(crate) fn anchor_plan(plan: &mut PhrasePlan, _project: &Project, track: &Track, tempo: &TempoMap, caps: &Capabilities) {
    let floor = minimum_segment_seconds(plan.route, caps);
    let Some(template) = plan.phrases.first().cloned() else { return };
    let all_notes: Vec<_> = track.notes.iter().collect();
    let mut scheduled = Vec::new();
    let mut errors = Vec::new();
    for phrase in &plan.phrases {
        let mut offset = 0;
        let mut source_anchors=std::collections::BTreeMap::<usize,verdant_domain::NoteId>::new();
        for id in &phrase.note_ids {
            let note_index = track.notes.iter().position(|note| &note.id == id).expect("planned note exists");
            let note = &track.notes[note_index];
            let nuclei = if note.is_continuation(){vec![true]}else{match note_support(note, caps).expect("legacy phrase passed preflight") {
                NoteSupport::VowelProxy { .. } => vec![true],
                NoteSupport::Pinyin(token) => if token.initial.is_empty() { vec![true] } else { vec![false, true] },
                NoteSupport::Phonemes(reading) => reading.phones.iter().map(|phone| phone.native.role == "nucleus").collect(),
                NoteSupport::Unsupported => unreachable!("unsupported note cannot enter a phrase"),
            }};
            let mut segments = phrase.score.segments[offset..offset+nuclei.len()].to_vec();
            let source_offset=offset;
            offset += nuclei.len();
            let continuation_anchor=segments.first().and_then(|segment|segment.continuation_of).and_then(|anchor|source_anchors.get(&anchor)).cloned();
            if note.is_continuation() && continuation_anchor.is_none() {
                errors.push(diagnostic(note,"E_CONTINUATION_ANCHOR","延音缺少同乐句的原始元音锚点".into(),"continuation link must remain in one source phrase".into()));
                continue;
            }
            if !note.is_continuation(){if let Some(nucleus)=nuclei.iter().rposition(|value|*value){source_anchors.insert(source_offset+nucleus,note.id.clone());}}
            for segment in &mut segments { segment.gap_after_seconds = 0.0; }
            let Some(nucleus) = nuclei.iter().position(|is_nucleus| *is_nucleus) else {
                errors.push(diagnostic(note, "E_PHONEME_TIMING_NUCLEUS",
                    "元音对拍需要明确的元音/韵母核；此音符只有辅音，不能推测锚点".into(),
                    "No inventory-declared nucleus in this note's materialized pronunciation".into()));
                continue;
            };
            let prefix: f64 = segments[..nucleus].iter().map(|segment| segment.duration_seconds).sum();
            let onset = tempo.seconds_at(note.start_tick as f64);
            let nominal_end = tempo.seconds_at(note.end_tick() as f64);
            let start = onset - prefix;
            if start < -EPSILON {
                errors.push(diagnostic(note, "E_PHONEME_TIMING_PREROLL",
                    format!("「{}」的辅音要比拍点早 {:.0} ms 开始，但离曲首太近，前面没有空间", note.lyric, prefix*1000.0),
                    format!("note_onset={onset:.12}; prefix={prefix:.12}; scheduled_start={start:.12}; negative timeline is not silently delayed")));
            }
            scheduled.push(Scheduled { note, note_index, segments, continuation_anchor, nuclei, nucleus, prefix,
                start: start.max(0.0), onset, nominal_end, end: nominal_end });
        }
        assert_eq!(offset, phrase.score.segments.len());
    }
    // A bad legacy phrase remains an acoustic barrier. The new policy never
    // borrows across an unrenderable/unresolved note in the source track.
    for index in 0..scheduled.len() {
        let next_start = scheduled.get(index+1)
            .filter(|next| next.note_index == scheduled[index].note_index+1)
            .map(|next| next.start);
        let note = &mut scheduled[index];
        note.end = next_start.map_or(note.nominal_end, |next| note.nominal_end.min(next));
        let target = note.end - note.onset;
        let fixed: f64 = note.segments.iter().zip(&note.nuclei).skip(note.nucleus)
            .filter(|(_, is_nucleus)| !**is_nucleus).map(|(segment, _)| segment.duration_seconds).sum();
        let original_vowels: f64 = note.segments.iter().zip(&note.nuclei)
            .filter(|(_, is_nucleus)| **is_nucleus).map(|(segment, _)| segment.duration_seconds).sum();
        let scale = (target-fixed)/original_vowels;
        let feasible = scale.is_finite() && scale > 0.0 && note.segments.iter().zip(&note.nuclei)
            .all(|(segment, is_nucleus)| !is_nucleus || segment.duration_seconds*scale >= floor-EPSILON);
        if !feasible {
            errors.push(diagnostic(note.note, "E_PHONEME_TIMING_BUDGET",
                format!("「{}」的元音预算不足：相邻辅音提前后，每个音素仍须至少 {:.0} ms", note.note.lyric, floor * 1000.0),
                format!("nucleus_anchor={:.12}; scheduled_end={:.12}; fixed_suffix={fixed:.12}; vowel_budget={:.12}", note.onset, note.end, target-fixed)));
            continue;
        }
        for (segment, is_nucleus) in note.segments.iter_mut().zip(&note.nuclei) {
            if *is_nucleus { segment.duration_seconds *= scale; }
        }
        // Absorb only floating arithmetic residue into the final nucleus.
        let total: f64 = note.segments.iter().map(|segment| segment.duration_seconds).sum();
        let last_nucleus = note.nuclei.iter().rposition(|is_nucleus| *is_nucleus).unwrap();
        note.segments[last_nucleus].duration_seconds += (note.end-note.start)-total;
    }
    if !errors.is_empty() {
        plan.diagnostics.extend(errors);
        for phrase in plan.phrases.drain(..) {
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
        }
        return;
    }
    let continuity = caps.native_score.as_ref().and_then(|n| n.phoneme_timing.as_ref())
        .and_then(|t| t.continuity.as_ref()).filter(|c| c.method == "continuous-runs-v1");
    if continuity.is_some_and(|c| !c.max_seconds.is_finite() || !(0.025..=120.0).contains(&c.max_seconds)
        || !(1..=2048).contains(&c.max_segments) || !(2..=32768).contains(&c.max_control_points)
        || c.minimum_silence_seconds != 0.005) {
        plan.diagnostics.push(diagnostic(scheduled[0].note, "E_CONTINUITY_CAPABILITY",
            "后端的连续乐句能力声明无效；不会退回发声中硬切分".into(),
            "continuous-runs-v1 requires <=120 seconds, <=2048 segments, <=32768 control points and a 5 ms real-rest boundary".into()));
        for phrase in plan.phrases.drain(..) {
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
        }
        return;
    }
    let max_phrase = continuity.map_or_else(
        || if caps.max_phrase_seconds > 0.0 { caps.max_phrase_seconds.min(30.0) } else { 30.0 },
        |c| c.max_seconds);
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, note) in scheduled.iter().enumerate() {
        if note.end-note.start > max_phrase+EPSILON {
            errors.push(diagnostic(note.note, "E_PHONEME_TIMING_PHRASE_LIMIT",
                "音符加上辅音提前量超过单次渲染时长上限；请缩短音符或辅音预算".into(),
                format!("scheduled_span={:.12}; max_phrase={max_phrase}", note.end-note.start)));
            continue;
        }
        if let Some(group) = groups.last_mut() {
            let first = &scheduled[group[0]];
            let previous = &scheduled[*group.last().unwrap()];
            let segment_count: usize = group.iter().map(|&i| scheduled[i].segments.len()).sum();
            let connected = note.note_index == previous.note_index+1;
            let can_append = if let Some(c) = continuity {
                // Whole-run limits are checked after grouping. A resource
                // budget must not create a hidden reset in continuous voice.
                connected && note.start-previous.end < c.minimum_silence_seconds-EPSILON
            } else {
                connected && note.start-previous.end < PHRASE_BREAK_SECONDS
                    && snap_seconds_bound(note.end-first.start, max_phrase) <= max_phrase
                    && segment_count+note.segments.len() <= MAX_SCORE_SEGMENTS
            };
            if can_append {
                group.push(index);
                continue;
            }
        }
        groups.push(vec![index]);
    }
    for group in &groups {
        let first=&scheduled[group[0]];
        if first.continuation_anchor.is_some(){errors.push(diagnostic(first.note,"E_CONTINUATION_PHRASE","延音链超出渲染边界；请插入真实休止或缩短连续段".into(),"resource limits cannot silently create a new onset".into()));}
    }
    if let Some(c) = continuity {
        for group in &groups {
            let first = &scheduled[group[0]];
            let last = &scheduled[*group.last().unwrap()];
            let count: usize = group.iter().map(|&i| scheduled[i].segments.len()).sum();
            if snap_seconds_bound(last.end-first.start, max_phrase) > max_phrase || count > c.max_segments {
                errors.push(diagnostic(first.note, "E_CONTINUOUS_RUN_LIMIT",
                    format!("连续发声段长 {:.2}s、{} 个音素，超过 {:.0}s / {} 段预算；请插入真实休止或选择更短段落", last.end-first.start, count, max_phrase, c.max_segments),
                    "A connected acoustic run is never silently split; acoustic and waveform reconstruction state must share the entire run".into()));
            }
        }
    }
    if !errors.is_empty() {
        plan.diagnostics.extend(errors);
        for phrase in plan.phrases.drain(..) {
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
        }
        return;
    }
    plan.phrases.clear();
    for group in groups {
        let first = &scheduled[group[0]];
        let last = &scheduled[*group.last().unwrap()];
        let mut phrase = template.clone();
        phrase.index = plan.phrases.len();
        phrase.note_ids = group.iter().map(|&i| scheduled[i].note.id.clone()).collect();
        phrase.start_tick = first.note.start_tick;
        phrase.end_tick = last.note.end_tick();
        phrase.start_seconds = first.start;
        phrase.seconds = snap_seconds_bound(last.end-first.start, max_phrase);
        let mut score = NativeScore {
            title: format!("Verdant {} phrase {}", track.name, phrase.index+1),
            segments: Vec::new(),
            original_hierarchy: template.score.original_hierarchy.clone(),
            synthesis: template.score.synthesis.clone(),
            phoneme_timing: None,
            continuity: None,
        };
        let mut trace = Vec::with_capacity(group.len());
        let mut leads = Vec::new();
        let mut emitted_anchors=std::collections::BTreeMap::<verdant_domain::NoteId,usize>::new();
        for (position, &index) in group.iter().enumerate() {
            let note = &scheduled[index];
            let offset = score.segments.len();
            let mut segments = note.segments.clone();
            if let Some(anchor)=&note.continuation_anchor {
                let anchor=*emitted_anchors.get(anchor).expect("continuation group retains original anchor");
                for segment in &mut segments{segment.continuation_of=Some(anchor);}
            }else if let Some(nucleus)=note.nuclei.iter().rposition(|value|*value){emitted_anchors.insert(note.note.id.clone(),offset+nucleus);}
            if let Some(next) = group.get(position+1).map(|&index| &scheduled[index]) {
                segments.last_mut().unwrap().gap_after_seconds = (next.start-note.end).max(0.0);
            }
            trace.push(PhonemeTimingNote {
                note_id: note.note.id.to_string(),
                note_onset_seconds: note.onset-first.start,
                note_end_seconds: note.nominal_end-first.start,
                first_segment_index: offset,
                nucleus_segment_index: offset+note.nucleus,
                end_segment_index: offset+segments.len(),
                prefix_seconds: note.prefix,
                tail_trim_seconds: (note.nominal_end-note.end).max(0.0),
            });
            if note.prefix > 0.0 {
                leads.push(Preutterance { note_index: note.note_index, start_seconds: note.start, end_seconds: note.onset });
            }
            score.segments.extend(segments);
        }
        score.phoneme_timing = Some(PhonemeTimingTrace { method: "vowel-anchor-v1".into(), notes: trace });
        score.continuity = continuity.map(|_| ContinuousRuns { method: "continuous-runs-v1".into() });
        let control = if continuity.is_some() {
            continuous_control_track(&all_notes, tempo, phrase.start_seconds, phrase.seconds, &leads, max_phrase)
        } else {
            anchored_control_track(&all_notes, tempo, phrase.start_seconds, phrase.seconds, &leads)
        };
        let range = crate::adapter::authored_f0_hz_range(plan.route, caps);
        let valid_control = if let Some(c) = continuity {
            crate::f0::validate_with_limits(&control, max_phrase, c.max_control_points)
        } else { crate::f0::validate(&control) };
        if valid_control.is_err() || control.f0_hz.iter().zip(&control.uv)
            .any(|(&hz, &uv)| !uv && !(range[0]..=range[1]).contains(&hz)) {
            plan.diagnostics.push(diagnostic(first.note, "E_PHONEME_TIMING_CONTROL",
                "元音对拍的音高控制轨无效或超出后端范围".into(),
                "Full-track boundary context and scheduled preutterance pitch are validated after composition".into()));
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            continue;
        }
        phrase.score = Arc::new(score);
        phrase.control_track = Some(Arc::new(control));
        plan.phrases.push(phrase);
    }
}
