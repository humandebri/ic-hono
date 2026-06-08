//! `crates/ic-edge-runtime` exposes the `ic.audit` JavaScript API.
//! The Rust host owns storage; JavaScript receives JSON values.

pub(crate) const SOURCE: &str = r#"
globalThis.ic ||= {}
globalThis.ic.audit = {
  reserve(id, payloadJson = '{}') {
    return JSON.parse(globalThis.__ic_edge_audit_reserve(String(id), String(payloadJson)))
  },
  commit(id, payloadJson = '{}') {
    return JSON.parse(globalThis.__ic_edge_audit_commit(String(id), String(payloadJson)))
  },
  fail(id, payloadJson = '{}') {
    return JSON.parse(globalThis.__ic_edge_audit_fail(String(id), String(payloadJson)))
  },
  get(id) {
    const value = globalThis.__ic_edge_audit_get(String(id))
    return value ? JSON.parse(value) : null
  },
  list(offset = 0, limit = 50) {
    return JSON.parse(globalThis.__ic_edge_audit_list(Number(offset), Number(limit)))
  },
  root() {
    return JSON.parse(globalThis.__ic_edge_audit_root())
  }
}
"#;
