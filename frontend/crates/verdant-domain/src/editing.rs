//! Ordinary note operations use the same atomic domain batches as gestures.
//! Draw previews stay outside the document; a zero-duration preview is armed
//! but cannot produce an insertion batch.
use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{Command, CommandBatch, Note, NoteId, PointId, Project, TrackId, Vibrato};
use crate::model::{NoteCurves, PitchPoint, GainPoint, ExpressionPoint};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation { #[default] Linear, Hold, Cosine, Cubic }

/// Update an uncommitted drawing in either horizontal direction. The caller
/// supplies snapped endpoints and the active grid step (one tick without snap).
/// Returning to the anchor disarms the insertion, even after a longer preview.
pub fn preview_note_draw(note: &mut Note, anchor: i64, endpoint: i64, minimum: i64) {
    let anchor = anchor.max(0);
    let endpoint = endpoint.max(0);
    note.start_tick = anchor.min(endpoint);
    note.duration_tick = if anchor == endpoint { 0 } else {
        anchor.abs_diff(endpoint).min(i64::MAX as u64) as i64
    };
    if note.duration_tick > 0 {
        note.duration_tick = note.duration_tick.max(minimum.max(1)).min(i64::MAX - note.start_tick);
    }
}

/// Consume the preview on release. Clicks and cancelled drawings have no
/// batch, so they cannot create a revision or an undo entry.
pub fn finish_note_draw(track: TrackId, note: Option<Note>) -> Option<CommandBatch> {
    let note = note.filter(|note| note.duration_tick > 0)?;
    Some(CommandBatch::single("绘制音符", Command::InsertNotes { track, notes: vec![note] }))
}

impl Note {
    pub fn is_muted(&self) -> bool { self.unknown.get("muted").and_then(Value::as_bool).unwrap_or(false) }
    pub fn set_muted(&mut self, muted: bool) { if muted { self.unknown.insert("muted".into(), Value::Bool(true)); } else { self.unknown.remove("muted"); } }
    pub fn is_continuation(&self) -> bool { self.unknown.get("continuation").and_then(Value::as_bool).unwrap_or(false) || matches!(self.lyric.trim(), "+"|"-") }
    pub fn set_continuation(&mut self, continuation: bool) { if continuation { self.unknown.insert("continuation".into(), Value::Bool(true)); } else { self.unknown.remove("continuation"); } }
    /// New objects never inherit a source object's semantic identity.
    pub fn regenerate_ids(&mut self) {
        self.id = NoteId::generate();
        for p in &mut self.curves.pitch_millicent_points { p.id = PointId::generate(); }
        for p in &mut self.curves.gain_millidecibel_points { p.id = PointId::generate(); }
        for p in self.curves.expression_points.values_mut().flatten() { p.id = PointId::generate(); }
    }
}
impl NoteCurves {
    pub fn interpolation(&self, parameter: &str) -> Interpolation {
        self.unknown.get("interpolation").and_then(|v| v.get(parameter)).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
    }
    pub fn set_interpolation(&mut self, parameter: &str, interpolation: Interpolation) {
        let value = self.unknown.entry("interpolation").or_insert_with(|| serde_json::json!({}));
        if let Some(map) = value.as_object_mut() { map.insert(parameter.into(), serde_json::to_value(interpolation).expect("interpolation enum")); }
    }
}
impl Vibrato {
    pub fn fade_in_permille(&self) -> u32 { self.unknown.get("fade_in_permille").and_then(Value::as_u64).unwrap_or(0).min(1000) as u32 }
    pub fn fade_out_permille(&self) -> u32 { self.unknown.get("fade_out_permille").and_then(Value::as_u64).unwrap_or(0).min(1000) as u32 }
    pub fn phase_millidegrees(&self) -> i64 { self.unknown.get("phase_millidegrees").and_then(Value::as_i64).unwrap_or(0).rem_euclid(360_000) }
    pub fn set_fades_phase(&mut self, fade_in: u32, fade_out: u32, phase: i64) {
        self.unknown.insert("fade_in_permille".into(), Value::from(fade_in));
        self.unknown.insert("fade_out_permille".into(), Value::from(fade_out));
        self.unknown.insert("phase_millidegrees".into(), Value::from(phase.rem_euclid(360_000)));
    }
}

/// Allocation-free interpolation shared by model display and audible controls.
/// Cubic is a monotone Hermite curve: bounded controllers cannot overshoot.
pub fn interpolate<P>(points: &[P], tick: f64, default: f64, mode: Interpolation, x: impl Fn(&P) -> i64, y: impl Fn(&P) -> f64) -> f64 {
    if points.is_empty() { return default; }
    let right = points.partition_point(|p| (x(p) as f64) <= tick);
    if right == 0 { return y(&points[0]); }
    if right >= points.len() { return y(points.last().expect("nonempty points")); }
    let left = right - 1; let a = &points[left]; let b = &points[right];
    let h = (x(b) - x(a)).max(1) as f64; let t = ((tick - x(a) as f64) / h).clamp(0., 1.);
    let (a, b) = (y(a), y(b));
    match mode {
        Interpolation::Hold => a,
        Interpolation::Linear => a + (b-a)*t,
        Interpolation::Cosine => a + (b-a)*(0.5-0.5*(std::f64::consts::PI*t).cos()),
        Interpolation::Cubic => {
            let slope = (b-a)/h;
            let tangent = |first: usize, second: usize| (y(&points[second])-y(&points[first])) / (x(&points[second])-x(&points[first])).max(1) as f64;
            let harmonic = |u: f64,v: f64| if u*v <= 0. { 0. } else { 2.*u*v/(u+v) };
            let m0 = if left == 0 { slope } else { harmonic(tangent(left-1,left),slope) };
            let m1 = if right+1 == points.len() { slope } else { harmonic(slope,tangent(right,right+1)) };
            let t2=t*t; let t3=t2*t;
            ((2.*t3-3.*t2+1.)*a + (t3-2.*t2+t)*h*m0 + (-2.*t3+3.*t2)*b + (t3-t2)*h*m1).clamp(a.min(b),a.max(b))
        }
    }
}

pub fn validate_note(note: &Note) -> Result<(), String> {
    for key in ["muted", "continuation"] {
        if note.unknown.get(key).is_some_and(|v| !v.is_boolean()) { return Err(format!("{key} must be a boolean")); }
    }
    if note.unknown.get("portamento_curve_percent").is_some_and(|v| v.as_i64().is_none_or(|v| !(-100..=100).contains(&v))) { return Err("portamento_curve_percent must be an integer in -100..=100".into()); }
    if let Some(v) = &note.vibrato {
        for key in ["fade_in_permille", "fade_out_permille"] {
            if v.unknown.get(key).is_some_and(|v| v.as_u64().is_none_or(|v| v > 1000)) { return Err(format!("{key} must be 0..1000")); }
        }
        if v.unknown.get("phase_millidegrees").is_some_and(|v| v.as_i64().is_none()) { return Err("vibrato phase must be integer millidegrees".into()); }
    }
    if let Some(value) = note.curves.unknown.get("interpolation") {
        let Some(map) = value.as_object() else { return Err("interpolation must be an object".into()) };
        for (parameter, value) in map {
            if parameter != "PIT" && parameter != "DYN" && crate::model::expression_default(parameter).is_none() { return Err(format!("unknown interpolation controller {parameter}")); }
            serde_json::from_value::<Interpolation>(value.clone()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Crop attached curves in absolute note-relative time, retaining interior
/// point identities and inserting evaluated boundary values where necessary.
pub fn crop_note(note: &Note, from: i64, to: i64, id: NoteId) -> Note {
    let from = from.clamp(0,note.duration_tick-1); let to = to.clamp(from+1,note.duration_tick);
    let mut out = note.clone(); out.id=id; out.start_tick += from; out.duration_tick=to-from;
    if !note.curves.pitch_millicent_points.is_empty() {
        let src=&note.curves.pitch_millicent_points; let mode=note.curves.interpolation("PIT");
        let mut points=vec![PitchPoint { id: PointId::generate(), tick:0, millicents:interpolate(src,from as f64,0.,mode,|p|p.tick,|p|p.millicents as f64).round() as i64, unknown:Default::default() }];
        points.extend(src.iter().filter(|p| p.tick>from&&p.tick<to).cloned().map(|mut p|{p.tick-=from;p}));
        points.push(PitchPoint { id:PointId::generate(),tick:to-from,millicents:interpolate(src,to as f64,0.,mode,|p|p.tick,|p|p.millicents as f64).round() as i64,unknown:Default::default() }); out.curves.pitch_millicent_points=points;
    }
    if !note.curves.gain_millidecibel_points.is_empty() {
        let src=&note.curves.gain_millidecibel_points; let mode=note.curves.interpolation("DYN");
        let mut points=vec![GainPoint{id:PointId::generate(),tick:0,millidecibels:interpolate(src,from as f64,0.,mode,|p|p.tick,|p|p.millidecibels as f64).round() as i64,unknown:Default::default()}];
        points.extend(src.iter().filter(|p|p.tick>from&&p.tick<to).cloned().map(|mut p|{p.tick-=from;p}));
        points.push(GainPoint{id:PointId::generate(),tick:to-from,millidecibels:interpolate(src,to as f64,0.,mode,|p|p.tick,|p|p.millidecibels as f64).round() as i64,unknown:Default::default()});out.curves.gain_millidecibel_points=points;
    }
    for (parameter,src) in &note.curves.expression_points {
        if src.is_empty(){continue;} let mode=note.curves.interpolation(parameter);
        let value=|tick|interpolate(src,tick,crate::model::expression_default(parameter).unwrap_or(0) as f64,mode,|p|p.tick,|p|p.value as f64).round().clamp(0.,127.) as u8;
        let mut points=vec![ExpressionPoint{id:PointId::generate(),tick:0,value:value(from as f64),unknown:Default::default()}];
        points.extend(src.iter().filter(|p|p.tick>from&&p.tick<to).cloned().map(|mut p|{p.tick-=from;p}));
        points.push(ExpressionPoint{id:PointId::generate(),tick:to-from,value:value(to as f64),unknown:Default::default()});out.curves.expression_points.insert(parameter.clone(),points);
    }
    out
}

pub fn split(project: &Project, track: &TrackId, ids: &[NoteId], tick: i64) -> Result<(CommandBatch, Vec<NoteId>), String> {
    let track_ref=project.track(track).ok_or("track no longer exists")?; let mut commands=Vec::new();let mut selection=Vec::new();
    for id in ids {
        let n=track_ref.note(id).ok_or("note no longer exists")?;
        if tick<=n.start_tick || tick>=n.end_tick(){continue;}
        let left=crop_note(n,0,tick-n.start_tick,n.id.clone());
        let mut right=crop_note(n,tick-n.start_tick,n.duration_tick,NoteId::generate());right.lyric="+".into();right.set_continuation(true);
        selection.extend([left.id.clone(),right.id.clone()]);
        commands.push(Command::ReplaceNotes{track:track.clone(),notes:vec![left]});commands.push(Command::InsertNotes{track:track.clone(),notes:vec![right]});
    }
    if commands.is_empty(){return Err("split position must be inside a selected note".into());}
    Ok((CommandBatch{label:"分割音符".into(),commands},selection))
}

pub fn join(project: &Project, track: &TrackId, ids: &[NoteId]) -> Result<(CommandBatch,NoteId),String> {
    let t=project.track(track).ok_or("track no longer exists")?;let selected:BTreeSet<_>=ids.iter().collect();
    let notes:Vec<_>=t.notes.iter().filter(|n|selected.contains(&n.id)).collect();if notes.len()<2{return Err("select at least two notes".into());}
    let first=notes[0];let mut joined=first.clone();let end=notes.last().expect("notes").end_tick();
    for pair in notes.windows(2){if pair[0].end_tick()!=pair[1].start_tick{return Err("join requires adjacent notes; close gaps or remove overlaps first".into());}}
    if notes.iter().any(|n| (!n.is_continuation()&&n.lyric!=first.lyric)||n.pronunciation!=first.pronunciation||n.vibrato!=first.vibrato||n.is_muted()!=first.is_muted()) {
        return Err("join requires one syllable with matching pronunciation, vibrato and mute state; use + for its continuation".into());
    }
    joined.duration_tick=end-first.start_tick; joined.curves.pitch_millicent_points.clear();joined.curves.gain_millidecibel_points.clear();joined.curves.expression_points.clear();
    let expression_keys:BTreeSet<_>=notes.iter().flat_map(|n|n.curves.expression_points.keys().cloned()).collect();
    for n in &notes{
        let offset=n.start_tick-first.start_tick;let pitch=(n.midi_note-first.midi_note) as i64*100_000+i64::from(n.detune_millicents-first.detune_millicents);
        let mut pit=n.curves.pitch_millicent_points.clone();if pit.is_empty(){pit.push(PitchPoint{id:PointId::generate(),tick:0,millicents:0,unknown:Default::default()});}
        for mut p in pit{p.tick+=offset;p.millicents+=pitch;joined.curves.pitch_millicent_points.push(p);}
        let mut dyn_=n.curves.gain_millidecibel_points.clone();if dyn_.is_empty(){dyn_.push(GainPoint{id:PointId::generate(),tick:0,millidecibels:0,unknown:Default::default()});}
        for mut p in dyn_{p.tick+=offset;joined.curves.gain_millidecibel_points.push(p);}
        for key in &expression_keys{
            let mut points=n.curves.expression(key).to_vec();if points.is_empty(){points.push(ExpressionPoint{id:PointId::generate(),tick:0,value:crate::model::expression_default(key).unwrap_or(0),unknown:Default::default()});}
            joined.curves.expression_points.entry(key.clone()).or_default().extend(points.into_iter().map(|mut p|{p.tick+=offset;p}));
        }
    }
    joined.curves.pitch_millicent_points=crate::command::normalize_points(joined.curves.pitch_millicent_points,|p|p.tick);
    joined.curves.gain_millidecibel_points=crate::command::normalize_points(joined.curves.gain_millidecibel_points,|p|p.tick);
    for points in joined.curves.expression_points.values_mut(){*points=crate::command::normalize_points(std::mem::take(points),|p|p.tick);}
    Ok((CommandBatch{label:"连接音符".into(),commands:vec![Command::DeleteNotes{track:track.clone(),note_ids:notes.iter().skip(1).map(|n|n.id.clone()).collect()},Command::ReplaceNotes{track:track.clone(),notes:vec![joined]}]},first.id.clone()))
}

pub fn quantize(project:&Project,track:&TrackId,ids:&[NoteId],grid:i64)->Result<CommandBatch,String>{
    if grid<1{return Err("grid must be positive".into());}let t=project.track(track).ok_or("track no longer exists")?;
    let meter=crate::MeterMap::new(&project.time_signatures,project.ppq)?;
    let snap=|tick:i64| { let origin=meter.signature_at(tick).tick;origin+(((tick-origin)as i128+grid as i128/2)/grid as i128*grid as i128).min((i64::MAX-origin)as i128)as i64 };
    let commands=ids.iter().map(|id|{let n=t.note(id).ok_or("note no longer exists")?;let start=snap(n.start_tick);let end=snap(n.end_tick()).max(start.saturating_add(1));Ok(Command::SetTiming{track:track.clone(),note_id:id.clone(),start_tick:start,duration_tick:end-start})}).collect::<Result<_,String>>()?;
    Ok(CommandBatch{label:"量化音符".into(),commands})
}

pub fn cleanup(project:&Project,track:&TrackId,ids:&[NoteId],max_gap:i64,overlaps:bool)->Result<CommandBatch,String>{
    let t=project.track(track).ok_or("track no longer exists")?;let selected:BTreeSet<_>=ids.iter().collect();let mut commands=Vec::new();
    for pair in t.notes.windows(2){let (a,b)=(&pair[0],&pair[1]);if !selected.contains(&a.id){continue;}let gap=b.start_tick-a.end_tick();
        if (overlaps&&gap<0)||(!overlaps&&gap>0&&gap<=max_gap){let duration=b.start_tick-a.start_tick;if duration<1{return Err("coincident starts cannot be cleaned without deleting a note".into());}
            commands.push(Command::SetTiming{track:track.clone(),note_id:a.id.clone(),start_tick:a.start_tick,duration_tick:duration});}
    }
    Ok(CommandBatch{label:if overlaps{"消除重叠"}else{"闭合短间隙"}.into(),commands})
}
