#![no_main]

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result as WebResult};
use libfuzzer_sys::fuzz_target;
use serde_json::json;
use std::path::PathBuf;
use std::sync::OnceLock;

const BUNDLE_PATH: &str = "examples/hono-status/dist/app.bundle.js";

fuzz_target!(|data: &[u8]| {
    let Some(source) = bundle_source() else {
        return;
    };
    let Ok(mut runtime) = QuickJsRuntime::new() else {
        return;
    };
    if runtime.install_fetch(EchoFetch).is_err() {
        return;
    }
    if runtime.eval_module(BUNDLE_PATH, source).is_err() {
        return;
    }

    let request = fuzz_request(data);
    let response = runtime.call_app_fetch(request).expect("hono-status request trapped");
    assert!(response.status < 500, "unexpected server error: {}", response.status);
    assert!(
        response.body.bytes().len() <= 1024 * 1024,
        "response body exceeded runtime limit"
    );
});

fn bundle_source() -> Option<&'static String> {
    static SOURCE: OnceLock<Option<String>> = OnceLock::new();
    SOURCE
        .get_or_init(|| std::fs::read_to_string(bundle_path()).ok())
        .as_ref()
}

fn bundle_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(BUNDLE_PATH)
}

fn fuzz_request(data: &[u8]) -> Request {
    let selector = data.first().copied().unwrap_or_default() % 7;
    let body = data.get(1..).unwrap_or_default();
    match selector {
        0 => request("GET", "/", Vec::new()),
        1 => request("GET", "/api/health", Vec::new()),
        2 => request("GET", "/api/incidents", Vec::new()),
        3 => json_request("/api/incidents", body),
        4 => request("POST", "/api/incidents/fuzz/resolve", Vec::new()),
        5 => request("GET", &format!("/api/check?url={}", lossy(body)), Vec::new()),
        _ => request("GET", &format!("/{}", lossy(body)), Vec::new()),
    }
}

fn request(method: &str, url: &str, body: Vec<u8>) -> Request {
    Request::new(
        method.to_string(),
        url.to_string(),
        Headers::new(),
        Body::from_bytes(body),
    )
}

fn json_request(url: &str, body: &[u8]) -> Request {
    let mut headers = Headers::new();
    headers
        .set("content-type", "application/json".to_string())
        .unwrap();
    Request::new(
        "POST".to_string(),
        url.to_string(),
        headers,
        Body::from_bytes(body.to_vec()),
    )
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|value| value.is_ascii_graphic())
        .take(160)
        .collect()
}

struct EchoFetch;

impl HostFetch for EchoFetch {
    fn fetch(&mut self, request: Request) -> WebResult<Response> {
        let mut headers = Headers::new();
        headers.set("content-type", "application/json".to_string())?;
        Response::new(
            200,
            headers,
            Body::from_bytes(json!({ "url": request.url }).to_string().into_bytes()),
        )
    }
}
