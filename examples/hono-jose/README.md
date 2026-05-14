# Hono with jose

`jose` HS256 smoke。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

現在の `crypto.subtle` 対応範囲は raw HMAC-SHA-256 のみ。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-jose/dist/app.bundle.js GET /jwt --show-response
```

この repo example の `npm run build` は workspace の `cargo run -q -p ic-edge-pack --bin ic-edge -- pack ...` を呼ぶ。生成 app の `npm run build` は installed `ic-edge` を使う。
