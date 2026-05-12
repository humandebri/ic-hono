# Durable Edge Runtime on ICP

Run Hono and Worker-compatible Fetch/Core packages inside ICP canisters.

This is a **v1 preview** runtime contract. The crate semver is `0.2.0`; the `v1` label describes the supported API/product contract, not a `1.0.0` stability claim.

This repository builds an open-source QuickJS-based, Web Standards, canister-native durable runtime for the Internet Computer. It is a runtime kernel, not a managed app platform.

## What This Is

`ic-hono` runs bundled Hono applications inside ICP canisters through a Worker-compatible Fetch/Core API subset.

Supported in v1 preview:

- Hono `app.fetch()` request handling
- `Request` / `Response` / `Headers` / `URL` / `URLSearchParams`
- `fetch()` through ICP HTTPS outcalls
- `crypto.getRandomValues` and SHA-256 / HMAC-SHA-256 `crypto.subtle` subset
- canister-local stable Cache API subset
- stable bundle/env storage and runtime rollback

Not supported in v1 preview:

- Full Cloudflare Workers compatibility
- Streams (`ReadableStream`, `WritableStream`, `TransformStream`)
- DOM APIs
- Node.js core modules and native addons
- managed platform bindings

## Components

- `ic-edge-runtime`: quickjs-ic execution, Promise drain, exception mapping, console, Rust <-> JS bridge, Cache API callbacks
- `ic-edge-web`: Worker-compatible Fetch/Core subset and fixed v1 limits
- `ic-edge-canister`: IC HTTP bridge and HTTPS outcall adapter
- `ic-edge-store`: stable-memory backed module / KV store
- `ic-edge-pack`: local init / pack / upload CLI
- `examples/*`: Hono and package compatibility examples

## Library Use

Use `ic-edge-web` for runtime-neutral HTTP values, `ic-edge-runtime` to embed the QuickJS runtime, `ic-edge-canister` to map IC HTTP and HTTPS outcalls, `ic-edge-store` for memory/stable-memory storage, and `ic-edge-pack` for bundle packaging.

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
# Ok::<(), ic_edge_web::Error>(())
```

See [Library Guide](docs/library.md) for crate roles, feature flags, MSRV, semver policy, and publish checks.

## Quickstart

Prerequisites:

- Rust toolchain with `wasm32-wasip1`
- `icp` CLI
- `wasi2ic`
- Node.js / npm for example bundle builds

```bash
cargo test
scripts/package_smoke.sh
```

Create a Hono app:

```bash
cargo run -p ic-edge-pack --bin ic-edge -- init hono my-app
cd my-app
npm install
npm run build
```

Local ICP flow:

```bash
icp network start -d
icp deploy edge --yes
cargo run -p ic-edge-pack --bin ic-edge -- upload \
  examples/hono-basic/dist/app.bundle.js \
  --canister edge \
  --environment local
```

## Status

v1 preview targets a Worker-compatible Core+Cache subset, not a full Cloudflare Workers implementation. Streams, DOM, Node core modules, and managed platform bindings are unsupported.

Host smoke is green for Hono basic routes, JSON echo, params/query, CORS, zod, Cache API subset, JS fetch host bridge, OpenAI non-streaming mock bridge, and Upstash mock bridge.

Canister backend is quickjs-ic only: `wasm32-wasip1` build, WASI import stubbing, then `wasi2ic`. Local canister smoke covers direct update, Gateway, HTTPS outcall, stable Cache, runtime generation cache, and rollback.

Cache API is canister-local stable storage, not a CDN cache. It supports `caches.default`, `caches.open`, `match`, `put`, `delete`, and `Cache-Control: max-age=N` expiration; Range and conditional request behavior are out of scope for v1 preview.

Runtime limits are fixed: bundle 2 MiB, inbound body 1 MiB, JS response body 1 MiB, cache entry 256 KiB, cache total 4 MiB, fetch response default 64 KiB / max 2 MiB, request fetch count 16, runtime history 5 generations, env names 64, env value 16 KiB.

## Release Gates

Before publishing changes, run:

```bash
scripts/ci_local.sh
scripts/icp_local_smoke.sh
scripts/package_crates.sh
```

Mainnet preflight is not part of the v1 preview gate. Local deploy smoke is the required release gate.

## Docs

- [Docs Index](docs/README.md)
- [Quickstart](docs/quickstart.md)
- [Runtime API](docs/runtime-api.md)
- [Compatibility Matrix](docs/compatibility.md)
- [Limitations](docs/limitations.md)
- [Security Model](docs/security-model.md)
- [Release Checklist](docs/release-checklist.md)

See [Docs Index](docs/README.md) for runtime model, HTTPS outcalls, positioning, release evidence, and examples.

## Contributing

Issues and pull requests are welcome. Keep changes small, documented, and covered by focused tests. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Do not file public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).

## License

Licensed under the MIT license. See [LICENSE](LICENSE).
