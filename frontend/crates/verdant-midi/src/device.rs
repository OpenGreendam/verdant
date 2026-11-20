use std::sync::{Arc, atomic::{AtomicU8, AtomicU64, Ordering}, mpsc::{self, Receiver, TryRecvError, TrySendError}};
use std::time::Instant;
use midir::{Ignore, MidiInput, MidiInputConnection};
use crate::{CapturedEvent, MidiError, parse_message};

const RUNNING: u8 = 0;
const STOPPED: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiPort {
    /// The backend's stable port identity, never a display-list index.
    pub id: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[repr(u8)]
pub enum CallbackRejection {
    #[error("capture queue overflowed")]
    QueueFull = 2,
    #[error("capture receiver closed")]
    ReceiverClosed = 3,
    #[error("consumer rejected an event")]
    ConsumerRejected = 4,
    #[error("malformed native MIDI message")]
    MalformedMessage = 5,
    #[error("native MIDI timestamps moved backwards")]
    TimestampOrder = 6,
}

impl CallbackRejection {
    fn from_state(state: u8) -> Option<Self> {
        match state {
            2 => Some(Self::QueueFull), 3 => Some(Self::ReceiverClosed),
            4 => Some(Self::ConsumerRejected), 5 => Some(Self::MalformedMessage),
            6 => Some(Self::TimestampOrder), _ => None,
        }
    }
}

pub struct MidiInputService;

impl MidiInputService {
    pub fn ports() -> Result<Vec<MidiPort>, MidiError> {
        let input = MidiInput::new("Verdant input discovery").map_err(|e| MidiError::Native(e.to_string()))?;
        input.ports().into_iter().map(|port| {
            Ok(MidiPort { id: port.id(), name: input.port_name(&port).map_err(|e| MidiError::Native(e.to_string()))? })
        }).collect()
    }

    /// Open only `port.id`, using its stable native identity rather than its
    /// display name. `None` records all channels (the default); `Some(0..=15)`
    /// filters in the native callback before an event enters the bounded queue.
    /// The queue never blocks the native callback. Overflow rejects the entire
    /// take instead of silently dropping note-offs or leaving sustained notes.
    pub fn open(port: &MidiPort, channel: Option<u8>, capacity: usize) -> Result<MidiCapture, MidiError> {
        if !(1..=1_048_576).contains(&capacity) { return Err(MidiError::InvalidCapacity); }
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let connection = Self::open_with_callback(port, channel, move |event| {
            sender.try_send(event).map_err(|e| match e {
                TrySendError::Full(_) => CallbackRejection::QueueFull,
                TrySendError::Disconnected(_) => CallbackRejection::ReceiverClosed,
            })
        })?;
        Ok(MidiCapture { connection, receiver })
    }

    /// Called on the backend's MIDI thread. The consumer must return promptly,
    /// avoid UI calls, and reject explicitly if it cannot retain an event.
    /// `stop` closes and joins the native input before returning to the caller.
    pub fn open_with_callback<F>(port: &MidiPort, channel: Option<u8>, deliver: F) -> Result<MidiConnection, MidiError>
    where F: FnMut(CapturedEvent) -> Result<(), CallbackRejection> + Send + 'static {
        if channel.is_some_and(|c| c > 15) { return Err(MidiError::InvalidChannel); }
        let mut input = MidiInput::new("Verdant MIDI input").map_err(|e| MidiError::Native(e.to_string()))?;
        input.ignore(Ignore::None);
        let native_port = input.find_port_by_id(&port.id).ok_or(MidiError::PortUnavailable)?;
        let started = Instant::now();
        let state = Arc::new(AtomicU8::new(RUNNING));
        let latest = Arc::new(AtomicU64::new(0));
        let mut dispatch = Dispatch { deliver, state: state.clone(), latest: latest.clone(), started, channel, native_origin: None, last_native: None };
        let native = input.connect(&native_port, "Verdant recording", move |timestamp, bytes, _| {
            dispatch.message(timestamp, bytes);
        }, ()).map_err(|e| MidiError::Native(e.to_string()))?;
        Ok(MidiConnection { native: Some(native), state, latest, started })
    }
}

/// Owns the real OS input connection. Drop also closes and joins it.
/// A rejection remains observable after `stop`; it is never reset to success.
pub struct MidiConnection {
    native: Option<MidiInputConnection<()>>,
    state: Arc<AtomicU8>,
    latest: Arc<AtomicU64>,
    started: Instant,
}

impl MidiConnection {
    pub fn timestamp_micros(&self) -> u64 { micros(self.started).max(self.latest.load(Ordering::Acquire)) }
    pub fn is_running(&self) -> bool { self.state.load(Ordering::Acquire) == RUNNING }
    pub fn check(&self) -> Result<(), MidiError> {
        match CallbackRejection::from_state(self.state.load(Ordering::Acquire)) {
            Some(reason) => Err(MidiError::CallbackRejected(reason)), None => Ok(()),
        }
    }
    pub fn stop(&mut self) -> Result<(), MidiError> {
        let _ = self.state.compare_exchange(RUNNING, STOPPED, Ordering::AcqRel, Ordering::Acquire);
        if let Some(native) = self.native.take() { let _ = native.close(); }
        self.check()
    }
}

impl Drop for MidiConnection {
    fn drop(&mut self) { let _ = self.stop(); }
}

pub struct MidiCapture {
    connection: MidiConnection,
    receiver: Receiver<CapturedEvent>,
}

impl MidiCapture {
    pub fn timestamp_micros(&self) -> u64 { self.connection.timestamp_micros() }
    pub fn is_running(&self) -> bool { self.connection.is_running() }
    pub fn try_recv(&self) -> Result<Option<CapturedEvent>, MidiError> {
        self.connection.check()?;
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => Ok(None),
        }
    }
    /// Stop before finishing the take. The returned tail includes every event
    /// accepted before the input was closed. Rejections return an error instead.
    pub fn stop(&mut self) -> Result<Vec<CapturedEvent>, MidiError> {
        self.connection.stop()?;
        Ok(self.receiver.try_iter().collect())
    }
}

fn micros(started: Instant) -> u64 { started.elapsed().as_micros().try_into().unwrap_or(u64::MAX) }

struct Dispatch<F> {
    deliver: F,
    state: Arc<AtomicU8>,
    latest: Arc<AtomicU64>,
    started: Instant,
    channel: Option<u8>,
    native_origin: Option<(u64, u64)>,
    last_native: Option<u64>,
}

impl<F: FnMut(CapturedEvent) -> Result<(), CallbackRejection>> Dispatch<F> {
    fn reject(&self, reason: CallbackRejection) {
        let _ = self.state.compare_exchange(RUNNING, reason as u8, Ordering::AcqRel, Ordering::Acquire);
    }
    fn message(&mut self, native_time: u64, bytes: &[u8]) {
        if self.state.load(Ordering::Acquire) != RUNNING { return; }
        let event = match parse_message(0, bytes) {
            Ok(Some(event)) => event, Ok(None) => return,
            Err(_) => { self.reject(CallbackRejection::MalformedMessage); return; }
        };
        if self.channel.is_some_and(|c| c != event.channel) { return; }
        if self.last_native.is_some_and(|previous| native_time < previous) {
            self.reject(CallbackRejection::TimestampOrder); return;
        }
        self.last_native = Some(native_time);
        // Native backends use an unspecified timestamp origin. Calibrate its
        // first event to this connection's monotonic host clock, preserving
        // the backend's subsequent inter-event timing and the initial wait.
        let (origin, host) = *self.native_origin.get_or_insert_with(|| (native_time, micros(self.started)));
        let Some(timestamp_micros) = host.checked_add(native_time - origin) else {
            self.reject(CallbackRejection::TimestampOrder); return;
        };
        self.latest.store(timestamp_micros, Ordering::Release);
        if let Err(reason) = (self.deliver)(CapturedEvent { timestamp_micros, ..event }) {
            self.reject(reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_delivery_rejects_without_losing_note_off_as_success() {
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (tx, rx) = mpsc::sync_channel(1);
        let mut dispatch = Dispatch {
            deliver: move |event| tx.try_send(event).map_err(|_| CallbackRejection::QueueFull),
            state: state.clone(), latest: Arc::new(AtomicU64::new(0)), started: Instant::now(), channel: None,
            native_origin: None, last_native: None,
        };
        dispatch.message(900_000, &[0x90, 60, 100]);
        dispatch.message(901_000, &[0x80, 60, 0]);
        assert_eq!(state.load(Ordering::Acquire), CallbackRejection::QueueFull as u8);
        assert!(matches!(rx.try_recv().unwrap().kind, crate::EventKind::NoteOn { .. }));
        dispatch.message(902_000, &[0x90, 61, 100]);
        assert!(rx.try_recv().is_err());
    }
    #[test]
    fn native_epoch_retains_deltas_and_stop_fences_delivery() {
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (tx, rx) = mpsc::sync_channel(4);
        let mut dispatch = Dispatch {
            deliver: move |e| { tx.try_send(e).unwrap(); Ok(()) }, state: state.clone(), latest: Arc::new(AtomicU64::new(0)),
            started: Instant::now(), channel: Some(2), native_origin: None, last_native: None,
        };
        dispatch.message(1_000_000, &[0x92, 69, 99]);
        dispatch.message(1_125_000, &[0x82, 69, 0]);
        let first = rx.try_recv().unwrap(); let second = rx.try_recv().unwrap();
        assert_eq!(second.timestamp_micros - first.timestamp_micros, 125_000);
        state.store(STOPPED, Ordering::Release);
        dispatch.message(1_130_000, &[0x92, 70, 99]);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn channel_three_capture_filters_before_queue_and_materialization_with_sustain() {
        // MIDI's user-facing channel 3 is status nibble 2. The queue fits only
        // its four events, so filtering after delivery would reject this take.
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (tx, rx) = mpsc::sync_channel(4);
        let mut dispatch = Dispatch {
            deliver: move |event| tx.try_send(event).map_err(|_| CallbackRejection::QueueFull),
            state: state.clone(), latest: Arc::new(AtomicU64::new(10_000)), started: Instant::now(),
            channel: Some(2), native_origin: Some((9_000_000, 10_000)), last_native: None,
        };
        for (offset, bytes) in [
            (100_000, [0x90, 60, 80]),
            (200_000, [0x92, 69, 103]),
            (300_000, [0xb2, 64, 127]),
            (400_000, [0x82, 69, 0]),
            (500_000, [0xb0, 64, 0]),
            (600_000, [0xb0, 120, 0]),
            (700_000, [0xb2, 64, 0]),
            (800_000, [0x80, 60, 0]),
        ] {
            dispatch.message(9_000_000 + offset, &bytes);
        }
        assert_eq!(state.load(Ordering::Acquire), RUNNING);
        let events = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(events.iter().map(|event| (event.timestamp_micros, event.channel)).collect::<Vec<_>>(),
            vec![(210_000, 2), (310_000, 2), (410_000, 2), (710_000, 2)]);
        // Leave the take itself unrestricted to prove the capture filtered it.
        let mut take = crate::MidiTake::new(crate::CaptureOrigin { timestamp_micros: 10_000, timeline_tick: 480 }, None).unwrap();
        for event in events { take.push(event).unwrap(); }
        let notes = take.finish(910_000, &crate::TempoMap::constant(120.0)).unwrap();
        assert_eq!(notes, vec![crate::CapturedNote {
            start_tick: 864, duration_tick: 960, midi_note: 69, velocity: 103, channel: 2,
        }]);
    }

    #[test]
    fn default_all_channel_capture_keeps_other_channels_and_their_pedals() {
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (tx, rx) = mpsc::sync_channel(8);
        let mut dispatch = Dispatch {
            deliver: move |event| tx.try_send(event).map_err(|_| CallbackRejection::QueueFull),
            state: state.clone(), latest: Arc::new(AtomicU64::new(10_000)), started: Instant::now(),
            channel: None, native_origin: Some((9_000_000, 10_000)), last_native: None,
        };
        dispatch.message(9_100_000, &[0x90, 60, 80]);
        dispatch.message(9_200_000, &[0x92, 69, 103]);
        dispatch.message(9_300_000, &[0xb2, 64, 127]);
        dispatch.message(9_400_000, &[0x82, 69, 0]);
        dispatch.message(9_500_000, &[0xb0, 64, 0]);
        dispatch.message(9_600_000, &[0xb0, 120, 0]);
        dispatch.message(9_700_000, &[0xb2, 64, 0]);
        dispatch.message(9_800_000, &[0x80, 60, 0]);
        assert_eq!(state.load(Ordering::Acquire), RUNNING);
        let mut take = crate::MidiTake::new(crate::CaptureOrigin { timestamp_micros: 10_000, timeline_tick: 480 }, None).unwrap();
        for event in rx.try_iter() { take.push(event).unwrap(); }
        let notes = take.finish(910_000, &crate::TempoMap::constant(120.0)).unwrap();
        assert_eq!(notes, vec![
            crate::CapturedNote { start_tick: 672, duration_tick: 960, midi_note: 60, velocity: 80, channel: 0 },
            crate::CapturedNote { start_tick: 864, duration_tick: 960, midi_note: 69, velocity: 103, channel: 2 },
        ]);
    }
}
