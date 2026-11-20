# verdant-dsh(7) — DeepSeek Harness integration

`verdant-dsh` connects an installed Rust `verdant-cli` to Cordis tools, native
host jobs, and `/verdant`. It manages score inputs, editable projects, audio,
and portable desktop handoffs. TypeScript performs host integration and file
bookkeeping; the CLI owns music editing, pronunciation, synthesis, formats,
and audio processing.

Requires DeepSeek Harness `0.2.2-rc.5`. Installation uses a built `.tgz`.
Local file retrieval uses the host attachment store. Channel delivery
requires a configured `verdantHost` adapter and an explicit request.
[Pinned DSH source](https://github.com/deepseek-ai/deepseek-harness/tree/2e39e70496aa02986ca6a820c5d497e816b69d7e)

## Install a built package

Install matching Verdant CLI/worker and an independently admitted voice first.
End users need Node.js `>=24.9.0` and the pinned DSH host. A prebuilt `.tgz`
needs neither Rust nor TypeScript on the deployment machine.

```sh
dsh plugin --profile music add /absolute/path/verdant-dsh-0.1.0.tgz
dsh --profile music --dump-config
```

Use an existing profile selected by its owner. The bundle inserts one row
named `verdant`; it does not replace models, credentials, or other plugins.
Its required services are `tools`, `commands`, `jobs`, `attachments`, and
`agents`. Background execution needs the concrete `@deepseek-ai/dsh-jobs-local`
and the controller from `@deepseek-ai/dsh-tool-jobs`; loading the abstract
`@deepseek-ai/dsh-jobs` definition alone is insufficient. The attachment provider
must support `readFileStream` and `saveFileStream`, not just images.

Set the row in the profile's own `cordis.patch.yml`; a config override replaces
that row's whole config:

```yaml
- id: verdant
  config:
    executable: /opt/verdant/bin/verdant-cli
    cliVersion: 0.1.0
    workspaceRoot: /var/lib/dsh/verdant
    worker: /opt/verdant/bin/verdant-worker
    voiceRegistry: /var/lib/verdant/voices.json
    sessionPolicy: private
    maxActiveJobs: 2
    maxRenderSeconds: 600
```

Paths are administrator configuration, never tool arguments. Relative
`workspaceRoot` resolves against the host process working directory; choose
an absolute private directory in production. Default executable resolution
uses `PATH`, then fixes its canonical path and content SHA-256. Optional
`executableSha256` pins expected bytes. Replacement after startup requires
reload. Only native executables are accepted; shell scripts are not supported
deployment entry points on Windows. Optional `voicePackage`, `projectRoot`,
and `onnxruntimeLibrary` are forwarded only from config.

Startup performs the real `capabilities` handshake using
`verdant-cli request --json` with a versioned request on stdin, followed by EOF.
The response must be one JSON envelope with the same protocol and request ID,
an agreeing exit status, and diagnostics. The configured CLI version must
match. `doctor` probes the real worker and voice catalog. Missing voices are
reported as unavailable and leave project editing usable; they are never
replaced by a successful fake probe.

## Use tools and the command

The plugin registers these native tools:

| Tool | Native work |
| --- | --- |
| `verdant_capabilities` | CLI identity, native doctor, effective budgets and host integration declaration |
| `verdant_schema` | Installed CLI and atomic domain operation schemas |
| `verdant_project` | Attach/new/inspect/import/unpack/save_as/recover/migrate/pack |
| `verdant_edit` | One shared reducer transaction at an expected revision, including dry run |
| `verdant_history` | Native list/undo/redo |
| `verdant_validate` | Read-only validation; optional genuine native render planning |
| `verdant_render` | Shared Rust render, verified audio/report and portable project |
| `verdant_export` | Shared Rust audio or score export, with loss policy |
| `verdant_voice` | Read-only list/verify; no model installation or default mutation |
| `verdant_artifacts` | Owned files, host publication, explicit delivery, jobs and cleanup |

Every CLI operation is checked against its advertised capabilities before
execution. Render additionally requires the advertised native
`max_duration_seconds` admission, and unpack requires native
`max_expanded_bytes`. Rendering queries native `effect.status` and rejects
projects with configured inserts, so effects cannot be silently bypassed.
A missing capability is an explicit error, not an approximate fallback.
File reads and writes use managed references. Authored
project/edit JSON cannot select native plugin libraries, executable paths, or
arbitrary host files. Effects remain disabled for bot projects; a project
containing effect metadata does not authorize code loading. Other ordinary
domain operations pass through to the native schema/reducer unchanged.

Start with an uploaded score or project:

1. Submit `/verdant attach` with `.verp`, UST, USTX, UFDATA, MIDI, SVP, VSQX,
   VPR, or an authentic portable project ZIP. The command declares
   `input.attachments: true` and receives host-admitted file blocks.
2. Use `/verdant import <workspace_id> <attachment_id>` or `verdant_project`
   with `action: import`, the returned `workspace_id`,
   and managed `attachment_id`. For a portable ZIP choose `action: unpack`
   or `/verdant unpack <workspace_id> <attachment_id>`.
   `.verp` input is inspected and saved through the native reader/writer;
   source bytes remain intact. Scores use native import, strict by default.
3. Query `verdant_schema` for `project.apply`, then use `verdant_edit` with
   the current `expect_revision` to change notes, lyrics, curves, or tracks.
4. Use `verdant_render` or `/verdant render <workspace_id>`.
5. List `verdant_artifacts` or `/verdant artifacts <workspace_id>` and publish
   a chosen artifact with `/verdant get <workspace_id> <artifact_id>`.

In a natural language turn, `verdant_project action: attach` can select the
exact host attachment ID from that turn's actual requesting message. An ID
invented by the model, copied from another session, or absent from that message
does not authorize reading an attachment. An image, lyrics, or song title alone
does not establish a melody, automatic composition, or transcription capability.

`/verdant help` lists human command syntax. DSH Web/Desktop command registration
does not automatically create a QQ slash command. The Tencent channel's fixed
command table requires its own adapter and acceptance.

## Jobs, identity and shutdown

Foreground tools forward the tool cancellation signal. Background renders use
`ctx.jobs.start` with `owner: exec.agent.id` (the actual session ID), an
independent cancellation controller, and the creating tool call IDs. The
foreground invocation's later cancellation cannot kill an already published
job. Native `job_output`, `job_list`, and `job_kill` control private-session
jobs; `/verdant cancel <job_id>` and the artifact job actions are also available.

Long renders use the actual `request --events` transport. Bounded JSONL parsing
checks protocol, request ID, sequence, one terminal response and exit status.
The first native progress job ID establishes the host/native mapping; the
initial host return truthfully has only the bridge `run_id` and host `job_id`
until the native ID exists. Progress reports real lifecycle events, not
synthetic completion percentages. Native cancellation is requested when the
native job is known, then subprocess supervision signals its process tree.
Plugin disposal aborts owned work, awaits child close and POSIX process-group
retirement (Windows uses native `taskkill.exe /T /F`), awaits persistence, then
releases the root lock. Deadlines, output caps and spawn/protocol failures use
the same cleanup path.

Core DSH exposes a session identity. It does not expose a universal individual
group-user identity. `sessionPolicy: private` is an operator assertion that one
requester owns the session. When a trusted adapter supplies a user ID, ownership
also binds that ID. `sessionPolicy: shared` refuses work without a trusted
channel requester. Background jobs in shared sessions additionally require
`requesterJobs: true`: the host must fence its UI and RPC job readers and
cancellers too, because the core job registry is session-scoped. The plugin's
tool guard prevents generic model-facing job tools from bypassing its requester
checks. Use `verdant_artifacts` actions `jobs`, `job_output`, and `job_cancel`
in that mode. A session owner is never presented as group-member isolation.

Workspace manifests retain byte identities, input snapshots, run state and
artifacts. They are not a persistent host job queue. Restart changes incomplete
runs to `interrupted`; pending sends become `unknown`. A repeat render key plus
identical pinned input returns a verified completed record; a changed request
with the same key is rejected. Deliberate retries require a new key. Old host
job handles are not revived after restart. Reclaim completed work explicitly
with `remove`, or `expire` after `retentionHours`; active workspaces cannot be
deleted. Reusing a private root with another plugin process is rejected by
`.instance.lock`. After an unclean host exit the administrator must first prove
the old processes stopped before removing that stale lock.

Default limits are two active CLI/render processes, 128 managed workspaces,
16 attachments, 64 render records and 256 artifacts per workspace,
16 MiB/input, 256 MiB/artifact, 512 MiB/workspace,
600 seconds/rendered audio, 1 MiB/request, 4 MiB/complete response stream,
64 KiB/stderr, 1024 edit operations, 30 seconds/query, 10 minutes/render,
3 seconds/termination grace, and seven days/expiry eligibility. A single reserved
control process permits bounded native cancellation when render slots are full.
All limits are configured through `Config`; the native ZIP limits can only be
tightened by the bridge. Use a filesystem quota for the configured aggregate
root. Native output uses local files and bounded streams, never embedded audio
base64. The plugin executes fixed argv without a shell and never downloads URLs.

## Desktop handoff and channel delivery

A complete render result contains verified audio, its native report, and a
`project.pack` ZIP with an editable `.verp`, relative admitted assets and the
native integrity manifest. Unpack the ZIP and open the `.verp` in Verdant on the
desktop. The ZIP is a transport container. Voice weights are not included;
the desktop still needs the matching admitted voice. Exported master audio,
channel-transcoded preview audio, and playback/listening acceptance are separate
facts. Optional `desktopUrl` is an administrator-provided real HTTPS download
entry; omitted by default.

Publishing saves a verified file through the host's `AttachmentStore` and
returns its durable attachment identity. It does not send a message. A local
path is not a download URL. Host UI support for opening/downloading that returned
attachment reference must be verified. For IM, a separate operator-owned
`verdantHost` adapter may declare `fileDelivery`, `nativeVoice`,
`backgroundDelivery`, and `requesterJobs`. Merely declaring them is not platform
acceptance. The exported TypeScript interfaces define this bounded integration:

```ts
import type { HostIntegration } from 'verdant-dsh'

// An operator plugin provides this service before the Verdant row loads.
// identity(call) resolves actual channel identity from the host invocation.
// captureDelivery(call) snapshots a destination bound to session/user/platform.
// deliver(context, file, signal) returns queued/sent/failed/unknown receipts.
declare const adapter: HostIntegration
```

The adapter must not take a model-selected destination. Its token must bind the
original session, requester, platform and foreground delivery consumer. It must
honor cancellation and deadlines. Returning `queued` requires an owned byte
snapshot before return, so workspace cleanup cannot remove a borrowed file.
`sent` requires an explicit platform result; it never implies read or playback.
The plugin persists `pending` before calling it, records its actual receipt,
classifies timeout/error without an authoritative result as `unknown`, and
refuses blind repeat delivery of pending/queued/sent/unknown artifacts.

The pinned Tencent `@tencent-connect/dsh-qqbot` `0.5.5` dispatches WAV/MP3/OGG
to its voice path and other files to file delivery; that source branch does
not establish actual account playback. Pinned `@xmanrui/dsh-im` `5.10.0`
`dsh_im_return_file` returns `queued` and needs the active turn consumer. Its
QQ path uses ordinary file sending. Background completion has no automatic
foreground consumer. If safe context restoration is unavailable, render without
`deliver`, retain files, and explicitly retrieve in the original requester
context. This package never logs into QQ or sends a test message on load.

## Build and verify the package (maintainer)

Development execution belongs in the existing doescontainer development
container. Use a staged Node.js `>=24.9.0` there if its default Node is older.
Install the exact peer/dev packages from npm if available, or from artifacts
built from the pinned upstream commit. Keep the host's Cordis/tools/jobs
instances shared; runtime packages are peers rather than bundled copies.
No Verdant sibling source directory participates in TypeScript compilation.

From `plugins/dsh` in that container:

```sh
npm install --ignore-scripts
npm run typecheck
npm pack
tar -tf verdant-dsh-0.1.0.tgz
```

`prepack` compiles this package's `src` only. The archive must contain
`package/package.json`, `cordis.patch.yml`, `README.md`, `LICENSE`, and every
`lib/*.js` / `lib/*.d.ts` import closure, with no source-tree dependency,
model data, private files, runtime workspace, or absolute deployment defaults.
Use the pinned dependency artifacts in an offline install when npm cannot
resolve those versions; do not silently substitute a newer host.

Verify install closure in a new directory **outside the Verdant checkout**:

```sh
mkdir /tmp/verdant-dsh-packed-check
cd /tmp/verdant-dsh-packed-check
npm init -y
npm install --ignore-scripts /absolute/path/verdant-dsh-0.1.0.tgz
node --input-type=module -e 'const p = await import("verdant-dsh"); if (p.name !== "verdant" || typeof p.apply !== "function" || !p.Config) throw new Error("invalid plugin exports")'
dsh plugin --profile verdant-packed-check add /absolute/path/verdant-dsh-0.1.0.tgz
dsh --profile verdant-packed-check --dump-config
```

Then load that disposable profile on pinned DSH with a matching installed CLI,
private empty workspace root, real file attachment provider, jobs-local and
tool-jobs. Verify all tool schemas and `/verdant`, missing-voice editing,
bad handshake/CLI replacement, invalid/native file inputs, hash corruption,
ZIP expansion limits, revision conflict, foreground cancellation, background
kill, owner disposal, plugin unload/reload and process retirement. Exercise two
sessions and two trusted users in one shared session. Render an authorized
public/synthetic score, retrieve the bundle from the actual host UI, then open
it in the desktop and compare native project identity and audio settings.
Real channel receipt/voice playback tests require an explicitly authorized
channel and must be recorded independently. No real message is sent by any
package verification command above.

Pinned API sources:
[tool exports/schema](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/core/tools/src/schema.ts),
[tool execution contract](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/core/tools/src/contracts.ts),
[jobs](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/jobs/jobs/src/types.ts),
[command invocation](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/interaction/commands/src/index.ts),
[attachment streams](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/attachment/attachment/src/index.ts),
[package/profile loading](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/docs/user/develop/basic/publish.md).
