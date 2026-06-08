//! `crates/ic-edge-runtime` installs host QuickJS audit callbacks.
//! The default host is deterministic memory storage for tests and examples.

use crate::AuditHost;
use ic_edge_web::{Error, Result};
use rquickjs::{Context, Function};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const EMPTY_ROOT: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_EVENTS: usize = 10_000;
const MAX_EVENT_BYTES: usize = 64 * 1024;

pub(crate) fn install(
    context: &Context,
    audit_host: Rc<RefCell<Box<dyn AuditHost>>>,
) -> Result<()> {
    context
        .with(|ctx| {
            let global = ctx.globals();
            let reserve_host = Rc::clone(&audit_host);
            global.set(
                "__ic_edge_audit_reserve",
                Function::new(ctx.clone(), move |id: String, payload: String| {
                    reserve_host
                        .borrow_mut()
                        .reserve(&id, &payload)
                        .map_err(to_js_error)
                })?,
            )?;
            let commit_host = Rc::clone(&audit_host);
            global.set(
                "__ic_edge_audit_commit",
                Function::new(ctx.clone(), move |id: String, payload: String| {
                    commit_host
                        .borrow_mut()
                        .commit(&id, &payload)
                        .map_err(to_js_error)
                })?,
            )?;
            let fail_host = Rc::clone(&audit_host);
            global.set(
                "__ic_edge_audit_fail",
                Function::new(ctx.clone(), move |id: String, payload: String| {
                    fail_host
                        .borrow_mut()
                        .fail(&id, &payload)
                        .map_err(to_js_error)
                })?,
            )?;
            let get_host = Rc::clone(&audit_host);
            global.set(
                "__ic_edge_audit_get",
                Function::new(ctx.clone(), move |id: String| {
                    get_host
                        .borrow_mut()
                        .get(&id)
                        .map(|value| value.unwrap_or_default())
                        .map_err(to_js_error)
                })?,
            )?;
            let list_host = Rc::clone(&audit_host);
            global.set(
                "__ic_edge_audit_list",
                Function::new(ctx.clone(), move |offset: u64, limit: u64| {
                    list_host
                        .borrow_mut()
                        .list(offset, limit)
                        .map_err(to_js_error)
                })?,
            )?;
            global.set(
                "__ic_edge_audit_root",
                Function::new(ctx.clone(), move || {
                    audit_host.borrow_mut().root().map_err(to_js_error)
                })?,
            )
        })
        .map_err(|error| Error::Runtime(error.to_string()))
}

#[derive(Default)]
pub(crate) struct MemoryAuditHost {
    events: Vec<String>,
    latest: BTreeMap<String, usize>,
    root: String,
}

impl AuditHost for MemoryAuditHost {
    fn reserve(&mut self, id: &str, payload_json: &str) -> Result<String> {
        if self.latest.contains_key(id) {
            return Err(Error::Runtime("audit id already exists".to_string()));
        }
        self.append(id, "reserve", payload_json)
    }

    fn commit(&mut self, id: &str, payload_json: &str) -> Result<String> {
        self.ensure_latest_kind(id, "reserve")?;
        self.append(id, "commit", payload_json)
    }

    fn fail(&mut self, id: &str, payload_json: &str) -> Result<String> {
        self.ensure_latest_kind(id, "reserve")?;
        self.append(id, "fail", payload_json)
    }

    fn get(&mut self, id: &str) -> Result<Option<String>> {
        Ok(self
            .latest
            .get(id)
            .and_then(|index| self.events.get(*index).cloned()))
    }

    fn list(&mut self, offset: u64, limit: u64) -> Result<String> {
        let start = usize::try_from(offset)
            .map_err(|_| Error::Runtime("audit offset exceeds usize".to_string()))?;
        let limit = usize::try_from(limit.min(100))
            .map_err(|_| Error::Runtime("audit limit exceeds usize".to_string()))?;
        let events = self
            .events
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        Ok(format!("[{}]", events.join(",")))
    }

    fn root(&mut self) -> Result<String> {
        Ok(format!(
            r#"{{"root":"{}","count":{}}}"#,
            self.current_root(),
            self.events.len()
        ))
    }
}

impl MemoryAuditHost {
    fn append(&mut self, id: &str, kind: &str, payload_json: &str) -> Result<String> {
        if id.is_empty() || payload_json.len() > MAX_EVENT_BYTES || self.events.len() >= MAX_EVENTS
        {
            return Err(Error::Runtime("audit event exceeds v1 limit".to_string()));
        }
        let index = self.events.len();
        let prev_root = self.current_root();
        let event_hash = event_hash(index, id, kind, payload_json, &prev_root);
        let root = root_hash(&prev_root, &event_hash);
        let event = format!(
            r#"{{"index":{index},"id":"{}","kind":"{kind}","payloadJson":"{}","prev_root":"{prev_root}","event_hash":"{event_hash}","root":"{root}"}}"#,
            json_escape(id),
            json_escape(payload_json),
        );
        self.events.push(event.clone());
        self.latest.insert(id.to_string(), index);
        self.root = root;
        Ok(event)
    }

    fn ensure_latest_kind(&mut self, id: &str, kind: &str) -> Result<()> {
        let Some(event) = self.get(id)? else {
            return Err(Error::Runtime("audit id not found".to_string()));
        };
        if event.contains(&format!(r#""kind":"{kind}""#)) {
            Ok(())
        } else {
            Err(Error::Runtime("audit id is already finalized".to_string()))
        }
    }

    fn current_root(&self) -> String {
        if self.root.is_empty() {
            EMPTY_ROOT.to_string()
        } else {
            self.root.clone()
        }
    }
}

fn event_hash(index: usize, id: &str, kind: &str, payload_json: &str, prev_root: &str) -> String {
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

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn to_js_error(error: Error) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("Rust", "JavaScript", format!("{error:?}"))
}
