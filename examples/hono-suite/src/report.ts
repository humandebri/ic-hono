// examples/hono-suite/src/report.ts computes operational summaries.
// It keeps derived data separate from route handlers and persisted records.

import type { Incident } from './types'

export function report(incidents: Incident[]) {
  return {
    total: incidents.length,
    open: incidents.filter((incident) => incident.status !== 'resolved').length,
    bySeverity: countBy(incidents, (incident) => incident.severity),
    byStatus: countBy(incidents, (incident) => incident.status),
  }
}

function countBy<T extends string>(incidents: Incident[], key: (incident: Incident) => T): Record<string, number> {
  const output: Record<string, number> = {}
  for (const incident of incidents) {
    const value = key(incident)
    output[value] = (output[value] || 0) + 1
  }
  return output
}
