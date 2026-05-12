#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo fmt --all --check
cargo test
scripts/check_canister_interface.sh
scripts/check_release_audit.sh
scripts/package_smoke.sh
cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic
scripts/build_canister_backend_wasm.sh /tmp/ic_edge_runtime_import_check.wasm
if wasm-objdump -x /tmp/ic_edge_runtime_import_check.wasm | grep -q '<.*> <- \\(env\\|wasi_snapshot_preview1\\)\\.'; then
  echo "unexpected env.* or wasi_snapshot_preview1.* import after backend build" >&2
  exit 1
fi

echo "local CI checks passed"
