//! Verdant domain layer: the project document and its single writer.
//!
//! Project persistence lives here; GPUI, audio callbacks and backend process
//! execution stay outside the domain (SPEC §2 ownership table).

pub mod command;
pub mod digest;
pub mod ids;
mod interval;
pub mod io;
pub mod journal;
pub mod model;
pub mod persistence;
pub mod pronunciation;
pub mod store;
pub mod synthesis;
pub mod timing;

pub use command::{Command, CommandBatch, Edge};
pub use ids::{NoteId, PointId, Revision, TrackId};
pub use journal::JournalError;
pub use model::{Note, PPQ, Project, Pronunciation, PronunciationSource, TRACK_COLORS, Track, TrackHeader, Vibrato};
pub use persistence::{PersistenceError, ProjectPersistence, RecoveryReport};
pub use pronunciation::{HANZI_PINYIN_PROFILE, hanzi_readings, hanzi_primary_reading, pronunciation_for_lyric, parse_phonetic_lyric, pronunciation_display};
pub use store::{Commit, CommitKind, DomainError, ProjectStore};
pub use synthesis::{ConsonantDynamicsSettings, ModelDynamicsSettings, PhonemeTimingSettings, PhoneticCorrectionSettings, SynthesisSettings, VocalTractSettings, VocoderHighbandSettings, WidebandSettings};
pub use synthesis::OutputTransientsSettings;
pub use timing::{MeterMap, MusicalPosition, TempoMap};
pub mod automation;
pub mod editing;
pub mod parts;
