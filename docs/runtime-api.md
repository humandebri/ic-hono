# Runtime API

v1 preview Worker互換 Core+Cache runtime の内部 API。crate semver は `0.2.0`。

## Rust Runtime

`ic-edge-runtime` は `EdgeRuntime` trait を公開する。

```rust
pub trait EdgeRuntime {
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()>;
    fn call_app_fetch(&mut self, request: Request) -> Result<Response>;
}
```

async host 境界用に `AsyncEdgeRuntime` も公開する。

```rust
pub trait AsyncEdgeRuntime {
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()>;
    fn call_app_fetch<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + 'a>>;
}
```

host `QuickJsRuntime` は `rquickjs` で bundle を eval し、`app.fetch(request)` を呼ぶ。
canister build は `quickjs-ic` feature のみを使う。`wasm32-wasip1` build、WASI import stub、`wasi2ic` 変換を行う。

## Bundle Contract

v0.1 は ESM loader ではなく IIFE bundle を使う。`ic-edge pack` は local `esbuild` を実行し、この形を生成する。

```bash
esbuild src/app.ts \
  --bundle \
  --format=iife \
  --global-name=__ic_edge_bundle \
  --platform=neutral \
  --conditions=browser,worker,import \
  --outfile=dist/app.bundle.js
```

CLI は出力 bundle を QuickJS runtime で eval し、`globalThis.__ic_edge_bundle.default.fetch` が function であることを検査する。runtime は eval 後に次の値を app として扱う。

```ts
globalThis.__ic_edge_bundle.default
```

## CLI

```bash
ic-edge init hono my-app
cd my-app
npm install
npm run build
```

`ic-edge init hono` は Hono basic template を生成する。既存ファイルは上書きしない。

## Web API Subset

現在の polyfill:

- `Headers`
- `Request`
- `Response`
- `console`
- `URL`
- `URLSearchParams`
- `Blob` 簡易版
- `FormData` 簡易版
- `AbortController` 簡易版
- `setTimeout` / `clearTimeout` 簡易版
- `TextEncoder` / `TextDecoder` 簡易版
- `crypto.getRandomValues`
- `crypto.subtle.digest` SHA-256
- `crypto.subtle.importKey/sign/verify` raw HMAC-SHA-256
- `Cache` / `caches.default` / `caches.open`
- `atob` 最小実装
- `process.env` read-only 相当
- `ic.caller()` / `ic.time()` / `ic.canisterId()` local placeholder

`Request.text()`、`Request.json()`、`Request.arrayBuffer()`、`Request.formData()`、`Response.text()`、`Response.json()`、`Response.arrayBuffer()`、`Response.formData()` は Promise を返す。body read 後は `bodyUsed` が true になり、再 read と `clone()` は `TypeError` を投げる。

`Headers` は `append` / `set` / `delete` / `get` / `has` / `forEach` / `entries` / `keys` / `values` / `getSetCookie` / iterator を実装する。name は HTTP token、value は CR / LF / NUL なしを検証する。`Set-Cookie` は `getSetCookie()` で個別値を返す。

v0.2 host runtime は `Request` / `Response` / `Blob` body に `Uint8Array`、typed array view、`ArrayBuffer` を受ける。`arrayBuffer()` は typed array view の実範囲だけを返す。Rust-JS bridge は request / response / host fetch body を byte array JSON で渡すため、non-UTF-8 byte sequence を維持する。`text()` は UTF-8 text 変換用 API。

`AbortController` / `AbortSignal` は abort 済み `fetch()` を host fetch / HTTPS outcall 前に reject する。進行中 outcall の中断は未対応。

Streams は v1 対象外。`ReadableStream` / `WritableStream` / `TransformStream` は提供しない。

Cache API は `match` / `put` / `delete` と `caches.open` の subset。canister では stable memory KV に保存する。storage key は `cache:` prefix と JSON tuple `[cache_name,"GET",normalized_url]` で構造化し、cache 名 / URL 境界の衝突を避ける。`Cache-Control: max-age=N` は expiration として扱う。`cache.put()` は `Set-Cookie` response を拒否する。Range、conditional request は対象外。

canister `quickjs-ic` backend は crypto callback を Rust 側に登録する。`getRandomValues` は `http_request_update` 開始時に取得した `raw_rand` seed を使う。対象は integer TypedArray。`byteLength` 分を埋め、65,536 bytes 超過と TypedArray 以外は error。

canister `quickjs-ic` backend は `http_request_update` 開始時の IC time を `ic.time()` に注入する。host runtime は local placeholder を使う。

Rust-JS bridge DTO は number を number として扱う。request dispatch body、response body、host fetch request body、host fetch response body は `number[]` を JSON 化した private wire shape で渡す。JS 側では `number[]` を `Uint8Array` に戻す。

## URL Handling

canister から渡る path-only URL は runtime 内で仮 origin に正規化する。

```text
/users/123?q=test -> https://ic-edge.local/users/123?q=test
```

## Canister Boundary

`ic-edge-canister` は CDK 用 DTO を公開する。

- `CdkHttpRequest`
- `CdkHttpResponse`
- `handle_cdk_http`
- `handle_cdk_http_async`

`ic-edge-canister` は `wasm32-unknown-unknown` build 済み。

`examples/canister-template` は `http_request` / `http_request_update` と bundle upload 入口を持つ。

`icp.yaml` は `scripts/build_canister_backend_wasm.sh` を使う。script は `wasi_snapshot_preview1.*` import を stub 化してから `wasi2ic` で IC canister Wasm へ変換する。

v0.2 の QuickJS backend 調査では DFINITY [`ic-quickjs-demo`](https://github.com/dfinity/ic-quickjs-demo) を第一参照にする。比較結果は [`quickjs-backend-comparison.md`](quickjs-backend-comparison.md) に記録する。

## Bundle Upload

`ic-edge-pack` は local upload contract を持つ。

```bash
ic-edge upload dist/app.bundle.js --module app
ic-edge upload dist/app.bundle.js --module app --canister edge --environment local
```

`--canister` なしでは local `MemoryEdgeStore` へ保存する。`--canister` 指定時は `icp canister call <canister> upload_bundle --args-file ...` を実行する。

canister template 側は `upload_bundle(module, bytes)`、`bundle_size(module)`、`set_env(name, value)`、`env_names()`、`runtime_info()`、`runtime_history()`、`rollback_runtime(generation)`、direct smoke 用 `fetch_outcall(url)` を公開する。mutation API と `fetch_outcall(url)` は controller 限定。
`runtime_info()` は runtime backend 名と cache invalidation 用 generation を返す。generation は `upload_bundle` と `set_env` 成功時に増加し、canister upgrade では維持される。

`quickjs-ic` backend は canister global の generation-scoped runtime cache を使う。同じ generation の連続 `http_request_update` は既存 QuickJS runtime を再利用し、`upload_bundle()`、`set_env()`、`rollback_runtime()` 後は generation mismatch により runtime を再生成する。

`rollback_runtime(generation)` は直近 5 世代の bundle/env snapshot から復元し、復元後に新しい generation を発行する。

OpenAI / Upstash のような HTTPS outcall 用 secret は bundle に埋め込まず、deploy 後に注入する。

```bash
icp canister call edge set_env '("OPENAI_API_KEY", "sk-...")' --environment local
icp canister call edge set_env '("UPSTASH_REDIS_REST_URL", "https://...")' --environment local
icp canister call edge set_env '("UPSTASH_REDIS_REST_TOKEN", "...")' --environment local
```

`ic-edge-store` は `MemoryEdgeStore` と `StableEdgeStore` を持つ。`StableEdgeStore` は `ic-stable-structures 0.7.2` の `MemoryManager` と 2 つの `StableBTreeMap` で module/KV を分離する。

## Fetch Bridge

host runtime は `HostFetch` trait を持つ。

```rust
pub trait HostFetch {
    fn fetch(&mut self, request: Request) -> Result<Response>;
}
```

`QuickJsRuntime::install_fetch()` で JS `fetch()` から Rust handler を呼ぶ。ICP では同じ境界を HTTPS outcalls に接続する。

`ic-edge-canister` は HTTPS outcalls adapter を持つ。

- `https_outcall_fetch(request, "transform_strip_headers", max_response_bytes)`
- `build_https_outcall_args(...)`
- `transform_strip_headers(...)`
- `examples/canister-template::fetch_outcall(url)`

制約:

- URL は `https://` のみ。
- URL credentials、空 host、localhost、private / loopback / link-local / multicast / unspecified IP、metadata host は拒否。
- DNS 解決による private IP 判定は canister 内非決定性を避けるため行わない。
- method は `GET` / `POST` / `HEAD` のみ。
- `max_response_bytes` は既定 64 KiB。
- transform は consensus 用に response header を削除する。
