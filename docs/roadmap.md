# Roadmap

Known limitations are tracked here as implementation issues. v1 preview remains a Worker-compatible Core+Cache subset.

## Release Blockers

- Mainnet preflight evidence must be captured under `docs/release-evidence/` before public release.
- Public API contract changes must update `docs/public-api-contract.md` and release audit evidence.

## Compatibility Work

- Streams: add `ReadableStream`, `WritableStream`, and `TransformStream` only after a focused design and compatibility test plan.
- ESM loader: replace IIFE-only bundle contract without breaking existing `ic-edge pack` users.
- multipart `FormData`: parse multipart request bodies without adding a large dependency.
- URL parser completeness: expand current minimal parser only when package compatibility requires it.
- Crypto: add RSA / ECDSA / JWK support after `jose` compatibility requirements are explicit.

## Operations Work

- Upgrade hooks: add explicit pre/post upgrade hooks once runtime state migration requirements are stable.
- Mainnet observability: document cycles, status, and failure triage after preflight is stable.
- Cookbook expansion: add recipes only for examples covered by host or canister smoke.
- QuickJS binding replacement: keep `quickjs-wasm-rs` for v1 preview, then evaluate candidates against the recorded replacement criteria. Completion evidence must record candidate name, verification commands, smoke results, and either defer reason or next action.
