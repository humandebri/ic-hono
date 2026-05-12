//! `examples/canister-template` backs the Worker Cache API with stable KV.
//! The cache is canister-local durable storage, not a global CDN cache.
#![cfg_attr(
    not(all(target_arch = "wasm32", feature = "quickjs-ic")),
    allow(dead_code)
)]

use ic_edge_runtime::CacheHost;
use ic_edge_store::{EdgeStore, StableEdgeStore};
use ic_edge_web::{limits, Error, Result};
use serde::{Deserialize, Serialize};

const CACHE_INDEX_KEY: &str = "__cache_index";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CacheIndexEntry {
    cache_name: String,
    key: String,
    size: usize,
}

pub(crate) struct StableCacheHost;

impl CacheHost for StableCacheHost {
    fn match_entry(&mut self, cache_name: &str, key: &str) -> Result<Option<String>> {
        crate::STORE.with_borrow(|store| cache_match(store, cache_name, key))
    }

    fn put_entry(&mut self, cache_name: &str, key: &str, response_json: &str) -> Result<()> {
        crate::STORE.with_borrow_mut(|store| cache_put(store, cache_name, key, response_json))
    }

    fn delete_entry(&mut self, cache_name: &str, key: &str) -> Result<bool> {
        crate::STORE.with_borrow_mut(|store| cache_delete(store, cache_name, key))
    }
}

fn cache_match(store: &StableEdgeStore, cache_name: &str, key: &str) -> Result<Option<String>> {
    store
        .get_kv(&cache_key(cache_name, key))
        .map_err(store_error)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| Error::Runtime(error.to_string()))
}

fn cache_put(
    store: &mut StableEdgeStore,
    cache_name: &str,
    key: &str,
    response_json: &str,
) -> Result<()> {
    let size = response_json.len();
    if size > limits::MAX_CACHE_ENTRY_BYTES {
        return Err(Error::Runtime("cache entry exceeds v1 limit".to_string()));
    }
    let mut index = read_index(store)?;
    let old_size = index
        .iter()
        .find(|entry| entry.cache_name == cache_name && entry.key == key)
        .map(|entry| entry.size)
        .unwrap_or(0);
    let total = index.iter().map(|entry| entry.size).sum::<usize>() - old_size + size;
    if total > limits::MAX_CACHE_TOTAL_BYTES {
        return Err(Error::Runtime("cache total exceeds v1 limit".to_string()));
    }
    index.retain(|entry| !(entry.cache_name == cache_name && entry.key == key));
    index.push(CacheIndexEntry {
        cache_name: cache_name.to_string(),
        key: key.to_string(),
        size,
    });
    store
        .put_kv(&cache_key(cache_name, key), response_json.as_bytes())
        .map_err(store_error)?;
    write_index(store, &index)
}

fn cache_delete(store: &mut StableEdgeStore, cache_name: &str, key: &str) -> Result<bool> {
    let mut index = read_index(store)?;
    let before = index.len();
    index.retain(|entry| !(entry.cache_name == cache_name && entry.key == key));
    let deleted = index.len() != before;
    if deleted {
        store
            .delete_kv(&cache_key(cache_name, key))
            .map_err(store_error)?;
        write_index(store, &index)?;
    }
    Ok(deleted)
}

fn read_index(store: &StableEdgeStore) -> Result<Vec<CacheIndexEntry>> {
    let Some(bytes) = store.get_kv(CACHE_INDEX_KEY).map_err(store_error)? else {
        return Ok(Vec::new());
    };
    serde_json::from_slice(&bytes).map_err(|error| Error::Runtime(error.to_string()))
}

fn write_index(store: &mut StableEdgeStore, index: &[CacheIndexEntry]) -> Result<()> {
    let bytes = serde_json::to_vec(index).map_err(|error| Error::Runtime(error.to_string()))?;
    store.put_kv(CACHE_INDEX_KEY, &bytes).map_err(store_error)
}

fn cache_key(cache_name: &str, key: &str) -> String {
    format!("cache:{cache_name}\nGET\n{key}")
}

fn store_error(error: ic_edge_store::Error) -> Error {
    Error::Runtime(format!("{error:?}"))
}
