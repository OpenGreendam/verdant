# verdant-import

Rust UST/USTX import for the editor and `verdant-import` command. Original
bytes become an editable `verdant_domain::Project` and a translation/loss
report. Parsing performs no file I/O or execution of source-specified programs.

## Library API

    use verdant_import::{import, ImportFormat, ImportOptions, LyricMode};

    let options = ImportOptions {
        source_name: Some("authorized.ustx".into()),
        expected_source_sha256: Some(expected_sha256.to_owned()),
        lyric_mode: LyricMode::Pinyin,
        ..ImportOptions::default()
    };
    let result = import(ImportFormat::Ustx, original_bytes, &options)?;
    let project: verdant_domain::Project = result.project;
    let report: verdant_import::ImportReport = result.report;

The public entry point is:

    pub fn import(
        format: ImportFormat,
        bytes: &[u8],
        options: &ImportOptions,
    ) -> Result<ImportResult, ImportError>;

ImportFormat has Ust and Ustx variants. ImportResult exposes project and report.
ImportError identifies an input location and reason, or the original located
YAML/JSON deserialization error. The library performs no file I/O.

ImportOptions implements Default and Serde serialization:

| Field | Contract |
| --- | --- |
| title | Optional report/catalog title; otherwise the source project name |
| source_name | Optional display name; never a path read by the library |
| tracks | Zero-based source track indices; empty selects all vocal tracks |
| ust_encoding | UstEncoding::Auto, Cp932, or Utf8 |
| lyric_mode | LyricMode::Preserve by default, or explicitly Pinyin |
| readings | ReadingOverrides with by_source_id and by_lyric pronunciation maps |
| extra_rest_lyrics | Additional caller-inspected silence markers |
| expected_source_sha256 | Optional original-byte identity admission, before decoding |
| credits | Attribution retained in the report |

The source byte SHA-256 is always computed. A supplied expected hash must match.
Persistent project, track, note and control-point IDs are deterministic for the
source and options. Event identities use t{track}/p{part}/n{note}, independently
of its distinct project note. Original source IDs, phoneme indices, aliases and
voicebank fields remain in their retained field values. Every sounding note
also carries source_note_ids for direct navigation to its original events.

No pronunciation is inferred by default. Source text survives with an unset
reading and a reading_required notice. Pinyin is an explicit caller declaration:
numbered and unnumbered Latin spellings are retained exactly; the importer does
not append a neutral tone, pick a Hanzi reading, convert kana or interpret a
voicebank phoneme alias as IPA. Acoustic inventory and phone-duration admission
remain the synthesis planner's responsibility. No MIDI pitch is clamped or
transposed to fit a voice.

Inspected readings use the existing domain Pronunciation representation:

    {
      "by_source_id": {"t0/p0/n0": {"pinyin": "ma3"}},
      "by_lyric": {
        "synthetic": {"language": "en", "alphabet": "ipa", "phonemes": ["s", "i"]}
      }
    }

A source ID override wins over a lyric override. Explicit phonemes require
language and alphabet. Unused source-ID overrides are errors, including IDs
outside a selected track. parse_readings_json(&[u8]) reads this JSON with recursive
duplicate-key rejection. Reading overrides are never generated automatically. Each event also records
source_reading_evidence (lyric_only, voicebank_phoneme_alias or not_applicable)
and its reading disposition (unresolved, declared_source_pinyin, one of the two
inspected override types, continuation or silence). Source phoneme aliases and
hints never count as canonical IPA. Timing-only phoneme overrides remain
lyric-only evidence. ImportReport::unresolved_readings() yields exact original
source IDs and lyrics for editor reading edits or an explicit acceptance report.

## Source and expression admission

* UST accepts strict CP932/Windows-31J, including Windows extensions, or strict
  UTF-8. Auto recognizes a UTF-8 BOM or valid UTF-8, otherwise CP932; the selected
  decoding is recorded. Invalid sequences are rejected without replacement.
* UST sections and keys must be unique; numbered events run from #0000 in order,
  one #SETTING is required and #TRACKEND must be last. Plugin/control sections
  are rejected. UST is 480 PPQ; a conflicting declared resolution is an error.
  The optional #SETTING TimeSignatures field admits comma-separated
  (numerator/denominator/bar_index) tuples and one optional trailing comma.
  The first meter begins at bar zero; subsequent bars increase strictly. Bar
  boundaries use the preceding meter's numerator and denominator, share the
  USTX exact-tick/overflow admission rules, and retain the original setting in
  the translated-field report. A missing setting defaults to 4/4; a malformed
  present setting is rejected.
  Length timing and the UST 2 Delta/Duration timing form are supported. Mode2
  bends need an authored PBS and at least two points; zero-width knots retain
  the right-hand value, while backward-time segments are rejected.
* Each UST event's Tempo applies at that event's onset. USTX uses a nonempty
  tempos map in preference to the compatibility bpm field. Maps begin at zero,
  increase strictly, and contain finite positive BPM up to 1000. Domain tempos
  use micro-BPM; any rounding is reported and original BPM remains in the report.
* USTX is strict UTF-8 YAML, with one document and recursively unique string
  mapping keys. Merge keys, application tags, non-finite numbers, wrong scalar
  types and malformed note/curve arrays are rejected. YAML is deserialized with
  a custom duplicate-checking visitor before constructing an object map.
* USTX resolution, part offsets, tempo changes, bar-based meter changes and vocal
  tracks are honored. Destination note/meter boundaries must be exactly
  representable at the domain's 960 PPQ. Non-power-of-two meter denominators,
  overflow and overlapping notes within a selected vocal track are errors.
* R, r, br, BR, SP and AP preserve silence timing. They do not invent breath
  audio. Gaps and trailing part silence have explicit report events. Every +
  event remains a distinct domain note with its original pitch, timing and
  note-local controls, including when MIDI pitch changes. Its continuation flag
  sustains the preceding adjacent note's final nucleus without an invented
  pronunciation or a new consonant onset. The report records both its own
  project_note_id and continuation_of_project_note_id. Ordinary adjacent notes
  with the same pitch retain separate articulation boundaries. Orphan,
  cross-rest and gapped + events are rejected. An unresolved anchor stays
  unresolved for synthesis preflight; import never guesses its reading.

Supported source controls are converted into actual editable domain controls:

| Source | Source units | Domain destination |
| --- | --- | --- |
| UST Intensity | Percent linear amplitude | gain_millidecibel_points using 20 log10(amplitude) |
| UST PBS/PBW/PBY/PBM | Milliseconds; pitch Y = 10 cents; linear/sine shapes | pitch_millicent_points |
| UST VBR | Length %, period ms, depth cents, fades, phase, drift | Sampled pitch_millicent_points |
| USTX note pitch | X ms; Y = 10 cents; l/io/i/o shapes, snap_first | pitch_millicent_points |
| USTX tuning | Cents | pitch_millicent_points |
| USTX pitd part curve | Cents; part-relative ticks | Added pitch_millicent_points |
| USTX dyn part curve | 0.1 dB; part-relative ticks | gain_millidecibel_points |
| USTX vibrato | Length %, period ms, depth cents, fades, phase, drift, volume link | Sampled pitch and gain controls |
| USTX mixer | Volume dB, pan -100..100, mute, solo | Track gain, pan, mute and solo |

Source bends, vibrato and dynamics are combined before writing controls. Rich
vibrato is compiled to pitch/gain curves rather than losing fades or phase in a
simpler vibrato preset. No extra automatic portamento is introduced. Sampling
uses at most five project ticks between points, includes curve knots and tempo
boundaries, and rounds values to millicents/millidecibels. The expression_sampling
notice records this approximation; it does not claim continuous renderer or
waveform equivalence. Original expression parameters remain in FieldReport.
The finite domain gain/pitch bounds are checked without clamping. Zero UST
Intensity requires an explicit silence decision because a finite gain curve
cannot represent exact zero. An expressive note requiring more than one million
samples is rejected with an authored-split diagnostic.

Pitch regions before or after the owning notated note have no corresponding
note-local domain control. The in-note portion is translated and the
pitch_outside_note notice identifies the retained, unexecuted region. Spline
pitch, legacy UST Mode1 bends, sample-dependent envelopes/preutter/overlap,
resampler flags, per-phoneme voicebank expressions, phoneme timing overrides,
custom renderer controls, singer/plugin settings and audio parts remain exact
report values without guessed substitutes. An unsupported shape retains its
whole bend rather than silently substituting a different interpolation.
Source controls on a rest are retained as unsupported, never counted as active
translations. Source singer/plugin settings do not alter the domain's voice-sn
reference or execute any source voicebank. Render preferences start from the
authoritative Project::new_empty defaults; the caller selects rendering assets
and route separately.

Source schema, unit and interpolation authorities:

* [LibreSVIP UST TimeSignatures grammar](https://github.com/SoulMelody/LibreSVIP/blob/main/libresvip/plugins/ust/model.py)
* [OpenUtau UstNote](https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Classic/UstNote.cs)
* [OpenUtau UNote / UVibrato / UPitch](https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Ustx/UNote.cs)
* [OpenUtau UCurve](https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Ustx/UCurve.cs)
* [OpenUtau RenderPhrase](https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Render/RenderPhrase.cs)
* [OpenUtau interpolation functions](https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Util/MusicMath.cs)

The Rust importer verifies source identity and preserves ticks, rests, tempo maps,
supported expression, and note continuations. Reproducing fixed-tempo research
checks requires their original inputs.

## Report and CLI

ImportReport uses schema verdant-score-import/1. It records original-byte identity,
source encoding/PPQ, source tempo/meter maps, source-to-domain event mappings, the declared lyric mode and per-event reading
provenance, track counts/ranges, original values, field dispositions, notices, duration and
credits. Field dispositions are translated, retained_unsupported and metadata.
unsupported_fields() selects the retained unsupported fields; notices also carry
partial-translation limits such as out-of-note pitch regions. The complete
report is embedded at project.extensions.score_import and also returned to the
caller, so unsupported fields survive a normal project save.

    verdant-import /path/to/song.ustx --output /path/to/new/song.verp \
      --report /path/to/new/song.import.json \
      --catalog /path/to/new/manifest.json --pinyin \
      --source-sha256 ORIGINAL_SHA256 --credit 'Source attribution'

SOURCE and --output are required. The extension selects UST/USTX unless --format
ust or ustx is supplied. --track INDEX is repeatable; --encoding auto|cp932|utf8
selects UST decoding. --title, --readings JSON, repeatable --rest-lyric and
repeatable --credit correspond to ImportOptions. --report defaults to the project
path with .import.json as its extension. --help prints the complete contract.
The command prints a JSON summary with paths, source hash, counts and duration.

The command writes `.verp` and adds that suffix to an extensionless `--output`.
UtaFormatix `.ufdata`, MIDI, SVP, VSQX and VPR import/export are provided by the
`verdant-exchange` CLI and editor file menu.

Every output must be a new path. The CLI refuses existing files and symlinks,
uses create_new for final admission, and writes private mode 0600 on Unix. It
never overwrites source scores or existing audition projects. A requested
catalog is a new single-project verdant-audition-catalog/1 manifest using the
existing app schema; it does not append to an existing catalog or claim to
render audio. ImportResult::audition_catalog(project_path) exposes the same
manifest construction for editor integration.

## Checks

From `frontend/`:

```sh
cargo test --offline --locked -p verdant-import
```

Repository fixtures are synthetic. For a local score, import into new paths,
open the result in the editor, and compare tempo/meter maps, source event IDs,
note ranges, readings and translated controls with the report. Each sounding
event, including `+`, owns a separate note; rests remain report events. Reports
bind the original file SHA-256. Keep source scores, derived projects, sidecars
and audio outside the source/release payload. Synthesis and listening require
separate checks.
