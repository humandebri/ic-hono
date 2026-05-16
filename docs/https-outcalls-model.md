# HTTPS Outcalls Model

## Scope

HTTPS outcall は JS external `fetch()` 専用。Inbound HTTP request 処理には使わない。
Hono を ICP 上で受ける inbound path には不要。外部 API を canister から呼ぶ場合だけ使う。

## Flow

```text
JS fetch("https://...")
-> ic.replicated option parse
-> runtime fetch queue
-> Rust AsyncHostFetch
-> ic_edge_canister::https_outcall_fetch
-> IC management canister http_request
-> transform_strip_headers
-> JS Promise resume
```

## Limits

- URL は `https://` 必須
- method は `GET` / `POST` / `HEAD`
- default `max_response_bytes` は 64 KiB
- IC の最大 response body は 2 MiB。明示 `max_response_bytes` が 2 MiB を超える場合は reject する。
- 未指定の replication mode は非複製。複製が必要な場合は `fetch(url, { ic: { replicated: true } })` を使う。
- 非複製 outcall は実験的で、複製 outcall より完全性保証が弱い。
- cycles cost は request / response size に依存
- timeout / upstream failure は JS fetch rejection に map する
- non-idempotent POST は複製時だけでなく network retry でも duplicate request risk がある

## Current Evidence

Host smoke は JS `fetch()` -> Rust `HostFetch` mock bridge を検証済み。

`scripts/icp_local_smoke.sh` は controller identity から `fetch_outcall("https://example.com/")` と `fetch_outcall_replicated("https://example.com/")` を呼び、status 200 を確認する。これは `ic_edge_canister::https_outcall_fetch`、management canister `http_request`、`transform_strip_headers` の実 canister evidence。

同じ smoke で Hono fetch example も direct update 経路を通過した。JS `fetch()` -> runtime queue -> HTTPS outcall -> Promise resume の end-to-end evidence は取得済み。

OpenAI example は OpenAI 公式 Responses API docs の `responses.create({ model, input })` / `response.output_text` 形に合わせる。model は `OPENAI_MODEL` で差し替え可能。実 API smoke は user-provided secret が必要なため optional。
