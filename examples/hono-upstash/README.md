# Hono with Upstash Redis

`@upstash/redis` HTTP client smoke。

bundle に Upstash secret は埋め込まない。`process.env.UPSTASH_REDIS_REST_URL` と `process.env.UPSTASH_REDIS_REST_TOKEN` から読む。
example は runtime smoke の決定性を優先して Upstash telemetry を無効化する。

host smoke では実 Upstash API は呼ばない。runtime の `HostFetch` mock が REST response を返す。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_upstash -- \
  examples/hono-upstash/dist/app.bundle.js
```
