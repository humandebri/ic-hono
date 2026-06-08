# Compatibility Matrix

Durable Edge Runtime on ICP v1 preview の互換性状況。crate semver は `0.2.0`。

## Package Status

`Status` は host QuickJS smoke と local canister smoke の状態。
実 API secret が必要な項目は `IC_EDGE_FULL_SMOKE=1` で任意確認できる。v1 preview release gate は local deploy smoke、直接 HTTPS outcall、JS `fetch()` canister smoke を必須 evidence とする。

| Package | Status | Host Evidence | Canister Evidence |
| --- | --- | --- | --- |
| `hono` | pass | `examples/hono-basic` の IIFE bundle から生成した bytecode を QuickJS で eval。basic route、JSON、params/query、CORS、Cache roundtrip が通過 | `scripts/icp_local_smoke.sh` で bytecode upload canister smoke、direct update / Gateway / stable Cache が通過 |
| `zod` | pass | `examples/hono-zod` で valid JSON validation が通過 | `scripts/icp_local_smoke.sh` で direct update が通過 |
| `jose` | pass | `examples/hono-jose` で HS256 sign/verify roundtrip が通過 | `scripts/icp_local_smoke.sh` で direct update が通過 |
| `openai` | optional real API smoke | `examples/hono-openai` で `responses.create` non-streaming を mock `HostFetch` で通過。model は `OPENAI_MODEL` で差し替え可能 | optional `IC_EDGE_FULL_SMOKE=1` with real API key |
| `@upstash/redis` | optional real API smoke | `examples/hono-upstash` で `redis.get()` を mock `HostFetch` で通過。telemetry は無効化 | optional `IC_EDGE_FULL_SMOKE=1` with real REST token |

## Hono Coverage

| Feature | Status | Note |
| --- | --- | --- |
| `app.fetch()` | pass | bytecode eval 後の `default.fetch` を QuickJS から呼ぶ |
| text response | pass | host と Gateway `GET /` -> `ok` |
| JSON request body | pass | host と Gateway `POST /echo` -> request JSON |
| JSON numeric response | pass | host と canister `/number` -> `{"count":1}` |
| Uint8Array response body | pass | host と canister `/bytes` -> `ic` |
| Request arrayBuffer | pass | host と canister `/body-bytes` -> byte length / first byte |
| route params | pass | host と Gateway `/users/:id` |
| query string | pass | host と Gateway `c.req.query('q')` |
| middleware | pass | CORS の response header と `OPTIONS` preflight が通過 |
| external fetch | pass | JS `fetch()` -> HTTPS outcall が local canister direct update で通過 |
| runtime generation | pass | `runtime_info()` generation が chunk upload/env で増加し、upgrade 後と連続 request 後も維持される |
| rollback | pass | `runtime_history()` と `rollback_runtime()` で bytecode/env/manifest snapshot を復元 |

## Web Standards Subset

| API | Status | Note |
| --- | --- | --- |
| `Request` | partial | method / url / headers / bodyUsed / clone / `Request(Request)` / text / json / exact-range arrayBuffer / urlencoded formData |
| `Response` | partial | status / statusText / headers / bodyUsed / clone / `Response(Response)` / url / redirected / type / text / json / exact-range arrayBuffer / `Response.json`; host は `Uint8Array` body smoke 済み |
| `Headers` | partial | append / set / delete / get / has / forEach / entries / keys / values / getSetCookie / iteration。CR / LF / NUL value は拒否 |
| `URL` | minimal | absolute URL と base 付き path |
| `URLSearchParams` | partial | string / array / object init、append / set / get / getAll / has / delete / sort / forEach / toString、`+` decode |
| `TextEncoder` | pass | UTF-8 encode を unit test 済み |
| `TextDecoder` | pass | UTF-8 decode を unit test 済み |
| `Blob` | minimal | string / `Uint8Array` / `ArrayBuffer` parts、text / exact-range arrayBuffer |
| `FormData` | partial | append / get / entries。`Request.formData()` は urlencoded body に対応 |
| `AbortController` / `AbortSignal` | minimal | abort 済み `fetch()` を outcall 前に reject |
| `fetch` | pass | host bridge / async queue / HTTPS outcall direct smoke |
| `Cache` / `caches` | partial | `caches.default`、`caches.open`、`match` / `put` / `delete`。canister stable memory に保存。`Cache-Control: max-age=N` expiration 対応。`Set-Cookie` response、Range、conditional request は非対応 |
| Streams | unsupported | `ReadableStream` / `WritableStream` / `TransformStream` は v1 対象外 |
| `crypto.getRandomValues` | partial | host callback。canister backend は `raw_rand` seed から同期 bytes を派生。package smoke 対象 |
| `crypto.subtle` | partial | SHA-256 digest、raw HMAC-SHA-256 sign / verify。canister `quickjs-ic` callback build 済み |

## Current Smoke Commands

```bash
cd examples/hono-basic
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-basic/dist/app.bundle.js GET /
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-basic/dist/app.bundle.js POST /echo '{"hello":"world"}'
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-basic/dist/app.bundle.js GET '/users/123?q=test'
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-basic/dist/app.bundle.js OPTIONS / '' \
  'Origin: https://example.com' \
  'Access-Control-Request-Method: POST' \
  --show-response
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-zod/dist/app.bundle.js POST /validate \
  '{"name":"ic","count":1}' --show-response
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-jose/dist/app.bundle.js GET /jwt --show-response
cargo run -p ic-edge-runtime --example eval_bundle_fetch -- \
  examples/hono-fetch/dist/app.bundle.js
cargo run -p ic-edge-runtime --example eval_openai -- \
  examples/hono-openai/dist/app.bundle.js
cargo run -p ic-edge-runtime --example eval_upstash -- \
  examples/hono-upstash/dist/app.bundle.js
cargo run -p ic-edge-pack -- upload \
  examples/hono-basic/dist/app.qjbc --module app
cargo run -p ic-edge-pack -- upload \
  examples/hono-basic/dist/app.qjbc --module app \
  --canister edge --environment local
cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic
scripts/build_canister_backend_wasm.sh /tmp/ic_edge_runtime_import_check.wasm
scripts/icp_local_smoke.sh
```
