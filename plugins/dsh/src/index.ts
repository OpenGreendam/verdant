import type { Context } from '@deepseek-ai/cordis'
import { defineTool } from '@deepseek-ai/dsh-tools'
import type { ParameterSchemaSpec } from '@deepseek-ai/dsh-tools'
import type { ContentBlock } from '@deepseek-ai/dsh-llm'
import { AttachmentId } from '@deepseek-ai/dsh-attachment'
import type {} from '@deepseek-ai/dsh-agent'
import type {} from '@deepseek-ai/dsh-attachment'
import type {} from '@deepseek-ai/dsh-commands'
import type {} from '@deepseek-ai/dsh-jobs'
import { Config } from './config.js'
import { VerdantBridge } from './bridge.js'
import { object } from './types.js'
import type { Json } from './types.js'

export { Config }
export type { HostCall, HostIdentity, HostIntegration, DeliveryContext, DeliveryReceipt, DeliveryFile } from './host.js'

export const name = 'verdant'
export const inject = ['tools', 'commands', 'jobs', 'attachments', 'agents']

const workspace = { type: 'string', required: true, description: 'Owned workspace_id returned by /verdant attach or verdant_project.' } as const
const recovery = { type: 'string', enum: ['saved', 'recovered'], description: 'Explicit project recovery choice, when native diagnostics require it.' } as const
const creator = { type: 'string', description: 'Exact creator version the user approved; omit unless explicitly approved.' } as const
const operations = { type: 'array', items: { type: 'object', additionalProperties: true,
  properties: { op: { type: 'string', required: true } } },
  description: 'Native domain operations from verdant_schema project.apply. One atomic transaction; paths/native effects need operator admission.' } as const
const renderParameters = {
  workspace_id: workspace,
  run_in_background: { type: 'boolean', description: 'Publish a native host job; use job_output/job_list/job_kill. Shared sessions need an admitted host adapter.' },
  idempotency_key: { type: 'string', description: 'Same key and pinned input deduplicates; a deliberate new attempt requires a new key.' },
  sample_rate: { type: 'integer', enum: [44100, 48000, 88200, 96000, 192000] },
  channels: { type: 'integer', enum: [1, 2] },
  encoding: { type: 'string', enum: ['Float32', 'Pcm16', 'Pcm24'] },
  scope: { type: 'string', enum: ['Mix', 'ActiveTrack', 'Stems'] },
  track: { type: 'string', description: 'Stable track ID; required with ActiveTrack.' },
  start_tick: { type: 'integer' }, end_tick: { type: 'integer' }, expect_revision: { type: 'integer' },
  recovery, allowed_creator: creator, seed: { type: 'integer' },
  deliver: { type: 'boolean', description: 'Explicitly request the configured trusted channel adapter to deliver audio and portable project; default false.' },
} as const satisfies ParameterSchemaSpec

function renderResult(value: Json): ContentBlock[] {
  const blocks: ContentBlock[] = [{ type: 'text', text: JSON.stringify(value) }]
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return blocks
  const artifacts = Array.isArray(value.artifacts) ? value.artifacts : value.artifact === undefined ? [] : [value.artifact]
  for (const artifact of artifacts) {
    if (artifact === null || typeof artifact !== 'object' || Array.isArray(artifact)) continue
    const attachment = artifact.attachment
    if (attachment === null || typeof attachment !== 'object' || Array.isArray(attachment)) continue
    if (typeof attachment.attachmentId !== 'string' || !/^sha256:[0-9a-f]{64}$/u.test(attachment.attachmentId)
      || typeof attachment.name !== 'string' || typeof attachment.bytes !== 'number' || !Number.isSafeInteger(attachment.bytes)) continue
    blocks.push({ type: 'file', attachment: { attachmentId: AttachmentId(attachment.attachmentId),
      name: attachment.name, bytes: attachment.bytes } })
  }
  return blocks
}

/** Native plugin entry. All runtime imports are package peers, Node builtins, or packaged lib files. */
export async function apply(ctx: Context, config: Config): Promise<void> {
  const bridge = await VerdantBridge.create(ctx, config)
  // Order-dependent shutdown stays in one awaited disposer.
  ctx.effect(() => () => bridge.dispose())
  const register = <const P extends ParameterSchemaSpec>(toolName: string, description: string, parameters: P): void => {
    ctx.tools.register(defineTool({
      name: toolName, description, parameters,
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: (_args, value) => renderResult(value),
      },
      execute: (args, exec) => bridge.tool(toolName, object(args, 'tool parameters'), exec),
    }))
  }
  register('verdant_capabilities', 'Inspect installed Verdant CLI, voice readiness, effective host integration and budgets. Configuration or registration does not establish a successful render or IM delivery.', {
    refresh: { type: 'boolean', description: 'Run the bounded read-only native doctor again.' },
  })
  register('verdant_schema', 'Discover the installed CLI operation and domain editing schemas. The bridge supplies every file path; use managed references for input/output.', {
    op: { type: 'string', description: 'For example project.apply, render, import, or export.project. Omit to list all native schemas.' },
  })
  register('verdant_project', 'Create, inspect, import, recover, migrate, copy or pack a managed editable .verp project. First admit files with /verdant attach. A song name or lyrics alone do not supply a melody.', {
    action: { type: 'string', required: true, enum: ['attach', 'new', 'inspect', 'import', 'unpack', 'save_as', 'recover', 'migrate', 'pack'] },
    workspace_id: { type: 'string', description: 'Owned workspace. Omit only when creating a new empty workspace/project.' },
    attachment_id: { type: 'string', description: 'For attach: exact host file attachment ID from the requesting message. For import/unpack: managed attachment_id from attach; never a path or URL.' },
    ppq: { type: 'integer' },
    project: { type: 'object', additionalProperties: true, description: 'Optional native project template. Native reducer validates every field.' },
    operations,
    format: { type: 'string' }, options: { type: 'object', additionalProperties: true },
    loss_policy: { type: 'string', enum: ['strict', 'allow'], description: 'strict by default. allow requires the user to accept format losses.' },
    expect_revision: { type: 'integer' }, recovery, allowed_creator: creator, label: { type: 'string' },
  })
  register('verdant_edit', 'Apply an atomic native project transaction at the expected revision. Query verdant_schema project.apply for genuine domain operation names. No direct project JSON editing.', {
    workspace_id: workspace, operations: { ...operations, required: true },
    expect_revision: { type: 'integer', required: true }, label: { type: 'string' }, dry_run: { type: 'boolean' },
    recovery, allowed_creator: creator,
  })
  register('verdant_history', 'Read or navigate the same native project history used by the desktop. Undo and redo require the current expected revision.', {
    workspace_id: workspace, action: { type: 'string', required: true, enum: ['list', 'undo', 'redo'] },
    expect_revision: { type: 'integer' }, recovery, allowed_creator: creator,
  })
  register('verdant_validate', 'Validate a managed project without creating journals. Optional renderability calls the real native plan only when the installed CLI advertises it.', {
    workspace_id: workspace, recovery, allowed_creator: creator, renderability: { type: 'boolean' },
  })
  register('verdant_render', 'Render a fixed native project with the shared Rust engine and return verified audio plus a portable desktop project ZIP. Native effects are disabled for bot projects. Background work is a native host job. Files are retained; delivery is explicit.', renderParameters)
  register('verdant_export', 'Export native shared-engine audio or a supported project exchange format with explicit loss policy. Audio includes the desktop handoff bundle; platform delivery is a separate receipt.', {
    ...renderParameters,
    kind: { type: 'string', required: true, enum: ['audio', 'project'] },
    format: { type: 'string' }, options: { type: 'object', additionalProperties: true },
    loss_policy: { type: 'string', enum: ['strict', 'allow'] }, track_id: { type: 'string' },
  })
  register('verdant_voice', 'Read the actual installed worker voice catalog or verify its admission. Singing never installs models or changes the host default voice.', {
    action: { type: 'string', required: true, enum: ['list', 'verify'] },
  })
  register('verdant_artifacts', 'List owned workspaces/artifacts, verify bytes, publish a durable host file attachment, deliver through explicit host integration, or remove completed work. Published means locally stored; queued means queued; sent requires a channel receipt. Unknown delivery is never automatically retried.', {
    action: { type: 'string', enum: ['workspaces', 'list', 'verify', 'publish', 'deliver', 'remove', 'expire', 'jobs', 'job_output', 'job_cancel'] },
    workspace_id: { type: 'string' }, artifact_id: { type: 'string' }, job_id: { type: 'string' },
  })
  ctx.commands.register({ name: 'verdant', description: 'Verdant managed score input, native rendering and desktop project handoff',
    input: { hint: 'help | capabilities | attach [workspace_id] | import/unpack <workspace_id> <attachment_id> | inspect/validate/pack/render/artifacts <workspace_id> | get <workspace_id> <artifact_id> | cancel <job_id>', attachments: true },
    handler: invocation => bridge.command(invocation),
  })
}
