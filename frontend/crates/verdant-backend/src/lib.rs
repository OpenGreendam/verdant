//! Verdant ↔ `ogd-worker/1`.
//!
//! * [`wire`]: exact request/event types of the existing worker.
//! * [`pinyin`]: the recovered `make-pinyin-score` inventory/context contract.
//! * [`adapter`]: preflight + phrase plan (no invented phonetics).
//! * [`scheduler`]: job lifecycle, cancel overlay, revision-safe binding.
//! * [`artifact`]: path containment, size/SHA-256, WAV/NPZ decoding.
//! * [`cache`]: verified, content-digest-keyed render cache on disk.
//! * [`process`]: the stdio child process.

pub mod adapter;
pub mod automation_plan;
pub mod voices;
pub mod artifact;
pub mod cache;
pub mod f0;
pub mod expression;
pub mod params;
pub mod pinyin;
pub mod phonemes;
pub mod phoneme_timing;
pub mod kana;
pub mod reading;
pub mod post;
pub mod platform;
pub mod process;
pub mod scheduler;
pub mod wire;
pub mod phonemizer;

pub use adapter::{
    BlockedPhrase, Diagnostic, NoteSupport, Phrase, PhrasePlan, Severity, SubSyllableTiming, note_support, plan, plan_all,
    sub_syllable_timing, track_route,
};
pub use cache::{ArtifactCache, CacheError};
pub use process::{WorkerClient, WorkerConfig, WorkerMessage};
pub use scheduler::{Action, CacheEntry, F0Kind, F0Track, Job, JobState, RenderScheduler, SlotStatus, VerifiedAudio, VerifiedF0Track};
pub use wire::{Capabilities, Request, Route, WorkerEvent};
