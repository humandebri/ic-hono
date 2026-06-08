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
    let args: Vec<String> = env::args().collect();
    let bundle_path = args
        .get(1)
        .expect("usage: eval_bundle_fetch <bundle.js> [method] [url] [body] [--show-response]");
    let method = args.get(2).cloned().unwrap_or_else(|| "GET".to_string());
    let url = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "/github".to_string());
    let body = args.get(4).cloned().unwrap_or_default();
    let show_response = args.iter().any(|arg| arg == "--show-response");
    let headers: Vec<(String, String)> = args
        .iter()
        .skip(5)
        .filter(|arg| *arg != "--show-response")
        .filter_map(|arg| arg.split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect();
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
            method,
            url,
            Headers::from_pairs(headers).expect("invalid request headers"),
            Body::from_bytes(body.into_bytes()),
        ))
        .expect("failed to call app.fetch");
    if show_response {
        println!("status: {}", response.status);
        for (name, value) in response.headers.entries() {
            println!("header: {name}: {value}");
        }
    }
    println!(
        "{}",
        response.body.text().expect("response body is not utf-8")
    );
}
