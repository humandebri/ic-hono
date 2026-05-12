# Security Model

## Trust Boundary

User bundle は canister 内 QuickJS context で実行する。browser DOM、native addon、canister 内 npm install は対象外。

## Bundle Upload

`upload_bundle(module, bytes)` は controller 限定。bundle は stable memory に保存され、HTTP request 時に `app` module として評価される。

`set_env(name, value)` と direct smoke 用 `fetch_outcall(url)` も controller 限定。secret 値は query で返さない。`env_names()` は設定済みの名前だけを返す。

`scripts/icp_local_smoke.sh` は fresh deploy 後に `set_env("IC_EDGE_SMOKE", "ok")` が `Ok` を返し、後続 `env_names()` で `IC_EDGE_SMOKE` を確認する。

## Secrets

API key を bundle に埋め込まない。OpenAI / Upstash examples は `process.env` から secret を読む。host smoke は dummy env を注入する。

実 canister では HTTPS outcall 用 secret を `set_env` で注入する。bundle 評価前に template が stable store から `process.env` へ復元する。
`IC_EDGE_FULL_SMOKE=1` の preflight は不足 env 名だけを出し、secret 値は出力しない。

許可する env 名は `A-Z`、`0-9`、`_` のみ。値は JSON string として JS に注入するため、quote / 改行を含む値も構文を破壊しない。

## Network

Inbound HTTP は HTTPS outcall を使わない。IC Gateway から `http_request` / `http_request_update` に入り、JS `Request` へ変換する。

Outbound は JS `fetch("https://...")` のみ HTTPS outcall に接続する。plain HTTP、URL credentials、空 host、localhost、private / loopback / link-local / multicast / unspecified IP、metadata host は runtime 境界で拒否する。DNS 解決による private IP 判定は canister 内非決定性を避けるため行わない。

## Runtime Surface

公開 API は compatibility matrix に記録した subset のみ。Node core modules、filesystem、process mutation、DOM、streams full support は非対応。

`http_request_update` は `raw_rand().await` 完了後に generation / bundle / env を stable store から読む。dispatch 開始時点の runtime snapshot で request を完了し、`upload_bundle()` / `set_env()` / `rollback_runtime()` 完了後の新規 request は新 generation を読む。

## Cache

Worker Cache API subset は canister stable memory に保存する。global CDN cache ではない。`cache.put()` は GET key のみ許可し、`206`、`Vary: *`、`Set-Cookie` response を拒否する。cache name、key、entry count、serialized index に固定上限を置く。

## Limits

固定上限を超えた入力は拒否する。inbound body 超過は 413、controller API 超過は `Err(text)`、runtime / cache / fetch 超過は runtime error として扱う。

Cache 追加上限: name 128 bytes、key 2 KiB、index 1024 entries、index JSON 128 KiB。

## Rollback

`rollback_runtime(generation)` は controller 限定。直近 5 世代の bundle/env snapshot だけを復元対象にする。
