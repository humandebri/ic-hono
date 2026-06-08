# Public API Contract

v1 preview の公開契約。crate semver は `0.2.0`。`v1` は product/API contract 名であり、Rust crate の `1.0.0` 安定化宣言ではない。

## Stable Surface

- Rust crates: `ic-edge-runtime`, `ic-edge-web`, `ic-edge-canister`, `ic-edge-store`, `ic-edge-pack`, `ic-edge-loader`, `ic-edge-bytecode-compiler`
- Runtime traits: `EdgeRuntime`, `AsyncEdgeRuntime`, `AsyncHostFetch`, `CacheHost`, `AuditHost`
- Runtime type: `QuickJsRuntime`
- Runtime fetch options: `HostFetchOptions`
- Web value types: `Request`, `Response`, `Headers`, `Body`, `Error`, `limits`
- Canister HTTP bridge: `CdkHttpRequest`, `CdkHttpResponse`, `handle_cdk_http`, `handle_cdk_http_async`
- HTTPS outcall bridge: `OutcallReplication`, `https_outcall_fetch`, `https_outcall_fetch_with_replication`, `build_https_outcall_args`, `build_https_outcall_args_with_replication`, `transform_strip_headers`
- CLI commands: `ic-edge init hono`, `ic-edge pack`, `ic-edge upload`
- Canister methods: `http_request`, `http_request_update`, `upload_bytecode` (manifest なし raw upload を拒否), `begin_bytecode_upload(module, total_bytes, manifest_json)`, `append_bytecode_chunk`, `commit_bytecode_upload`, `abort_bytecode_upload`, `set_env`, `env_names`, `bytecode_size`, `runtime_info`, `runtime_history`, `rollback_runtime`, `fetch_outcall`, `fetch_outcall_replicated`

## Web API Subset

- `Request` / `Response` / `Headers` / `URL` / `URLSearchParams`
- `Blob` and urlencoded `FormData`
- `fetch()` via host bridge or canister HTTPS outcalls。`ic.replicated` は outcall replication mode を指定する。
- `crypto.getRandomValues`, `crypto.subtle.digest` SHA-256, raw HMAC-SHA-256 `importKey` / `sign` / `verify`
- `Cache` / `caches.default` / `caches.open` with `match` / `put` / `delete` and `Cache-Control: max-age=N` expiration
- `process.env` read-only style injection
- `ic.caller()` / `ic.time()` / `ic.canisterId()` / `ic.audit`

## Out Of Contract

- Full Cloudflare Workers compatibility
- Streams
- DOM APIs
- Node.js core modules and native addons
- full ESM loader
- multipart `FormData`
- RSA / ECDSA / JWK full crypto
- managed platform bindings

## Change Rule

- Additive API changes are allowed when tests and docs update in the same change.
- Breaking changes require updating this file, `docs/runtime-api.md`, `docs/compatibility.md`, `docs/limitations.md`, and `docs/release-audit.json`.
- Public behavior changes must include focused unit tests and, when canister behavior changes, `scripts/icp_local_smoke.sh` coverage.
- Library publish readiness requires `scripts/package_crates.sh`.
