#!/usr/bin/env node
// scripts/measure_local_cycles.mjs measures local canister cycle deltas.
// It reports observed balance decreases, corrected for idle burn elapsed time.

import { execFileSync } from 'node:child_process'

const CANISTER = 'edge'
const ENVIRONMENT = 'local'
const REPEATS = Number(process.env.IC_EDGE_CYCLE_REPEATS || '5')
const SHORT_REPEATS = Number(process.env.IC_EDGE_CYCLE_SHORT_REPEATS || '3')
const APP_ENTRY = process.env.IC_EDGE_MEASURE_ENTRY || 'examples/hono-status/src/app.ts'
const BUNDLE = process.env.IC_EDGE_MEASURE_BUNDLE || 'examples/hono-status/dist/app.bundle.js'
const BYTECODE = BUNDLE.replace(/\.bundle\.js$/, '.qjbc')

const rows = []

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', options.stderr || 'pipe'],
  })
}

function canisterCall(method, args) {
  return run('icp', ['canister', 'call', CANISTER, method, args, '--environment', ENVIRONMENT])
}

function status() {
  return run('icp', ['canister', 'status', CANISTER, '--environment', ENVIRONMENT])
}

function cycleState() {
  const text = status()
  const cycles = numberLine(text, /^\s*Cycles:\s*([0-9_]+)/m)
  const idlePerDay = numberLine(text, /^\s*Idle cycles burned per day:\s*([0-9_]+)/m)
  return { cycles, idlePerDay }
}

function numberLine(text, pattern) {
  const match = text.match(pattern)
  if (!match) throw new Error(`missing status field: ${pattern}`)
  return BigInt(match[1].replaceAll('_', ''))
}

function nowMs() {
  return Number(run('node', ['-e', 'console.log(Date.now())']).trim())
}

function blob(bytes) {
  return `blob "${Array.from(bytes, (byte) => `\\${byte.toString(16).padStart(2, '0')}`).join('')}"`
}

function httpArg(method, url, body = '', headers = 'vec {}') {
  const bytes = Buffer.from(body)
  return `(record { method = "${method}"; url = "${url}"; headers = ${headers}; body = ${blob(bytes)}; certificate_version = null })`
}

function jsonHeaders() {
  return 'vec { record { "content-type"; "application/json" } }'
}

function median(values) {
  const sorted = [...values].sort((a, b) => Number(a - b))
  return sorted[Math.floor(sorted.length / 2)]
}

function avg(values) {
  return values.reduce((sum, value) => sum + value, 0n) / BigInt(values.length)
}

function measure(name, repeats, setup, action, validate) {
  const samples = []
  for (let i = 0; i < repeats; i += 1) {
    setup?.(i)
    const before = cycleState()
    const start = nowMs()
    const output = action(i)
    const end = nowMs()
    const after = cycleState()
    validate?.(output)

    const rawDelta = before.cycles - after.cycles
    const elapsedMs = BigInt(Math.max(0, end - start))
    const idleCorrection = (before.idlePerDay * elapsedMs) / 86_400_000n
    samples.push(rawDelta - idleCorrection)
  }
  const sorted = [...samples].sort((a, b) => Number(a - b))
  rows.push({
    name,
    repeats,
    min: sorted[0],
    median: median(samples),
    avg: avg(samples),
    max: sorted[sorted.length - 1],
    samples,
  })
}

function expectContains(value, pattern, label) {
  if (!value.includes(pattern)) {
    throw new Error(`${label} missing ${pattern}\n${value}`)
  }
}

function postIncident(title = 'Measured incident') {
  const payload = JSON.stringify({
    title,
    summary: 'Cycle measurement payload',
    status: 'identified',
    severity: 'major',
  })
  return canisterCall('http_request_update', httpArg('POST', '/api/incidents', payload, jsonHeaders()))
}

function incidentId(output) {
  const match = output.match(/\\22id\\22:\\22([^\\]+)\\22/)
  if (!match) throw new Error(`failed to extract incident id\n${output}`)
  return match[1]
}

function manifestUpload() {
  return run('cargo', [
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
  ])
}

function prepare() {
  try {
    run('icp', ['network', 'status'])
  } catch {
    run('icp', ['network', 'start', '-d'], { stderr: 'inherit' })
  }
  run('icp', ['deploy', CANISTER, '--yes'], { stderr: 'inherit' })
  run('cargo', ['run', '-q', '-p', 'ic-edge-pack', '--bin', 'ic-edge', '--', 'pack', APP_ENTRY, '--out', BUNDLE], {
    stderr: 'inherit',
  })
  run('cargo', ['run', '-q', '-p', 'ic-edge-pack', '--bin', 'ic-edge', '--', 'upload', BYTECODE, '--canister', CANISTER, '--environment', ENVIRONMENT], {
    stderr: 'inherit',
  })
}

function renderMarkdown() {
  const lines = [
    '# Local Cycle Measurements',
    '',
    `- canister: ${CANISTER}`,
    `- environment: ${ENVIRONMENT}`,
    `- app entry: ${APP_ENTRY}`,
    `- bundle: ${BUNDLE}`,
    `- bytecode: ${BYTECODE}`,
    `- repeats: ${REPEATS}`,
    '- delta: `before_cycles - after_cycles - idle_burn_elapsed`',
    '- scope: local PocketIC/icp-cli environment; not mainnet pricing',
    '',
    '| operation | repeats | median cycles | avg cycles | min | max | samples |',
    '|---|---:|---:|---:|---:|---:|---|',
  ]
  for (const row of rows) {
    lines.push(
      `| ${row.name} | ${row.repeats} | ${row.median} | ${row.avg} | ${row.min} | ${row.max} | ${row.samples.join(', ')} |`,
    )
  }
  lines.push('')
  return `${lines.join('\n')}\n`
}

prepare()

measure(
  'abort_bytecode_upload_missing',
  REPEATS,
  null,
  (i) => canisterCall('abort_bytecode_upload', `("missing-${i}")`),
  (out) => expectContains(out, 'Ok', 'abort_bytecode_upload'),
)

measure(
  'set_env_replace_16b',
  REPEATS,
  () => canisterCall('set_env', '("MEASURE_REPLACE", "seed")'),
  (i) => canisterCall('set_env', `("MEASURE_REPLACE", "0123456789abcdef${i}")`),
  (out) => expectContains(out, 'Ok', 'set_env'),
)

measure(
  'http_get_health_cold_after_generation_change',
  SHORT_REPEATS,
  (i) => canisterCall('set_env', `("MEASURE_COLD", "generation-${process.pid}-${i}")`),
  () => canisterCall('http_request_update', httpArg('GET', '/api/health')),
  (out) => expectContains(out, 'status_code = 200', 'health cold'),
)

measure(
  'http_get_health_warm_update',
  REPEATS,
  () => canisterCall('http_request_update', httpArg('GET', '/api/health')),
  () => canisterCall('http_request_update', httpArg('GET', '/api/health')),
  (out) => expectContains(out, 'status_code = 200', 'health'),
)

measure(
  'http_get_incidents_update',
  REPEATS,
  () => canisterCall('http_request_update', httpArg('GET', '/demo')),
  () => canisterCall('http_request_update', httpArg('GET', '/api/incidents')),
  (out) => expectContains(out, 'status_code = 200', 'incidents'),
)

measure(
  'http_get_page_update',
  REPEATS,
  () => canisterCall('http_request_update', httpArg('GET', '/demo')),
  () => canisterCall('http_request_update', httpArg('GET', '/')),
  (out) => expectContains(out, 'status_code = 200', 'page'),
)

measure(
  'http_get_demo_write_cache',
  REPEATS,
  null,
  () => canisterCall('http_request_update', httpArg('GET', '/demo')),
  (out) => expectContains(out, 'status_code = 200', 'demo'),
)

measure(
  'http_post_incident_create',
  REPEATS,
  () => canisterCall('http_request_update', httpArg('GET', '/demo')),
  (i) => postIncident(`Measured incident ${i}`),
  (out) => expectContains(out, 'status_code = 201', 'create incident'),
)

measure(
  'http_post_incident_resolve',
  REPEATS,
  () => {},
  (i) => {
    canisterCall('http_request_update', httpArg('GET', '/demo'))
    const created = postIncident(`Resolvable incident ${i}`)
    const id = incidentId(created)
    return canisterCall('http_request_update', httpArg('POST', `/api/incidents/${id}/resolve`))
  },
  (out) => expectContains(out, 'status_code = 200', 'resolve incident'),
)

measure(
  'manifest_chunk_upload_app_bundle',
  SHORT_REPEATS,
  null,
  () => manifestUpload(),
  (out) => expectContains(out, 'uploaded', 'manifest upload'),
)

measure(
  'http_check_example_replicated_fetch_warm',
  SHORT_REPEATS,
  () => canisterCall('http_request_update', httpArg('GET', '/api/health')),
  () => canisterCall('http_request_update', httpArg('GET', '/api/check?url=https%3A%2F%2Fexample.com%2F')),
  (out) => expectContains(out, 'status_code = 200', 'check example'),
)

measure(
  'direct_fetch_outcall_non_replicated',
  SHORT_REPEATS,
  null,
  () => canisterCall('fetch_outcall', '("https://example.com/")'),
  (out) => expectContains(out, 'status_code = 200', 'fetch outcall'),
)

measure(
  'direct_fetch_outcall_replicated',
  SHORT_REPEATS,
  null,
  () => canisterCall('fetch_outcall_replicated', '("https://example.com/")'),
  (out) => expectContains(out, 'status_code = 200', 'fetch outcall replicated'),
)

console.log(renderMarkdown())
