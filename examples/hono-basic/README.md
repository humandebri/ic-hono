# Hono Basic

最初の互換性目標。

```bash
npm install
npm run build
```

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

