// examples/hono-x402-paid-api/src/app.ts exposes a paid Hono API on ic-edge.
// It ports the official x402 V2 custom server pattern to canister-friendly Web APIs.

import './polyfills'
import { Hono } from 'hono'
import { auditRoot, findReceipt, readAuditEvents, readReceipts } from './storage'
import type { CheckResult } from './validation'
import { parseCheckUrl } from './validation'
import { demoPaymentSignature, requirePayment, x402Catalog } from './x402'

const app = new Hono()

app.get('/', async (c) => c.json({ service: 'ic-edge x402 paid API', catalog: await x402Catalog() }))
app.get('/free/catalog', async (c) => c.json(await x402Catalog()))
app.get('/receipts', async (c) => c.json({ receipts: await readReceipts() }))
app.get('/receipts/:id', async (c) => c.json({ receipt: await findReceipt(c.req.param('id')) }))
app.get('/audit/root', (c) => c.json(auditRoot()))
app.get('/audit/events', async (c) => c.json({ events: await readAuditEvents() }))

app.get('/demo/payment-signature', async (c) => {
  const signature = await demoPaymentSignature()
  if (!signature) return c.json({ error: 'demo signature is disabled when X402_FACILITATOR_URL is set' }, 409)
  return c.json({ header: 'PAYMENT-SIGNATURE', value: signature })
})

app.get('/paid/report', async (c) =>
  requirePayment(c, '/paid/report', async () => ({
    title: 'Canister x402 settlement report',
    summary: 'Paid result generated only after x402 verification and before settlement.',
    metrics: {
      receipts: (await readReceipts()).length,
      canisterTime: now(),
    },
  })),
)

app.get('/paid/outcall', async (c) =>
  requirePayment(c, '/paid/outcall', async () => {
    const parsed = parseCheckUrl(c.req.query('url'))
    if (parsed.ok === false) return { error: parsed.error }

    const response = await fetch(parsed.url, { method: 'GET', ic: { replicated: true } })
    const result: CheckResult = {
      url: parsed.url,
      ok: response.status >= 200 && response.status < 400,
      status: response.status,
      contentType: response.headers.get('content-type'),
      checkedAt: now(),
    }
    return { check: result }
  }),
)

app.get('/favicon.ico', (c) => c.body(null, 204))

function now(): string {
  return globalThis.ic.time().toString()
}

export default app
