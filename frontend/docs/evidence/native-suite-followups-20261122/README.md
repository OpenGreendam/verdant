# Native studio follow-up acceptance

Date: 2026-11-22. Follow-up to merged PR #4, base `91a7755`.
These records describe actual native binaries and PCM outputs. Historical
receipts are preserved as captured; later closures are explained below.

## Delivered surface

- 14 native Adwaita device editors: 12 effects, Synth and One, including shared
  presets, A/B, precise parameter editing and complete parameter-list fallback.
- 280 original factory presets, 20 per device, with complete parameter states,
  stable IDs, categories, authors, tags and usage notes.
- Shared GUI / CLI / worker clip editing: trim, fractional-tick movement,
  split/range operations, portable copy/paste, slip, reverse, rate and
  pitch-preserving stretch, fades/crossfade, normalization, silence and
  zero-crossing split, join and float-WAV bounce.
- Plugin, mixer, automation, preset and analysis operations through the shared
  transactional service; 95 CLI operations and 128 offline Agent Skills.
- Nonblocking command entry, explicit `verdant gui`, root launcher, guarded
  offline skill installation, and native package manifest v3.
- EQ Sketch/Match and IR parameter fitting with diagnostic errors; realtime
  callback admission/PDC; native offline processing at five selected rates.

## Local integration gates

| Gate | Result | Record |
| --- | --- | --- |
| Offline, locked workspace build | Passed | `followups-workspace-build.log` |
| Complete workspace tests | 1,120 passed, 0 failed; 16 opt-in cases initially ignored | `followups-workspace-tests.log` |
| Actual plugin host signal/control/rate cases | 6 passed | `followups-real-host-tests.log` |
| Hosted CLAP instrument and external FFP corpus | 2 passed | Same log |
| Actual sidechain, tempo and physical-slot routing | 1 passed | `followups-sidechain-host-final.log` |
| Final CLI/launcher entrypoint cases | 4 passed | `followups-entrypoints-final.log` |
| Proto workspace GPUI rules | Five cases passed | `followups-proto-rules.log` |
| Pinned Proto generated-output check | Local skip: pinned upstream checkout absent | `followups-proto-generated.log` |
| Current offline Skills file manifest | 128 skills; file hashes and LF bytes verified | `integration.json` |

Nine of the 16 opt-in cases were subsequently executed above. Remaining cases
require a separately supplied external native effect bundle or source-preserving
voice-bank fixtures; they are not counted as passed. The generated-output and
platform package gates run again in PR CI.

The full workspace includes actual-DSP calibration of all 280 presets, EQ/IR
fitting, clip batches, atomic plugin/mixer edits, callback budget/PDC, and skills
installation protection. Current native executable hashes are in `integration.json`.

## Presets and compatibility

`factory-presets.json` records all 280 presets and their actual rendered PCM
hash, peak, RMS, latency and finite-sample checks. Calibration is one second of
stereo multitone/noise plus one second of tail, or C4 at velocity 0.7 for
instruments. All presets are audible and finite in that calibration. This is
original Verdant content; the test does not guarantee clipping prevention or
perceptual quality for arbitrary material.

`ffp-coverage.json` records 1,769 external presets: 23 fully mapped, 437 partially
mapped, and 1,309 identified without a supported complete mapping. Current
Compressor/Reverb translations remove 372 unsupported fields in the affected
corpus. Vendor preset files and UI assets are not included in the repository.

## Independent Agent forward tests and closures

The first agent used only installed Skills, schemas and the CLI to import/split
stereo audio, reverse the second half, add native vocal compressor/reverb presets,
set gain, save/reopen, export at 44.1/96 kHz, measure PCM and use the NDJSON
worker. Independent decoding verified reverse plus gain with maximum sample
error 1.6993644e-8. Dry runs and invalid batches did not change persisted inputs.
See `skills-forward-test.json`.

That historical receipt retains its original findings. Current dispositions:

| Finding | Closure |
| --- | --- |
| Plan/export media authorization disagreement | Shared admission; missing-root negative regression passed |
| Explicit-range plan included the full tail | Bounded output/processing timelines; 0–2 s and 1–2 s regression passed |
| Ambiguous asset_roots/project_root/workspace | Generated Skills and automation guide explain all three |
| Incomplete add-track and native-unit guidance | Executable examples/unit mapping; independent new-track and 123 BPM test passed |
| Dry-run revision/IDs looked persisted | Preview semantics; independent disk hash/revision check passed |
| Explicit-range report said Full | Selection reporting; final 88.2/192 kHz exports each contain exactly one second |

`skills-forward-final.json` records the independent second pass with all 128
skills: new track, tempo, dry run, EQ fit/match, IR analysis, returned-parameter
application and read-only analysis checks. Its minor documentation notes were
subsequently clarified in the generated bundle. `range-export-final.json` records
the final actual Selection exports. `skills-installer.json` is an earlier
125-skill receipt; the current manifest and package gate cover all 128.

`review-closure.json` closes implementation review findings: real One bypass
silence and bit-identical restore, authorized cross-project copy/paste, and
staged bounce publication/recoverable errors. Bounce → CopyRange in one batch
also passes in the same NDJSON worker without a crash or dry-run file mutation.
Publication recovery was checked by source inspection and targeted service
tests; real disk failure was not injected into the GUI.

## Native UI and imported audio

The 14-device comparison, dark/narrow layouts, preset workflow and actual analysis
interactions are in [the aesthetic evidence](../native-suite-aesthetics-20261122/README.md)
and [the visual review](../../native-plugin-visual-review.md).

Clip screenshots here show reopened stereo waveforms, playback with native EQ,
Alt precision movement and verified copy/paste. The saved precision example is
`start_tick=742`, `start_subtick=0.9424008190403583`. Final GUI acceptance used the
current debug UI with an optimized release DSP host. An earlier unoptimized
Limiter throughput failure and the successful optimized-host replay of the same
fixture are recorded in the aesthetic evidence.

## Practical boundaries

These are independent Verdant processors with comparable workflows; proprietary
FabFilter DSP and binary preset equivalence are not claimed. IR analysis fits an
algorithmic reverb, rather than loading a convolution kernel. The host is stereo,
without Atmos object routing/certification. Realtime remains 48 kHz; offline
44.1/48/88.2/96/192 kHz uses the selected native processing clock. Native
instrument static parameters, MIDI expression and bypass are supported; project
automation targets remain effect/mixer based.

Linux native windows were observed under Xvfb/Openbox/Mesa. Windows/macOS native
builds and packages are evaluated by PR CI. Physical audio/MIDI, screen readers,
IME composition, BSD desktops and multi-display HiDPI need target-hardware
acceptance and are not marked tested here.
