//! Authored voice controls on the same score clock as vowel preutterance/DYN.
use std::collections::BTreeMap;
use serde::Serialize;
use verdant_domain::{Note, NoteId, TempoMap, Track};
use verdant_domain::model::ExpressionPoint;
use crate::adapter::Phrase;

/// Derived seconds-clock points have a separate budget from the 4096 authored
/// points retained in the project. Tempo boundaries must never be discarded.
pub const MAX_COMPILED_CURVE_POINTS: usize = 32768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionCompileError {
    pub note_id: NoteId,
    pub parameter: String,
    pub point_count: usize,
    pub maximum_points: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExpressionTrack {
    pub method: String,
    pub segments: Vec<ExpressionSegment>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExpressionSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub note_onset_seconds: f64,
    pub curves: BTreeMap<String, Vec<[f64; 2]>>,
}

/// Linear tick curves are only linear in seconds within one tempo interval.
/// Materialize every intervening tempo boundary without quantizing its value;
/// fractional interpolated values are part of the worker's numeric contract.
/// Held ends need no additional knots, including a note's consonant prefix.
fn linear_curve_on_seconds_clock(note: &Note, points: &[ExpressionPoint], tempo: &TempoMap, maximum_points: usize) -> Result<Vec<[f64; 2]>, usize> {
    let authored: Vec<_> = points.iter().map(|point| (point.tick, f64::from(point.value))).collect();
    let mut knots = authored.clone();
    if knots.len() > maximum_points { return Err(knots.len()); }
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        let first_tick = note.start_tick + first.tick;
        let last_tick = note.start_tick + last.tick;
        for interval in tempo.points().windows(2).filter(|pair| pair[0].micro_bpm != pair[1].micro_bpm) {
            let boundary = &interval[1];
            if boundary.tick <= first_tick || boundary.tick >= last_tick { continue; }
            let tick = boundary.tick - note.start_tick;
            if let Err(index) = authored.binary_search_by_key(&tick, |point| point.0) {
                if authored[index-1].1 != authored[index].1 {
                    knots.push((tick, crate::f0::interp(&authored, tick as f64)));
                    if knots.len() > maximum_points { return Err(knots.len()); }
                }
            }
        }
    }
    knots.sort_by_key(|point| point.0);
    let onset = tempo.seconds_at(note.start_tick as f64);
    Ok(knots.into_iter().map(|(tick, value)| [tempo.seconds_at((note.start_tick + tick) as f64) - onset, value]).collect())
}

fn curve_on_seconds_clock(note:&Note,key:&str,points:&[ExpressionPoint],tempo:&TempoMap,phrase_origin:f64,maximum_points:usize)->Result<Vec<[f64;2]>,usize>{
    use verdant_domain::editing::Interpolation;
    let mode=note.curves.interpolation(key);
    if mode==Interpolation::Linear{return linear_curve_on_seconds_clock(note,points,tempo,maximum_points);}
    if points.len()>maximum_points{return Err(points.len());}
    let onset=tempo.seconds_at(note.start_tick as f64);
    let mut knots=points.iter().map(|p|[tempo.seconds_at((note.start_tick+p.tick)as f64)-onset,p.value as f64]).collect::<Vec<_>>();
    let Some(first)=knots.first().copied()else{return Ok(knots);};let last=knots.last().copied().unwrap();
    if mode==Interpolation::Hold{
        // One representable predecessor keeps the authored jump abrupt even
        // when a worker evaluates between its regular 5 ms control frames.
        for pair in points.windows(2){let time=tempo.seconds_at((note.start_tick+pair[1].tick)as f64)-onset;if time>0.{let before=f64::from_bits(time.to_bits()-1);if before>tempo.seconds_at((note.start_tick+pair[0].tick)as f64)-onset{knots.push([before,pair[0].value as f64]);}}if knots.len()>maximum_points{return Err(knots.len());}}
    }else{
        // The admitted native expression clock is 5 ms. Sample nonlinear
        // tick-space curves at every frame on that exact phrase clock.
        let mut frame=((onset+first[0]-phrase_origin)/0.005).ceil().max(0.)as u64;
        loop{let absolute=phrase_origin+frame as f64*0.005;let relative=absolute-onset;if relative>=last[0]{break;}if relative>first[0]{let tick=tempo.tick_at(absolute)-note.start_tick as f64;let value=verdant_domain::editing::interpolate(points,tick,0.,mode,|p|p.tick,|p|p.value as f64);knots.push([relative,value]);if knots.len()>maximum_points{return Err(knots.len());}}frame+=1;}
    }
    knots.sort_by(|a,b|a[0].total_cmp(&b[0]));knots.dedup_by(|a,b|a[0]==b[0]);Ok(knots)
}

pub fn for_phrase(phrase: &Phrase, track: &Track, tempo: &TempoMap, maximum_points: usize) -> Result<Option<ExpressionTrack>, ExpressionCompileError> {
    let maximum_points = maximum_points.min(MAX_COMPILED_CURVE_POINTS);
    let mut segments = Vec::new();
    for id in &phrase.note_ids {
        let Some(note) = track.note(id) else { continue };
        let curves: BTreeMap<_, _> = note.curves.expression_points.iter()
            .filter(|(key, points)| key.as_str() != "POR" && !points.is_empty())
            .map(|(key, points)| curve_on_seconds_clock(note,key,points,tempo,phrase.start_seconds,maximum_points)
                .map(|points| (key.clone(), points))
                .map_err(|point_count| ExpressionCompileError { note_id: note.id.clone(), parameter: key.clone(), point_count, maximum_points }))
            .collect::<Result<_, _>>()?;
        if curves.is_empty() { continue; }
        let onset = tempo.seconds_at(note.start_tick as f64) - phrase.start_seconds;
        let end = tempo.seconds_at(note.end_tick() as f64) - phrase.start_seconds;
        let trace = phrase.score.phoneme_timing.as_ref().and_then(|t| t.notes.iter().find(|n| n.note_id == id.as_str()));
        let start = onset - trace.map_or(0., |n| n.prefix_seconds);
        let end = end - trace.map_or(0., |n| n.tail_trim_seconds);
        segments.push(ExpressionSegment { start_seconds: start.max(0.), end_seconds: end,
            note_onset_seconds: onset.max(0.), curves });
    }
    Ok((!segments.is_empty()).then_some(ExpressionTrack { method: "vocaloid-controls-v1".into(), segments }))
}
