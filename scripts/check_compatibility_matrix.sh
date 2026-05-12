#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MATRIX="docs/compatibility.md"

require_matrix() {
  local text="$1"
  if ! grep -q "$text" "$MATRIX"; then
    echo "compatibility matrix missing: $text" >&2
    exit 1
  fi
}

require_smoke() {
  local text="$1"
  if ! grep -q "$text" scripts/package_smoke.sh scripts/icp_local_smoke.sh; then
    echo "compatibility smoke missing: $text" >&2
    exit 1
  fi
}

for package in hono zod jose openai "@upstash/redis"; do
  require_matrix "$package"
done

for api in Request Response Headers URLSearchParams FormData fetch Cache Streams crypto; do
  require_matrix "$api"
done

for smoke in \
  "hono-basic" \
  "hono-zod" \
  "hono-jose" \
  "hono-fetch" \
  "hono-openai" \
  "hono-upstash" \
  "cache-expired" \
  "/time"
do
  require_smoke "$smoke"
done

cargo test -p ic-edge-runtime --test web_api
scripts/package_smoke.sh

echo "compatibility matrix check passed"
