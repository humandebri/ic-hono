// examples/hono-x402-paid-api/src/crypto.ts creates stable digests.
// Digests bind receipts to the exact result returned after settlement.

export async function sha256Hex(value: string): Promise<string> {
  const bytes = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)))
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

export function stableJson(value: unknown): string {
  return JSON.stringify(sortJson(value))
}

function sortJson(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortJson)
  if (!isRecord(value)) return value

  const sorted: Record<string, unknown> = {}
  for (const key of Object.keys(value).sort()) sorted[key] = sortJson(value[key])
  return sorted
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
