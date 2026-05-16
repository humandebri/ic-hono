#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

init_tmp="/tmp/ic-edge-init-smoke"
rm -rf "$init_tmp"
cargo run -q -p ic-edge-pack --bin ic-edge -- init hono "$init_tmp" >/dev/null
test -f "$init_tmp/src/app.ts" || {
  echo "smoke failed: ic-edge init hono did not create src/app.ts" >&2
  exit 1
}

pack_tmp="/tmp/ic-edge-pack-basic.js"
cargo run -q -p ic-edge-pack --bin ic-edge -- pack examples/hono-basic/src/app.ts --out "$pack_tmp" >/dev/null

(
  cd examples/hono-fetch
  ./node_modules/.bin/tsc --noEmit --lib es2022,dom --moduleResolution bundler --module esnext --target es2022 src/app.ts
)

for entry in \
  examples/hono-basic/src/app.ts \
  examples/hono-zod/src/app.ts \
  examples/hono-fetch/src/app.ts \
  examples/hono-openai/src/app.ts \
  examples/hono-upstash/src/app.ts \
  examples/hono-jose/src/app.ts
do
  example_dir="$(dirname "$(dirname "$entry")")"
  out="$example_dir/dist/app.bundle.js"
  cargo run -q -p ic-edge-pack --bin ic-edge -- pack "$entry" --out "$out" >/dev/null
done

expect() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  if [[ "$actual" != "$expected" ]]; then
    printf 'smoke failed: %s\nexpected: %s\nactual:   %s\n' "$label" "$expected" "$actual" >&2
    exit 1
  fi
}

basic_root="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET / '')"
expect "$basic_root" "ok" "hono-basic GET /"

basic_echo="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js POST /echo '{"hello":"world"}' 'content-type: application/json')"
expect "$basic_echo" '{"hello":"world"}' "hono-basic POST /echo"

basic_user="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET '/users/123?q=test' '')"
expect "$basic_user" '{"id":"123","q":"test"}' "hono-basic GET /users/:id"

basic_number="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET /number '')"
expect "$basic_number" '{"count":1}' "hono-basic GET /number"

basic_bytes="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET /bytes '')"
expect "$basic_bytes" "ic" "hono-basic GET /bytes"

basic_body_bytes="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js POST /body-bytes ic '')"
expect "$basic_body_bytes" '{"first":105,"length":2}' "hono-basic POST /body-bytes"

basic_cache="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET /cache-roundtrip '')"
expect "$basic_cache" "cached" "hono-basic GET /cache-roundtrip"

basic_cache_expired="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET /cache-expired '')"
expect "$basic_cache_expired" "missing" "hono-basic GET /cache-expired"

cors_headers="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-basic/dist/app.bundle.js GET / '' --show-response)"
grep -q 'header: access-control-allow-origin: \*' <<<"$cors_headers" || {
  echo "smoke failed: hono-basic CORS header missing" >&2
  exit 1
}

zod_ok="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-zod/dist/app.bundle.js POST /validate '{"name":"Ada","count":1}' 'content-type: application/json')"
expect "$zod_ok" '{"ok":true,"greeting":"hello Ada","count":"1"}' "hono-zod valid payload"

jose_ok="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-jose/dist/app.bundle.js GET /jwt '')"
grep -q '"sub":"edge"' <<<"$jose_ok" || {
  echo "smoke failed: hono-jose HS256 response missing sub" >&2
  exit 1
}

fetch_ok="$(cargo run -q -p ic-edge-runtime --example eval_bundle_fetch -- examples/hono-fetch/dist/app.bundle.js)"
expect "$fetch_ok" '{"url":"https://api.github.com"}' "hono-fetch host bridge"

openai_ok="$(cargo run -q -p ic-edge-runtime --example eval_openai -- examples/hono-openai/dist/app.bundle.js)"
expect "$openai_ok" '{"id":"resp_test","text":"mocked"}' "hono-openai non-streaming mock bridge"

upstash_ok="$(cargo run -q -p ic-edge-runtime --example eval_upstash -- examples/hono-upstash/dist/app.bundle.js)"
expect "$upstash_ok" '{"value":"mocked-value"}' "hono-upstash mock bridge"

echo "package smoke passed"
