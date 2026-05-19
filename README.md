# Durable Edge Runtime on ICP

Run Hono and Worker-compatible Fetch/Core packages inside ICP canisters.

This is a **v1 preview** runtime contract. The crate semver is `0.2.0`; the `v1` label describes the supported API/product contract, not a `1.0.0` stability claim.

This repository builds an open-source QuickJS-based, Web Standards, canister-native durable runtime for the Internet Computer. It is a runtime kernel, not a managed app platform.

## What This Is

`ic-edge-workers` runs bundled Hono applications inside ICP canisters through a Worker-compatible Fetch/Core API subset.

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
- network access for the first `quickjs-wasm-sys` WASI SDK fetch, or `QUICKJS_WASM_SYS_WASI_SDK_PATH` pointing at a local WASI SDK

```bash
cargo test
scripts/package_smoke.sh
```

Create a Hono app:

```bash
cargo install --path crates/ic-edge-pack --bin ic-edge
cargo run -p ic-edge-pack --bin ic-edge -- init hono my-app
cd my-app
npm install
npm run build
```

Generated apps assume an installed `ic-edge` CLI. Repository examples are developer fixtures and their `npm run build` scripts call `cargo run -q -p ic-edge-pack --bin ic-edge -- pack ...` from this workspace.

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

Host smoke is green for Hono basic routes, JSON echo, params/query, CORS, zod, Cache API subset, JS fetch host bridge, OpenAI non-streaming mock bridge, Upstash mock bridge, and x402 V2 paid API mock flow.

Canister backend is quickjs-ic only: `wasm32-wasip1` build, WASI import stubbing, then `wasi2ic`. Local canister smoke covers chunked bundle upload, direct update, Gateway, HTTPS outcall, stable Cache, runtime generation cache, and rollback.

Cache API is canister-local stable storage, not a CDN cache. It supports `caches.default`, `caches.open`, `match`, `put`, `delete`, and `Cache-Control: max-age=N` expiration; `Set-Cookie` responses, Range, and conditional request behavior are out of scope for v1 preview.

Runtime limits are fixed: bundle 2 MiB, bundle upload chunk 512 KiB, inbound body 1 MiB, JS response body 1 MiB, cache entry 256 KiB, cache total 4 MiB, cache name 128 bytes, cache key 2 KiB, cache index 1024 entries / 128 KiB JSON, fetch response default 64 KiB / max 2 MiB, request fetch count 16, runtime history 5 generations, env names 64, env value 16 KiB.

## Local Cycle Measurements

Local cycle measurements use `icp canister status edge --environment local` before and after each update call, then subtract elapsed idle burn from `Idle cycles burned per day`. Query calls are excluded because this method measures canister balance debit. HTTP routes are measured through `http_request_update`, not the Gateway.

These numbers are local PocketIC measurements, not mainnet pricing. USD assumes `1T cycles = $1.33`.

Baseline app: `examples/hono-status`, 33,885 bytes raw / 13,125 bytes gzip.

| Operation | Repeats | Median cycles | Median USD | Avg USD | Min USD | Max USD |
|-----------|--------:|--------------:|-----------:|--------:|--------:|--------:|
| `abort_bundle_upload` missing | 5 | 9,145,219 | $0.000012 | $0.000012 | $0.000012 | $0.000012 |
| `set_env` replace 16B | 5 | 23,729,077 | $0.000032 | $0.000031 | $0.000030 | $0.000032 |
| `GET /api/health` cold after generation change | 3 | 106,426,761 | $0.000142 | $0.000142 | $0.000141 | $0.000142 |
| `GET /api/health` warm | 5 | 32,672,020 | $0.000043 | $0.000043 | $0.000043 | $0.000043 |
| `GET /api/incidents` | 5 | 38,896,393 | $0.000052 | $0.000052 | $0.000052 | $0.000052 |
| `GET /` HTML page | 5 | 158,018,986 | $0.000210 | $0.000210 | $0.000210 | $0.000211 |
| `GET /demo` Cache write | 5 | 47,080,866 | $0.000063 | $0.000063 | $0.000062 | $0.000063 |
| `POST /api/incidents` | 5 | 51,961,241 | $0.000069 | $0.000069 | $0.000069 | $0.000069 |
| `POST /api/incidents/:id/resolve` | 5 | 148,102,636 | $0.000197 | $0.000197 | $0.000197 | $0.000197 |
| `GET /api/check` warm + replicated fetch | 3 | 769,365,170 | $0.001023 | $0.001023 | $0.001023 | $0.001023 |
| direct `fetch_outcall` non-replicated | 3 | 746,783,624 | $0.000993 | $0.000993 | $0.000993 | $0.000993 |
| direct `fetch_outcall_replicated` | 3 | 746,808,084 | $0.000993 | $0.000993 | $0.000993 | $0.000993 |

Full suite app: `examples/hono-suite`, 119,560 bytes raw / 34,361 bytes gzip. It includes Hono middleware, zod validation, jose HS256 JWT sign/verify, Web Crypto SHA-256, Cache API state, audit log storage, SSR HTML, and replicated HTTPS fetch.

| Operation | Repeats | Median cycles | Median USD | Avg USD | Min USD | Max USD |
|-----------|--------:|--------------:|-----------:|--------:|--------:|--------:|
| `abort_bundle_upload` missing | 5 | 9,103,351 | $0.000012 | $0.000012 | $0.000012 | $0.000012 |
| `set_env` replace 16B | 5 | 75,606,532 | $0.000101 | $0.000091 | $0.000068 | $0.000101 |
| `GET /api/health` cold after generation change | 3 | 240,354,320 | $0.000320 | $0.000320 | $0.000319 | $0.000320 |
| `GET /api/health` warm | 5 | 45,929,244 | $0.000061 | $0.000061 | $0.000061 | $0.000061 |
| `GET /api/incidents` | 5 | 40,993,993 | $0.000055 | $0.000055 | $0.000054 | $0.000055 |
| `GET /` HTML page | 5 | 205,158,959 | $0.000273 | $0.000273 | $0.000270 | $0.000276 |
| `GET /demo` Cache write | 5 | 110,508,840 | $0.000147 | $0.000147 | $0.000136 | $0.000158 |
| `POST /api/incidents` | 5 | 149,243,137 | $0.000198 | $0.000199 | $0.000176 | $0.000221 |
| `POST /api/incidents/:id/resolve` | 5 | 595,254,814 | $0.000792 | $0.000792 | $0.000686 | $0.000899 |
| `GET /api/check` warm + replicated fetch | 3 | 954,397,936 | $0.001269 | $0.001269 | $0.001264 | $0.001275 |
| direct `fetch_outcall` non-replicated | 3 | 746,832,512 | $0.000993 | $0.000993 | $0.000993 | $0.000993 |
| direct `fetch_outcall_replicated` | 3 | 746,852,781 | $0.000993 | $0.000993 | $0.000993 | $0.000993 |

x402 paid API app: `examples/hono-x402-paid-api`, 298,651 bytes raw / 55,646 bytes gzip. It includes official x402 V2 custom server flow, endpoint-specific route catalog pricing, mock/HTTP facilitator switch, `ic.audit` receipt transparency log, replay rejection, payer hashing, and paid replicated HTTPS outcall.

Route catalog defaults: `GET /paid/report` costs `$0.001` via `X402_REPORT_PRICE`; `GET /paid/outcall` costs `$0.003` via `X402_OUTCALL_PRICE`. Payee resolution is route env (`X402_REPORT_PAY_TO` / `X402_OUTCALL_PAY_TO`), then shared `X402_PAY_TO`, then the demo default.

| Operation | Repeats | Median cycles | Median USD | Avg USD | Min USD | Max USD |
|-----------|--------:|--------------:|-----------:|--------:|--------:|--------:|
| `GET /free/catalog` | 3 | 54,567,042 | $0.000073 | $0.000186 | $0.000073 | $0.000414 |
| `GET /paid/report` unpaid | 3 | 62,867,345 | $0.000084 | $0.000084 | $0.000084 | $0.000084 |
| `GET /paid/report` paid | 3 | 151,836,330 | $0.000202 | $0.000202 | $0.000201 | $0.000202 |
| `GET /paid/report` replay rejected | 3 | 107,950,007 | $0.000144 | $0.000143 | $0.000143 | $0.000144 |
| `GET /receipts` | 3 | 249,256,691 | $0.000332 | $0.000331 | $0.000331 | $0.000332 |
| `GET /audit/root` | 3 | 22,812,779 | $0.000030 | $0.000030 | $0.000030 | $0.000030 |
| `GET /paid/outcall` paid replicated | 3 | 886,849,413 | $0.001180 | $0.001179 | $0.001179 | $0.001180 |

Cold and warm runtime calls are separated because generation changes invalidate the QuickJS runtime cache. Full historical samples are in [baseline local cycle evidence](docs/release-evidence/local-cycle-measurements-2026-05-16.md), [Hono suite local cycle evidence](docs/release-evidence/local-cycle-measurements-hono-suite-2026-05-16.md), and [Hono x402 paid API local cycle evidence](docs/release-evidence/local-cycle-measurements-hono-x402-paid-api-2026-05-19.md); those files may include pre-manifest raw upload measurements.

## Release Gates

Local release gate:

```bash
scripts/ci_local.sh
scripts/icp_local_smoke.sh
scripts/package_crates.sh
```

Public release gate:

```bash
scripts/ci_local.sh
scripts/icp_local_smoke.sh
scripts/package_crates.sh
IC_EDGE_PREFLIGHT_EVIDENCE=docs/release-evidence/mainnet-preflight-YYYY-MM-DD.md \
  IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh
```

Mainnet preflight does not deploy. Capture public release evidence under `docs/release-evidence/`.

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
