# Release Checklist

v1 Worker互換 Core+Cache runtime の preview gate。
crate semver は `0.2.0`。`v1` は API/product contract 名として扱う。
機械可読版は [`release-audit.json`](release-audit.json)。

## Local Release Gate

- [x] `cargo fmt --all --check`
- [x] `cargo test`
- [x] `scripts/check_api_contract.sh`
- [x] `scripts/check_compatibility_matrix.sh`
- [x] `scripts/check_canister_interface.sh`
- [x] `scripts/check_release_audit.sh`
- [x] `scripts/package_smoke.sh`
- [x] `cargo build --target wasm32-wasip1 -p ic-edge-canister-template --release --features quickjs-ic`
- [x] `scripts/build_canister_backend_wasm.sh /tmp/ic_edge_runtime_import_check.wasm`
- [x] wasm import audit: no `env.*` or `wasi_snapshot_preview1.*`
- [x] `scripts/package_crates.sh`
- [x] `scripts/icp_local_smoke.sh`

## Public Release Gate

- [ ] `IC_EDGE_PREFLIGHT_EVIDENCE=docs/release-evidence/mainnet-preflight-YYYY-MM-DD.md IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh`
- [ ] crates.io publish: `scripts/publish_crates.sh`

Mainnet preflight は deploy しない。`-e ic` の canister mapping、identity、cycles、canister status を確認する公開前 gate。証跡は `docs/release-evidence/mainnet-preflight-YYYY-MM-DD.md` に保存する。

crates.io publish は `ic-edge-web`、`ic-edge-loader`、`ic-edge-store`、`ic-edge-runtime`、`ic-edge-canister`、`ic-edge-pack` の順で実行する。公開済み version は上書き不可。未コミット差分を含めて公開する場合のみ `IC_EDGE_PUBLISH_ALLOW_DIRTY=1 scripts/publish_crates.sh` を使う。

## Product Contract

| Area | Required evidence | Status |
| --- | --- | --- |
| quickjs-ic backend | `wasm32-wasip1` + WASI stub + `wasi2ic` | green |
| Worker Core subset | runtime unit tests and compatibility matrix | green |
| Cache API subset | host Cache tests and canister stable Cache smoke | green |
| Streams exclusion | runtime docs and compatibility matrix | documented |
| fixed limits | shared constants and unit/smoke coverage | green |
| rollback | `runtime_history()` / `rollback_runtime()` tests and smoke | green |
| chunk upload | canister chunk API tests and CLI upload smoke | green |
| Hono examples | package smoke and canister smoke | green |
| external fetch | direct HTTPS outcall and JS fetch canister path | green |
| optional real APIs | OpenAI / Upstash with user secrets | non-blocking |
| public API contract | `docs/public-api-contract.md` and `scripts/check_api_contract.sh` | green |
| compatibility matrix CI | `.github/workflows/ci.yml` and `scripts/check_compatibility_matrix.sh` | green |
| mainnet preflight | `IC_EDGE_PREFLIGHT_EVIDENCE=docs/release-evidence/mainnet-preflight-YYYY-MM-DD.md IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh` | required gate |
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
