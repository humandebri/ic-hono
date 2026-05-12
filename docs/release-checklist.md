# Release Checklist

v1 Worker互換 Core+Cache runtime の preview gate。
crate semver は `0.2.0`。`v1` は API/product contract 名として扱う。
機械可読版は [`release-audit.json`](release-audit.json)。

## Required Green Gates

- [x] `cargo fmt --all --check`
- [x] `cargo test`
- [x] `scripts/check_canister_interface.sh`
- [x] `scripts/check_release_audit.sh`
- [x] `scripts/package_smoke.sh`
- [x] `cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic`
- [x] `scripts/build_canister_backend_wasm.sh /tmp/ic_edge_runtime_import_check.wasm`
- [x] wasm import audit: no `env.*` or `wasi_snapshot_preview1.*`
- [x] `scripts/icp_local_smoke.sh`

Mainnet preflight は今回 scope 外。公開前 gate は local deploy smoke に固定する。

## Product Contract

| Area | Required evidence | Status |
| --- | --- | --- |
| quickjs-ic backend | `wasm32-wasip1` + WASI stub + `wasi2ic` | green |
| Worker Core subset | runtime unit tests and compatibility matrix | green |
| Cache API subset | host Cache tests and canister stable Cache smoke | green |
| Streams exclusion | runtime docs and compatibility matrix | documented |
| fixed limits | shared constants and unit/smoke coverage | green |
| rollback | `runtime_history()` / `rollback_runtime()` tests and smoke | green |
| Hono examples | package smoke and canister smoke | green |
| external fetch | direct HTTPS outcall and JS fetch canister path | green |
| optional real APIs | OpenAI / Upstash with user secrets | non-blocking |
| mainnet preflight | scope 外。local deploy smoke を正式 gate とする | not a gate |
| semver | crate/template package version `0.2.0`; v1 is product contract | documented |

## Wording Guard

Avoid:

- Cloudflare Workers の完全互換
- Node / Bun full compatibility
- npm full compatibility
- managed platform claims

Use:

- Worker-compatible Core+Cache subset
- Fetch/Core API subset
- quickjs-ic canister runtime
- canister-local stable Cache
