---
name: verdant-edit-set-track-header
description: "Use to set track header in Verdant as one revision-checked, undoable atomic project edit."
---

# set track header

Inspect the project and resolve stable track/note IDs. Read `references/operation.json`; refresh `verdant schema --op project.apply --json` if the runtime differs.

Submit one completed `project.apply` request:

```json
{
  "protocol": "verdant-cli/1",
  "request_id": "unique-edit-id",
  "op": "project.apply",
  "params": {
    "path": "ABSOLUTE_PROJECT_PATH",
    "expect_revision": 0,
    "expect_project_id": "INSPECTED_PROJECT_ID",
    "label": "set track header",
    "operations": [
      {
        "op": "set_track_header",
        "track": "INSPECTED_TRACK_ID",
        "header": {
          "name": "Audio",
          "voice_ref": null,
          "monophonic": false,
          "color": "teal",
          "gain_millidecibels": -6000,
          "pan_permille": 0,
          "muted": false,
          "solo": false,
          "render_route": null
        }
      }
    ]
  }
}
```

Replace the example path, ID and revision with inspected values, and add the operation fields from the reference. The example omits task-specific data and must be completed before execution. Native units come from the schema/domain record. Multiple related commands in operations commit as one revision.

Check JSON `ok`, read back changed state and record the revision. `dry_run=true` validates without saving. Re-inspect on conflict. Preserve unknown document data and the user's recovery choice. Use `history.undo` for a requested reversal. The same request works over `verdant worker --stdio`.

Units: read project.ppq for ticks per quarter note. Tempo uses micro_bpm = BPM * 1000000 (120 BPM = 120000000). Mixer document gain is millidecibels and pan is permille; plugin controls use their native descriptor units. A dry-run creates no persistent revision or IDs: submit with the inspected base revision and use the actual result.
