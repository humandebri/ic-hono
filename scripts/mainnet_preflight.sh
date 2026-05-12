#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ "${IC_EDGE_MAINNET_PREFLIGHT:-0}" != "1" ]]; then
  echo "set IC_EDGE_MAINNET_PREFLIGHT=1 to run the mainnet preflight" >&2
  exit 1
fi

cargo fmt --all --check
cargo test
scripts/check_api_contract.sh
scripts/check_compatibility_matrix.sh
scripts/check_release_audit.sh
scripts/check_canister_interface.sh
icp build edge
scripts/build_canister_backend_wasm.sh /tmp/ic_edge_mainnet_preflight.wasm
if wasm-objdump -x /tmp/ic_edge_mainnet_preflight.wasm | grep -q '<.*> <- \\(env\\|wasi_snapshot_preview1\\)\\.'; then
  echo "unexpected env.* or wasi_snapshot_preview1.* import after backend build" >&2
  exit 1
fi

principal="$(icp identity principal)"
test -n "$principal"
icp cycles balance -n ic >/dev/null
icp canister list -e ic | grep -q 'edge' || {
  echo "edge canister is not mapped in the ic environment" >&2
  exit 1
}
status="$(icp canister status edge -e ic)"
grep -qi 'running' <<<"$status" || {
  echo "edge canister is not running on mainnet" >&2
  exit 1
}

echo "mainnet preflight passed for identity $principal"
