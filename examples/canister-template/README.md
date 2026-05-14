# Canister Template

IC HTTP endpoint と bundle upload 入口の最小形。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

`icp.yaml` は backend build script を使う。v1 は `quickjs-ic` のみを使い、`wasm32-wasip1` build 後に WASI import stub と `wasi2ic` 変換を行う。

- `http_request`
- `http_request_update`
- `upload_bundle(module, bytes)`
- `begin_bundle_upload(module, total_bytes)`
- `append_bundle_chunk(module, offset, bytes)`
- `commit_bundle_upload(module)`
- `abort_bundle_upload(module)`
- `bundle_size(module)`
- `set_env(name, value)`
- `env_names()`
- `transform_strip_headers(args)`
- `fetch_outcall(url)` controller-only direct HTTPS outcall smoke helper

`upload_bundle` は small/direct/debug 互換 API。CLI 標準経路は chunk upload。`upload_bundle`、chunk upload、`set_env`、`fetch_outcall` は controller 限定。`env_names` は secret 値を返さない。

```bash
icp network start -d
icp deploy edge --yes
cargo run -p ic-edge-pack --bin ic-edge -- upload ../hono-basic/dist/app.bundle.js --canister edge --environment local
```

`quickjs-wasm` stub backend は削除済み。rollback は `runtime_history()` と `rollback_runtime(generation)` を使う。
