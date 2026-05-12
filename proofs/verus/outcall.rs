//! Verus proof model for `crates/ic-edge-canister` HTTPS outcall argument rules.
//! It mirrors URL, response-size, method, and body-selection decisions.

#![crate_type = "lib"]

use vstd::prelude::*;

verus! {

pub enum Method {
    Get,
    Post,
    Head,
    Put,
}

pub open spec fn default_fetch_response_bytes() -> nat {
    64 * 1024
}

pub open spec fn max_fetch_response_bytes() -> nat {
    2 * 1024 * 1024
}

pub open spec fn response_limit(max_response_bytes: Option<nat>) -> nat {
    match max_response_bytes {
        Some(limit) => limit,
        None => default_fetch_response_bytes(),
    }
}

pub open spec fn valid_response_limit(max_response_bytes: Option<nat>) -> bool {
    response_limit(max_response_bytes) <= max_fetch_response_bytes()
}

pub open spec fn supported_method(method: Method) -> bool {
    match method {
        Method::Get => true,
        Method::Post => true,
        Method::Head => true,
        Method::Put => false,
    }
}

pub open spec fn request_body(method: Method, body_len: nat) -> Option<nat> {
    match method {
        Method::Post => Some(body_len),
        _ => None,
    }
}

pub open spec fn builds_outcall(
    has_https_scheme: bool,
    has_authority: bool,
    method: Method,
    max_response_bytes: Option<nat>,
) -> bool {
    has_https_scheme
        && has_authority
        && supported_method(method)
        && valid_response_limit(max_response_bytes)
}

pub proof fn default_response_limit_is_within_ic_limit()
    ensures
        response_limit(None) == default_fetch_response_bytes(),
        valid_response_limit(None),
{
}

pub proof fn response_limit_above_ic_max_is_rejected(limit: nat)
    requires
        limit > max_fetch_response_bytes(),
    ensures
        !valid_response_limit(Some(limit)),
        !builds_outcall(true, true, Method::Get, Some(limit)),
{
}

pub proof fn non_https_url_is_rejected(method: Method, max_response_bytes: Option<nat>)
    ensures
        !builds_outcall(false, true, method, max_response_bytes),
{
}

pub proof fn missing_authority_is_rejected(method: Method, max_response_bytes: Option<nat>)
    ensures
        !builds_outcall(true, false, method, max_response_bytes),
{
}

pub proof fn unsupported_method_is_rejected(max_response_bytes: Option<nat>)
    ensures
        !supported_method(Method::Put),
        !builds_outcall(true, true, Method::Put, max_response_bytes),
{
}

pub proof fn post_keeps_body_and_get_head_drop_body(body_len: nat)
    ensures
        request_body(Method::Post, body_len) == Some(body_len),
        request_body(Method::Get, body_len) == None::<nat>,
        request_body(Method::Head, body_len) == None::<nat>,
{
}

} // verus!
