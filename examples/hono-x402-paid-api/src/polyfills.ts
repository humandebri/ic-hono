// examples/hono-x402-paid-api/src/polyfills.ts fills small Web API gaps used by x402.
// x402 clones plain payment requirement objects; JSON cloning is enough for this example.

if (typeof globalThis.structuredClone !== 'function') {
  Object.defineProperty(globalThis, 'structuredClone', {
    value: <T>(value: T): T => JSON.parse(JSON.stringify(value)),
  })
}
