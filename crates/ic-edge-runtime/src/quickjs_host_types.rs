//! `crates/ic-edge-runtime` maps host QuickJS wire JSON to Rust values.
//! Keeping host conversion here keeps the runtime control path small.

use crate::quickjs::HostFetch;
use ic_edge_web::{limits, Body, Error, Headers, Request, Response, Result};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn host_fetch<F>(
    fetcher: &Rc<RefCell<F>>,
    method: String,
    url: String,
    headers_json: String,
    body_json: String,
) -> rquickjs::Result<String>
where
    F: HostFetch,
{
    let pairs: Vec<(String, String)> = serde_json::from_str(&headers_json).map_err(|error| {
        rquickjs::Error::new_from_js_message("Headers", "Headers", error.to_string())
    })?;
    let body = serde_json::from_str(&body_json)
        .map_err(|error| rquickjs::Error::new_from_js_message("Body", "Body", error.to_string()))?;
    let request = Request::new(
        method,
        url,
        Headers::from_pairs(pairs).map_err(to_js_error)?,
        Body::from_bytes(body),
    );
    let response = fetcher.borrow_mut().fetch(request).map_err(to_js_error)?;
    RuntimeResponse::from_response(response).map_err(to_js_error)
}

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
    status: Option<RuntimeStatus>,
    headers: Vec<(String, String)>,
    body: RuntimeBody,
}

impl RuntimeResponse {
    pub(crate) fn from_json(value: &str) -> Result<Response> {
        let decoded: Self =
            serde_json::from_str(value).map_err(|error| Error::Runtime(error.to_string()))?;
        let body = decoded.body.into_bytes();
        if body.len() > limits::MAX_JS_RESPONSE_BODY_BYTES {
            return Err(Error::Runtime(
                "JS response body exceeds v1 limit".to_string(),
            ));
        }
        Response::new(
            decoded.status.map_or(Ok(200), |status| status.to_u16())?,
            Headers::from_pairs(decoded.headers)?,
            Body::from_bytes(body),
        )
    }

    fn from_response(response: Response) -> Result<String> {
        let encoded = Self {
            status: Some(RuntimeStatus::Code(response.status)),
            headers: response
                .headers
                .entries()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: RuntimeBody::Bytes(response.body.bytes().to_vec()),
        };
        serde_json::to_string(&encoded).map_err(|error| Error::Runtime(error.to_string()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum RuntimeBody {
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum RuntimeStatus {
    Code(u16),
    Text(String),
}

impl RuntimeStatus {
    fn to_u16(&self) -> Result<u16> {
        match self {
            Self::Code(status) => Ok(*status),
            Self::Text(status) => status
                .parse::<u16>()
                .map_err(|error| Error::Runtime(error.to_string())),
        }
    }
}

fn to_js_error(error: Error) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("Rust", "JavaScript", format!("{error:?}"))
}
