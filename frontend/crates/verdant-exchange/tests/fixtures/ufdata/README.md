# UtaFormatix Data fixtures and interpretation

`official-standard-midi.ufdata` is the unchanged four-note solfege diagnostic
`src/jsTest/resources/fixtures/standard_mid_import.ufdata` from the official
[sdercolin/utaformatix3 repository](https://github.com/sdercolin/utaformatix3/blob/91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae/src/jsTest/resources/fixtures/standard_mid_import.ufdata),
commit `91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae`, upstream Git blob
`0f7c087d5637b8adec83fb08f001a71b565be975`. Its upstream Apache-2.0 notice is
included as `UPSTREAM-LICENSE.txt`; the complete license terms are in
[`LICENSE-APACHE-2.0.txt`](LICENSE-APACHE-2.0.txt), from the
[Apache Software Foundation](https://www.apache.org/licenses/LICENSE-2.0.txt).
The notice applies to this four-note upstream fixture; it does not change the
existing licenses of Verdant implementation code. This is a score fixture,
without recordings, voice assets or synthesis software.

`multitrack-controls.ufdata` is an original Verdant diagnostic. It exercises
decimal tempos, changes of meter, an empty track, opaque phoneme strings,
absolute/null and relative pitch, events in rests, and unknown-field retention.

## Pinned primary sources

- [Data README](https://github.com/sdercolin/utaformatix-data/blob/609287a0ac8661a1f9c92ee3dc5906820c9a26eb/README.md): 480 ticks per quarter note, MIDI keys and held semitone-valued pitch.
- [Version.kt](https://github.com/sdercolin/utaformatix-data/blob/609287a0ac8661a1f9c92ee3dc5906820c9a26eb/lib/kotlin-mpp/src/commonMain/kotlin/com/sdercolin/utaformatix/data/Version.kt): format version 1. Library/package versions are not format versions.
- [Pitch.kt](https://github.com/sdercolin/utaformatix-data/blob/609287a0ac8661a1f9c92ee3dc5906820c9a26eb/lib/kotlin-mpp/src/commonMain/kotlin/com/sdercolin/utaformatix/data/Pitch.kt): parallel tick/value arrays, relative or absolute values, and `null` only in absolute mode to end the preceding override.
- [Tempo.kt](https://github.com/sdercolin/utaformatix-data/blob/609287a0ac8661a1f9c92ee3dc5906820c9a26eb/lib/kotlin-mpp/src/commonMain/kotlin/com/sdercolin/utaformatix/data/Tempo.kt): BPM is a `Double`; the README's integer table entry is inaccurate.
- [UfData.kt](https://github.com/sdercolin/utaformatix3/blob/91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae/core/src/main/kotlin/core/io/UfData.kt): passes project ticks and `measurePrefix` directly through the interchange model.
- [Vsqx.kt](https://github.com/sdercolin/utaformatix3/blob/91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae/core/src/main/kotlin/core/io/Vsqx.kt): source prefix ticks are removed during VSQX parsing and restored on generation. A second subtraction when reading `.ufdata` would be wrong.
- [PitchCalculation.kt](https://github.com/sdercolin/utaformatix3/blob/91d0aeb994bc846e380ea98cd4c6f75c5ea7c6ae/core/src/main/kotlin/core/process/pitch/PitchCalculation.kt): hold semantics and relative zero / absolute-null conversion.

## Verdant conversion contract

Import materializes a 480-PPQ project, maps zero-based measure positions to tick
positions using the preceding meter, and preserves `measurePrefix` as metadata
without moving the score. No default voice or reading is inferred. `phoneme`
has no standard alphabet or language declaration, so the complete text stays
editable under the explicit `ufdata-source-phoneme` alphabet. The report warns
that it needs inspection/replacement before synthesis; it is never relabelled
as IPA or as a native voicebank code.

Held track pitch becomes relative integer-millicent note curves. An absolute
`null` restores the note's unmodified pitch. Original off-note data, decimal
precision, unknown fields and opaque phoneme spacing survive compatible export
while retention is enabled and the corresponding authored data is unchanged.
Pitch snapshots compare timing, keys, detune, pitch values and interpolation,
not ephemeral point IDs. A lyric edit or native save/reopen therefore keeps the
source pitch; an actual pitch/timing edit regenerates it and reports discarded
off-note/source-only data instead of overwriting the edit.

Export uses 480 PPQ. Fractional positions round to the nearest tick with a
field-specific report; collapsed notes and colliding tempos are errors. A held
native curve stays sparse; changing linear/cosine/cubic curves are sampled once
per output tick with a quantization warning and an event/work limit. Sampling
does not bake automatic vibrato or portamento. Mixer settings, host routes,
voice references, dynamics, other expression curves, prosody and unsupported
metadata are reported as losses. Phoneme text is exported without guessing a
destination dictionary; a declared native alphabet/language cannot be carried
in the standard field and is reported separately.

Each track must be monophonic. Unlike upstream note-shaping behavior, Verdant
rejects overlapping notes instead of trimming them. Unknown versions,
duplicate JSON keys, nonfinite or overflowing numbers, mismatched/unsorted pitch
arrays, relative `null`, invalid maps, zero/negative notes and exceeded limits
are rejected. Empty tracks and explicit empty/default maps are supported; map
defaults are reported. Off-bar native meter changes cannot be represented by
the measure-indexed format and are rejected with a destination-specific error.
