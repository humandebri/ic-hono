import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { TrieRouter } from 'hono/router/trie-router'

declare global {
  var ic: { time: () => bigint }
}

const app = new Hono({ router: new TrieRouter() })

app.use('*', cors())

app.get('/', (c) => c.text('ok'))

app.post('/echo', async (c) => {
  return c.json(await c.req.json())
})

app.get('/users/:id', (c) => {
  return c.json({
    id: c.req.param('id'),
    q: c.req.query('q'),
  })
})

app.get('/number', (c) => {
  return c.json({ count: 1 })
})

app.get('/bytes', () => {
  return new Response(new Uint8Array([105, 99]))
})

app.post('/body-bytes', async (c) => {
  const bytes = new Uint8Array(await c.req.arrayBuffer())
  return c.json({ first: bytes[0], length: bytes.length })
})

app.get('/cache-put', async () => {
  await caches.default.put('https://ic-edge.local/cache-key', new Response('cached'))
  return new Response('stored')
})

app.get('/cache-get', async () => {
  const response = await caches.default.match('https://ic-edge.local/cache-key')
  return new Response(response ? await response.text() : 'missing')
})

app.get('/cache-roundtrip', async () => {
  await caches.default.put('https://ic-edge.local/cache-roundtrip-key', new Response('cached'))
  const response = await caches.default.match('https://ic-edge.local/cache-roundtrip-key')
  return new Response(response ? await response.text() : 'missing')
})

app.get('/cache-expired', async () => {
  const key = 'https://ic-edge.local/cache-expired-key'
  await caches.default.put(key, new Response('expired', {
    headers: { 'cache-control': 'max-age=0' },
  }))
  const response = await caches.default.match(key)
  return new Response(response ? await response.text() : 'missing')
})

app.get('/time', () => new Response(globalThis.ic.time().toString()))

export default app
