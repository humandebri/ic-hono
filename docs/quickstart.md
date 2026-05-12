# Quickstart

Durable Edge Runtime on ICP は Edge-compatible な Hono bundle を canister に upload して実行する。

## Local Package Smoke

```bash
cargo test
scripts/package_smoke.sh
```

検証内容:

- `ic-edge init hono`
- `ic-edge pack`
- Hono basic routing
- JSON echo
- params/query
- CORS header
- zod validation
- JS `fetch()` host bridge
- OpenAI / Upstash mock bridge

## Local CI Gate

```bash
scripts/ci_local.sh
```

`scripts/ci_local.sh` は canister deploy を除く CI 相当ゲートを実行する。
内容は format、unit test、canister interface audit、release audit、package smoke、`quickjs-ic` wasm build、post-link wasm import audit。

## Local ICP Flow

```bash
icp network start -d
icp deploy edge --yes
cargo run -p ic-edge-pack --bin ic-edge -- upload \
  examples/hono-basic/dist/app.bundle.js \
  --canister edge \
  --environment local
```

`scripts/icp_local_smoke.sh` は上記に加えて format、unit test、package smoke、wasm import audit を実行する。各 example は upload 前に `ic-edge pack` で再生成する。`icp build` / `icp deploy` の stdout/stderr は既定で `/tmp/ic-edge-smoke-logs` に保存する。保存先は `IC_EDGE_SMOKE_LOG_DIR` で変更できる。
成功時は `icp local smoke passed` とログ保存先を出力する。
既存 local network の update call が詰まる場合は `IC_EDGE_RESTART_NETWORK=1` を付け、script 内で local network を再起動してから fresh deploy する。

OpenAI / Upstash の実 API smoke は secret が必要なため、任意で明示的に有効化する。
`IC_EDGE_FULL_SMOKE=1` 付きで secret が不足する場合、script は build / deploy 前に不足 env 名だけを出して停止する。secret 値は出力しない。

```bash
IC_EDGE_FULL_SMOKE=1 \
OPENAI_API_KEY=sk-... \
OPENAI_MODEL=gpt-5.2 \
UPSTASH_REDIS_REST_URL=https://... \
UPSTASH_REDIS_REST_TOKEN=... \
scripts/icp_local_smoke.sh
```

2026-05-10 現在、`scripts/icp_local_smoke.sh` は local canister、IC Gateway、直接 HTTPS outcall、JS `fetch()` through HTTPS outcall で通過済み。
実 OpenAI / Upstash smoke は API key / REST token が必要なため任意確認。

## New App

```bash
cargo run -p ic-edge-pack --bin ic-edge -- init hono my-app
cd my-app
npm install
npm run build
```

生成 template は `src/app.ts` を持つ。既存ファイルは上書きしない。
