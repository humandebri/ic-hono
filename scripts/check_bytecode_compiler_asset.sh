#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

asset="crates/ic-edge-pack/assets/ic-edge-bytecode-compiler.wasm"
if [[ ! -s "$asset" ]]; then
  printf 'missing bytecode compiler asset: %s\n' "$asset" >&2
  printf 'run: scripts/build_bytecode_compiler_asset.sh\n' >&2
  exit 1
fi

cargo build -q \
  -p ic-edge-bytecode-compiler \
  --bin ic-edge-bytecode-compiler \
  --target wasm32-wasip1 \
  --release

built="target/wasm32-wasip1/release/ic-edge-bytecode-compiler.wasm"
if ! cmp -s "$built" "$asset"; then
  printf 'stale bytecode compiler asset: %s\n' "$asset" >&2
  printf 'run: scripts/build_bytecode_compiler_asset.sh\n' >&2
  exit 1
fi

echo "bytecode compiler asset is current"
