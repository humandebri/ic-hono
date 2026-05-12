# Durable Edge Runtime on ICP 計画書

Archive note: 現行仕様は [`docs/README.md`](../README.md) から辿る文書を正とする。この文書は設計背景と初期計画の記録。

この文書は設計計画。release 可否の根拠は
[`docs/release-checklist.md`](../release-checklist.md) の実 evidence を使う。

## 目的

Hono / Edge 向け npm ライブラリを canister 上で動かす。

作るものは Node/Bun ではなく、次の runtime kernel とする。

```text
QuickJS + Web Standards API + canister bindings + package loader
```

Cloudflare Workers 風の実行環境を ICP canister に載せる。Juno と被らないため、platform ではなく runtime kernel に絞る。

## 対象外

- full Node.js / Bun 互換
- `net` / `tls` / raw `dns`
- `child_process` / `worker_threads` / `cluster`
- native addons
- browser DOM
- canister 内 `npm install`
- Juno 風の app platform / auth / storage / hosting

## 最初のゴール

まず次の Hono app を canister で動かす。

```ts
import { Hono } from 'hono'
const app = new Hono()
app.get('/', (c) => c.text('ok'))
app.post('/echo', async (c) => {
  return c.json(await c.req.json())
})
export default app
```

変換経路:

```text
IC HTTP request
-> Rust HttpRequest
-> JS Request
-> app.fetch(request)
-> JS Response
-> Rust HttpResponse
-> IC response
```

## 全体アーキテクチャ

```text
User TS App
  -> JS Runtime Layer
  -> Web API Polyfill Layer
  -> Canister Binding Layer
  -> Rust Canister Host
```

Web API Polyfill Layer は `Request` / `Response` / `Headers` / `URL` / body API を提供する。Canister Binding Layer は HTTPS outcalls / time / caller / canister id / KV を提供する。

既存 VFS は high-level content store 寄りのため、runtime substrate として使うには lower layer が必要。

## crate 分割案

```text
crates/ic-edge-runtime
crates/ic-edge-web
crates/ic-edge-loader
crates/ic-edge-canister
crates/ic-edge-store
crates/ic-edge-pack
examples/hono-basic
examples/hono-json-api
examples/hono-with-zod
```

| crate | 責務 |
| --- | --- |
| `ic-edge-runtime` | QuickJS context、JS eval、promise job queue、exception mapping、`globalThis`、Rust <-> JS value bridge、`console` |
| `ic-edge-web` | `Request`、`Response`、`Headers`、`URL`、`URLSearchParams`、body API、`fetch`、`crypto.getRandomValues` |
| `ic-edge-loader` | ESM module registry、bundle manifest、package manifest、dynamic import 最低限 |
| `ic-edge-canister` | `http_request`、`http_request_update`、IC request/response bridge、HTTPS outcalls、time、caller、canister id、upgrade hooks |
| `ic-edge-store` | bundle store、module source store、asset store、small KV、package cache |
| `ic-edge-pack` | TS bundle、dependency 解決、manifest 生成、upload、compatibility check |

## Web API 方針

v1 必須:

- `Request` / `Response` / `Headers`
- `URL` / `URLSearchParams`
- `TextEncoder` / `TextDecoder`
- `Blob`
- `FormData` 簡易版
- `fetch`
- `crypto.getRandomValues`

後回し:

- full `ReadableStream` / `WritableStream`
- `crypto.subtle`
- `File`
- `AbortSignal` 完全実装

Hono 最小実行では body は全読み切りでよい。

```ts
await req.text()
await req.json()
await req.arrayBuffer()
```

## loader 方針

v1 は canister 内で npm install しない。ローカルで bundle して、canister には bundle 済み JS を置く。

最初は `esbuild` か `rolldown` で 1 bundle にする。

```bash
ic-edge pack ./src/app.ts --out ./dist/app.bundle.js
```

module loader は価値検証後に厚くする。

## canister binding 方針

JS からは次のように見せる。

```ts
globalThis.ic = {
  caller(): string,
  time(): bigint,
  canisterId(): string,
}
```

Hono には `env` として渡す。`ic` と `kv` は bindings として扱う。

## store 方針

最初は汎用 `fs` を公開しない。

初期 trait は `get_module` / `put_module` / `get_kv` / `put_kv` に絞る。

## CLI 方針

```bash
ic-edge init hono my-app
ic-edge pack
ic-edge upload
icp deploy
```

v1 では `npm install` は開発者のローカル環境で行う。

## 実装フェーズ

| Phase | 目標 | 主な作業 |
| --- | --- | --- |
| 1. Hello Hono | `app.get('/', c => c.text('ok'))` | QuickJS context、bundle eval、`Request` / `Response` / `Headers`、`app.fetch()`、IC response bridge |
| 2. JSON API | `/echo` で JSON を返す | request body bridge、`.text()`、`.json()`、content-type、UTF-8 |
| 3. Router | params / query を読む | `URL`、`URLSearchParams`、path 正規化、query bridge |
| 4. Middleware | Hono middleware を通す | `cors`、`logger`、`bearerAuth`、`prettyJSON`、`validator`、header API、status code、`console.log` |
| 5. 外部 fetch | JS `fetch` を HTTPS outcall に bridge | method、headers、body、response size limit、transform context、error mapping |
| 6. package set v1 | npm package 互換性検証 | `hono`、`zod`、`jose`、`openai`、`@upstash/redis` |

ICP の HTTPS outcalls は通常の Node fetch と異なるため、制約を docs に明記する。

## MVP の最低 API

Must: `globalThis`、`console`、`Promise`、`TextEncoder`、`TextDecoder`、`URL`、`URLSearchParams`、`Headers`、`Request`、`Response`、`fetch`、`crypto.getRandomValues`。

Should: `Blob`、`FormData`、`AbortController`、`setTimeout`、`clearTimeout`、`process.env` read-only、`Buffer`。

Later: `ReadableStream`、`WritableStream`、`TransformStream`、`crypto.subtle`、`File`、CommonJS bridge、`fs/promises` read-only。

## Repository 構成案

```text
ic-edge/
  crates/
    ic-edge-runtime/
    ic-edge-web/
    ic-edge-loader/
    ic-edge-canister/
    ic-edge-store/
    ic-edge-pack/
  examples/
    hono-basic/
    hono-json/
    hono-zod/
    hono-fetch/
  tests/compatibility/
  docs/
    compatibility.md
    runtime-api.md
    limitations.md
    juno-positioning.md
```

## 開発順チケット

1. QuickJS を canister build に組み込む
2. JS bundle eval と `console` を実装する
3. `Headers` / `Request` / `Response` を実装する
4. `app.fetch()` を Rust から呼ぶ
5. IC HTTP request / response bridge を実装する
6. Hono hello world を green にする
7. body `.text()` / `.json()` を実装する
8. URL / URLSearchParams を実装する
9. route params / query を green にする
10. CORS middleware を green にする
11. CLI bundler と canister template を作る
12. external fetch を実装する
13. zod example を追加する
14. openai non-streaming example を追加する

## README の打ち出し

推奨:

> Run Hono and Edge-compatible npm packages inside ICP canisters.

避ける表現:

- Node / Bun 全面互換を示す表現
- npm 全体の完全互換を示す表現
- serverless platform
- Juno の置換を示す表現

使う表現:

- Edge-compatible
- Web Standards
- QuickJS-based
- canister-native
- durable runtime
- package-compatible subset

## 成功条件

v0.1 の成功条件は [`release-checklist.md`](../release-checklist.md) で管理する。
host smoke だけでは release 完了扱いにしない。fresh `icp build` /
`icp deploy`、direct update、IC Gateway、実 HTTPS outcall の evidence が必要。

## 最短ルート

```text
QuickJS eval
-> bundled Hono app
-> Request/Response/Headers
-> app.fetch()
-> body text/json
-> URL/query
-> middleware
-> fetch outcall
-> zod/openai
```

MVP は Rust canister の HTTP endpoint から QuickJS 上の Hono `app.fetch()` を呼び、`Response` を返すこと。
