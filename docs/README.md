# Documentation

Durable Edge Runtime on ICP v1 preview の文書索引。現行仕様はこの索引から辿る文書を正とする。

## Read First

1. [Quickstart](quickstart.md): local smoke、local ICP flow、新規 app 作成。
2. [Runtime API](runtime-api.md): Rust traits、bundle contract、Web API subset、canister API。
3. [Compatibility Matrix](compatibility.md): package、Hono、Web Standards の検証状況。
4. [Limitations](limitations.md): 非対応 API、固定上限、backend 制約。

## Runtime Model

- [Security Model](security-model.md): trust boundary、secrets、network、Cache、rollback。
- [Determinism Model](determinism-model.md): query/update、time、random、transform。
- [HTTPS Outcalls Model](https-outcalls-model.md): outbound fetch flow、limits、evidence。
- [QuickJS Backend Comparison](quickjs-backend-comparison.md): DFINITY demo 比較、risk、replacement criteria。

## Release

- [Release Checklist](release-checklist.md): local release gate と product contract。
- [Release Audit](release-audit.json): machine-readable release evidence。

## Positioning

- [Juno Positioning](juno-positioning.md): Juno との位置付け。

## Examples

- [Hono Basic](../examples/hono-basic/README.md): basic route、JSON、params/query、CORS、Cache。
- [Hono Fetch](../examples/hono-fetch/README.md): JS `fetch()` bridge。
- [Hono with jose](../examples/hono-jose/README.md): HS256 sign/verify。
- [Hono with OpenAI](../examples/hono-openai/README.md): non-streaming OpenAI compatible request。
- [Hono with Upstash Redis](../examples/hono-upstash/README.md): Upstash REST request。
- [Hono with Zod](../examples/hono-zod/README.md): JSON validation。
- [Canister Template](../examples/canister-template/README.md): upload、env、runtime info、rollback。

## Maintenance

- API surface 変更時は [Runtime API](runtime-api.md)、[Compatibility Matrix](compatibility.md)、[Limitations](limitations.md) を同時更新する。
- security / secret / outbound network 変更時は [Security Model](security-model.md) と [HTTPS Outcalls Model](https-outcalls-model.md) を更新する。
- release gate 変更時は [Release Checklist](release-checklist.md) と [Release Audit](release-audit.json) を更新する。
