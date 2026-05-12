# Canister Template

IC HTTP endpoint と bundle upload 入口の最小形。

`icp.yaml` は backend build script を使う。v1 は `quickjs-ic` のみを使い、`wasm32-wasip1` build 後に WASI import stub と `wasi2ic` 変換を行う。

- `http_request`
- `http_request_update`
- `upload_bundle(module, bytes)`
- `bundle_size(module)`
- `set_env(name, value)`
- `env_names()`
- `transform_strip_headers(args)`
- `fetch_outcall(url)`

`upload_bundle` と `set_env` は controller 限定。`env_names` は secret 値を返さない。

```bash
icp network start -d
icp deploy edge --yes
cargo run -p ic-edge-pack --bin ic-edge -- upload ../hono-basic/dist/app.bundle.js --canister edge --environment local
```

`quickjs-wasm` stub backend は削除済み。rollback は `runtime_history()` と `rollback_runtime(generation)` を使う。
