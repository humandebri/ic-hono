//! `crates/ic-edge-runtime/examples` validates @upstash/redis over HostFetch.
//! It uses mock HTTP so no external Redis database is required.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result};
use std::env;
use std::fs;

struct UpstashMockFetch;

impl HostFetch for UpstashMockFetch {
    fn fetch(&mut self, request: Request) -> Result<Response> {
        let mut headers = Headers::new();
        headers.set("content-type", "application/json".to_string())?;

        let body = if request.url.ends_with("/pipeline") {
            r#"[{"result":"mocked-value"}]"#.to_string()
        } else if request.url.contains("/get/") {
            r#"{"result":"mocked-value"}"#.to_string()
        } else {
            format!(r#"{{"error":"unexpected url","url":"{}"}}"#, request.url)
        };

        Response::new(200, headers, Body::from_bytes(body.into_bytes()))
    }
}

fn main() {
    let bundle_path = env::args().nth(1).expect("usage: eval_upstash <bundle.js>");
    let source = fs::read_to_string(&bundle_path).expect("failed to read bundle");

    let mut runtime = QuickJsRuntime::new().expect("failed to create runtime");
    runtime
        .install_fetch(UpstashMockFetch)
        .expect("failed to install fetch");
    runtime
        .eval_module(
            "env",
            "process.env.UPSTASH_REDIS_REST_URL = 'https://example-upstash.test';
             process.env.UPSTASH_REDIS_REST_TOKEN = 'test-token';",
        )
        .expect("failed to set env");
    runtime
        .eval_module(&bundle_path, &source)
        .expect("failed to evaluate bundle");
    let response = runtime
        .call_app_fetch(Request::new(
            "GET".to_string(),
            "/kv/name".to_string(),
            Headers::new(),
            Body::empty(),
        ))
        .expect("failed to call app.fetch");
    if response.status >= 500 {
        if let Some(error) = runtime
            .take_console_error()
            .expect("failed to read console error")
        {
            eprintln!("{error}");
        }
    }
    println!(
        "{}",
        response.body.text().expect("response body is not utf-8")
    );
}
