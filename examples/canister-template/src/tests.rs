//! `examples/canister-template` verifies controller-facing runtime state.
//! Tests keep endpoint and rollback checks out of the production module.

use super::{
    abort_bundle_upload_in_store, append_bundle_chunk_in_store, begin_bundle_upload_in_store,
    bump_generation, commit_bundle_upload_in_store, history_support, http_request, read_generation,
    read_module_manifest, sha256_hex, upload_bundle_in_store,
};
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
fn chunk_upload_commits_bundle_and_bumps_generation_once() {
    let mut store = StableEdgeStore::new();
    begin_bundle_upload_in_store(&mut store, "app", 11, &manifest_json(b"hello world")).unwrap();
    append_bundle_chunk_in_store(&mut store, "app", 0, b"hello ").unwrap();
    append_bundle_chunk_in_store(&mut store, "app", 6, b"world").unwrap();

    assert_eq!(read_generation(&store), 0);
    assert_eq!(store.get_module("app"), Err(ic_edge_store::Error::NotFound));

    commit_bundle_upload_in_store(&mut store, "app").unwrap();

    assert_eq!(store.get_module("app").unwrap(), b"hello world");
    assert!(!read_module_manifest(&store, "app").is_empty());
    assert_eq!(read_generation(&store), 1);
}

#[test]
fn chunk_upload_rejects_invalid_sizes_and_offsets() {
    let mut store = StableEdgeStore::new();
    assert!(begin_bundle_upload_in_store(
        &mut store,
        "app",
        ic_edge_web::limits::MAX_BUNDLE_BYTES + 1,
        &manifest_json(b""),
    )
    .is_err());

    begin_bundle_upload_in_store(&mut store, "app", 4, &manifest_json(b"abcd")).unwrap();
    assert!(append_bundle_chunk_in_store(&mut store, "app", 1, b"a").is_err());
    assert!(append_bundle_chunk_in_store(
        &mut store,
        "app",
        0,
        &vec![0; ic_edge_web::limits::MAX_BUNDLE_UPLOAD_CHUNK_BYTES + 1],
    )
    .is_err());

    append_bundle_chunk_in_store(&mut store, "app", 0, b"abc").unwrap();
    assert!(append_bundle_chunk_in_store(&mut store, "app", 3, b"de").is_err());
    assert!(commit_bundle_upload_in_store(&mut store, "app").is_err());
}

#[test]
fn chunk_upload_abort_discards_staging() {
    let mut store = StableEdgeStore::new();
    begin_bundle_upload_in_store(&mut store, "app", 3, &manifest_json(b"abc")).unwrap();
    append_bundle_chunk_in_store(&mut store, "app", 0, b"abc").unwrap();
    abort_bundle_upload_in_store(&mut store, "app").unwrap();

    assert!(commit_bundle_upload_in_store(&mut store, "app").is_err());
    assert_eq!(read_generation(&store), 0);
}

#[test]
fn direct_upload_requires_manifest() {
    let mut store = StableEdgeStore::new();
    begin_bundle_upload_in_store(&mut store, "app", 3, &manifest_json(b"old")).unwrap();
    append_bundle_chunk_in_store(&mut store, "app", 0, b"old").unwrap();

    assert_eq!(
        upload_bundle_in_store(&mut store, "app", b"direct").unwrap_err(),
        "manifest is required"
    );
    abort_bundle_upload_in_store(&mut store, "app").unwrap();

    assert!(commit_bundle_upload_in_store(&mut store, "app").is_err());
    assert_eq!(store.get_module("app"), Err(ic_edge_store::Error::NotFound));
    assert_eq!(read_generation(&store), 0);
}

#[test]
fn chunk_upload_rejects_manifest_hash_mismatch() {
    let mut store = StableEdgeStore::new();
    begin_bundle_upload_in_store(&mut store, "app", 3, &manifest_json(b"bad")).unwrap();
    append_bundle_chunk_in_store(&mut store, "app", 0, b"abc").unwrap();

    assert_eq!(
        commit_bundle_upload_in_store(&mut store, "app").unwrap_err(),
        "bundle sha256 does not match manifest"
    );
    assert_eq!(store.get_module("app"), Err(ic_edge_store::Error::NotFound));
    assert_eq!(read_generation(&store), 0);
}

#[test]
fn runtime_history_keeps_recent_snapshots_and_rolls_back_bundle_env() {
    let mut store = StableEdgeStore::new();
    for generation in 1..=6 {
        store
            .put_module("app", format!("bundle-{generation}").as_bytes())
            .unwrap();
        super::put_module_manifest(
            &mut store,
            "app",
            manifest_json(format!("bundle-{generation}").as_bytes()).as_bytes(),
        )
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
    assert_eq!(
        read_module_manifest(&store, "app"),
        manifest_json(b"bundle-3").as_bytes()
    );
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

fn manifest_json(bundle: &[u8]) -> String {
    format!(
        "{{\"schema_version\":1,\"bundle_sha256\":\"{}\"}}",
        sha256_hex(bundle)
    )
}
