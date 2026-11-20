use std::{io::{Cursor, ErrorKind}, sync::Arc};
use sha2::{Digest, Sha256};
use symphonia::core::{audio::{Channels, SampleBuffer}, codecs::{CODEC_TYPE_NULL, DecoderOptions},
    errors::Error as DecodeError, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint};
use crate::MediaError;

#[derive(Clone, Copy, Debug)]
pub struct ImportLimits {
    pub max_source_bytes: u64,
    pub max_decoded_frames: u64,
    /// Bounds both decoded PCM and each prepared clip, in bytes.
    pub max_pcm_bytes: usize,
    pub max_project_pcm_bytes: usize,
    pub max_sample_rate: u32,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self { max_source_bytes: 512 * 1024 * 1024, max_decoded_frames: 192_000 * 60 * 60,
            max_pcm_bytes: 512 * 1024 * 1024, max_project_pcm_bytes: 1024 * 1024 * 1024,
            max_sample_rate: 384_000 }
    }
}

impl ImportLimits {
    pub(crate) fn validate(&self) -> Result<(), MediaError> {
        if self.max_source_bytes == 0 || self.max_decoded_frames == 0 || self.max_pcm_bytes < 4
            || self.max_project_pcm_bytes < 4 || self.max_sample_rate == 0 {
            return Err(MediaError::Invalid("import limits must be positive"));
        }
        Ok(())
    }
    pub(crate) fn pcm_len(&self, frames: u64, channels: u16) -> Result<usize, MediaError> {
        if frames > self.max_decoded_frames { return Err(MediaError::Limit("decoded frame limit")); }
        let samples = frames.checked_mul(u64::from(channels)).and_then(|n| usize::try_from(n).ok())
            .ok_or(MediaError::Limit("PCM address space"))?;
        if samples.checked_mul(std::mem::size_of::<f32>()).is_none_or(|n| n > self.max_pcm_bytes) {
            return Err(MediaError::Limit("PCM byte limit"));
        }
        Ok(samples)
    }
}

/// Immutable interleaved PCM. Its fingerprint is from the exact encoded bytes
/// supplied to the decoder, not a pathname/mtime or a hash of the output WAV.
#[derive(Clone, Debug)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Arc<[f32]>,
    pub sha256: String,
    pub byte_len: u64,
}

impl DecodedAudio {
    pub fn frames(&self) -> u64 { self.samples.len() as u64 / u64::from(self.channels.max(1)) }
    pub fn duration_seconds(&self) -> f64 { self.frames() as f64 / f64::from(self.sample_rate) }
    pub(crate) fn validate(&self, limits: ImportLimits) -> Result<(), MediaError> {
        self.validate_shape(limits)?;
        if self.samples.iter().any(|sample| !sample.is_finite()) { return Err(MediaError::Invalid("non-finite PCM sample")); }
        Ok(())
    }
    pub(crate) fn validate_shape(&self, limits: ImportLimits) -> Result<(), MediaError> {
        limits.validate()?;
        if !(1..=2).contains(&self.channels) { return Err(MediaError::Channels(self.channels as usize)); }
        if self.sample_rate == 0 || self.sample_rate > limits.max_sample_rate { return Err(MediaError::Invalid("sample rate")); }
        if self.samples.is_empty() || self.samples.len() % self.channels as usize != 0 { return Err(MediaError::Invalid("empty or incomplete PCM frames")); }
        if !valid_sha256(&self.sha256) || self.byte_len == 0 { return Err(MediaError::Invalid("source fingerprint")); }
        limits.pcm_len(self.frames(), self.channels)?;
        Ok(())
    }
}

pub(crate) fn valid_sha256(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Decode a caller-owned byte snapshot. The extension is only a probe hint;
/// container bytes, not the filename, determine the format.
pub fn decode_bytes(bytes: &[u8], extension: Option<&str>, limits: ImportLimits) -> Result<DecodedAudio, MediaError> {
    limits.validate()?;
    if bytes.len() as u64 > limits.max_source_bytes { return Err(MediaError::Limit("source byte limit")); }
    decode_owned(bytes.to_vec(), extension, limits)
}

pub(crate) fn decode_owned(bytes: Vec<u8>, extension: Option<&str>, limits: ImportLimits) -> Result<DecodedAudio, MediaError> {
    limits.validate()?;
    if bytes.is_empty() { return Err(MediaError::Invalid("empty source")); }
    if bytes.len() as u64 > limits.max_source_bytes { return Err(MediaError::Limit("source byte limit")); }
    validate_riff_wave(&bytes)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let byte_len = bytes.len() as u64;
    let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = extension { hint.with_extension(extension.trim_start_matches('.')); }
    let options = FormatOptions { enable_gapless: true, ..Default::default() };
    let probed = symphonia::default::get_probe().format(&hint, stream, &options, &MetadataOptions::default())
        .map_err(|e| MediaError::Decode(e.to_string()))?;
    let mut format = probed.format;
    let track = format.default_track().filter(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .or_else(|| format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL))
        .ok_or(MediaError::Invalid("container has no audio track"))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions { verify: true })
        .map_err(|e| MediaError::Decode(e.to_string()))?;
    let mut sample_rate = None;
    let mut channel_count = None;
    let mut scratch: Option<SampleBuffer<f32>> = None;
    let mut samples = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(MediaError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id { continue; }
        // A damaged packet must not disappear from the imported arrangement.
        let decoded = decoder.decode(&packet).map_err(|e| MediaError::Decode(e.to_string()))?;
        let spec = *decoded.spec();
        let channels = spec.channels.count();
        if !(1..=2).contains(&channels) { return Err(MediaError::Channels(channels)); }
        if channels == 2 && spec.channels != (Channels::FRONT_LEFT | Channels::FRONT_RIGHT) {
            return Err(MediaError::Invalid("stereo channel layout is not left/right"));
        }
        if spec.rate == 0 || spec.rate > limits.max_sample_rate { return Err(MediaError::Invalid("sample rate")); }
        if sample_rate.is_some_and(|rate| rate != spec.rate) || channel_count.is_some_and(|count| count != channels as u16) {
            return Err(MediaError::Invalid("mid-stream rate/channel change"));
        }
        sample_rate = Some(spec.rate); channel_count = Some(channels as u16);
        let total_frames = (samples.len() / channels).checked_add(decoded.frames()).ok_or(MediaError::Limit("decoded frame limit"))?;
        limits.pcm_len(total_frames as u64, channels as u16)?;
        let capacity = limits.pcm_len(decoded.capacity() as u64, channels as u16)?;
        if scratch.as_ref().is_none_or(|s| s.capacity() < capacity) {
            scratch = Some(SampleBuffer::new(decoded.capacity() as u64, spec));
        }
        let scratch = scratch.as_mut().expect("sample buffer allocated for decoded packet");
        scratch.copy_interleaved_ref(decoded);
        if scratch.samples().iter().any(|s| !s.is_finite()) { return Err(MediaError::Invalid("non-finite PCM sample")); }
        samples.try_reserve(scratch.len()).map_err(|_| MediaError::Limit("PCM allocation"))?;
        samples.extend_from_slice(scratch.samples());
    }
    if decoder.finalize().verify_ok == Some(false) { return Err(MediaError::Decode("codec checksum verification failed".into())); }
    let audio = DecodedAudio { sample_rate: sample_rate.ok_or(MediaError::Invalid("no decoded audio"))?,
        channels: channel_count.ok_or(MediaError::Invalid("no decoded audio"))?, samples: samples.into(), sha256, byte_len };
    audio.validate_shape(limits)?;
    Ok(audio)
}

/// A demuxer's normal EOF and a truncated WAV can both be UnexpectedEof.
/// Check RIFF framing first so a short source cannot be accepted as a shorter
/// valid clip. Other admitted containers retain their decoder/checksum errors.
fn validate_riff_wave(bytes: &[u8]) -> Result<(), MediaError> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" { return Ok(()); }
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let end = u32_at(4).checked_add(8).ok_or(MediaError::Invalid("RIFF length overflow"))?;
    if end > bytes.len() || end < 12 { return Err(MediaError::Invalid("truncated RIFF/WAVE")); }
    let mut at = 12usize;
    let mut block_align = None;
    let mut data_length = None;
    while at < end {
        if end - at < 8 { return Err(MediaError::Invalid("truncated RIFF chunk header")); }
        let size = u32_at(at + 4);
        let body = at + 8;
        let chunk_end = body.checked_add(size).ok_or(MediaError::Invalid("RIFF chunk overflow"))?;
        if chunk_end > end { return Err(MediaError::Invalid("truncated RIFF chunk")); }
        match &bytes[at..at + 4] {
            b"fmt " => {
                if size < 16 { return Err(MediaError::Invalid("short WAVE format chunk")); }
                let align = u16::from_le_bytes(bytes[body + 12..body + 14].try_into().unwrap());
                if align == 0 { return Err(MediaError::Invalid("zero WAVE block alignment")); }
                if block_align.replace(align as usize).is_some() { return Err(MediaError::Invalid("duplicate WAVE format chunk")); }
            }
            b"data" => {
                if data_length.replace(size).is_some() { return Err(MediaError::Invalid("duplicate WAVE sample data")); }
            },
            _ => {}
        }
        at = chunk_end.checked_add(size & 1).ok_or(MediaError::Invalid("RIFF padding overflow"))?;
        if at > end { return Err(MediaError::Invalid("truncated RIFF padding")); }
    }
    let align = block_align.ok_or(MediaError::Invalid("missing WAVE format chunk"))?;
    if data_length.is_none_or(|length| length == 0 || length % align != 0) {
        return Err(MediaError::Invalid("missing, duplicate or incomplete WAVE sample data"));
    }
    Ok(())
}
