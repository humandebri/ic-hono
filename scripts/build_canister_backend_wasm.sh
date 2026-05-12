#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUTPUT="${1:-target/wasm32-unknown-unknown/release/ic_edge_canister_template.wasm}"

cargo build --package ic-edge-canister-template --target wasm32-wasip1 --release --features quickjs-ic
tmp_wasi="${OUTPUT}.wasi-stubbed"
node scripts/stub_wasm_wasi_imports.mjs \
  target/wasm32-wasip1/release/ic_edge_canister_template.wasm \
  "$tmp_wasi"
wasi2ic "$tmp_wasi" "$OUTPUT"
