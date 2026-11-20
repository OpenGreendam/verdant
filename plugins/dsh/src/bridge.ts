import type { Context } from '@deepseek-ai/cordis'
import type { CommandInvocation } from '@deepseek-ai/dsh-commands'
import type { JobId, JobOutcome } from '@deepseek-ai/dsh-jobs'
import type { ToolRunContext } from '@deepseek-ai/dsh-tools'
import { createHash, randomUUID } from 'node:crypto'
import { mkdir } from 'node:fs/promises'
import { extname, isAbsolute, join, relative } from 'node:path'
import type { Config } from './config.js'
import { HostAccess } from './host.js'
import type { DeliveryContext, HostIntegration, Principal } from './host.js'
import { CliTransport } from './transport.js'
import { Workspaces, publicArtifact, publicWorkspace, within } from './workspaces.js'
import type { Artifact, RunRecord, Workspace } from './workspaces.js'
import { BridgeError, PINNED_HOST, PROTOCOL, canonical, errorText, integer, keys, object, text } from './types.js'
import type { Json, ObjectJson } from './types.js'

declare module '@deepseek-ai/dsh-jobs' {
  interface JobKindMap { verdant: 'verdant' }
}

interface ActiveRun {
  readonly controller: AbortController
  readonly done: Promise<unknown>
}

interface JobOwner {
  readonly owner: string
  readonly sessionId: string
  readonly shared: boolean
}

const PROJECT_ACTIONS: Record<string, string> = {
  inspect: 'project.inspect', new: 'project.new', import: 'import', unpack: 'project.unpack',
  save_as: 'project.save-as', recover: 'project.recover', migrate: 'project.migrate', pack: 'project.pack',
}
const PROJECT_KEYS = ['action', 'workspace_id', 'attachment_id', 'ppq', 'project', 'operations', 'format',
  'options', 'loss_policy', 'expect_revision', 'recovery', 'allowed_creator', 'label']
const RENDER_KEYS = ['workspace_id', 'run_in_background', 'idempotency_key', 'sample_rate', 'channels', 'encoding', 'scope',
  'track', 'start_tick', 'end_tick', 'expect_revision', 'recovery', 'allowed_creator', 'seed', 'deliver']

/** Reject filesystem authority in model JSON before the native service interprets it. */
function authoredData(value: Json, depth = 0): void {
  if (depth > 48) throw new BridgeError('Input nesting exceeds limit', 'RESOURCE_EXHAUSTED')
  if (Array.isArray(value)) { for (const child of value) authoredData(child, depth + 1); return }
  if (value === null || typeof value !== 'object') return
  if (value.op === 'set_track_inserts') throw new BridgeError('Native effects require explicit operator admission', 'UNSUPPORTED_CAPABILITY')
  if ((value.op === 'set_track_data' || value.op === 'set_project_extension') && typeof value.key === 'string'
    && /(?:media|asset|effect|insert|plugin|runtime|credential)/iu.test(value.key)) {
    throw new BridgeError('File and native effect metadata require managed host integration', 'UNSUPPORTED_CAPABILITY')
  }
  for (const [key, child] of Object.entries(value)) {
    if (/(?:^path$|_path$|^url$|^uri$|^executable$|^library$|^program$)/iu.test(key) && child !== null && child !== '') {
      throw new BridgeError(`File authority is not accepted in authored field ${key}; use an admitted attachment`, 'INVALID_ARGUMENT')
    }
    if (/^(?:inserts|effects|plugins)$/iu.test(key) && Array.isArray(child) && child.length > 0) {
      throw new BridgeError('Native effects are not admitted for bot-authored projects', 'UNSUPPORTED_CAPABILITY')
    }
    authoredData(child, depth + 1)
  }
}

function choices(args: ObjectJson): ObjectJson {
  const fields: ObjectJson = {}
  if (args.recovery !== undefined) {
    if (args.recovery !== 'saved' && args.recovery !== 'recovered') throw new BridgeError('Choose saved or recovered explicitly', 'INVALID_ARGUMENT')
    fields.recovery = args.recovery
  }
  if (args.allowed_creator !== undefined) fields.allowed_creator = text(args.allowed_creator, 'approved creator version', 128)
  return fields
}

export class VerdantBridge {
  readonly host: HostAccess
  private readonly active = new Set<ActiveRun>()
  private readonly handling = new Set<Promise<void>>()
  private readonly shutdown = new AbortController()
  private readonly jobOwners = new Map<string, JobOwner>()
  private closed = false
  private doctor: ObjectJson | undefined
  private doctorError: ObjectJson | undefined
  private constructor(private readonly ctx: Context, readonly config: Config, readonly store: Workspaces,
    private readonly cli: CliTransport, private readonly capabilities: ObjectJson) {
    this.host = new HostAccess(ctx, config)
    ctx.on('agent/disposed', ({ agent }) => {
      for (const [id, owner] of this.jobOwners) if (owner.sessionId === agent.id) this.jobOwners.delete(id)
    })
    // Core JobSpec isolates sessions; this extra guard fences users sharing one session.
    ctx.tools.guard(execution => {
      if (!['job_list', 'job_output', 'job_kill'].includes(execution.name)) return
      const agent = execution.agent
      const requested = object(execution.arguments, 'job control arguments').job_id
      const owner = typeof requested === 'string' ? this.jobOwners.get(requested) : undefined
      if (owner !== undefined && owner.shared) return 'Shared-session Verdant jobs require verdant_artifacts action job_output or job_cancel with trusted requester identity'
      if (execution.name === 'job_list' && agent !== undefined
        && [...this.jobOwners.values()].some(job => job.shared && job.sessionId === agent.id)) {
        return 'Shared-session job listings require verdant_artifacts action jobs'
      }
      return undefined
    })
  }

  static async create(ctx: Context, config: Config): Promise<VerdantBridge> {
    if (config.maxWorkspaceBytes < config.maxArtifactBytes || config.maxWorkspaceBytes < config.maxAttachmentBytes) {
      throw new BridgeError('Workspace limit must cover each attachment and artifact limit', 'INVALID_ARGUMENT')
    }
    if (config.desktopUrl !== undefined) {
      const url = new URL(config.desktopUrl)
      if (url.protocol !== 'https:' || url.username !== '' || url.password !== '') throw new BridgeError('desktopUrl must be a public HTTPS URL without credentials', 'INVALID_ARGUMENT')
    }
    const store = await Workspaces.create(config)
    let cli: CliTransport | undefined
    try {
      cli = await CliTransport.create(config)
      const response = await cli.request('capabilities', {}, new AbortController().signal, config.queryTimeoutMs, store.root)
      const capabilities = object(response.result, 'CLI capabilities')
      if (capabilities.protocol !== PROTOCOL || capabilities.version !== config.cliVersion
        || !Array.isArray(capabilities.operations) || capabilities.operations.some(op => typeof op !== 'string')) {
        throw new BridgeError('Installed CLI protocol/version/capability shape is incompatible', 'VERSION_MISMATCH')
      }
      const bridge = new VerdantBridge(ctx, config, store, cli, capabilities)
      await bridge.probe(new AbortController().signal)
      return bridge
    } catch (error) {
      await cli?.dispose()
      await store.dispose()
      throw error
    }
  }

  private supports(op: string): void {
    if (!(this.capabilities.operations as Json[]).includes(op)) throw new BridgeError(`Installed CLI does not advertise ${op}`, 'UNSUPPORTED_CAPABILITY')
  }

  private worker(): ObjectJson {
    return { ...(this.config.worker === undefined ? {} : { worker: this.config.worker }),
      ...(this.config.voiceRegistry === undefined ? {} : { voice_registry: this.config.voiceRegistry }),
      ...(this.config.voicePackage === undefined ? {} : { voice_package: this.config.voicePackage }),
      ...(this.config.projectRoot === undefined ? {} : { project_root: this.config.projectRoot }),
      ...(this.config.onnxruntimeLibrary === undefined ? {} : { onnxruntime_library: this.config.onnxruntimeLibrary }) }
  }

  private async probe(signal: AbortSignal): Promise<void> {
    try {
      this.supports('doctor')
      this.doctor = object((await this.cli.request('doctor', this.worker(), signal, this.config.queryTimeoutMs, this.store.root)).result, 'doctor result')
      this.doctorError = undefined
    } catch (error) {
      signal.throwIfAborted()
      this.doctor = undefined
      this.doctorError = { code: error instanceof BridgeError ? error.code : 'DOCTOR_FAILED', message: errorText(error) }
    }
  }

  private scrub(value: Json, workspace?: Workspace, depth = 0): Json {
    if (depth > 64) throw new BridgeError('CLI output nesting exceeds limit', 'PROTOCOL_ERROR')
    if (typeof value === 'string') {
      let scrubbed = value.replaceAll(this.store.root, '<managed>')
      if (workspace !== undefined) scrubbed = value.replaceAll(this.store.directory(workspace), '<workspace>').replaceAll(this.store.root, '<managed>')
      for (const path of [this.config.worker, this.config.voiceRegistry, this.config.voicePackage, this.config.projectRoot,
        this.config.onnxruntimeLibrary, this.cli.executable.path]) {
        if (path !== undefined && path.length > 0) scrubbed = scrubbed.replaceAll(path, '<configured>')
      }
      if (isAbsolute(scrubbed) || /^[A-Za-z]:[\\/]/u.test(scrubbed)) return '<host-path>'
      return scrubbed
    }
    if (Array.isArray(value)) return value.map(child => this.scrub(child, workspace, depth + 1))
    if (value === null || typeof value !== 'object') return value
    const output: ObjectJson = {}
    for (const [key, child] of Object.entries(value)) {
      output[key] = /^(?:path|output|workspace|source_model|source_token|onnxruntime_library|worker|project_root|voice_registry|voice_package)$/u.test(key)
        && typeof child === 'string' && isAbsolute(child) ? '<managed-or-configured>' : this.scrub(child, workspace, depth + 1)
    }
    return output
  }

  private async request(op: string, params: ObjectJson, signal: AbortSignal, workspace: Workspace,
    timeout = this.config.queryTimeoutMs, requestId?: string, onProgress?: (event: ObjectJson) => void): Promise<ObjectJson> {
    this.supports(op)
    const response = await this.cli.request(op, params, signal, timeout, this.store.directory(workspace), requestId, onProgress)
    await this.store.usage(workspace)
    return response
  }

  private async exclusive<T>(workspace: Workspace, signal: AbortSignal, task: () => Promise<T>): Promise<T> {
    signal.throwIfAborted()
    if (this.closed) throw new BridgeError('Verdant plugin is unloading', 'CANCELLED')
    if (workspace.active) throw new BridgeError('Workspace is already in use', 'PROJECT_BUSY')
    workspace.active = true
    try { return await task() } finally { workspace.active = false; await this.store.persist(workspace) }
  }

  private project(workspace: Workspace): Promise<string> {
    if (workspace.project === undefined) throw new BridgeError('Workspace has no project; create or import one first', 'MISSING_ASSET')
    return this.store.path(workspace, workspace.project)
  }

  private async noEffects(workspace: Workspace, path: string, args: ObjectJson, signal: AbortSignal): Promise<void> {
    const response = await this.request('effect.status', { path, ...choices(args) }, signal, workspace)
    const result = object(response.result, 'native effect status')
    if (result.native_code_loaded !== false || !Array.isArray(result.inserts)) throw new BridgeError('Installed CLI cannot certify read-only effect admission', 'PROTOCOL_ERROR')
    if (result.inserts.length > 0) throw new BridgeError('Project contains effect inserts requiring explicit operator admission; bot rendering cannot silently bypass them', 'UNSUPPORTED_CAPABILITY')
  }

  private operations(value: Json | undefined): Json[] {
    if (!Array.isArray(value) || value.length === 0 || value.length > this.config.maxOperations) {
      throw new BridgeError(`operations must contain 1–${this.config.maxOperations} domain operations`, 'INVALID_ARGUMENT')
    }
    for (const operation of value) {
      text(object(operation, 'domain operation').op, 'operation name', 96)
      authoredData(operation)
    }
    return value
  }

  async capabilitiesResult(refresh: boolean, signal: AbortSignal): Promise<ObjectJson> {
    if (refresh) await this.probe(signal)
    return { plugin: 'verdant-dsh', version: '0.1.0', protocol: PROTOCOL, pinned_host_commit: PINNED_HOST,
      cli: this.scrub(this.capabilities), executable_sha256: this.cli.executable.sha256,
      doctor: this.doctor === undefined ? null : this.scrub(this.doctor), doctor_error: this.doctorError ?? null,
      host: this.host.capabilities(), effects: 'disabled for bot projects; explicit operator admission required',
      limits: { active_jobs: this.config.maxActiveJobs, workspaces: this.config.maxWorkspaces,
        attachment_bytes: this.config.maxAttachmentBytes, artifact_bytes: this.config.maxArtifactBytes,
        workspace_bytes: this.config.maxWorkspaceBytes, render_seconds: this.config.maxRenderSeconds,
        request_bytes: this.config.maxRequestBytes, response_bytes: this.config.maxResponseBytes },
      validation: 'Pinned source contracts; installed-host and real-platform validation must be recorded separately' }
  }

  async tool(name: string, args: ObjectJson, exec: ToolRunContext): Promise<ObjectJson> {
    const completion = Promise.withResolvers<void>()
    this.handling.add(completion.promise)
    const signal = AbortSignal.any([exec.signal, this.shutdown.signal])
    try {
      signal.throwIfAborted()
      const principal = await this.host.tool({ ...exec, signal })
      signal.throwIfAborted()
      switch (name) {
        case 'verdant_capabilities':
          keys(args, ['refresh'])
          return await this.capabilitiesResult(args.refresh === true, signal)
        case 'verdant_project': return await this.projectAction(args, principal, signal)
        case 'verdant_edit': return await this.edit(args, principal, signal)
        case 'verdant_validate': return await this.validate(args, principal, signal)
        case 'verdant_render': return await this.render(args, principal, signal, exec)
        case 'verdant_export':
          if (args.kind === 'audio') {
            const { kind: _kind, ...render } = args
            return await this.render(render, principal, signal, exec, 'export.audio')
          }
          return await this.exportProject(args, principal, signal)
        case 'verdant_artifacts': return await this.artifacts(args, principal, signal)
        case 'verdant_history': return await this.history(args, principal, signal)
        case 'verdant_schema': return await this.schema(args, signal)
        case 'verdant_voice': return await this.voice(args, signal)
        default: throw new BridgeError('Unknown Verdant tool', 'INVALID_ARGUMENT')
      }
    } catch (error) {
      signal.throwIfAborted()
      if (error instanceof BridgeError) throw new BridgeError(this.scrub(error.message) as string, error.code,
        error.details === undefined ? undefined : this.scrub(error.details))
      throw error
    } finally {
      completion.resolve()
      this.handling.delete(completion.promise)
    }
  }

  private async schema(args: ObjectJson, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['op'])
    this.supports('schema')
    const response = await this.cli.request('schema', args, signal, this.config.queryTimeoutMs, this.store.root)
    return { cli: this.scrub(response.result!), constraints: 'The bridge supplies all paths and rejects native effects/file authority in authored data' }
  }

  private async voice(args: ObjectJson, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['action'])
    if (args.action !== 'list' && args.action !== 'verify') throw new BridgeError('Only read-only voice queries are exposed', 'INVALID_ARGUMENT')
    const op = `voice.${args.action}`
    this.supports(op)
    return { result: this.scrub((await this.cli.request(op, this.worker(), signal, this.config.queryTimeoutMs, this.store.root)).result!) }
  }

  private async projectAction(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, PROJECT_KEYS)
    const action = text(args.action, 'project action')
    if (action === 'attach') {
      const messageFiles = principal.call.message?.content.filter(block => block.type === 'file') ?? []
      const file = messageFiles.find(block => block.attachment.attachmentId === args.attachment_id)
      if (file === undefined) throw new BridgeError('Attachment must belong to the actual requesting host message; use /verdant attach if no active message attachment is available', 'NOT_FOUND')
      const workspace = args.workspace_id === undefined ? await this.store.create(principal) : this.store.get(text(args.workspace_id, 'workspace ID'), principal)
      return this.exclusive(workspace, signal, async () => {
        await this.store.importAttachment(workspace, file.attachment, this.ctx.attachments, signal)
        return publicWorkspace(workspace)
      })
    }
    const op = PROJECT_ACTIONS[action]
    if (op === undefined) throw new BridgeError('Unknown project action', 'INVALID_ARGUMENT')
    this.supports(op)
    const workspace = args.workspace_id === undefined ? await this.store.create(principal) : this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    return this.exclusive(workspace, signal, async () => {
      let params: ObjectJson = choices(args)
      if (action === 'new') {
        if (workspace.project !== undefined) throw new BridgeError('Workspace already has a project', 'PROJECT_BUSY')
        params.output = await this.store.path(workspace, 'project.verp')
        if (args.ppq !== undefined) params.ppq = integer(args.ppq, 'ppq', 1)
        if (args.project !== undefined) { authoredData(args.project); params.project = object(args.project, 'project') }
        if (args.operations !== undefined) params.operations = this.operations(args.operations)
        if (args.label !== undefined) params.label = text(args.label, 'label', 512)
        // New projects establish their own creator and have no recovery branch.
        delete params.recovery
        delete params.allowed_creator
      } else if (action === 'import' || action === 'unpack') {
        if (workspace.project !== undefined) throw new BridgeError('Import into a new workspace', 'PROJECT_BUSY')
        const input = workspace.inputs.find(input => input.id === args.attachment_id)
        if (input === undefined) throw new BridgeError('Select an admitted attachment from this workspace', 'NOT_FOUND')
        if (extname(input.name).toLowerCase() === '.verp' && action === 'import') {
          // Save-as uses the shared native reader and writer; the input bytes stay immutable.
          const path = await this.store.path(workspace, input.relativePath)
          const inspected = await this.request('project.inspect', { path, ...params }, signal, workspace)
          const info = object(inspected.result, 'inspect result')
          params = { path, output: await this.store.path(workspace, 'project.verp'), ...params,
            expect_revision: integer(info.revision, 'imported revision') }
          const result = await this.request('project.save-as', params, signal, workspace)
          workspace.project = 'project.verp'
          return { ...publicWorkspace(workspace), result: this.scrub(result.result!, workspace), diagnostics: this.scrub(result.diagnostics!, workspace) }
        }
        params = action === 'unpack' ? { path: await this.store.path(workspace, input.relativePath), output: await this.store.path(workspace, 'portable') }
          : { source: await this.store.path(workspace, input.relativePath), output: await this.store.path(workspace, 'project.verp') }
        if (action === 'import') {
          if (args.format !== undefined) params.format = text(args.format, 'source format', 32)
          if (args.options !== undefined) { authoredData(args.options); params.options = object(args.options, 'import options') }
          params.loss_policy = args.loss_policy ?? 'strict'
        } else {
          const schema = await this.cli.request('schema', { op: 'project.unpack' }, signal, this.config.queryTimeoutMs, this.store.root)
          const properties = object(object(object(schema.result, 'unpack schema').params, 'unpack params').properties, 'unpack properties')
          if (properties.max_expanded_bytes === undefined) throw new BridgeError('Installed CLI lacks bounded portable extraction', 'UNSUPPORTED_CAPABILITY')
          params.max_expanded_bytes = Math.max(1, this.config.maxWorkspaceBytes - await this.store.usage(workspace))
          if (properties.max_archive_bytes !== undefined) params.max_archive_bytes = this.config.maxAttachmentBytes
        }
      } else {
        params.path = await this.project(workspace)
        if (action === 'pack') {
          params.output = await this.store.path(workspace, join('artifacts', `project-${randomUUID()}.zip`))
          params.allow_external_media = false
        }
        if (action === 'save_as' || action === 'migrate') {
          params.output = await this.store.path(workspace, join('artifacts', `project-${randomUUID()}.verp`))
        }
        if (['save_as', 'migrate', 'recover'].includes(action)) params.expect_revision = integer(args.expect_revision, 'expected revision')
      }
      const response = await this.request(op, params, signal, workspace)
      if (action === 'new' || action === 'import') workspace.project = 'project.verp'
      if (action === 'unpack') {
        const result = object(response.result, 'unpack result')
        const path = text(result.path, 'unpacked project path', 4096)
        if (!within(this.store.directory(workspace), path) || extname(path) !== '.verp') throw new BridgeError('Unpack returned an unsafe project path', 'ARTIFACT_INVALID')
        workspace.project = relative(this.store.directory(workspace), path)
        await this.store.path(workspace, workspace.project)
      }
      if (typeof params.output === 'string' && ['pack', 'save_as', 'migrate'].includes(action)) {
        await this.store.artifact(workspace, relative(this.store.directory(workspace), params.output), action === 'pack' ? 'desktop_bundle' : 'project', signal)
      }
      return { ...publicWorkspace(workspace), result: this.scrub(response.result!, workspace), diagnostics: this.scrub(response.diagnostics!, workspace) }
    })
  }

  private async edit(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['workspace_id', 'operations', 'expect_revision', 'label', 'dry_run', 'recovery', 'allowed_creator'])
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    return this.exclusive(workspace, signal, async () => {
      const params: ObjectJson = { path: await this.project(workspace), expect_revision: integer(args.expect_revision, 'expected revision'),
        operations: this.operations(args.operations), label: args.label === undefined ? 'DSH edit' : text(args.label, 'edit label', 512),
        dry_run: args.dry_run === true, ...choices(args) }
      const response = await this.request('project.apply', params, signal, workspace)
      return { workspace_id: workspace.id, result: this.scrub(response.result!, workspace), diagnostics: this.scrub(response.diagnostics!, workspace) }
    })
  }

  private async validate(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['workspace_id', 'recovery', 'allowed_creator', 'renderability'])
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    return this.exclusive(workspace, signal, async () => {
      const params = { path: await this.project(workspace), ...choices(args) }
      const response = await this.request('project.validate', params, signal, workspace)
      if (args.renderability !== true) return { workspace_id: workspace.id, result: this.scrub(response.result!, workspace) }
      await this.noEffects(workspace, params.path, args, signal)
      const plan = await this.request('render.plan', { ...params, ...this.worker(), effects: false,
        asset_roots: [this.store.directory(workspace)] }, signal, workspace)
      return { workspace_id: workspace.id, result: this.scrub(response.result!, workspace), render_plan: this.scrub(plan.result!, workspace) }
    })
  }

  private async history(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['workspace_id', 'action', 'expect_revision', 'recovery', 'allowed_creator'])
    if (!['list', 'undo', 'redo'].includes(String(args.action))) throw new BridgeError('Unknown history action', 'INVALID_ARGUMENT')
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    return this.exclusive(workspace, signal, async () => {
      const params: ObjectJson = { path: await this.project(workspace), ...choices(args) }
      if (args.action !== 'list') params.expect_revision = integer(args.expect_revision, 'expected revision')
      const response = await this.request(`history.${String(args.action)}`, params, signal, workspace)
      return { workspace_id: workspace.id, result: this.scrub(response.result!, workspace) }
    })
  }

  private async exportProject(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['kind', 'workspace_id', 'format', 'options', 'loss_policy', 'track_id', 'recovery', 'allowed_creator'])
    if (args.kind !== 'project') throw new BridgeError('Choose audio or project export', 'INVALID_ARGUMENT')
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    return this.exclusive(workspace, signal, async () => {
      const format = text(args.format, 'export format', 16).toLowerCase()
      const formats = this.capabilities.formats
      if (!Array.isArray(formats) || !formats.includes(format) || !/^[a-z0-9]+$/u.test(format)) throw new BridgeError('Installed CLI does not advertise this format', 'UNSUPPORTED_CAPABILITY')
      const output = await this.store.path(workspace, join('artifacts', `score-${randomUUID()}.${format === 'midi' ? 'mid' : format}`))
      const params: ObjectJson = { path: await this.project(workspace), output, format, loss_policy: args.loss_policy ?? 'strict', ...choices(args) }
      if (args.options !== undefined) { authoredData(args.options); params.options = object(args.options, 'export options') }
      if (args.track_id !== undefined) params.track_id = text(args.track_id, 'track ID')
      const response = await this.request('export.project', params, signal, workspace)
      const artifact = await this.store.artifact(workspace, relative(this.store.directory(workspace), output), 'score', signal)
      return { workspace_id: workspace.id, artifact: publicArtifact(artifact), result: this.scrub(response.result!, workspace) }
    })
  }

  private async render(args: ObjectJson, principal: Principal, signal: AbortSignal, exec: ToolRunContext | undefined,
    operation = 'render'): Promise<ObjectJson> {
    keys(args, RENDER_KEYS)
    this.supports(operation)
    this.supports('project.pack')
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    if (workspace.active) throw new BridgeError('Workspace is already in use', 'PROJECT_BUSY')
    if (this.active.size >= this.config.maxActiveJobs) throw new BridgeError('Render concurrency budget reached', 'RESOURCE_EXHAUSTED')
    signal.throwIfAborted()
    // Claim the input before any asynchronous snapshot, schema or delivery admission.
    workspace.active = true
    try {
      const result = await this.prepareRender(args, principal, signal, exec, operation, workspace)
      if (result.kind !== 'background') workspace.active = false
      return result
    } catch (error) {
      workspace.active = false
      throw error
    }
  }

  private async prepareRender(args: ObjectJson, principal: Principal, signal: AbortSignal, exec: ToolRunContext | undefined,
    operation: string, workspace: Workspace): Promise<ObjectJson> {
    const projectPath = await this.project(workspace)
    await this.noEffects(workspace, projectPath, args, signal)
    const identity = await this.store.inspectFile(workspace, workspace.project!, signal, this.config.maxAttachmentBytes)
    const requestDigest = createHash('sha256').update(canonical({ operation, args, project_sha256: identity.sha256 })).digest('hex')
    const idempotencyKey = args.idempotency_key === undefined ? principal.messageId : text(args.idempotency_key, 'idempotency key', 512)
    const requestKey = createHash('sha256').update(JSON.stringify([principal.key, idempotencyKey])).digest('hex')
    const prior = workspace.runs.find(run => run.id === requestKey)
    if (prior !== undefined) {
      if (prior.digest !== requestDigest) throw new BridgeError('Idempotency key was already used with different input', 'IDEMPOTENCY_CONFLICT')
      if (prior.status === 'succeeded' && prior.result !== undefined) {
        for (const artifact of workspace.artifacts) await this.store.verify(workspace, artifact, signal)
        return { ...prior.result, deduplicated: true }
      }
      return { workspace_id: workspace.id, run_id: prior.id, status: prior.status,
        ...(prior.jobId === undefined ? {} : { job_id: prior.jobId }), retry: 'Use a new explicit idempotency_key for a new attempt' }
    }
    if (workspace.runs.length >= this.config.maxRunsPerWorkspace) throw new BridgeError('Workspace render record limit reached', 'RESOURCE_EXHAUSTED')
    const run: RunRecord = { id: requestKey, digest: requestDigest, status: 'running' }
    const runDirectory = join('runs', randomUUID())
    const directory = await this.store.path(workspace, runDirectory)
    await mkdir(directory, { mode: 0o700 })
    const params: ObjectJson = { path: projectPath, output: join(directory, 'preview.wav'), report: join(directory, 'render.json'),
      workspace: join(directory, 'jobs'), asset_roots: [this.store.directory(workspace)], effects: false,
      ...this.worker(), ...choices(args), timeout_seconds: Math.max(1, Math.floor(this.config.renderTimeoutMs / 1000)),
      idempotency_key: requestKey }
    for (const key of ['sample_rate', 'channels', 'encoding', 'scope', 'track', 'start_tick', 'end_tick', 'expect_revision', 'seed']) {
      if (args[key] !== undefined) params[key] = args[key]!
    }
    // Resource bounds are checked by the native planner, never a second TS timing engine.
    const schemaResponse = await this.cli.request('schema', { op: operation }, signal, this.config.queryTimeoutMs, this.store.root)
    const properties = object(object(object(schemaResponse.result, 'render schema').params, 'render params schema').properties, 'render properties')
    if (properties.max_duration_seconds === undefined) {
      throw new BridgeError('Installed CLI lacks native render duration admission', 'UNSUPPORTED_CAPABILITY')
    }
    params.max_duration_seconds = this.config.maxRenderSeconds
    const wantsBackground = args.run_in_background === true
    const wantsDelivery = args.deliver === true
    const adapter = this.host.adapter()
    if (wantsBackground && principal.shared && adapter?.capabilities.requesterJobs !== true) {
      throw new BridgeError('Shared-session background jobs require trusted requester isolation across host UI and RPC job surfaces', 'UNSUPPORTED_CAPABILITY')
    }
    const delivery = wantsDelivery ? await this.host.delivery(principal) : undefined
    if (wantsDelivery && (delivery === undefined || adapter?.deliver === undefined || !adapter.capabilities.fileDelivery)) {
      throw new BridgeError('No active trusted delivery context; retrieve artifacts from the original foreground turn', 'DELIVERY_UNAVAILABLE')
    }
    if (wantsDelivery && wantsBackground && !adapter!.capabilities.backgroundDelivery) {
      throw new BridgeError('This channel cannot restore background delivery context; start without deliver and retrieve later', 'DELIVERY_UNAVAILABLE')
    }
    // Recheck after every admission await before publishing or claiming the workspace.
    signal.throwIfAborted()
    if (this.active.size >= this.config.maxActiveJobs || this.closed) throw new BridgeError('Render admission changed', 'RESOURCE_EXHAUSTED')
    workspace.runs.push(run)
    const controller = new AbortController()
    let progress: ((event: ObjectJson) => void) | undefined
    let cancellation: Promise<void> | undefined
    const cancel = (reason: unknown): void => {
      if (cancellation !== undefined || controller.signal.aborted) return
      cancellation = (async () => {
        if (run.cliJobId !== undefined && (this.capabilities.operations as Json[]).includes('job.cancel')) {
          try {
            await this.cli.request('job.cancel', { workspace: params.workspace!, job_id: run.cliJobId },
              new AbortController().signal, this.config.killGraceMs, this.store.directory(workspace), randomUUID(), undefined, true)
          } catch (error) {
            // The native lease may already be terminal; process supervision remains authoritative.
            run.error = this.scrub(errorText(error), workspace) as string
          }
        }
        controller.abort(reason)
      })()
    }
    const completion = Promise.withResolvers<ObjectJson>()
    const active = { controller, done: completion.promise }
    this.active.add(active)
    const execute = async (): Promise<ObjectJson> => {
      try {
        await this.store.persist(workspace)
        const result = await this.runRender(workspace, run, params, controller.signal, runDirectory, adapter, delivery, operation,
          event => { progress?.(event) })
        run.status = 'succeeded'
        run.result = result
        return result
      } catch (error) {
        run.status = controller.signal.aborted ? 'cancelled' : 'failed'
        run.error = this.scrub(errorText(error), workspace) as string
        throw error
      } finally {
        if (cancellation !== undefined) await cancellation
        try { await this.store.persist(workspace) } finally { workspace.active = false; this.active.delete(active) }
      }
    }
    if (wantsBackground) {
      try {
        const jobId = this.ctx.jobs.start({ kind: 'verdant', label: 'Verdant render and desktop project handoff', owner: principal.agent.id,
          ...(exec === undefined ? {} : { toolCall: { callId: exec.callId, rootCallId: exec.rootCallId } }),
          outputLimitBytes: this.config.maxResponseBytes,
          run: job => {
            run.jobId = job.id
            job.updateProgress('Rendering fixed project input')
            progress = event => {
              if (typeof event.job_id === 'string') job.updateProgress(`Verdant ${text(event.job_id, 'native job ID')}`)
            }
            const done: Promise<JobOutcome> = execute().then(result => {
              completion.resolve(result)
              return { status: 'completed' as const, result: principal.shared
                ? JSON.stringify({ job_id: run.jobId, instruction: 'Collect owned results with verdant_artifacts' })
                : JSON.stringify(result) }
            }, error => {
              completion.reject(error)
              return { status: controller.signal.aborted ? 'killed' as const : 'failed' as const,
                detail: this.scrub(errorText(error), workspace) as string }
            })
            return { cancel: reason => { cancel(new DOMException(reason ?? 'Job cancelled', 'AbortError')) }, done }
          } })
        this.jobOwners.set(jobId, { owner: principal.key, sessionId: principal.sessionId, shared: principal.shared })
        // Background errors are consumed by JobOutcome; the active promise is also drained on unload.
        void completion.promise.catch(() => undefined)
        return { kind: 'background', job_id: jobId, workspace_id: workspace.id, run_id: run.id,
          status: 'running', delivery: wantsDelivery ? 'pending' : 'not_requested',
          instructions: principal.shared ? 'Use verdant_artifacts jobs/job_output/job_cancel' : 'Use native job_output, job_list, or job_kill' }
      } catch (error) {
        controller.abort(error)
        run.status = 'failed'
        run.error = errorText(error)
        workspace.active = false
        this.active.delete(active)
        completion.resolve({ status: 'not_started' })
        await this.store.persist(workspace)
        throw error
      }
    }
    const abort = (): void => { cancel(signal.reason) }
    signal.addEventListener('abort', abort, { once: true })
    if (signal.aborted) abort()
    try {
      const result = await execute()
      completion.resolve(result)
      return result
    } catch (error) {
      completion.resolve({ status: 'failed' })
      throw error
    } finally {
      signal.removeEventListener('abort', abort)
    }
  }

  private async runRender(workspace: Workspace, run: RunRecord, params: ObjectJson, signal: AbortSignal,
    runDirectory: string, adapter: HostIntegration | undefined, delivery: DeliveryContext | undefined, operation: string,
    progress: (event: ObjectJson) => void): Promise<ObjectJson> {
    const response = await this.request(operation, params, signal, workspace, this.config.renderTimeoutMs, run.id, event => {
      if (event.job_id !== undefined && event.job_id !== null) {
        const jobId = text(event.job_id, 'native job ID')
        if (run.cliJobId !== undefined && run.cliJobId !== jobId) throw new BridgeError('Native job identity changed during render', 'PROTOCOL_ERROR')
        run.cliJobId = jobId
      }
      progress(event)
    })
    const result = object(response.result, 'render result')
    run.cliJobId = text(result.job_id, 'Verdant job ID')
    const frames = integer(result.frames, 'rendered frames')
    const sampleRate = integer(result.sample_rate, 'rendered sample rate', 1)
    if (frames / sampleRate > this.config.maxRenderSeconds) throw new BridgeError('Rendered duration exceeds configured limit', 'RESOURCE_EXHAUSTED')
    if (!Array.isArray(result.artifacts) || result.artifacts.length === 0) throw new BridgeError('CLI returned no verified artifacts', 'ARTIFACT_INVALID')
    const artifacts: Artifact[] = []
    for (const item of result.artifacts) {
      const artifact = object(item, 'CLI artifact')
      const path = text(artifact.path, 'CLI artifact path', 4096)
      const directory = join(this.store.directory(workspace), runDirectory)
      if (!within(directory, path)) throw new BridgeError('CLI returned an artifact outside its run directory', 'ARTIFACT_INVALID')
      artifacts.push(await this.store.artifact(workspace, relative(this.store.directory(workspace), path), text(artifact.role, 'artifact role'), signal,
        { bytes: integer(artifact.bytes, 'artifact bytes'), sha256: text(artifact.sha256, 'artifact hash', 64) }))
    }
    const output = await this.store.path(workspace, join(runDirectory, 'project.zip'))
    const packed = await this.request('project.pack', { path: params.path!, output, allow_external_media: false, ...choices(params) }, signal, workspace)
    const pack = object(packed.result, 'project pack result')
    const bundle = await this.store.artifact(workspace, join(runDirectory, 'project.zip'), 'desktop_bundle', signal,
      { bytes: integer(pack.byte_len, 'bundle bytes'), sha256: text(pack.sha256, 'bundle hash', 64) })
    artifacts.push(bundle)
    // The ZIP contains the editable .verp, relative assets, and native hash manifest.
    if (delivery !== undefined && adapter !== undefined) {
      for (const artifact of artifacts.filter(artifact => artifact.role === 'audio' || artifact.role === 'desktop_bundle')) {
        await this.deliver(workspace, artifact, adapter, delivery, signal)
      }
    }
    return { kind: 'completed', workspace_id: workspace.id, run_id: run.id, verdant_job_id: run.cliJobId,
      ...(run.jobId === undefined ? {} : { job_id: run.jobId }),
      frames, sample_rate: sampleRate, project_id: result.project_id ?? null, revision: result.revision ?? null,
      artifacts: artifacts.map(publicArtifact), diagnostics: this.scrub(response.diagnostics!, workspace),
      desktop: { instruction: 'Unpack the portable ZIP and open its .verp in Verdant to continue editing.',
        ...(this.config.desktopUrl === undefined ? {} : { url: this.config.desktopUrl }) } }
  }

  private async deliver(workspace: Workspace, artifact: Artifact, adapter: HostIntegration, context: DeliveryContext,
    signal: AbortSignal): Promise<void> {
    if (artifact.delivery !== 'not_requested' && artifact.delivery !== 'failed') {
      throw new BridgeError('Delivery is already queued, sent, pending, or unknown; reconcile the original receipt before retrying', 'DELIVERY_UNCERTAIN')
    }
    const path = await this.store.verify(workspace, artifact, signal)
    artifact.delivery = 'pending'
    await this.store.persist(workspace)
    const deadline = AbortSignal.timeout(this.config.queryTimeoutMs)
    try {
      // Adapter must honor cancellation, snapshot queued bytes, and release path use before settling.
      const receipt = await adapter.deliver!(context, { artifactId: artifact.id, path, name: artifact.name,
        bytes: artifact.bytes, sha256: artifact.sha256, role: artifact.role }, AbortSignal.any([signal, deadline]))
      if (!['queued', 'sent', 'failed', 'unknown'].includes(receipt.state)) throw new BridgeError('Adapter returned an invalid delivery state', 'DELIVERY_UNCERTAIN')
      artifact.delivery = receipt.state
      artifact.receipt = { state: receipt.state,
        ...(receipt.receiptId === undefined ? {} : { receipt_id: text(receipt.receiptId, 'delivery receipt ID', 1024) }),
        ...(receipt.messageId === undefined ? {} : { message_id: text(receipt.messageId, 'delivery message ID', 1024) }),
        ...(receipt.detail === undefined ? {} : { detail: text(receipt.detail, 'delivery detail', 4096) }) }
    } catch (error) {
      artifact.delivery = 'unknown'
      artifact.receipt = { state: 'unknown', detail: this.scrub(errorText(error), workspace) as string }
    }
    await this.store.persist(workspace)
  }

  private async artifacts(args: ObjectJson, principal: Principal, signal: AbortSignal): Promise<ObjectJson> {
    keys(args, ['action', 'workspace_id', 'artifact_id', 'job_id'])
    const action = args.action ?? 'list'
    if (action === 'workspaces') return { workspaces: this.store.list(principal).map(publicWorkspace) }
    if (action === 'jobs') return { jobs: this.store.list(principal).flatMap(workspace => workspace.runs.map(run => ({ workspace_id: workspace.id,
      run_id: run.id, status: run.status, ...(run.jobId === undefined ? {} : { job_id: run.jobId }) }))) }
    if (action === 'job_output' || action === 'job_cancel') {
      const id = text(args.job_id, 'host job ID')
      const owner = this.jobOwners.get(id)
      if (owner === undefined || owner.owner !== principal.key) throw new BridgeError('Job is unavailable to this requester', 'NOT_FOUND')
      if (action === 'job_cancel') return { job_id: id, cancellation: this.ctx.jobs.kill(id as JobId, principal.agent.id, 'Requester cancelled') }
      const read = this.ctx.jobs.read(id as JobId, principal.agent.id)
      const ownedRun = this.store.list(principal).flatMap(workspace => workspace.runs).find(run => run.jobId === id)
      return { job_id: id, status: read.job.status, chunks: read.chunks as unknown as Json,
        lossy: read.lossy, ...(ownedRun?.result === undefined ? {} : { result: ownedRun.result }) }
    }
    const workspace = this.store.get(text(args.workspace_id, 'workspace ID'), principal)
    if (action === 'remove') {
      await this.store.remove(workspace)
      return { workspace_id: workspace.id, removed: true }
    }
    if (action === 'expire') {
      const expired = Date.now() - workspace.updatedAt >= this.config.retentionHours * 3600000
      if (!expired) throw new BridgeError('Workspace retention period has not elapsed', 'INVALID_ARGUMENT')
      await this.store.remove(workspace)
      return { workspace_id: workspace.id, removed: true }
    }
    if (action === 'list') return publicWorkspace(workspace)
    return this.exclusive(workspace, signal, async () => {
      const artifact = workspace.artifacts.find(artifact => artifact.id === args.artifact_id)
      if (artifact === undefined) throw new BridgeError('Artifact is unavailable to this requester', 'NOT_FOUND')
      if (action === 'verify') {
        await this.store.verify(workspace, artifact, signal)
        return { workspace_id: workspace.id, artifact: publicArtifact(artifact), verified: true }
      }
      if (action === 'publish') {
        await this.store.publish(workspace, artifact, this.ctx.attachments, signal)
        return { workspace_id: workspace.id, artifact: publicArtifact(artifact), delivery: 'not_requested',
          note: 'Stored as a host attachment; no IM send has occurred' }
      }
      if (action === 'deliver') {
        const adapter = this.host.adapter()
        const context = await this.host.delivery(principal)
        if (adapter?.deliver === undefined || !adapter.capabilities.fileDelivery || context === undefined) throw new BridgeError('No active trusted foreground delivery context', 'DELIVERY_UNAVAILABLE')
        await this.deliver(workspace, artifact, adapter, context, signal)
        return { workspace_id: workspace.id, artifact: publicArtifact(artifact) }
      }
      throw new BridgeError('Unknown artifact action', 'INVALID_ARGUMENT')
    })
  }

  async command(invocation: CommandInvocation): Promise<{ kind: 'success' | 'error'; text: string }> {
    const completion = Promise.withResolvers<void>()
    this.handling.add(completion.promise)
    const signal = AbortSignal.any([invocation.signal, this.shutdown.signal])
    try {
      signal.throwIfAborted()
      const principal = await this.host.command({ ...invocation, signal })
      signal.throwIfAborted()
      const input = invocation.rawInput.trim()
      if (input === '' || input === 'help') {
        if (invocation.attachments.length > 0) return { kind: 'error', text: 'Use /verdant attach with score or project attachments.' }
        return { kind: 'success', text: 'Verdant: capabilities; attach [workspace_id]; import|unpack <workspace_id> <attachment_id>; inspect|validate|pack|render <workspace_id>; artifacts <workspace_id>; get <workspace_id> <artifact_id>; cancel <job_id>. Rendering uses native host jobs. Use verdant_schema for domain editing schemas.' }
      }
      const [action, id, artifactId, ...rest] = input.split(/\s+/u)
      if (rest.length > 0) throw new BridgeError('Too many command arguments', 'INVALID_ARGUMENT')
      const expectedArguments = action === 'get' || action === 'import' || action === 'unpack' ? 2 : action === 'capabilities' ? 0 : 1
      if ((expectedArguments < 2 && artifactId !== undefined) || (expectedArguments === 0 && id !== undefined)) {
        throw new BridgeError('Unexpected command argument; use /verdant help', 'INVALID_ARGUMENT')
      }
      if (action !== 'attach' && invocation.attachments.length > 0) throw new BridgeError('This command does not consume attachments; use attach', 'INVALID_ARGUMENT')
      let result: ObjectJson
      if (action === 'capabilities') result = await this.capabilitiesResult(true, signal)
      else if (action === 'attach') {
        const files = invocation.attachments.filter(block => block.type === 'file')
        if (files.length === 0 || files.length !== invocation.attachments.length) throw new BridgeError('Attach project or score files; images and voice-only inputs cannot supply a melody', 'INVALID_ARGUMENT')
        if (files.length > this.config.maxAttachments) throw new BridgeError('Attachment count exceeds limit', 'RESOURCE_EXHAUSTED')
        const workspace = id === undefined ? await this.store.create(principal) : this.store.get(id, principal)
        result = await this.exclusive(workspace, signal, async () => {
          for (const file of files) await this.store.importAttachment(workspace, file.attachment, this.ctx.attachments, signal)
          return publicWorkspace(workspace)
        })
      } else if (action === 'import' || action === 'unpack') result = await this.projectAction({ action, workspace_id: text(id, 'workspace ID'), attachment_id: text(artifactId, 'attachment ID') }, principal, signal)
      else if (action === 'inspect' || action === 'pack') result = await this.projectAction({ action, workspace_id: text(id, 'workspace ID') }, principal, signal)
      else if (action === 'validate') result = await this.validate({ workspace_id: text(id, 'workspace ID') }, principal, signal)
      else if (action === 'render') result = await this.render({ workspace_id: text(id, 'workspace ID'), run_in_background: true }, principal, signal, undefined)
      else if (action === 'artifacts') result = await this.artifacts({ workspace_id: text(id, 'workspace ID'), action: 'list' }, principal, signal)
      else if (action === 'get') result = await this.artifacts({ workspace_id: text(id, 'workspace ID'), artifact_id: text(artifactId, 'artifact ID'), action: 'publish' }, principal, signal)
      else if (action === 'cancel') result = await this.artifacts({ job_id: text(id, 'job ID'), action: 'job_cancel' }, principal, signal)
      else throw new BridgeError('Unknown /verdant command; use /verdant help', 'INVALID_ARGUMENT')
      return { kind: 'success', text: JSON.stringify(result) }
    } catch (error) {
      signal.throwIfAborted()
      return { kind: 'error', text: this.scrub(errorText(error)) as string }
    } finally {
      completion.resolve()
      this.handling.delete(completion.promise)
    }
  }

  async dispose(): Promise<void> {
    this.closed = true
    this.shutdown.abort(new DOMException('Plugin unloaded', 'AbortError'))
    for (const run of this.active) run.controller.abort(new DOMException('Plugin unloaded', 'AbortError'))
    await this.cli.dispose()
    await Promise.allSettled([...this.active].map(run => run.done))
    await Promise.allSettled(this.handling)
    await this.store.dispose()
  }
}
