import type { Agent } from '@deepseek-ai/dsh-agent'
import type { Context } from '@deepseek-ai/cordis'
import type { ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { CommandInvocation } from '@deepseek-ai/dsh-commands'
import type { UserMessage } from '@deepseek-ai/dsh-session'
import { createHash } from 'node:crypto'
import type { Config } from './config.js'
import { BridgeError, text } from './types.js'
import type { ObjectJson } from './types.js'

export interface HostCall {
  readonly agent: Agent
  readonly invocationId: string
  readonly signal: AbortSignal
  readonly message?: UserMessage
  readonly turn?: number
  readonly command?: CommandInvocation
}

export interface HostIdentity {
  /** Trusted channel requester identity, never model-supplied text. */
  readonly userId: string
  readonly platform: string
  readonly messageId?: string
  readonly turnId?: string
}

export interface DeliveryContext {
  /** Immutable host-owned snapshot binding the destination and requester. */
  readonly token: string
  readonly sessionId: string
  readonly userId: string
  readonly platform: string
  readonly turnId?: string
}

export interface DeliveryReceipt {
  readonly state: 'queued' | 'sent' | 'failed' | 'unknown'
  readonly receiptId?: string
  readonly messageId?: string
  readonly detail?: string
}

export interface DeliveryFile {
  readonly artifactId: string
  readonly path: string
  readonly name: string
  readonly bytes: number
  readonly sha256: string
  readonly role: string
}

/** Optional administrator-owned adapter; not a new QQ login or message transport. */
export interface HostIntegration {
  readonly protocol: 'verdant-dsh-host/1'
  readonly capabilities: {
    readonly fileDelivery: boolean
    readonly nativeVoice: boolean
    readonly backgroundDelivery: boolean
    /** Host also fences every UI/RPC job reader and canceller by requester. */
    readonly requesterJobs: boolean
  }
  identity(call: HostCall): HostIdentity | undefined | Promise<HostIdentity | undefined>
  captureDelivery?(call: HostCall): DeliveryContext | undefined | Promise<DeliveryContext | undefined>
  /** A queued receipt MUST own a byte snapshot, not borrow the path after return. */
  deliver?(context: DeliveryContext, file: DeliveryFile, signal: AbortSignal): Promise<DeliveryReceipt>
}

declare module '@deepseek-ai/cordis' {
  interface Context {
    verdantHost: HostIntegration
  }
}

export interface Principal {
  readonly agent: Agent
  readonly sessionId: string
  readonly userId: string
  readonly platform: string
  readonly key: string
  readonly messageId: string
  readonly turnId?: string
  readonly call: HostCall
  readonly shared: boolean
}

export class HostAccess {
  private readonly claimed = new WeakMap<Agent, { message: UserMessage; turn: number }>()

  constructor(private readonly ctx: Context, private readonly config: Config) {
    ctx.on('agent/inbox/claimed', ({ agent, message, turn }) => {
      // Tool completion notices cannot replace the human requester identity.
      const source: string = message.source.kind
      if (source !== 'tool-jobs') this.claimed.set(agent, { message, turn })
    })
    ctx.on('session/event', (session, event) => {
      if (event.type !== 'turn/end') return
      const agent = ctx.agents.get(session.id)
      if (agent !== undefined) this.claimed.delete(agent)
    })
  }

  adapter(): HostIntegration | undefined {
    const adapter = this.ctx.get('verdantHost')
    if (adapter !== undefined && adapter.protocol !== 'verdant-dsh-host/1') {
      throw new BridgeError('Unsupported trusted host adapter protocol', 'VERSION_MISMATCH')
    }
    return adapter
  }

  async tool(exec: ToolRunContext): Promise<Principal> {
    if (exec.agent === undefined) throw new BridgeError('Verdant requires a live caller session', 'CONTEXT_UNAVAILABLE')
    const claimed = this.claimed.get(exec.agent)
    return this.resolve({ agent: exec.agent, invocationId: exec.callId, signal: exec.signal,
      ...(claimed === undefined ? {} : { message: claimed.message, turn: claimed.turn }) })
  }

  command(invocation: CommandInvocation): Promise<Principal> {
    return this.resolve({ agent: invocation.agent, invocationId: invocation.commandId,
      signal: invocation.signal, command: invocation })
  }

  private async resolve(call: HostCall): Promise<Principal> {
    call.signal.throwIfAborted()
    const identity = await this.adapter()?.identity(call)
    call.signal.throwIfAborted()
    const shared = this.config.sessionPolicy === 'shared'
    if (shared && identity === undefined) {
      throw new BridgeError('Shared sessions require trusted requester identity from verdantHost', 'REQUESTER_UNAVAILABLE')
    }
    const sessionId = text(call.agent.id, 'host session ID', 1024)
    const userId = identity === undefined ? `private-session:${sessionId}` : text(identity.userId, 'host requester ID', 1024)
    const platform = identity === undefined ? 'dsh-private' : text(identity.platform, 'host platform', 128)
    const turnId = identity?.turnId ?? (call.turn === undefined ? undefined : String(call.turn))
    const key = createHash('sha256').update(JSON.stringify([sessionId, platform, userId])).digest('hex')
    return { agent: call.agent, sessionId, userId, platform, key, shared, call,
      messageId: text(identity?.messageId ?? call.message?.id ?? call.invocationId, 'host message ID', 1024),
      ...(turnId === undefined ? {} : { turnId: text(turnId, 'host turn ID', 1024) }) }
  }

  capabilities(): ObjectJson {
    const adapter = this.adapter()
    return {
      identity: adapter === undefined ? 'private-session-only' : 'trusted-host-adapter',
      session_policy: this.config.sessionPolicy,
      file_delivery: adapter?.capabilities.fileDelivery ?? false,
      native_voice: adapter?.capabilities.nativeVoice ?? false,
      background_delivery: adapter?.capabilities.backgroundDelivery ?? false,
      requester_jobs: adapter?.capabilities.requesterJobs ?? false,
      delivery_validation: 'operator adapter declaration; platform acceptance required',
    }
  }

  async delivery(principal: Principal): Promise<DeliveryContext | undefined> {
    const context = await this.adapter()?.captureDelivery?.(principal.call)
    if (context !== undefined && (context.sessionId !== principal.sessionId || context.userId !== principal.userId
      || context.platform !== principal.platform || (principal.turnId !== undefined && context.turnId !== principal.turnId))) {
      throw new BridgeError('Delivery context does not belong to this requester', 'OWNERSHIP_MISMATCH')
    }
    if (context !== undefined) text(context.token, 'delivery context token', 4096)
    return context
  }
}
