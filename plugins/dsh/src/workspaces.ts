import { createHash, randomUUID } from 'node:crypto'
import { constants } from 'node:fs'
import type { Stats } from 'node:fs'
import { chmod, lstat, mkdir, open, readdir, realpath, rename, rm, unlink } from 'node:fs/promises'
import type { FileHandle } from 'node:fs/promises'
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import type { FileAttachmentRef } from '@deepseek-ai/dsh-attachment'
import type { AttachmentStore } from '@deepseek-ai/dsh-attachment'
import type { Config } from './config.js'
import type { Principal } from './host.js'
import { BridgeError, integer, object, text } from './types.js'
import type { Json, ObjectJson } from './types.js'

const WORKSPACE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u
const SHA256 = /^[0-9a-f]{64}$/u
const INPUT_EXTENSIONS: Record<string, true> = { '.verp': true, '.ust': true, '.ustx': true, '.ufdata': true,
  '.mid': true, '.midi': true, '.svp': true, '.vsqx': true, '.vpr': true, '.zip': true }

export interface FileIdentity {
  readonly bytes: number
  readonly sha256: string
}

export interface InputFile extends FileIdentity {
  readonly id: string
  readonly name: string
  readonly relativePath: string
}

export interface Artifact extends InputFile {
  readonly role: string
  delivery: 'not_requested' | 'pending' | 'queued' | 'sent' | 'failed' | 'unknown'
  receipt?: ObjectJson
  attachment?: FileAttachmentRef
}

export interface RunRecord {
  id: string
  digest: string
  status: 'running' | 'succeeded' | 'failed' | 'cancelled' | 'interrupted'
  jobId?: string
  cliJobId?: string
  error?: string
  result?: ObjectJson
}

export interface Workspace {
  version: 'verdant-dsh-workspace/1'
  id: string
  owner: string
  createdAt: number
  updatedAt: number
  project?: string
  inputs: InputFile[]
  artifacts: Artifact[]
  runs: RunRecord[]
  active: boolean
}

function unchanged(left: Stats, right: Stats): boolean {
  return left.dev === right.dev && left.ino === right.ino && left.size === right.size
    && left.mtimeMs === right.mtimeMs && left.ctimeMs === right.ctimeMs
}

export function within(root: string, path: string): boolean {
  const child = relative(root, path)
  return child !== '' && !isAbsolute(child) && child !== '..' && !child.startsWith(`..${sep}`)
}

async function regular(path: string): Promise<FileHandle> {
  const file = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0))
  try {
    const info = await file.stat()
    if (!info.isFile() || info.nlink !== 1) throw new BridgeError('Only a regular unaliased file may be read', 'ARTIFACT_INVALID')
    return file
  } catch (error) {
    await file.close()
    throw error
  }
}

async function smallJson(path: string, limit: number): Promise<ObjectJson> {
  const file = await regular(path)
  try {
    const before = await file.stat()
    if (before.size > limit) throw new BridgeError('Workspace metadata exceeds limit', 'RESOURCE_EXHAUSTED')
    const bytes = await file.readFile()
    if (bytes.length > limit || !unchanged(before, await file.stat())) throw new BridgeError('Workspace metadata changed', 'ARTIFACT_INVALID')
    return object(JSON.parse(bytes.toString('utf8')) as unknown, 'workspace metadata')
  } finally {
    await file.close()
  }
}

async function atomicJson(path: string, value: unknown): Promise<void> {
  const temporary = join(dirname(path), `.metadata-${randomUUID()}`)
  const file = await open(temporary, 'wx', 0o600)
  try {
    await file.writeFile(JSON.stringify(value))
    await file.sync()
  } catch (error) {
    await file.close()
    await unlink(temporary)
    throw error
  }
  await file.close()
  await rename(temporary, path)
}

export class Workspaces {
  private readonly entries = new Map<string, Workspace>()
  private closed = false
  private constructor(readonly root: string, private readonly config: Config, private readonly lock: FileHandle) {}

  static async create(config: Config): Promise<Workspaces> {
    const requested = resolve(config.workspaceRoot)
    await mkdir(requested, { recursive: true, mode: 0o700 })
    if ((await lstat(requested)).isSymbolicLink()) throw new BridgeError('Workspace root must not be a symlink', 'INVALID_ARGUMENT')
    const root = await realpath(requested)
    const existing = await readdir(root)
    if (!existing.includes('verdant-dsh-root.json') && existing.length > 0) {
      throw new BridgeError('Workspace root must be empty or an existing Verdant managed root', 'INVALID_ARGUMENT')
    }
    if (existing.includes('verdant-dsh-root.json')) {
      if ((await smallJson(join(root, 'verdant-dsh-root.json'), 1024)).version !== 'verdant-dsh-root/1') {
        throw new BridgeError('Workspace root has an unsupported identity', 'VERSION_MISMATCH')
      }
    } else {
      await atomicJson(join(root, 'verdant-dsh-root.json'), { version: 'verdant-dsh-root/1' })
    }
    await chmod(root, 0o700)
    let lock: FileHandle
    try { lock = await open(join(root, '.instance.lock'), 'wx', 0o600) } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'EEXIST') {
        throw new BridgeError('Workspace root is locked. Drain the owning instance; after a crash verify processes stopped before removing .instance.lock', 'PROJECT_BUSY')
      }
      throw error
    }
    try {
      await lock.writeFile(JSON.stringify({ pid: process.pid, startedAt: Date.now() }))
    } catch (error) {
      await lock.close()
      await unlink(join(root, '.instance.lock'))
      throw error
    }
    const store = new Workspaces(root, config, lock)
    try {
      for (const name of existing) {
        if (!WORKSPACE_ID.test(name)) continue
        const directory = join(root, name)
        const info = await lstat(directory)
        if (!info.isDirectory() || info.isSymbolicLink()) throw new BridgeError('Managed workspace directory changed', 'ARTIFACT_INVALID')
        const entry = await smallJson(join(directory, 'manifest.json'), config.maxResponseBytes)
        if (entry.version !== 'verdant-dsh-workspace/1' || entry.id !== name || typeof entry.owner !== 'string'
          || !SHA256.test(entry.owner) || !Array.isArray(entry.inputs) || !Array.isArray(entry.artifacts) || !Array.isArray(entry.runs)) {
          throw new BridgeError('Managed workspace manifest is invalid', 'ARTIFACT_INVALID')
        }
        integer(entry.createdAt, 'workspace created time')
        integer(entry.updatedAt, 'workspace updated time')
        const workspace = entry as unknown as Workspace
        if (workspace.inputs.length > config.maxAttachments || workspace.runs.length > config.maxRunsPerWorkspace
          || workspace.artifacts.length > config.maxArtifactsPerWorkspace) {
          throw new BridgeError('Existing workspace exceeds configured record limits', 'RESOURCE_EXHAUSTED')
        }
        workspace.active = false
        for (const run of workspace.runs) {
          if (run.status === 'running') { run.status = 'interrupted'; run.error = 'Host restarted before a terminal result' }
        }
        for (const artifact of workspace.artifacts) {
          if (artifact.delivery === 'pending') artifact.delivery = 'unknown'
        }
        store.entries.set(name, workspace)
        await atomicJson(join(directory, 'manifest.json'), workspace)
      }
      return store
    } catch (error) {
      await store.dispose()
      throw error
    }
  }

  directory(workspace: Workspace): string { return join(this.root, workspace.id) }

  async path(workspace: Workspace, requested: string): Promise<string> {
    text(requested, 'managed relative path', 4096)
    const root = this.directory(workspace)
    const path = resolve(root, requested)
    if (!within(root, path)) throw new BridgeError('Path escapes managed workspace', 'INVALID_ARGUMENT')
    let cursor = root
    const rootInfo = await lstat(root)
    if (!rootInfo.isDirectory() || rootInfo.isSymbolicLink()) throw new BridgeError('Workspace root changed', 'ARTIFACT_INVALID')
    for (const part of relative(root, path).split(sep)) {
      cursor = join(cursor, part)
      try {
        const info = await lstat(cursor)
        if (info.isSymbolicLink()) throw new BridgeError('Managed path contains a symlink', 'ARTIFACT_INVALID')
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') break
        throw error
      }
    }
    return path
  }

  get(id: string, principal: Principal): Workspace {
    const workspace = this.entries.get(id)
    if (workspace === undefined || workspace.owner !== principal.key) throw new BridgeError('Workspace is unavailable to this requester', 'NOT_FOUND')
    return workspace
  }

  list(principal: Principal): Workspace[] {
    return [...this.entries.values()].filter(workspace => workspace.owner === principal.key)
  }

  async create(principal: Principal): Promise<Workspace> {
    if (this.closed) throw new BridgeError('Workspace service is closing', 'CANCELLED')
    if (this.entries.size >= this.config.maxWorkspaces) throw new BridgeError('Managed workspace count limit reached; remove completed workspaces', 'RESOURCE_EXHAUSTED')
    const workspace: Workspace = { version: 'verdant-dsh-workspace/1', id: randomUUID(), owner: principal.key,
      createdAt: Date.now(), updatedAt: Date.now(), inputs: [], artifacts: [], runs: [], active: false }
    // Reserve before the first await so concurrent requests cannot exceed count.
    this.entries.set(workspace.id, workspace)
    try {
      await mkdir(this.directory(workspace), { mode: 0o700 })
      for (const name of ['inputs', 'artifacts', 'runs']) await mkdir(join(this.directory(workspace), name), { mode: 0o700 })
      await this.persist(workspace)
      return workspace
    } catch (error) {
      this.entries.delete(workspace.id)
      await rm(this.directory(workspace), { recursive: true, force: true })
      throw error
    }
  }

  async persist(workspace: Workspace): Promise<void> {
    workspace.updatedAt = Date.now()
    await atomicJson(await this.path(workspace, 'manifest.json'), workspace)
  }

  async usage(workspace: Workspace): Promise<number> {
    let bytes = 0
    let count = 0
    const visit = async (directory: string): Promise<void> => {
      for (const entry of await readdir(directory, { withFileTypes: true })) {
        if (++count > 100000) throw new BridgeError('Workspace file-count limit exceeded', 'RESOURCE_EXHAUSTED')
        const path = join(directory, entry.name)
        const info = await lstat(path)
        if (info.isSymbolicLink() || (!info.isDirectory() && (!info.isFile() || info.nlink !== 1))) {
          throw new BridgeError('Managed workspace contains an unsupported file', 'ARTIFACT_INVALID')
        }
        if (info.isDirectory()) await visit(path)
        else bytes += info.size
        if (bytes > this.config.maxWorkspaceBytes) throw new BridgeError('Workspace byte limit exceeded', 'RESOURCE_EXHAUSTED')
      }
    }
    await this.path(workspace, 'manifest.json')
    await visit(this.directory(workspace))
    return bytes
  }

  async importAttachment(workspace: Workspace, ref: FileAttachmentRef, attachments: AttachmentStore, signal: AbortSignal): Promise<InputFile> {
    if (workspace.inputs.length >= this.config.maxAttachments) throw new BridgeError('Workspace attachment count limit reached', 'RESOURCE_EXHAUSTED')
    const bytes = integer(ref.bytes, 'attachment size', 0, this.config.maxAttachmentBytes)
    const name = text(ref.name, 'attachment name', 255)
    if (name === '.' || name === '..' || /[/\\:]/u.test(name)) throw new BridgeError('Attachment name must be a leaf filename', 'INVALID_ARGUMENT')
    const extension = extname(name).toLowerCase()
    if (INPUT_EXTENSIONS[extension] !== true) throw new BridgeError('Attach a .verp, supported score, or portable project ZIP', 'UNSUPPORTED_CAPABILITY')
    const digest = String(ref.attachmentId).replace(/^sha256:/u, '')
    if (!SHA256.test(digest)) throw new BridgeError('Host attachment identity must be SHA-256', 'ARTIFACT_INVALID')
    const prior = workspace.inputs.find(input => input.sha256 === digest && input.name === name)
    if (prior !== undefined) return prior
    if (await this.usage(workspace) + bytes > this.config.maxWorkspaceBytes) throw new BridgeError('Attachment exceeds workspace budget', 'RESOURCE_EXHAUSTED')
    const id = randomUUID()
    const relativePath = join('inputs', `${id}${extension}`)
    const output = await open(await this.path(workspace, relativePath), 'wx', 0o600)
    const hash = createHash('sha256')
    let total = 0
    try {
      for await (const part of attachments.readFileStream(ref, signal)) {
        signal.throwIfAborted()
        total += part.byteLength
        if (total > bytes || total > this.config.maxAttachmentBytes) throw new BridgeError('Attachment size exceeds its admitted identity', 'ARTIFACT_INVALID')
        hash.update(part)
        await output.writeFile(part)
      }
      const sha256 = hash.digest('hex')
      if (total !== bytes || sha256 !== digest) throw new BridgeError('Attachment hash or size differs from host identity', 'ARTIFACT_INVALID')
      await output.sync()
      await output.close()
      const input = { id, name, relativePath, bytes, sha256 }
      workspace.inputs.push(input)
      await this.persist(workspace)
      return input
    } catch (error) {
      await output.close()
      await unlink(await this.path(workspace, relativePath))
      throw error
    }
  }

  async inspectFile(workspace: Workspace, relativePath: string, signal: AbortSignal, maxBytes = this.config.maxArtifactBytes): Promise<FileIdentity> {
    const file = await regular(await this.path(workspace, relativePath))
    try {
      const before = await file.stat()
      if (before.size > maxBytes) throw new BridgeError('Artifact exceeds file byte limit', 'RESOURCE_EXHAUSTED')
      const hash = createHash('sha256')
      let bytes = 0
      for await (const chunk of file.createReadStream({ autoClose: false, signal, highWaterMark: 65536 })) {
        bytes += chunk.length
        if (bytes > maxBytes) throw new BridgeError('Artifact grew beyond byte limit', 'RESOURCE_EXHAUSTED')
        hash.update(chunk)
      }
      if (bytes !== before.size || !unchanged(before, await file.stat())) throw new BridgeError('Artifact changed during verification', 'ARTIFACT_INVALID')
      return { bytes, sha256: hash.digest('hex') }
    } finally {
      await file.close()
    }
  }

  async artifact(workspace: Workspace, relativePath: string, role: string, signal: AbortSignal,
    expected?: FileIdentity): Promise<Artifact> {
    const identity = await this.inspectFile(workspace, relativePath, signal)
    if (expected !== undefined && (expected.bytes !== identity.bytes || expected.sha256 !== identity.sha256)) {
      throw new BridgeError('CLI artifact digest or size does not match bytes', 'ARTIFACT_INVALID')
    }
    const existing = workspace.artifacts.find(artifact => artifact.relativePath === relativePath && artifact.sha256 === identity.sha256)
    if (existing !== undefined) return existing
    if (workspace.artifacts.length >= this.config.maxArtifactsPerWorkspace) throw new BridgeError('Workspace artifact count limit reached', 'RESOURCE_EXHAUSTED')
    const artifact: Artifact = { id: randomUUID(), relativePath, name: relativePath.split(sep).at(-1)!, role,
      ...identity, delivery: 'not_requested' }
    workspace.artifacts.push(artifact)
    await this.persist(workspace)
    return artifact
  }

  async verify(workspace: Workspace, artifact: Artifact, signal: AbortSignal): Promise<string> {
    const identity = await this.inspectFile(workspace, artifact.relativePath, signal)
    if (identity.bytes !== artifact.bytes || identity.sha256 !== artifact.sha256) throw new BridgeError('Artifact identity changed', 'ARTIFACT_INVALID')
    return this.path(workspace, artifact.relativePath)
  }

  async publish(workspace: Workspace, artifact: Artifact, attachments: AttachmentStore, signal: AbortSignal): Promise<FileAttachmentRef> {
    await this.verify(workspace, artifact, signal)
    const file = await regular(await this.path(workspace, artifact.relativePath))
    try {
      const before = await file.stat()
      const ref = await attachments.saveFileStream({ name: artifact.name, data: file.createReadStream({ autoClose: false, signal, highWaterMark: 65536 }), signal })
      if (!unchanged(before, await file.stat()) || ref.bytes !== artifact.bytes
        || String(ref.attachmentId).replace(/^sha256:/u, '') !== artifact.sha256) {
        throw new BridgeError('Published attachment differs from verified artifact', 'ARTIFACT_INVALID')
      }
      artifact.attachment = ref
      await this.persist(workspace)
      return ref
    } finally {
      await file.close()
    }
  }

  async remove(workspace: Workspace): Promise<void> {
    if (workspace.active) throw new BridgeError('Workspace is in use', 'PROJECT_BUSY')
    workspace.active = true
    const directory = this.directory(workspace)
    try {
      if (!within(this.root, directory) || !WORKSPACE_ID.test(workspace.id)) throw new BridgeError('Workspace identity is invalid', 'ARTIFACT_INVALID')
      const info = await lstat(directory)
      if (!info.isDirectory() || info.isSymbolicLink()) throw new BridgeError('Workspace directory changed', 'ARTIFACT_INVALID')
      await rm(directory, { recursive: true })
      this.entries.delete(workspace.id)
    } finally {
      workspace.active = false
    }
  }

  async dispose(): Promise<void> {
    if (this.closed) return
    this.closed = true
    await this.lock.close()
    await unlink(join(this.root, '.instance.lock'))
  }
}

export function publicArtifact(artifact: Artifact): ObjectJson {
  return { artifact_id: artifact.id, name: artifact.name, bytes: artifact.bytes, sha256: artifact.sha256,
    role: artifact.role, delivery: artifact.delivery,
    ...(artifact.receipt === undefined ? {} : { receipt: artifact.receipt }),
    ...(artifact.attachment === undefined ? {} : { attachment: artifact.attachment as unknown as Json }) }
}

export function publicWorkspace(workspace: Workspace): ObjectJson {
  return { workspace_id: workspace.id, project: workspace.project !== undefined,
    created_at: workspace.createdAt, updated_at: workspace.updatedAt, active: workspace.active,
    attachments: workspace.inputs.map(input => ({ attachment_id: input.id, name: input.name, bytes: input.bytes, sha256: input.sha256 })),
    artifacts: workspace.artifacts.map(publicArtifact),
    runs: workspace.runs.map(run => ({ run_id: run.id, status: run.status,
      ...(run.jobId === undefined ? {} : { job_id: run.jobId }),
      ...(run.cliJobId === undefined ? {} : { verdant_job_id: run.cliJobId }),
      ...(run.error === undefined ? {} : { error: run.error }) })) }
}
