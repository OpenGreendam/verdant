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

/// Source trim and duration use source-rate frames, independent of project
/// tempo. Only the placement is musical; tempo edits move that placement.
/// Pan is -1 (left)..1 (right); gain is a finite linear multiplier 0..64.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClip {
    pub id: Uuid,
    pub source: AudioSource,
    pub start_tick: i64,
    pub source_offset_frames: u64,
    pub duration_frames: u64,
    /// Linear edge envelopes in source-rate frames. Zero leaves PCM unchanged;
    /// a one-frame fade silences the edge frame. Old documents default to zero.
    #[serde(default)]
    pub fade_in_frames: u64,
    #[serde(default)]
    pub fade_out_frames: u64,
    pub gain: f32,
    pub pan: f32,
    pub muted: bool,
    pub track: TrackId,
}

impl AudioClip {
    pub fn new(source: AudioSource, track: TrackId, start_tick: i64) -> Self {
        let duration_frames = source.frames;
        Self { id: Uuid::new_v4(), source, start_tick, source_offset_frames: 0, duration_frames,
            fade_in_frames: 0, fade_out_frames: 0, gain: 1.0, pan: 0.0, muted: false, track }
    }
    pub fn validate(&self) -> Result<(), MediaError> {
        self.source.validate()?;
        if self.id.is_nil() { return Err(MediaError::Invalid("nil audio clip ID")); }
        if self.start_tick < 0 { return Err(MediaError::Invalid("negative clip start")); }
        if self.duration_frames == 0 || self.source_offset_frames.checked_add(self.duration_frames).is_none_or(|end| end > self.source.frames) {
            return Err(MediaError::Invalid("clip trim exceeds source"));
        }
        if !self.gain.is_finite() || !(0.0..=64.0).contains(&self.gain) { return Err(MediaError::Invalid("clip gain")); }
        if !self.pan.is_finite() || !(-1.0..=1.0).contains(&self.pan) { return Err(MediaError::Invalid("clip pan")); }
        if self.fade_in_frames > self.duration_frames || self.fade_out_frames > self.duration_frames {
            return Err(MediaError::Invalid("fade exceeds clip duration"));
        }
        Ok(())
    }

    /// Copy edit metadata and the source fingerprint, allocating only a new ID.
    pub fn duplicate_at_tick(&self, tick: i64) -> Result<Self, MediaError> {
        let mut copy = self.clone();
        copy.id = Uuid::new_v4();
        copy.start_tick = tick;
        copy.validate()?;
        Ok(copy)
    }

    /// Split by musical placement, rounding exactly once to a decoded-source
    /// frame. The left region keeps its ID; the right receives a fresh ID.
    /// Outer edge fades stay with their edge and are bounded to the new region.
    pub fn split_at_tick(&self, tick: i64, tempo: &TempoMap) -> Result<(Self, Self), MediaError> {
        self.validate()?;
        if tick <= self.start_tick { return Err(MediaError::Invalid("split must be inside clip")); }
        let frames = ((tempo.seconds_at(tick as f64) - tempo.seconds_at(self.start_tick as f64)) * self.source.sample_rate as f64).round();
        if !frames.is_finite() || frames < 1.0 || frames >= self.duration_frames as f64 {
            return Err(MediaError::Invalid("split must be inside clip"));
        }
        let frames = frames as u64;
        let (mut left, mut right) = (self.clone(), self.clone());
        left.duration_frames = frames;
        left.fade_in_frames = left.fade_in_frames.min(frames);
        left.fade_out_frames = 0;
        right.id = Uuid::new_v4();
        right.start_tick = tick;
        right.source_offset_frames += frames;
        right.duration_frames -= frames;
        right.fade_in_frames = 0;
        right.fade_out_frames = right.fade_out_frames.min(right.duration_frames);
        Ok((left, right))
    }

    /// Joining is lossless only for contiguous source and timeline ranges with
    /// matching mix settings and no fade at the removed internal boundary.
    pub fn join_right(&self, right: &Self, tempo: &TempoMap) -> Result<Self, MediaError> {
        self.validate()?; right.validate()?;
        let end_tick = tempo.tick_at(tempo.seconds_at(self.start_tick as f64)
            + self.duration_frames as f64 / self.source.sample_rate as f64).round() as i64;
        if self.id == right.id || self.track != right.track
            || self.source.sha256 != right.source.sha256 || self.source.byte_len != right.source.byte_len
            || self.source.sample_rate != right.source.sample_rate || self.source.channels != right.source.channels
            || self.source.frames != right.source.frames
            || self.source_offset_frames + self.duration_frames != right.source_offset_frames
            || right.start_tick != end_tick {
            return Err(MediaError::Invalid("join requires adjacent regions of the same source"));
        }
        if self.gain != right.gain || self.pan != right.pan || self.muted != right.muted
            || self.fade_out_frames != 0 || right.fade_in_frames != 0 {
            return Err(MediaError::Invalid("join would discard mix settings or internal fades"));
        }
        let mut joined = self.clone();
        joined.duration_frames += right.duration_frames;
        joined.fade_out_frames = right.fade_out_frames;
        joined.validate()?;
        Ok(joined)
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
    let seconds = tempo.seconds_at(clip.start_tick as f64);
    let start = (seconds * f64::from(engine_rate)).round();
    if !start.is_finite() || start < 0.0 || start >= u64::MAX as f64 { return Err(MediaError::Invalid("clip timeline position")); }
    let output_frames = ((clip.duration_frames as f64 * f64::from(engine_rate) / f64::from(audio.sample_rate)).round()) as u64;
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
        && clip.fade_in_frames == 0 && clip.fade_out_frames == 0 {
        (audio.samples.clone(), audio.channels)
    } else {
        let converted = convert_rate(input, audio.channels, audio.sample_rate, engine_rate, limits)?;
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
    if clip.fade_in_frames == 0 && clip.fade_out_frames == 0 { return; }
    let frames = samples.len() / channels as usize;
    let fade_frames = |source_frames: u64| {
        if source_frames == 0 { 0 } else {
            ((source_frames as f64 * engine_rate as f64 / clip.source.sample_rate as f64).round() as usize).max(1).min(frames)
        }
    };
    let (fade_in, fade_out) = (fade_frames(clip.fade_in_frames), fade_frames(clip.fade_out_frames));
    for (i, frame) in samples.chunks_exact_mut(channels as usize).enumerate() {
        let mut gain = 1.0;
        if i < fade_in { gain *= if fade_in == 1 { 0.0 } else { i as f32 / (fade_in - 1) as f32 }; }
        let remaining = frames - 1 - i;
        if remaining < fade_out { gain *= if fade_out == 1 { 0.0 } else { remaining as f32 / (fade_out - 1) as f32 }; }
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
