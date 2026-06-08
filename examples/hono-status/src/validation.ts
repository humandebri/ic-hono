// examples/hono-status/src/validation.ts validates public API input.
// It avoids trusting JSON shape before storing or using request data.

import type { Incident, IncidentInput, IncidentStatus, Severity } from './types'

export async function readJson(request: Request): Promise<unknown> {
  try {
    return await request.json()
  } catch {
    return null
  }
}

export function parseIncidentInput(
  value: unknown,
): { ok: true; value: IncidentInput } | { ok: false; error: string } {
  if (!isRecord(value)) return { ok: false, error: 'expected JSON object' }

  const title = stringField(value, 'title')
  const summary = stringField(value, 'summary') || ''
  const status = stringField(value, 'status')
  const severity = stringField(value, 'severity')

  if (title.length < 4 || title.length > 120) return { ok: false, error: 'invalid title' }
  if (summary.length > 500) return { ok: false, error: 'invalid summary' }
  if (!isIncidentStatus(status)) return { ok: false, error: 'invalid status' }
  if (!isSeverity(severity)) return { ok: false, error: 'invalid severity' }

  return { ok: true, value: { title, summary, status, severity } }
}

export function validateCheckUrl(value: string | undefined): { ok: true; url: string } | { ok: false; error: string } {
  if (!value) return { ok: false, error: 'missing url' }

  try {
    const url = new URL(value)
    if (url.protocol !== 'https:') return { ok: false, error: 'url must be https' }
    if (url.username || url.password) return { ok: false, error: 'url credentials are not allowed' }
    if (isBlockedHost(url.hostname)) return { ok: false, error: 'host is not public' }
    return { ok: true, url: url.toString() }
  } catch {
    return { ok: false, error: 'invalid url' }
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isIncident(value: unknown): value is Incident {
  if (!isRecord(value)) return false
  return (
    typeof value.id === 'string' &&
    typeof value.title === 'string' &&
    typeof value.summary === 'string' &&
    isIncidentStatus(value.status) &&
    isSeverity(value.severity) &&
    typeof value.createdAt === 'string' &&
    typeof value.updatedAt === 'string'
  )
}

function stringField(record: Record<string, unknown>, key: string): string {
  const value = record[key]
  return typeof value === 'string' ? value.trim() : ''
}

function isIncidentStatus(value: unknown): value is IncidentStatus {
  return value === 'investigating' || value === 'identified' || value === 'monitoring' || value === 'resolved'
}

function isSeverity(value: unknown): value is Severity {
  return value === 'minor' || value === 'major' || value === 'critical'
}

function isBlockedHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/\.$/, '')
  return host === 'localhost' || host === 'metadata' || host === 'metadata.google.internal' || host.startsWith('127.')
}
