//! Verus proof model for `ic-edge-web` response and inbound body limits.
//! It keeps status-range and limit-response contracts independent of runtime code.

#![crate_type = "lib"]

use vstd::prelude::*;

verus! {

pub open spec fn max_inbound_body_bytes() -> nat {
    1024 * 1024
}

pub open spec fn limit_status() -> nat {
    413
}

pub open spec fn valid_status(status: nat) -> bool {
    100 <= status && status <= 599
}

pub open spec fn response_new_succeeds(status: nat) -> bool {
    valid_status(status)
}

pub open spec fn inbound_over_limit(body_len: nat) -> bool {
    body_len > max_inbound_body_bytes()
}

pub open spec fn handle_http_status(body_len: nat, runtime_status: nat) -> nat {
    if inbound_over_limit(body_len) {
        limit_status()
    } else {
        runtime_status
    }
}

pub proof fn response_accepts_boundary_statuses()
    ensures
        response_new_succeeds(100),
        response_new_succeeds(599),
{
}

pub proof fn response_rejects_out_of_range_statuses()
    ensures
        !response_new_succeeds(99),
        !response_new_succeeds(600),
{
}

pub proof fn inbound_body_above_limit_returns_413(body_len: nat, runtime_status: nat)
    requires
        body_len > max_inbound_body_bytes(),
    ensures
        handle_http_status(body_len, runtime_status) == limit_status(),
{
}

pub proof fn inbound_body_at_limit_uses_runtime_status(runtime_status: nat)
    ensures
        handle_http_status(max_inbound_body_bytes(), runtime_status) == runtime_status,
{
}

} // verus!
