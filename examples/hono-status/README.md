# Hono Status

Practical Hono example for an ICP canister runtime.

It serves an HTML status page, JSON health endpoints, stable incident storage through the Cache API, and replicated HTTPS checks through `fetch()`.

```bash
npm install
npm run build
cargo run -p ic-edge-pack --bin ic-edge -- upload dist/app.qjbc --canister edge --environment local
```

Gateway URL:

```bash
http://127.0.0.1:8000/?canisterId=<edge-canister-id>
```
