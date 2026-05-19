//! `examples/canister-template` backs `ic.audit` with stable memory.
//! Audit events are append-only and public through application routes.
#![cfg_attr(
    not(all(target_arch = "wasm32", feature = "quickjs-ic")),
    allow(dead_code)
)]

use ic_edge_runtime::AuditHost;
use ic_edge_web::{Error, Result};

pub(crate) struct StableAuditHost;

impl AuditHost for StableAuditHost {
    fn reserve(&mut self, id: &str, payload_json: &str) -> Result<String> {
        crate::STORE
            .with_borrow_mut(|store| store.audit_reserve(id, payload_json))
            .map_err(store_error)
    }

    fn commit(&mut self, id: &str, payload_json: &str) -> Result<String> {
        crate::STORE
            .with_borrow_mut(|store| store.audit_commit(id, payload_json))
            .map_err(store_error)
    }

    fn fail(&mut self, id: &str, payload_json: &str) -> Result<String> {
        crate::STORE
            .with_borrow_mut(|store| store.audit_fail(id, payload_json))
            .map_err(store_error)
    }

    fn get(&mut self, id: &str) -> Result<Option<String>> {
        crate::STORE
            .with_borrow(|store| store.audit_get(id))
            .map_err(store_error)
    }

    fn list(&mut self, offset: u64, limit: u64) -> Result<String> {
        crate::STORE
            .with_borrow(|store| store.audit_list(offset, limit))
            .map_err(store_error)
    }

    fn root(&mut self) -> Result<String> {
        Ok(crate::STORE.with_borrow(|store| store.audit_root()))
    }
}

fn store_error(error: ic_edge_store::Error) -> Error {
    Error::Runtime(format!("{error:?}"))
}
