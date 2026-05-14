# Hono with Zod

`zod` package compatibility smoke。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-zod/dist/app.bundle.js POST /validate '{"name":"ic","count":1}'
```

この repo example の `npm run build` は workspace の `cargo run -q -p ic-edge-pack --bin ic-edge -- pack ...` を呼ぶ。生成 app の `npm run build` は installed `ic-edge` を使う。
