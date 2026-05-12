//! `examples/canister-template` verifies controller-facing runtime state.
//! Tests keep endpoint and rollback checks out of the production module.

use super::{bump_generation, history_support, http_request, read_generation};
use ic_edge_canister::CdkHttpRequest;
use ic_edge_store::{EdgeStore, StableEdgeStore};

#[test]
fn query_http_request_forces_update_replay() {
    let response = http_request(CdkHttpRequest {
        method: "GET".to_string(),
        url: "/".to_string(),
        headers: Vec::new(),
        body: Vec::new(),
        certificate_version: None,
    });

    assert_eq!(response.status_code, 200);
    assert_eq!(response.body, Vec::<u8>::new());
    assert_eq!(response.upgrade, Some(true));
}

#[test]
fn generation_starts_at_zero_and_increments() {
    let mut store = StableEdgeStore::new();
    assert_eq!(read_generation(&store), 0);
    assert_eq!(bump_generation(&mut store).unwrap(), 1);
    assert_eq!(read_generation(&store), 1);
}

#[test]
fn runtime_history_keeps_recent_snapshots_and_rolls_back_bundle_env() {
    let mut store = StableEdgeStore::new();
    for generation in 1..=6 {
        store
            .put_module("app", format!("bundle-{generation}").as_bytes())
            .unwrap();
        store
            .put_kv("env:ACTIVE", format!("env-{generation}").as_bytes())
            .unwrap();
        store.put_kv("__env_names", b"ACTIVE").unwrap();
        history_support::record_snapshot(&mut store, generation).unwrap();
    }

    let history = history_support::runtime_history(&store);
    assert_eq!(history.len(), 5);
    assert_eq!(history[0].generation, 2);
    assert_eq!(history[4].generation, 6);

    history_support::rollback(&mut store, 3).unwrap();
    assert_eq!(store.get_module("app").unwrap(), b"bundle-3");
    assert_eq!(store.get_kv("env:ACTIVE").unwrap(), Some(b"env-3".to_vec()));
    assert_eq!(read_generation(&store), 1);
}

#[test]
fn runtime_history_updates_duplicate_generation_without_duplicate_entry() {
    let mut store = StableEdgeStore::new();
    store.put_module("app", b"old").unwrap();
    history_support::record_snapshot(&mut store, 1).unwrap();
    store.put_module("app", b"new").unwrap();
    history_support::record_snapshot(&mut store, 1).unwrap();

    let history = history_support::runtime_history(&store);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].generation, 1);
    assert_eq!(history[0].bundle_bytes, 3);
}
