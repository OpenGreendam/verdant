//! Native MIDI capture, with one timestamp epoch per connection.
//!
//! Enumerate actual ports, open the selected stable native identifier, drain
//! the bounded queue from the UI thread, and stop before committing a take.
//! Capture defaults to all channels (`None`); pass a zero-based channel to
//! filter notes and controllers before queueing, preserving channel-local
//! sustain. Only the selected native port is opened, even for duplicate names.
//! A rejected callback invalidates the take: it must never become a partial
//! successful recording. [`MidiTake`] extends released notes through sustain,
//! then maps absolute note boundaries through the project's [`TempoMap`].
//! [`notes_to_batch`] produces one existing domain transaction/undo entry.
//!
//! Device discovery/opening is fallible; there is no synthetic device fallback.
//! This crate does not send MIDI or create a native/audio/UI event loop.

mod device;
mod event;
mod take;

pub use device::{CallbackRejection, MidiCapture, MidiConnection, MidiInputService, MidiPort};
pub use event::{CapturedEvent, EventKind, parse_message};
pub use take::{CaptureOrigin, CapturedNote, MidiTake, notes_to_batch};
pub use verdant_domain::TempoMap;

#[derive(Clone, Debug, thiserror::Error)]
pub enum MidiError {
    #[error("native MIDI input: {0}")]
    Native(String),
    #[error("the selected MIDI input is no longer available")]
    PortUnavailable,
    #[error("MIDI channel must be 0..15")]
    InvalidChannel,
    #[error("MIDI capture queue capacity must be 1..1048576")]
    InvalidCapacity,
    #[error("malformed MIDI channel message")]
    MalformedMessage,
    #[error("MIDI callback rejected the take: {0}")]
    CallbackRejected(CallbackRejection),
    #[error("MIDI event timestamps must be nondecreasing and at/after capture origin")]
    TimestampOrder,
    #[error("MIDI take exceeds the bounded note count")]
    TakeLimit,
    #[error("MIDI take cannot be represented by the project tempo/tick range")]
    TickRange,
}
