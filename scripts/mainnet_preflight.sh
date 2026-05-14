#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ "${IC_EDGE_MAINNET_PREFLIGHT:-0}" != "1" ]]; then
  echo "set IC_EDGE_MAINNET_PREFLIGHT=1 to run the mainnet preflight" >&2
  exit 1
fi

write_evidence() {
  local path="${IC_EDGE_PREFLIGHT_EVIDENCE:-}"
  if [[ -z "$path" ]]; then
    return 0
  fi

  mkdir -p "$(dirname "$path")"
  {
    printf '# Mainnet Preflight Evidence\n\n'
    printf -- '- Date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '- Command: `IC_EDGE_PREFLIGHT_EVIDENCE=%s IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh`\n' "$path"
    printf -- '- Identity principal: `%s`\n' "$principal"
    printf -- '- Edge canister status:\n\n'
    printf '```text\n%s\n```\n\n' "$status"
    printf -- '- Wasm import audit result: no `env.*` or `wasi_snapshot_preview1.*` imports after backend build\n'
    printf -- '- Secret check: no secrets included\n'
  } >"$path"
}

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

write_evidence
echo "mainnet preflight passed for identity $principal"
