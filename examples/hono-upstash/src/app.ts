import { Redis } from '@upstash/redis'
import { Hono } from 'hono'
import { TrieRouter } from 'hono/router/trie-router'

const redis = new Redis({
  url: process.env.UPSTASH_REDIS_REST_URL,
  token: process.env.UPSTASH_REDIS_REST_TOKEN,
  enableTelemetry: false,
})

const app = new Hono({ router: new TrieRouter() })

app.get('/kv/:key', async (c) => {
  const value = await redis.get(c.req.param('key'))
  return c.json({ value })
})

export default app
