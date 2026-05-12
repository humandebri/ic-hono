//! Verus proof model for Cache API quota arithmetic.
//! It mirrors entry-size rejection and replacement-aware total-size checks.

#![crate_type = "lib"]

use vstd::prelude::*;

verus! {

pub open spec fn max_cache_entry_bytes() -> nat {
    256 * 1024
}

pub open spec fn max_cache_total_bytes() -> nat {
    4 * 1024 * 1024
}

pub open spec fn replacement_total(old_total: nat, old_size: nat, new_size: nat) -> int {
    old_total - old_size + new_size
}

pub open spec fn cache_put_allowed(old_total: nat, old_size: nat, new_size: nat) -> bool {
    old_size <= old_total
        && new_size <= max_cache_entry_bytes()
        && replacement_total(old_total, old_size, new_size) <= max_cache_total_bytes()
}

pub proof fn oversized_entry_is_rejected(old_total: nat, old_size: nat, new_size: nat)
    requires
        new_size > max_cache_entry_bytes(),
    ensures
        !cache_put_allowed(old_total, old_size, new_size),
{
}

pub proof fn accepted_put_keeps_total_within_limit(
    old_total: nat,
    old_size: nat,
    new_size: nat,
)
    requires
        cache_put_allowed(old_total, old_size, new_size),
    ensures
        replacement_total(old_total, old_size, new_size) <= max_cache_total_bytes(),
{
}

pub proof fn replacing_larger_entry_can_reduce_total(old_total: nat, old_size: nat, new_size: nat)
    requires
        new_size <= old_size <= old_total,
    ensures
        replacement_total(old_total, old_size, new_size) <= old_total,
{
}

pub proof fn exact_entry_limit_is_allowed_when_total_fits()
    ensures
        cache_put_allowed(0, 0, max_cache_entry_bytes()),
        replacement_total(0, 0, max_cache_entry_bytes()) == max_cache_entry_bytes(),
{
}

} // verus!
