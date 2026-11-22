use crate::MidiError;

/// Connection-relative host microseconds, not wall-clock time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedEvent {
    pub timestamp_micros: u64,
    /// Zero based MIDI channel (0..15).
    pub channel: u8,
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Control { controller:u8, value:u8 },
    PitchBend { value:i16 },
    ChannelPressure { pressure:u8 },
    PolyPressure { midi_note:u8, pressure:u8 },
    NoteOn { midi_note: u8, velocity: u8 },
    NoteOff { midi_note: u8, velocity: u8 },
    Sustain { down: bool },
    /// CC123 releases held keys; sustain still applies.
    AllNotesOff,
    /// CC120 immediately silences held and sustained voices.
    AllSoundOff,
    /// CC121 clears sustain; held keys remain held.
    ResetControllers,
}

/// Decode one complete channel message, as supplied by a native MIDI backend.
/// Other channel controls and system/SysEx/realtime messages are ignored.
/// Note-on velocity zero has MIDI's note-off meaning.
pub fn parse_message(timestamp_micros: u64, bytes: &[u8]) -> Result<Option<CapturedEvent>, MidiError> {
    let Some(&status) = bytes.first() else { return Err(MidiError::MalformedMessage); };
    if status >= 0xf0 { return Ok(None); }
    if status < 0x80 { return Err(MidiError::MalformedMessage); }
    let expected = if matches!(status & 0xf0, 0xc0 | 0xd0) { 2 } else { 3 };
    if bytes.len() != expected || bytes[1..].iter().any(|b| *b >= 0x80) {
        return Err(MidiError::MalformedMessage);
    }
    let kind = match status & 0xf0 {
        0x80 => EventKind::NoteOff { midi_note: bytes[1], velocity: bytes[2] },
        0x90 if bytes[2] == 0 => EventKind::NoteOff { midi_note: bytes[1], velocity: 0 },
        0x90 => EventKind::NoteOn { midi_note: bytes[1], velocity: bytes[2] },
        0xb0 => match bytes[1] {
            64 => EventKind::Sustain { down: bytes[2] >= 64 },
            120 => EventKind::AllSoundOff,
            121 => EventKind::ResetControllers,
            123 => EventKind::AllNotesOff,
            controller => EventKind::Control {controller,value:bytes[2]},
        },
        0xe0 => EventKind::PitchBend {value:((u16::from(bytes[2])<<7)|u16::from(bytes[1]))as i16-8192},
        0xd0 => EventKind::ChannelPressure {pressure:bytes[1]},
        0xa0 => EventKind::PolyPressure {midi_note:bytes[1],pressure:bytes[2]},
        _ => return Ok(None),
    };
    Ok(Some(CapturedEvent { timestamp_micros, channel: status & 15, kind }))
}
