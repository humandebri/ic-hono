#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CONTRACT="docs/public-api-contract.md"

require_contract() {
  local text="$1"
  if ! grep -q "$text" "$CONTRACT"; then
    echo "public API contract missing: $text" >&2
    exit 1
  fi
}

require_source() {
  local text="$1"
  local path="$2"
  if ! grep -q "$text" "$path"; then
    echo "source missing API contract item '$text' in $path" >&2
    exit 1
  fi
}

test -f "$CONTRACT" || {
  echo "$CONTRACT missing" >&2
  exit 1
}

for item in \
  EdgeRuntime \
  AsyncEdgeRuntime \
  AsyncHostFetch \
  HostFetchOptions \
  CacheHost \
  AuditHost \
  QuickJsRuntime \
  CdkHttpRequest \
  CdkHttpResponse \
  handle_cdk_http \
  handle_cdk_http_async \
  OutcallReplication \
  https_outcall_fetch \
  https_outcall_fetch_with_replication \
  build_https_outcall_args \
  build_https_outcall_args_with_replication \
  transform_strip_headers \
  "ic-edge init hono" \
  "ic-edge pack" \
  "ic-edge upload" \
  begin_bytecode_upload \
  append_bytecode_chunk \
  commit_bytecode_upload \
  abort_bytecode_upload \
  runtime_history \
  rollback_runtime \
  fetch_outcall_replicated \
  Streams \
  "full ESM loader" \
  "multipart"
do
  require_contract "$item"
done

for item in EdgeRuntime AsyncEdgeRuntime AsyncHostFetch CacheHost AuditHost; do
  require_source "trait $item" "crates/ic-edge-runtime/src/lib.rs"
done

require_source "struct HostFetchOptions" "crates/ic-edge-runtime/src/lib.rs"

for item in CdkHttpRequest CdkHttpResponse handle_cdk_http handle_cdk_http_async OutcallReplication https_outcall_fetch https_outcall_fetch_with_replication build_https_outcall_args build_https_outcall_args_with_replication transform_strip_headers; do
  require_source "$item" "crates/ic-edge-canister/src/lib.rs"
done

scripts/check_canister_interface.sh >/dev/null

echo "public API contract check passed"
