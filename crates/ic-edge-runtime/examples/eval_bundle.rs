//! `crates/ic-edge-runtime/examples` evaluates a local bundle file.
//! It is a smoke tool for the one-bundle MVP path.

use ic_edge_runtime::{EdgeRuntime, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bundle_path = args
        .get(1)
        .expect("usage: eval_bundle <bundle.js> [method] [url] [body] [--show-response]");
    let method = args.get(2).cloned().unwrap_or_else(|| "GET".to_string());
    let url = args.get(3).cloned().unwrap_or_else(|| "/".to_string());
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
    if response.status >= 500 {
        if let Some(error) = runtime
            .take_console_error()
            .expect("failed to read console error")
        {
            eprintln!("{error}");
        }
    }
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
