# RCIP stack integration with the compiler-bump mainline

This integration preserves the frozen production files from stack tip
`3efb43dda237618de0935a59b41555ca69b22c2a` and merges main
`9239516638d23e6bf5211fe15a3419550ede4096` through an ordinary two-parent merge.
Only this explanation, its machine record and supporting evidence are new.
The existing #15, #16, #17 and #19 commits remain unchanged in the ancestry.

## Why the merge needs source adjudication

The common ancestor is `ac0402289e335d9a8228b4e056fb95320482458a`.
Main added #18, authored as `5818784833fa94742c14108d909a45e9bcf9632c`.
Although described as a compiler bump, that change also replaced application
protocols without migrating their Rust consumers. The actual merge has 51
conflicted paths: 33 generated files, 14 receipts, the compiler pin, README,
Action source and continuous-control tests. Another 11 main-only paths would
merge without conflict markers and therefore also require review.

Main #18's official headless job failed during `cargo fetch --locked`: the app
still required `generated-gpui-453d0654bb061524567e58b3f919af45`, but the replaced
Action artifact no longer provided that package. Fresh and retained producer
jobs passed; native, Linux and real-window jobs were skipped. The complete
[failed job log](../docs/evidence/rcip-integration-20261124/main18-headless-failure.log)
and its [source receipt](../docs/evidence/rcip-integration-20261124/main18-headless-failure-receipt.json)
preserve that boundary. The tested checkout was
`513b08c3a0066553d6d6fdfbf069551f3f5e78e2`, not a newly tested integration candidate.

The selected compiler remains `e9340df6279e063cf372080bf4cc97caea3fd901`.
Main's `bd5800dcc833e3155f7b8ca89d10d03aaef6606e` is its actual ancestor.
Selecting the descendant keeps the upstream capabilities sought by #18 and
the later native context-input repair already required by the RCIP stack.
Neither compiler history nor source acceptance is inferred from commit dates.

## Source and consumer decisions

The unchanged Rust consumers require the current Collection search,
reconciliation, context and reveal APIs; scalar key admission; Editor command
events; Scroll fallback and geometry transport; Action submit/context/held
authority; and momentary PerformanceKey triggers. The
[independent source review](../docs/evidence/rcip-integration-20261124/main18-source-contract-review.json)
identifies the actual consumer locations. The
[complete 66-path resolution plan](../docs/evidence/rcip-integration-20261124/path-resolution-plan.json)
records each selected source and reason. No side was selected solely because
it was named ours or theirs.

All 11 main-only paths were adjudicated as follows:

| Path under `frontend/proto-ui/` | Decision |
| --- | --- |
| `MIGRATION-2026-11-23.md` | Preserve the frozen report's original source and execution boundaries. No new execution supports rewriting its historical claims. |
| `src/editor-surface.proto.ts` | Keep `commandMode` and the `command` event consumed by the native editor. The replacement `audioKey` method has no migrated native caller. |
| `src/graph-point.proto.ts` | Keep the frozen source identity. The main delta only joins declarations onto one line. |
| `src/performance-key.proto.ts` | Keep physical momentary triggers and retirement when momentary policy changes; the piano-roll audition consumer relies on them. |
| `src/scroll-region.proto.ts` | Keep explicit key fallback, reveal geometry and authority, and horizontal projection. Raw descendant editing events remain protected. |
| `src/text-field.proto.ts` | Keep the official `asInputEditor` composition for directly named fields. No new independent Label pair is required by current consumers. |
| `src/upstream-input/PROVENANCE.md` | Preserve the recorded vendor-source history instead of rewriting it to an older snapshot. Compiler and vendor identities are distinguished below. |
| `src/upstream-input/provenance.json` | Preserve the exact recorded vendor hashes. |
| `src/upstream-input/root.proto.ts` | Keep both official hooks, including the full `asInputRoot` ControlLabel target. No Label capability is removed. |
| `tools/test/advanced-editors.test.mjs` | Keep the existing momentary, editor-command, threshold and retirement assertions removed by #18. |
| `tools/test/text-field.test.mjs` | Keep exact vendor provenance checks and the established directly named editor composition assertion. |

The compiler revision and the vendored Input revision are separate identities:
the compiler is `e9340df`; the immutable vendor files remain recorded at
`4e499ae25e825e130e740f16af66d70b23a743cf`. The old provenance prose's same-pin
statement belongs to that earlier snapshot and is not asserted for this
integration. The full vendor `asInputRoot` already retains ControlLabel target
behavior; the pinned GPUI profile supports it. This integration does not claim
that Label support is unavailable.

Main's early Space default-prevention line is not added separately. The current
native press path already consumes accepted Space activation, and the existing
source tests protect its ordering and authority. This merge adds no new Label
consumer or keyboard behavior. Existing runtime and native assertions remain
unchanged.

## Executed evidence

Three existing Runtime contract tests were held byte-identical and executed
against the same actual `e9340df` checkout. With the stack's authored source,
all three passed. Replacing only the authored source directory with exact main
source made all three fail: a missing momentary trigger, a missing filtered
Collection result event, and the missing scalar `key` method. There were zero
skipped cases in both executions. The exact stack source was restored afterward.
This bounded comparison is not a complete test of main's compiler or native app.

The normal merge first resolved authored sources and pin decisions. The coherent
frozen artifact snapshot supplied a valid publication inventory; the official
producer then recreated every artifact. A separate fresh check and retained
check followed. No generated Rust or receipt content was manually edited.

| Execution on the selected source | Actual result |
| --- | --- |
| Existing Runtime discriminators, stack source | 3 passed, 0 failed, 0 skipped |
| Same tests, exact main authored source | 0 passed, 3 failed, 0 skipped; expected incompatible-source control |
| Official pinned producer publication | 14 units / 81 artifacts, fresh compilation, exit 0 |
| Independent fresh compilation check | 14 units / 81 artifacts, exit 0 |
| Retained artifact check | 14 units, exit 0; explicitly not fresh compilation |
| Production-tree comparison with `3efb43d` | No changed pre-existing path after regeneration |

Commands, raw outputs, hashes, source bindings and original failures are in
[VALIDATION-RCIP-INTEGRATION-2026-11-24.json](VALIDATION-RCIP-INTEGRATION-2026-11-24.json).
Original execution paths in raw logs are retained without normalization.
Timestamped empty CI lines and the original TAP assertion layout also retain
their whitespace. The unrestricted Git whitespace check reports those raw
payload lines; the authored Markdown and JSON check passes. The raw evidence
is not trimmed to make that optional formatting check pass.

## Remaining acceptance

This local integration did not run Cargo dependency resolution, the native GUI
suite, packaging or actual OS-window input. Its own published candidate must
pass those gates before merge. Earlier #15/#16/#17/#19 results remain attached
to their actual heads; source-byte identity does not relabel them as executions
of this new merge commit. Existing frozen reports and hashes remain unchanged.
