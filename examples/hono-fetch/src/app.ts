import { Hono } from 'hono'

declare global {
  interface RequestInit {
    ic?: { replicated?: boolean }
  }

  interface Request {
    ic?: { replicated: boolean }
  }
}

const app = new Hono()

app.get('/github', async (c) => {
  const response = await fetch('https://api.github.com')
  return c.json(await response.json())
})

app.get('/example-replicated', async (c) => {
  const response = await fetch('https://example.com', {
    ic: { replicated: true },
  })
  return c.json({ status: response.status })
})

export default app
