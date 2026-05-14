//! `crates/ic-edge-web` defines small Web API-shaped value types.
//! It keeps request/response conversion logic independent from QuickJS.

use std::collections::BTreeMap;

/// Result type used by the Web value crates.
pub type Result<T> = std::result::Result<T, Error>;

/// Fixed v1 preview resource limits shared by runtime and canister adapters.
pub mod limits {
    /// Maximum JavaScript bundle size accepted by upload APIs.
    pub const MAX_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
    /// Maximum bundle upload chunk size accepted by the canister API.
    pub const MAX_BUNDLE_UPLOAD_CHUNK_BYTES: usize = 512 * 1024;
    /// Maximum inbound HTTP body size.
    pub const MAX_INBOUND_BODY_BYTES: usize = 1024 * 1024;
    /// Maximum response body produced by JavaScript.
    pub const MAX_JS_RESPONSE_BODY_BYTES: usize = 1024 * 1024;
    /// Default HTTPS outcall response limit.
    pub const DEFAULT_FETCH_RESPONSE_BYTES: u64 = 64 * 1024;
    /// Maximum HTTPS outcall response limit.
    pub const MAX_FETCH_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
    /// Maximum external fetch calls per request.
    pub const MAX_FETCHES_PER_REQUEST: usize = 16;
    /// Maximum serialized Cache API entry size.
    pub const MAX_CACHE_ENTRY_BYTES: usize = 256 * 1024;
    /// Maximum total Cache API storage tracked by the template.
    pub const MAX_CACHE_TOTAL_BYTES: usize = 4 * 1024 * 1024;
    /// Maximum Cache API namespace length.
    pub const MAX_CACHE_NAME_BYTES: usize = 128;
    /// Maximum normalized Cache API key length.
    pub const MAX_CACHE_KEY_BYTES: usize = 2 * 1024;
    /// Maximum number of Cache API index entries.
    pub const MAX_CACHE_INDEX_ENTRIES: usize = 1024;
    /// Maximum serialized Cache API index size.
    pub const MAX_CACHE_INDEX_BYTES: usize = 128 * 1024;
    /// Number of runtime snapshots retained for rollback.
    pub const MAX_RUNTIME_HISTORY: usize = 5;
    /// Maximum number of environment variable names.
    pub const MAX_ENV_NAMES: usize = 64;
    /// Maximum environment variable value size.
    pub const MAX_ENV_VALUE_BYTES: usize = 16 * 1024;
}

/// Public error contract for runtime-neutral Web value handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Header name is empty or contains a byte outside the HTTP token set.
    InvalidHeaderName,
    /// Header value contains bytes that can break HTTP header framing.
    InvalidHeaderValue,
    /// HTTP status is outside `100..=599`.
    InvalidStatus,
    /// Body bytes could not be decoded as UTF-8 text.
    InvalidUtf8Body,
    /// Runtime-specific error surfaced as text in the v1 preview.
    Runtime(String),
}

/// Case-insensitive HTTP headers preserving multiple values per name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headers {
    values: BTreeMap<String, Vec<String>>,
}

impl Headers {
    /// Creates an empty header map.
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Builds headers from `(name, value)` pairs.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Result<Self> {
        let mut headers = Self::new();
        for (name, value) in pairs {
            headers.append(&name, value)?;
        }
        Ok(headers)
    }

    /// Appends a value without removing existing values for the same name.
    pub fn append(&mut self, name: &str, value: String) -> Result<()> {
        let normalized = normalize_header_name(name)?;
        validate_header_value(&value)?;
        self.values.entry(normalized).or_default().push(value);
        Ok(())
    }

    /// Replaces all existing values for `name`.
    pub fn set(&mut self, name: &str, value: String) -> Result<()> {
        let normalized = normalize_header_name(name)?;
        validate_header_value(&value)?;
        self.values.insert(normalized, vec![value]);
        Ok(())
    }

    /// Returns values for `name` joined by `, `.
    pub fn get(&self, name: &str) -> Option<String> {
        let normalized = normalize_header_name(name).ok()?;
        self.values.get(&normalized).map(|values| values.join(", "))
    }

    /// Iterates over each stored `(name, value)` pair.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().flat_map(|(name, values)| {
            values
                .iter()
                .map(move |value| (name.as_str(), value.as_str()))
        })
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

/// Byte body shared by request and response values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    bytes: Vec<u8>,
}

impl Body {
    /// Creates an empty body.
    pub fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Creates a body from raw bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Decodes the body as UTF-8 text.
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.bytes.clone()).map_err(|_| Error::InvalidUtf8Body)
    }

    /// Borrows the raw body bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Runtime-neutral HTTP request value passed into `EdgeRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// HTTP method.
    pub method: String,
    /// Absolute URL or path-like URL before runtime normalization.
    pub url: String,
    /// Request headers.
    pub headers: Headers,
    /// Request body bytes.
    pub body: Body,
}

impl Request {
    /// Creates a new request value.
    pub fn new(method: String, url: String, headers: Headers, body: Body) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }
}

/// Runtime-neutral HTTP response value returned by `EdgeRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: Headers,
    /// Response body bytes.
    pub body: Body,
}

impl Response {
    /// Creates a response and validates the status range.
    pub fn new(status: u16, headers: Headers, body: Body) -> Result<Self> {
        if !(100..=599).contains(&status) {
            return Err(Error::InvalidStatus);
        }
        Ok(Self {
            status,
            headers,
            body,
        })
    }

    /// Creates a `200 text/plain; charset=utf-8` response.
    pub fn text(value: impl Into<String>) -> Result<Self> {
        let mut headers = Headers::new();
        headers.set("content-type", "text/plain; charset=utf-8".to_string())?;
        Self::new(200, headers, Body::from_bytes(value.into().into_bytes()))
    }
}

fn normalize_header_name(name: &str) -> Result<String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
    {
        return Err(Error::InvalidHeaderName);
    }
    Ok(name.to_ascii_lowercase())
}

fn validate_header_value(value: &str) -> Result<()> {
    if value.bytes().any(|byte| matches!(byte, b'\r' | b'\n' | 0)) {
        return Err(Error::InvalidHeaderValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_case_insensitive() {
        let mut headers = Headers::new();
        headers
            .set("Content-Type", "text/plain".to_string())
            .unwrap();
        assert_eq!(headers.get("content-type"), Some("text/plain".to_string()));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("text/plain".to_string()));
    }

    #[test]
    fn headers_reject_invalid_names() {
        let mut headers = Headers::new();
        assert_eq!(
            headers.set("", "value".to_string()).unwrap_err(),
            Error::InvalidHeaderName
        );
        assert_eq!(
            headers.set("bad header", "value".to_string()).unwrap_err(),
            Error::InvalidHeaderName
        );
        assert_eq!(
            headers.set("x:y", "value".to_string()).unwrap_err(),
            Error::InvalidHeaderName
        );
    }

    #[test]
    fn headers_reject_invalid_values() {
        let mut headers = Headers::new();
        assert_eq!(
            headers.set("x-edge", "bad\rvalue".to_string()).unwrap_err(),
            Error::InvalidHeaderValue
        );
        assert_eq!(
            headers.set("x-edge", "bad\nvalue".to_string()).unwrap_err(),
            Error::InvalidHeaderValue
        );
        assert_eq!(
            headers.set("x-edge", "bad\0value".to_string()).unwrap_err(),
            Error::InvalidHeaderValue
        );
        assert_eq!(
            Headers::from_pairs(vec![("x-edge".to_string(), "bad\rvalue".to_string())])
                .unwrap_err(),
            Error::InvalidHeaderValue
        );
    }

    #[test]
    fn append_preserves_value_order() {
        let mut headers = Headers::new();
        headers.append("accept", "text/plain".to_string()).unwrap();
        headers
            .append("Accept", "application/json".to_string())
            .unwrap();
        assert_eq!(
            headers.get("ACCEPT"),
            Some("text/plain, application/json".to_string())
        );
    }

    #[test]
    fn set_replaces_existing_values() {
        let mut headers = Headers::new();
        headers
            .append("cache-control", "max-age=60".to_string())
            .unwrap();
        headers
            .append("Cache-Control", "private".to_string())
            .unwrap();
        headers
            .set("CACHE-CONTROL", "no-store".to_string())
            .unwrap();
        assert_eq!(headers.get("cache-control"), Some("no-store".to_string()));
    }

    #[test]
    fn response_rejects_invalid_status() {
        let err = Response::new(99, Headers::new(), Body::empty()).unwrap_err();
        assert_eq!(err, Error::InvalidStatus);
        let err = Response::new(600, Headers::new(), Body::empty()).unwrap_err();
        assert_eq!(err, Error::InvalidStatus);
        assert!(Response::new(100, Headers::new(), Body::empty()).is_ok());
        assert!(Response::new(599, Headers::new(), Body::empty()).is_ok());
    }

    #[test]
    fn body_text_rejects_invalid_utf8() {
        let body = Body::from_bytes(vec![0xff]);
        assert_eq!(body.text().unwrap_err(), Error::InvalidUtf8Body);
    }
}
