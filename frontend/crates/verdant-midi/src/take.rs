use verdant_domain::{Command, CommandBatch, Note, TempoMap, TrackId};
use crate::{CapturedEvent, EventKind, MidiError};

const MAX_NOTES: usize = 1_048_576;
const MAX_EXPRESSION_POINTS:usize=1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureOrigin {
    pub timestamp_micros: u64,
    pub timeline_tick: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedNote {
    pub start_tick: i64,
    pub duration_tick: i64,
    pub midi_note: u8,
    pub velocity: u8,
    pub channel: u8,
    pub expression:Vec<CapturedExpression>,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct CapturedExpression {pub offset_tick:i64,pub channel:u8,pub kind:EventKind}

#[derive(Clone)]
struct Pending {
    start: u64, midi_note: u8, velocity: u8, channel: u8, key_down: bool, expression:Vec<(u64,u8,EventKind)>,
}
struct Finished { note: Pending, end: u64 }

/// One recording intent. Its notes are committed only after both the capture
/// and `finish` succeed, giving a single atomic domain command batch.
pub struct MidiTake {
    origin: CaptureOrigin,
    channel: Option<u8>,
    last: u64,
    sustain: [bool; 16],
    pending: Vec<Pending>,
    finished: Vec<Finished>,
    failure: Option<MidiError>,
    controllers:[[Option<u8>;128];16],
    pitch_bend:[i16;16],
    pressure:[u8;16],
    expression_count:usize,

}

impl MidiTake {
    pub fn new(origin: CaptureOrigin, channel: Option<u8>) -> Result<Self, MidiError> {
        if channel.is_some_and(|c| c > 15) { return Err(MidiError::InvalidChannel); }
        if origin.timeline_tick < 0 { return Err(MidiError::TickRange); }
        Ok(Self { origin, channel, last: origin.timestamp_micros, sustain: [false; 16], pending: Vec::new(), finished: Vec::new(), failure: None,controllers:[[None;128];16],pitch_bend:[0;16],pressure:[0;16],expression_count:0 })
    }

    pub fn push(&mut self, event: CapturedEvent) -> Result<(), MidiError> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let result = self.push_event(event);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn push_event(&mut self, event: CapturedEvent) -> Result<(), MidiError> {
        if event.channel > 15 { return Err(MidiError::InvalidChannel); }
        if event.timestamp_micros < self.last { return Err(MidiError::TimestampOrder); }
        if matches!(event.kind, EventKind::NoteOn { midi_note: 128.., .. } | EventKind::NoteOff { midi_note: 128.., .. }
            | EventKind::NoteOn { velocity: 128.., .. } | EventKind::NoteOff { velocity: 128.., .. }
            | EventKind::Control {controller:128..,..}|EventKind::Control{value:128..,..}
            | EventKind::ChannelPressure{pressure:128..}|EventKind::PolyPressure{midi_note:128..,..}|EventKind::PolyPressure{pressure:128..,..}
            | EventKind::PitchBend{value:8192..}|EventKind::PitchBend{value:..=-8193}) {
            return Err(MidiError::MalformedMessage);
        }
        self.last = event.timestamp_micros;
        if self.channel.is_some_and(|c| c != event.channel) { return Ok(()); }
        let at = event.timestamp_micros;
        let channel = event.channel;
        let kind=match event.kind{
            EventKind::Control{controller:64,value}=>EventKind::Sustain{down:value>=64},
            EventKind::Control{controller:120,..}=>EventKind::AllSoundOff,
            EventKind::Control{controller:121,..}=>EventKind::ResetControllers,
            EventKind::Control{controller:123,..}=>EventKind::AllNotesOff,
            kind=>kind,
        };
        match kind {
            EventKind::NoteOn { midi_note, velocity: 0 } | EventKind::NoteOff { midi_note, .. } => {
                if let Some(i) = self.pending.iter().position(|n| n.channel == channel && n.midi_note == midi_note && n.key_down) {
                    if self.sustain[channel as usize] {
                        if self.expression_count>=MAX_EXPRESSION_POINTS{return Err(MidiError::TakeLimit);}
                        self.pending[i].expression.push((at,channel,EventKind::NoteOff{midi_note,velocity:0}));self.expression_count+=1;
                        self.pending[i].key_down = false;
                    }
                    else { self.end(i, at); }
                }
            }
            EventKind::NoteOn { midi_note, velocity } => {
                if self.pending.len() + self.finished.len() >= MAX_NOTES { return Err(MidiError::TakeLimit); }
                // A retrigger without a preceding off terminates the held key.
                // Already released pedal-held voices can overlap the new key.
                if let Some(i) = self.pending.iter().position(|n| n.channel == channel && n.midi_note == midi_note && n.key_down) {
                    self.end(i, at);
                }
                let mut expression=vec![(at,channel,EventKind::PitchBend{value:self.pitch_bend[channel as usize]}),(at,channel,EventKind::ChannelPressure{pressure:self.pressure[channel as usize]})];
                for (controller,value) in self.controllers[channel as usize].iter().enumerate() {
                    if let Some(value)=value{expression.push((at,channel,EventKind::Control{controller:controller as u8,value:*value}));}
                }
                if channel!=0 {
                    expression.push((at,0,EventKind::PitchBend{value:self.pitch_bend[0]}));
                    expression.push((at,0,EventKind::ChannelPressure{pressure:self.pressure[0]}));
                    for(controller,value)in self.controllers[0].iter().enumerate(){
                        if let Some(value)=value{expression.push((at,0,EventKind::Control{controller:controller as u8,value:*value}));}
                    }
                }
                if self.expression_count+expression.len()>MAX_EXPRESSION_POINTS{return Err(MidiError::TakeLimit);}
                self.expression_count+=expression.len();
                self.pending.push(Pending { start: at, midi_note, velocity, channel, key_down: true,expression });
            }
            EventKind::Sustain { down } => {
                self.controllers[channel as usize][64]=Some(if down{127}else{0});
                self.capture_expression(channel,at,EventKind::Control{controller:64,value:if down{127}else{0}})?;
                self.sustain[channel as usize] = down;
                if !down { self.release(channel, at, false); }
            }
            EventKind::AllSoundOff => {self.capture_expression(channel,at,EventKind::AllSoundOff)?;self.release(channel, at, true);},
            EventKind::AllNotesOff => {
                self.capture_expression(channel,at,EventKind::AllNotesOff)?;
                for note in self.pending.iter_mut().filter(|n| n.channel == channel) { note.key_down = false; }
                if !self.sustain[channel as usize] { self.release(channel, at, true); }
            }
            EventKind::ResetControllers => {
                self.capture_expression(channel,at,EventKind::ResetControllers)?;
                self.controllers[channel as usize]=[None;128];self.pitch_bend[channel as usize]=0;self.pressure[channel as usize]=0;
                self.sustain[channel as usize] = false;
                self.release(channel, at, false);
            }
            EventKind::Control{controller,value}=>{
                self.controllers[channel as usize][controller as usize]=Some(value);
                self.capture_expression(channel,at,event.kind)?;
            }
            EventKind::PitchBend{value}=>{self.pitch_bend[channel as usize]=value;self.capture_expression(channel,at,event.kind)?;}
            EventKind::ChannelPressure{pressure}=>{self.pressure[channel as usize]=pressure;self.capture_expression(channel,at,event.kind)?;}
            EventKind::PolyPressure{..}=>{self.capture_expression(channel,at,event.kind)?;}
        }
        Ok(())
    }

    fn capture_expression(&mut self,channel:u8,at:u64,kind:EventKind)->Result<(),MidiError> {
        let matches=|n:&&mut Pending|match kind{EventKind::PolyPressure{midi_note,..}=>n.channel==channel&&n.midi_note==midi_note,_=>n.channel==channel||channel==0};
        let count=self.pending.iter_mut().filter(matches).count();
        if self.expression_count+count>MAX_EXPRESSION_POINTS{return Err(MidiError::TakeLimit);}
        for note in self.pending.iter_mut().filter(matches){note.expression.push((at,channel,kind));}
        self.expression_count+=count;Ok(())
    }
    fn end(&mut self, index: usize, at: u64) {
        self.finished.push(Finished { note: self.pending.swap_remove(index), end: at });
    }
    fn release(&mut self, channel: u8, at: u64, include_held: bool) {
        let mut i = 0;
        while i < self.pending.len() {
            let note = &self.pending[i];
            if note.channel == channel && (include_held || !note.key_down) { self.end(i, at); }
            else { i += 1; }
        }
    }

    pub fn finish(mut self, stop_micros: u64, tempo: &TempoMap) -> Result<Vec<CapturedNote>, MidiError> {
        if let Some(error) = self.failure.take() { return Err(error); }
        if stop_micros < self.last { return Err(MidiError::TimestampOrder); }
        while !self.pending.is_empty() { self.end(self.pending.len() - 1, stop_micros); }
        let origin_seconds = tempo.seconds_at(self.origin.timeline_tick as f64);
        let tick = |at: u64| -> Result<i64, MidiError> {
            let elapsed = at.checked_sub(self.origin.timestamp_micros).ok_or(MidiError::TimestampOrder)?;
            let tick = tempo.tick_at(origin_seconds + elapsed as f64 / 1_000_000.0).round();
            if !tick.is_finite() || tick < 0.0 || tick >= i64::MAX as f64 { return Err(MidiError::TickRange); }
            Ok(tick as i64)
        };
        let mut notes = Vec::with_capacity(self.finished.len());
        for finished in self.finished {
            let start_tick = tick(finished.note.start)?;
            let duration_tick = tick(finished.end)?.checked_sub(start_tick).ok_or(MidiError::TickRange)?.max(1);
            start_tick.checked_add(duration_tick).ok_or(MidiError::TickRange)?;
            notes.push(CapturedNote { start_tick, duration_tick, midi_note: finished.note.midi_note,
                velocity: finished.note.velocity, channel: finished.note.channel,
                expression:finished.note.expression.into_iter().map(|(at,channel,kind)|Ok(CapturedExpression{channel,offset_tick:tick(at)?.checked_sub(start_tick).ok_or(MidiError::TickRange)?,kind})).collect::<Result<_,MidiError>>()? });
        }
        notes.sort_by_key(|n| (n.start_tick, n.midi_note, n.channel));
        Ok(notes)
    }
}

/// Velocity/channel are retained as source MIDI metadata. Velocity is not
/// silently converted to a proprietary singing parameter or volume curve.
pub fn notes_to_batch(track: TrackId, captured: &[CapturedNote], lyric: &str) -> CommandBatch {
    let notes = captured.iter().map(|captured| {
        let mut note = Note::new(captured.start_tick, captured.duration_tick, i32::from(captured.midi_note));
        note.lyric = lyric.into();
        note.unknown.insert("midi_velocity".into(), captured.velocity.into());
        note.unknown.insert("midi_channel".into(), captured.channel.into());
        if !captured.expression.is_empty() {
            let points:Vec<_>=captured.expression.iter().filter_map(|point| {
                let mut value=match point.kind{
                    EventKind::Control{controller,value}=>serde_json::json!({"type":"control","controller":controller,"value":value}),
                    EventKind::PitchBend{value}=>serde_json::json!({"type":"pitch_bend","value":value}),
                    EventKind::ChannelPressure{pressure}=>serde_json::json!({"type":"pressure","value":pressure}),
                    EventKind::PolyPressure{pressure,..}=>serde_json::json!({"type":"poly_pressure","value":pressure}),
                    EventKind::NoteOff{..}=>serde_json::json!({"type":"note_off","value":0}),
                    EventKind::Sustain{down}=>serde_json::json!({"type":"control","controller":64,"value":if down{127}else{0}}),
                    EventKind::AllSoundOff=>serde_json::json!({"type":"control","controller":120,"value":0}),
                    EventKind::ResetControllers=>serde_json::json!({"type":"control","controller":121,"value":0}),
                    EventKind::AllNotesOff=>serde_json::json!({"type":"control","controller":123,"value":0}),
                    EventKind::NoteOn{..}=>return None,
                };
                value["offset_tick"]=point.offset_tick.into();value["channel"]=point.channel.into();Some(value)
            }).collect();
            note.unknown.insert("midi_expression".into(),points.into());
        }
        note
    }).collect();
    CommandBatch::single("Record MIDI", Command::InsertNotes { track, notes })
}
