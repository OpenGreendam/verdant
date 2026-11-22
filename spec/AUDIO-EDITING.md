# Audio clip editing

Verdant stores audio edits as source-frame metadata. The original imported audio
is never rewritten by a split, trim, move, fade, reversal, or rate change. The
GUI, CLI and workflow worker use `verdant_service::audio_edit`; one successful
edit or edit batch creates one ordinary document-journal entry.

## Graphical workflow

Import an audio file from the integration menu. Each clip displays a two-channel
waveform when the source is stereo, a source name (or your clip name), edge fades,
and reverse/rate indicators. Its waveform uses the verified source peak cache,
with clip gain and its retained fade envelope applied for display.

The arrangement toolbar provides **Select**, **Range**, **Split** and **Clip**.
Use Shift-click to add or remove clips from a selection. Drag a selected clip's
body to move the group, including to another track. Drag its edges to reveal or
hide source frames; the original source remains available. Upper edge handles
edit fades. Alt temporarily bypasses musical grid snapping. The existing
arrangement/piano-roll splitter can enlarge the audio timeline.

The range tool selects time and tracks together. A range can be copied, cut,
deleted, ripple-deleted, trimmed, or split at both ends. The Clip menu also offers
an explicit range in seconds and a precise source window in decoded frames.

| Action | Arrangement shortcut / interaction |
| --- | --- |
| Add/remove a clip | Shift-click |
| Split at cursor or both range edges | Ctrl/Cmd+I; Ctrl-click a clip |
| Copy / cut / paste | Ctrl/Cmd+C / X / V |
| Duplicate at playhead | Ctrl/Cmd+D |
| Delete while retaining the gap | Delete or Backspace |
| Ripple delete | Shift+Delete |
| Join contiguous selected source regions | Ctrl/Cmd+J |
| Range / selection tool | R / V |
| Select audio clips | Ctrl/Cmd+A |
| Cancel a drag/range | Escape |

These shortcuts act on audio while the arrangement owns keyboard focus. Piano
roll and text-field shortcuts keep their own focus scopes.

Right-click or double-click a clip to find gain in dB, pan, fades, rename,
mute, reverse, speed, pitch preservation, source slip, silence cuts, zero-crossing
cuts, peak normalization, equal-power crossfades and bounce. The existing native
clip inspector provides direct gain/pan controls and a structured trim dialog.
Audio analysis and bounce run on background workers; stale results never replace
a newer project revision.

## Shared atomic operations

Use `verdant capabilities` and `verdant schema clip.edit` for the installed
request contract. A CLI example is:

```sh
verdant clip split --params split.json --json
verdant clip edit --params edits.json --json
verdant worker --stdio
```

A split parameter file has this shape (replace the project path, revision and
stable IDs with values from `clip inspect`):

```json
{
  "path": "/absolute/path/song.verp",
  "expect_revision": 7,
  "clips": [
    {
      "track": "00000000-0000-4000-8000-000000000001",
      "id": "00000000-0000-4000-8000-000000000002"
    }
  ],
  "tick": 1920.125
}
```

For `clip.edit`, supply an `edit` object tagged by `operation`. A batch can
combine operations against the staged project; an invalid ID, destination,
non-finite value, source range or property rejects the entire project edit.

| Operation | Meaning |
| --- | --- |
| `split` | Split selected clips intersecting an exact tick; outside clips stay unchanged |
| `split_range` | Split all intersecting clips on explicit tracks at both range edges |
| `trim` | Set a source-frame window and optional placement; can restore hidden content |
| `trim_to_range` | Retain the intersection with a time range |
| `slip` | Shift source content while retaining timeline placement and duration |
| `move` | Apply a tick offset and relative track offset to the selection |
| `duplicate` | Copy source references to a new placement with new clip IDs |
| `delete` | Remove clips; optional per-track ripple closes their time spans |
| `delete_range` | Remove time intersections, split retained edges, optionally close the gap |
| `copy`, `copy_range`, `paste` | Serialize source references and relative seconds/track offsets |
| `set_properties` | Name, gain, pan, mute, reverse, rate, pitch preservation and fades |
| `join` | Rejoin adjacent regions without rewriting samples |
| `crossfade` | Apply linear or equal-power fades across an existing overlap |
| `split_silence` | Cut at the midpoint of each sufficiently long internal silent run |
| `split_zero_crossing` | Find a nearby shared-channel low-amplitude crossing and split |
| `normalize` | Set peak gain independently or preserve relative gain across a group |
| `bounce` | Encode selected clip processing into a new float WAV |
| `batch` | Stage multiple operations as a single undoable document transaction |

A silence split preserves every source frame, including silence. It does not
silently remove a quiet passage. Ripple-deleting whole clips refuses an
overlapping unselected clip; use an explicit shared `delete_range` when that
overlap should also be cut.

Clipboard data contains no PCM. Pasting allocates fresh IDs and preserves relative
elapsed seconds across tempo changes. Relative track offsets must fit the
destination project; tracks are never silently created or truncated.

## Source timing and rendering

Legacy `verdant-audio-clips/1` documents retain their defaults. Additional fields
are optional on input: `start_subtick`, `name`, `playback_rate`,
`preserve_pitch`, `reversed`, `fade_shape`, and `fade_window`.
The window stores a cropped envelope so splitting through an existing fade
does not restart or rescale that fade. Restoring its full source window restores
the original envelope. Adjacent source-frame cuts use fractional tick placement,
and output lengths use the difference between rounded absolute start/end sample
positions to avoid one-frame gaps between converted regions.

Rate changes span 0.0625–16×. Varispeed uses channel-linked bandlimited sinc
conversion. Pitch preservation uses offline waveform-similarity overlap-add
(WSOLA), with a shared alignment choice for both channels so the stereo image
does not drift. Rate conversion, reverse, clip pan and fades are prepared once;
playback and export consume that same immutable PCM.

WSOLA is an explicit processing mode, not a promise of transparent sound at
extreme ratios. Large stretches can repeat transients or smear dense/polyphonic
material; audition the result. Source clips shorter than 32 frames cannot contain
enough periodic information for a reliable pitch estimate and use a deterministic
sample-clock-preserving repetition. Algorithm reference:
[Roelands and Verhelst, Eurospeech 1993](https://www.isca-archive.org/eurospeech_1993/roelands93_eurospeech.html).

Waveform display is a cached source envelope mapped onto edited duration, not
a second independently rendered effect path. Track/master processing belongs to
mix export. Clip bounce includes clip gain, pan, reverse, rate and fades; it does
not print track or master inserts.

## Assets, publication and failure behavior

Planning is read-only, including media analysis and bounce encoding. A bounce plan
contains pending encoded bytes and an output fingerprint but does not create a
directory or file. The CLI checks the writer lease, expected revision and the
staged domain batch before explicitly publishing assets.

Publication writes and syncs a uniquely named temporary WAV in the chosen
directory, then links the complete file to a new destination without overwriting
an existing file or symlink. Imported sources are never overwritten. The bytes
are checked against the pending fingerprint before publication. Project writes
retain the usual atomic persistence and undo behavior.

Decoded sources and prepared clips are bounded by the media import limits;
analysis caches and bounce workspaces also have aggregate PCM budgets. External
sources must be explicitly authorized and still match their saved SHA-256,
byte count and decoded format. Missing or changed sources remain visible as
asset errors and do not become substitute silence.

Slow preparation is tracked by generation. Transport receipt polling does not
continually restart an already pending WSOLA/decode job. A newer edit supersedes
the previous result; an older completion cannot clear its newer generation.
If asset preparation fails, playback stays stopped and the topology receipt
is released so the document can still be saved or repaired.

## Verification

Regression coverage includes old documents, source-frame split/join identity,
faded and reversed split PCM equality, source-window restoration, absolute
engine-frame boundaries across output rates, anti-aliasing, pitch/duration and
stereo phase, one-entry multi-track undo/redo, invalid atomic batches, range ripple,
cross-tempo clipboard placement, source authorization/fingerprint verification,
silence/zero crossings, bounce no-write planning/no-clobber publication and
playback/bounce sample equality. Background-generation tests perform real slow
pitch-preserving preparation while exercising receipt polling and supersession.

## Cross-project clipboard and asynchronous publication

GUI and CLI copy/copy-range run with the media policy, verify the source
fingerprint and decoded format, and put its canonical absolute reference in the
existing `verdant-audio-clipboard/1` payload. Relative legacy references remain
supported within their originating project. Paste verifies every incoming
source again under the destination caller's policy; copying a reference does
not authorize a CLI/worker to read it in another process. Supply the appropriate
asset roots for a cross-project paste. The GUI retains its own verified
clipboard across project changes and carries only those copied file grants.

The metadata-only planner permits paste of sources already present in that
project. A fresh source requires the media planner and cannot bypass source
verification. No clipboard operation copies or writes audio bytes.

Bounce planning encodes bytes in memory without filesystem writes. Background
execution writes, syncs and seals private same-directory staging files using
`stage_audio_assets`. The GUI then performs its final revision check, short
no-clobber hard-link publication, and journal commit in the same control turn,
with no asynchronous yield between those operations. A stale preparation drops
its private staging files without creating the final output.

After publication, a persistence error or a later output collision never
deletes a published asset. `StagedAudioAssets::published_paths` records the
completed outputs, including a partial publication, and GUI failure notices
name the retained paths and explain recovery. This also prevents deleting a
file that another user operation replaced after publication.

Audio dragging keeps fractional musical positions. Alt disables the beat grid;
source trims still quantize to exact decoded source frames, and group movement
clamps to the earliest selected start and available destination tracks. A
click without a drag reduces a multiple selection to that clip; Shift retains
explicit selection toggling.
