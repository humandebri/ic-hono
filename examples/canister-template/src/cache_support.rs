//! `examples/canister-template` backs the Worker Cache API with stable KV.
//! The cache is canister-local durable storage, not a global CDN cache.
#![cfg_attr(
    not(all(target_arch = "wasm32", feature = "quickjs-ic")),
    allow(dead_code)
)]

use ic_edge_runtime::CacheHost;
use ic_edge_store::EdgeStore;
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

fn cache_match(store: &impl EdgeStore, cache_name: &str, key: &str) -> Result<Option<String>> {
    let storage_key = cache_key(cache_name, key)?;
    store
        .get_kv(&storage_key)
        .map_err(store_error)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| Error::Runtime(error.to_string()))
}

fn cache_put(
    store: &mut impl EdgeStore,
    cache_name: &str,
    key: &str,
    response_json: &str,
) -> Result<()> {
    let size = response_json.len();
    if size > limits::MAX_CACHE_ENTRY_BYTES {
        return Err(Error::Runtime("cache entry exceeds v1 limit".to_string()));
    }
    let mut index = read_index(store)?;
    let existing = index
        .iter()
        .find(|entry| entry.cache_name == cache_name && entry.key == key);
    let old_size = existing.map(|entry| entry.size).unwrap_or(0);
    let total = index.iter().map(|entry| entry.size).sum::<usize>() - old_size + size;
    if total > limits::MAX_CACHE_TOTAL_BYTES {
        return Err(Error::Runtime("cache total exceeds v1 limit".to_string()));
    }
    let storage_key = cache_key(cache_name, key)?;
    if existing.is_none() && index.len() >= limits::MAX_CACHE_INDEX_ENTRIES {
        return Err(Error::Runtime("cache index exceeds v1 limit".to_string()));
    }
    index.retain(|entry| !(entry.cache_name == cache_name && entry.key == key));
    index.push(CacheIndexEntry {
        cache_name: cache_name.to_string(),
        key: key.to_string(),
        size,
    });
    store
        .put_kv(&storage_key, response_json.as_bytes())
        .map_err(store_error)?;
    write_index(store, &index)
}

fn cache_delete(store: &mut impl EdgeStore, cache_name: &str, key: &str) -> Result<bool> {
    let storage_key = cache_key(cache_name, key)?;
    let mut index = read_index(store)?;
    let before = index.len();
    index.retain(|entry| !(entry.cache_name == cache_name && entry.key == key));
    let deleted = index.len() != before;
    if deleted {
        store.delete_kv(&storage_key).map_err(store_error)?;
        write_index(store, &index)?;
    }
    Ok(deleted)
}

fn read_index(store: &impl EdgeStore) -> Result<Vec<CacheIndexEntry>> {
    let Some(bytes) = store.get_kv(CACHE_INDEX_KEY).map_err(store_error)? else {
        return Ok(Vec::new());
    };
    if bytes.len() > limits::MAX_CACHE_INDEX_BYTES {
        return Err(Error::Runtime(
            "cache index bytes exceed v1 limit".to_string(),
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| Error::Runtime(error.to_string()))
}

fn write_index(store: &mut impl EdgeStore, index: &[CacheIndexEntry]) -> Result<()> {
    if index.len() > limits::MAX_CACHE_INDEX_ENTRIES {
        return Err(Error::Runtime("cache index exceeds v1 limit".to_string()));
    }
    let bytes = serde_json::to_vec(index).map_err(|error| Error::Runtime(error.to_string()))?;
    if bytes.len() > limits::MAX_CACHE_INDEX_BYTES {
        return Err(Error::Runtime(
            "cache index bytes exceed v1 limit".to_string(),
        ));
    }
    store.put_kv(CACHE_INDEX_KEY, &bytes).map_err(store_error)
}

fn cache_key(cache_name: &str, key: &str) -> Result<String> {
    validate_cache_key_parts(cache_name, key)?;
    let encoded = serde_json::to_string(&(cache_name, "GET", key))
        .map_err(|error| Error::Runtime(error.to_string()))?;
    Ok(format!("cache:{encoded}"))
}

fn validate_cache_key_parts(cache_name: &str, key: &str) -> Result<()> {
    if cache_name.len() > limits::MAX_CACHE_NAME_BYTES {
        return Err(Error::Runtime("cache name exceeds v1 limit".to_string()));
    }
    if key.len() > limits::MAX_CACHE_KEY_BYTES {
        return Err(Error::Runtime("cache key exceeds v1 limit".to_string()));
    }
    Ok(())
}

fn store_error(error: ic_edge_store::Error) -> Error {
    Error::Runtime(format!("{error:?}"))
}

#[cfg(test)]
mod limit_tests {
    use super::*;
    use ic_edge_store::MemoryEdgeStore;

    #[test]
    fn cache_rejects_oversized_name_and_key() {
        let mut store = MemoryEdgeStore::new();
        let long_name = "n".repeat(limits::MAX_CACHE_NAME_BYTES + 1);
        let long_key = "k".repeat(limits::MAX_CACHE_KEY_BYTES + 1);
        assert!(cache_put(&mut store, &long_name, "https://cache.test", "{}").is_err());
        assert!(cache_put(&mut store, "default", &long_key, "{}").is_err());
    }

    #[test]
    fn cache_rejects_too_many_index_entries() {
        let mut store = MemoryEdgeStore::new();
        for index in 0..limits::MAX_CACHE_INDEX_ENTRIES {
            cache_put(
                &mut store,
                "default",
                &format!("https://cache.test/{index}"),
                "{}",
            )
            .unwrap();
        }
        assert!(cache_put(&mut store, "default", "https://cache.test/overflow", "{}",).is_err());
        assert!(cache_put(
            &mut store,
            "default",
            "https://cache.test/0",
            "{\"ok\":true}",
        )
        .is_ok());
    }

    #[test]
    fn cache_put_match_and_delete_roundtrip() {
        let mut store = MemoryEdgeStore::new();
        cache_put(&mut store, "default", "/a", r#"{"status":200}"#).unwrap();
        assert_eq!(
            cache_match(&store, "default", "/a").unwrap(),
            Some(r#"{"status":200}"#.to_string())
        );
        assert!(cache_delete(&mut store, "default", "/a").unwrap());
        assert_eq!(cache_match(&store, "default", "/a").unwrap(), None);
        assert!(!cache_delete(&mut store, "default", "/a").unwrap());
    }

    #[test]
    fn cache_put_rejects_entry_above_limit() {
        let mut store = MemoryEdgeStore::new();
        let value = "x".repeat(limits::MAX_CACHE_ENTRY_BYTES + 1);
        let err = cache_put(&mut store, "default", "/large", &value).unwrap_err();
        assert!(
            matches!(err, Error::Runtime(message) if message == "cache entry exceeds v1 limit")
        );
    }

    #[test]
    fn cache_replacement_subtracts_previous_size() {
        let mut store = MemoryEdgeStore::new();
        let large = "x".repeat(limits::MAX_CACHE_ENTRY_BYTES);
        cache_put(&mut store, "default", "/same", &large).unwrap();
        cache_put(&mut store, "default", "/same", "small").unwrap();
        assert_eq!(
            cache_match(&store, "default", "/same").unwrap(),
            Some("small".to_string())
        );
    }
}
