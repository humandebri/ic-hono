# Limitations

v1 preview Worker互換 Core+Cache runtime の制約。crate semver は `0.2.0`。

## Runtime

- QuickJS は host smoke 済み。
- `rquickjs-sys` は `wasm32-unknown-unknown` で `stdlib.h` 欠如により失敗。
- 既定 canister build は `quickjs-ic` feature のみを使う。build path は `wasm32-wasip1`、WASI import stub、`wasi2ic` 変換。
- `quickjs-wasm-rs` は QuickJS binding として残るが、旧 `wasm32-unknown-unknown` / `env.*` stub backend は削除済み。
- `free` は no-op、`malloc` / `realloc` は Rust allocator 上に C 用 header を持つ暫定実装。printf 系と一部 math/time 系は最小 stub。これは v0.1 の既知制約。
- `quickjs-wasm-rs` は deprecated 扱い。v1 では DFINITY `ic-quickjs-demo` を第一参照にした `wasm32-wasip1 + wasi2ic` 経路へ一本化した。
- canister QuickJS では numeric serialization に既知制約がある。Hono の integer JSON body、runtime response status、fetch id は canister smoke 済み。任意 package の小数や特殊 numeric 出力は v0.1 support subset 外。
- full ESM loader は未実装。
- v0.1 は 1 bundle IIFE eval を使う。
- Promise job queue は `Runtime::execute_pending_job` で drain する。
- `AsyncEdgeRuntime` と async HTTP handler は追加済み。wasm QuickJS では JS `fetch()` を queue 化し、Rust HTTPS outcall 後に Promise を再開する。

## Web API

- `Headers` / `Request` / `Response` は Hono MVP 用の最小実装。`Headers.has` / `Headers.forEach` と `Uint8Array` body の `arrayBuffer()` roundtrip を smoke 済み。
- `URL` / `URLSearchParams` は最小実装。
- `Blob` / `FormData` は最小実装。`FormData` multipart parse は未実装。
- `AbortSignal` は abort 済み `fetch()` の事前 reject のみ対応。進行中 HTTPS outcall の cancel は未実装。
- Streams は未実装。
- Cache API は `caches.default` / `caches.open` / `match` / `put` / `delete` の subset。canister stable memory に保存する。TTL、Cache-Control 評価、Range、conditional request は未実装。
- `crypto.getRandomValues` は最小実装。canister では request 開始時の `raw_rand` seed から同期 API 用 bytes を派生する。
- `crypto.subtle.digest` は SHA-256 のみ実装。
- `crypto.subtle.importKey/sign/verify` は raw HMAC-SHA-256 のみ実装。

## ICP

- `http_request` / `http_request_update` の CDK endpoint 関数は template 実装済み。
- CDK 形状の request/response DTO と変換関数は実装済み。
- `examples/canister-template` は bundle upload 入口付きで wasm build 済み。`icp.yaml` は backend build script で release wasm を build し、Candid metadata を埋め込む。
- `scripts/icp_local_smoke.sh` は fresh `icp build edge`、`icp deploy edge --yes`、bundle upload、direct update、IC Gateway、JS `fetch()` HTTPS outcall、stable Cache、rollback、zod direct update を検証する。
- 実 OpenAI / Upstash smoke は API key / REST token が必要なため任意確認。外部 fetch 経路自体は `example.com` と Hono fetch example で canister smoke 済み。
- HTTPS outcalls adapter と transform は実装済み。
- JS `fetch()` から Rust `HostFetch` への mock bridge は host smoke 済み。
- `fetch_outcall(url)` update endpoint は controller-only smoke helper として HTTPS outcalls adapter に接続済み。
- canister 内 JS `fetch()` から HTTPS outcalls adapter への実配線は `http_request_update` の async 経路で実装済み。
- stable memory-backed store は `ic-stable-structures` 実装済み。
- local memory store、stable memory store、`ic-edge upload`、canister `upload_bundle` 入口、`icp canister call` upload 経路は実装済み。
- upgrade hooks は未実装。現行 smoke は stable memory に保存した bundle / env が upgrade 後も読めることを確認する。
- `runtime_history()` と `rollback_runtime()` は直近 5 世代の bundle/env snapshot を扱う。module は v1 では `app` を snapshot 対象にする。

## Fixed Limits

- bundle: 2 MiB
- inbound body: 1 MiB
- JS response body: 1 MiB
- fetch response: default 64 KiB / max 2 MiB
- fetch count per request: 16
- cache entry: 256 KiB
- cache total: 4 MiB
- runtime history: 5 generations
- env names: 64
- env value: 16 KiB

## Package Compatibility

- `hono` は basic routing、JSON echo、params/query、CORS を確認済み。
- `zod` は host smoke と canister direct update smoke 済み。example は validated number を decimal string として返す。
- `openai` は mock `HostFetch` による non-streaming smoke 済み。実 API 接続は secret が必要な任意 outcall smoke。
- `@upstash/redis` は mock `HostFetch` による `redis.get()` smoke 済み。実 API 接続は secret が必要な任意 outcall smoke。
- `jose` は HS256 の host smoke と canister direct update smoke 済み。RSA/ECDSA/JWK full support は対象外。
