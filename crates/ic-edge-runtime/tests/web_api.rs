//! `ic-edge-runtime` verifies Web API behavior exposed to bundled apps.
//! These tests cover v0.2 API additions without depending on private modules.

use ic_edge_runtime::{EdgeRuntime, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request};

fn req(method: &str, url: &str, body: &[u8]) -> Request {
    Request::new(
        method.to_string(),
        url.to_string(),
        Headers::new(),
        Body::from_bytes(body.to_vec()),
    )
}

#[test]
fn headers_support_has_and_foreach() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const headers = new Headers([['x-one', '1'], ['x-two', '2']])
                const seen = []
                headers.forEach((value, name) => seen.push(`${name}:${value}`))
                return Response.json({
                  hasOne: headers.has('X-One'),
                  hasMissing: headers.has('x-missing'),
                  seen
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"hasOne":true,"hasMissing":false,"seen":["x-one:1","x-two:2"]}"#
    );
}

#[test]
fn binary_body_roundtrips_through_array_buffer() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async (req) => {
                const requestBytes = Array.from(new Uint8Array(await req.arrayBuffer()))
                const response = new Response(new Uint8Array([65, 66, 67]))
                const responseBytes = Array.from(new Uint8Array(await response.arrayBuffer()))
                return Response.json({ requestBytes, responseBytes, text: await response.text() })
            } }",
        )
        .unwrap();
    let response = runtime
        .call_app_fetch(req("POST", "/", &[1, 2, 3]))
        .unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"requestBytes":[1,2,3],"responseBytes":[65,66,67],"text":"ABC"}"#
    );
}

#[test]
fn aborted_fetch_rejects_before_host_fetch() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const controller = new AbortController()
                controller.abort()
                try {
                  await fetch('https://api.example.test', { signal: controller.signal })
                  return Response.json({ aborted: false })
                } catch (error) {
                  return Response.json({ aborted: true, message: error.message })
                }
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"aborted":true,"message":"This operation was aborted"}"#
    );
}

#[test]
fn url_search_params_and_form_data_cover_edge_subset() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const url = new URL('/search?q=a+b&encoded=a%2Bb')
                const params = new URLSearchParams([['space', 'a b']])
                const form = new FormData()
                form.append('name', 'edge')
                return Response.json({
                  plus: url.searchParams.get('q'),
                  encoded: url.searchParams.get('encoded'),
                  query: params.toString(),
                  form: form.get('name'),
                  entries: Array.from(form.entries())
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"plus":"a b","encoded":"a+b","query":"space=a%20b","form":"edge","entries":[["name","edge"]]}"#
    );
}

#[test]
fn cache_api_put_match_delete_and_open() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const named = await caches.open('named')
                await caches.default.put('https://cache.test/a', new Response('A'))
                await named.put(new Request('https://cache.test/b'), Response.json({ ok: true }))
                const a = await caches.default.match('https://cache.test/a')
                const b = await named.match('https://cache.test/b')
                const deleted = await named.delete('https://cache.test/b')
                const missing = await named.match('https://cache.test/b')
                return Response.json({
                  a: await a.text(),
                  b: await b.json(),
                  deleted,
                  missing: missing === undefined
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"a":"A","b":{"ok":true},"deleted":true,"missing":true}"#
    );
}

#[test]
fn cache_api_rejects_unsupported_put_inputs() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                try {
                  await caches.default.put(new Request('https://cache.test/a', { method: 'POST' }), new Response('A'))
                  return Response.json({ rejected: false })
                } catch (error) {
                  return Response.json({ rejected: true, message: error.message })
                }
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"rejected":true,"message":"cache.put only supports GET requests"}"#
    );
}
