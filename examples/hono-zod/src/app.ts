import { Hono } from 'hono'
import { z } from 'zod'

const payloadSchema = z.object({
  name: z.string().min(1),
  count: z.number().int().nonnegative(),
})

const decimal = (value: number) => {
  if (value === 0) return '0'
  let remaining = value
  let output = ''
  while (remaining > 0) {
    const digit = remaining % 10
    output = '0123456789'[digit] + output
    remaining = (remaining - digit) / 10
  }
  return output
}

const app = new Hono()

app.post('/validate', async (c) => {
  const result = payloadSchema.safeParse(await c.req.json())

  if (!result.success) {
    return c.json({ ok: false }, 400)
  }

  return c.json({
    ok: true,
    greeting: `hello ${result.data.name}`,
    count: decimal(result.data.count),
  })
})

export default app
