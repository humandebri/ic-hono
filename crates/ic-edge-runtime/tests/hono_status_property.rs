//! `hono-status` property tests exercise the bundled Hono app through QuickJS.
//! The bundle is built by `scripts/package_smoke.sh` before this test runs.

use ic_edge_runtime::{EdgeRuntime, HostFetch, QuickJsRuntime};
use ic_edge_web::{Body, Headers, Request, Response, Result as WebResult};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const BUNDLE_PATH: &str = "examples/hono-status/dist/app.bundle.js";
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

fn valid_incident_input() -> impl Strategy<Value = Value> {
    (
        "[A-Za-z0-9][A-Za-z0-9._-]{3,80}",
        prop::collection::vec(any::<char>(), 0..120)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
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

fn invalid_incident_input() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(json!(null)),
        Just(json!({})),
        Just(json!({ "title": "bad", "summary": "", "status": "identified", "severity": "minor" })),
        Just(
            json!({ "title": "Valid title", "summary": "", "status": "bad", "severity": "minor" })
        ),
        Just(
            json!({ "title": "Valid title", "summary": "", "status": "identified", "severity": "bad" })
        ),
        prop::collection::vec(any::<char>(), 501..540).prop_map(|chars| json!({
            "title": "Valid title",
            "summary": chars.into_iter().collect::<String>(),
            "status": "identified",
            "severity": "minor",
        })),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn valid_incidents_roundtrip_through_cache(payload in valid_incident_input()) {
        let Some(mut runtime) = runtime() else { return Ok(()) };

        let created = runtime
            .call_app_fetch(json_request("POST", "/api/incidents", payload.clone()))
            .unwrap();
        prop_assert_eq!(created.status, 201);

        let body = response_json(created);
        prop_assert_eq!(
            body["incident"]["title"].as_str(),
            Some(payload["title"].as_str().unwrap().trim())
        );
        prop_assert_eq!(
            body["incident"]["status"].as_str(),
            payload["status"].as_str()
        );
        prop_assert_eq!(
            body["incident"]["severity"].as_str(),
            payload["severity"].as_str()
        );

        let listed = response_json(
            runtime
                .call_app_fetch(request("GET", "/api/incidents", Vec::new()))
                .unwrap(),
        );
        prop_assert_eq!(listed["incidents"].as_array().unwrap().len(), 1);
        prop_assert_eq!(
            listed["incidents"][0]["title"].as_str(),
            body["incident"]["title"].as_str()
        );
    }

    #[test]
    fn invalid_incidents_are_rejected_without_state_change(payload in invalid_incident_input()) {
        let Some(mut runtime) = runtime() else { return Ok(()) };

        let response = runtime
            .call_app_fetch(json_request("POST", "/api/incidents", payload))
            .unwrap();
        prop_assert_eq!(response.status, 400);

        let health = response_json(
            runtime
                .call_app_fetch(request("GET", "/api/health", Vec::new()))
                .unwrap(),
        );
        prop_assert_eq!(health["incidentCount"].as_u64(), Some(0));
        prop_assert_eq!(health["openIncidentCount"].as_u64(), Some(0));
    }

    #[test]
    fn check_endpoint_accepts_public_https_urls(path in "[A-Za-z0-9/_?=&.-]{0,80}") {
        let Some(mut runtime) = runtime() else { return Ok(()) };
        let target = format!("https://example.com/{path}");
        let url = format!("/api/check?url={target}");

        let response = runtime
            .call_app_fetch(request("GET", &url, Vec::new()))
            .unwrap();
        prop_assert_eq!(response.status, 200);

        let body = response_json(response);
        prop_assert_eq!(body["ok"].as_bool(), Some(true));
        prop_assert_eq!(body["status"].as_u64(), Some(200));
        prop_assert!(body["url"].as_str().unwrap().starts_with("https://example.com/"));
    }
}
