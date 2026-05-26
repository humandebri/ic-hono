#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
LOG_DIR="${IC_EDGE_SMOKE_LOG_DIR:-/tmp/ic-edge-smoke-logs}"
mkdir -p "$LOG_DIR"

run_logged() {
  local name="$1"
  shift
  "$@" >"$LOG_DIR/$name.out" 2>"$LOG_DIR/$name.err"
}

run_logged_retry() {
  local name="$1"
  shift
  run_logged "$name" "$@" || {
    sleep 3
    run_logged "$name-retry" "$@"
  }
}

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "missing required env: $name" >&2
    exit 1
  fi
}

if [[ "${IC_EDGE_FULL_SMOKE:-0}" == "1" ]]; then
  require_env OPENAI_API_KEY
  require_env UPSTASH_REDIS_REST_URL
  require_env UPSTASH_REDIS_REST_TOKEN
fi

cargo fmt --all --check
cargo test
scripts/check_canister_interface.sh
scripts/check_release_audit.sh
scripts/package_smoke.sh
cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic
scripts/build_canister_backend_wasm.sh /tmp/ic_edge_runtime_import_check.wasm
wasm-objdump -x /tmp/ic_edge_runtime_import_check.wasm | grep -q '<.*> <- \\(env\\|wasi_snapshot_preview1\\)\\.' && {
  echo "unexpected env.* or wasi_snapshot_preview1.* import after backend build" >&2
  exit 1
}

call_update() {
  local method="$1"
  local url="$2"
  local body="$3"
  local headers="${4-}"
  if [[ -z "$headers" ]]; then
    headers='vec { record { "content-type"; "application/json" } }'
  fi
  icp canister call edge http_request_update \
    "(record { method = \"${method}\"; url = \"${url}\"; headers = ${headers}; body = blob \"${body}\"; certificate_version = null })" \
    --environment local
}

upload_example() {
  local bundle="$1"
  local bytecode="${bundle%.bundle.js}.qjbc"
  cargo run -p ic-edge-pack --bin ic-edge -- upload "$bytecode" --canister edge --environment local
}

pack_example() {
  local entry="$1"
  local out="$2"
  echo "packing bundle with manifest: $entry -> $out"
  PATH="$ROOT/examples/hono-basic/node_modules/.bin:$PATH" \
    cargo run -p ic-edge-pack --bin ic-edge -- pack "$entry" --out "$out" >/dev/null
}

set_env_value() {
  local name="$1"
  local value="$2"
  local argument_path
  local output
  argument_path="$(mktemp)"
  NAME="$name" VALUE="$value" node -e 'const fs = require("fs"); fs.writeFileSync(process.argv[1], `(${JSON.stringify(process.env.NAME)}, ${JSON.stringify(process.env.VALUE)})`)' "$argument_path"
  output="$(icp canister call edge set_env --args-file "$argument_path" --environment local)"
  grep -q 'Ok' <<<"$output"
}

expect_contains() {
  local pattern="$1"
  shift
  local output
  output="$("$@")"
  grep -q "$pattern" <<<"$output"
}

expect_not_contains() {
  local pattern="$1"
  shift
  local output
  output="$("$@")"
  ! grep -q "$pattern" <<<"$output"
}

run_logged icp-build icp build edge
if [[ "${IC_EDGE_RESTART_NETWORK:-0}" == "1" ]] && icp network status >/dev/null 2>&1; then
  run_logged icp-network-stop icp network stop
  sleep 3
fi
if ! icp network status >/dev/null 2>&1; then
  run_logged_retry icp-network-start icp network start -d
fi
run_logged_retry icp-deploy-initial icp deploy edge --yes
pack_example examples/hono-basic/src/app.ts examples/hono-basic/dist/app.bundle.js
upload_example examples/hono-basic/dist/app.bundle.js
set_env_value IC_EDGE_SMOKE ok
expect_contains 'IC_EDGE_SMOKE' icp canister call edge env_names '()' --environment local
rollback_generation="$(icp canister call edge runtime_info '()' --environment local | sed -n 's/.*generation = \([0-9_]*\) : nat64.*/\1/p' | tr -d '_')"
test -n "$rollback_generation"
set_env_value IC_EDGE_ROLLBACK discard
expect_contains 'IC_EDGE_ROLLBACK' icp canister call edge env_names '()' --environment local
expect_contains 'Ok' icp canister call edge rollback_runtime "(${rollback_generation} : nat64)" --environment local
expect_contains 'IC_EDGE_SMOKE' icp canister call edge env_names '()' --environment local
expect_not_contains 'IC_EDGE_ROLLBACK' icp canister call edge env_names '()' --environment local
expect_contains 'status_code = 200' icp canister call edge fetch_outcall '("https://example.com/")' --environment local
expect_contains 'status_code = 200' icp canister call edge fetch_outcall_replicated '("https://example.com/")' --environment local
runtime_info_before_upgrade="$(icp canister call edge runtime_info '()' --environment local)"
grep -q 'generation' <<<"$runtime_info_before_upgrade"
run_logged_retry icp-deploy-upgrade icp deploy edge --yes
expect_contains 'opt' icp canister call edge bytecode_size '("app")' --environment local
expect_contains 'IC_EDGE_SMOKE' icp canister call edge env_names '()' --environment local
runtime_info_after_upgrade="$(icp canister call edge runtime_info '()' --environment local)"
[[ "$runtime_info_after_upgrade" == "$runtime_info_before_upgrade" ]]
runtime_info_before_requests="$(icp canister call edge runtime_info '()' --environment local)"
expect_contains 'body = blob "ok"' call_update GET / ''
expect_contains '\\22hello\\22:\\22world\\22' call_update POST /echo '\7b\22hello\22\3a\22world\22\7d'
expect_contains '\\22id\\22:\\22123\\22' call_update GET '/users/123?q=test' ''
expect_contains '\\22count\\22:1' call_update GET /number ''
expect_contains 'body = blob "ic"' call_update GET /bytes ''
expect_contains '\\22first\\22:105' call_update POST /body-bytes '\69\63'
expect_contains 'body = blob "stored"' call_update GET /cache-put ''
expect_contains 'body = blob "cached"' call_update GET /cache-get ''
expect_contains 'body = blob "missing"' call_update GET /cache-expired ''
time_output="$(call_update GET /time '')"
grep -Eq 'body = blob "[0-9]+"' <<<"$time_output"
runtime_info_after_requests="$(icp canister call edge runtime_info '()' --environment local)"
[[ "$runtime_info_after_requests" == "$runtime_info_before_requests" ]]
expect_contains 'access-control-allow-origin' call_update OPTIONS / '' 'vec { record { "Origin"; "https://example.com" }; record { "Access-Control-Request-Method"; "POST" } }'

CANISTER_ID="$(icp canister status edge --environment local | awk '/Canister Id:/ { print $3; exit }')"
GATEWAY_URL="$(icp network status --json | sed -n 's/.*"gateway_url": "\(.*\)",/\1/p')"
BASE_URL="${GATEWAY_URL%/}/?canisterId=${CANISTER_ID}"

curl -fsS "$BASE_URL" | grep -qx 'ok'
curl -fsS -X POST \
  -H 'content-type: application/json' \
  --data '{"hello":"world"}' \
  "${GATEWAY_URL%/}/echo?canisterId=${CANISTER_ID}" | grep -qx '{"hello":"world"}'
curl -fsS "${GATEWAY_URL%/}/users/123?q=test&canisterId=${CANISTER_ID}" | grep -qx '{"id":"123","q":"test"}'
curl -fsS "${GATEWAY_URL%/}/number?canisterId=${CANISTER_ID}" | grep -qx '{"count":1}'
curl -fsS "${GATEWAY_URL%/}/bytes?canisterId=${CANISTER_ID}" | grep -qx 'ic'
curl -fsS -X POST \
  --data 'ic' \
  "${GATEWAY_URL%/}/body-bytes?canisterId=${CANISTER_ID}" | grep -qx '{"first":105,"length":2}'
curl -fsS "${GATEWAY_URL%/}/cache-expired?canisterId=${CANISTER_ID}" | grep -qx 'missing'
curl -fsS "${GATEWAY_URL%/}/time?canisterId=${CANISTER_ID}" | grep -Eq '^[0-9]+$'
curl -fsSI "$BASE_URL" | grep -qi '^access-control-allow-origin:'

binary_source="$LOG_DIR/binary-echo.ts"
binary_bundle="$LOG_DIR/binary-echo.bundle.js"
cat >"$binary_source" <<'JS'
export default {
  fetch: async (request) => new Response(new Uint8Array(await request.arrayBuffer()))
}
JS
pack_example "$binary_source" "$binary_bundle"
upload_example "$binary_bundle"
expect_contains 'body = blob "\\ff\\00\\80"' call_update POST /binary '\ff\00\80'

pack_example examples/hono-zod/src/app.ts examples/hono-zod/dist/app.bundle.js
upload_example examples/hono-zod/dist/app.bundle.js
expect_contains '\\22ok\\22:true' call_update POST /validate '\7b\22name\22\3a\22ic\22\2c\22count\22\3a1\7d'

pack_example examples/hono-fetch/src/app.ts examples/hono-fetch/dist/app.bundle.js
upload_example examples/hono-fetch/dist/app.bundle.js
expect_contains 'status_code = 200' call_update GET /github ''
expect_contains '\\22status\\22:200' call_update GET /example-replicated ''

pack_example examples/hono-jose/src/app.ts examples/hono-jose/dist/app.bundle.js
upload_example examples/hono-jose/dist/app.bundle.js
expect_contains '\\22sub\\22:\\22edge\\22' call_update GET /jwt ''

if [[ "${IC_EDGE_FULL_SMOKE:-0}" == "1" ]]; then
  set_env_value OPENAI_API_KEY "$OPENAI_API_KEY"
  if [[ -n "${OPENAI_MODEL:-}" ]]; then
    set_env_value OPENAI_MODEL "$OPENAI_MODEL"
  fi
  pack_example examples/hono-openai/src/app.ts examples/hono-openai/dist/app.bundle.js
  upload_example examples/hono-openai/dist/app.bundle.js
  expect_contains '\\22text\\22' call_update POST /respond '\68\65\6c\6c\6f'
  set_env_value UPSTASH_REDIS_REST_URL "$UPSTASH_REDIS_REST_URL"
  set_env_value UPSTASH_REDIS_REST_TOKEN "$UPSTASH_REDIS_REST_TOKEN"
  pack_example examples/hono-upstash/src/app.ts examples/hono-upstash/dist/app.bundle.js
  upload_example examples/hono-upstash/dist/app.bundle.js
  expect_contains '\\22value\\22' call_update GET /kv/ic-edge-smoke ''
fi

echo "icp local smoke passed"
echo "logs: $LOG_DIR"
