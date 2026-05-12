import { Hono } from 'hono'
import OpenAI from 'openai'

const client = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY,
  dangerouslyAllowBrowser: true,
})

const app = new Hono()

app.post('/respond', async (c) => {
  const response = await client.responses.create({
    model: process.env.OPENAI_MODEL || 'gpt-5.2',
    input: await c.req.text(),
  })

  return c.json({
    id: response.id,
    text: response.output_text,
  })
})

export default app
