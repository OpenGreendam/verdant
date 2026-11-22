---
name: verdant-mixer-automate
description: "Use when an AI Agent needs to author a validated automation lane. Performs mixer.automate through Verdant's CLI or NDJSON worker."
---

# mixer.automate

Complete one operation: author a validated automation lane.

## Procedure

1. Locate `verdant` and run `verdant capabilities --json` once per session. Bare `verdant` displays help and exits; GUI startup is explicit with `verdant gui`.
2. Read `references/operation.json`. Refresh with `verdant schema --op mixer.automate --json` when the runtime differs. Supply only admitted fields, stable IDs and native units.
3. Run `project.inspect` on the chosen document. Resolve tracks/clips/instances from its result. Use the current recoverable revision for `expect_revision` and bind `expect_project_id`. Honor the user's saved/recovered branch choice; do not silently discard recovery.
4. Submit the edit. Use `dry_run=true` when review is needed; re-inspect after another writer changes the revision. Validation precedes one undoable transaction.

## Invocation

Use structured stdin with an argument array:

```python
request = {
    "protocol": "verdant-cli/1",
    "request_id": "unique-task-operation-id",
    "op": "mixer.automate",
    "params": params,  # Resolve from the schema and actual project IDs.
}
run = subprocess.run(
    [verdant_binary, "request", "--json"],
    input=json.dumps(request), text=True, capture_output=True, timeout=timeout_seconds,
)
response = json.loads(run.stdout)
if not response.get("ok"):
    raise RuntimeError(response.get("error"))
```

Required fields: `path`, `expect_revision`, `lane`. The reference contains optional fields and nested structures.

## Details

Ticks strictly increase. Use step interpolation for stepped controls; control-only parameters cannot be automated. Gain lanes use millidecibels, pan lanes use permille. Plugin lanes use native parameter units.

A dry-run response is a preview: its revision, commit and newly generated IDs are not persisted. Submit the real edit against the original inspected revision and use only the real response's created IDs.

## Verification and recovery

Check exit status and JSON `ok`. Return actual IDs, revisions, paths and material diagnostics. Read back edited state. Use `history.undo` for a requested reversal. On `REVISION_CONFLICT` inspect and rebuild the edit. Preserve the user's open document on writer-lock errors. After a transport timeout inspect before retrying a non-idempotent edit. Missing audio and incomplete mappings are explicit errors; never substitute silence or guessed controls.

The same request works as one JSON line on `verdant worker --stdio`. Match `request_id`, consume progress through its result record, and close stdin when finished. Requests are sequential. This workflow worker differs from the synthesis executable `verdant-worker`.
