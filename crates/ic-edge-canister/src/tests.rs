//! `crates/ic-edge-canister` verifies HTTP bridge and outcall mapping.
//! Tests stay separate so the production module remains small.

use super::*;
use ic_edge_runtime::{AsyncEdgeRuntime, StaticRuntime};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[test]
fn maps_ic_http_to_runtime_response() {
    let mut runtime = StaticRuntime::new();
    let response = handle_http(
        &mut runtime,
        IcHttpRequest {
            method: "GET".to_string(),
            url: "/".to_string(),
            headers: vec![("accept".to_string(), "text/plain".to_string())],
            body: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.body, b"ok");
}

#[test]
fn rejects_inbound_body_above_v1_limit_with_413() {
    let mut runtime = StaticRuntime::new();
    let response = handle_http(
        &mut runtime,
        IcHttpRequest {
            method: "POST".to_string(),
            url: "/".to_string(),
            headers: Vec::new(),
            body: vec![0; ic_edge_web::limits::MAX_INBOUND_BODY_BYTES + 1],
        },
    )
    .unwrap();
    assert_eq!(response.status_code, 413);
}

#[test]
fn maps_cdk_http_to_runtime_response() {
    let mut runtime = StaticRuntime::new();
    let response = handle_cdk_http(
        &mut runtime,
        CdkHttpRequest {
            method: "GET".to_string(),
            url: "/".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
            certificate_version: Some(2),
        },
    )
    .unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.upgrade, None);
    assert_eq!(response.body, b"ok");
}

#[test]
fn rejects_non_https_outcall_url() {
    let request = Request::new(
        "GET".to_string(),
        "http://example.com".to_string(),
        Headers::new(),
        Body::empty(),
    );
    let result = build_https_outcall_args(request, "transform_strip_headers", Some(1024));
    assert!(matches!(result, Err(Error::Runtime(_))));
}

#[test]
fn builds_get_outcall_with_default_response_limit() {
    let mut headers = Headers::new();
    headers
        .set("accept", "application/json".to_string())
        .unwrap();
    let request = Request::new(
        "GET".to_string(),
        "https://example.com/api".to_string(),
        headers,
        Body::from_bytes(b"ignored".to_vec()),
    );
    let args = build_https_outcall_args(request, "transform_strip_headers", None).unwrap();
    assert_eq!(args.url, "https://example.com/api");
    assert_eq!(args.max_response_bytes, Some(64 * 1024));
    assert!(matches!(args.method, HttpMethod::GET));
    assert_eq!(args.headers.len(), 1);
    assert_eq!(args.body, None);
    assert!(args.transform.is_none());
}

#[test]
fn builds_post_outcall_with_body_and_explicit_limit() {
    let request = Request::new(
        "POST".to_string(),
        "https://example.com/api".to_string(),
        Headers::new(),
        Body::from_bytes(br#"{"ok":true}"#.to_vec()),
    );
    let args = build_https_outcall_args(request, "transform_strip_headers", Some(4096)).unwrap();
    assert_eq!(args.max_response_bytes, Some(4096));
    assert!(matches!(args.method, HttpMethod::POST));
    assert_eq!(args.body, Some(br#"{"ok":true}"#.to_vec()));
}

#[test]
fn rejects_outcall_response_limit_above_ic_max() {
    let request = Request::new(
        "GET".to_string(),
        "https://example.com/api".to_string(),
        Headers::new(),
        Body::empty(),
    );
    let result = build_https_outcall_args(request, "transform_strip_headers", Some(2_097_153));
    assert!(matches!(result, Err(Error::Runtime(_))));
}

#[test]
fn rejects_unsupported_outcall_method() {
    let request = Request::new(
        "PUT".to_string(),
        "https://example.com/api".to_string(),
        Headers::new(),
        Body::empty(),
    );
    let result = build_https_outcall_args(request, "transform_strip_headers", None);
    assert!(matches!(result, Err(Error::Runtime(_))));
}

#[test]
fn strips_outcall_headers_in_transform() {
    let response = transform_strip_headers(TransformArgs {
        context: Vec::new(),
        response: HttpRequestResult {
            status: candid::Nat::from(200u64),
            headers: vec![HttpHeader {
                name: "date".to_string(),
                value: "now".to_string(),
            }],
            body: b"ok".to_vec(),
        },
    });
    assert!(response.headers.is_empty());
    assert_eq!(response.body, b"ok");
}

struct AsyncStaticRuntime(StaticRuntime);

impl AsyncEdgeRuntime for AsyncStaticRuntime {
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()> {
        EdgeRuntime::eval_module(&mut self.0, name, source)
    }

    fn call_app_fetch<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + 'a>> {
        Box::pin(async move { self.0.call_app_fetch(request) })
    }
}

#[test]
fn maps_async_cdk_http_to_runtime_response() {
    let mut runtime = AsyncStaticRuntime(StaticRuntime::new());
    let response = block_ready(handle_cdk_http_async(
        &mut runtime,
        CdkHttpRequest {
            method: "GET".to_string(),
            url: "/".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
            certificate_version: Some(2),
        },
    ))
    .unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.body, b"ok");
}

fn block_ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("future unexpectedly pending"),
    }
}
