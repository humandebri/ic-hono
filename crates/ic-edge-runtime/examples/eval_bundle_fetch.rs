//! `crates/ic-edge-runtime/examples` evaluates a bundle with mock host fetch.
//! It validates the runtime fetch contract without external network access.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result};
use std::env;
use std::fs;

struct MockFetch;

impl HostFetch for MockFetch {
    fn fetch(&mut self, request: Request) -> Result<Response> {
        let mut headers = Headers::new();
        headers.set("content-type", "application/json".to_string())?;
        Response::new(
            200,
            headers,
            Body::from_bytes(format!(r#"{{"url":"{}"}}"#, request.url).into_bytes()),
        )
    }
}

fn main() {
    let bundle_path = env::args()
        .nth(1)
        .expect("usage: eval_bundle_fetch <bundle.js>");
    let source = fs::read_to_string(&bundle_path).expect("failed to read bundle");

    let mut runtime = QuickJsRuntime::new().expect("failed to create runtime");
    runtime
        .install_fetch(MockFetch)
        .expect("failed to install fetch");
    runtime
        .eval_module(&bundle_path, &source)
        .expect("failed to evaluate bundle");
    let response = runtime
        .call_app_fetch(Request::new(
            "GET".to_string(),
            "/github".to_string(),
            Headers::new(),
            Body::empty(),
        ))
        .expect("failed to call app.fetch");
    println!(
        "{}",
        response.body.text().expect("response body is not utf-8")
    );
}
