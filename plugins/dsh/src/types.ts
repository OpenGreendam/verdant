import { HarnessError } from '@deepseek-ai/dsh-llm'

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json }
export type ObjectJson = { [key: string]: Json }

export const PROTOCOL = 'verdant-cli/1'
export const PINNED_HOST = '2e39e70496aa02986ca6a820c5d497e816b69d7e'

export class BridgeError extends HarnessError {
  constructor(message: string, code = 'VERDANT_BRIDGE_ERROR', readonly details?: Json) {
    super(message, code)
    this.name = 'VerdantError'
  }
}

export function object(value: unknown, label: string): ObjectJson {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new BridgeError(`${label} must be an object`, 'INVALID_ARGUMENT')
  }
  return value as ObjectJson
}

export function text(value: unknown, label: string, max = 256): string {
  if (typeof value !== 'string' || value.length === 0 || Buffer.byteLength(value) > max || /[\x00-\x1f\x7f]/u.test(value)) {
    throw new BridgeError(`${label} must be nonempty, bounded text without control characters`, 'INVALID_ARGUMENT')
  }
  return value
}

export function integer(value: unknown, label: string, min = 0, max = Number.MAX_SAFE_INTEGER): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < min || value > max) {
    throw new BridgeError(`${label} must be an integer between ${min} and ${max}`, 'INVALID_ARGUMENT')
  }
  return value
}

export function keys(value: ObjectJson, allowed: readonly string[]): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) throw new BridgeError(`Unknown parameter ${key}`, 'INVALID_ARGUMENT')
  }
}

export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : 'An operation failed without an Error value'
}

export function canonical(value: Json): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key]!)}`).join(',')}}`
  }
  return JSON.stringify(value)
}
