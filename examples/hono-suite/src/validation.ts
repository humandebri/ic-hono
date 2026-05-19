// examples/hono-suite/src/validation.ts validates API input with zod.
// All writes pass through these schemas before hitting Cache API state.

import { z } from 'zod'
import type { AuditEvent, Incident, IncidentInput, IncidentStatus, Severity } from './types'

const statusSchema = z.union([
  z.literal('investigating'),
  z.literal('identified'),
  z.literal('monitoring'),
  z.literal('resolved'),
])

const severitySchema = z.union([z.literal('minor'), z.literal('major'), z.literal('critical')])

const incidentInputSchema = z.object({
  title: z.string().trim().min(4).max(120),
  summary: z.string().trim().max(500).default(''),
  status: statusSchema,
  severity: severitySchema,
})

const checkUrlSchema = z.string().url().refine((value) => {
  const url = new URL(value)
  return url.protocol === 'https:' && !value.includes('@') && !isBlockedHost(url.hostname)
}, 'url must be public https without credentials')

export async function readJson(request: Request): Promise<unknown> {
  try {
    return await request.json()
  } catch {
    return null
  }
}

export function parseIncidentInput(value: unknown): { ok: true; value: IncidentInput } | { ok: false; error: string } {
  const result = incidentInputSchema.safeParse(value)
  if (!result.success) return { ok: false, error: 'invalid incident' }
  const data = result.data
  if (!isIncidentStatus(data.status) || !isSeverity(data.severity)) return { ok: false, error: 'invalid incident' }
  return {
    ok: true,
    value: {
      title: data.title || '',
      summary: data.summary || '',
      status: data.status,
      severity: data.severity,
    },
  }
}

export function parseCheckUrl(value: string | undefined): { ok: true; url: string } | { ok: false; error: string } {
  const result = checkUrlSchema.safeParse(value)
  if (!result.success) return { ok: false, error: 'invalid check url' }
  return { ok: true, url: new URL(result.data).toString() }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isIncident(value: unknown): value is Incident {
  if (!isRecord(value)) return false
  return (
    typeof value.id === 'string' &&
    incidentInputSchema.safeParse(value).success &&
    typeof value.createdAt === 'string' &&
    typeof value.updatedAt === 'string'
  )
}

export function isAuditEvent(value: unknown): value is AuditEvent {
  if (!isRecord(value)) return false
  return (
    typeof value.id === 'string' &&
    typeof value.action === 'string' &&
    typeof value.detail === 'string' &&
    typeof value.createdAt === 'string'
  )
}

function isBlockedHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/\.$/, '')
  return host === 'localhost' || host === 'metadata' || host === 'metadata.google.internal' || host.startsWith('127.')
}

function isIncidentStatus(value: unknown): value is IncidentStatus {
  return value === 'investigating' || value === 'identified' || value === 'monitoring' || value === 'resolved'
}

function isSeverity(value: unknown): value is Severity {
  return value === 'minor' || value === 'major' || value === 'critical'
}
