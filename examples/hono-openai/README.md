# Hono with OpenAI

OpenAI SDK non-streaming smoke。

bundle に API key は埋め込まない。`process.env.OPENAI_API_KEY` から読む。
model は `process.env.OPENAI_MODEL` で指定できる。未指定時は OpenAI docs の Responses API text generation 例に合わせて `gpt-5.2` を使う。

host smoke では実 API 呼び出しは行わない。runtime の `HostFetch` mock が `responses.create` の JSON を返す。

```bash
npm install
npm run build
cd ../..
cargo run -p ic-edge-runtime --example eval_openai -- \
  examples/hono-openai/dist/app.bundle.js
```
