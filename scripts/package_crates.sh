#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

crates=(
  ic-edge-web
  ic-edge-loader
  ic-edge-store
  ic-edge-bytecode-compiler
  ic-edge-runtime
  ic-edge-canister
  ic-edge-pack
)

publish_ready_crates=(
  ic-edge-web
  ic-edge-loader
  ic-edge-store
  ic-edge-bytecode-compiler
)

dependent_crates=(
  ic-edge-runtime
  ic-edge-canister
  ic-edge-pack
)

scripts/check_bytecode_compiler_asset.sh

cargo fmt --all --check
cargo test
cargo doc --workspace --no-deps

for crate in "${publish_ready_crates[@]}"; do
  cargo package -p "$crate" --allow-dirty
done

for crate in "${dependent_crates[@]}"; do
  cargo package -p "$crate" --list --allow-dirty >/dev/null
done

cargo package -p ic-edge-pack --list --allow-dirty \
  | grep -q 'assets/ic-edge-bytecode-compiler.wasm'

for crate in "${publish_ready_crates[@]}"; do
  cargo publish -p "$crate" --dry-run --allow-dirty
done

if [[ "${IC_EDGE_FULL_PUBLISH_DRY_RUN:-0}" == "1" ]]; then
  for crate in "${dependent_crates[@]}"; do
    cargo publish -p "$crate" --dry-run --allow-dirty
  done
else
  printf 'skipped dependent crate publish dry-run until workspace dependencies exist on crates.io: %s\n' "${dependent_crates[*]}"
fi

echo "crate package dry-run passed"
