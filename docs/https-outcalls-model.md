# HTTPS Outcalls Model

## Scope

HTTPS outcall は JS external `fetch()` 専用。Inbound HTTP request 処理には使わない。
Hono を ICP 上で受ける v0.1 中核には不要。外部 API を canister から呼ぶ場合だけ使う。

## Flow

```text
JS fetch("https://...")
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
- cycles cost は request / response size に依存
- timeout / upstream failure は JS fetch rejection に map する
- non-idempotent POST は duplicate request risk がある

## Current Evidence

Host smoke は JS `fetch()` -> Rust `HostFetch` mock bridge を検証済み。

2026-05-10 の fresh local canister smoke では `fetch_outcall("https://example.com/")` が status 200 を返した。これは `ic_edge_canister::https_outcall_fetch`、management canister `http_request`、`transform_strip_headers` の実 canister evidence。

同じ smoke で Hono fetch example も direct update 経路を通過した。JS `fetch()` -> runtime queue -> HTTPS outcall -> Promise resume の end-to-end evidence は取得済み。

OpenAI example は OpenAI 公式 Responses API docs の `responses.create({ model, input })` / `response.output_text` 形に合わせる。model は `OPENAI_MODEL` で差し替え可能。実 API smoke は user-provided secret が必要なため optional。
