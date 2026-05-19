// examples/hono-x402-paid-api/src/validation.ts keeps outbound paid checks constrained.
// The paid outcall endpoint accepts public HTTPS URLs only.

export type CheckResult = {
  url: string
  ok: boolean
  status: number
  contentType: string | null
  checkedAt: string
}

export function parseCheckUrl(value: string | undefined): { ok: true; url: string } | { ok: false; error: string } {
  if (!value) return { ok: true, url: 'https://example.com/' }

  try {
    const url = new URL(value)
    if (url.protocol !== 'https:') return { ok: false, error: 'url must use https' }
    if (value.includes('@')) return { ok: false, error: 'url credentials are not allowed' }
    if (isBlockedHost(url.hostname)) return { ok: false, error: 'host is not allowed' }
    return { ok: true, url: url.toString() }
  } catch {
    return { ok: false, error: 'invalid url' }
  }
}

function isBlockedHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/\.$/, '')
  return host === 'localhost' || host === 'metadata' || host === 'metadata.google.internal' || host.startsWith('127.')
}
