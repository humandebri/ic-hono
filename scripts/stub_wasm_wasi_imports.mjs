#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const [input, output = input] = process.argv.slice(2)

if (!input) {
  console.error('usage: stub_wasm_wasi_imports.mjs <input.wasm> [output.wasm]')
  process.exit(2)
}

const wat = execFileSync('wasm-tools', ['print', input], {
  encoding: 'utf8',
  maxBuffer: 128 * 1024 * 1024,
})
const lines = wat.split('\n')
const imports = []
const symbols = new Map()
const kept = []

for (const line of lines) {
  const match = line.match(
    /^  \(import "wasi_snapshot_preview1" "([^"]+)" \(func ([^ ]+) \(;\d+;\) \(type \d+\)\)\)$/,
  )
  if (match) {
    imports.push(match[1])
    symbols.set(match[1], match[2])
    continue
  }
  kept.push(line)
}

if (imports.length === 0) {
  if (input !== output) {
    writeFileSync(output, readFileSync(input))
  }
  process.exit(0)
}

const lastImport = kept.reduce((last, line, index) => (line.startsWith('  (import ') ? index : last), -1)
if (lastImport === -1) {
  throw new Error('wasm module has no import section')
}

const stubs = `
  (func ${symbols.get('random_get')} (param i32 i32) (result i32)
    (local i32)
    local.get 0
    local.set 2
    loop
      local.get 1
      i32.eqz
      if
        i32.const 0
        return
      end
      local.get 2
      i32.const 0
      i32.store8
      local.get 2
      i32.const 1
      i32.add
      local.set 2
      local.get 1
      i32.const 1
      i32.sub
      local.set 1
      br 0
    end
    i32.const 0)
  (func ${symbols.get('fd_write')} (param i32 i32 i32 i32) (result i32)
    (local i32 i32)
    loop
      local.get 2
      i32.eqz
      if
        local.get 3
        local.get 4
        i32.store
        i32.const 0
        return
      end
      local.get 4
      local.get 1
      i32.const 4
      i32.add
      i32.load
      i32.add
      local.set 4
      local.get 1
      i32.const 8
      i32.add
      local.set 1
      local.get 2
      i32.const 1
      i32.sub
      local.set 2
      br 0
    end
    i32.const 0)
  (func ${symbols.get('path_open')} (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)
    i32.const 44)
  (func ${symbols.get('environ_get')} (param i32 i32) (result i32)
    i32.const 0)
  (func ${symbols.get('environ_sizes_get')} (param i32 i32) (result i32)
    local.get 0
    i32.const 0
    i32.store
    local.get 1
    i32.const 0
    i32.store
    i32.const 0)
  (func ${symbols.get('clock_time_get')} (param i32 i64 i32) (result i32)
    local.get 2
    i64.const 0
    i64.store
    i32.const 0)
  (func ${symbols.get('fd_close')} (param i32) (result i32)
    i32.const 0)
  (func ${symbols.get('fd_fdstat_get')} (param i32 i32) (result i32)
    i32.const 0)
  (func ${symbols.get('fd_prestat_get')} (param i32 i32) (result i32)
    i32.const 8)
  (func ${symbols.get('fd_prestat_dir_name')} (param i32 i32 i32) (result i32)
    i32.const 8)
  (func ${symbols.get('fd_seek')} (param i32 i64 i32 i32) (result i32)
    local.get 3
    i64.const 0
    i64.store
    i32.const 0)
  (func ${symbols.get('proc_exit')} (param i32)
    unreachable)
`.trim().split('\n')

const next = [...kept.slice(0, lastImport + 1), ...stubs, ...kept.slice(lastImport + 1)].join('\n')
const watPath = join(tmpdir(), `ic-edge-wasi-stubbed-${Date.now()}.wat`)
writeFileSync(watPath, next)
execFileSync('wasm-tools', ['parse', watPath, '-o', output], { stdio: 'inherit' })
console.error(`stubbed wasi imports: ${imports.join(', ')}`)
