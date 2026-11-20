use std::{collections::VecDeque, ops::Range, sync::Arc};
use crate::{DecodedAudio, ImportLimits, MediaError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peak { pub min: f32, pub max: f32, pub rms: f32 }

#[derive(Clone, Debug)]
pub struct WaveformPreview {
    pub source_sha256: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub start_frame: u64,
    pub frame_count: u64,
    pub bins: usize,
    /// Interleaved by bin then channel: `peaks[bin * channels + channel]`.
    pub peaks: Arc<[Peak]>,
}

#[derive(Clone, PartialEq, Eq)]
struct SourceKey { hash: [u8; 32], rate: u32, channels: u16, frames: u64 }
#[derive(Clone, PartialEq, Eq)]
struct PreviewKey { start: u64, end: u64, bins: usize }
struct PreviewEntry { key: PreviewKey, value: Arc<WaveformPreview>, bytes: usize }

#[derive(Clone, Copy)]
struct Summary { min: f32, max: f32, squared: f64 }
impl Summary {
    const EMPTY: Self = Self { min: f32::INFINITY, max: f32::NEG_INFINITY, squared: 0.0 };
    fn include(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.squared += other.squared;
    }
    fn sample(&mut self, value: f32) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.squared += f64::from(value) * f64::from(value);
    }
}

struct SourceEntry {
    key: SourceKey,
    block_frames: usize,
    /// Each level doubles the frame span; channels remain interleaved.
    levels: Box<[Box<[Summary]>]>,
    previews: VecDeque<PreviewEntry>,
    bytes: usize,
}

impl SourceEntry {
    fn layout(frames: usize, channels: usize, block_frames: usize) -> (usize, usize) {
        let mut blocks = frames.div_ceil(block_frames);
        let mut levels = 1;
        let mut summaries = blocks;
        while blocks > 1 {
            blocks = blocks.div_ceil(2);
            summaries += blocks;
            levels += 1;
        }
        // Reserve spare deque slots as well as the source/level allocations.
        let bytes = 2 * std::mem::size_of::<Self>()
            + levels * std::mem::size_of::<Box<[Summary]>>()
            + summaries * channels * std::mem::size_of::<Summary>();
        (levels, bytes)
    }

    fn build(audio: &DecodedAudio, key: SourceKey, budget: usize) -> Result<Self, MediaError> {
        let frames = audio.frames() as usize;
        let channels = audio.channels as usize;
        let mut block_frames = 256;
        let (level_count, bytes) = loop {
            let layout = Self::layout(frames, channels, block_frames);
            if layout.1 <= budget { break layout; }
            if block_frames >= frames { return Err(MediaError::Limit("waveform byte limit")); }
            block_frames *= 2;
        };
        let blocks = frames.div_ceil(block_frames);
        let mut first = vec![Summary::EMPTY; blocks * channels];
        // Only the first request for a source scans PCM. Retain summaries, never
        // a second PCM buffer or even an Arc keeping the decoded buffer alive.
        for (block, samples) in audio.samples.chunks(block_frames * channels).enumerate() {
            for frame in samples.chunks_exact(channels) {
                for channel in 0..channels {
                    let sample = frame[channel];
                    if !sample.is_finite() { return Err(MediaError::Invalid("non-finite waveform PCM")); }
                    first[block * channels + channel].sample(sample);
                }
            }
        }
        let mut levels = Vec::with_capacity(level_count);
        levels.push(first.into_boxed_slice());
        while levels.len() < level_count {
            let previous = levels.last().expect("base peak level exists");
            let mut next = vec![Summary::EMPTY; (previous.len() / channels).div_ceil(2) * channels];
            for (block, summaries) in previous.chunks(channels).enumerate() {
                for channel in 0..channels { next[(block / 2) * channels + channel].include(summaries[channel]); }
            }
            levels.push(next.into_boxed_slice());
        }
        Ok(Self { key, block_frames, levels: levels.into_boxed_slice(), previews: VecDeque::new(), bytes })
    }

    fn window(&self, audio: &DecodedAudio, range: Range<usize>, channel: usize) -> Summary {
        let channels = audio.channels as usize;
        let mut result = Summary::EMPTY;
        let mut first = range.start.div_ceil(self.block_frames);
        // A final short base block is complete when the window reaches EOF.
        let end = if range.end == audio.frames() as usize { range.end.div_ceil(self.block_frames) }
            else { range.end / self.block_frames };
        if first >= end {
            for frame in range { result.sample(audio.samples[frame * channels + channel]); }
            return result;
        }
        // Partial blocks must use their exact PCM edges: including an entire
        // neighbouring block would leak peaks outside a trim or pixel window.
        for frame in range.start..first * self.block_frames {
            result.sample(audio.samples[frame * channels + channel]);
        }
        while first < end {
            let remaining = end - first;
            let level = first.trailing_zeros().min(usize::BITS - 1 - remaining.leading_zeros()) as usize;
            result.include(self.levels[level][(first >> level) * channels + channel]);
            first += 1usize << level;
        }
        for frame in (end * self.block_frames).min(range.end)..range.end {
            result.sample(audio.samples[frame * channels + channel]);
        }
        result
    }

    fn preview(&self, audio: &DecodedAudio, key: &PreviewKey) -> Arc<WaveformPreview> {
        let frames = key.end - key.start;
        let mut peaks = Vec::with_capacity(key.bins * audio.channels as usize);
        for bin in 0..key.bins {
            let first = key.start + (u128::from(frames) * bin as u128 / key.bins as u128) as u64;
            let end = key.start + (u128::from(frames) * (bin + 1) as u128 / key.bins as u128) as u64;
            for channel in 0..audio.channels as usize {
                let summary = self.window(audio, first as usize..end as usize, channel);
                peaks.push(Peak { min: summary.min, max: summary.max, rms: (summary.squared / (end - first) as f64).sqrt() as f32 });
            }
        }
        Arc::new(WaveformPreview { source_sha256: audio.sha256.clone(), sample_rate: audio.sample_rate,
            channels: audio.channels, start_frame: key.start, frame_count: frames, bins: key.bins, peaks: peaks.into() })
    }
}

/// A byte-bounded LRU of source peak pyramids and trim/zoom previews.
///
/// A source is identified by SHA-256, sample rate, channels and frame count;
/// each preview additionally includes the exact trim range and bin count.
/// Zoom/trim requests share its multiresolution min/max/squared-sum pyramid.
/// Only partial base blocks read the caller's PCM again. Base blocks start at
/// 256 frames and grow for sources whose pyramid would exceed half the budget.
/// No decoding, PCM copies or retained decoded buffers occur here.
///
/// The byte budget includes peak storage and entry bookkeeping. Eviction drops
/// the least recently used source and its previews; within a source older
/// previews are discarded first. Returned Arcs may outlive eviction, so memory
/// owned by live UI previews is outside the cache's retention budget.
pub struct WaveformCache {
    max_bytes: usize, max_bins: usize, bytes: usize, entries: VecDeque<SourceEntry>,
}

impl Default for WaveformCache {
    fn default() -> Self { Self { max_bytes: 8 * 1024 * 1024, max_bins: 8192, bytes: 0, entries: VecDeque::new() } }
}

impl WaveformCache {
    pub fn new(max_bytes: usize, max_bins: usize) -> Result<Self, MediaError> {
        if max_bytes < 512 || max_bins == 0 || max_bins > 1_048_576 { return Err(MediaError::Invalid("waveform cache bounds")); }
        Ok(Self { max_bytes, max_bins, bytes: 0, entries: VecDeque::new() })
    }
    pub fn retained_bytes(&self) -> usize { self.bytes }
    /// Number of retained sources, including all of each source's previews.
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn clear(&mut self) { self.entries = VecDeque::new(); self.bytes = 0; }

    pub fn preview(&mut self, audio: &DecodedAudio, range: Range<u64>, bins: usize) -> Result<Arc<WaveformPreview>, MediaError> {
        audio.validate_shape(ImportLimits::default())?;
        if bins == 0 || bins > self.max_bins { return Err(MediaError::Limit("waveform bin limit")); }
        if range.start >= range.end || range.end > audio.frames() { return Err(MediaError::Invalid("waveform source range")); }
        let bins = bins.min(usize::try_from(range.end - range.start).unwrap_or(usize::MAX));
        let mut hash = [0; 32];
        hex::decode_to_slice(&audio.sha256, &mut hash).map_err(|_| MediaError::Invalid("source fingerprint"))?;
        let source_key = SourceKey { hash, rate: audio.sample_rate, channels: audio.channels, frames: audio.frames() };
        let key = PreviewKey { start: range.start, end: range.end, bins };
        let count = bins.checked_mul(audio.channels as usize).ok_or(MediaError::Limit("waveform address space"))?;
        let bytes = count.checked_mul(std::mem::size_of::<Peak>())
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<PreviewEntry>() + std::mem::size_of::<WaveformPreview>() + 64 + 32))
            .ok_or(MediaError::Limit("waveform byte limit"))?;
        if bytes > self.max_bytes { return Err(MediaError::Limit("waveform byte limit")); }

        let position = self.entries.iter().position(|entry| entry.key == source_key);
        let mut source = match position {
            Some(position) => self.entries.remove(position).expect("cache source position exists"),
            None => SourceEntry::build(audio, source_key, (self.max_bytes / 2).min(self.max_bytes - bytes))?,
        };
        if position.is_some() { self.bytes -= source.bytes; }
        if let Some(position) = source.previews.iter().position(|entry| entry.key == key) {
            let entry = source.previews.remove(position).expect("cache preview position exists");
            let result = entry.value.clone();
            source.previews.push_front(entry);
            self.bytes += source.bytes;
            self.entries.push_front(source);
            return Ok(result);
        }
        // Make room within this source before evicting other sources. The
        // pyramid itself never changes with trim or zoom, and is never rescanned.
        while source.bytes > self.max_bytes - bytes || source.previews.len() >= 128 {
            match source.previews.pop_back() {
                Some(entry) => source.bytes -= entry.bytes,
                None => {
                    if position.is_some() { self.bytes += source.bytes; self.entries.push_front(source); }
                    return Err(MediaError::Limit("waveform byte limit"));
                }
            }
        }
        let value = source.preview(audio, &key);
        source.bytes += bytes;
        source.previews.push_front(PreviewEntry { key, value: value.clone(), bytes });
        while self.bytes > self.max_bytes - source.bytes || self.entries.len() >= 128 {
            if let Some(entry) = self.entries.pop_back() { self.bytes -= entry.bytes; } else { break; }
        }
        self.bytes += source.bytes;
        self.entries.push_front(source);
        Ok(value)
    }
}
