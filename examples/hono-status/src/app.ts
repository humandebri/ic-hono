// examples/hono-status/src/app.ts wires Hono routes for the status site.
// Business state stays in Cache API, while checks use replicated fetch outcalls.

import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { secureHeaders } from 'hono/secure-headers'
import { TrieRouter } from 'hono/router/trie-router'
import { renderPage } from './page'
import { MAX_INCIDENTS, readIncidents, writeIncidents } from './storage'
import type { Incident, IncidentStatus } from './types'
import { parseIncidentInput, readJson, validateCheckUrl } from './validation'

const app = new Hono({ router: new TrieRouter() })

app.use('*', secureHeaders())
app.use('/api/*', cors())

app.get('/', async (c) => {
  return c.html(renderPage(await readIncidents(), now()))
})

app.get('/api/health', async (c) => {
  const incidents = await readIncidents()
  return c.json({
    ok: true,
    runtime: 'ic-edge',
    now: now(),
    incidentCount: incidents.length,
    openIncidentCount: incidents.filter((incident) => incident.status !== 'resolved').length,
  })
})

app.get('/api/incidents', async (c) => {
  return c.json({ incidents: await readIncidents() })
})

app.post('/api/incidents', async (c) => {
  const input = parseIncidentInput(await readJson(c.req.raw))
  if (input.ok === false) return c.json({ error: input.error }, 400)

  const timestamp = now()
  const incident: Incident = {
    id: `inc-${timestamp}`,
    ...input.value,
    createdAt: timestamp,
    updatedAt: timestamp,
  }
  const incidents = [incident, ...(await readIncidents())].slice(0, MAX_INCIDENTS)
  await writeIncidents(incidents)

  return c.json({ incident }, 201)
})

app.post('/api/incidents/:id/resolve', async (c) => {
  const id = c.req.param('id')
  const incidents = await readIncidents()
  const timestamp = now()
  const status: IncidentStatus = 'resolved'
  let found = false
  const updated = incidents.map((incident) => {
    if (incident.id !== id) return incident
    found = true
    return { ...incident, status, updatedAt: timestamp }
  })

  if (!found) return c.json({ error: 'incident not found' }, 404)

  await writeIncidents(updated)
  return c.json({ incident: updated.find((incident) => incident.id === id) })
})

app.get('/api/check', async (c) => {
  const validation = validateCheckUrl(c.req.query('url'))
  if (validation.ok === false) return c.json({ error: validation.error }, 400)

  const response = await fetch(validation.url, {
    method: 'GET',
    ic: { replicated: true },
  })
  return c.json({
    url: validation.url,
    ok: response.status >= 200 && response.status < 400,
    status: response.status,
    contentType: response.headers.get('content-type'),
    checkedAt: now(),
  })
})

app.get('/demo', async (c) => {
  const incident = demoIncident(now())
  await writeIncidents([incident])
  return c.json({
    health: (await readIncidents()).length === 1 ? 'ok' : 'failed',
    incident,
  })
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

function now(): string {
  return globalThis.ic.time().toString()
}

export default app
