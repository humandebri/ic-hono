// examples/hono-suite/src/auth.ts exercises jose on the runtime.
// Tokens are local status assertions, not user authentication credentials.

import { SignJWT, jwtVerify } from 'jose'

const secret = new TextEncoder().encode('ic-edge-suite-secret')

export async function statusToken(openIncidentCount: number, issuedAt: string): Promise<string> {
  return new SignJWT({ scope: 'status:read', openIncidentCount, issuedAt })
    .setProtectedHeader({ alg: 'HS256' })
    .sign(secret)
}

export async function verifyStatusToken(token: string): Promise<{ scope: string; openIncidentCount: number }> {
  const verified = await jwtVerify(token, secret)
  const scope = typeof verified.payload.scope === 'string' ? verified.payload.scope : ''
  const open = Number(verified.payload.openIncidentCount || 0)
  return { scope, openIncidentCount: Number.isFinite(open) ? open : 0 }
}
