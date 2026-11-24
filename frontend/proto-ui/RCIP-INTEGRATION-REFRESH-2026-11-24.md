# RCIP integration refresh for main #20

Publication of the earlier candidate stopped before push or PR creation when
main advanced to `79c0aa67ec37e777efc95bac5018cf2f5318da65` through #20. This
ordinary follow-up merge has parents
`9ec5f41d1d5d336ac8ab323f1ced4c68697ba0a8` and that new main commit. The prior
`9490a828` merge, `9ec5f41` clarification, reports and raw evidence remain
unchanged.

#20's authored commit `6a3586078aff9adff2b4ba8e5cd7c74a9daf755f` changes only
`frontend/crates/verdant-app/Cargo.toml` and `frontend/Cargo.lock`. It repairs
eight generated-package references; the other four native control aliases
already matched. Its constraint is retained: the app alias and lock entry must
name the package actually emitted for the selected control source and compiler.

| Exact source | App alias / generated manifest / lock matches |
| --- | --- |
| Main before #20: `92395166` | 4 of 12 |
| Current main after #20: `79c0aa67` | 12 of 12 |
| Selected `e9340df` production snapshot: `3efb43d` | 12 of 12 |

The missing-package failure preserved in the earlier integration record belongs
to historical #18. **Current main #20 has fixed that identity mismatch for its
own generated artifacts.** Those literal identities differ from the selected
`e9340df` output. The integration's existing app manifest and lock already match
that selected output, so both conflicted files remain byte-identical to
`3efb43d`.

All twelve native control mappings were parsed from actual TOML at each exact
ref. Replacing only generated identity strings with stable consumer aliases,
then canonicalizing package and dependency order, makes the entire app manifest
and all **836 lock package records** identical across the three inputs. No
additional version, source, feature or dependency-edge change is omitted.
This is a structural check, not an executed Cargo resolution.

The official retained-artifact check was rerun after resolution: **14 units,
exit 0**, with `freshCompilation: false`. No authored source, generated artifact,
pin, consumer, test or pre-existing evidence changed. The prior fresh producer
and Runtime executions retain their original source bindings; they were not
rerun or relabeled for this refresh. The newly published candidate still needs
its own locked Cargo, native, packaging and actual OS-window CI.

Exact mappings, normalization results, commands and raw receipts are in
[VALIDATION-RCIP-INTEGRATION-REFRESH-2026-11-24.json](VALIDATION-RCIP-INTEGRATION-REFRESH-2026-11-24.json).
The [earlier integration explanation](RCIP-INTEGRATION-2026-11-24.md) remains the
record of the independently reviewed source-contract decisions.
