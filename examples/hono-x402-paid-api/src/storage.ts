// examples/hono-x402-paid-api/src/storage.ts maps receipt reads to ic.audit.
// The canister runtime owns append-only storage and hash-root maintenance.

import type { AuditEvent, AuditRoot, Receipt } from './types'

const MAX_EVENTS = 100

export function auditRoot(): AuditRoot {
  return globalThis.ic.audit.root()
}

export function reserveReceipt(id: string, payload: Record<string, unknown>): AuditEvent {
  return globalThis.ic.audit.reserve(id, JSON.stringify(payload))
}

export function commitReceipt(id: string, receipt: Receipt): AuditEvent {
  return globalThis.ic.audit.commit(id, JSON.stringify(receipt))
}

export function failReceipt(id: string, payload: Record<string, unknown>): AuditEvent {
  return globalThis.ic.audit.fail(id, JSON.stringify(payload))
}

export async function readAuditEvents(): Promise<AuditEvent[]> {
  const count = Math.max(0, auditRoot().count)
  const offset = Math.max(0, count - MAX_EVENTS)
  return globalThis.ic.audit.list(offset, MAX_EVENTS).filter(isAuditEvent)
}

export async function readReceipts(): Promise<Receipt[]> {
  const events = await readAuditEvents()
  return events
    .filter((event) => event.kind === 'commit')
    .map(receiptFromEvent)
    .filter(isReceipt)
}

export async function findReceipt(id: string): Promise<Receipt | null> {
  const event = globalThis.ic.audit.get(id)
  if (!event || event.kind !== 'commit') return null
  const receipt = receiptFromEvent(event)
  return isReceipt(receipt) ? receipt : null
}

function receiptFromEvent(event: AuditEvent): unknown {
  try {
    return JSON.parse(event.payloadJson)
  } catch {
    return null
  }
}

function isAuditEvent(value: unknown): value is AuditEvent {
  if (!isRecord(value)) return false
  return (
    typeof value.index === 'number' &&
    typeof value.id === 'string' &&
    isAuditKind(value.kind) &&
    typeof value.payloadJson === 'string' &&
    typeof value.prev_root === 'string' &&
    typeof value.event_hash === 'string' &&
    typeof value.root === 'string'
  )
}

function isAuditKind(value: unknown): value is AuditEvent['kind'] {
  return value === 'reserve' || value === 'commit' || value === 'fail'
}

function isReceipt(value: unknown): value is Receipt {
  if (!isRecord(value)) return false
  return (
    typeof value.id === 'string' &&
    typeof value.productId === 'string' &&
    typeof value.endpoint === 'string' &&
    typeof value.method === 'string' &&
    typeof value.price === 'string' &&
    typeof value.payTo === 'string' &&
    typeof value.paymentRequirementsHash === 'string' &&
    typeof value.paymentSignatureHash === 'string' &&
    typeof value.payerHash === 'string' &&
    typeof value.transaction === 'string' &&
    typeof value.network === 'string' &&
    typeof value.amount === 'string' &&
    typeof value.resultDigest === 'string' &&
    typeof value.canisterId === 'string' &&
    typeof value.canonicalResource === 'string' &&
    typeof value.createdAt === 'string'
  )
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
