#!/usr/bin/env node
// scripts/measure_x402_cycles.mjs measures x402 paid API local cycle deltas.
// It uses canister balance changes, corrected for elapsed idle burn.

import { execFileSync } from 'node:child_process'

const CANISTER = 'edge'
const ENVIRONMENT = 'local'
const BASE_URL = 'http://edge.local.localhost:8000'
const ENTRY = 'examples/hono-x402-paid-api/src/app.ts'
const BUNDLE = 'examples/hono-x402-paid-api/dist/app.bundle.js'
const BYTECODE = 'examples/hono-x402-paid-api/dist/app.qjbc'
const REPEATS = Number(process.env.IC_EDGE_X402_CYCLE_REPEATS || '3')
const USD_PER_T_CYCLE = 1.33

const rows = []

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', options.stderr || 'pipe'],
  })
}

function status() {
  return run('icp', ['canister', 'status', CANISTER, '--environment', ENVIRONMENT])
}

function cycleState() {
  const text = status()
  return {
    cycles: numberLine(text, /^\s*Cycles:\s*([0-9_]+)/m),
    idlePerDay: numberLine(text, /^\s*Idle cycles burned per day:\s*([0-9_]+)/m),
  }
}

function numberLine(text, pattern) {
  const match = text.match(pattern)
  if (!match) throw new Error(`missing status field: ${pattern}`)
  return BigInt(match[1].replaceAll('_', ''))
}

async function getJson(path, init = {}) {
  const response = await fetch(`${BASE_URL}${path}`, init)
  const body = await response.json()
  return { status: response.status, headers: response.headers, body }
}

async function demoSignature(endpoint) {
  const path = `/demo/payment-signature?endpoint=${encodeURIComponent(endpoint)}`
  const response = await getJson(path)
  if (response.status !== 200 || typeof response.body.value !== 'string') {
    throw new Error(`failed to get demo signature: ${JSON.stringify(response)}`)
  }
  return response.body.value
}

function paidHeaders(signature) {
  return { headers: { 'PAYMENT-SIGNATURE': signature } }
}

async function prepare() {
  try {
    run('icp', ['network', 'status', '--environment', ENVIRONMENT])
  } catch {
    run('icp', ['network', 'start', '-d'], { stderr: 'inherit' })
  }
  run('icp', ['deploy', CANISTER, '--environment', ENVIRONMENT, '--mode', 'reinstall', '--yes'], {
    stderr: 'inherit',
  })
  run('cargo', ['run', '-q', '-p', 'ic-edge-pack', '--bin', 'ic-edge', '--', 'pack', ENTRY, '--out', BUNDLE], {
    stderr: 'inherit',
  })
  run(
    'cargo',
    [
      'run',
      '-q',
      '-p',
      'ic-edge-pack',
      '--bin',
      'ic-edge',
      '--',
      'upload',
      BYTECODE,
      '--canister',
      CANISTER,
      '--environment',
      ENVIRONMENT,
    ],
    { stderr: 'inherit' },
  )
}

async function measure(name, setup, action, validate) {
  const samples = []
  for (let i = 0; i < REPEATS; i += 1) {
    const state = await setup?.(i)
    const before = cycleState()
    const start = Date.now()
    const output = await action(state, i)
    const end = Date.now()
    const after = cycleState()
    validate(output)
    const rawDelta = before.cycles - after.cycles
    const idleCorrection = (before.idlePerDay * BigInt(Math.max(0, end - start))) / 86_400_000n
    samples.push(rawDelta - idleCorrection)
  }
  const sorted = [...samples].sort((a, b) => Number(a - b))
  rows.push({
    name,
    repeats: REPEATS,
    min: sorted[0],
    median: sorted[Math.floor(sorted.length / 2)],
    avg: samples.reduce((sum, value) => sum + value, 0n) / BigInt(samples.length),
    max: sorted[sorted.length - 1],
    samples,
  })
}

function expectStatus(expected) {
  return (output) => {
    if (output.status !== expected) throw new Error(`expected ${expected}, got ${output.status}`)
  }
}

function expectPaid(output) {
  expectStatus(200)(output)
  if (!output.headers.get('payment-response')) throw new Error('missing PAYMENT-RESPONSE')
  if (!output.body.receipt?.payerHash) throw new Error('missing payerHash')
  if (JSON.stringify(output.body).includes('demo-payer')) throw new Error('raw payer leaked')
}

function usd(value) {
  return `$${((Number(value) / 1_000_000_000_000) * USD_PER_T_CYCLE).toFixed(6)}`
}

function markdown() {
  const lines = [
    '# Hono x402 Paid API Local Cycle Measurements',
    '',
    `- canister: ${CANISTER}`,
    `- environment: ${ENVIRONMENT}`,
    `- app entry: ${ENTRY}`,
    `- bundle: ${BUNDLE}`,
    `- bytecode: ${BYTECODE}`,
    `- repeats: ${REPEATS}`,
    '- delta: `before_cycles - after_cycles - idle_burn_elapsed`',
    '- scope: local PocketIC/icp-cli environment; not mainnet pricing',
    '- pricing: `1T cycles = $1.33`',
    '',
    '| operation | repeats | median cycles | median USD | avg cycles | avg USD | min | max | samples |',
    '|---|---:|---:|---:|---:|---:|---:|---:|---|',
  ]
  for (const row of rows) {
    lines.push(
      `| ${row.name} | ${row.repeats} | ${row.median} | ${usd(row.median)} | ${row.avg} | ${usd(row.avg)} | ${row.min} | ${row.max} | ${row.samples.join(', ')} |`,
    )
  }
  return `${lines.join('\n')}\n`
}

await prepare()

await measure('GET /free/catalog', null, () => getJson('/free/catalog'), expectStatus(200))
await measure('GET /paid/report unpaid', null, () => getJson('/paid/report'), expectStatus(402))
await measure(
  'GET /paid/report paid',
  () => demoSignature('/paid/report'),
  (signature) => getJson('/paid/report', paidHeaders(signature)),
  expectPaid,
)
await measure(
  'GET /paid/report replay rejected',
  async () => {
    const signature = await demoSignature('/paid/report')
    await getJson('/paid/report', paidHeaders(signature))
    return signature
  },
  (signature) => getJson('/paid/report', paidHeaders(signature)),
  expectStatus(409),
)
await measure('GET /receipts', null, () => getJson('/receipts'), expectStatus(200))
await measure('GET /audit/root', null, () => getJson('/audit/root'), expectStatus(200))
await measure(
  'GET /paid/outcall paid replicated',
  () => demoSignature('/paid/outcall'),
  (signature) => getJson('/paid/outcall?url=https%3A%2F%2Fexample.com%2F', paidHeaders(signature)),
  expectPaid,
)

console.log(markdown())
