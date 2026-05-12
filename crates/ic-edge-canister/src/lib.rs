//! `crates/ic-edge-canister` maps canister HTTP values and HTTPS outcalls.
//! It keeps IC-specific bindings outside the QuickJS runtime.

use candid::CandidType;
#[cfg(not(test))]
use ic_cdk::management_canister::transform_context_from_query;
use ic_cdk::management_canister::{
    http_request, HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult, TransformArgs,
    TransformContext,
};
use ic_edge_runtime::{AsyncEdgeRuntime, EdgeRuntime};
use ic_edge_web::{limits, Body, Error, Headers, Request, Response, Result};
use serde::Deserialize;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, CandidType, Deserialize, PartialEq, Eq)]
pub struct CdkHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub certificate_version: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcHttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, CandidType, PartialEq, Eq)]
pub struct CdkHttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub upgrade: Option<bool>,
}

pub fn handle_http(
    runtime: &mut impl EdgeRuntime,
    request: IcHttpRequest,
) -> Result<IcHttpResponse> {
    if request.body.len() > limits::MAX_INBOUND_BODY_BYTES {
        return Ok(limit_response("inbound body exceeds v1 limit"));
    }
    let js_request = Request::new(
        request.method,
        request.url,
        Headers::from_pairs(request.headers)?,
        Body::from_bytes(request.body),
    );
    let js_response = runtime.call_app_fetch(js_request)?;
    Ok(to_ic_response(js_response))
}

pub fn handle_cdk_http(
    runtime: &mut impl EdgeRuntime,
    request: CdkHttpRequest,
) -> Result<CdkHttpResponse> {
    let response = handle_http(runtime, request.into())?;
    Ok(response.into())
}

pub async fn handle_http_async(
    runtime: &mut impl AsyncEdgeRuntime,
    request: IcHttpRequest,
) -> Result<IcHttpResponse> {
    if request.body.len() > limits::MAX_INBOUND_BODY_BYTES {
        return Ok(limit_response("inbound body exceeds v1 limit"));
    }
    let js_request = Request::new(
        request.method,
        request.url,
        Headers::from_pairs(request.headers)?,
        Body::from_bytes(request.body),
    );
    let js_response = runtime.call_app_fetch(js_request).await?;
    Ok(to_ic_response(js_response))
}

pub async fn handle_cdk_http_async(
    runtime: &mut impl AsyncEdgeRuntime,
    request: CdkHttpRequest,
) -> Result<CdkHttpResponse> {
    let response = handle_http_async(runtime, request.into()).await?;
    Ok(response.into())
}

pub async fn https_outcall_fetch(
    request: Request,
    transform_name: &str,
    max_response_bytes: Option<u64>,
) -> Result<Response> {
    let args = build_https_outcall_args(request, transform_name, max_response_bytes)?;
    let response = http_request(&args)
        .await
        .map_err(|error| Error::Runtime(format!("{error:?}")))?;
    from_outcall_response(response)
}

pub fn build_https_outcall_args(
    request: Request,
    transform_name: &str,
    max_response_bytes: Option<u64>,
) -> Result<HttpRequestArgs> {
    if !request.url.starts_with("https://") {
        return Err(Error::Runtime(
            "HTTPS outcalls require an https:// URL".to_string(),
        ));
    }
    let response_limit = max_response_bytes.unwrap_or(limits::DEFAULT_FETCH_RESPONSE_BYTES);
    if response_limit > limits::MAX_FETCH_RESPONSE_BYTES {
        return Err(Error::Runtime(format!(
            "HTTPS outcall max_response_bytes exceeds {}",
            limits::MAX_FETCH_RESPONSE_BYTES
        )));
    }
    Ok(HttpRequestArgs {
        url: request.url,
        max_response_bytes: Some(response_limit),
        method: to_http_method(&request.method)?,
        headers: request
            .headers
            .entries()
            .map(|(name, value)| HttpHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect(),
        body: body_for_method(&request.method, request.body),
        transform: transform_context(transform_name),
    })
}

pub fn transform_strip_headers(args: TransformArgs) -> HttpRequestResult {
    HttpRequestResult {
        status: args.response.status,
        headers: Vec::new(),
        body: args.response.body,
    }
}

fn to_ic_response(response: Response) -> IcHttpResponse {
    let headers = response
        .headers
        .entries()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    IcHttpResponse {
        status_code: response.status,
        headers,
        body: response.body.bytes().to_vec(),
    }
}

fn limit_response(message: &str) -> IcHttpResponse {
    IcHttpResponse {
        status_code: 413,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: message.as_bytes().to_vec(),
    }
}

fn to_http_method(method: &str) -> Result<HttpMethod> {
    match method.to_ascii_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "HEAD" => Ok(HttpMethod::HEAD),
        other => Err(Error::Runtime(format!(
            "HTTPS outcalls do not support {other}"
        ))),
    }
}

fn body_for_method(method: &str, body: Body) -> Option<Vec<u8>> {
    match method.to_ascii_uppercase().as_str() {
        "POST" => Some(body.bytes().to_vec()),
        _ => None,
    }
}

#[cfg(not(test))]
fn transform_context(transform_name: &str) -> Option<TransformContext> {
    Some(transform_context_from_query(
        transform_name.to_string(),
        Vec::new(),
    ))
}

#[cfg(test)]
fn transform_context(_transform_name: &str) -> Option<TransformContext> {
    None
}

fn from_outcall_response(response: HttpRequestResult) -> Result<Response> {
    let status = response
        .status
        .to_string()
        .parse::<u16>()
        .map_err(|error: std::num::ParseIntError| Error::Runtime(error.to_string()))?;
    let headers = Headers::from_pairs(
        response
            .headers
            .into_iter()
            .map(|header| (header.name, header.value)),
    )?;
    Response::new(status, headers, Body::from_bytes(response.body))
}

impl From<CdkHttpRequest> for IcHttpRequest {
    fn from(request: CdkHttpRequest) -> Self {
        Self {
            method: request.method,
            url: request.url,
            headers: request.headers,
            body: request.body,
        }
    }
}

impl From<IcHttpResponse> for CdkHttpResponse {
    fn from(response: IcHttpResponse) -> Self {
        Self {
            status_code: response.status_code,
            headers: response.headers,
            body: response.body,
            upgrade: None,
        }
    }
}
