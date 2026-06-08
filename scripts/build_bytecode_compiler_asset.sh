#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build -q \
  -p ic-edge-bytecode-compiler \
  --bin ic-edge-bytecode-compiler \
  --target wasm32-wasip1 \
  --release

mkdir -p crates/ic-edge-pack/assets
cp \
  target/wasm32-wasip1/release/ic-edge-bytecode-compiler.wasm \
  crates/ic-edge-pack/assets/ic-edge-bytecode-compiler.wasm

echo "updated crates/ic-edge-pack/assets/ic-edge-bytecode-compiler.wasm"
