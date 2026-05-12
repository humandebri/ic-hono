//! `crates/ic-edge-web` defines small Web API-shaped value types.
//! It keeps request/response conversion logic independent from QuickJS.

use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, Error>;

pub mod limits {
    pub const MAX_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
    pub const MAX_INBOUND_BODY_BYTES: usize = 1024 * 1024;
    pub const MAX_JS_RESPONSE_BODY_BYTES: usize = 1024 * 1024;
    pub const DEFAULT_FETCH_RESPONSE_BYTES: u64 = 64 * 1024;
    pub const MAX_FETCH_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
    pub const MAX_FETCHES_PER_REQUEST: usize = 16;
    pub const MAX_CACHE_ENTRY_BYTES: usize = 256 * 1024;
    pub const MAX_CACHE_TOTAL_BYTES: usize = 4 * 1024 * 1024;
    pub const MAX_RUNTIME_HISTORY: usize = 5;
    pub const MAX_ENV_NAMES: usize = 64;
    pub const MAX_ENV_VALUE_BYTES: usize = 16 * 1024;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidHeaderName,
    InvalidStatus,
    InvalidUtf8Body,
    Runtime(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headers {
    values: BTreeMap<String, Vec<String>>,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Result<Self> {
        let mut headers = Self::new();
        for (name, value) in pairs {
            headers.append(&name, value)?;
        }
        Ok(headers)
    }

    pub fn append(&mut self, name: &str, value: String) -> Result<()> {
        let normalized = normalize_header_name(name)?;
        self.values.entry(normalized).or_default().push(value);
        Ok(())
    }

    pub fn set(&mut self, name: &str, value: String) -> Result<()> {
        let normalized = normalize_header_name(name)?;
        self.values.insert(normalized, vec![value]);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let normalized = normalize_header_name(name).ok()?;
        self.values.get(&normalized).map(|values| values.join(", "))
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    bytes: Vec<u8>,
}

impl Body {
    pub fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.bytes.clone()).map_err(|_| Error::InvalidUtf8Body)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Headers,
    pub body: Body,
}

impl Request {
    pub fn new(method: String, url: String, headers: Headers, body: Body) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Headers,
    pub body: Body,
}

impl Response {
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
    }

    #[test]
    fn response_rejects_invalid_status() {
        let err = Response::new(99, Headers::new(), Body::empty()).unwrap_err();
        assert_eq!(err, Error::InvalidStatus);
    }

    #[test]
    fn body_text_rejects_invalid_utf8() {
        let body = Body::from_bytes(vec![0xff]);
        assert_eq!(body.text().unwrap_err(), Error::InvalidUtf8Body);
    }
}
