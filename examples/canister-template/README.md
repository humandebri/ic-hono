# Canister Template

IC HTTP endpoint と bytecode upload 入口の最小形。
Cookbook は [`docs/cookbook.md`](../../docs/cookbook.md) に集約する。

`icp.yaml` は backend build script を使う。v1 は `quickjs-ic` のみを使い、`wasm32-wasip1` build 後に WASI import stub と `wasi2ic` 変換を行う。

- `http_request`
- `http_request_update`
- `upload_bytecode(module, bytes)` manifest なし raw upload reject stub
- `begin_bytecode_upload(module, total_bytes, manifest_json)`
- `append_bytecode_chunk(module, offset, bytes)`
- `commit_bytecode_upload(module)`
- `abort_bytecode_upload(module)`
- `bytecode_size(module)`
- `set_env(name, value)`
- `env_names()`
- `transform_strip_headers(args)`
- `fetch_outcall(url)` controller-only direct HTTPS outcall smoke helper
- `fetch_outcall_replicated(url)` controller-only replicated HTTPS outcall smoke helper

`ic-edge upload` は manifest 付き bytecode chunk upload を使う。`upload_bytecode` は manifest なし raw upload を拒否する。upload、`set_env`、`fetch_outcall`、`fetch_outcall_replicated` は controller 限定。`env_names` は secret 値を返さない。

```bash
icp network start -d
icp deploy edge --yes
cargo run -p ic-edge-pack --bin ic-edge -- upload ../hono-basic/dist/app.qjbc --canister edge --environment local
```

`quickjs-wasm` stub backend は削除済み。rollback は `runtime_history()` と `rollback_runtime(generation)` を使う。
