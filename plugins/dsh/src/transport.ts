import { spawn } from 'node:child_process'
import { constants } from 'node:fs'
import type { Stats } from 'node:fs'
import { access, open, realpath, stat } from 'node:fs/promises'
import { createHash, randomUUID } from 'node:crypto'
import { delimiter, isAbsolute, join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import type { Config } from './config.js'
import { BridgeError, PROTOCOL, object, text } from './types.js'
import type { Json, ObjectJson } from './types.js'

interface Program {
  path: string
  identity: string
  sha256: string
}

function identity(value: Stats): string {
  return `${value.dev}:${value.ino}:${value.size}:${value.mtimeMs}:${value.ctimeMs}`
}

async function program(requested: string): Promise<Program> {
  text(requested, 'executable', 4096)
  const selected = process.platform === 'win32' && !/\.[a-z0-9]+$/iu.test(requested) ? `${requested}.exe` : requested
  const candidates = isAbsolute(selected) || selected.includes('/') || selected.includes('\\')
    ? [selected]
    : (process.env.PATH ?? '').split(delimiter).filter(Boolean).map(directory => join(directory, selected))
  for (const candidate of candidates) {
    let path: string
    try {
      path = await realpath(candidate)
      await access(path, process.platform === 'win32' ? constants.F_OK : constants.X_OK)
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT' || (error as NodeJS.ErrnoException).code === 'EACCES') continue
      throw error
    }
    const file = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0))
    try {
      const before = await file.stat()
      if (!before.isFile() || /\.(?:cmd|bat)$/iu.test(path)) {
        throw new BridgeError('Configure a native verdant-cli executable', 'INVALID_ARGUMENT')
      }
      const hash = createHash('sha256')
      for await (const chunk of file.createReadStream({ autoClose: false })) hash.update(chunk)
      const after = await file.stat()
      if (identity(before) !== identity(after)) throw new BridgeError('CLI executable changed during admission', 'VERSION_MISMATCH')
      return { path, identity: identity(after), sha256: hash.digest('hex') }
    } finally {
      await file.close()
    }
  }
  throw new BridgeError('Installed verdant-cli was not found or is not executable', 'CLI_UNAVAILABLE')
}

function missingProcess(error: unknown): boolean {
  return (error as NodeJS.ErrnoException).code === 'ESRCH'
}

export class CliTransport {
  private readonly running = new Set<{ cancel: () => void; done: Promise<unknown> }>()
  private closing = false
  private constructor(private readonly config: Config, readonly executable: Program) {}

  static async create(config: Config): Promise<CliTransport> {
    const executable = await program(config.executable)
    if (config.executableSha256 !== undefined && config.executableSha256.toLowerCase() !== executable.sha256) {
      throw new BridgeError('Installed CLI SHA-256 differs from configuration', 'VERSION_MISMATCH')
    }
    return new CliTransport(config, executable)
  }

  async request(op: string, params: ObjectJson, signal: AbortSignal, timeoutMs: number, cwd: string,
    requestId: string = randomUUID(), onProgress?: (event: ObjectJson) => void, control = false): Promise<ObjectJson> {
    signal.throwIfAborted()
    if (this.closing) throw new BridgeError('Plugin is unloading', 'CANCELLED')
    const slots = this.config.maxActiveJobs + (control ? 1 : 0)
    if (this.running.size >= slots) throw new BridgeError('CLI process budget is full', 'RESOURCE_EXHAUSTED')
    const bytes = Buffer.from(JSON.stringify({ protocol: PROTOCOL, request_id: requestId, op, params }))
    if (bytes.length > this.config.maxRequestBytes) throw new BridgeError('CLI request exceeds byte limit', 'RESOURCE_EXHAUSTED')
    const admitted = await stat(this.executable.path)
    signal.throwIfAborted()
    if (identity(admitted) !== this.executable.identity) throw new BridgeError('Installed CLI changed; reload the plugin', 'VERSION_MISMATCH')
    // Recheck admission after the asynchronous executable check, before spawn.
    if (this.closing || this.running.size >= slots) {
      throw new BridgeError('CLI process admission is unavailable', 'RESOURCE_EXHAUSTED')
    }
    const child = spawn(this.executable.path, ['request', onProgress === undefined ? '--json' : '--events'], {
      cwd, shell: false, windowsHide: true, detached: process.platform !== 'win32', stdio: ['pipe', 'pipe', 'pipe'],
    })
    let failure: unknown
    let hasFailure = false
    let forceTimer: NodeJS.Timeout | undefined
    let killer: Promise<void> | undefined
    const fail = (error: unknown): void => {
      if (!hasFailure) { hasFailure = true; failure = error }
    }
    const kill = (force: boolean): void => {
      if (child.pid === undefined) return
      if (process.platform === 'win32') {
        if (killer !== undefined) return
        const systemRoot = process.env.SystemRoot
        if (systemRoot === undefined || !isAbsolute(systemRoot)) {
          fail(new BridgeError('Windows tree termination requires SystemRoot', 'CLEANUP_FAILED'))
          child.kill('SIGKILL')
          return
        }
        const completion = Promise.withResolvers<void>()
        killer = completion.promise
        const task = spawn(join(systemRoot, 'System32', 'taskkill.exe'), ['/PID', String(child.pid), '/T', '/F'],
          { shell: false, windowsHide: true, stdio: 'ignore' })
        task.once('error', completion.reject)
        task.once('close', code => code === 0 || child.exitCode !== null || child.signalCode !== null
          ? completion.resolve() : completion.reject(new BridgeError('Windows process tree termination failed', 'CLEANUP_FAILED')))
        // The close path awaits this promise; keep early rejection observed.
        void killer.catch(error => { fail(error) })
        return
      }
      try { process.kill(-child.pid, force ? 'SIGKILL' : 'SIGTERM') } catch (error) {
        if (!missingProcess(error)) fail(error)
      }
    }
    const cancel = (reason: unknown): void => {
      fail(reason)
      kill(false)
      forceTimer ??= setTimeout(() => { kill(true) }, this.config.killGraceMs)
    }
    const onAbort = (): void => { cancel(signal.reason ?? new DOMException('Operation cancelled', 'AbortError')) }
    const timeout = setTimeout(() => {
      cancel(new BridgeError('Verdant CLI deadline exceeded', 'TIMEOUT'))
    }, timeoutMs)
    const stdout: Buffer[] = []
    let terminal: ObjectJson | undefined
    let sequence = 0
    let pending: Buffer[] = []
    let pendingBytes = 0
    const line = (bytes: Buffer): void => {
      if (bytes.length === 0) return
      const event = object(JSON.parse(bytes.toString('utf8')) as unknown, 'CLI event')
      if (event.protocol !== PROTOCOL || event.request_id !== requestId || event.sequence !== sequence + 1 || terminal !== undefined) {
        throw new BridgeError('CLI event identity or sequence mismatch', 'PROTOCOL_ERROR')
      }
      sequence += 1
      if (event.event === 'progress') onProgress!(event)
      else if (event.event === 'result') terminal = event
      else throw new BridgeError('Unexpected CLI event kind', 'PROTOCOL_ERROR')
    }
    let stdoutBytes = 0
    let stderrBytes = 0
    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBytes += chunk.length
      if (stdoutBytes > this.config.maxResponseBytes) {
        cancel(new BridgeError('CLI response exceeds byte limit', 'RESOURCE_EXHAUSTED'))
        return
      }
      if (onProgress === undefined) { stdout.push(chunk); return }
      try {
        let start = 0
        for (let end = chunk.indexOf(10, start); end !== -1; end = chunk.indexOf(10, start)) {
          const part = chunk.subarray(start, end)
          line(pendingBytes === 0 ? part : Buffer.concat([...pending, part], pendingBytes + part.length))
          pending = []
          pendingBytes = 0
          start = end + 1
        }
        if (start < chunk.length) {
          const part = chunk.subarray(start)
          pending.push(part)
          pendingBytes += part.length
        }
      } catch (error) { cancel(error instanceof BridgeError ? error : new BridgeError('CLI emitted malformed JSONL', 'PROTOCOL_ERROR')) }
    })
    // Do not echo arbitrary native stderr into host history or credentials logs.
    child.stderr.on('data', (chunk: Buffer) => {
      stderrBytes += chunk.length
      if (stderrBytes > this.config.maxStderrBytes) cancel(new BridgeError('CLI diagnostic stream exceeds byte limit', 'RESOURCE_EXHAUSTED'))
    })
    child.stdin.on('error', error => { if ((error as NodeJS.ErrnoException).code !== 'EPIPE') cancel(error) })
    child.once('error', error => { fail(error) })
    const completion = Promise.withResolvers<{ code: number | null; signal: NodeJS.Signals | null }>()
    const closed = completion.promise
    child.once('close', (code, exitSignal) => { completion.resolve({ code, signal: exitSignal }) })
    const active = { cancel: () => { cancel(new DOMException('Plugin unloaded', 'AbortError')) }, done: closed as Promise<unknown> }
    this.running.add(active)
    signal.addEventListener('abort', onAbort, { once: true })
    if (signal.aborted) onAbort()
    // One-shot CLI consumes stdin to EOF. Never leave a live writable pipe.
    child.stdin.end(bytes)
    const complete = async (): Promise<ObjectJson> => {
      try {
        const exit = await closed
        if (killer !== undefined) await killer
        // A trusted CLI must not leave worker processes in its process group.
        if (process.platform !== 'win32' && child.pid !== undefined) {
          kill(true)
          const deadline = Date.now() + this.config.killGraceMs
          for (;;) {
            let present = false
            try { process.kill(-child.pid, 0); present = true } catch (error) {
              if (!missingProcess(error)) throw error
            }
            if (!present) break
            if (Date.now() >= deadline) throw new BridgeError('CLI process group did not retire within cleanup deadline', 'CLEANUP_FAILED')
            await delay(25)
          }
        }
        if (hasFailure) throw failure
        if (exit.signal !== null) throw new BridgeError(`CLI terminated by ${exit.signal}`, 'WORKER_FAILED')
        let parsed: unknown
        try {
          if (onProgress !== undefined && pendingBytes > 0) line(Buffer.concat(pending, pendingBytes))
          parsed = onProgress === undefined ? JSON.parse(Buffer.concat(stdout, stdoutBytes).toString('utf8')) as unknown : terminal
        } catch {
          throw new BridgeError('CLI did not return one complete JSON response', 'PROTOCOL_ERROR')
        }
        const response = object(parsed, 'CLI response')
        if (response.protocol !== PROTOCOL || response.request_id !== requestId || typeof response.ok !== 'boolean'
          || !Array.isArray(response.diagnostics)) {
          throw new BridgeError('CLI protocol or response identity mismatch', 'PROTOCOL_ERROR')
        }
        if (response.ok) {
          if (exit.code !== 0 || !Object.hasOwn(response, 'result') || Object.hasOwn(response, 'error')) {
            throw new BridgeError('CLI success envelope disagrees with exit status', 'PROTOCOL_ERROR')
          }
          return response
        }
        const error = object(response.error, 'CLI error')
        if (exit.code === 0 || Object.hasOwn(response, 'result')) throw new BridgeError('CLI failure envelope disagrees with exit status', 'PROTOCOL_ERROR')
        if (typeof error.message !== 'string' || typeof error.retryable !== 'boolean') throw new BridgeError('CLI error is malformed', 'PROTOCOL_ERROR')
        const details = { diagnostics: response.diagnostics as Json[], retryable: error.retryable,
          ...(error.details === undefined ? {} : { details: error.details }) }
        throw new BridgeError(`${error.message}\n${JSON.stringify(details)}`, text(error.code, 'CLI error code'), details)
      } finally {
        clearTimeout(timeout)
        if (forceTimer !== undefined) clearTimeout(forceTimer)
        signal.removeEventListener('abort', onAbort)
        this.running.delete(active)
      }
    }
    const done = complete()
    active.done = done
    return done
  }

  async dispose(): Promise<void> {
    this.closing = true
    const active = [...this.running]
    for (const run of active) run.cancel()
    await Promise.allSettled(active.map(run => run.done))
  }
}
