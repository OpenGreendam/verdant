//! Serializable, atomic audio editing shared by the GUI, CLI and worker.
//! Planning validates the complete operation against one project snapshot.
//! It never mutates a project, source file or output directory. Bounce assets
//! are encoded in memory and published only by the explicit commit helper.
use std::{collections::{BTreeSet, HashMap}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use verdant_domain::{Command, CommandBatch, Project, TempoMap, TrackId};
use verdant_media::{AUDIO_CLIPS_KEY, AssetPolicy, AudioClip, AudioSource, DecodedAudio, FadeShape,
    ImportLimits, MediaImporter, clips_value, decode_bytes, prepare_clip, read_clips};

const CLIPBOARD_SCHEMA: &str = "verdant-audio-clipboard/1";
const MAX_EDIT_ITEMS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClipRef { pub track: TrackId, pub id: Uuid }
impl From<&AudioClip> for AudioClipRef {
    fn from(clip: &AudioClip) -> Self { Self { track: clip.track.clone(), id: clip.id } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipboardClip { pub clip: AudioClip, pub track_offset: usize, pub offset_seconds: f64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClipboard { pub schema: String, pub clips: Vec<ClipboardClip> }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClipProperties {
    pub name: Option<String>,
    pub gain_db: Option<f64>,
    pub pan: Option<f32>,
    pub muted: Option<bool>,
    pub reversed: Option<bool>,
    pub playback_rate: Option<f64>,
    pub preserve_pitch: Option<bool>,
    pub fade_in_ms: Option<f64>,
    pub fade_out_ms: Option<f64>,
    pub fade_shape: Option<FadeShape>,
}

/// Ticks may be fractional. Source trims and slip offsets are decoded frames.
/// Track offsets are relative to the project's visible track order.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioEdit {
    Batch { edits: Vec<AudioEdit> },
    Split { clips: Vec<AudioClipRef>, tick: f64 },
    SplitRange { tracks: Vec<TrackId>, start_tick: f64, end_tick: f64 },
    Trim { clip: AudioClipRef, source_offset_frames: u64, duration_frames: u64, start_tick: Option<f64> },
    TrimToRange { clips: Vec<AudioClipRef>, start_tick: f64, end_tick: f64 },
    Slip { clips: Vec<AudioClipRef>, delta_frames: i64 },
    Move { clips: Vec<AudioClipRef>, delta_ticks: f64, #[serde(default)] track_offset: i32 },
    Duplicate { clips: Vec<AudioClipRef>, at_tick: f64, target_track: Option<TrackId> },
    Delete { clips: Vec<AudioClipRef>, #[serde(default)] ripple: bool },
    DeleteRange { tracks: Vec<TrackId>, start_tick: f64, end_tick: f64, #[serde(default)] ripple: bool },
    Copy { clips: Vec<AudioClipRef> },
    CopyRange { tracks: Vec<TrackId>, start_tick: f64, end_tick: f64 },
    Paste { clipboard: AudioClipboard, at_tick: f64, target_track: TrackId },
    SetProperties { clips: Vec<AudioClipRef>, properties: AudioClipProperties },
    Join { clips: Vec<AudioClipRef> },
    Crossfade { left: AudioClipRef, right: AudioClipRef, #[serde(default)] shape: FadeShape },
    SplitSilence { clips: Vec<AudioClipRef>, threshold_db: f64, min_silence_ms: f64 },
    SplitZeroCrossing { clip: AudioClipRef, tick: f64, window_ms: f64 },
    Normalize { clips: Vec<AudioClipRef>, target_db: f64, #[serde(default)] independent: bool },
    /// Sum selected clip processing into a new float WAV. Track/master inserts
    /// are deliberately outside this clip operation; use mix/export for those.
    Bounce { clips: Vec<AudioClipRef>, path: String, sample_rate: u32, #[serde(default)] replace: bool },
}

#[derive(Clone, Debug, Serialize)]
pub struct PendingAudioAsset {
    pub path: PathBuf, pub sha256: String, pub byte_len: u64,
    #[serde(skip)] pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AudioEditPlan {
    pub batch: CommandBatch,
    pub selection: Vec<AudioClipRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard: Option<AudioClipboard>,
    pub pending_assets: Vec<PendingAudioAsset>,
}

pub fn plan_audio_edit(project: &Project, edit: &AudioEdit) -> Result<AudioEditPlan, String> {
    plan(project, edit, None)
}
pub fn plan_audio_edit_with_media(project: &Project, edit: &AudioEdit, policy: AssetPolicy) -> Result<AudioEditPlan, String> {
    plan(project, edit, Some(MediaImporter::new(policy, ImportLimits::default())))
}
fn plan(project: &Project, edit: &AudioEdit, importer: Option<MediaImporter>) -> Result<AudioEditPlan, String> {
    let tracks = project.tracks.iter().map(read_clips).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    let original = tracks.clone();
    let mut ids = BTreeSet::new();
    for clip in tracks.iter().flatten() { if !ids.insert(clip.id) { return Err("duplicate project audio clip ID".into()); } }
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).map_err(|e| e.to_string())?;
    let mut state = EditState { project, tempo, tracks, importer, decoded: HashMap::new(), verified_sources:HashMap::new(), decoded_bytes: 0, selection: Vec::new(), clipboard: None, assets: Vec::new() };
    state.apply(edit, 0)?;
    let mut commands = Vec::new();
    let mut all_ids = BTreeSet::new();
    for (index, clips) in state.tracks.iter().enumerate() {
        for clip in clips { if !all_ids.insert(clip.id) { return Err("duplicate edited audio clip ID".into()); } }
        let track = &project.tracks[index].id;
        let value = clips_value(track.clone(), clips).map_err(|e| e.to_string())?;
        if *clips != original[index] { commands.push(Command::SetTrackData { track: track.clone(), key: AUDIO_CLIPS_KEY.into(), value }); }
    }
    state.selection.retain(|reference| state.tracks.iter().flatten().any(|clip| clip.id == reference.id && clip.track == reference.track));
    state.selection.sort(); state.selection.dedup();
    Ok(AudioEditPlan { batch: CommandBatch { label: "编辑音频片段".into(), commands }, selection: state.selection,
        clipboard: state.clipboard, pending_assets: state.assets })
}

struct EditState<'a> {
    project: &'a Project, tempo: TempoMap, tracks: Vec<Vec<AudioClip>>, importer: Option<MediaImporter>,
    decoded: HashMap<(String, String), DecodedAudio>, verified_sources:HashMap<(String,String),AudioSource>, decoded_bytes: usize, selection: Vec<AudioClipRef>,
    clipboard: Option<AudioClipboard>, assets: Vec<PendingAudioAsset>,
}
impl EditState<'_> {
    fn track_index(&self, track: &TrackId) -> Result<usize, String> {
        self.project.tracks.iter().position(|item| &item.id == track).ok_or_else(|| "unknown audio track".into())
    }
    fn get(&self, reference: &AudioClipRef) -> Result<AudioClip, String> {
        self.tracks[self.track_index(&reference.track)?].iter().find(|clip| clip.id == reference.id).cloned().ok_or_else(|| "unknown audio clip ID".into())
    }
    fn selected(&self, refs: &[AudioClipRef]) -> Result<Vec<AudioClip>, String> {
        if refs.is_empty() || refs.len() > MAX_EDIT_ITEMS { return Err("audio selection must contain 1..100000 clips".into()); }
        let mut ids = BTreeSet::new();
        refs.iter().map(|reference| {
            if !ids.insert(reference.clone()) { return Err("duplicate clip in selection".into()); }
            self.get(reference)
        }).collect()
    }
    fn track_indices(&self, tracks: &[TrackId]) -> Result<Vec<usize>, String> {
        if tracks.is_empty() { return Err("audio range requires explicit tracks".into()); }
        let indices = tracks.iter().map(|track| self.track_index(track)).collect::<Result<Vec<_>, _>>()?;
        if indices.iter().collect::<BTreeSet<_>>().len() != indices.len() { return Err("duplicate track in range".into()); }
        Ok(indices)
    }
    fn replace(&mut self, old: &AudioClip, clips: Vec<AudioClip>) -> Result<(), String> {
        let index = self.track_index(&old.track)?;
        self.tracks[index].retain(|clip| clip.id != old.id);
        for clip in clips {
            clip.validate().map_err(|e| e.to_string())?;
            let index = self.track_index(&clip.track)?;
            self.selection.push(AudioClipRef::from(&clip));
            self.tracks[index].push(clip);
        }
        Ok(())
    }
    fn load(&mut self, clip: &AudioClip) -> Result<DecodedAudio, String> {
        let key = (clip.source.path.clone(), clip.source.sha256.clone());
        if let Some(audio) = self.decoded.get(&key) { clip.source.matches(audio).map_err(|e| e.to_string())?; return Ok(audio.clone()); }
        let importer = self.importer.as_ref().ok_or("this edit requires an authorized media policy")?;
        let estimate=clip.source.frames.checked_mul(u64::from(clip.source.channels)).and_then(|n|n.checked_mul(4)).and_then(|n|usize::try_from(n).ok()).ok_or("edit PCM size overflow")?;
        let bytes=self.decoded_bytes.checked_add(estimate).filter(|bytes|*bytes<=ImportLimits::default().max_project_pcm_bytes).ok_or("audio edit exceeds project PCM limit")?;
        let (source,audio)=if let Some(asset)=self.assets.iter().find(|asset|asset.path==Path::new(&clip.source.path)&&asset.sha256==clip.source.sha256){
            let audio=decode_bytes(&asset.bytes,Some("wav"),ImportLimits::default()).map_err(|e|e.to_string())?;
            (clip.source.clone(),audio)
        }else{let imported=importer.import_file(Path::new(&clip.source.path)).map_err(|e|e.to_string())?;(imported.source,imported.audio)};
        clip.source.matches(&audio).map_err(|e|e.to_string())?;
        self.decoded_bytes=bytes;
        self.verified_sources.insert(key.clone(),source);
        self.decoded.insert(key, audio.clone()); Ok(audio)
    }
    fn range(&self, start: f64, end: f64) -> Result<(f64, f64), String> {
        valid_tick(start)?; valid_tick(end)?;
        if start >= end { return Err("audio range must have positive duration".into()); }
        Ok((self.tempo.seconds_at(start), self.tempo.seconds_at(end)))
    }
    fn slice_seconds(&self, clip: &AudioClip, start: f64, end: f64) -> Result<Option<AudioClip>, String> {
        let from = start.max(clip.start_seconds(&self.tempo));
        let to = end.min(clip.end_seconds(&self.tempo));
        if from >= to { return Ok(None); }
        let scale = clip.source.sample_rate as f64 * clip.playback_rate;
        let begin = ((from - clip.start_seconds(&self.tempo)) * scale).round().clamp(0.0, clip.duration_frames as f64) as u64;
        let finish = ((to - clip.start_seconds(&self.tempo)) * scale).round().clamp(0.0, clip.duration_frames as f64) as u64;
        if begin >= finish { return Ok(None); }
        clip.slice_frames(begin, finish, &self.tempo).map(Some).map_err(|e| e.to_string())
    }
    fn clipboard_for(&mut self, clips: &[AudioClip], origin_seconds: Option<f64>) -> Result<AudioClipboard, String> {
        if clips.is_empty() { return Err("audio selection contains no source frames".into()); }
        let origin = origin_seconds.unwrap_or_else(|| clips.iter().map(|clip| clip.start_seconds(&self.tempo)).fold(f64::INFINITY, f64::min));
        let first = clips.iter().map(|clip| self.track_index(&clip.track)).collect::<Result<Vec<_>, _>>()?.into_iter().min().unwrap();
        let mut result=Vec::with_capacity(clips.len());
        for clip in clips{
            let mut copied=clip.clone();
            if self.importer.is_some(){
                self.load(clip)?;
                copied.source=self.verified_sources.get(&(clip.source.path.clone(),clip.source.sha256.clone()))
                    .cloned().ok_or("verified clipboard source is unavailable")?;
            }
            result.push(ClipboardClip{clip:copied,track_offset:self.track_index(&clip.track)?-first,
                offset_seconds:(clip.start_seconds(&self.tempo)-origin).max(0.)});
        }
        Ok(AudioClipboard { schema: CLIPBOARD_SCHEMA.into(), clips:result })
    }
    fn paste(&mut self, clipboard: &AudioClipboard, tick: f64, target: &TrackId) -> Result<(), String> {
        valid_tick(tick)?;
        if clipboard.schema != CLIPBOARD_SCHEMA || clipboard.clips.is_empty() || clipboard.clips.len() > MAX_EDIT_ITEMS {
            return Err("invalid audio clipboard".into());
        }
        let first = self.track_index(target)?;
        let start = self.tempo.seconds_at(tick);
        let mut seen = BTreeSet::new();
        for item in &clipboard.clips {
            item.clip.validate().map_err(|e| e.to_string())?;
            if !item.offset_seconds.is_finite() || item.offset_seconds < 0.0 || !seen.insert(item.clip.id) { return Err("invalid clipboard offset or duplicate clip".into()); }
            let index = first.checked_add(item.track_offset).filter(|index| *index < self.tracks.len()).ok_or("clipboard exceeds destination tracks")?;
            let mut clip = item.clip.duplicate_at(self.tempo.tick_at(start + item.offset_seconds)).map_err(|e| e.to_string())?;
            if self.importer.is_some(){
                self.load(&item.clip)?;
                clip.source=self.verified_sources.get(&(item.clip.source.path.clone(),item.clip.source.sha256.clone()))
                    .cloned().ok_or("verified pasted source is unavailable")?;
            }else if !self.project.tracks.iter().filter_map(|track|read_clips(track).ok()).flatten().any(|known|known.source==item.clip.source){
                return Err("pasting a new audio source requires an authorized media policy".into());
            }
            clip.track = self.project.tracks[index].id.clone();
            self.selection.push(AudioClipRef::from(&clip)); self.tracks[index].push(clip);
        }
        Ok(())
    }
    fn apply(&mut self, edit: &AudioEdit, depth: usize) -> Result<(), String> {
        if depth > 16 { return Err("audio edit batch nesting limit".into()); }
        self.selection.clear();
        match edit {
            AudioEdit::Batch { edits } => {
                if edits.is_empty() || edits.len() > 1000 { return Err("audio batch must contain 1..1000 edits".into()); }
                for edit in edits { self.apply(edit, depth + 1)?; }
            }
            AudioEdit::Split { clips, tick } => {
                valid_tick(*tick)?;
                let selected=self.selected(clips)?;let mut count=0;
                for clip in selected {
                    if *tick<=clip.tick()||*tick>=clip.end_tick(&self.tempo){continue;}
                    let frame=clip.frame_at_tick(*tick,&self.tempo).map_err(|e|e.to_string())?;
                    if frame==0||frame>=clip.duration_frames{continue;}
                    let (left, right) = clip.split_at(*tick, &self.tempo).map_err(|e| e.to_string())?;
                    self.replace(&clip, vec![left, right])?;count+=1;
                }
                if count==0{return Err("split must be inside a selected clip".into());}
            }
            AudioEdit::SplitRange { tracks, start_tick, end_tick } => {
                self.range(*start_tick, *end_tick)?;
                for index in self.track_indices(tracks)? {
                    for tick in [*start_tick, *end_tick] {
                        for clip in self.tracks[index].clone() {
                            if tick > clip.tick() && tick < clip.end_tick(&self.tempo) {
                                let frame = clip.frame_at_tick(tick, &self.tempo).map_err(|e| e.to_string())?;
                                if frame > 0 && frame < clip.duration_frames {
                                    let (left, right) = clip.split_at(tick, &self.tempo).map_err(|e| e.to_string())?;
                                    self.replace(&clip, vec![left, right])?;
                                }
                            }
                        }
                    }
                }
            }
            AudioEdit::Trim { clip, source_offset_frames, duration_frames, start_tick } => {
                let old = self.get(clip)?;
                let trimmed = old.trim_source(*source_offset_frames, *duration_frames, start_tick.unwrap_or(old.tick()), &self.tempo).map_err(|e| e.to_string())?;
                self.replace(&old, vec![trimmed])?;
            }
            AudioEdit::TrimToRange { clips, start_tick, end_tick } => {
                let (start, end) = self.range(*start_tick, *end_tick)?;
                for clip in self.selected(clips)? {
                    let trimmed = self.slice_seconds(&clip, start, end)?.ok_or("selected clip does not intersect the range")?;
                    self.replace(&clip, vec![trimmed])?;
                }
            }
            AudioEdit::Slip { clips, delta_frames } => {
                for clip in self.selected(clips)? {
                    let mut slipped = clip.clone();
                    slipped.source_offset_frames = u64::try_from(i128::from(clip.source_offset_frames) + i128::from(*delta_frames)).map_err(|_| "slip exceeds source")?;
                    slipped.validate().map_err(|e| e.to_string())?;
                    self.replace(&clip, vec![slipped])?;
                }
            }
            AudioEdit::Move { clips, delta_ticks, track_offset } => {
                if !delta_ticks.is_finite() { return Err("non-finite move offset".into()); }
                for clip in self.selected(clips)? {
                    let index = i64::try_from(self.track_index(&clip.track)?).map_err(|_| "track index overflow")? + i64::from(*track_offset);
                    if index < 0 || index >= self.tracks.len() as i64 { return Err("move exceeds destination tracks".into()); }
                    let mut moved = clip.clone(); moved.set_tick(clip.tick() + delta_ticks).map_err(|e| e.to_string())?;
                    moved.track = self.project.tracks[index as usize].id.clone();
                    self.replace(&clip, vec![moved])?;
                }
            }
            AudioEdit::Duplicate { clips, at_tick, target_track } => {
                let selected = self.selected(clips)?;
                let first = selected.iter().map(|clip| self.track_index(&clip.track)).collect::<Result<Vec<_>, _>>()?.into_iter().min().unwrap();
                let target = target_track.clone().unwrap_or_else(|| self.project.tracks[first].id.clone());
                let clipboard = self.clipboard_for(&selected, None)?;
                self.paste(&clipboard, *at_tick, &target)?;
            }
            AudioEdit::Delete { clips, ripple } => {
                let selected = self.selected(clips)?;
                let ids: BTreeSet<_> = selected.iter().map(|clip| clip.id).collect();
                if *ripple {
                    for index in 0..self.tracks.len() {
                        let mut ranges: Vec<_> = selected.iter().filter(|clip| clip.track == self.project.tracks[index].id)
                            .map(|clip| (clip.start_seconds(&self.tempo), clip.end_seconds(&self.tempo))).collect();
                        ranges.sort_by(|a,b| a.0.total_cmp(&b.0));
                        let mut merged: Vec<(f64,f64)> = Vec::new();
                        for range in ranges {
                            if let Some(last) = merged.last_mut().filter(|last| range.0 <= last.1) { last.1 = last.1.max(range.1); } else { merged.push(range); }
                        }
                        for clip in &mut self.tracks[index] {
                            if ids.contains(&clip.id) { continue; }
                            let start = clip.start_seconds(&self.tempo);
                            if merged.iter().any(|(a,b)| start < *b && clip.end_seconds(&self.tempo) > *a) {
                                return Err("ripple deletion overlaps an unselected clip; use delete_range for a shared time cut".into());
                            }
                            let shift: f64 = merged.iter().filter(|(_,end)| *end <= start + 1e-10).map(|(begin,end)| end-begin).sum();
                            clip.set_tick(self.tempo.tick_at((start-shift).max(0.0))).map_err(|e| e.to_string())?;
                        }
                    }
                }
                for track in &mut self.tracks { track.retain(|clip| !ids.contains(&clip.id)); }
            }
            AudioEdit::DeleteRange { tracks, start_tick, end_tick, ripple } => {
                let (start, end) = self.range(*start_tick, *end_tick)?;
                for index in self.track_indices(tracks)? {
                    let mut output = Vec::new();
                    for clip in self.tracks[index].clone() {
                        let mut pieces = Vec::new();
                        if clip.end_seconds(&self.tempo) <= start || clip.start_seconds(&self.tempo) >= end { pieces.push(clip.clone()); }
                        else {
                            if let Some(left) = self.slice_seconds(&clip, clip.start_seconds(&self.tempo), start)? { pieces.push(left); }
                            if let Some(mut right) = self.slice_seconds(&clip, end, clip.end_seconds(&self.tempo))? {
                                if !pieces.is_empty() { right.id = Uuid::new_v4(); } pieces.push(right);
                            }
                        }
                        for mut piece in pieces {
                            if *ripple && piece.start_seconds(&self.tempo) >= end - 0.5 / (piece.source.sample_rate as f64 * piece.playback_rate) {
                                piece.set_tick(self.tempo.tick_at((piece.start_seconds(&self.tempo) - (end - start)).max(0.0))).map_err(|e| e.to_string())?;
                            }
                            output.push(piece);
                        }
                    }
                    self.tracks[index] = output;
                }
            }
            AudioEdit::Copy { clips } => {
                let clips = self.selected(clips)?; self.clipboard = Some(self.clipboard_for(&clips, None)?);
                self.selection = clips.iter().map(AudioClipRef::from).collect();
            }
            AudioEdit::CopyRange { tracks, start_tick, end_tick } => {
                let (start,end) = self.range(*start_tick,*end_tick)?;
                let mut selected = Vec::new();
                for index in self.track_indices(tracks)? { for clip in &self.tracks[index] {
                    if let Some(slice) = self.slice_seconds(clip,start,end)? { selected.push(slice); }
                }}
                self.clipboard = Some(self.clipboard_for(&selected, Some(start))?);
                self.selection = selected.iter().map(AudioClipRef::from).collect();
            }
            AudioEdit::Paste { clipboard, at_tick, target_track } => self.paste(clipboard,*at_tick,target_track)?,
            AudioEdit::SetProperties { clips, properties } => {
                for clip in self.selected(clips)? {
                    let mut next = clip.clone();
                    if let Some(name) = &properties.name { next.name = name.clone(); }
                    if let Some(db) = properties.gain_db { finite_between(db,-120.0,36.0,"gain dB")?; next.gain = 10.0_f64.powf(db / 20.0) as f32; }
                    if let Some(pan) = properties.pan { next.pan = pan; }
                    if let Some(value) = properties.muted { next.muted = value; }
                    if let Some(value) = properties.reversed { next.reversed = value; }
                    if let Some(value) = properties.playback_rate { next.playback_rate = value; }
                    if let Some(value) = properties.preserve_pitch { next.preserve_pitch = value; }
                    if let Some(value) = properties.fade_shape { next.fade_shape = value; }
                    if properties.fade_in_ms.is_some() || properties.fade_out_ms.is_some() {
                        next.fade_window = None;
                        let frames = |value: f64| -> Result<u64, String> {
                            finite_between(value,0.0,3_600_000.0,"fade duration")?;
                            Ok((value * next.source.sample_rate as f64 * next.playback_rate / 1000.0).round() as u64)
                        };
                        if let Some(value) = properties.fade_in_ms { next.fade_in_frames = frames(value)?; }
                        if let Some(value) = properties.fade_out_ms { next.fade_out_frames = frames(value)?; }
                    }
                    next.validate().map_err(|e| e.to_string())?; self.replace(&clip,vec![next])?;
                }
            }
            AudioEdit::Join { clips } => {
                let mut clips = self.selected(clips)?;
                if clips.len() < 2 { return Err("join requires at least two clips".into()); }
                clips.sort_by(|a,b| a.tick().total_cmp(&b.tick()));
                let mut joined = clips[0].clone();
                for clip in &clips[1..] { joined = joined.join_right(clip,&self.tempo).map_err(|e| e.to_string())?; }
                for clip in &clips { self.replace(clip,Vec::new())?; }
                let index = self.track_index(&joined.track)?; self.selection.push(AudioClipRef::from(&joined)); self.tracks[index].push(joined);
            }
            AudioEdit::Crossfade { left,right,shape } => {
                let (a,b) = (self.get(left)?,self.get(right)?);
                if a.id == b.id || a.track != b.track || b.start_seconds(&self.tempo) < a.start_seconds(&self.tempo) {
                    return Err("crossfade requires distinct ordered clips on the same track".into());
                }
                let overlap = a.end_seconds(&self.tempo) - b.start_seconds(&self.tempo);
                if overlap <= 0.0 || overlap > b.duration_seconds() { return Err("crossfade requires an overlap at both clip edges".into()); }
                let (mut aa,mut bb) = (a.clone(),b.clone());
                aa.fade_window = None; bb.fade_window = None; aa.fade_shape = *shape; bb.fade_shape = *shape;
                aa.fade_out_frames = (overlap * aa.source.sample_rate as f64 * aa.playback_rate).round() as u64;
                bb.fade_in_frames = (overlap * bb.source.sample_rate as f64 * bb.playback_rate).round() as u64;
                self.replace(&a,vec![aa])?; self.replace(&b,vec![bb])?;
            }
            AudioEdit::SplitSilence { clips,threshold_db,min_silence_ms } => {
                finite_between(*threshold_db,-120.0,0.0,"silence threshold")?;
                finite_between(*min_silence_ms,0.1,3_600_000.0,"minimum silence")?;
                let threshold = 10.0_f64.powf(threshold_db / 20.0) as f32;
                for clip in self.selected(clips)? {
                    let audio = self.load(&clip)?;
                    let prepared = prepare_clip(&clip,&audio,&self.tempo,clip.source.sample_rate,0).map_err(|e|e.to_string())?;
                    let minimum = (min_silence_ms * clip.source.sample_rate as f64 / 1000.0).ceil() as usize;
                    let mut start = None; let mut cuts = vec![0u64];
                    for (i,frame) in prepared.samples.chunks_exact(prepared.channels as usize).enumerate() {
                        let quiet = frame.iter().all(|sample| sample.abs() * prepared.gain <= threshold);
                        if quiet && start.is_none() { start=Some(i); }
                        if !quiet {
                            if let Some(begin) = start.take() {
                                if i-begin >= minimum && begin > 0 {
                                    let cut = (((begin+i) as f64 * 0.5) * clip.playback_rate).round() as u64;
                                    if cut > *cuts.last().unwrap() && cut < clip.duration_frames { cuts.push(cut); }
                                }
                            }
                        }
                        if cuts.len() > MAX_EDIT_ITEMS { return Err("silence split exceeds clip limit".into()); }
                    }
                    cuts.push(clip.duration_frames);
                    let mut pieces=Vec::new();
                    for pair in cuts.windows(2) {
                        let mut piece=clip.slice_frames(pair[0],pair[1],&self.tempo).map_err(|e|e.to_string())?;
                        if !pieces.is_empty() { piece.id=Uuid::new_v4(); } pieces.push(piece);
                    }
                    self.replace(&clip,pieces)?;
                }
            }
            AudioEdit::SplitZeroCrossing { clip,tick,window_ms } => {
                finite_between(*window_ms,0.01,1000.0,"zero crossing search window")?;
                let clip=self.get(clip)?; let audio=self.load(&clip)?;
                let at=clip.frame_at_tick(*tick,&self.tempo).map_err(|e|e.to_string())?;
                let radius=(window_ms*clip.source.sample_rate as f64*clip.playback_rate/1000.0).ceil() as u64;
                let low=at.saturating_sub(radius).max(1); let high=at.saturating_add(radius).min(clip.duration_frames.saturating_sub(1));
                let sample=|frame:u64,c:usize| {
                    let source=clip.source_offset_frames+if clip.reversed {clip.duration_frames-1-frame}else{frame};
                    audio.samples[source as usize*audio.channels as usize+c]
                };
                let mut best=None;
                for frame in low..=high {
                    let mut crossing=false; let mut energy=0.0_f32;
                    for c in 0..audio.channels as usize {
                        let (a,b)=(sample(frame-1,c),sample(frame,c)); crossing |= a==0.0||b==0.0||a.signum()!=b.signum();
                        energy=energy.max(a.abs().min(b.abs()));
                    }
                    if crossing {
                        let score=energy as f64 + frame.abs_diff(at) as f64 / radius.max(1) as f64 * 1e-5;
                        if best.is_none_or(|(_,value)| score<value) {best=Some((frame,score));}
                    }
                }
                let frame=best.ok_or("no zero crossing within the requested window")?.0;
                let (left,right)=clip.split_at(clip.tick_at_frame(frame,&self.tempo),&self.tempo).map_err(|e|e.to_string())?;
                self.replace(&clip,vec![left,right])?;
            }
            AudioEdit::Normalize { clips,target_db,independent } => {
                finite_between(*target_db,-120.0,0.0,"normalization peak")?;
                let clips=self.selected(clips)?; let mut peaks=Vec::new();
                for clip in &clips {
                    let audio=self.load(clip)?;
                    let prepared=prepare_clip(clip,&audio,&self.tempo,clip.source.sample_rate,0).map_err(|e|e.to_string())?;
                    peaks.push(prepared.samples.iter().fold(0.0_f32,|peak,sample|peak.max(sample.abs())));
                }
                let target=10.0_f64.powf(target_db/20.0) as f32;
                let group_peak=clips.iter().zip(&peaks).map(|(clip,peak)|clip.gain*peak).fold(0.0_f32,f32::max);
                if group_peak <= 1e-12 { return Err("cannot normalize silent clips".into()); }
                for (clip,peak) in clips.iter().zip(peaks) {
                    let mut normalized=clip.clone();
                    if *independent { if peak>1e-12 {normalized.gain=target/peak;} } else {normalized.gain*=target/group_peak;}
                    normalized.validate().map_err(|e|e.to_string())?; self.replace(clip,vec![normalized])?;
                }
            }
            AudioEdit::Bounce { clips,path,sample_rate,replace } => self.bounce(clips,path,*sample_rate,*replace)?,
        }
        if self.tracks.iter().any(|track|track.len()>MAX_EDIT_ITEMS) {return Err("audio edit exceeds track clip limit".into());}
        Ok(())
    }
    fn bounce(&mut self, refs:&[AudioClipRef], path:&str, rate:u32, replace:bool)->Result<(),String>{
        if !(8_000..=384_000).contains(&rate){return Err("bounce rate must be 8000..384000".into());}
        let clips=self.selected(refs)?;
        let path=Path::new(path);
        if !path.is_absolute() || path.file_name().is_none(){return Err("bounce requires an explicit absolute output file".into());}
        let parent=path.parent().ok_or("bounce output has no parent")?.canonicalize().map_err(|e|e.to_string())?;
        let path=parent.join(path.file_name().unwrap());
        if path.try_exists().map_err(|e|e.to_string())? || self.assets.iter().any(|asset|asset.path==path){return Err("bounce output already exists".into());}
        let start=clips.iter().map(|clip|clip.start_seconds(&self.tempo)).fold(f64::INFINITY,f64::min);
        let end=clips.iter().map(|clip|clip.end_seconds(&self.tempo)).fold(0.0_f64,f64::max);
        let start_frame=(start*rate as f64).round() as u64;
        let end_frame=(end*rate as f64).round() as u64;
        let frames=end_frame.checked_sub(start_frame).filter(|frames|*frames>0).ok_or("empty bounce range")?;
        let samples=frames.checked_mul(2).and_then(|n|usize::try_from(n).ok()).ok_or("bounce size overflow")?;
        if samples.checked_mul(4).is_none_or(|n|n>ImportLimits::default().max_pcm_bytes){return Err("bounce exceeds PCM limit".into());}
        let pending_bytes=self.assets.iter().try_fold(0usize,|sum,asset|sum.checked_add(asset.bytes.len())).ok_or("pending audio size overflow")?;
        if samples.checked_mul(12).and_then(|n|n.checked_add(self.decoded_bytes)).and_then(|n|n.checked_add(pending_bytes)).is_none_or(|n|n>ImportLimits::default().max_project_pcm_bytes){return Err("bounce exceeds project workspace limit".into());}
        let mut mix=vec![0.0_f32;samples];
        for clip in &clips{
            let audio=self.load(clip)?;let prepared=prepare_clip(clip,&audio,&self.tempo,rate,0).map_err(|e|e.to_string())?;
            let offset=prepared.start.checked_sub(start_frame).ok_or("bounce placement underflow")? as usize;
            for (i,frame) in prepared.samples.chunks_exact(prepared.channels as usize).enumerate(){
                if offset+i>=frames as usize{break;}
                mix[(offset+i)*2]+=frame[0]*prepared.gain;
                mix[(offset+i)*2+1]+=frame[if prepared.channels==1{0}else{1}]*prepared.gain;
            }
        }
        if mix.iter().any(|sample|!sample.is_finite()){return Err("non-finite bounced audio".into());}
        let bytes=float_wav(&mix,rate)?;
        let decoded=decode_bytes(&bytes,Some("wav"),ImportLimits::default()).map_err(|e|e.to_string())?;
        let source=AudioSource{path:path.to_str().ok_or("non-Unicode bounce path")?.into(),sha256:decoded.sha256.clone(),byte_len:decoded.byte_len,sample_rate:rate,channels:2,frames};
        let mut clip=AudioClip::new(source,clips[0].track.clone(),0);
        clip.name=path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        clip.set_tick(self.tempo.tick_at(start_frame as f64/rate as f64)).map_err(|e|e.to_string())?;
        clip.validate().map_err(|e|e.to_string())?;
        if replace{for old in &clips{self.replace(old,Vec::new())?;}}
        let key=(clip.source.path.clone(),clip.source.sha256.clone());
        self.verified_sources.insert(key.clone(),clip.source.clone());
        self.decoded.insert(key,decoded);
        self.assets.push(PendingAudioAsset{path,sha256:clip.source.sha256.clone(),byte_len:bytes.len()as u64,bytes});
        self.selection.push(AudioClipRef::from(&clip));let index=self.track_index(&clip.track)?;self.tracks[index].push(clip);
        Ok(())
    }
}
fn valid_tick(tick:f64)->Result<(),String>{if tick.is_finite()&&tick>=0.0&&tick<i64::MAX as f64{Ok(())}else{Err("invalid audio edit tick".into())}}
fn finite_between(value:f64,min:f64,max:f64,label:&str)->Result<(),String>{if value.is_finite()&&(min..=max).contains(&value){Ok(())}else{Err(format!("invalid {label}: expected {min}..{max}"))}}
fn float_wav(samples:&[f32],rate:u32)->Result<Vec<u8>,String>{
    let len=samples.len().checked_mul(4).and_then(|n|u32::try_from(n).ok()).ok_or("WAV byte count overflow")?;
    let total=len.checked_add(36).ok_or("WAV size overflow")?;
    let mut bytes=Vec::with_capacity(len as usize+44);
    bytes.extend_from_slice(b"RIFF");bytes.extend_from_slice(&total.to_le_bytes());bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());bytes.extend_from_slice(&3u16.to_le_bytes());bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());bytes.extend_from_slice(&(rate*8).to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());bytes.extend_from_slice(&32u16.to_le_bytes());bytes.extend_from_slice(b"data");bytes.extend_from_slice(&len.to_le_bytes());
    for sample in samples{bytes.extend_from_slice(&sample.to_le_bytes());}Ok(bytes)
}

/// Files written and synced on a background/control worker, but not yet
/// visible at their requested final names. Dropping a stale plan removes only
/// its private staging files. Published files are never removed by this type.
pub struct StagedAudioAssets {
    assets: Vec<StagedAudioAsset>,
    published: Vec<PathBuf>,
}
struct StagedAudioAsset { temporary:PathBuf, path:PathBuf, file:std::fs::File, sealed:std::fs::Metadata }
impl Drop for StagedAudioAsset {
    fn drop(&mut self){
        // A user may have replaced a staging entry; never remove that entry.
        if let (Ok(named),Ok(owned))=(std::fs::symlink_metadata(&self.temporary),self.file.metadata()){
            if staged_identity(&named,&owned){let _=std::fs::remove_file(&self.temporary);}
        }
    }
}
fn staged_identity(a:&std::fs::Metadata,b:&std::fs::Metadata)->bool{
    if !a.is_file()||!b.is_file()||a.len()!=b.len()||a.modified().ok()!=b.modified().ok(){return false;}
    #[cfg(unix)]{use std::os::unix::fs::MetadataExt;a.dev()==b.dev()&&a.ino()==b.ino()}
    #[cfg(not(unix))]{a.created().ok()==b.created().ok()}
}
impl StagedAudioAssets{
    pub fn published_paths(&self)->&[PathBuf]{&self.published}
    /// Short, no-clobber metadata operation. Call in the same control turn as
    /// the final revision check and journal commit; no encoding or PCM write.
    pub fn publish(&mut self)->Result<Vec<PathBuf>,String>{
        if !self.published.is_empty(){return Err("staged audio assets were already published".into());}
        for asset in &self.assets{
            let result=(||->Result<(),String>{
                let named=std::fs::symlink_metadata(&asset.temporary).map_err(|e|e.to_string())?;
                let owned=asset.file.metadata().map_err(|e|e.to_string())?;
                if !staged_identity(&named,&owned)||!staged_identity(&owned,&asset.sealed){return Err("staged audio asset changed before publication".into());}
                std::fs::hard_link(&asset.temporary,&asset.path).map_err(|e|e.to_string())?;
                self.published.push(asset.path.clone());
                let published=std::fs::symlink_metadata(&asset.path).map_err(|e|e.to_string())?;
                if !staged_identity(&published,&owned){return Err("published audio identity changed".into());}
                Ok(())
            })();
            if let Err(error)=result{
                let paths=self.published.iter().map(|path|path.display().to_string()).collect::<Vec<_>>().join(", ");
                return Err(if paths.is_empty(){error}else{format!("{error}; published audio retained: {paths}; project edit not completed")});
            }
        }
        Ok(self.published.clone())
    }
}
/// Background execution only, after planning. Planning/dry-run never call this
/// function and therefore perform no file or directory writes.
pub fn stage_audio_assets(plan:&AudioEditPlan)->Result<StagedAudioAssets,String>{
    use std::io::Write;
    use sha2::{Digest,Sha256};
    let mut seen=BTreeSet::new();
    for asset in &plan.pending_assets{
        if !seen.insert(&asset.path){return Err("duplicate pending audio output path".into());}
        if asset.bytes.len()as u64!=asset.byte_len||hex::encode(Sha256::digest(&asset.bytes))!=asset.sha256{
            return Err("pending audio asset fingerprint mismatch".into());
        }
    }
    let mut staged=StagedAudioAssets{assets:Vec::new(),published:Vec::new()};
    for asset in &plan.pending_assets{
        let temporary=asset.path.parent().ok_or("audio asset has no parent")?.join(format!(".verdant-bounce-{}.tmp",Uuid::new_v4()));
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e|e.to_string())?;
        let sealed=file.metadata().map_err(|e|e.to_string())?;
        let mut entry=StagedAudioAsset{temporary,path:asset.path.clone(),file,sealed};
        entry.file.write_all(&asset.bytes).and_then(|_|entry.file.sync_all()).map_err(|e|e.to_string())?;
        entry.sealed=entry.file.metadata().map_err(|e|e.to_string())?;
        staged.assets.push(entry);
    }
    Ok(staged)
}
/// Compatibility helper for callers already holding a project writer lease.
pub fn publish_audio_assets(plan:&AudioEditPlan)->Result<Vec<PathBuf>,String>{
    stage_audio_assets(plan)?.publish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_domain::{ProjectStore, Track};

    struct Fixture { directory: PathBuf, project: Project, clip: AudioClip }
    impl Fixture {
        fn new() -> Self {
            let directory=std::env::temp_dir().join(format!("verdant-audio-edit-{}",Uuid::new_v4()));
            std::fs::create_dir(&directory).unwrap();
            // macOS temp_dir may use /var while verified assets resolve to /private/var.
            let directory=std::fs::canonicalize(directory).unwrap();
            let path=directory.join("original.wav");
            let samples:Vec<_>=(0..48_000).flat_map(|i|{
                let value=if (16_000..32_000).contains(&i){0.0}else{(i as f32*std::f32::consts::TAU*440.0/48_000.0).sin()*0.25};
                [value,-value]
            }).collect();
            let bytes=float_wav(&samples,48_000).unwrap();std::fs::write(&path,&bytes).unwrap();
            let importer=MediaImporter::new(AssetPolicy::for_file(&path).unwrap(),ImportLimits::default());
            let source=importer.import_file(&path).unwrap().source;
            let mut project=Project::new_empty();project.tracks=vec![Track::new("One","blue"),Track::new("Two","green"),Track::new("Three","orange")];
            let clip=AudioClip::new(source,project.tracks[0].id.clone(),0);
            verdant_media::write_clips(&mut project.tracks[0],&[clip.clone()]).unwrap();
            Self{directory,project,clip}
        }
        fn policy(&self)->AssetPolicy{AssetPolicy::new([self.directory.clone()]).unwrap()}
        fn reference(&self)->AudioClipRef{AudioClipRef::from(&self.clip)}
    }
    impl Drop for Fixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.directory);}}
    fn apply(project:Project,plan:AudioEditPlan)->ProjectStore{
        let mut store=ProjectStore::new(project);store.apply(store.revision(),plan.batch).unwrap();store
    }

    #[test]
    fn multi_track_move_is_one_undo_and_invalid_batches_do_not_partially_apply(){
        let mut f=Fixture::new();
        let mut second=f.clip.duplicate_at_tick(2400).unwrap();second.track=f.project.tracks[1].id.clone();
        verdant_media::write_clips(&mut f.project.tracks[1],&[second.clone()]).unwrap();
        let edit=AudioEdit::Move{clips:vec![f.reference(),AudioClipRef::from(&second)],delta_ticks:480.25,track_offset:1};
        let plan=plan_audio_edit(&f.project,&edit).unwrap();assert_eq!(plan.batch.commands.len(),3);
        let mut store=apply(f.project.clone(),plan);
        assert_eq!(store.undo_depth(),1);assert_eq!(read_clips(&store.project().tracks[1]).unwrap()[0].tick(),480.25);
        assert_eq!(read_clips(&store.project().tracks[2]).unwrap()[0].tick(),2880.25);
        store.undo().unwrap();assert_eq!(store.project().tracks,f.project.tracks);
        store.redo().unwrap();assert_eq!(read_clips(&store.project().tracks[2]).unwrap()[0].id,second.id);
        let invalid=AudioEdit::Batch{edits:vec![
            AudioEdit::SetProperties{clips:vec![f.reference()],properties:AudioClipProperties{muted:Some(true),..Default::default()}},
            AudioEdit::Move{clips:vec![AudioClipRef{track:f.clip.track.clone(),id:Uuid::new_v4()}],delta_ticks:1.0,track_offset:0}
        ]};
        assert!(plan_audio_edit(&f.project,&invalid).is_err());
        assert!(!read_clips(&f.project.tracks[0]).unwrap()[0].muted);
        assert!(plan_audio_edit(&f.project,&AudioEdit::Move{clips:vec![f.reference()],delta_ticks:f64::NAN,track_offset:0}).is_err());
    }

    #[test]
    fn range_cut_closes_time_gap_and_preserves_source_frames_and_history(){
        let f=Fixture::new();
        let plan=plan_audio_edit(&f.project,&AudioEdit::DeleteRange{tracks:vec![f.clip.track.clone()],start_tick:480.0,end_tick:960.0,ripple:true}).unwrap();
        let mut store=apply(f.project.clone(),plan);
        let clips=read_clips(&store.project().tracks[0]).unwrap();assert_eq!(clips.len(),2);
        assert_eq!(clips[0].duration_frames,12_000);assert_eq!(clips[1].source_offset_frames,24_000);
        assert_eq!(clips[1].duration_frames,24_000);
        let tempo=TempoMap::constant(120.0);assert!((clips[0].end_seconds(&tempo)-clips[1].start_seconds(&tempo)).abs()<1e-12);
        assert_eq!(clips.iter().map(|clip|clip.duration_frames).sum::<u64>(),36_000);
        let encoded=serde_json::to_string(store.project()).unwrap();
        let reopened:Project=serde_json::from_str(&encoded).unwrap();assert_eq!(read_clips(&reopened.tracks[0]).unwrap(),clips);
        store.undo().unwrap();assert_eq!(read_clips(&store.project().tracks[0]).unwrap(),vec![f.clip.clone()]);
        store.redo().unwrap();assert_eq!(read_clips(&store.project().tracks[0]).unwrap(),clips);
    }

    #[test]
    fn clipboard_range_is_source_only_and_paste_preserves_seconds_across_tempo(){
        let f=Fixture::new();
        let copied=plan_audio_edit(&f.project,&AudioEdit::CopyRange{tracks:vec![f.clip.track.clone()],start_tick:480.0,end_tick:1440.0}).unwrap();
        assert!(copied.batch.commands.is_empty());assert!(copied.pending_assets.is_empty());
        let clipboard=copied.clipboard.unwrap();assert_eq!(clipboard.clips[0].clip.source_offset_frames,12_000);
        let mut destination=f.project.clone();destination.tempo_map[0].micro_bpm=60_000_000;
        let plan=plan_audio_edit(&destination,&AudioEdit::Paste{clipboard,at_tick:1920.0,target_track:destination.tracks[1].id.clone()}).unwrap();
        let store=apply(destination,plan);let clip=read_clips(&store.project().tracks[1]).unwrap().remove(0);
        assert_eq!(clip.tick(),1920.0);assert_eq!(clip.duration_seconds(),0.5);assert_ne!(clip.id,f.clip.id);
        assert_eq!(clip.source,f.clip.source);
    }

    #[test]
    fn silence_and_zero_crossing_splits_use_authorized_verified_audio(){
        let f=Fixture::new();
        let operation=AudioEdit::SplitSilence{clips:vec![f.reference()],threshold_db:-60.0,min_silence_ms:100.0};
        assert!(plan_audio_edit(&f.project,&operation).is_err());
        let plan=plan_audio_edit_with_media(&f.project,&operation,f.policy()).unwrap();
        let store=apply(f.project.clone(),plan);let clips=read_clips(&store.project().tracks[0]).unwrap();
        assert_eq!(clips.len(),2);assert_eq!(clips.iter().map(|clip|clip.duration_frames).sum::<u64>(),48_000);
        assert!((clips[0].duration_frames as i64-24_000).abs()<=1);
        let edit=AudioEdit::SplitZeroCrossing{clip:f.reference(),tick:123.1,window_ms:10.0};
        let plan=plan_audio_edit_with_media(&f.project,&edit,f.policy()).unwrap();
        let store=apply(f.project.clone(),plan);assert_eq!(read_clips(&store.project().tracks[0]).unwrap().len(),2);
        let path=Path::new(&f.clip.source.path);std::fs::write(path,b"changed").unwrap();
        assert!(plan_audio_edit_with_media(&f.project,&edit,f.policy()).is_err());
    }

    #[test]
    fn crossfade_normalize_and_bounce_share_playback_pcm_and_dry_run_writes_nothing(){
        let mut f=Fixture::new();
        let mut second=f.clip.duplicate_at_tick(960).unwrap();second.reversed=true;
        verdant_media::write_clips(&mut f.project.tracks[0],&[f.clip.clone(),second.clone()]).unwrap();
        let refs=vec![f.reference(),AudioClipRef::from(&second)];
        let plan=plan_audio_edit(&f.project,&AudioEdit::Crossfade{left:refs[0].clone(),right:refs[1].clone(),shape:FadeShape::EqualPower}).unwrap();
        let store=apply(f.project.clone(),plan);let clips=read_clips(&store.project().tracks[0]).unwrap();
        assert_eq!(clips[0].fade_out_frames,24_000);assert_eq!(clips[1].fade_in_frames,24_000);
        let normal=plan_audio_edit_with_media(store.project(),&AudioEdit::Normalize{clips:refs.clone(),target_db:-6.0,independent:false},f.policy()).unwrap();
        let normalized=apply(store.project().clone(),normal);
        let path=f.directory.join("bounce.wav");
        let edit=AudioEdit::Bounce{clips:refs,path:path.to_string_lossy().into_owned(),sample_rate:48_000,replace:true};
        let plan=plan_audio_edit_with_media(normalized.project(),&edit,f.policy()).unwrap();
        assert!(!path.exists());assert_eq!(plan.pending_assets.len(),1);
        let encoded=serde_json::to_string(&plan).unwrap();assert!(!encoded.contains("\"bytes\""));
        let bytes=plan.pending_assets[0].bytes.clone();
        let published=publish_audio_assets(&plan).unwrap();assert_eq!(published,vec![path.clone()]);
        assert!(publish_audio_assets(&plan).is_err());assert_eq!(std::fs::read(&path).unwrap(),bytes);
        let bounced=apply(normalized.project().clone(),plan);
        let importer=MediaImporter::new(f.policy(),ImportLimits::default());
        let original=importer.prepare_project_clips(normalized.project(),&TempoMap::constant(120.0),48_000).unwrap();
        let output=importer.prepare_project_clips(bounced.project(),&TempoMap::constant(120.0),48_000).unwrap();
        assert_eq!(output.len(),1);assert_eq!(output[0].frames(),72_000);
        let mut expected=vec![0.0_f32;144_000];
        for clip in original{for (i,frame) in clip.samples.chunks_exact(2).enumerate(){for c in 0..2{expected[(clip.start as usize+i)*2+c]+=frame[c]*clip.gain;}}}
        assert_eq!(output[0].samples.as_ref(),expected);
    }

    #[test]
    fn portable_clipboard_resolves_verified_sources_across_project_directories(){
        let mut f=Fixture::new();
        f.clip.source.path="original.wav".into();
        verdant_media::write_clips(&mut f.project.tracks[0],&[f.clip.clone()]).unwrap();
        let source_policy=f.policy().with_project_path(&f.directory.join("source.verp")).unwrap();
        let target_dir=f.directory.join("different-project");std::fs::create_dir(&target_dir).unwrap();
        std::fs::write(target_dir.join("original.wav"),float_wav(&vec![0.;96_000],48_000).unwrap()).unwrap();
        let mut destination=Project::new_empty();destination.tracks=vec![Track::new("Destination","orange")];
        let destination_path=target_dir.join("target.verp");
        let policy=AssetPolicy::new([f.directory.clone()]).unwrap().with_project_path(&destination_path).unwrap();
        for operation in [
            AudioEdit::Copy{clips:vec![f.reference()]},
            AudioEdit::CopyRange{tracks:vec![f.clip.track.clone()],start_tick:240.,end_tick:960.},
        ]{
            let plan=plan_audio_edit_with_media(&f.project,&operation,source_policy.clone()).unwrap();
            assert!(plan.batch.commands.is_empty());assert!(plan.pending_assets.is_empty());
            let clipboard=plan.clipboard.unwrap();
            assert_eq!(Path::new(&clipboard.clips[0].clip.source.path),f.directory.join("original.wav"));
            let decoded:AudioClipboard=serde_json::from_str(&serde_json::to_string(&clipboard).unwrap()).unwrap();
            let paste=AudioEdit::Paste{clipboard:decoded,at_tick:480.,target_track:destination.tracks[0].id.clone()};
            assert!(plan_audio_edit(&destination,&paste).is_err());
            let denied=AssetPolicy::new([]).unwrap().with_project_path(&destination_path).unwrap();
            assert!(plan_audio_edit_with_media(&destination,&paste,denied).is_err());
            let pasted=plan_audio_edit_with_media(&destination,&paste,policy.clone()).unwrap();
            let project=apply(destination.clone(),pasted);
            let clip=read_clips(&project.project().tracks[0]).unwrap().remove(0);
            assert_eq!(clip.source.sha256,f.clip.source.sha256);
            assert_eq!(Path::new(&clip.source.path),f.directory.join("original.wav"));
            MediaImporter::new(policy.clone(),ImportLimits::default()).load_source(&clip.source).unwrap();
            let original=std::fs::read(f.directory.join("original.wav")).unwrap();
            std::fs::write(f.directory.join("original.wav"),float_wav(&vec![0.;96_000],48_000).unwrap()).unwrap();
            assert!(plan_audio_edit_with_media(&destination,&paste,policy.clone()).is_err());
            std::fs::write(f.directory.join("original.wav"),original).unwrap();
        }
    }

    #[test]
    fn stale_bounce_staging_cleans_private_files_without_publishing_final_output(){
        let f=Fixture::new();let path=f.directory.join("stale.wav");
        let plan=plan_audio_edit_with_media(&f.project,&AudioEdit::Bounce{clips:vec![f.reference()],
            path:path.to_string_lossy().into(),sample_rate:48_000,replace:true},f.policy()).unwrap();
        let before=std::fs::read_dir(&f.directory).unwrap().count();
        let staged=stage_audio_assets(&plan).unwrap();
        assert!(!path.exists());assert!(staged.published_paths().is_empty());
        assert_eq!(std::fs::read_dir(&f.directory).unwrap().count(),before+1);
        let mut store=ProjectStore::new(f.project.clone());let base=store.revision();
        let concurrent=plan_audio_edit(store.project(),&AudioEdit::SetProperties{clips:vec![f.reference()],
            properties:AudioClipProperties{gain_db:Some(-3.),..Default::default()}}).unwrap();
        store.apply(base,concurrent.batch).unwrap();
        assert_ne!(base,store.revision());
        drop(staged); // The UI's stale revision branch never calls publish.
        assert!(!path.exists());assert_eq!(std::fs::read_dir(&f.directory).unwrap().count(),before);
    }

    #[test]
    fn publication_receipt_retains_complete_assets_and_never_clobbers_a_racing_file(){
        let f=Fixture::new();let first=f.directory.join("first.wav");let second=f.directory.join("second.wav");
        let mut plan=plan_audio_edit_with_media(&f.project,&AudioEdit::Bounce{clips:vec![f.reference()],
            path:first.to_string_lossy().into(),sample_rate:48_000,replace:true},f.policy()).unwrap();
        let mut another=plan.pending_assets[0].clone();another.path=second.clone();plan.pending_assets.push(another);
        let mut staged=stage_audio_assets(&plan).unwrap();
        std::fs::write(&second,b"user-created-after-stage").unwrap();
        assert!(staged.publish().is_err());assert_eq!(staged.published_paths(),&[first.clone()]);
        drop(staged);
        assert_eq!(std::fs::read(&first).unwrap(),plan.pending_assets[0].bytes);
        assert_eq!(std::fs::read(&second).unwrap(),b"user-created-after-stage");
        assert_eq!(std::fs::read_dir(&f.directory).unwrap().filter_map(Result::ok).filter(|e|e.file_name().to_string_lossy().starts_with(".verdant-bounce-")).count(),0);
    }

    #[test]
    fn bounce_then_copy_range_keeps_verified_pending_source_in_one_atomic_plan(){
        let f=Fixture::new();let path=f.directory.join("pending-copy.wav");
        let edit=AudioEdit::Batch{edits:vec![
            AudioEdit::Bounce{clips:vec![f.reference()],path:path.to_string_lossy().into(),sample_rate:48_000,replace:true},
            AudioEdit::CopyRange{tracks:vec![f.clip.track.clone()],start_tick:0.,end_tick:960.}
        ]};
        let plan=plan_audio_edit_with_media(&f.project,&edit,f.policy()).unwrap();
        assert!(!path.exists());assert_eq!(plan.pending_assets.len(),1);
        let clipboard=plan.clipboard.as_ref().unwrap();
        assert_eq!(clipboard.clips.len(),1);
        assert_eq!(Path::new(&clipboard.clips[0].clip.source.path),path);
        assert_eq!(clipboard.clips[0].clip.source.sha256,plan.pending_assets[0].sha256);
        let mut staged=stage_audio_assets(&plan).unwrap();staged.publish().unwrap();
        let store=apply(f.project.clone(),plan);
        let clipboard=clipboard_for_test_source(&store.project().tracks[0]);
        MediaImporter::new(f.policy(),ImportLimits::default()).load_source(&clipboard.source).unwrap();
    }
    fn clipboard_for_test_source(track:&Track)->AudioClip{read_clips(track).unwrap().remove(0)}
}
