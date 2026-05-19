//! `crates/ic-edge-canister` fuzzes HTTPS outcall argument construction.
//! Generated inputs protect URL parsing, host filtering, and method/body rules.

use ic_cdk::management_canister::HttpMethod;
use ic_edge_canister::build_https_outcall_args;
use ic_edge_web::{limits, Body, Headers, Request};
use proptest::prelude::*;

fn request(method: String, url: String, body: Vec<u8>) -> Request {
    Request::new(method, url, Headers::new(), Body::from_bytes(body))
}

fn public_https_url() -> impl Strategy<Value = String> {
    (
        "[a-z]{1,12}",
        prop::collection::vec(prop::sample::select(URL_PATH_BYTES), 0..80),
    )
        .prop_map(|(label, path)| {
            let path: String = path.into_iter().map(char::from).collect();
            format!("https://{label}.example.com/{path}")
        })
}

fn maybe_limit() -> impl Strategy<Value = Option<u64>> {
    prop_oneof![
        Just(None),
        (0u64..=limits::MAX_FETCH_RESPONSE_BYTES).prop_map(Some),
    ]
}

const URL_PATH_BYTES: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789/-._~?=&%";
const METHODS: &[&str] = &["GET", "get", "HEAD", "head", "POST", "post"];
const REJECTED_URLS: &[&str] = &[
    "https://localhost/api",
    "https://localhost./api",
    "https://metadata/api",
    "https://metadata.google.internal/computeMetadata/v1",
    "https://127.0.0.1/api",
    "https://10.0.0.1/api",
    "https://172.16.0.1/api",
    "https://192.168.0.1/api",
    "https://169.254.169.254/latest/meta-data",
    "https://[::1]/api",
    "https://[fe80::1]/api",
    "https://[fc00::1]/api",
    "https://user:pass@example.com/api",
];

proptest! {
    #[test]
    fn public_https_urls_build_outcall_args(
        method in prop::sample::select(METHODS),
        url in public_https_url(),
        body in prop::collection::vec(any::<u8>(), 0..256),
        limit in maybe_limit(),
    ) {
        let args = build_https_outcall_args(
            request(method.to_string(), url.clone(), body.clone()),
            "transform_strip_headers",
            limit,
        )
        .unwrap();

        prop_assert_eq!(args.url, url::Url::parse(&url).unwrap().to_string());
        prop_assert_eq!(args.max_response_bytes, Some(limit.unwrap_or(limits::DEFAULT_FETCH_RESPONSE_BYTES)));
        match method.to_ascii_uppercase().as_str() {
            "GET" => {
                prop_assert!(matches!(args.method, HttpMethod::GET));
                prop_assert_eq!(args.body, None);
            }
            "HEAD" => {
                prop_assert!(matches!(args.method, HttpMethod::HEAD));
                prop_assert_eq!(args.body, None);
            }
            "POST" => {
                prop_assert!(matches!(args.method, HttpMethod::POST));
                prop_assert_eq!(args.body, Some(body));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn non_public_or_credentialed_hosts_are_rejected(url in prop::sample::select(REJECTED_URLS)) {
        let result = build_https_outcall_args(
            request("GET".to_string(), url.to_string(), Vec::new()),
            "transform_strip_headers",
            Some(1024),
        );

        prop_assert!(result.is_err(), "{url}");
    }

    #[test]
    fn generated_url_fuzz_inputs_do_not_panic(raw_url in ".{0,256}") {
        let result = build_https_outcall_args(
            request("GET".to_string(), raw_url, Vec::new()),
            "transform_strip_headers",
            Some(1024),
        );

        if let Ok(args) = result {
            prop_assert!(args.url.starts_with("https://"));
            prop_assert!(args.max_response_bytes <= Some(limits::MAX_FETCH_RESPONSE_BYTES));
        }
    }
}
