//! `crates/ic-edge-web` validates Web value invariants with generated inputs.
//! It keeps broad header/body/status checks out of the compact unit tests.

use ic_edge_web::{Body, Error, Headers, Response};
use proptest::prelude::*;

fn valid_header_name() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(HEADER_NAME_BYTES), 1..64)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

fn header_value() -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..=255, 0..128)
        .prop_filter("no header framing bytes", |bytes| {
            bytes.iter().all(|byte| !matches!(*byte, b'\r' | b'\n' | 0))
        })
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

fn invalid_header_name() -> impl Strategy<Value = String> {
    any::<String>().prop_filter("invalid HTTP token", |name| !is_valid_header_name(name))
}

fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || HEADER_NAME_BYTES.contains(&byte))
}

const HEADER_NAME_BYTES: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&'*+-.^_`|~";

proptest! {
    #[test]
    fn headers_join_values_in_append_order(name in valid_header_name(), values in prop::collection::vec(header_value(), 1..12)) {
        let mut headers = Headers::new();
        for value in &values {
            headers.append(&name, value.clone()).unwrap();
        }

        prop_assert_eq!(headers.get(&name.to_ascii_uppercase()), Some(values.join(", ")));
        prop_assert_eq!(headers.entries().count(), values.len());
    }

    #[test]
    fn headers_reject_generated_invalid_names(name in invalid_header_name()) {
        let mut headers = Headers::new();
        prop_assert_eq!(headers.set(&name, "value".to_string()), Err(Error::InvalidHeaderName));
    }

    #[test]
    fn response_accepts_only_http_status_range(status in 0u16..700) {
        let result = Response::new(status, Headers::new(), Body::empty());
        prop_assert_eq!(result.is_ok(), (100..=599).contains(&status));
    }

    #[test]
    fn body_text_matches_std_utf8(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let body = Body::from_bytes(bytes.clone());
        match String::from_utf8(bytes) {
            Ok(text) => prop_assert_eq!(body.text(), Ok(text)),
            Err(_) => prop_assert_eq!(body.text(), Err(Error::InvalidUtf8Body)),
        }
    }
}
