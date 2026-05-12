//! `crates/ic-edge-store` defines durable storage boundaries.
//! The API stores runtime assets without exposing a generic filesystem.

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use std::collections::BTreeMap as StdBTreeMap;

/// Result type used by storage backends.
pub type Result<T> = std::result::Result<T, Error>;

/// Storage errors exposed by the v1 preview store API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Requested module does not exist.
    NotFound,
    /// Stable or memory storage failed.
    StorageFailure,
}

/// Storage boundary for bundles and runtime key-value data.
pub trait EdgeStore {
    /// Reads a stored bundle module.
    fn get_module(&self, path: &str) -> Result<Vec<u8>>;
    /// Stores a bundle module.
    fn put_module(&mut self, path: &str, bytes: &[u8]) -> Result<()>;
    /// Reads optional key-value data.
    fn get_kv(&self, key: &str) -> Result<Option<Vec<u8>>>;
    /// Stores key-value data.
    fn put_kv(&mut self, key: &str, value: &[u8]) -> Result<()>;
    /// Deletes key-value data.
    fn delete_kv(&mut self, key: &str) -> Result<()>;
}

/// In-memory store for host tests and local tooling.
#[derive(Debug, Default)]
pub struct MemoryEdgeStore {
    modules: StdBTreeMap<String, Vec<u8>>,
    kv: StdBTreeMap<String, Vec<u8>>,
}

impl MemoryEdgeStore {
    /// Creates an empty memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

type StableMemory = VirtualMemory<DefaultMemoryImpl>;

/// Stable-memory store for canister templates.
pub struct StableEdgeStore {
    modules: StableBTreeMap<String, Vec<u8>, StableMemory>,
    kv: StableBTreeMap<String, Vec<u8>, StableMemory>,
}

impl StableEdgeStore {
    /// Creates a stable store using memory IDs 0 and 1.
    pub fn new() -> Self {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        Self {
            modules: StableBTreeMap::init(manager.get(MemoryId::new(0))),
            kv: StableBTreeMap::init(manager.get(MemoryId::new(1))),
        }
    }
}

impl Default for StableEdgeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeStore for StableEdgeStore {
    fn get_module(&self, path: &str) -> Result<Vec<u8>> {
        self.modules.get(&path.to_string()).ok_or(Error::NotFound)
    }

    fn put_module(&mut self, path: &str, bytes: &[u8]) -> Result<()> {
        self.modules.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn get_kv(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.kv.get(&key.to_string()))
    }

    fn put_kv(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.kv.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete_kv(&mut self, key: &str) -> Result<()> {
        self.kv.remove(&key.to_string());
        Ok(())
    }
}

impl EdgeStore for MemoryEdgeStore {
    fn get_module(&self, path: &str) -> Result<Vec<u8>> {
        self.modules.get(path).cloned().ok_or(Error::NotFound)
    }

    fn put_module(&mut self, path: &str, bytes: &[u8]) -> Result<()> {
        self.modules.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn get_kv(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.kv.get(key).cloned())
    }

    fn put_kv(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.kv.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete_kv(&mut self, key: &str) -> Result<()> {
        self.kv.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_modules_and_kv_values() {
        let mut store = MemoryEdgeStore::new();
        store.put_module("app", b"bundle").unwrap();
        store.put_kv("key", b"value").unwrap();
        assert_eq!(store.get_module("app").unwrap(), b"bundle");
        assert_eq!(store.get_kv("key").unwrap(), Some(b"value".to_vec()));
        store.delete_kv("key").unwrap();
        assert_eq!(store.get_kv("key").unwrap(), None);
    }

    #[test]
    fn stable_store_keeps_modules_and_kv_values() {
        let mut store = StableEdgeStore::new();
        store.put_module("app", b"bundle").unwrap();
        store.put_kv("key", b"value").unwrap();
        assert_eq!(store.get_module("app").unwrap(), b"bundle");
        assert_eq!(store.get_kv("key").unwrap(), Some(b"value".to_vec()));
    }
}
