// examples/hono-x402-paid-api/src/x402.ts adapts the official x402 V2 custom server pattern.
// A mock facilitator keeps local canister smoke deterministic; env can switch to a real facilitator.

import { HTTPFacilitatorClient, x402ResourceServer } from '@x402/core/server'
import type { FacilitatorClient } from '@x402/core/server'
import type {
  PaymentPayload,
  PaymentRequirements,
  SettleResponse,
  SupportedResponse,
  VerifyResponse,
} from '@x402/core/types'
import { decodePaymentSignatureHeader, encodePaymentRequiredHeader, encodePaymentResponseHeader } from '@x402/core/http'
import { ExactEvmScheme } from '@x402/evm/exact/server'
import type { Context } from 'hono'
import { sha256Hex, stableJson } from './crypto'
import { canisterId, canonicalResourceBase, productForEndpoint, products } from './product'
import type { Product } from './product'
import { commitReceipt, failReceipt, findReceipt, reserveReceipt } from './storage'
import type { PaidResult, Receipt } from './types'

const NETWORK = 'eip155:84532'
const SCHEME = 'exact'
const DEFAULT_TOKEN = 'paid-demo'

type Runtime = {
  mode: 'mock' | 'http'
  server: x402ResourceServer
  requirements: Map<string, Promise<PaymentRequirements[]>>
}

let runtimePromise: Promise<Runtime> | null = null

export async function x402Catalog() {
  const runtime = await getRuntime()
  return {
    mode: runtime.mode,
    network: NETWORK,
    scheme: SCHEME,
    canisterId: canisterId(),
    canonicalResourceBase: canonicalResourceBase(),
    products: products(),
    headers: ['PAYMENT-REQUIRED', 'PAYMENT-SIGNATURE', 'PAYMENT-RESPONSE'],
  }
}

export async function demoPaymentSignature(endpoint = '/paid/report'): Promise<string | null> {
  const runtime = await getRuntime()
  if (runtime.mode !== 'mock') return null
  const product = productForEndpoint(endpoint)
  if (!product) return null

  return encodeDemoPayment({
    x402Version: 2,
    accepted: (await getRequirements(runtime, product))[0],
    payload: { demoToken: demoToken(), nonce: now() },
  })
}

export async function requirePayment(
  c: Context,
  endpoint: string,
  buildResult: () => Promise<Record<string, unknown>>,
): Promise<Response> {
  const runtime = await getRuntime()
  const product = productForEndpoint(endpoint)
  if (!product) return c.json({ error: 'unknown paid product', endpoint }, 404)
  const productRequirements = await getRequirements(runtime, product)
  const signature = c.req.header('PAYMENT-SIGNATURE')

  if (!signature) return paymentRequired(c, runtime, product, productRequirements)

  let payload: PaymentPayload
  try {
    payload = decodePaymentSignatureHeader(signature)
  } catch {
    return paymentRequired(c, runtime, product, productRequirements, 'invalid payment signature')
  }

  const requirements = runtime.server.findMatchingRequirements(productRequirements, payload)
  if (!requirements) return paymentRequired(c, runtime, product, productRequirements, 'payment does not match requirements')

  const receiptId = await sha256Hex(stableJson({ endpoint, signature }))
  const paymentRequirementsHash = await sha256Hex(stableJson(requirements))
  const paymentSignatureHash = await sha256Hex(signature)
  try {
    reserveReceipt(receiptId, {
      productId: product.id,
      endpoint,
      method: c.req.method,
      price: product.price,
      payTo: product.payTo,
      paymentRequirementsHash,
      paymentSignatureHash,
      canisterId: canisterId(),
      canonicalResource: product.canonicalResource,
      createdAt: now(),
    })
  } catch {
    return c.json({ error: 'payment replay rejected', receiptId, receipt: await findReceipt(receiptId) }, 409)
  }

  const verified = await runtime.server.verifyPayment(payload, requirements, {}, { endpoint })
  if (!verified.isValid) {
    failAudit(receiptId, product, verified.invalidReason || 'payment rejected')
    return paymentRequired(c, runtime, product, productRequirements, verified.invalidReason || 'payment rejected', payload)
  }

  const result = await paidResult(endpoint, buildResult)
  const settled = await runtime.server.settlePayment(payload, requirements, {}, { endpoint, result })
  if (!settled.success) {
    failAudit(receiptId, product, settled.errorReason || 'settlement failed')
    return paymentRequired(c, runtime, product, productRequirements, settled.errorReason || 'settlement failed', payload)
  }

  const receipt = await receiptFromSettlement(
    receiptId,
    product,
    c.req.method,
    paymentRequirementsHash,
    paymentSignatureHash,
    result.digest,
    settled,
  )
  const event = commitReceipt(receiptId, receipt)
  c.header('PAYMENT-RESPONSE', encodePaymentResponseHeader(settled))
  c.header('Cache-Control', 'no-store')
  return c.json({ result, receipt, audit: { root: event.root, eventHash: event.event_hash } })
}

async function paymentRequired(
  c: Context,
  runtime: Runtime,
  product: Product,
  requirements: PaymentRequirements[],
  error?: string,
  payload?: PaymentPayload,
): Promise<Response> {
  const paymentRequired = await runtime.server.createPaymentRequiredResponse(
    requirements,
    {
      url: product.canonicalResource,
      description: product.description,
      mimeType: 'application/json',
      serviceName: 'ic-edge x402 paid API',
      tags: ['ic-edge', 'hono', 'x402'],
    },
    error,
    {},
    { endpoint: product.endpoint, productId: product.id },
    payload,
  )
  c.header('PAYMENT-REQUIRED', encodePaymentRequiredHeader(paymentRequired))
  c.header('Cache-Control', 'no-store')
  return c.json(paymentRequired, 402)
}

async function paidResult(endpoint: string, buildResult: () => Promise<Record<string, unknown>>): Promise<PaidResult> {
  const payload = await buildResult()
  const generatedAt = now()
  const digest = await sha256Hex(stableJson({ endpoint, generatedAt, payload }))
  return { endpoint, generatedAt, digest, payload }
}

async function receiptFromSettlement(
  id: string,
  product: Product,
  method: string,
  paymentRequirementsHash: string,
  paymentSignatureHash: string,
  resultDigest: string,
  settled: SettleResponse,
): Promise<Receipt> {
  return {
    id,
    productId: product.id,
    endpoint: product.endpoint,
    method,
    price: product.price,
    payTo: product.payTo,
    paymentRequirementsHash,
    paymentSignatureHash,
    payerHash: await sha256Hex(settled.payer || 'unknown'),
    transaction: settled.transaction,
    network: settled.network,
    amount: settled.amount || '0',
    resultDigest,
    canisterId: canisterId(),
    canonicalResource: product.canonicalResource,
    createdAt: now(),
  }
}

function failAudit(id: string, product: Product, reason: string): void {
  try {
    failReceipt(id, {
      productId: product.id,
      endpoint: product.endpoint,
      reason,
      failedAt: now(),
      canisterId: canisterId(),
    })
  } catch {
  }
}

async function getRuntime(): Promise<Runtime> {
  if (!runtimePromise) runtimePromise = createRuntime()
  return runtimePromise
}

async function createRuntime(): Promise<Runtime> {
  const facilitator = facilitatorUrl()
    ? new HTTPFacilitatorClient({ url: facilitatorUrl() })
    : new DemoFacilitator()
  const mode = facilitator instanceof DemoFacilitator ? 'mock' : 'http'
  const server = new x402ResourceServer(facilitator).register(NETWORK, new ExactEvmScheme())
  await server.initialize()
  return { mode, server, requirements: new Map() }
}

async function getRequirements(runtime: Runtime, product: Product): Promise<PaymentRequirements[]> {
  const cached = runtime.requirements.get(product.id)
  if (cached) return cached
  const next = runtime.server.buildPaymentRequirements({
    scheme: SCHEME,
    network: NETWORK,
    payTo: product.payTo,
    price: product.price,
    maxTimeoutSeconds: 60,
    extra: { binding: 'method:path:result-digest', settlement: runtime.mode, productId: product.id },
  })
  runtime.requirements.set(product.id, next)
  return next
}

function facilitatorUrl(): string {
  return process.env.X402_FACILITATOR_URL || ''
}

function demoToken(): string {
  return process.env.X402_DEMO_TOKEN || DEFAULT_TOKEN
}

function now(): string {
  return globalThis.ic.time().toString()
}

function encodeDemoPayment(payload: PaymentPayload): string {
  return btoa(JSON.stringify(payload))
}

class DemoFacilitator implements FacilitatorClient {
  async getSupported(): Promise<SupportedResponse> {
    return { kinds: [{ x402Version: 2, scheme: SCHEME, network: NETWORK }], extensions: [], signers: {} }
  }

  async verify(paymentPayload: PaymentPayload, paymentRequirements: PaymentRequirements): Promise<VerifyResponse> {
    const valid =
      paymentPayload.x402Version === 2 &&
      paymentPayload.accepted.network === paymentRequirements.network &&
      paymentPayload.accepted.scheme === paymentRequirements.scheme &&
      paymentPayload.payload.demoToken === demoToken()
    return valid ? { isValid: true, payer: 'demo-payer' } : { isValid: false, invalidReason: 'demo token rejected' }
  }

  async settle(paymentPayload: PaymentPayload, paymentRequirements: PaymentRequirements): Promise<SettleResponse> {
    return {
      success: true,
      transaction: await sha256Hex(stableJson(paymentPayload)),
      network: paymentRequirements.network,
      amount: paymentRequirements.amount,
      payer: 'demo-payer',
    }
  }
}
