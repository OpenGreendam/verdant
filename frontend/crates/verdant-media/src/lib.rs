//! User-asset audio import and preparation, outside the realtime callback.
//!
//! [`MediaImporter`] opens regular files only within explicitly authorized
//! canonical roots, individually selected files, or the selected native
//! document's directory for strict portable relative references. Absolute
//! references still require their own explicit authorization. Source SHA-256 binds the
//! persisted clip to the same bytes on reload; a changed/missing asset is an
//! error, never substitute silence. Imported source files are never modified.
//!
//! Symphonia's admitted codecs include PCM16/24/32 and float WAV, AIFF/CAF,
//! FLAC, MPEG audio, AAC/ALAC in supported MP4 containers, and Vorbis/Ogg.
//! The selected container still needs a supported mono/stereo codec. Opus,
//! DRM and arbitrary multichannel downmix are not implemented by this decoder.
//!
//! [`AudioClip`] documents are stored in `Track.unknown["audio_clips"]` as a
//! versioned extension. They store source identity and edit metadata, never PCM.
//! [`prepare_clip`] performs source trim, bandlimited rate conversion and clip
//! pan once. Its immutable PCM is shared with playback, offline mix, stems and
//! range export; none of those consumers should decode or resample separately.
//!
//! [`WaveformCache`] borrows that already decoded PCM to build a source peak
//! pyramid once. Exact trim/zoom previews reuse its min/max and RMS summaries,
//! reading PCM only for partial base blocks. Source SHA-256, rate and shape bind
//! the pyramid; source-frame trim bounds and bin count bind each preview. The
//! byte-bounded source LRU never owns decoded PCM, and UI-held preview Arcs
//! remain valid after a trim, source eviction or cache clear.

mod asset;
mod clip;
mod decode;
mod preview;
mod stretch;

pub use asset::{AssetPolicy, ImportedAudio, MediaImporter};
pub use clip::{AUDIO_CLIPS_KEY, AUDIO_CLIPS_SCHEMA, AudioClip, AudioSource, ClipFadeWindow, FadeShape, PreparedClip,
    clips_value, prepare_clip, read_clips, read_clips_value, write_clips};
pub use decode::{DecodedAudio, ImportLimits, decode_bytes};
pub use preview::{Peak, WaveformCache, WaveformPreview};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("audio asset I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio asset is outside the explicitly authorized locations")]
    UnauthorizedPath,
    #[error("audio asset must be a regular file with an absolute or portable project-relative path")]
    InvalidPath,
    #[error("audio asset path cannot be represented by the project format")]
    NonUnicodePath,
    #[error("audio asset changed while being opened/read, or differs from its saved fingerprint")]
    SourceChanged,
    #[error("audio decoder: {0}")]
    Decode(String),
    #[error("audio import supports mono or stereo; received {0} channels")]
    Channels(usize),
    #[error("audio import exceeded {0}")]
    Limit(&'static str),
    #[error("invalid audio clip: {0}")]
    Invalid(&'static str),
    #[error("audio clip document: {0}")]
    Json(#[from] serde_json::Error),
}
