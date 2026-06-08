//! `crates/ic-edge-store` stores append-only audit events.
//! The log uses stable memory directly so receipt state survives upgrades.

use crate::{Error, Result, StableMemory};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableCell, StableLog};
use serde::Serialize;
use sha2::{Digest, Sha256};

const EMPTY_ROOT: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_AUDIT_EVENT_BYTES: usize = 64 * 1024;
const MAX_AUDIT_LIST_LIMIT: u64 = 100;

/// Stable append-only storage for public audit events.
pub struct StableAuditStore {
    log: StableLog<Vec<u8>, StableMemory, StableMemory>,
    id_index: StableBTreeMap<String, u64, StableMemory>,
    root: StableCell<String, StableMemory>,
}

/// Small, JSON-shaped audit event stored in the stable log.
#[derive(Serialize)]
struct AuditEvent<'a> {
    index: u64,
    id: &'a str,
    kind: &'a str,
    #[serde(rename = "payloadJson")]
    payload_json: &'a str,
    prev_root: &'a str,
    event_hash: &'a str,
    root: &'a str,
}

impl StableAuditStore {
    /// Creates audit storage using memory IDs 2, 3, 4, and 5.
    pub fn new(manager: &MemoryManager<DefaultMemoryImpl>) -> Self {
        Self {
            log: StableLog::init(manager.get(MemoryId::new(2)), manager.get(MemoryId::new(3))),
            id_index: StableBTreeMap::init(manager.get(MemoryId::new(4))),
            root: StableCell::init(manager.get(MemoryId::new(5)), EMPTY_ROOT.to_string()),
        }
    }

    /// Reserves an id before external settlement starts.
    pub fn reserve(&mut self, id: &str, payload_json: &str) -> Result<String> {
        if self.id_index.contains_key(&id.to_string()) {
            return Err(Error::StorageFailure);
        }
        self.append(id, "reserve", payload_json)
    }

    /// Commits the final settlement event for an existing reservation.
    pub fn commit(&mut self, id: &str, payload_json: &str) -> Result<String> {
        self.ensure_latest_kind(id, "reserve")?;
        self.append(id, "commit", payload_json)
    }

    /// Records a failed verification or settlement for an existing reservation.
    pub fn fail(&mut self, id: &str, payload_json: &str) -> Result<String> {
        self.ensure_latest_kind(id, "reserve")?;
        self.append(id, "fail", payload_json)
    }

    /// Reads the latest event for an id.
    pub fn get(&self, id: &str) -> Result<Option<String>> {
        let Some(index) = self.id_index.get(&id.to_string()) else {
            return Ok(None);
        };
        self.log.get(index).map(decode_event).transpose()
    }

    /// Reads a bounded slice of audit events.
    pub fn list(&self, offset: u64, limit: u64) -> Result<String> {
        let limit = limit.min(MAX_AUDIT_LIST_LIMIT);
        let end = offset.saturating_add(limit).min(self.log.len());
        let mut events = Vec::new();
        for index in offset..end {
            let Some(bytes) = self.log.get(index) else {
                break;
            };
            events.push(decode_event(bytes)?);
        }
        Ok(format!("[{}]", events.join(",")))
    }

    /// Returns the current hash root and event count JSON.
    pub fn root(&self) -> String {
        format!(
            r#"{{"root":"{}","count":{}}}"#,
            self.root.get(),
            self.log.len()
        )
    }

    fn append(&mut self, id: &str, kind: &str, payload_json: &str) -> Result<String> {
        validate_event(id, payload_json)?;
        let index = self.log.len();
        let prev_root = self.root.get().clone();
        let event_hash = event_hash(index, id, kind, payload_json, &prev_root);
        let next_root = root_hash(&prev_root, &event_hash);
        let event = AuditEvent {
            index,
            id,
            kind,
            payload_json,
            prev_root: &prev_root,
            event_hash: &event_hash,
            root: &next_root,
        };
        let json = serde_json::to_string(&event).map_err(|_| Error::StorageFailure)?;
        self.log
            .append(&json.clone().into_bytes())
            .map_err(|_| Error::StorageFailure)?;
        self.id_index.insert(id.to_string(), index);
        self.root.set(next_root);
        Ok(json)
    }

    fn ensure_latest_kind(&self, id: &str, kind: &str) -> Result<()> {
        let Some(event) = self.get(id)? else {
            return Err(Error::NotFound);
        };
        let value: serde_json::Value =
            serde_json::from_str(&event).map_err(|_| Error::StorageFailure)?;
        if value.get("kind").and_then(serde_json::Value::as_str) == Some(kind) {
            Ok(())
        } else {
            Err(Error::StorageFailure)
        }
    }
}

fn validate_event(id: &str, payload_json: &str) -> Result<()> {
    if id.is_empty() || id.len() > 256 || payload_json.len() > MAX_AUDIT_EVENT_BYTES {
        return Err(Error::StorageFailure);
    }
    Ok(())
}

fn decode_event(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| Error::StorageFailure)
}

fn event_hash(index: u64, id: &str, kind: &str, payload_json: &str, prev_root: &str) -> String {
    let input = format!("{index}\n{id}\n{kind}\n{payload_json}\n{prev_root}");
    hex_sha256(input.as_bytes())
}

fn root_hash(prev_root: &str, event_hash: &str) -> String {
    let mut bytes = Vec::with_capacity(prev_root.len() + event_hash.len());
    bytes.extend_from_slice(prev_root.as_bytes());
    bytes.extend_from_slice(event_hash.as_bytes());
    hex_sha256(&bytes)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_reserve_rejects_replay() {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut store = StableAuditStore::new(&manager);
        store.reserve("r1", "{}").unwrap();
        assert!(store.reserve("r1", "{}").is_err());
    }

    #[test]
    fn reserve_commit_and_fail_advance_root() {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut store = StableAuditStore::new(&manager);
        let before = store.root();
        store.reserve("r1", "{}").unwrap();
        store.commit("r1", r#"{"ok":true}"#).unwrap();
        store.reserve("r2", "{}").unwrap();
        store
            .fail("r2", r#"{"error":"settlement failed"}"#)
            .unwrap();
        let after = store.root();
        assert_ne!(before, after);
        assert!(after.contains(r#""count":4"#));
    }

    #[test]
    fn get_returns_latest_event() {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut store = StableAuditStore::new(&manager);
        store.reserve("r1", "{}").unwrap();
        store.commit("r1", r#"{"resultDigest":"abc"}"#).unwrap();
        let event = store.get("r1").unwrap().unwrap();
        assert!(event.contains(r#""kind":"commit""#));
    }

    #[test]
    fn list_returns_append_order_events() {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut store = StableAuditStore::new(&manager);
        store.reserve("r1", "{}").unwrap();
        store.reserve("r2", "{}").unwrap();
        assert!(store.list(1, 1).unwrap().contains(r#""id":"r2""#));
    }
}
