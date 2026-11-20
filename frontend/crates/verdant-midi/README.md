# verdant-midi

## Open a device

Native device input uses midir 0.11.1: ALSA on Linux, CoreMIDI on macOS and
WinMM on Windows. `MidiInputService::ports()` returns the real backend port ID
and name. Keep the ID from the selected row and pass that `MidiPort` to `open`;
a disappeared device or backend error is returned to the caller.

`MidiInputService::open(&port, channel, capacity)` owns one actual input
connection and a bounded event queue. Channels are zero based; `None` captures
all channels. `MidiCapture::try_recv()` drains accepted timestamped events.
For a newly opened recording, use `CaptureOrigin { timestamp_micros: 0,
timeline_tick: transport_start }`. If an input was opened earlier, drain its
pre-recording queue and use its current timestamp as the recording origin;
discard queued events earlier than that origin before feeding the take.

## Record a take

Create one `MidiTake`, feed each accepted event with `push`, and on stop:

1. Call `capture.stop()` and feed its returned tail to the take.
2. Read `capture.timestamp_micros()` after stop, so an in-flight callback cannot
   extend beyond the recording end sampled by the UI.
3. Call `take.finish(stop_time, &tempo_map)`.
4. Submit `notes_to_batch(track_id, &notes, lyric)` once to the project store.

Every input/queue/take error invalidates the recording. Overflow, malformed
channel messages, backwards native timestamps and explicit consumer rejection
remain observable after stop. Drop closes and joins the real connection.
`open_with_callback` offers direct native-thread delivery with an explicit
`Result<(), CallbackRejection>`; callbacks must return promptly and stay out
of UI APIs. No callback is delivered after `stop` returns.

## Event semantics

Note-off and zero-velocity note-on release a key. CC64 sustains released notes;
CC123 obeys sustain, CC120 ends all voices immediately, and CC121 releases
pedal-held voices while preserving physically held keys. Stop closes every
remaining note. Retriggered pedal-held voices can overlap. The domain's normal
monophonic validation still decides whether the target track accepts a take.

Capture timestamps use one monotonic connection-relative epoch. The first
native event calibrates the backend's unspecified epoch to elapsed host time;
subsequent event deltas retain native timestamps. This preserves the initial
wait and hardware event spacing, but does not estimate unknown device input
latency or audio/MIDI driver clock drift. Tempo mapping integrates from the
recording's actual musical origin and rounds absolute start/end boundaries.

Velocity is retained as `Note.unknown["midi_velocity"]` (1..127) and channel as
`Note.unknown["midi_channel"]` (0..15). These are source performance metadata;
this crate makes no claim that a singing backend implements MIDI velocity.

## Checks

From `frontend/`:

```sh
cargo test --offline --locked -p verdant-midi
```

Regression sources cover tempo changes, nonzero recording origins, sustain,
retriggering, channel separation, controller semantics, rejected delivery,
stop fencing, and one atomic InsertNotes/undo transaction. They do not open a
physical MIDI device. Check enumeration, selection, velocity, sustain,
rejection reporting and close with a real device through the editor.
The backend choices above describe platform code; Windows and macOS native
acceptance remains pending.
