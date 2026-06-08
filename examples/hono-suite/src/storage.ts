// examples/hono-suite/src/storage.ts stores suite state in Cache API.
// Cache API acts as canister-local durable app storage in this example.

import type { AuditEvent, Incident } from './types'
import { isAuditEvent, isIncident, isRecord } from './validation'

const INCIDENTS_KEY = 'https://ic-edge.local/suite/incidents'
const AUDIT_KEY = 'https://ic-edge.local/suite/audit'
export const MAX_INCIDENTS = 50
const MAX_AUDIT_EVENTS = 100

export async function readIncidents(): Promise<Incident[]> {
  return readList(INCIDENTS_KEY, 'incidents', isIncident, MAX_INCIDENTS)
}

export async function writeIncidents(incidents: Incident[]): Promise<void> {
  await writeJson(INCIDENTS_KEY, { incidents: incidents.slice(0, MAX_INCIDENTS) })
}

export async function readAudit(): Promise<AuditEvent[]> {
  return readList(AUDIT_KEY, 'events', isAuditEvent, MAX_AUDIT_EVENTS)
}

export async function appendAudit(event: AuditEvent): Promise<void> {
  await writeJson(AUDIT_KEY, { events: [event, ...(await readAudit())].slice(0, MAX_AUDIT_EVENTS) })
}

async function readList<T>(
  key: string,
  field: string,
  guard: (value: unknown) => value is T,
  limit: number,
): Promise<T[]> {
  const response = await caches.default.match(key)
  if (!response) return []

  const parsed: unknown = JSON.parse(await response.text())
  if (!isRecord(parsed) || !Array.isArray(parsed[field])) return []

  return parsed[field].filter(guard).slice(0, limit)
}

async function writeJson(key: string, value: unknown): Promise<void> {
  await caches.default.put(
    key,
    new Response(JSON.stringify(value), {
      headers: { 'content-type': 'application/json', 'cache-control': 'no-store' },
    }),
  )
}
