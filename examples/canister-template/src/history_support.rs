//! `examples/canister-template` stores runtime generations for rollback.
//! Each snapshot captures the app bundle and env values as one unit.

use crate::{bump_generation, read_generation};
use candid::CandidType;
use ic_edge_store::{EdgeStore, StableEdgeStore};
use ic_edge_web::limits;
use serde::{Deserialize, Serialize};

const HISTORY_KEY: &str = "__runtime_history";

#[derive(CandidType)]
pub(crate) struct RuntimeSnapshotInfo {
    pub(crate) generation: u64,
    pub(crate) bundle_bytes: u64,
    pub(crate) env_names: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct RuntimeSnapshot {
    generation: u64,
    bundle: Vec<u8>,
    env: Vec<(String, String)>,
}

pub(crate) fn record_snapshot(store: &mut StableEdgeStore, generation: u64) -> Result<(), String> {
    let snapshot = RuntimeSnapshot {
        generation,
        bundle: store.get_module("app").unwrap_or_default(),
        env: read_env(store)?,
    };
    let bytes = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
    store
        .put_kv(&snapshot_key(generation), &bytes)
        .map_err(|error| format!("{error:?}"))?;
    let mut history = read_history_ids(store);
    history.retain(|item| *item != generation);
    history.push(generation);
    while history.len() > limits::MAX_RUNTIME_HISTORY {
        let old = history.remove(0);
        store
            .delete_kv(&snapshot_key(old))
            .map_err(|error| format!("{error:?}"))?;
    }
    write_history_ids(store, &history)
}

pub(crate) fn runtime_history(store: &StableEdgeStore) -> Vec<RuntimeSnapshotInfo> {
    read_history_ids(store)
        .into_iter()
        .filter_map(|generation| read_snapshot(store, generation).ok())
        .map(|snapshot| RuntimeSnapshotInfo {
            generation: snapshot.generation,
            bundle_bytes: snapshot.bundle.len() as u64,
            env_names: snapshot.env.into_iter().map(|(name, _)| name).collect(),
        })
        .collect()
}

pub(crate) fn rollback(store: &mut StableEdgeStore, generation: u64) -> Result<(), String> {
    let snapshot = read_snapshot(store, generation)?;
    store
        .put_module("app", &snapshot.bundle)
        .map_err(|error| format!("{error:?}"))?;
    for name in crate::read_env_names(store) {
        store
            .delete_kv(&format!("env:{name}"))
            .map_err(|error| format!("{error:?}"))?;
    }
    let names = snapshot
        .env
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for (name, value) in snapshot.env {
        store
            .put_kv(&format!("env:{name}"), value.as_bytes())
            .map_err(|error| format!("{error:?}"))?;
    }
    store
        .put_kv("__env_names", names.as_bytes())
        .map_err(|error| format!("{error:?}"))?;
    let next = bump_generation(store).map_err(|error| format!("{error:?}"))?;
    record_snapshot(store, next)
}

fn read_env(store: &StableEdgeStore) -> Result<Vec<(String, String)>, String> {
    crate::read_env_names(store)
        .into_iter()
        .map(|name| {
            let value = store
                .get_kv(&format!("env:{name}"))
                .map_err(|error| format!("{error:?}"))?
                .ok_or_else(|| format!("missing env {name}"))?;
            let value = String::from_utf8(value).map_err(|error| error.to_string())?;
            Ok((name, value))
        })
        .collect()
}

fn read_snapshot(store: &StableEdgeStore, generation: u64) -> Result<RuntimeSnapshot, String> {
    let bytes = store
        .get_kv(&snapshot_key(generation))
        .map_err(|error| format!("{error:?}"))?
        .ok_or_else(|| format!("missing runtime generation {generation}"))?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn read_history_ids(store: &StableEdgeStore) -> Vec<u64> {
    store
        .get_kv(HISTORY_KEY)
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_history_ids(store: &mut StableEdgeStore, history: &[u64]) -> Result<(), String> {
    let bytes = serde_json::to_vec(history).map_err(|error| error.to_string())?;
    store
        .put_kv(HISTORY_KEY, &bytes)
        .map_err(|error| format!("{error:?}"))
}

fn snapshot_key(generation: u64) -> String {
    format!("__runtime_snapshot:{generation}")
}

#[allow(dead_code)]
fn _current_generation(store: &StableEdgeStore) -> u64 {
    read_generation(store)
}
