# Durable Edge Runtime on ICP

Run Hono and Worker-compatible Fetch/Core packages inside ICP canisters.

This is a **v1 preview** runtime contract. The crate semver is `0.2.0`; the `v1` label describes the supported API/product contract, not a `1.0.0` stability claim.

This repository builds a QuickJS-based, Web Standards, canister-native durable runtime. It is a runtime kernel, not a managed app platform.

## Components

- `ic-edge-runtime`: quickjs-ic execution, Promise drain, exception mapping, console, Rust <-> JS bridge, Cache API callbacks
- `ic-edge-web`: Worker-compatible Fetch/Core subset and fixed v1 limits
- `ic-edge-canister`: IC HTTP bridge and HTTPS outcall adapter
- `ic-edge-store`: stable-memory backed module / KV store
- `ic-edge-pack`: local init / pack / upload CLI
- `examples/*`: Hono and package compatibility examples

## Quickstart

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

Cache API is canister-local stable storage, not a CDN cache. It supports `caches.default`, `caches.open`, `match`, `put`, and `delete`; TTL, Cache-Control evaluation, Range, and conditional request behavior are out of scope for v1 preview.

Runtime limits are fixed: bundle 2 MiB, inbound body 1 MiB, JS response body 1 MiB, cache entry 256 KiB, cache total 4 MiB, fetch response default 64 KiB / max 2 MiB, request fetch count 16, runtime history 5 generations, env names 64, env value 16 KiB.

## Docs

- [Quickstart](docs/quickstart.md)
- [Runtime API](docs/runtime-api.md)
- [Compatibility Matrix](docs/compatibility.md)
- [Limitations](docs/limitations.md)
- [Security Model](docs/security-model.md)
- [Determinism Model](docs/determinism-model.md)
- [HTTPS Outcalls Model](docs/https-outcalls-model.md)
- [Juno Positioning](docs/juno-positioning.md)
- [Release Checklist](docs/release-checklist.md)
