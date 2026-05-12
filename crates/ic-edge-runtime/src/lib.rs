//! `crates/ic-edge-runtime` owns app execution.
//! The current interface lets the canister host call a future QuickJS-backed app.

#[cfg(not(target_arch = "wasm32"))]
mod crypto_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod crypto_polyfill;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod fetch_queue_polyfill;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod json_polyfill;
#[cfg(not(target_arch = "wasm32"))]
mod quickjs;
#[cfg(not(target_arch = "wasm32"))]
mod quickjs_cache_host;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod quickjs_wasm;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod quickjs_wasm_cache;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod quickjs_wasm_crypto;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod quickjs_wasm_types;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod web_cache_polyfill;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod web_dispatch_polyfill;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod web_polyfill;
#[cfg(any(not(target_arch = "wasm32"), feature = "quickjs-ic"))]
mod web_url_polyfill;

use ic_edge_web::{Request, Response, Result};
#[cfg(not(target_arch = "wasm32"))]
pub use quickjs::{HostFetch, QuickJsRuntime};
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
pub use quickjs_wasm::QuickJsRuntime;
use std::future::Future;
use std::pin::Pin;

/// Synchronous runtime boundary for evaluating a bundle and calling `app.fetch`.
pub trait EdgeRuntime {
    /// Evaluates a named JavaScript module or bundle.
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()>;
    /// Calls the loaded app's `fetch(request)` entrypoint.
    fn call_app_fetch(&mut self, request: Request) -> Result<Response>;
}

/// Async runtime boundary for canister HTTPS outcalls.
pub trait AsyncEdgeRuntime {
    /// Evaluates a named JavaScript module or bundle.
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()>;
    /// Calls the loaded app's `fetch(request)` entrypoint and allows host async work.
    fn call_app_fetch<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + 'a>>;
}

/// Async host fetch implementation used by the wasm QuickJS runtime.
pub trait AsyncHostFetch {
    /// Performs an external fetch for a JavaScript `fetch()` request.
    fn fetch<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + 'a>>;
}

/// Persistence boundary backing the Worker Cache API subset.
pub trait CacheHost {
    /// Reads a serialized cached response.
    fn match_entry(&mut self, cache_name: &str, key: &str) -> Result<Option<String>>;
    /// Stores a serialized cached response.
    fn put_entry(&mut self, cache_name: &str, key: &str, response_json: &str) -> Result<()>;
    /// Deletes a cached response.
    fn delete_entry(&mut self, cache_name: &str, key: &str) -> Result<bool>;
}

/// Minimal test runtime used by canister bridge unit tests.
#[derive(Debug, Default)]
pub struct StaticRuntime {
    loaded_module: Option<(String, String)>,
}

impl StaticRuntime {
    /// Creates a static runtime returning `ok`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl EdgeRuntime for StaticRuntime {
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()> {
        self.loaded_module = Some((name.to_string(), source.to_string()));
        Ok(())
    }

    fn call_app_fetch(&mut self, _request: Request) -> Result<Response> {
        Response::text("ok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_edge_web::{Body, Headers};

    fn req(method: &str, url: &str, body: &[u8]) -> Request {
        Request::new(
            method.to_string(),
            url.to_string(),
            Headers::new(),
            Body::from_bytes(body.to_vec()),
        )
    }

    #[test]
    fn static_runtime_returns_hello_hono_response() {
        let mut runtime = StaticRuntime::new();
        runtime.eval_module("app", "export default app").unwrap();
        let res = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.body.text().unwrap(), "ok");
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_calls_global_fetch_contract() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async (req) => new Response(`${req.method} ${req.url} ${await req.text()}`) }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("POST", "/echo", b"ok")).unwrap();
        assert_eq!(
            res.body.text().unwrap(),
            "POST https://ic-edge.local/echo ok"
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_reads_response_status_and_headers() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: () => new Response('created', { status: 201, headers: [['x-edge', 'quickjs']] }) }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
        assert_eq!(res.status, 201);
        assert_eq!(res.headers.get("x-edge"), Some("quickjs".to_string()));
        assert_eq!(res.body.text().unwrap(), "created");
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_drains_async_fetch_response() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async (req) => Response.json({ method: req.method, body: await req.text() }) }",
            )
            .unwrap();
        let res = runtime
            .call_app_fetch(req("POST", "/echo", br#"{"ok":true}"#))
            .unwrap();
        assert_eq!(
            res.headers.get("content-type"),
            Some("application/json".to_string())
        );
        assert_eq!(
            res.body.text().unwrap(),
            r#"{"method":"POST","body":"{\"ok\":true}"}"#
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_uses_host_fetch() {
        struct EchoFetch;

        impl HostFetch for EchoFetch {
            fn fetch(&mut self, request: Request) -> Result<Response> {
                let mut headers = Headers::new();
                headers.set("content-type", "application/json".to_string())?;
                Response::new(
                    200,
                    headers,
                    Body::from_bytes(
                        format!(
                            r#"{{"url":"{}","body":"{}"}}"#,
                            request.url,
                            request.body.text()?
                        )
                        .into_bytes(),
                    ),
                )
            }
        }

        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime.install_fetch(EchoFetch).unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async () => {
                    const response = await fetch('https://api.example.test/echo', {
                      method: 'POST',
                      body: 'hello'
                    })
                    return Response.json(await response.json())
                } }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
        assert_eq!(
            res.body.text().unwrap(),
            r#"{"url":"https://api.example.test/echo","body":"hello"}"#
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_maps_console_and_async_exceptions() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async () => {
                    console.error('before-fail')
                    throw new TypeError('boom')
                } }",
            )
            .unwrap();
        let error = runtime.call_app_fetch(req("GET", "/", b"")).unwrap_err();
        assert!(format!("{error:?}").contains("TypeError: boom"));
        assert_eq!(
            runtime.take_console_error().unwrap(),
            Some("before-fail".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_installs_blob_array_buffer_and_ic_binding() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async (req) => {
                    const blob = new Blob(['edge'])
                    const bytes = new Uint8Array(await req.arrayBuffer())
                    return Response.json({
                      blob: await blob.text(),
                      byteLength: bytes.byteLength,
                      caller: ic.caller(),
                      canisterId: ic.canisterId()
                    })
                } }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("POST", "/", b"abc")).unwrap();
        assert_eq!(
            res.body.text().unwrap(),
            r#"{"blob":"edge","byteLength":3,"caller":"anonymous","canisterId":"local-canister"}"#
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_uses_utf8_text_coding() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async () => {
                    const input = 'edge ✓ 火'
                    const bytes = Array.from(new TextEncoder().encode(input))
                    const text = new TextDecoder().decode(new Uint8Array(bytes))
                    return Response.json({ bytes, text })
                } }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
        assert_eq!(
            res.body.text().unwrap(),
            r#"{"bytes":[101,100,103,101,32,226,156,147,32,231,129,171],"text":"edge ✓ 火"}"#
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn quickjs_runtime_uses_real_sha256_and_hmac() {
        let mut runtime = QuickJsRuntime::new().unwrap();
        runtime
            .eval_module(
                "app",
                "globalThis.__ic_edge_app = { fetch: async () => {
                    const encoder = new TextEncoder()
                    const key = await crypto.subtle.importKey(
                      'raw',
                      encoder.encode('secret'),
                      { name: 'HMAC', hash: 'SHA-256' },
                      false,
                      ['sign', 'verify']
                    )
                    const digest = Array.from(new Uint8Array(
                      await crypto.subtle.digest('SHA-256', encoder.encode('abc'))
                    ))
                    const signature = await crypto.subtle.sign('HMAC', key, encoder.encode('abc'))
                    const verified = await crypto.subtle.verify(
                      'HMAC',
                      key,
                      signature,
                      encoder.encode('abc')
                    )
                    const tampered = await crypto.subtle.verify(
                      'HMAC',
                      key,
                      new Uint8Array(signature).fill(0),
                      encoder.encode('abc')
                    )
                    return Response.json({ digest, verified, tampered })
                } }",
            )
            .unwrap();
        let res = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
        assert_eq!(
            res.body.text().unwrap(),
            r#"{"digest":[186,120,22,191,143,1,207,234,65,65,64,222,93,174,34,35,176,3,97,163,150,23,122,156,180,16,255,97,242,0,21,173],"verified":true,"tampered":false}"#
        );
    }
}
