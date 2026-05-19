// examples/hono-suite/src/types.ts defines app contracts and ic-edge globals.
// The suite intentionally exercises multiple supported Worker-style APIs.

declare global {
  var ic: { time: () => bigint }

  interface RequestInit {
    ic?: { replicated?: boolean }
  }

  interface Request {
    ic?: { replicated: boolean }
  }

  interface CacheStorage {
    default: Cache
  }
}

export type IncidentStatus = 'investigating' | 'identified' | 'monitoring' | 'resolved'
export type Severity = 'minor' | 'major' | 'critical'

export type IncidentInput = {
  title: string
  summary: string
  status: IncidentStatus
  severity: Severity
}

export type Incident = IncidentInput & {
  id: string
  createdAt: string
  updatedAt: string
}

export type AuditEvent = {
  id: string
  action: string
  detail: string
  createdAt: string
}

export type CheckResult = {
  url: string
  ok: boolean
  status: number
  contentType: string | null
  checkedAt: string
}
