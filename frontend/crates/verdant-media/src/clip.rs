use std::{collections::BTreeSet, path::Path, sync::Arc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use verdant_domain::{TempoMap, Track, TrackId};
use crate::{DecodedAudio, ImportLimits, MediaError};

pub const AUDIO_CLIPS_KEY: &str = "audio_clips";
pub const AUDIO_CLIPS_SCHEMA: &str = "verdant-audio-clips/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSource {
    pub path: String,
    pub sha256: String,
    pub byte_len: u64,
    pub sample_rate: u32,
    pub channels: u16,
    /// Decoded frames after the codec's declared gapless trim.
    pub frames: u64,
}

impl AudioSource {
    pub fn validate(&self) -> Result<(), MediaError> {
        if self.path.contains('\0') || (!Path::new(&self.path).is_absolute() && !portable_path(&self.path)) {
            return Err(MediaError::InvalidPath);
        }
        if !crate::decode::valid_sha256(&self.sha256) || self.byte_len == 0 { return Err(MediaError::Invalid("source fingerprint")); }
        if self.sample_rate == 0 || self.frames == 0 { return Err(MediaError::Invalid("source rate/frame count")); }
        if !(1..=2).contains(&self.channels) { return Err(MediaError::Channels(self.channels as usize)); }
        Ok(())
    }
    pub fn matches(&self, audio: &DecodedAudio) -> Result<(), MediaError> {
        if self.sha256 != audio.sha256 || self.byte_len != audio.byte_len || self.sample_rate != audio.sample_rate
            || self.channels != audio.channels || self.frames != audio.frames() { return Err(MediaError::SourceChanged); }
        Ok(())
    }
}

/// A portable reference has identical components on Unix and Windows. Reject
/// drive/UNC syntax, normalization aliases and reserved device filenames even
/// when the current host would treat them as ordinary names.
pub(crate) fn portable_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 1024 || path.starts_with('/') || path.contains('\\')
        || path.chars().any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*')) {
        return false;
    }
    let mut count = 0;
    for component in path.split('/') {
        count += 1;
        if count > 64 || component.is_empty() || component.len() > 120 || component == "." || component == ".."
            || component.ends_with(['.', ' ']) {
            return false;
        }
        let stem = component.split('.').next().unwrap_or_default();
        if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].iter().any(|name| stem.eq_ignore_ascii_case(name)) {
            return false;
        }
        if stem.as_bytes().get(..3).is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"COM") || prefix.eq_ignore_ascii_case(b"LPT"))
            && matches!(stem.get(3..), Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")) {
            return false;
        }
    }
    true
}

/// Edge-curve shape. Equal power is useful for decorrelated overlapping clips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FadeShape { #[default] Linear, EqualPower }

/// A cropped view of an existing fade envelope. Splitting a fade must not
/// restart its ramp or change any of the retained samples.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipFadeWindow {
    pub duration_frames: u64,
    pub offset_frames: u64,
    pub fade_in_frames: u64,
    pub fade_out_frames: u64,
}

fn unit_rate() -> f64 { 1.0 }

/// Source trim and duration use decoded source frames. Placement is musical;
/// playback rate is independent of tempo. All editing metadata is non-destructive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClip {
    pub id: Uuid,
    pub source: AudioSource,
    pub start_tick: i64,
    /// Fractional tick remainder keeps cuts aligned to their exact source frame.
    #[serde(default)]
    pub start_subtick: f64,
    pub source_offset_frames: u64,
    pub duration_frames: u64,
    #[serde(default)]
    pub fade_in_frames: u64,
    #[serde(default)]
    pub fade_out_frames: u64,
    #[serde(default)]
    pub fade_shape: FadeShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_window: Option<ClipFadeWindow>,
    #[serde(default)]
    pub name: String,
    #[serde(default = "unit_rate")]
    pub playback_rate: f64,
    #[serde(default)]
    pub preserve_pitch: bool,
    #[serde(default)]
    pub reversed: bool,
    pub gain: f32,
    pub pan: f32,
    pub muted: bool,
    pub track: TrackId,
}

impl AudioClip {
    pub fn new(source: AudioSource, track: TrackId, start_tick: i64) -> Self {
        let duration_frames = source.frames;
        Self { id: Uuid::new_v4(), source, start_tick, start_subtick: 0.0,
            source_offset_frames: 0, duration_frames, fade_in_frames: 0, fade_out_frames: 0,
            fade_shape: FadeShape::Linear, fade_window: None, name: String::new(),
            playback_rate: 1.0, preserve_pitch: false, reversed: false,
            gain: 1.0, pan: 0.0, muted: false, track }
    }
    pub fn validate(&self) -> Result<(), MediaError> {
        self.source.validate()?;
        if self.id.is_nil() { return Err(MediaError::Invalid("nil audio clip ID")); }
        if self.start_tick < 0 || !self.start_subtick.is_finite() || !(0.0..1.0).contains(&self.start_subtick) {
            return Err(MediaError::Invalid("clip timeline position"));
        }
        if self.name.len() > 512 || self.name.chars().any(char::is_control) { return Err(MediaError::Invalid("clip name")); }
        if !self.playback_rate.is_finite() || !(0.0625..=16.0).contains(&self.playback_rate) {
            return Err(MediaError::Invalid("playback rate must be 0.0625..16"));
        }
        if self.duration_frames == 0 || self.source_offset_frames.checked_add(self.duration_frames).is_none_or(|end| end > self.source.frames) {
            return Err(MediaError::Invalid("clip trim exceeds source"));
        }
        if !self.gain.is_finite() || !(0.0..=64.0).contains(&self.gain) { return Err(MediaError::Invalid("clip gain")); }
        if !self.pan.is_finite() || !(-1.0..=1.0).contains(&self.pan) { return Err(MediaError::Invalid("clip pan")); }
        if self.fade_in_frames > self.duration_frames || self.fade_out_frames > self.duration_frames {
            return Err(MediaError::Invalid("fade exceeds clip duration"));
        }
        if let Some(window) = &self.fade_window {
            if window.duration_frames == 0 || window.offset_frames.checked_add(self.duration_frames).is_none_or(|end| end > window.duration_frames)
                || window.fade_in_frames > window.duration_frames || window.fade_out_frames > window.duration_frames {
                return Err(MediaError::Invalid("cropped fade envelope"));
            }
        }
        Ok(())
    }
    pub fn tick(&self) -> f64 { self.start_tick as f64 + self.start_subtick }
    pub fn duration_seconds(&self) -> f64 { self.duration_frames as f64 / self.source.sample_rate as f64 / self.playback_rate }
    pub fn start_seconds(&self, tempo: &TempoMap) -> f64 { tempo.seconds_at(self.tick()) }
    pub fn end_seconds(&self, tempo: &TempoMap) -> f64 { self.start_seconds(tempo) + self.duration_seconds() }
    pub fn end_tick(&self, tempo: &TempoMap) -> f64 { tempo.tick_at(self.end_seconds(tempo)) }
    pub fn set_tick(&mut self, tick: f64) -> Result<(), MediaError> {
        if !tick.is_finite() || tick < 0.0 || tick >= i64::MAX as f64 { return Err(MediaError::Invalid("clip timeline position")); }
        let nearest = tick.round();
        let tick = if (tick - nearest).abs() < 1e-9 { nearest } else { tick };
        self.start_tick = tick.floor() as i64;
        self.start_subtick = tick - self.start_tick as f64;
        Ok(())
    }
    pub fn frame_at_tick(&self, tick: f64, tempo: &TempoMap) -> Result<u64, MediaError> {
        if !tick.is_finite() { return Err(MediaError::Invalid("non-finite edit position")); }
        let frame = ((tempo.seconds_at(tick) - self.start_seconds(tempo)) * self.source.sample_rate as f64 * self.playback_rate).round();
        if !frame.is_finite() || frame < 0.0 || frame > self.duration_frames as f64 { return Err(MediaError::Invalid("edit position outside clip")); }
        Ok(frame as u64)
    }
    pub fn tick_at_frame(&self, frame: u64, tempo: &TempoMap) -> f64 {
        tempo.tick_at(self.start_seconds(tempo) + frame as f64 / self.source.sample_rate as f64 / self.playback_rate)
    }
    pub fn duplicate_at_tick(&self, tick: i64) -> Result<Self, MediaError> { self.duplicate_at(tick as f64) }
    pub fn duplicate_at(&self, tick: f64) -> Result<Self, MediaError> {
        let mut copy = self.clone(); copy.id = Uuid::new_v4(); copy.set_tick(tick)?; copy.validate()?; Ok(copy)
    }
    /// Retain a source-frame range in playback order. The source is never copied.
    pub fn slice_frames(&self, begin: u64, end: u64, tempo: &TempoMap) -> Result<Self, MediaError> {
        self.validate()?;
        if begin >= end || end > self.duration_frames { return Err(MediaError::Invalid("empty or out-of-bounds clip slice")); }
        let mut slice = self.clone();
        slice.source_offset_frames += if self.reversed { self.duration_frames - end } else { begin };
        slice.duration_frames = end - begin;
        slice.set_tick(self.tick_at_frame(begin, tempo))?;
        if let Some(window) = &self.fade_window {
            let mut window = window.clone(); window.offset_frames += begin; slice.fade_window = Some(window);
        } else if (begin != 0 || end != self.duration_frames) && (self.fade_in_frames > 0 || self.fade_out_frames > 0) {
            slice.fade_window = Some(ClipFadeWindow { duration_frames: self.duration_frames, offset_frames: begin,
                fade_in_frames: self.fade_in_frames, fade_out_frames: self.fade_out_frames });
        }
        slice.fade_in_frames = if begin == 0 { self.fade_in_frames.min(slice.duration_frames) } else { 0 };
        slice.fade_out_frames = if end == self.duration_frames { self.fade_out_frames.min(slice.duration_frames) } else { 0 };
        slice.validate()?;
        Ok(slice)
    }
    pub fn split_at_tick(&self, tick: i64, tempo: &TempoMap) -> Result<(Self, Self), MediaError> { self.split_at(tick as f64, tempo) }
    pub fn split_at(&self, tick: f64, tempo: &TempoMap) -> Result<(Self, Self), MediaError> {
        let frame = self.frame_at_tick(tick, tempo)?;
        if frame == 0 || frame >= self.duration_frames { return Err(MediaError::Invalid("split must be inside clip")); }
        let left = self.slice_frames(0, frame, tempo)?;
        let mut right = self.slice_frames(frame, self.duration_frames, tempo)?;
        right.id = Uuid::new_v4();
        Ok((left, right))
    }
    /// Re-trim the source window, including revealing previously hidden samples.
    /// When the retained window is a slice, its exact envelope is preserved.
    pub fn trim_source(&self, offset: u64, duration: u64, start: f64, tempo: &TempoMap) -> Result<Self, MediaError> {
        let end = offset.checked_add(duration).ok_or(MediaError::Invalid("clip trim overflow"))?;
        if duration == 0 || end > self.source.frames { return Err(MediaError::Invalid("clip trim exceeds source")); }
        let old_end = self.source_offset_frames + self.duration_frames;
        let mut clip = if offset >= self.source_offset_frames && end <= old_end {
            let (begin, finish) = if self.reversed { (old_end - end, old_end - offset) } else { (offset - self.source_offset_frames, end - self.source_offset_frames) };
            self.slice_frames(begin, finish, tempo)?
        } else {
            let mut expanded = self.clone();
            expanded.fade_window = None;
            expanded.source_offset_frames = offset; expanded.duration_frames = duration;
            expanded.fade_in_frames = expanded.fade_in_frames.min(duration);
            expanded.fade_out_frames = expanded.fade_out_frames.min(duration);
            if let Some(window) = &self.fade_window {
                let delta = if self.reversed { i128::from(old_end) - i128::from(end) } else { i128::from(offset) - i128::from(self.source_offset_frames) };
                let position = i128::from(window.offset_frames) + delta;
                if position >= 0 && position + i128::from(duration) <= i128::from(window.duration_frames) {
                    let mut restored = window.clone(); restored.offset_frames = position as u64;
                    expanded.fade_in_frames = if position == 0 { window.fade_in_frames.min(duration) } else { 0 };
                    expanded.fade_out_frames = if position + i128::from(duration) == i128::from(window.duration_frames) { window.fade_out_frames.min(duration) } else { 0 };
                    if position == 0 && duration == window.duration_frames { expanded.fade_window = None; }
                    else { expanded.fade_window = Some(restored); }
                }
            }
            expanded
        };
        clip.set_tick(start)?; clip.validate()?; Ok(clip)
    }
    /// Gain envelope at a playback-order source frame, also used for waveform UI.
    pub fn envelope_gain(&self, frame: f64) -> f32 {
        let (position, duration, fade_in, fade_out) = if let Some(window) = &self.fade_window {
            (frame + window.offset_frames as f64, window.duration_frames, window.fade_in_frames, window.fade_out_frames)
        } else { (frame, self.duration_frames, self.fade_in_frames, self.fade_out_frames) };
        let ramp = |distance: f64, length: u64| -> f32 {
            if length == 0 || distance >= length as f64 { return 1.0; }
            let x = if length == 1 { 0.0 } else { (distance / (length - 1) as f64).clamp(0.0, 1.0) };
            match self.fade_shape { FadeShape::Linear => x as f32, FadeShape::EqualPower => (x * std::f64::consts::FRAC_PI_2).sin() as f32 }
        };
        ramp(position, fade_in) * ramp(duration.saturating_sub(1) as f64 - position, fade_out)
    }
    pub fn join_right(&self, right: &Self, tempo: &TempoMap) -> Result<Self, MediaError> {
        self.validate()?; right.validate()?;
        let source_adjacent = if self.reversed { right.source_offset_frames + right.duration_frames == self.source_offset_frames }
            else { self.source_offset_frames + self.duration_frames == right.source_offset_frames };
        let delta_frames = (self.end_seconds(tempo) - right.start_seconds(tempo)).abs() * self.source.sample_rate as f64 * self.playback_rate;
        if self.id == right.id || self.track != right.track || self.source.sha256 != right.source.sha256
            || self.source.byte_len != right.source.byte_len || self.source.sample_rate != right.source.sample_rate
            || self.source.channels != right.source.channels || self.source.frames != right.source.frames
            || !source_adjacent || delta_frames > 0.001 {
            return Err(MediaError::Invalid("join requires adjacent regions of the same source"));
        }
        if self.gain != right.gain || self.pan != right.pan || self.muted != right.muted || self.playback_rate != right.playback_rate
            || self.preserve_pitch != right.preserve_pitch || self.reversed != right.reversed || self.fade_shape != right.fade_shape {
            return Err(MediaError::Invalid("join would discard mix settings"));
        }
        let window = match (&self.fade_window, &right.fade_window) {
            (Some(a), Some(b)) if a.duration_frames == b.duration_frames && a.offset_frames + self.duration_frames == b.offset_frames
                && a.fade_in_frames == b.fade_in_frames && a.fade_out_frames == b.fade_out_frames => Some(a.clone()),
            (None, None) if self.fade_out_frames == 0 && right.fade_in_frames == 0 => None,
            _ => return Err(MediaError::Invalid("join would discard internal fades; bounce the selection instead")),
        };
        let mut joined = self.clone();
        joined.source_offset_frames = self.source_offset_frames.min(right.source_offset_frames);
        joined.duration_frames += right.duration_frames;
        joined.fade_in_frames = self.fade_in_frames;
        joined.fade_out_frames = right.fade_out_frames;
        joined.fade_window = window;
        if let Some(window) = &joined.fade_window {
            if window.offset_frames == 0 && window.duration_frames == joined.duration_frames {
                joined.fade_in_frames = window.fade_in_frames; joined.fade_out_frames = window.fade_out_frames;
                joined.fade_window = None;
            }
        }
        joined.validate()?; Ok(joined)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipExtension<C> { schema: String, clips: C }

/// Values are validated before a caller submits `Command::SetTrackData`.
/// None removes the extension rather than serializing an unused empty field.
pub fn clips_value(track: TrackId, clips: &[AudioClip]) -> Result<Option<Value>, MediaError> {
    validate_collection(&track, clips)?;
    serialize_clips(clips)
}

fn serialize_clips(clips: &[AudioClip]) -> Result<Option<Value>, MediaError> {
    if clips.is_empty() { return Ok(None); }
    Ok(Some(serde_json::to_value(ClipExtension { schema: AUDIO_CLIPS_SCHEMA.into(), clips })?))
}

pub fn read_clips(track: &Track) -> Result<Vec<AudioClip>, MediaError> {
    let Some(value) = track.unknown.get(AUDIO_CLIPS_KEY) else { return Ok(Vec::new()); };
    read_clips_value(&track.id, value)
}

/// Read the identical clip extension from a retained history patch without
/// constructing a synthetic track or cloning unrelated track data.
pub fn read_clips_value(track: &TrackId, value: &Value) -> Result<Vec<AudioClip>, MediaError> {
    let extension: ClipExtension<Vec<AudioClip>> = ClipExtension::deserialize(value)?;
    if extension.schema != AUDIO_CLIPS_SCHEMA { return Err(MediaError::Invalid("unsupported audio clip schema")); }
    validate_collection(track, &extension.clips)?;
    Ok(extension.clips)
}

/// For a document builder/importer. Live editor edits should submit the value
/// from `clips_value` through the domain command/journal so undo remains atomic.
pub fn write_clips(track: &mut Track, clips: &[AudioClip]) -> Result<(), MediaError> {
    validate_collection(&track.id, clips)?;
    match serialize_clips(clips)? {
        Some(value) => { track.unknown.insert(AUDIO_CLIPS_KEY.into(), value); }
        None => { track.unknown.remove(AUDIO_CLIPS_KEY); }
    }
    Ok(())
}

fn validate_collection(track: &TrackId, clips: &[AudioClip]) -> Result<(), MediaError> {
    if clips.len() > 100_000 { return Err(MediaError::Limit("audio clip count per track")); }
    let mut ids = BTreeSet::new();
    for clip in clips {
        clip.validate()?;
        if &clip.track != track { return Err(MediaError::Invalid("audio clip track mismatch")); }
        if !ids.insert(clip.id) { return Err(MediaError::Invalid("duplicate audio clip ID")); }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PreparedClip {
    pub id: Uuid,
    pub start: u64,
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: u32,
    pub gain: f32,
    /// Clip pan has already been baked into samples. Kept explicit for the
    /// native StereoClip adapter, whose neutral pan must be used here.
    pub pan: f32,
    pub track: u16,
}

impl PreparedClip {
    pub fn frames(&self) -> u64 { self.samples.len() as u64 / u64::from(self.channels) }
    pub fn end(&self) -> u64 { self.start + self.frames() }
}

pub fn prepare_clip(clip: &AudioClip, audio: &DecodedAudio, tempo: &TempoMap, engine_rate: u32, engine_track: u16) -> Result<PreparedClip, MediaError> {
    audio.validate(ImportLimits::default())?;
    prepare_with_limits(clip, audio, tempo, engine_rate, engine_track, ImportLimits::default())
}

pub(crate) fn prepare_with_limits(clip: &AudioClip, audio: &DecodedAudio, tempo: &TempoMap, engine_rate: u32, engine_track: u16, limits: ImportLimits) -> Result<PreparedClip, MediaError> {
    clip.validate()?; audio.validate_shape(limits)?; clip.source.matches(audio)?;
    if engine_rate == 0 || engine_rate > limits.max_sample_rate { return Err(MediaError::Invalid("engine sample rate")); }
    if engine_track as usize >= verdant_audio::MAX_TRACKS { return Err(MediaError::Invalid("audio track exceeds mixer channel count")); }
    let seconds = clip.start_seconds(tempo);
    let start = (seconds * f64::from(engine_rate)).round();
    if !start.is_finite() || start < 0.0 || start >= u64::MAX as f64 { return Err(MediaError::Invalid("clip timeline position")); }
    let output_frames = ((clip.end_seconds(tempo) * f64::from(engine_rate)).round() - start) as u64;
    if output_frames == 0 { return Err(MediaError::Invalid("clip is shorter than one engine frame")); }
    let output_channels = if audio.channels == 1 && clip.pan != 0.0 { 2 } else { audio.channels };
    limits.pcm_len(output_frames, output_channels)?;
    let begin = usize::try_from(clip.source_offset_frames).ok().and_then(|n| n.checked_mul(audio.channels as usize))
        .ok_or(MediaError::Limit("trim address space"))?;
    let count = usize::try_from(clip.duration_frames).ok().and_then(|n| n.checked_mul(audio.channels as usize))
        .ok_or(MediaError::Limit("trim address space"))?;
    let end = begin.checked_add(count).ok_or(MediaError::Limit("trim address space"))?;
    let input = audio.samples.get(begin..end).ok_or(MediaError::Invalid("clip trim exceeds decoded PCM"))?;
    let (samples, channels) = if engine_rate == audio.sample_rate && begin == 0 && end == audio.samples.len() && clip.pan == 0.0
        && clip.fade_in_frames == 0 && clip.fade_out_frames == 0 && clip.fade_window.is_none()
        && clip.playback_rate == 1.0 && !clip.reversed {
        (audio.samples.clone(), audio.channels)
    } else {
        let mut reversed;
        let input = if clip.reversed {
            reversed = input.to_vec();
            let channels = audio.channels as usize;
            for i in 0..clip.duration_frames as usize / 2 {
                for c in 0..channels { reversed.swap(i * channels + c, (clip.duration_frames as usize - 1 - i) * channels + c); }
            }
            reversed.as_slice()
        } else { input };
        let converted = if clip.playback_rate == 1.0 {
            convert_rate(input, audio.channels, audio.sample_rate, engine_rate, limits)?
        } else if clip.preserve_pitch {
            let stretched = crate::stretch::preserve_pitch(input, audio.channels, audio.sample_rate, clip.playback_rate, limits)?;
            convert_rate(&stretched, audio.channels, audio.sample_rate, engine_rate, limits)?
        } else {
            crate::stretch::resample_to_frames(input, audio.channels, output_frames, limits)?
        };
        let expected = limits.pcm_len(output_frames, audio.channels)?;
        let mut converted = converted;
        // Two rounded conversion stages may differ by one frame.
        if converted.len() > expected { converted.truncate(expected); }
        else if converted.len() < expected {
            let tail = if converted.is_empty() { input[input.len()-audio.channels as usize..].to_vec() }
                else { converted[converted.len()-audio.channels as usize..].to_vec() };
            while converted.len() < expected { converted.extend_from_slice(&tail); }
            converted.truncate(expected);
        }
        let (mut converted, channels) = pan_pcm(converted, audio.channels, clip.pan, limits)?;
        apply_fades(&mut converted, channels, clip, engine_rate);
        if converted.iter().any(|sample| !sample.is_finite()) { return Err(MediaError::Invalid("non-finite prepared PCM")); }
        (Arc::from(converted), channels)
    };
    let start = start as u64;
    start.checked_add(samples.len() as u64 / u64::from(channels)).ok_or(MediaError::Invalid("clip end overflow"))?;
    Ok(PreparedClip { id: clip.id, start, samples, channels, sample_rate: engine_rate,
        gain: if clip.muted { 0.0 } else { clip.gain }, pan: 0.0, track: engine_track })
}

fn apply_fades(samples: &mut [f32], channels: u16, clip: &AudioClip, engine_rate: u32) {
    if clip.fade_in_frames == 0 && clip.fade_out_frames == 0 && clip.fade_window.is_none() { return; }
    let scale = clip.source.sample_rate as f64 * clip.playback_rate / engine_rate as f64;
    for (index, frame) in samples.chunks_exact_mut(channels as usize).enumerate() {
        let gain = clip.envelope_gain(index as f64 * scale);
        if gain != 1.0 { for sample in frame { *sample *= gain; } }
    }
}

fn convert_rate(input: &[f32], channels: u16, from: u32, to: u32, limits: ImportLimits) -> Result<Vec<f32>, MediaError> {
    if from == to { return Ok(input.to_vec()); }
    let frames = (input.len() as f64 / f64::from(channels) * f64::from(to) / f64::from(from)).round() as u64;
    let len = limits.pcm_len(frames, channels)?;
    if channels == 1 { return Ok(verdant_audio::resample(input, from, to)); }
    let mut output = vec![0.0; len];
    let mut channel = Vec::with_capacity(input.len() / channels as usize);
    for c in 0..channels as usize {
        channel.clear(); channel.extend(input.chunks_exact(channels as usize).map(|frame| frame[c]));
        let resampled = verdant_audio::resample(&channel, from, to);
        if resampled.len() != frames as usize { return Err(MediaError::Invalid("resampler frame count")); }
        for (i, sample) in resampled.into_iter().enumerate() { output[i * channels as usize + c] = sample; }
    }
    Ok(output)
}

fn pan_pcm(mut samples: Vec<f32>, channels: u16, pan: f32, limits: ImportLimits) -> Result<(Vec<f32>, u16), MediaError> {
    if pan == 0.0 { return Ok((samples, channels)); }
    let angle = (f64::from(pan) + 1.0) * std::f64::consts::FRAC_PI_4;
    if channels == 1 {
        let len = limits.pcm_len(samples.len() as u64, 2)?;
        let mut stereo = Vec::with_capacity(len);
        let (left, right) = (angle.cos() as f32, angle.sin() as f32);
        for sample in samples { stereo.extend_from_slice(&[sample * left, sample * right]); }
        Ok((stereo, 2))
    } else {
        // Stereo balance leaves the nearer channel at unity, preserving the
        // source's center gain and its stereo image instead of summing to mono.
        let left = (angle.cos() * std::f64::consts::SQRT_2).min(1.0) as f32;
        let right = (angle.sin() * std::f64::consts::SQRT_2).min(1.0) as f32;
        for frame in samples.chunks_exact_mut(2) { frame[0] *= left; frame[1] *= right; }
        Ok((samples, 2))
    }
}
