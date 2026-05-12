//! `crates/ic-edge-runtime/examples` validates OpenAI SDK non-streaming flow.
//! It uses mock host fetch so no external API key or network is required.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result};
use std::env;
use std::fs;

struct OpenAiMockFetch;

impl HostFetch for OpenAiMockFetch {
    fn fetch(&mut self, request: Request) -> Result<Response> {
        let mut headers = Headers::new();
        headers.set("content-type", "application/json".to_string())?;
        headers.set("x-request-id", "req_test".to_string())?;

        let body = if request.url.ends_with("/responses") {
            r#"{"id":"resp_test","object":"response","output":[{"type":"message","content":[{"type":"output_text","text":"mocked"}]}]}"#.to_string()
        } else {
            format!(r#"{{"error":"unexpected url","url":"{}"}}"#, request.url)
        };

        Response::new(200, headers, Body::from_bytes(body.into_bytes()))
    }
}

fn main() {
    let bundle_path = env::args().nth(1).expect("usage: eval_openai <bundle.js>");
    let source = fs::read_to_string(&bundle_path).expect("failed to read bundle");

    let mut runtime = QuickJsRuntime::new().expect("failed to create runtime");
    runtime
        .install_fetch(OpenAiMockFetch)
        .expect("failed to install fetch");
    runtime
        .eval_module("env", "process.env.OPENAI_API_KEY = 'test-key'")
        .expect("failed to set env");
    runtime
        .eval_module(&bundle_path, &source)
        .expect("failed to evaluate bundle");
    let response = runtime
        .call_app_fetch(Request::new(
            "POST".to_string(),
            "/respond".to_string(),
            Headers::new(),
            Body::from_bytes(b"hello".to_vec()),
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
