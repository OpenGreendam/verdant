---
name: verdant-analysis-eq-fit
description: "Use when an AI Agent needs to fit a measured or drawn target response with native EQ bands. Performs analysis.eq-fit through Verdant's CLI or NDJSON worker."
---

# analysis.eq-fit

Complete one operation: fit a measured or drawn target response with native EQ bands.

## Procedure

1. Locate `verdant` and run `verdant capabilities --json` once per session. Bare `verdant` displays help and exits; GUI startup is explicit with `verdant gui`.
2. Read `references/operation.json`. Refresh with `verdant schema --op analysis.eq-fit --json` when the runtime differs. Supply only admitted fields, stable IDs and native units.
3. Resolve required paths and IDs from the user task or an inspect/list response. Choose explicit output destinations for file-producing operations. Submit and verify the response.

## Invocation

Use structured stdin with an argument array:

```python
request = {
    "protocol": "verdant-cli/1",
    "request_id": "unique-task-operation-id",
    "op": "analysis.eq-fit",
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

Required fields: `points`. The reference contains optional fields and nested structures.

## Details

points are [frequency_Hz, gain_dB] pairs. max_bands is 1..24; use the intended processing sample_rate. This is read-only: inspect fit.rms_error_db/max_error_db, then apply returned values to an EQ instance with revision-checked plugin.set. A finite band fit does not exactly reproduce every drawn curve.

## Verification and recovery

Check exit status and JSON `ok`. Return actual IDs, revisions, paths and material diagnostics. Read back edited state. Use `history.undo` for a requested reversal. On `REVISION_CONFLICT` inspect and rebuild the edit. Preserve the user's open document on writer-lock errors. After a transport timeout inspect before retrying a non-idempotent edit. Missing audio and incomplete mappings are explicit errors; never substitute silence or guessed controls.

The same request works as one JSON line on `verdant worker --stdio`. Match `request_id`, consume progress through its result record, and close stdin when finished. Requests are sequential. This workflow worker differs from the synthesis executable `verdant-worker`.
