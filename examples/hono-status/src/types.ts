// examples/hono-status/src/types.ts defines runtime and app-level contracts.
// It centralizes ambient declarations needed by ic-edge Web API extensions.

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

export type Incident = {
  id: string
  title: string
  summary: string
  status: IncidentStatus
  severity: Severity
  createdAt: string
  updatedAt: string
}

export type IncidentInput = {
  title: string
  summary: string
  status: IncidentStatus
  severity: Severity
}
