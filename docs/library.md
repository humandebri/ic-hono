# Library Guide

`ic-hono` は v1 preview の Rust library workspace。crate semver は `0.2.0`。

## Crates

- `ic-edge-web`: runtime-neutral `Request` / `Response` / `Headers` / `Body` / limits。
- `ic-edge-runtime`: QuickJS runtime traits and host/canister runtime implementations。
- `ic-edge-canister`: IC HTTP bridge and HTTPS outcall adapter。
- `ic-edge-store`: memory and stable-memory stores for bundles and runtime KV。
- `ic-edge-loader`: bundle manifest type。
- `ic-edge-pack`: pack/upload library plus `ic-edge` CLI binary。

## Host Runtime Embed

```rust
use ic_edge_runtime::{EdgeRuntime, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request};

let mut runtime = QuickJsRuntime::new()?;
runtime.eval_module(
    "app",
    "globalThis.__ic_edge_app = { fetch: async () => new Response('ok') }",
)?;
let response = runtime.call_app_fetch(Request::new(
    "GET".to_string(),
    "/".to_string(),
    Headers::new(),
    Body::empty(),
))?;
assert_eq!(response.body.text()?, "ok");
# Ok::<(), ic_edge_web::Error>(())
```

## Canister Bridge

Use `ic-edge-canister` inside a canister template to convert CDK HTTP DTOs into runtime requests.

```rust
use ic_edge_canister::{handle_cdk_http_async, CdkHttpRequest, CdkHttpResponse};
use ic_edge_runtime::AsyncEdgeRuntime;

async fn handle<R: AsyncEdgeRuntime>(
    runtime: &mut R,
    request: CdkHttpRequest,
) -> ic_edge_web::Result<CdkHttpResponse> {
    handle_cdk_http_async(runtime, request).await
}
```

## Feature Flags

- `ic-edge-runtime/default`: host runtime on non-wasm targets.
- `ic-edge-runtime/quickjs-ic`: enables the wasm QuickJS backend for `wasm32 + quickjs-ic` canister builds.
- `ic-edge-canister-template/quickjs-ic`: forwards to `ic-edge-runtime/quickjs-ic`.

Host builds do not need `quickjs-ic`. Canister release builds use:

```bash
cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic
```

## Semver

- Rust crate semver remains `0.x` until the runtime contract is stable enough for `1.0.0`.
- `v1 preview` names the product/API contract, not a crate major version.
- Breaking public API changes must update `docs/public-api-contract.md`, `docs/runtime-api.md`, `docs/compatibility.md`, and `docs/release-audit.json`.
- Additive changes require focused tests and docs in the same change.

## Toolchain

- MSRV: Rust `1.85`.
- Required target: `wasm32-wasip1`.
- Node.js 20+ for example bundle builds.
- `wasi2ic`, `ic-wasm`, and `wabt`/`wasm-objdump` for canister build audits.

## Publish Check

Run:

```bash
scripts/package_crates.sh
```

The script runs tests, docs, verified `cargo package` for dependency-free crates, and package file-list checks for crates whose workspace dependencies are not yet on crates.io. On first release it runs `cargo publish --dry-run` for dependency-free crates only, because crates.io rejects package and publish verification for crates that depend on unpublished workspace crates. After `ic-edge-web`, `ic-edge-loader`, and `ic-edge-store` exist on crates.io, run:

```bash
IC_EDGE_FULL_PUBLISH_DRY_RUN=1 scripts/package_crates.sh
```
