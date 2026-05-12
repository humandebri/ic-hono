//! Verus proof model for canister-template environment name indexing.
//! It covers env-name validity and idempotent insertion into a valid-name set.

#![crate_type = "lib"]

use vstd::prelude::*;
use vstd::set::*;
use vstd::seq::*;

verus! {

pub open spec fn is_upper_alpha(byte: int) -> bool {
    65 <= byte && byte <= 90
}

pub open spec fn is_digit(byte: int) -> bool {
    48 <= byte && byte <= 57
}

pub open spec fn is_env_name_byte(byte: int) -> bool {
    is_upper_alpha(byte) || is_digit(byte) || byte == 95
}

pub open spec fn all_env_name_bytes(raw: Seq<int>) -> bool
    decreases raw.len(),
{
    if raw.len() == 0 {
        true
    } else {
        is_env_name_byte(raw[0]) && all_env_name_bytes(raw.drop_first())
    }
}

pub open spec fn valid_env_name(raw: Seq<int>) -> bool {
    raw.len() > 0 && all_env_name_bytes(raw)
}

pub open spec fn insert_env_id(index: Set<int>, name_id: int, valid_name: bool) -> Set<int> {
    if valid_name {
        index.insert(name_id)
    } else {
        index
    }
}

pub proof fn env_name_accepts_uppercase_digits_and_underscore()
    ensures
        valid_env_name(seq![79int, 80, 69, 78, 65, 73, 95, 75, 69, 89, 50]),
{
    reveal_with_fuel(all_env_name_bytes, 12);
}

pub proof fn env_name_rejects_empty_lowercase_and_hyphen()
    ensures
        !valid_env_name(Seq::<int>::empty()),
        !valid_env_name(seq![79int, 112, 101, 110, 65, 73]),
        !valid_env_name(seq![84int, 79, 75, 69, 78, 45, 78, 65, 77, 69]),
{
    reveal_with_fuel(all_env_name_bytes, 12);
}

pub proof fn invalid_insert_preserves_index(index: Set<int>, name_id: int)
    ensures
        insert_env_id(index, name_id, false) == index,
{
}

pub proof fn valid_insert_contains_name(index: Set<int>, name_id: int)
    ensures
        insert_env_id(index, name_id, true).contains(name_id),
{
}

pub proof fn insert_is_idempotent(index: Set<int>, name_id: int)
    ensures
        insert_env_id(insert_env_id(index, name_id, true), name_id, true)
            == insert_env_id(index, name_id, true),
{
    assert(insert_env_id(insert_env_id(index, name_id, true), name_id, true)
        =~= insert_env_id(index, name_id, true));
}

} // verus!
