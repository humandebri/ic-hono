//! `ic-edge-runtime` verifies Web API behavior exposed to bundled apps.
//! These tests cover v0.2 API additions without depending on private modules.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result as WebResult};

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
fn headers_support_iterator_helpers() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const headers = new Headers([
                  ['set-cookie', 'a=1'],
                  ['x-one', '1'],
                  ['set-cookie', 'b=2']
                ])
                return Response.json({
                  keys: Array.from(headers.keys()),
                  values: Array.from(headers.values()),
                  cookies: headers.getSetCookie(),
                  tag: Object.prototype.toString.call(headers)
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"keys":["set-cookie","x-one","set-cookie"],"values":["a=1","1","b=2"],"cookies":["a=1","b=2"],"tag":"[object Headers]"}"#
    );
}

#[test]
fn binary_body_roundtrips_through_array_buffer_and_clone() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async (req) => {
                const requestClone = req.clone()
                const requestBytes = Array.from(new Uint8Array(await req.arrayBuffer()))
                let secondRead = 'not attempted'
                try {
                  await req.text()
                } catch (error) {
                  secondRead = error.name
                }
                const response = new Response(new Uint8Array([255, 0, 128]))
                const responseClone = response.clone()
                const responseBytes = Array.from(new Uint8Array(await response.arrayBuffer()))
                let cloneAfterRead = 'not attempted'
                try {
                  response.clone()
                } catch (error) {
                  cloneAfterRead = error.name
                }
                return Response.json({
                  requestBytes,
                  requestCloneBytes: Array.from(new Uint8Array(await requestClone.arrayBuffer())),
                  requestUsed: req.bodyUsed,
                  responseBytes,
                  responseCloneBytes: Array.from(new Uint8Array(await responseClone.arrayBuffer())),
                  responseUsed: response.bodyUsed,
                  secondRead,
                  cloneAfterRead,
                  url: response.url,
                  redirected: response.redirected,
                  type: response.type
                })
            } }",
        )
        .unwrap();
    let response = runtime
        .call_app_fetch(req("POST", "/", &[0xff, 0x00, 0x80]))
        .unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"requestBytes":[255,0,128],"requestCloneBytes":[255,0,128],"requestUsed":true,"responseBytes":[255,0,128],"responseCloneBytes":[255,0,128],"responseUsed":true,"secondRead":"TypeError","cloneAfterRead":"TypeError","url":"","redirected":false,"type":"default"}"#
    );
}

#[test]
fn request_constructor_copies_request_and_allows_init_overrides() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const original = new Request('https://edge.test/a', {
                  method: 'POST',
                  headers: { 'x-one': '1' },
                  body: 'original'
                })
                const copied = new Request(original, {
                  method: 'PUT',
                  headers: { 'x-two': '2' },
                  body: 'override'
                })
                return Response.json({
                  url: copied.url,
                  method: copied.method,
                  one: copied.headers.get('x-one'),
                  two: copied.headers.get('x-two'),
                  body: await copied.text()
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"url":"https://edge.test/a","method":"PUT","one":null,"two":"2","body":"override"}"#
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
                params.append('space', 'again')
                const seen = []
                params.forEach((value, name) => seen.push(`${name}:${value}`))
                params.sort()
                params.delete('missing')
                return Response.json({
                  plus: url.searchParams.get('q'),
                  encoded: url.searchParams.get('encoded'),
                  query: params.toString(),
                  hasSpace: params.has('space'),
                  allSpace: params.getAll('space'),
                  seen,
                  form: form.get('name'),
                  entries: Array.from(form.entries())
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"plus":"a b","encoded":"a+b","query":"space=a%20b&space=again","hasSpace":true,"allSpace":["a b","again"],"seen":["space:a b","space:again"],"form":"edge","entries":[["name","edge"]]}"#
    );
}

#[test]
fn request_form_data_reads_urlencoded_body() {
    let mut headers = Headers::new();
    headers
        .set(
            "content-type",
            "application/x-www-form-urlencoded; charset=utf-8".to_string(),
        )
        .unwrap();
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async (req) => {
                const form = await req.formData()
                return Response.json({
                  name: form.get('name'),
                  role: form.get('role'),
                  used: req.bodyUsed
                })
            } }",
        )
        .unwrap();
    let response = runtime
        .call_app_fetch(Request::new(
            "POST".to_string(),
            "/".to_string(),
            headers,
            Body::from_bytes(b"name=ic+edge&role=runtime".to_vec()),
        ))
        .unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"name":"ic edge","role":"runtime","used":true}"#
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

#[test]
fn cache_api_expires_max_age_zero_and_keeps_entries_without_ttl() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                await caches.default.put('https://cache.test/expired', new Response('expired', {
                  headers: { 'cache-control': 'max-age=0' }
                }))
                await caches.default.put('https://cache.test/persist', new Response('persist'))
                const expired = await caches.default.match('https://cache.test/expired')
                const persist = await caches.default.match('https://cache.test/persist')
                return Response.json({
                  expired: expired === undefined,
                  persist: await persist.text()
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"expired":true,"persist":"persist"}"#
    );
}

struct EchoFetch;

impl HostFetch for EchoFetch {
    fn fetch(&mut self, request: Request) -> WebResult<Response> {
        let bytes = request
            .body
            .bytes()
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        Response::text(format!("[{bytes}]"))
    }
}

#[test]
fn fetch_post_body_keeps_non_utf8_bytes() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime.install_fetch(EchoFetch).unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const response = await fetch('https://api.example.test/echo', {
                  method: 'POST',
                  body: new Uint8Array([255, 0, 128])
                })
                return Response.json({ echoed: await response.text() })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(response.body.text().unwrap(), r#"{"echoed":"[255,0,128]"}"#);
}

#[test]
fn cache_api_keeps_structured_keys_separate() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_app = { fetch: async () => {
                const first = await caches.open('a')
                const second = await caches.open('a\\nGET\\nhttps://cache.test/b')
                await first.put('https://cache.test/b\\nGET\\nhttps://cache.test/c', new Response('first'))
                await second.put('https://cache.test/c', new Response('second'))
                const firstHit = await first.match('https://cache.test/b\\nGET\\nhttps://cache.test/c')
                const secondHit = await second.match('https://cache.test/c')
                return Response.json({
                  first: await firstHit.text(),
                  second: await secondHit.text()
                })
            } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"first":"first","second":"second"}"#
    );
}

#[test]
fn get_random_values_uses_byte_length_and_validates_input() {
    let mut runtime = QuickJsRuntime::new().unwrap();
    runtime
        .eval_module(
            "app",
            "globalThis.__ic_edge_crypto_random = (length) => {
                return JSON.stringify(Array.from({ length }, (_, index) => index + 1))
              }
              globalThis.__ic_edge_app = { fetch: async () => {
                const values = new Uint32Array(2)
                crypto.getRandomValues(values)
                let typeError = ''
                try {
                  crypto.getRandomValues({ length: 1 })
                } catch (error) {
                  typeError = error.name
                }
                let limitError = ''
                try {
                  crypto.getRandomValues(new Uint8Array(65537))
                } catch (error) {
                  limitError = error.message
                }
                return Response.json({
                  bytes: Array.from(new Uint8Array(values.buffer)),
                  typeError,
                  limitError
                })
              } }",
        )
        .unwrap();
    let response = runtime.call_app_fetch(req("GET", "/", b"")).unwrap();
    assert_eq!(
        response.body.text().unwrap(),
        r#"{"bytes":[1,2,3,4,5,6,7,8],"typeError":"TypeError","limitError":"crypto.getRandomValues exceeds 65536 bytes"}"#
    );
}
