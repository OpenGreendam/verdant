---
name: verdant
description: "Use when an AI Agent needs to operate Verdant end to end, choose atomic skills, install the offline bundle, or use the CLI and workflow worker."
---

# Verdant Agent workflow

Use the installed headless `verdant` command. Bare `verdant` prints help and exits. Open a desktop with `verdant gui`; packaged users can double-click the root `verdant-launcher`.

## Discover and install

```text
verdant capabilities --json
verdant schema --json
verdant skill list --json
verdant skill install --all --agent codex --scope user
```

Use `--agent claude` or `--agent generic` for those layouts. `--scope project --project-root /absolute/project` installs for a project; `--dest` chooses an explicit directory. Read `skill.show` for one skill. Installation is offline, hash checked and protects edited/unrelated files.

## Work from stable identities

1. Inspect the project; resolve track, clip and plugin instance IDs. Device IDs identify types; instance_id identifies a saved slot.
2. Choose the operation-specific `verdant-*` skill, inspect its schema and send structured JSON on stdin.
3. Bind mutations to the recoverable revision and project ID. Honor the saved/recovered choice. Keep related edits in one `project.apply`, `plugin.edit`, `mixer.apply` or `clip.edit` transaction.
4. Read back. Export through `render.plan` then `export.audio`; measure the actual file with `analysis.audio`.
5. Report actual outputs and diagnostics. Loading a preset does not prove mix quality; inspect the rendered material.

| Task | Family |
| --- | --- |
| Native project lifecycle, recovery, history | project, history |
| Notes, lyrics, curves and structure | verdant-edit-* via project.apply |
| Audio admission and fingerprints | media |
| Slice, trim, slip, reverse, stretch, fades, bounce | clip |
| Inserts, native controls, bypass and sidechain | plugin |
| Faders, pan, mute, solo and automation | mixer |
| Factory library and native/FFP import/export | preset |
| Mix, stems, ranges, jobs and cache | render, export, job, cache |
| Voices and runtime diagnostics | voice, doctor |
| External plugin discovery | effect |
| Measured file statistics | analysis |
| Offline Agent Skills lifecycle | skill |

## Worker and atomicity

Launch `verdant worker --stdio`. Send complete `verdant-cli/1` requests as JSON lines, match progress/result by request_id, and close stdin. One request runs at a time. `verdant-worker` is the distinct synthesis backend.

`batch` is sequential and preserves prior commits after a later failure. Use typed atomic edit batches for all-or-nothing changes. After a timeout inspect state before retrying. JSON errors are authoritative; resolve missing assets and avoid guessed parameter mappings.
