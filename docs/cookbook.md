# Cookbook

Small recipes for the v1 preview Worker-compatible Core+Cache subset.

## Basic Hono App

Use `examples/hono-basic` for routing, JSON echo, params/query, CORS, byte bodies, Cache, and `ic.time()`.

```bash
cd examples/hono-basic
npm install
npm run build
```

Run the bundle on host QuickJS:

```bash
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-basic/dist/app.bundle.js GET /
```

## JSON Validation

Use `examples/hono-zod` for request JSON validation with `zod`.

```bash
cd examples/hono-zod
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-zod/dist/app.bundle.js POST /validate \
  '{"name":"ic","count":1}' 'content-type: application/json'
```

## External Fetch

Use `examples/hono-fetch` for JS `fetch()` through the host bridge and canister HTTPS outcalls.

```bash
cd examples/hono-fetch
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle_fetch -- \
  examples/hono-fetch/dist/app.bundle.js
```

## Secrets For HTTPS APIs

Do not embed secrets in bundles. Inject them after deployment with controller-only `set_env`.

```bash
icp canister call edge set_env '("OPENAI_API_KEY", "sk-...")' --environment local
icp canister call edge set_env '("UPSTASH_REDIS_REST_URL", "https://...")' --environment local
icp canister call edge set_env '("UPSTASH_REDIS_REST_TOKEN", "...")' --environment local
```

## Cache

Use `caches.default` for canister-local stable cache. `Cache-Control: max-age=N` expires cached responses.

```ts
await caches.default.put(
  'https://ic-edge.local/item',
  new Response('value', { headers: { 'cache-control': 'max-age=60' } }),
)
const hit = await caches.default.match('https://ic-edge.local/item')
```

## Local Canister Smoke

Run the full local canister gate:

```bash
scripts/icp_local_smoke.sh
```

Use `IC_EDGE_RESTART_NETWORK=1` when the existing local network is stuck.

## Mainnet Preflight

Run the non-mutating mainnet release gate before publishing:

```bash
IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh
```

The preflight builds locally, audits imports, checks identity/cycles on mainnet, and checks the configured `edge` canister status with `-e ic`.
