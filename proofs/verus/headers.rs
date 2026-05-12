//! Verus proof model for `crates/ic-edge-web` header normalization.
//! It mirrors only pure header-name validation and header-map behavior.

#![crate_type = "lib"]

use vstd::map::*;
use vstd::prelude::*;
use vstd::seq::*;

verus! {

pub struct HeaderName {
    pub raw: Seq<int>,
    pub normalized_key: int,
}

pub struct HeadersModel {
    pub values: Map<int, Seq<Seq<int>>>,
}

pub open spec fn is_upper_alpha(byte: int) -> bool {
    65 <= byte && byte <= 90
}

pub open spec fn is_lower_alpha(byte: int) -> bool {
    97 <= byte && byte <= 122
}

pub open spec fn is_digit(byte: int) -> bool {
    48 <= byte && byte <= 57
}

pub open spec fn is_http_token_symbol(byte: int) -> bool {
    byte == 33 || byte == 35 || byte == 36 || byte == 37 || byte == 38 || byte == 39
        || byte == 42 || byte == 43 || byte == 45 || byte == 46 || byte == 94
        || byte == 95 || byte == 96 || byte == 124 || byte == 126
}

pub open spec fn is_tchar(byte: int) -> bool {
    is_upper_alpha(byte) || is_lower_alpha(byte) || is_digit(byte)
        || is_http_token_symbol(byte)
}

pub open spec fn lower_byte(byte: int) -> int {
    if is_upper_alpha(byte) {
        byte + 32
    } else {
        byte
    }
}

pub open spec fn normalize_name(raw: Seq<int>) -> Seq<int> {
    raw.map_values(|byte: int| lower_byte(byte))
}

pub open spec fn all_tchar(raw: Seq<int>) -> bool
    decreases raw.len(),
{
    if raw.len() == 0 {
        true
    } else {
        is_tchar(raw[0]) && all_tchar(raw.drop_first())
    }
}

pub open spec fn is_valid_header_name(raw: Seq<int>) -> bool {
    raw.len() > 0 && all_tchar(raw)
}

impl HeaderName {
    pub open spec fn valid(self) -> bool {
        is_valid_header_name(self.raw)
    }

    pub open spec fn normalized(self) -> int {
        self.normalized_key
    }
}

impl HeadersModel {
    pub open spec fn empty() -> Self {
        HeadersModel {
            values: Map::empty(),
        }
    }

    pub open spec fn get(self, name: HeaderName) -> Option<Seq<Seq<int>>> {
        self.values.get(name.normalized())
    }

    pub open spec fn append(self, name: HeaderName, value: Seq<int>) -> Self
        recommends
            name.valid(),
    {
        let key = name.normalized();
        let old_values = if self.values.contains_key(key) {
            self.values[key]
        } else {
            Seq::empty()
        };
        HeadersModel {
            values: self.values.insert(key, old_values.push(value)),
        }
    }

    pub open spec fn set(self, name: HeaderName, value: Seq<int>) -> Self
        recommends
            name.valid(),
    {
        HeadersModel {
            values: self.values.insert(name.normalized(), seq![value]),
        }
    }
}

pub open spec fn content_type_upper_raw() -> Seq<int> {
    seq![67int, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101]
}

pub open spec fn content_type_lower_raw() -> Seq<int> {
    seq![99int, 111, 110, 116, 101, 110, 116, 45, 116, 121, 112, 101]
}

pub open spec fn accept_lower_raw() -> Seq<int> {
    seq![97int, 99, 99, 101, 112, 116]
}

pub open spec fn accept_upper_raw() -> Seq<int> {
    seq![65int, 99, 99, 101, 112, 116]
}

pub open spec fn cache_control_lower_raw() -> Seq<int> {
    seq![99int, 97, 99, 104, 101, 45, 99, 111, 110, 116, 114, 111, 108]
}

pub open spec fn cache_control_upper_raw() -> Seq<int> {
    seq![67int, 97, 99, 104, 101, 45, 67, 111, 110, 116, 114, 111, 108]
}

pub open spec fn content_type_upper() -> HeaderName {
    HeaderName { raw: content_type_upper_raw(), normalized_key: 1 }
}

pub open spec fn content_type_lower() -> HeaderName {
    HeaderName { raw: content_type_lower_raw(), normalized_key: 1 }
}

pub open spec fn accept_lower() -> HeaderName {
    HeaderName { raw: accept_lower_raw(), normalized_key: 2 }
}

pub open spec fn accept_upper() -> HeaderName {
    HeaderName { raw: accept_upper_raw(), normalized_key: 2 }
}

pub open spec fn cache_control_lower() -> HeaderName {
    HeaderName { raw: cache_control_lower_raw(), normalized_key: 3 }
}

pub open spec fn cache_control_upper() -> HeaderName {
    HeaderName { raw: cache_control_upper_raw(), normalized_key: 3 }
}

pub proof fn empty_header_name_is_rejected()
    ensures
        !is_valid_header_name(Seq::<int>::empty()),
{
}

pub proof fn invalid_header_names_are_rejected()
    ensures
        !is_valid_header_name(seq![98int, 97, 100, 32, 104, 101, 97, 100, 101, 114]),
        !is_valid_header_name(seq![120int, 58, 121]),
{
    reveal_with_fuel(all_tchar, 12);
}

pub proof fn content_type_normalizes_to_lowercase()
    ensures
        content_type_upper().valid(),
        content_type_lower().valid(),
        content_type_upper().normalized() == content_type_lower().normalized(),
        normalize_name(content_type_upper_raw()) =~= content_type_lower_raw(),
        normalize_name(content_type_lower_raw()) =~= content_type_lower_raw(),
        normalize_name(normalize_name(content_type_upper_raw())) =~= normalize_name(content_type_upper_raw()),
{
    reveal_with_fuel(all_tchar, 14);
}

pub proof fn content_type_lookup_is_case_insensitive()
    ensures
        HeadersModel::empty()
            .set(content_type_upper(), seq![116int, 101, 120, 116])
            .get(content_type_lower())
            == Some(seq![seq![116int, 101, 120, 116]]),
{
}

pub proof fn append_preserves_existing_values_for_same_key()
    ensures
        HeadersModel::empty()
            .append(accept_lower(), seq![116int, 101, 120, 116])
            .append(accept_upper(), seq![106int, 115, 111, 110])
            .get(accept_lower())
            == Some(seq![seq![116int, 101, 120, 116], seq![106int, 115, 111, 110]]),
{
}

pub proof fn append_preserves_other_keys()
    ensures
        HeadersModel::empty()
            .set(content_type_lower(), seq![116int, 101, 120, 116])
            .append(accept_lower(), seq![106int, 115, 111, 110])
            .get(content_type_upper())
            == Some(seq![seq![116int, 101, 120, 116]]),
{
}

pub proof fn set_replaces_existing_values()
    ensures
        HeadersModel::empty()
            .append(cache_control_lower(), seq![109int, 97, 120])
            .append(cache_control_upper(), seq![112int, 114, 105, 118, 97, 116, 101])
            .set(cache_control_lower(), seq![110int, 111])
            .get(cache_control_lower())
            == Some(seq![seq![110int, 111]]),
{
}

} // verus!
