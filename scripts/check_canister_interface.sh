#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DID="examples/canister-template/ic-edge-canister-template.did"
SRC="examples/canister-template/src/lib.rs"

require_did() {
  local method="$1"
  local mode="${2:-}"
  if ! grep -q "^  ${method} : " "$DID"; then
    echo "missing $method in $DID" >&2
    exit 1
  fi
  if [[ -n "$mode" ]] && ! grep -q "^  ${method} : .* ${mode};" "$DID"; then
    echo "$method in $DID is not marked $mode" >&2
    exit 1
  fi
}

require_source() {
  local method="$1"
  if ! grep -q "fn ${method}" "$SRC"; then
    echo "missing fn $method in $SRC" >&2
    exit 1
  fi
}

for method in \
  http_request \
  http_request_update \
  upload_bundle \
  set_env \
  env_names \
  bundle_size \
  runtime_info \
  runtime_history \
  rollback_runtime \
  fetch_outcall \
  transform_strip_headers
do
  require_did "$method"
  require_source "$method"
done

require_did http_request query
require_did env_names query
require_did bundle_size query
require_did runtime_info query
require_did runtime_history query
require_did transform_strip_headers query

grep -q 'upgrade : opt bool' "$DID" || {
  echo "HttpResponse must expose upgrade : opt bool" >&2
  exit 1
}

grep -q 'type RuntimeInfo = record' "$DID" || {
  echo "RuntimeInfo type missing" >&2
  exit 1
}

grep -q 'type RuntimeSnapshotInfo = record' "$DID" || {
  echo "RuntimeSnapshotInfo type missing" >&2
  exit 1
}

grep -q 'generation : nat64' "$DID" || {
  echo "RuntimeInfo must expose generation : nat64" >&2
  exit 1
}

echo "canister interface check passed"
