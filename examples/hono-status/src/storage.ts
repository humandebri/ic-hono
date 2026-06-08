// examples/hono-status/src/storage.ts persists status data in Cache API storage.
// Cache gives the example durable canister-local state without external services.

import type { Incident } from './types'
import { isIncident, isRecord } from './validation'

const INCIDENTS_KEY = 'https://ic-edge.local/status/incidents'
export const MAX_INCIDENTS = 20

export async function readIncidents(): Promise<Incident[]> {
  const response = await caches.default.match(INCIDENTS_KEY)
  if (!response) return []

  const parsed: unknown = JSON.parse(await response.text())
  if (!isRecord(parsed) || !Array.isArray(parsed.incidents)) return []

  return parsed.incidents.filter(isIncident).slice(0, MAX_INCIDENTS)
}

export async function writeIncidents(incidents: Incident[]): Promise<void> {
  await caches.default.put(
    INCIDENTS_KEY,
    new Response(JSON.stringify({ incidents }), {
      headers: { 'content-type': 'application/json', 'cache-control': 'no-store' },
    }),
  )
}
