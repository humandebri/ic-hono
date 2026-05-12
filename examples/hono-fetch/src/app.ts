import { Hono } from 'hono'

const app = new Hono()

app.get('/github', async (c) => {
  const response = await fetch('https://api.github.com')
  return c.json(await response.json())
})

export default app

