// examples/hono-x402-paid-api/src/types.ts defines ic-edge globals.
// The example stays inside Worker-style APIs supported by ic-edge.

declare global {
  var ic: {
    time: () => bigint
    canisterId: () => string
    audit: AuditApi
  }

  interface RequestInit {
    ic?: { replicated?: boolean }
  }
}

export type AuditApi = {
  reserve: (id: string, payloadJson: string) => AuditEvent
  commit: (id: string, payloadJson: string) => AuditEvent
  fail: (id: string, payloadJson: string) => AuditEvent
  get: (id: string) => AuditEvent | null
  list: (offset?: number, limit?: number) => AuditEvent[]
  root: () => AuditRoot
}

export type AuditRoot = {
  root: string
  count: number
}

export type AuditEvent = {
  index: number
  id: string
  kind: 'reserve' | 'commit' | 'fail'
  payloadJson: string
  prev_root: string
  event_hash: string
  root: string
}

export type PaidResult = {
  endpoint: string
  generatedAt: string
  digest: string
  payload: Record<string, unknown>
}

export type Receipt = {
  id: string
  productId: string
  endpoint: string
  method: string
  price: string
  payTo: string
  paymentRequirementsHash: string
  paymentSignatureHash: string
  payerHash: string
  transaction: string
  network: string
  amount: string
  resultDigest: string
  canisterId: string
  canonicalResource: string
  createdAt: string
}
