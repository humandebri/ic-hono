# Determinism Model

## Query

`http_request` は query endpoint。現 template では JavaScript を評価せず、`upgrade = true` を返して gateway に `http_request_update` replay を要求する。

query 内で HTTPS outcall は行わない。JS が external `fetch()` を呼ぶ route も、通常 route も、現 v0.1 template では update 経路へ寄せる。

## Update

`http_request_update` は async endpoint。JS `fetch()` を queue 化し、Rust HTTPS outcall 完了後に Promise を resume する。

request runtime snapshot は dispatch 開始時点で固定する。template は async `raw_rand().await` を先に完了し、その後に generation / bundle / env を stable store から読むため、完了済みの bundle / env 更新は次の request に反映される。

## Time

`ic.time()` は runtime binding。host smoke では placeholder。canister `quickjs-ic` runtime では `http_request_update` 開始時の IC time を返す。

## Random

`crypto.getRandomValues` は API subset として提供する。現 template は JS 実行を update 経路へ寄せるため、query では random を実行しない。

canister `quickjs-ic` backend は `http_request_update` 開始時に IC management canister の `raw_rand` を呼び、得た seed と counter から request 内の同期 `getRandomValues` 用 bytes を派生する。

## Transform

HTTPS outcall response は transform で非決定 header を除去する。現実装は headers を全削除する。
