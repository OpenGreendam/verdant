# Acoustic option stages

Rust acoustic-option stages for native DSP. The numerical oracle is the frozen
`ce1ce6c` source identified below. The installed `verdant-voice` route has its own
versioned package contract; see [voice installation](../../../../../docs/verdant-voice.md).

The native DSP stages run in this order when their options are enabled:

1. `apply_spectral_options`: explicit phonetic realization followed by the
   bilinear or anchored LPC-envelope transform.
2. `dsp::synthesize_run`: raw excitation/filter output. DSP calls
   `consonant_noise_dynamics` once before filtering, applies its multiplicative
   `sample_gain` to the original noise envelope, and retains the unchanged
   lattice-filter reference only if that envelope actually changes.
3. `apply_phonetic_waveform`: inspect `calibration_reference_waveform` if it is
   present, otherwise the candidate; apply the one resulting contact envelope
   to both arrays.
4. `dsp::calibrate_run`: one existing relative-gain stage and one 0.65 run
   calibration using the unchanged-source reference peak when present.
5. `apply_waveform_options`: optional voiced-vowel model dynamics after
   calibration, with no subsequent normalization.

Run arrays retain the Python names: `lsf_radians`, `generated_log_gain`,
`lsf_radians_before_phonetic_correction`, and
`lsf_radians_before_vocal_tract`. The planner supplies `sample_rate`,
`hop_samples`, `segments`, `frame_counts`, and `native_keys` in run metadata.
The segments are the original prepared objects and keys are the actual audited
selection, rather than a label or a guessed identity. A null key cannot validate
an explicit phoneme trace.

Continuation notes retain separate segment rows, frame counts and
`controls.segment_identity` values so their timing traces, DYN/expression and
model-dynamics anchors retain note ownership. Their F0 remains the planner's
per-note F0. Source/excitation continuity uses the explicit continuation link
owned by the editor/planner/DSP: the segment's optional `continuation_of` points
to its original preceding final voiced-nucleus anchor, and the planner propagates
`source_continuation_links: [{segment_index, continuation_of}]` in run metadata.
Options preserves both forms, never infers that link from equal phone IDs, and
never merges segment identities. Compatible adjacent vowel recipes share their
source-relative scale and spectral edge envelope while preserving every report
row. Tract maps are shared by all frames, and consonant dynamics does not make a
copied voiced nucleus eligible for unvoiced-stop conditioning. Ordinary scores
retain the original option behavior.

Spectral reports are in run metadata `phonetic_correction` and `vocal_tract`.
`vocal_tract.output_bandwidth_gamma` is 1 for an active transform because the
source expansion is already included; neutral transforms retain the configured
gamma. `output_bandwidth_gamma` also records that effective value at the run
metadata root. DSP copies the reports into its `dsp` metadata. Contact correction
updates the audio's `dsp.phonetic_correction` report and emits the original
`phonetic_correction_gain_frame_mean` and
`phonetic_correction_gain_frame_min` arrays. Model dynamics emits
`model_dynamics_sample_gain` and `model_dynamics_sample_eligible`.

The neutral anchored configuration `[1, 1]` uses the same bilinear implementation
and evaluation order. Active anchors use SciPy's monotone PCHIP derivatives and
48 bisection steps to invert that actual forward cubic. The AR fit operates on
warped log power at 8192 FFT points, retains the same filter order, rejects an
unstable fit, and reports prediction-error gain and finite-order error. The
adapter intentionally retains the existing inverse-RMS/peak gain policy.

The phonetic profile and full phoneme inventory are byte-preserved embedded
assets. The inventory's canonical current member is
`frontend/crates/verdant-backend/src/data/phoneme_inventory.json`; acoustic
options and native voice semantics embed that same member. The distinct
`phonetic_correction_profiles.json` remains in this directory.
The Japanese recipes require the explicit language, IPA, contextual
realization, and actual native model path to agree. Ordinary Chinese selections
do not select a Japanese recipe. Existing helper spectra, UV/class/identity
boundaries, F0 and timing remain under their respective owners.

Historical oracle source paths and SHA-256 values at `ce1ce6c`:

| Source | SHA-256 |
| --- | --- |
| `native/greendam_native/phonetic_config.py` | `f82e2f28d0893d7dba84023a6df554366d4b5031b7810d2435c05313a6c08271` |
| `native/greendam_native/phonetic_correction.py` | `43c31cd27542de467924bcd251b1796a2586b105457913dd8a3ef7e423b0f5b6` |
| `native/greendam_native/phonetic_correction_profiles.json` | `914d64a666de32ba38533743387945416cad9ebaab1d15f87669f023d4aaefdf` |
| `native/greendam_native/phoneme_inventory.json` | `ec8b6d7d741bc4a08b4b7f68fa18981fc0e5770aedf24f270b5b2c0c1c7ce4cd` |
| `native/verdant_acoustics/voice_transform.py` | `dd8333ce0c165876b39f3a6a9f71c9397a1d577188727a792568b5862d9c020b` |
| `native/verdant_acoustics/model_dynamics.py` | `d1906b13c5dce07e72aed82ab329842204f7e05189235a17ae5ddd3e9315e708` |
| `native/verdant_acoustics/consonant_dynamics.py` | `0d8629738df0a57d524949d3371dc9856c23bbfa04b4f3325d997e1da4b764a2` |

The Rust code uses the existing `anyhow`, `serde_json`, `sha2`, `hex`, and
`rustfft` dependencies and the shared strict conversions
`dsp::lpc::{lsf_to_lpc,lpc_to_lsf}`. It adds no manifest or native runtime assets.
Reports distinguish the current Rust source digest from its numerical oracle
digest.

## Checks

From `frontend/`:

```sh
cargo test --offline --locked -p verdant-synthesis options::
```

Historical parity replay requires the frozen `ce1ce6c` Python source and its
matching tools and inputs. Current Rust regressions
cover exact neutral anchors, the actual anchored
inverse, bandwidth expansion once, explicit Japanese admission, protected outer
bands, split-vowel continuity, fractional clocks, extreme median residuals,
protected signed zero, and unchanged-reference contact decisions.
Active FFT/Toeplitz/LSF comparisons require
measured tolerances; exactness is asserted for neutral/protected data and the
two shared bilinear paths, not for cross-library active transforms.
