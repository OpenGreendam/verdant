//! Materialized vowel/rime anchors without moving MIDI geometry.
//!
//! Existing phone allocation supplies consonant budgets, not recovered OTO.
//! A consonant lead first consumes a rest, then available vowel time in the
//! previous note. Any remaining consonants sound inside their own note. An
//! event without a native nucleus occupies its original beat and duration.

use std::sync::Arc;

use verdant_domain::{Command, CommandBatch, Note, NoteId, Project, TempoMap, Track, TrackId};

use crate::adapter::{BlockedPhrase, Diagnostic, NoteSupport, PhrasePlan, Severity, compile_phrase_expression,
                     DURATION_ROUNDOFF_SECONDS, MAX_SCORE_SEGMENTS, PHRASE_BREAK_SECONDS, minimum_segment_seconds, note_support};
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

struct Scheduled<'a> {
    note: &'a Note,
    note_index: usize,
    follows_previous: bool,
    segments: Vec<Segment>,
    nuclei: Vec<bool>,
    nucleus: Option<usize>,
    desired_prefix: f64,
    prefix: f64,
    start: f64,
    onset: f64,
    nominal_end: f64,
    end: f64,
}

impl Scheduled<'_> {
    fn place_after(&mut self, earliest: f64) {
        let desired = self.onset - self.desired_prefix;
        self.start = desired.max(earliest).clamp(0.0, self.onset);
        // Preserve the original budget for an on-beat anchor, including
        // arithmetic residue at an exactly shared native segment boundary.
        self.prefix = if snap_seconds_bound(self.start, desired) == desired { self.desired_prefix }
            else { (self.onset - self.start).clamp(0.0, self.desired_prefix) };
    }

    fn nucleus_delay(&self) -> f64 { self.desired_prefix - self.prefix }

    fn earliest_end(&self, floor: f64) -> f64 {
        let Some(first) = self.nucleus else { return self.nominal_end };
        let tail: f64 = self.segments[first..].iter().zip(&self.nuclei[first..])
            .map(|(s, &vowel)| if vowel { floor } else { s.duration_seconds }).sum();
        (self.onset + self.nucleus_delay() + tail).min(self.nominal_end)
    }

    fn fit_vowels(&mut self, floor: f64) -> bool {
        let Some(first) = self.nucleus else { return self.end == self.nominal_end };
        let mut fixed = 0.0;
        let mut total = 0.0;
        let mut shortest = f64::INFINITY;
        let mut count = 0;
        let mut last = first;
        for (index, (segment, &vowel)) in self.segments.iter().zip(&self.nuclei).enumerate().skip(first) {
            if vowel {
                total += segment.duration_seconds;
                shortest = shortest.min(segment.duration_seconds);
                count += 1;
                last = index;
            } else {
                fixed += segment.duration_seconds;
            }
        }
        let target = self.end - self.onset - self.nucleus_delay() - fixed;
        let minimum = f64::from(count) * floor;
        if target < minimum - EPSILON { return false; }
        let scale = target / total;
        if shortest * scale >= floor {
            // This is the original on-beat allocation, including its arithmetic.
            for (segment, &vowel) in self.segments.iter_mut().zip(&self.nuclei) {
                if vowel { segment.duration_seconds *= scale; }
            }
        } else {
            // Once the shortest vowel reaches the floor, distribute only the
            // time still available above it. At that boundary both rules agree.
            let excess = total - f64::from(count) * shortest;
            let share = if excess > 0.0 { (target - minimum).max(0.0) / excess } else { 0.0 };
            for (segment, &vowel) in self.segments.iter_mut().zip(&self.nuclei) {
                if vowel { segment.duration_seconds = floor + (segment.duration_seconds - shortest) * share; }
            }
        }
        let emitted: f64 = self.segments.iter().map(|segment| segment.duration_seconds).sum();
        let residue = (self.end - self.start) - emitted;
        let corrected = self.segments[last].duration_seconds + residue;
        // Keep the absolute endpoints when the worker already admits this
        // residue. Clamping every note independently lengthens a whole run.
        if corrected < floor - DURATION_ROUNDOFF_SECONDS { return false; }
        self.segments[last].duration_seconds = corrected;
        true
    }
}

fn scheduled_end(notes: &[Scheduled<'_>], index: usize) -> f64 {
    let note = &notes[index];
    notes.get(index + 1).filter(|next| next.follows_previous)
        .map_or(note.nominal_end, |next| note.nominal_end.min(next.start))
}

struct Group {
    notes: std::ops::Range<usize>,
    segments: usize,
    within_limits: bool,
}

fn fit_request_span(notes: &mut [Scheduled<'_>], end: f64, max_phrase: f64, floor: f64) {
    let first = &mut notes[0];
    if snap_seconds_bound(end - first.start, max_phrase) <= max_phrase { return; }
    first.place_after(end - max_phrase);
    // Shortening the first lead can leave less room for the next one.
    for index in 1..notes.len() {
        let earliest = notes[index - 1].earliest_end(floor);
        if earliest <= notes[index].start { break; }
        notes[index].place_after(earliest);
    }
}

fn diagnostic(note: &Note, code: &'static str, message: String, detail: String) -> Diagnostic {
    Diagnostic { code, severity: Severity::Error, note_id: Some(note.id.clone()), message, detail }
}

/// Transform only an explicitly opted-in plan. Legacy expansion remains the
/// source of legal model paths and initial duration budgets; no worker ID is
/// used to guess phonetic boundaries here.
pub(crate) fn anchor_plan(plan: &mut PhrasePlan, track: &Track, tempo: &TempoMap, caps: &Capabilities, expression_point_limit: usize) {
    let floor = minimum_segment_seconds(plan.route, caps);
    let Some(template) = plan.phrases.first().cloned() else { return };
    let mut scheduled: Vec<Scheduled<'_>> = Vec::new();
    let mut source_notes = track.notes.iter().enumerate();
    for phrase in &plan.phrases {
        let mut offset = 0;
        for id in &phrase.note_ids {
            let (note_index, note) = source_notes.find(|(_, n)| &n.id == id).expect("source phrases follow track order");
            let nuclei = if note.is_continuation(){vec![true]}else{match note_support(note, caps).expect("legacy phrase passed preflight") {
                NoteSupport::VowelProxy { .. } => vec![true],
                NoteSupport::Pinyin(token) => if token.initial.is_empty() { vec![true] } else { vec![false, true] },
                NoteSupport::Phonemes(reading) => reading.phones.iter().map(|phone| phone.native.role == "nucleus").collect(),
                NoteSupport::Unsupported => unreachable!("unsupported note cannot enter a phrase"),
            }};
            let mut segments = phrase.score.segments[offset..offset+nuclei.len()].to_vec();
            offset += nuclei.len();
            for segment in &mut segments { segment.gap_after_seconds = 0.0; }
            let nucleus = nuclei.iter().position(|&vowel| vowel);
            let desired_prefix = segments[..nucleus.unwrap_or(0)].iter().map(|s| s.duration_seconds).sum();
            let onset = tempo.seconds_at(note.start_tick as f64);
            let nominal_end = tempo.seconds_at(note.end_tick() as f64);
            // Muted notes are silence. An unrenderable sounding note is a
            // barrier even when it is followed by one or more muted notes.
            let preceding = track.notes[..note_index].iter().rposition(|n| !n.is_muted());
            let previous = scheduled.last().filter(|n| Some(n.note_index) == preceding);
            let earliest = previous.map_or_else(
                || preceding.map_or(0.0, |i| tempo.seconds_at(track.notes[i].end_tick() as f64)),
                |n| n.earliest_end(floor));
            let mut event = Scheduled { note, note_index, follows_previous: previous.is_some(), segments, nuclei, nucleus, desired_prefix,
                prefix: 0.0, start: onset, onset, nominal_end, end: nominal_end };
            event.place_after(earliest);
            scheduled.push(event);
        }
        assert_eq!(offset, phrase.score.segments.len());
    }
    let timing = caps.native_score.as_ref().and_then(|n| n.phoneme_timing.as_ref());
    let constrained_nucleus_timing = timing.is_some_and(|t| t.constrained_nucleus_timing);
    let continuity = timing.and_then(|t| t.continuity.as_ref()).filter(|c| c.method == "continuous-runs-v1");
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
    // Keep an explicit continuation chain together. In continuous mode every
    // sounding run is indivisible, even if its source spans several phrases.
    let mut units: Vec<Group> = Vec::new();
    for (index, note) in scheduled.iter().enumerate() {
        if let Some(unit) = units.last_mut() {
            let previous = &scheduled[index - 1];
            let inseparable = continuity.map_or_else(|| note.note.is_continuation(),
                |c| note.start - previous.nominal_end < c.minimum_silence_seconds - EPSILON);
            if note.follows_previous && inseparable {
                unit.notes.end = index + 1;
                unit.segments += note.segments.len();
                continue;
            }
        }
        units.push(Group { notes: index..index + 1, segments: note.segments.len(), within_limits: false });
    }
    // An incoming lead can take an otherwise legal request past its duration
    // ceiling. Work right to left so that only a successor within its limits
    // can borrow a tail, and any excess lead returns to its own note.
    let segment_limit = continuity.map_or(MAX_SCORE_SEGMENTS, |c| c.max_segments);
    let mut next_within_limits = false;
    for unit in units.iter_mut().rev() {
        let last = unit.notes.end - 1;
        let end = if next_within_limits { scheduled_end(&scheduled, last) } else { scheduled[last].nominal_end };
        fit_request_span(&mut scheduled[unit.notes.clone()], end, max_phrase, floor);
        unit.within_limits = unit.segments <= segment_limit
            && snap_seconds_bound(end - scheduled[unit.notes.start].start, max_phrase) <= max_phrase;
        next_within_limits = unit.within_limits;
        for index in unit.notes.start..last {
            scheduled[index].end = scheduled_end(&scheduled, index);
        }
        scheduled[last].end = end;
    }
    let mut groups: Vec<Group> = Vec::new();
    for unit in units {
        if let Some(group) = groups.last_mut().filter(|g| continuity.is_none() && g.within_limits && unit.within_limits) {
            let previous = &scheduled[group.notes.end - 1];
            let next = &scheduled[unit.notes.start];
            let span = scheduled[unit.notes.end - 1].end - scheduled[group.notes.start].start;
            if next.follows_previous && next.start - previous.end < PHRASE_BREAK_SECONDS
                && snap_seconds_bound(span, max_phrase) <= max_phrase && group.segments + unit.segments <= MAX_SCORE_SEGMENTS {
                group.notes.end = unit.notes.end;
                group.segments += unit.segments;
                continue;
            }
        }
        groups.push(unit);
    }
    plan.phrases.clear();
    let first_new_blocked = plan.blocked.len();
    let all_notes: Vec<_> = track.notes.iter().collect();
    // These final checks can reject a whole group. Resolve each successor
    // before deciding whether the preceding group actually owes it a tail.
    let mut successor_admitted = false;
    while let Some(mut group) = groups.pop() {
        let next_admitted = std::mem::take(&mut successor_admitted);
        let last_index = group.notes.end - 1;
        let end = if next_admitted { scheduled_end(&scheduled, last_index) } else { scheduled[last_index].nominal_end };
        if group.within_limits && end != scheduled[last_index].end {
            // An ordinary merged request may need its last source chain in
            // a separate job once the tail is restored. Keep chains intact.
            if continuity.is_none() && snap_seconds_bound(end - scheduled[group.notes.start].onset, max_phrase) > max_phrase {
                let split = (group.notes.start + 1..=last_index).rev().find(|&i| !scheduled[i].note.is_continuation());
                if let Some(split) = split {
                    let tail = Group { notes: split..group.notes.end,
                        segments: scheduled[split..group.notes.end].iter().map(|n| n.segments.len()).sum(), within_limits: true };
                    group.notes.end = split;
                    group.segments -= tail.segments;
                    groups.push(group);
                    group = tail;
                }
            }
            fit_request_span(&mut scheduled[group.notes.clone()], end, max_phrase, floor);
            for index in group.notes.start..last_index {
                scheduled[index].end = scheduled_end(&scheduled, index);
            }
        }
        scheduled[last_index].end = end;
        let first = &scheduled[group.notes.start];
        let last = &scheduled[group.notes.end - 1];
        let first_note = first.note;
        let mut phrase = template.clone();
        phrase.note_ids = scheduled[group.notes.clone()].iter().map(|n| n.note.id.clone()).collect();
        phrase.start_tick = first.note.start_tick;
        phrase.end_tick = last.note.end_tick();
        phrase.start_seconds = first.start;
        phrase.seconds = snap_seconds_bound(last.end-first.start, max_phrase);
        if !group.within_limits || phrase.seconds > max_phrase {
            let code = if continuity.is_some() { "E_CONTINUOUS_RUN_LIMIT" } else if group.notes.len() > 1 {
                "E_CONTINUATION_PHRASE"
            } else { "E_PHONEME_TIMING_PHRASE_LIMIT" };
            plan.diagnostics.push(diagnostic(first_note, code,
                format!("连续发声段长 {:.2}s、{} 个音素，超过 {:.2}s / {} 段预算；请插入真实休止或缩短连续段", phrase.seconds, group.segments, max_phrase, segment_limit),
                "A source phone chain or continuous acoustic run cannot be split at an artificial boundary".into()));
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            continue;
        }
        let mut feasible = true;
        for note in &mut scheduled[group.notes.clone()] {
            if !note.fit_vowels(floor) {
                feasible = false;
                plan.diagnostics.push(diagnostic(note.note, "E_PHONEME_TIMING_BUDGET",
                    format!("「{}」的时间区间无法容纳全部音素，每段须保留至少 {:.0} ms", note.note.lyric, floor * 1000.0),
                    format!("scheduled={:.12}..{:.12}; note_onset={:.12}", note.start, note.end, note.onset)));
            } else if !constrained_nucleus_timing && (note.nucleus.is_none() || note.nucleus_delay() > 0.0) {
                feasible = false;
                let reason = if note.nucleus.is_none() { "没有元音核" } else { "元音核须在拍点之后起音" };
                plan.diagnostics.push(diagnostic(note.note, "E_PHONEME_TIMING_CAPABILITY",
                    format!("「{}」{reason}；请选择支持此类音符的新版合成后端", note.note.lyric),
                    "This acoustic group requires native_score.phoneme_timing.constrained_nucleus_timing".into()));
            }
        }
        if !feasible {
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            continue;
        }
        let mut score = NativeScore {
            title: String::new(),
            segments: Vec::with_capacity(group.segments),
            original_hierarchy: template.score.original_hierarchy.clone(),
            synthesis: template.score.synthesis.clone(),
            phoneme_timing: None,
            continuity: None,
        };
        let mut trace = Vec::with_capacity(group.notes.len());
        let mut leads = Vec::new();
        let mut source_nucleus = None;
        for index in group.notes.clone() {
            let next_start = (index + 1 < group.notes.end).then(|| scheduled[index + 1].start);
            let note = &mut scheduled[index];
            let offset = score.segments.len();
            if note.note.is_continuation() {
                let anchor = source_nucleus.expect("continuation chains stay with their source nucleus");
                note.segments[0].continuation_of = Some(anchor);
            } else {
                source_nucleus = note.nuclei.iter().rposition(|&vowel| vowel).map(|n| offset + n);
            }
            if let Some(next) = next_start {
                note.segments.last_mut().unwrap().gap_after_seconds = (next - note.end).max(0.0);
            }
            trace.push(PhonemeTimingNote {
                note_id: note.note.id.to_string(),
                note_onset_seconds: note.onset - phrase.start_seconds,
                note_end_seconds: note.nominal_end - phrase.start_seconds,
                first_segment_index: offset,
                nucleus_segment_index: note.nucleus.map(|n| offset + n),
                nucleus_delay_seconds: note.nucleus_delay(),
                end_segment_index: offset + note.segments.len(),
                prefix_seconds: note.prefix,
                tail_trim_seconds: (note.nominal_end-note.end).max(0.0),
            });
            if note.prefix > 0.0 {
                leads.push(Preutterance { note_index: note.note_index, start_seconds: note.start, end_seconds: note.onset });
            }
            score.segments.append(&mut note.segments);
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
            plan.diagnostics.push(diagnostic(first_note, "E_PHONEME_TIMING_CONTROL",
                "元音对拍的音高控制轨无效或超出后端范围".into(),
                "Full-track boundary context and scheduled preutterance pitch are validated after composition".into()));
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            continue;
        }
        phrase.score = Arc::new(score);
        phrase.control_track = Some(Arc::new(control));
        if let Err(diagnostic) = compile_phrase_expression(&mut phrase, track, tempo, expression_point_limit) {
            plan.diagnostics.push(diagnostic);
            plan.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            continue;
        }
        successor_admitted = true;
        plan.phrases.push(phrase);
    }
    plan.phrases.reverse();
    plan.blocked[first_new_blocked..].reverse();
    for (index, phrase) in plan.phrases.iter_mut().enumerate() {
        phrase.index = index;
        Arc::get_mut(&mut phrase.score).expect("new score has one owner").title = format!("Verdant {} phrase {}", track.name, index + 1);
    }
}
