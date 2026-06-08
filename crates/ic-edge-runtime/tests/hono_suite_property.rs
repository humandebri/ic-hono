//! `hono-suite` property tests exercise the larger practical Hono app.
//! `scripts/package_smoke.sh` builds the bundle before running this test.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result as WebResult};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const BUNDLE_PATH: &str = "examples/hono-suite/dist/app.bundle.js";
const STATUSES: &[&str] = &["investigating", "identified", "monitoring", "resolved"];
const SEVERITIES: &[&str] = &["minor", "major", "critical"];

fn request(method: &str, url: &str, body: impl Into<Vec<u8>>) -> Request {
    Request::new(
        method.to_string(),
        url.to_string(),
        Headers::new(),
        Body::from_bytes(body.into()),
    )
}

fn json_request(method: &str, url: &str, value: Value) -> Request {
    let mut headers = Headers::new();
    headers
        .set("content-type", "application/json".to_string())
        .unwrap();
    Request::new(
        method.to_string(),
        url.to_string(),
        headers,
        Body::from_bytes(value.to_string().into_bytes()),
    )
}

fn runtime() -> Option<QuickJsRuntime> {
    let source = fs::read_to_string(bundle_path()?).ok()?;
    let mut runtime = QuickJsRuntime::new().ok()?;
    runtime.install_fetch(EchoFetch).ok()?;
    runtime.eval_module(BUNDLE_PATH, &source).ok()?;
    Some(runtime)
}

fn bundle_path() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)?
        .join(BUNDLE_PATH);
    path.exists().then_some(path)
}

fn response_json(response: Response) -> Value {
    serde_json::from_str(&response.body.text().unwrap()).unwrap()
}

fn valid_incident_input() -> impl Strategy<Value = Value> {
    (
        "[A-Za-z0-9][A-Za-z0-9._-]{3,80}",
        "[A-Za-z0-9 ._-]{0,120}",
        prop::sample::select(STATUSES),
        prop::sample::select(SEVERITIES),
    )
        .prop_map(|(title, summary, status, severity)| {
            json!({
                "title": title,
                "summary": summary,
                "status": status,
                "severity": severity,
            })
        })
}

fn invalid_check_url() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z0-9._/-]{0,60}".prop_map(|path| format!("http://example.com/{path}")),
        "[A-Za-z0-9._/-]{0,60}".prop_map(|path| format!("https://localhost/{path}")),
        "[A-Za-z0-9._/-]{0,60}".prop_map(|path| format!("https://127.0.0.1/{path}")),
        "[A-Za-z0-9._/-]{0,60}".prop_map(|path| format!("https://user:pass@example.com/{path}")),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn incident_report_and_session_stay_consistent(payload in valid_incident_input()) {
        let Some(mut runtime) = runtime() else { return Ok(()) };

        let created = runtime
            .call_app_fetch(json_request("POST", "/api/incidents", payload))
            .unwrap();
        prop_assert_eq!(created.status, 201);

        let report = response_json(
            runtime
                .call_app_fetch(request("GET", "/api/report", Vec::new()))
                .unwrap(),
        );
        prop_assert_eq!(report["total"].as_u64(), Some(1));
        prop_assert!(report["digest"].as_str().unwrap().len() == 64);

        let session = response_json(
            runtime
                .call_app_fetch(request("GET", "/api/session", Vec::new()))
                .unwrap(),
        );
        prop_assert_eq!(session["verified"]["scope"].as_str(), Some("status:read"));
        prop_assert_eq!(session["verified"]["openIncidentCount"].as_u64(), report["open"].as_u64());
    }

    #[test]
    fn invalid_check_urls_are_rejected(value in invalid_check_url()) {
        let Some(mut runtime) = runtime() else { return Ok(()) };
        let response = runtime
            .call_app_fetch(request("GET", format!("/api/check?url={value}").as_str(), Vec::new()))
            .unwrap();
        prop_assert_eq!(response.status, 400);
    }
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
