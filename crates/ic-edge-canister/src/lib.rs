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
use std::net::{Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

#[cfg(test)]
mod tests;

/// Runtime-neutral IC HTTP request shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcHttpRequest {
    /// HTTP method.
    pub method: String,
    /// Request URL.
    pub url: String,
    /// HTTP headers.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

/// CDK-compatible HTTP request DTO.
#[derive(Debug, Clone, CandidType, Deserialize, PartialEq, Eq)]
pub struct CdkHttpRequest {
    /// HTTP method.
    pub method: String,
    /// Request URL.
    pub url: String,
    /// HTTP headers.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Gateway certificate version.
    pub certificate_version: Option<u16>,
}

/// Runtime-neutral IC HTTP response shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcHttpResponse {
    /// HTTP status code.
    pub status_code: u16,
    /// HTTP headers.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

/// CDK-compatible HTTP response DTO.
#[derive(Debug, Clone, CandidType, PartialEq, Eq)]
pub struct CdkHttpResponse {
    /// HTTP status code.
    pub status_code: u16,
    /// HTTP headers.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Whether the gateway should replay the request as an update call.
    pub upgrade: Option<bool>,
}

/// Handles a runtime-neutral IC HTTP request.
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

/// Handles a CDK-compatible HTTP request.
pub fn handle_cdk_http(
    runtime: &mut impl EdgeRuntime,
    request: CdkHttpRequest,
) -> Result<CdkHttpResponse> {
    let response = handle_http(runtime, request.into())?;
    Ok(response.into())
}

/// Handles a runtime-neutral IC HTTP request with async runtime support.
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

/// Handles a CDK-compatible HTTP request with async runtime support.
pub async fn handle_cdk_http_async(
    runtime: &mut impl AsyncEdgeRuntime,
    request: CdkHttpRequest,
) -> Result<CdkHttpResponse> {
    let response = handle_http_async(runtime, request.into()).await?;
    Ok(response.into())
}

/// Performs an HTTPS outcall for a runtime `Request`.
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

/// Builds management canister HTTPS outcall arguments.
pub fn build_https_outcall_args(
    request: Request,
    transform_name: &str,
    max_response_bytes: Option<u64>,
) -> Result<HttpRequestArgs> {
    let url = validate_outcall_url(&request.url)?;
    let response_limit = max_response_bytes.unwrap_or(limits::DEFAULT_FETCH_RESPONSE_BYTES);
    if response_limit > limits::MAX_FETCH_RESPONSE_BYTES {
        return Err(Error::Runtime(format!(
            "HTTPS outcall max_response_bytes exceeds {}",
            limits::MAX_FETCH_RESPONSE_BYTES
        )));
    }
    Ok(HttpRequestArgs {
        url,
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

fn validate_outcall_url(raw_url: &str) -> Result<String> {
    let url = Url::parse(raw_url)
        .map_err(|error| Error::Runtime(format!("invalid HTTPS outcall URL: {error}")))?;
    if url.scheme() != "https" {
        return Err(Error::Runtime(
            "HTTPS outcalls require an https:// URL".to_string(),
        ));
    }
    let authority = raw_url
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default())
        .unwrap_or_default();
    if authority.is_empty() {
        return Err(Error::Runtime("HTTPS outcalls require a host".to_string()));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Runtime(
            "HTTPS outcalls do not allow URL credentials".to_string(),
        ));
    }
    let host = url
        .host()
        .ok_or_else(|| Error::Runtime("HTTPS outcalls require a host".to_string()))?;
    match host {
        Host::Domain(domain) => validate_outcall_domain(domain)?,
        Host::Ipv4(address) => validate_outcall_ipv4(address)?,
        Host::Ipv6(address) => validate_outcall_ipv6(address)?,
    }
    Ok(url.to_string())
}

fn validate_outcall_domain(domain: &str) -> Result<()> {
    let normalized = domain.trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(Error::Runtime("HTTPS outcalls require a host".to_string()));
    }
    if normalized == "localhost"
        || normalized == "metadata"
        || normalized == "metadata.google.internal"
    {
        return Err(Error::Runtime(
            "HTTPS outcalls require a public host".to_string(),
        ));
    }
    Ok(())
}

fn validate_outcall_ipv4(address: Ipv4Addr) -> Result<()> {
    if address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_multicast()
    {
        return Err(Error::Runtime(
            "HTTPS outcalls require a public IPv4 host".to_string(),
        ));
    }
    Ok(())
}

fn validate_outcall_ipv6(address: Ipv6Addr) -> Result<()> {
    if address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_unicast_link_local()
        || is_unique_local_ipv6(address)
    {
        return Err(Error::Runtime(
            "HTTPS outcalls require a public IPv6 host".to_string(),
        ));
    }
    Ok(())
}

fn is_unique_local_ipv6(address: Ipv6Addr) -> bool {
    (address.segments()[0] & 0xfe00) == 0xfc00
}

/// Transform function that strips all HTTPS outcall response headers.
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
