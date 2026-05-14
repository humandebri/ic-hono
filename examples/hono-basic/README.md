# Hono Basic

最初の互換性目標。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

```bash
npm install
npm run build
```

この repo example の `npm run build` は workspace の `cargo run -q -p ic-edge-pack --bin ic-edge -- pack ...` を呼ぶ。生成 app の `npm run build` は installed `ic-edge` を使う。

期待する canister 経路:

```text
IC HTTP request
-> Rust HttpRequest
-> JS Request
-> app.fetch(request)
-> JS Response
-> Rust HttpResponse
-> IC response
```
