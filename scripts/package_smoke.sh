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
  examples/hono-jose/src/app.ts \
  examples/hono-status/src/app.ts \
  examples/hono-suite/src/app.ts \
  examples/hono-x402-paid-api/src/app.ts
do
  example_dir="$(dirname "$(dirname "$entry")")"
  out="$example_dir/dist/app.bundle.js"
  cargo run -q -p ic-edge-pack --bin ic-edge -- pack "$entry" --out "$out" >/dev/null
  test -f "$out.map" || {
    echo "smoke failed: source map missing for $out" >&2
    exit 1
  }
  test -f "$out.ic-edge-manifest.json" || {
    echo "smoke failed: manifest missing for $out" >&2
    exit 1
  }
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

status_health="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-status/dist/app.bundle.js GET /api/health '')"
grep -q '"runtime":"ic-edge"' <<<"$status_health" || {
  echo "smoke failed: hono-status health response missing runtime" >&2
  exit 1
}
grep -q '"incidentCount":0' <<<"$status_health" || {
  echo "smoke failed: hono-status health response has unexpected incident count" >&2
  exit 1
}

status_demo="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-status/dist/app.bundle.js GET /demo '')"
grep -q '"health":"ok"' <<<"$status_demo" || {
  echo "smoke failed: hono-status demo state check failed" >&2
  exit 1
}
cargo test -q -p ic-edge-runtime --test hono_status_property
cargo test -q -p ic-edge-runtime --test hono_suite_property

suite_health="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-suite/dist/app.bundle.js GET /api/health '')"
grep -q '"tokenScope":"status:read"' <<<"$suite_health" || {
  echo "smoke failed: hono-suite health response missing verified token scope" >&2
  exit 1
}

suite_report="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-suite/dist/app.bundle.js GET /api/report '')"
grep -q '"digest":"' <<<"$suite_report" || {
  echo "smoke failed: hono-suite report digest missing" >&2
  exit 1
}

x402_catalog="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET /free/catalog '')"
grep -q '"endpoint":"/paid/report"' <<<"$x402_catalog" || {
  echo "smoke failed: hono-x402 catalog missing report product" >&2
  exit 1
}
grep -q '"price":"$0.001"' <<<"$x402_catalog" || {
  echo "smoke failed: hono-x402 catalog missing report price" >&2
  exit 1
}
grep -q '"endpoint":"/paid/outcall"' <<<"$x402_catalog" || {
  echo "smoke failed: hono-x402 catalog missing outcall product" >&2
  exit 1
}
grep -q '"price":"$0.003"' <<<"$x402_catalog" || {
  echo "smoke failed: hono-x402 catalog missing outcall price" >&2
  exit 1
}

x402_unpaid="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET /paid/report '' --show-response)"
grep -q 'status: 402' <<<"$x402_unpaid" || {
  echo "smoke failed: hono-x402 unpaid request did not return 402" >&2
  exit 1
}
grep -q 'header: payment-required:' <<<"$x402_unpaid" || {
  echo "smoke failed: hono-x402 unpaid response missing PAYMENT-REQUIRED" >&2
  exit 1
}

x402_signature_json="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET '/demo/payment-signature?endpoint=/paid/report' '')"
x402_signature="$(node -e 'console.log(JSON.parse(process.argv[1]).value)' "$x402_signature_json")"
x402_paid="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET /paid/report '' "PAYMENT-SIGNATURE: $x402_signature" --show-response)"
grep -q 'status: 200' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid request did not return 200" >&2
  exit 1
}
grep -q 'header: payment-response:' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing PAYMENT-RESPONSE" >&2
  exit 1
}
grep -q '"payerHash":"' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing payerHash" >&2
  exit 1
}
grep -q '"eventHash":"' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing audit event hash" >&2
  exit 1
}
grep -q '"productId":"report"' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing report productId" >&2
  exit 1
}
grep -q '"price":"$0.001"' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing report price" >&2
  exit 1
}
grep -q '"payTo":"' <<<"$x402_paid" || {
  echo "smoke failed: hono-x402 paid response missing payTo" >&2
  exit 1
}
if grep -q '"payer":"demo-payer"' <<<"$x402_paid"; then
  echo "smoke failed: hono-x402 paid response leaked raw payer" >&2
  exit 1
fi
x402_wrong_outcall="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET /paid/outcall '' "PAYMENT-SIGNATURE: $x402_signature" --show-response)"
grep -q 'status: 402' <<<"$x402_wrong_outcall" || {
  echo "smoke failed: hono-x402 accepted report signature for outcall" >&2
  exit 1
}
grep -q 'payment does not match requirements' <<<"$x402_wrong_outcall" || {
  echo "smoke failed: hono-x402 wrong product signature did not explain mismatch" >&2
  exit 1
}
x402_outcall_signature_json="$(cargo run -q -p ic-edge-runtime --example eval_bundle -- examples/hono-x402-paid-api/dist/app.bundle.js GET '/demo/payment-signature?endpoint=/paid/outcall' '')"
x402_outcall_signature="$(node -e 'console.log(JSON.parse(process.argv[1]).value)' "$x402_outcall_signature_json")"
x402_outcall_paid="$(cargo run -q -p ic-edge-runtime --example eval_bundle_fetch -- examples/hono-x402-paid-api/dist/app.bundle.js GET '/paid/outcall?url=https%3A%2F%2Fexample.com%2F' '' "PAYMENT-SIGNATURE: $x402_outcall_signature" --show-response)"
grep -q 'status: 200' <<<"$x402_outcall_paid" || {
  echo "smoke failed: hono-x402 outcall paid request did not return 200" >&2
  exit 1
}
grep -q '"productId":"outcall"' <<<"$x402_outcall_paid" || {
  echo "smoke failed: hono-x402 outcall paid response missing outcall productId" >&2
  exit 1
}

fetch_ok="$(cargo run -q -p ic-edge-runtime --example eval_bundle_fetch -- examples/hono-fetch/dist/app.bundle.js)"
expect "$fetch_ok" '{"url":"https://api.github.com"}' "hono-fetch host bridge"

openai_ok="$(cargo run -q -p ic-edge-runtime --example eval_openai -- examples/hono-openai/dist/app.bundle.js)"
expect "$openai_ok" '{"id":"resp_test","text":"mocked"}' "hono-openai non-streaming mock bridge"

upstash_ok="$(cargo run -q -p ic-edge-runtime --example eval_upstash -- examples/hono-upstash/dist/app.bundle.js)"
expect "$upstash_ok" '{"value":"mocked-value"}' "hono-upstash mock bridge"

echo "package smoke passed"
