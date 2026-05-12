# Hono with jose

`jose` HS256 smoke。

現在の `crypto.subtle` 対応範囲は raw HMAC-SHA-256 のみ。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-jose/dist/app.bundle.js GET /jwt --show-response
```
