// examples/hono-suite/src/app.ts wires the full practical Hono suite.
// It combines middleware, validation, JWT, crypto, Cache API, and fetch.

import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { secureHeaders } from 'hono/secure-headers'
import { TrieRouter } from 'hono/router/trie-router'
import { statusToken, verifyStatusToken } from './auth'
import { sha256Hex } from './crypto'
import { renderPage } from './page'
import { report } from './report'
import { MAX_INCIDENTS, appendAudit, readAudit, readIncidents, writeIncidents } from './storage'
import type { AuditEvent, CheckResult, Incident, IncidentStatus } from './types'
import { parseCheckUrl, parseIncidentInput, readJson } from './validation'

const app = new Hono({ router: new TrieRouter() })

app.use('*', secureHeaders())
app.use('/api/*', cors())

app.get('/favicon.ico', (c) => c.body(null, 204))

app.get('/', async (c) => {
  const incidents = await readIncidents()
  const audit = await readAudit()
  const digest = await sha256Hex(JSON.stringify(report(incidents)))
  return c.html(renderPage(incidents, audit, digest, now()))
})

app.get('/api/health', async (c) => {
  const incidents = await readIncidents()
  const summary = report(incidents)
  const token = await statusToken(summary.open, now())
  const verified = await verifyStatusToken(token)
  return c.json({ ok: true, runtime: 'ic-edge', now: now(), ...summary, tokenScope: verified.scope })
})

app.get('/api/incidents', async (c) => c.json({ incidents: await readIncidents() }))
app.get('/api/audit', async (c) => c.json({ events: await readAudit() }))

app.post('/api/incidents', async (c) => {
  const input = parseIncidentInput(await readJson(c.req.raw))
  if (input.ok === false) return c.json({ error: input.error }, 400)

  const timestamp = now()
  const incident: Incident = { id: `inc-${timestamp}`, ...input.value, createdAt: timestamp, updatedAt: timestamp }
  await writeIncidents([incident, ...(await readIncidents())].slice(0, MAX_INCIDENTS))
  await appendAudit(event('incident.create', incident.title, timestamp))
  return c.json({ incident }, 201)
})

app.post('/api/incidents/:id/resolve', async (c) => {
  const id = c.req.param('id')
  const timestamp = now()
  const status: IncidentStatus = 'resolved'
  let found: Incident | null = null
  const incidents = (await readIncidents()).map((incident) => {
    if (incident.id !== id) return incident
    found = { ...incident, status, updatedAt: timestamp }
    return found
  })

  if (!found) return c.json({ error: 'incident not found' }, 404)

  await writeIncidents(incidents)
  await appendAudit(event('incident.resolve', id, timestamp))
  return c.json({ incident: found })
})

app.get('/api/check', async (c) => {
  const validation = parseCheckUrl(c.req.query('url'))
  if (validation.ok === false) return c.json({ error: validation.error }, 400)

  const response = await fetch(validation.url, { method: 'GET', ic: { replicated: true } })
  const result: CheckResult = {
    url: validation.url,
    ok: response.status >= 200 && response.status < 400,
    status: response.status,
    contentType: response.headers.get('content-type'),
    checkedAt: now(),
  }
  await appendAudit(event('check.run', validation.url, result.checkedAt))
  return c.json(result)
})

app.get('/api/report', async (c) => {
  const summary = report(await readIncidents())
  return c.json({ ...summary, digest: await sha256Hex(JSON.stringify(summary)) })
})

app.get('/api/session', async (c) => {
  const token = await statusToken(report(await readIncidents()).open, now())
  return c.json({ token, verified: await verifyStatusToken(token) })
})

app.get('/api/crypto', async (c) => {
  const value = c.req.query('value') || 'ic-edge'
  return c.json({ value, sha256: await sha256Hex(value) })
})

app.get('/demo', async (c) => {
  const timestamp = now()
  const incident = demoIncident(timestamp)
  await writeIncidents([incident])
  await appendAudit(event('demo.seed', incident.title, timestamp))
  return c.json({ health: (await readIncidents()).length === 1 ? 'ok' : 'failed', incident })
})

function demoIncident(timestamp: string): Incident {
  return {
    id: `demo-${timestamp}`,
    title: 'API latency above target',
    summary: 'Synthetic transaction checks crossed the warning threshold.',
    status: 'monitoring',
    severity: 'major',
    createdAt: timestamp,
    updatedAt: timestamp,
  }
}

function event(action: string, detail: string, createdAt: string): AuditEvent {
  return { id: `evt-${createdAt}`, action, detail, createdAt }
}

function now(): string {
  return globalThis.ic.time().toString()
}

export default app
