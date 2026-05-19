// examples/hono-x402-paid-api/src/product.ts defines paid API products.
// Route-local price and payee config keeps payment requirements stable per endpoint.

const DEFAULT_PAY_TO = '0x0000000000000000000000000000000000000402'

export type Product = {
  id: string
  method: 'GET'
  endpoint: string
  price: string
  payTo: string
  description: string
  canonicalResource: string
}

export function products(): Product[] {
  return [
    product('report', '/paid/report', 'X402_REPORT', '$0.001', 'Paid canister settlement report'),
    product('outcall', '/paid/outcall', 'X402_OUTCALL', '$0.003', 'Paid replicated HTTPS outcall check'),
  ]
}

export function productForEndpoint(endpoint: string): Product | null {
  return products().find((item) => item.endpoint === endpoint) || null
}

export function canonicalResourceBase(): string {
  return `https://${canisterId()}.icp0.io`
}

export function canonicalResource(endpoint: string): string {
  return `${canonicalResourceBase()}${endpoint}`
}

export function canisterId(): string {
  return globalThis.ic.canisterId()
}

function product(id: string, endpoint: string, envPrefix: string, defaultPrice: string, description: string): Product {
  return {
    id,
    method: 'GET',
    endpoint,
    price: routeValue(`${envPrefix}_PRICE`, defaultPrice),
    payTo: routeValue(`${envPrefix}_PAY_TO`, process.env.X402_PAY_TO || DEFAULT_PAY_TO),
    description,
    canonicalResource: canonicalResource(endpoint),
  }
}

function routeValue(name: string, fallback: string): string {
  return process.env[name] || fallback
}
