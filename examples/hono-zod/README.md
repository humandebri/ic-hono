# Hono with Zod

`zod` package compatibility smoke。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_bundle -- \
  examples/hono-zod/dist/app.bundle.js POST /validate '{"name":"ic","count":1}'
```

