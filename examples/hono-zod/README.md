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
