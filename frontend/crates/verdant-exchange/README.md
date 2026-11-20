# verdant-exchange

Rust score import/export for the editor and `verdant-exchange` command.
Input bytes become an editable `verdant_domain::Project` and a conversion
report. Compatible source extensions retain fields outside the editable score.

This crate supports UtaFormatix Data (UFDATA), MIDI SMF, Synthesizer V SVP,
VOCALOID VSQX 3/4 and VPR import and export, plus UST and USTX export. UST/USTX import remains exclusively in
`verdant-import`. It does not implement a UTAU sample engine or execute UTAU
sample controls, resamplers, Synthesizer V, VOCALOID, plugins or licensed voices.

## Library API

```rust,ignore
use verdant_exchange::{Format, ImportOptions, ExportOptions};
let imported = verdant_exchange::import(Format::Svp, &bytes, &ImportOptions::default())?;
let exported = verdant_exchange::export(Format::Midi, &imported.project, &ExportOptions::default())?;
// Display/report every issue to the user, including retained source-only data.
for issue in &exported.report.issues {
    println!("{}: {}", issue.path, issue.detail);
}
```

`import(Format, &[u8], &ImportOptions)` returns `ImportResult { project, report }`.
`export(Format, &Project, &ExportOptions)` returns `ExportResult { bytes, report }`.
Both return `Result<_, ExchangeError>`. `ExchangeReport` identifies the format,
direction, original source SHA-256/name, source PPQ and `issues`. Each
`FieldIssue` has `path`, `disposition`, `value` and `detail`.
`has_losses()` includes unsupported fields, retained but unexecuted fields and
quantization. A retained field is not a claim of native synthesis equivalence.

`parse_project_json(&[u8], &Limits)` reads a native project with recursive
JSON duplicate rejection, resource/timing validation and refreshed note indexes.
`parse_export_options_json(&[u8])` reads the options sidecar with the same
duplicate rejection. Options are strict JSON objects; unknown option keys error.

```rust,ignore
ExportOptions {
    track: Option<usize>,
    preserve_extensions: bool,
    midi_ppq: Option<u16>,
    ust_encoding: UstEncoding, // Utf8 or Cp932
    vocaloid_voice: Option<VocaloidVoice>,
    limits: Limits,
}
VocaloidVoice {
    comp_id: String,
    name: String,
    language_id: u8,
    program: u8,
}
```

For a multitrack UST export, `track` must select one zero-based track. Other
formats export all editable vocal tracks unless `track` is supplied. For VPR,
compatible source-only audio and unknown tracks remain in source order when
extension retention is enabled, including when `track` selects one vocal track.
VSQX requires actual
`vocaloid_voice` metadata or a real imported VSQX voice table. Missing metadata
is a clear error at `/options/vocaloid_voice`; a licensed compID is never
invented. Explicit voice metadata replaces the destination table/singer selection.
Imported voice identifiers are score metadata, not engine licenses. A fresh VPR
may carry an unassigned voice with a report asking the consumer to select an
installed voice; explicit metadata can also assign it.

## Native format behavior

| Format | Editable representation and exact timing | True format/model limits |
| --- | --- | --- |
| UFDATA v1 | Fixed 480 PPQ, multiple vocal/empty tracks, tempo/meter maps, note keys/timing/lyrics, opaque source phoneme strings, held relative/absolute track pitch and absolute-null resets | `measurePrefix` remains metadata without another timeline offset. Source phonemes have no declared alphabet/language and are not guessed as IPA. Arbitrary native PPQ exports round fractional ticks with per-field reports; collapsed notes, tempo collisions and off-bar meter changes error. Mixer, engine controls and automatic vibrato/portamento have no equivalent fields. |
| MIDI SMF 0/1 | Exact source metrical PPQ, global tempo/meter, original tracks, per-note channel/key/release boundaries/velocity/UTF-8 lyrics, RPN pitch-bend sensitivity and bend nodes | SMPTE clocks and format-2 independent songs are rejected. Sustain/CC/program/sysex events remain source-only; no pedal duration or singing timbre is guessed. Parts and native mixer state have no exact SMF representation. |
| SVP | Exactly 705,600,000 blicks per quarter, tempo/meter bars, mixer, group instances, lyric text, source phonemes, detune, linear pitchDelta/loudness curves | Nonlinear/automatic pitch and voice/timbre parameters remain source-only. Same-offset repeated groups share domain content. Different transposition offsets are materialized in independent domain content, with the original reference metadata retained. Edited divergent groups export separately. |
| VSQX 3/4 | Exact positive source resolution, tempo/meter maps, track numbers, parts, notes/velocity/lyrics, explicitly identified VOCALOID phonemes, PIT/PBS and admitted 0..127 controllers, mixer | Source pre-measures remain at their original nonnegative timeline ticks. Native vibrato, portamento, dB note gain, plugins and automatic styles are not reinterpreted as equivalent synthesis. Both schema versions retain their own namespaces and element spellings. |
| VPR | Exactly 480 ticks per quarter, real `Project/sequence.json` ZIP, active global/event tempo, meter bars, track order, parts, note fields, PIT/PBS and admitted controllers | Audio/unknown tracks and non-sequence archive members are retained without opening external paths or executing anything. Source-only expression/AI/vibrato fields remain metadata. Fixed-resolution exports reject nonintegral timing. |
| UST | One selected monophonic lane at exactly 480 PPQ; gap/rest events, tempo at event boundaries, TimeSignatures bar tuples, Mode2 bend nodes, constant per-note intensity where representable | Overlaps and tempo changes inside a sounding note error instead of splitting it. Parts/reference identity and mixer state do not carry over. CP932 rejects unencodable text; UTF-8 is available. Varying gain cannot become a single Intensity value. Older consumers may ignore TimeSignatures. |
| USTX | Native project resolution, tempo/meter maps, tracks, independent parts, mixer, note text, pitch nodes in real milliseconds and dynamics in tenths of dB | Linked references become independent parts. Hold/cubic pitch shapes and dynamics precision can be lost and are reported. Portable phonetic symbols are not guessed as UTAU sample aliases. |

VPR and VSQX dynamics use a two-segment linear conversion between integer DYN
and note gain. DYN 0, 64 and 127 correspond to −60, 0 and +12 dB. The default
before the first authored event is 64. Scores hold each event value until the
next event. Import places events at their original ticks within notes and adds
note boundary values. It reports rounding to whole millidecibels. Export uses
the inverse curve, reports rounding or clamping to the integer DYN range, and
reports when edited interpolation becomes a hold. This mapping defines editable
score gain and does not model VOCALOID acoustic dynamics.

PIT uses signed 14-bit values and PBS sets the sensitivity in semitones. Export
places the chosen sensitivity at the part start, converts the pitch points at
destination ticks, and reports bend quantization. BRE, BRI, CLE, GEN, POR, OPE
and GWL remain editable integer lanes from 0 through 127. Compatible exports
with extension retention preserve original controllers for unchanged lanes,
including events outside notes. Per-note integer velocity is preserved. Accent
and part-level velocity or accent controllers remain source fields in the report.

Except for UFDATA's explicitly reported fixed-480-PPQ quantization, ordinary note
onsets, lengths, map positions and controller ticks that change resolution must
scale exactly; there is no hidden tick rounding. SMF meter beats
that would be fractional at the original PPQ promote **every tick and PPQ** by
one common power of two. The report keeps the original PPQ and names the factor.
SMF export keeps an admitted resolution up to 32767; for larger project PPQs the
default candidate is 960 or an explicit `midi_ppq`. Unrepresentable boundaries
error. MIDI microseconds-per-quarter, VOCALOID hundredth-BPM, native micro-BPM,
14-bit bends and destination gain/pan precision have explicit conversion reports.
Original MIDI tempo, release velocity and meter clock metadata roundtrip when
compatible. Unedited source controller nodes outside note boundaries also remain
available on compatible exports.

Pronunciation alphabets are explicit. VOCALOID export accepts identified
VOCALOID XSAMPA spellings, and SVP export accepts the source reference phoneset.
An unrelated IPA inventory or voicebank alias is reported rather than silently
relabelled. Pinyin readings and native prosody metadata have separate field-loss
reports. Lyrics always remain independent editable text.

Unknown JSON fields are retained by scope; XML extensions are structured trees
with namespace-aware attributes/text, and unrelated VPR member bytes remain
unextracted. Compatible export overlays current ordinary fields onto these
extensions. Disabling retention or changing formats reports incompatible fields.
UST/USTX exporters can reuse matching importer field ledgers; fields whose source
event or part layout no longer exists are reported as unsupported.

UFDATA retains source pitch precision and off-note points while the editable
timing, keys and pitch remain unchanged. A lyric edit or native save/reopen keeps
that data; a pitch/timing edit regenerates the curve and reports removed source
data. Changing native linear/cosine/cubic curves are sampled per output tick
with an explicit quantization report; held curves stay sparse. Overlapping
notes are rejected without trimming. The pinned primary sources, licensed
upstream fixture and complete conversion contract are in the
[UFDATA fixture notes](tests/fixtures/ufdata/README.md).

## Admission

Defaults bound input/output files to 64 MiB, aggregate uncompressed VPR data to
128 MiB, archives to 256 members, projects to 4096 tracks/one million notes and
four million events. VSQX has its real one-to-sixteen vocal-track output limit.
Timing is checked with integer arithmetic within the exact JSON tick range.
Maps, known XML singular fields, JSON keys, note/group identities, controller
positions and MIDI note pairing are admitted explicitly. Domain meter limits
currently require numerator 1..64 and power-of-two denominator up to 64; source
meters outside that editable range error. Bar-based destinations require meter
changes at exact bar boundaries.

VSQX requires matching Yamaha root namespaces, UTF-8, bounded node count/depth,
DTD rejection and escaped XML output. No XML entity or external file is loaded.
VPR rejects absolute/traversal/backslash/drive paths, duplicate/case-colliding
members, symlinks, encrypted/non-UTF-8 names, unsupported compression, excessive
sizes and inconsistent member lengths. Members are never extracted.

## Command line

```sh
verdant-exchange import song.mid --output song.verp --report song.import.json
verdant-exchange import source.ufdata --output source.verp
verdant-exchange import source.vsqx --output source.verp
verdant-exchange export song.verp --output song.ufdata --report song.export.json
verdant-exchange export song.verp --output song.svp --report song.export.json
verdant-exchange export song.verp --output lead.ust --track 0 --cp932
verdant-exchange export song.verp --output song.vsqx --options actual-voice-options.json
```

`--format midi|ust|ustx|svp|vsqx|vpr|ufdata` overrides score-extension inference.
Score import writes a `.verp` project and adds that suffix to an extensionless
`--output`. Score export reads `.verp` and validates the native document format
and version.
`--voice-comp-id`, `--voice-name`, `--voice-language` and `--voice-program` are an
alternative to the options sidecar; all four are required together.
`--midi-ppq` and `--discard-extensions` are available. Explicit flags override the
sidecar. Default report name is the output path with `.exchange.json` extension.
Existing outputs are never overwritten; all output files are reserved with
`create_new` before reading inputs, and failed conversions remove only their own
newly created outputs. Every success writes an ordinary project/score, a full
report and a one-line JSON result naming both paths and `has_losses`.

## Checks

Source tests cover real SMF bytes and an independent midly readback, exact meter
promotion, ambiguous pairing, duplicate maps, chunk truncation, channel-wide
bend conflicts, real SVP group references and one-blick timing, VSQX 3/4 namespace
fixtures and editable lyric escaping, actual imported voice metadata, DTD and
namespace/duplicate rejection, real VPR archives and controller tails, path/size
admission, both UTAU exporters through the actual `verdant-import` consumer, and
CLI conversion/output protection. From `frontend/`:

```sh
cargo test -p verdant-exchange --lib --tests --locked --offline
```

For editor checks, import a score, edit a note/lyric/curve, export and reopen it,
then inspect the report. Check track selection for multitrack UST and destination
voice metadata for fresh VSQX. Unrelated lyric edits must retain compatible
source properties. Format conversion does not establish playback equivalence.

Format references used to bind native schemas:

- [LibreSVIP SVP model](https://github.com/SoulMelody/LibreSVIP/blob/main/libresvip/plugins/svp/model.py)
- [LibreSVIP VOCALOID 3/4 models](https://github.com/SoulMelody/LibreSVIP/tree/main/libresvip/plugins/vsqx/models)
- [LibreSVIP VPR model](https://github.com/SoulMelody/LibreSVIP/blob/main/libresvip/plugins/vpr/model.py)
- [UTAU TimeSignatures and Mode2 grammar](https://github.com/SoulMelody/LibreSVIP/blob/main/libresvip/plugins/ust/model.py)
- [Public VSQX voice metadata fixture](https://github.com/3c1u/vsqx/blob/337d7da358b65f4cc0d85a3b9b88c29cc3de6f4b/src/test/v4.vsqx): only voice identifiers and the documented style-plugin metadata are reused in independently authored synthetic test scores. They are never runtime defaults.
