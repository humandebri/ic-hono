//! `crates/ic-edge-runtime` keeps wasm QuickJS JSON bridge DTOs here.
//! Splitting these types keeps the runtime control flow small and auditable.

use crate::HostFetchOptions;
use ic_edge_web::{limits, Body, Error, Headers, Request, Response, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct HeaderPairs(Vec<(String, String)>);

impl HeaderPairs {
    pub(crate) fn from_headers(headers: &Headers) -> Self {
        Self(
            headers
                .entries()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RuntimeResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: RuntimeBody,
}

impl RuntimeResponse {
    pub(crate) fn from_response(response: Response) -> Self {
        Self {
            status: response.status,
            headers: response
                .headers
                .entries()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: RuntimeBody::Bytes(response.body.bytes().to_vec()),
        }
    }

    pub(crate) fn from_json(value: &str) -> Result<Response> {
        let decoded: Self = serde_json::from_str(value).map_err(|error| {
            let prefix: String = value.chars().take(120).collect();
            Error::Runtime(format!("{error}; output prefix: {prefix}"))
        })?;
        let body = decoded.body.into_bytes();
        if body.len() > limits::MAX_JS_RESPONSE_BODY_BYTES {
            return Err(Error::Runtime(
                "JS response body exceeds v1 limit".to_string(),
            ));
        }
        Response::new(
            decoded.status,
            Headers::from_pairs(decoded.headers)?,
            Body::from_bytes(body),
        )
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RuntimeFetchRequest {
    id: u64,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: RuntimeBody,
    replicated: bool,
}

impl RuntimeFetchRequest {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn to_request(&self) -> Result<Request> {
        Ok(Request::new(
            self.method.clone(),
            self.url.clone(),
            Headers::from_pairs(self.headers.clone())?,
            Body::from_bytes(self.body.clone().into_bytes()),
        ))
    }

    pub(crate) fn options(&self) -> HostFetchOptions {
        HostFetchOptions::new(self.replicated)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum RuntimeBody {
    Bytes(Vec<u8>),
    Text(String),
}

impl RuntimeBody {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Text(text) => text.into_bytes(),
        }
    }
}
