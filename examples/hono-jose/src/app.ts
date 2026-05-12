import { Hono } from 'hono'
import { SignJWT, jwtVerify } from 'jose'

const secret = new TextEncoder().encode('test-secret')
const app = new Hono()

app.onError((error, c) => c.text(error.message, 500))

app.get('/jwt', async (c) => {
  const token = await new SignJWT({ sub: 'edge' })
    .setProtectedHeader({ alg: 'HS256' })
    .sign(secret)
  const verified = await jwtVerify(token, secret)

  return c.json({
    sub: verified.payload.sub,
    token,
  })
})

export default app
