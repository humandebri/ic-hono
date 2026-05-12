//! Verus proof model for runtime history retention and rollback snapshot selection.
//! It models the generation list independently from stable-memory serialization.

#![crate_type = "lib"]

use vstd::prelude::*;
use vstd::seq::*;

verus! {

pub open spec fn max_runtime_history() -> nat {
    5
}

pub open spec fn trim_history(history: Seq<int>) -> Seq<int> {
    if history.len() > max_runtime_history() {
        history.subrange(history.len() as int - max_runtime_history(), history.len() as int)
    } else {
        history
    }
}

pub open spec fn append_generation(history: Seq<int>, generation: int) -> Seq<int> {
    trim_history(history.push(generation))
}

pub open spec fn rollback_bundle_generation(snapshot_generation: int) -> int {
    snapshot_generation
}

pub proof fn trim_keeps_short_history(history: Seq<int>)
    requires
        history.len() <= max_runtime_history(),
    ensures
        trim_history(history) == history,
{
}

pub proof fn append_to_full_history_drops_oldest()
    ensures
        append_generation(seq![1int, 2, 3, 4, 5], 6) =~= seq![2int, 3, 4, 5, 6],
{
}

pub proof fn retained_history_never_exceeds_limit_after_append(history: Seq<int>, generation: int)
    requires
        history.len() <= max_runtime_history(),
    ensures
        append_generation(history, generation).len() <= max_runtime_history(),
{
}

pub proof fn rollback_uses_selected_snapshot_generation(snapshot_generation: int)
    ensures
        rollback_bundle_generation(snapshot_generation) == snapshot_generation,
{
}

} // verus!
