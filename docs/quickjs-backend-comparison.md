# QuickJS Backend Comparison

v1 preview の backend 調査ノート。DFINITY [`ic-quickjs-demo`](https://github.com/dfinity/ic-quickjs-demo) を第一参照にし、現行実装との差分と保守リスクを固定する。

## DFINITY Demo

- 目的: Rust canister 内で QuickJS による JavaScript 実行を示す PoC。
- QuickJS binding: Bytecode Alliance Javy 由来の `quickjs-wasm-rs`。
- build path: `wasm32-wasi` binary を build し、`wasi2ic` で IC canister Wasm へ変換する。
- async model: `engine/mod.rs` と `engine/engine.js` が inter-canister call を JavaScript Promise として表現し、app JS は `async/await` で呼ぶ。
- endpoint model: Rust `lib.rs` 側で ic-cdk endpoint を公開し、`engine::execute()` に Candid -> JS args と JS result -> Candid reply の変換関数を渡す。
- production stance: README は PoC prototype と明記し、production code への verbatim copy は非推奨。

## v1 Backend

- QuickJS binding: `quickjs-wasm-rs`。
- build path: `wasm32-wasip1` canister build 後、`scripts/stub_wasm_wasi_imports.mjs` で WASI import を stub 化し、`wasi2ic` で IC canister Wasm へ変換する。
- async model: JS `fetch()` を queue に積み、Rust `AsyncHostFetch` が HTTPS outcall 後に Promise を resolve / reject する。
- endpoint model: `http_request_update` が uploaded IIFE bundle の `default.fetch(request)` を呼ぶ。
- Web API model: Worker互換 Core+Cache subset を polyfill と Rust callback で提供する。

## v0.2 Direction

- `ic-quickjs-demo` は copy せず、build path と async bridge の参照に限定する。
- 旧 `quickjs-wasm` backend は削除し、`quickjs-ic` を主 backend に昇格した。
- typed bridge は response status / fetch id の numeric DTO 化まで完了。小数や特殊 numeric は引き続き support subset 外。
- Hono basic / zod / jose / fetch / Cache / rollback smoke は `scripts/icp_local_smoke.sh` で確認する。
- minified bundle 評価後の IC context 注入は、単一 eval で wasm QuickJS の parser 状態不整合を起こしたため、context init / caller / canisterId を分割 eval する。

## Current Risk

- `quickjs-wasm-rs` は deprecated 扱いの binding。v1 preview では build path を `quickjs-ic` に一本化したが、binding 自体の長期保守リスクは残る。
- 置換は即時実施しない。現行 smoke と product contract を壊さずに置換可能な候補が出た時点で再評価する。

## Replacement Criteria

QuickJS binding / engine 置換候補は次の条件をすべて満たす必要がある。

- maintained な upstream がある。
- `wasm32-wasip1` build が安定する。
- `wasi2ic` 後の import audit で `env.*` / `wasi_snapshot_preview1.*` が残らない。
- Rust host callback、async fetch queue、Cache API callback、crypto callback を実装できる。
- Hono basic、zod、jose、JS fetch、Cache、rollback smoke が通る。
- `scripts/ci_local.sh` と `scripts/icp_local_smoke.sh` が追加 fallback なしで通る。
