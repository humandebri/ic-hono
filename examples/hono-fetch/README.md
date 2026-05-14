# Hono Fetch

外部 `fetch` bridge 用 smoke。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

この example は bundle 生成だけを持つ。実 HTTP は canister HTTPS outcalls 側で接続する。

```bash
npm install
npm run build
```

この repo example の `npm run build` は workspace の `cargo run -q -p ic-edge-pack --bin ic-edge -- pack ...` を呼ぶ。生成 app の `npm run build` は installed `ic-edge` を使う。
