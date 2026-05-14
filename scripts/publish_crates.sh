#!/usr/bin/env bash
set -euo pipefail

# Publish crates.io packages in dependency order.
# Cargo publishes one package at a time and waits for the index; this order lets
# dependent crates resolve freshly published workspace crates before upload.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

crates=(
  ic-edge-web
  ic-edge-loader
  ic-edge-store
  ic-edge-runtime
  ic-edge-canister
  ic-edge-pack
)

publish_args=()
if [[ "${IC_EDGE_PUBLISH_ALLOW_DIRTY:-0}" == "1" ]]; then
  publish_args+=(--allow-dirty)
fi

scripts/package_crates.sh

for crate in "${crates[@]}"; do
  cargo publish -p "$crate" "${publish_args[@]}"
done

echo "crate publish passed"
