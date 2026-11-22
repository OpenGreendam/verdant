---
name: verdant-project-new
description: "Use when an AI Agent needs to create a native project at an explicit output path. Performs project.new through Verdant's CLI or NDJSON worker."
---

# project.new

Complete one operation: create a native project at an explicit output path.

## Procedure

1. Locate `verdant` and run `verdant capabilities --json` once per session. Bare `verdant` displays help and exits; GUI startup is explicit with `verdant gui`.
2. Read `references/operation.json`. Refresh with `verdant schema --op project.new --json` when the runtime differs. Supply only admitted fields, stable IDs and native units.
3. Resolve required paths and IDs from the user task or an inspect/list response. Choose explicit output destinations for file-producing operations. Submit and verify the response.

## Invocation

Use structured stdin with an argument array:

```python
request = {
    "protocol": "verdant-cli/1",
    "request_id": "unique-task-operation-id",
    "op": "project.new",
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

Required fields: `output`. The reference contains optional fields and nested structures.

## Details

Use output alone for a valid initialized native document, then use revision-checked project.apply for edits. An optional project payload must be a complete validated native project record, not a partial patch. Inspect an existing project for its full structure; preserve unknown data.

A dry-run response is a preview: its revision, commit and newly generated IDs are not persisted. Submit the real edit against the original inspected revision and use only the real response's created IDs.

## Verification and recovery

Check exit status and JSON `ok`. Return actual IDs, revisions, paths and material diagnostics. Read back edited state. Use `history.undo` for a requested reversal. On `REVISION_CONFLICT` inspect and rebuild the edit. Preserve the user's open document on writer-lock errors. After a transport timeout inspect before retrying a non-idempotent edit. Missing audio and incomplete mappings are explicit errors; never substitute silence or guessed controls.

The same request works as one JSON line on `verdant worker --stdio`. Match `request_id`, consume progress through its result record, and close stdin when finished. Requests are sequential. This workflow worker differs from the synthesis executable `verdant-worker`.
