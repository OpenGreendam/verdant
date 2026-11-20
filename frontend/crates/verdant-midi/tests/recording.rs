use verdant_domain::{Command, Note, TempoMap, TrackId, model::TempoPoint};
use verdant_midi::{CaptureOrigin, EventKind, MidiError, MidiTake, notes_to_batch, parse_message};

fn push(take: &mut MidiTake, timestamp: u64, bytes: &[u8]) {
    if let Some(event) = parse_message(timestamp, bytes).unwrap() { take.push(event).unwrap(); }
}

#[test]
fn pedal_release_crosses_tempo_boundary_at_nonzero_transport_origin() {
    let tempo = TempoMap::new(&[
        TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() },
        TempoPoint { tick: 960, micro_bpm: 60_000_000, unknown: Default::default() },
    ], 960).unwrap();
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 10_000, timeline_tick: 480 }, None).unwrap();
    push(&mut take, 110_000, &[0x90, 69, 103]);
    push(&mut take, 210_000, &[0xb0, 64, 127]);
    push(&mut take, 310_000, &[0x80, 69, 41]);
    push(&mut take, 1_110_000, &[0xb0, 64, 0]);
    let notes = take.finish(1_210_000, &tempo).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!((notes[0].start_tick, notes[0].duration_tick), (672, 1104));
    assert_eq!((notes[0].midi_note, notes[0].velocity), (69, 103));
}

#[test]
fn pedal_retrigger_keeps_released_voices_and_channels_independent() {
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 0, timeline_tick: 0 }, None).unwrap();
    push(&mut take, 0, &[0x90, 60, 80]);
    push(&mut take, 100_000, &[0xb0, 64, 64]);
    push(&mut take, 200_000, &[0x90, 60, 0]);
    push(&mut take, 300_000, &[0x90, 60, 110]);
    push(&mut take, 300_000, &[0x91, 64, 90]);
    push(&mut take, 400_000, &[0x80, 60, 0]);
    push(&mut take, 400_000, &[0x81, 64, 0]);
    push(&mut take, 600_000, &[0xb0, 64, 0]);
    let notes = take.finish(700_000, &TempoMap::constant(120.0)).unwrap();
    assert_eq!(notes.iter().map(|n| (n.start_tick, n.duration_tick, n.midi_note, n.channel)).collect::<Vec<_>>(),
        vec![(0, 1152, 60, 0), (576, 576, 60, 0), (576, 192, 64, 1)]);
}

#[test]
fn controller_reset_releases_pedal_voices_and_stop_finishes_held_keys() {
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 0, timeline_tick: 0 }, Some(3)).unwrap();
    push(&mut take, 0, &[0x93, 60, 1]);
    push(&mut take, 0, &[0x93, 64, 127]);
    push(&mut take, 0, &[0x92, 67, 100]); // filtered channel
    push(&mut take, 10_000, &[0xb3, 64, 127]);
    push(&mut take, 250_000, &[0x83, 60, 0]);
    push(&mut take, 500_000, &[0xb3, 121, 0]);
    let notes = take.finish(750_000, &TempoMap::constant(120.0)).unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!((notes[0].duration_tick, notes[1].duration_tick), (960, 1440));
}

#[test]
fn all_notes_off_obeys_sustain_but_all_sound_off_is_immediate() {
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 0, timeline_tick: 0 }, None).unwrap();
    push(&mut take, 0, &[0x90, 69, 100]);
    push(&mut take, 10_000, &[0xb0, 64, 127]);
    push(&mut take, 250_000, &[0xb0, 123, 0]);
    push(&mut take, 500_000, &[0xb0, 120, 0]);
    let notes = take.finish(1_000_000, &TempoMap::constant(120.0)).unwrap();
    assert_eq!(notes[0].duration_tick, 960);
}

#[test]
fn malformed_and_backwards_input_cannot_be_committed_as_a_valid_take() {
    assert!(parse_message(0, &[0x90, 60]).is_err());
    assert!(parse_message(0, &[0x90, 128, 127]).is_err());
    assert!(parse_message(0, &[0xc0, 12]).unwrap().is_none());
    assert!(parse_message(0, &[0xf8]).unwrap().is_none());
    let off = parse_message(100, &[0x92, 64, 0]).unwrap().unwrap();
    assert_eq!(off.kind, EventKind::NoteOff { midi_note: 64, velocity: 0 });
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 101, timeline_tick: 0 }, None).unwrap();
    assert!(matches!(take.push(off), Err(MidiError::TimestampOrder)));
    assert!(matches!(take.finish(1_000_000, &TempoMap::constant(120.0)), Err(MidiError::TimestampOrder)));
}

#[test]
fn one_existing_insert_batch_retains_velocity_and_stable_generated_note_ids() {
    let mut take = MidiTake::new(CaptureOrigin { timestamp_micros: 0, timeline_tick: 0 }, None).unwrap();
    push(&mut take, 0, &[0x92, 60, 1]);
    push(&mut take, 125_000, &[0x82, 60, 0]);
    push(&mut take, 125_000, &[0x92, 64, 127]);
    let notes = take.finish(250_000, &TempoMap::constant(120.0)).unwrap();
    let track = TrackId::generate();
    let batch = notes_to_batch(track.clone(), &notes, "a");
    assert_eq!(batch.commands.len(), 1);
    let Command::InsertNotes { track: target, notes: inserted } = &batch.commands[0] else { panic!("recording must be one InsertNotes"); };
    assert_eq!(*target, track);
    assert_eq!(inserted.len(), 2);
    assert_ne!(inserted[0].id, inserted[1].id);
    assert_eq!(inserted[0].unknown["midi_velocity"], 1);
    assert_eq!(inserted[1].unknown["midi_velocity"], 127);
    assert_eq!(inserted[0].unknown["midi_channel"], 2);
    assert_eq!(inserted[0].lyric, "a");
    let _: &Note = &inserted[0];
    let mut project = verdant_domain::Project::new_empty(); project.tracks[0].id = track;
    let mut store = verdant_domain::ProjectStore::new(project);
    assert!(store.apply(store.revision(), batch).unwrap().is_some());
    assert_eq!(store.undo_depth(), 1); assert_eq!(store.project().tracks[0].notes.len(), 2);
    store.undo().unwrap();
    assert!(store.project().tracks[0].notes.is_empty());
}
