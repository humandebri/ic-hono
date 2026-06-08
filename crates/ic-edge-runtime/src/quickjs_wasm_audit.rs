//! `crates/ic-edge-runtime` installs wasm QuickJS audit callbacks.
//! Canister hosts supply durable append-only audit behavior.

use crate::AuditHost;
use ic_edge_web::{Error, Result};
use quickjs_wasm_rs::{JSContextRef, JSError, JSValue, JSValueRef};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn install(
    context: &JSContextRef,
    audit_host: Rc<RefCell<Option<Box<dyn AuditHost>>>>,
) -> Result<()> {
    let global = context.global_object().map_err(to_runtime_error)?;
    set_two_arg(
        context,
        &global,
        "__ic_edge_audit_reserve",
        Rc::clone(&audit_host),
        |audit, id, payload| audit.reserve(id, payload),
    )?;
    set_two_arg(
        context,
        &global,
        "__ic_edge_audit_commit",
        Rc::clone(&audit_host),
        |audit, id, payload| audit.commit(id, payload),
    )?;
    set_two_arg(
        context,
        &global,
        "__ic_edge_audit_fail",
        Rc::clone(&audit_host),
        |audit, id, payload| audit.fail(id, payload),
    )?;
    let get_host = Rc::clone(&audit_host);
    global
        .set_property(
            "__ic_edge_audit_get",
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let id = arg_string(args, 0).map_err(to_js_error)?;
                    let event = with_audit(&get_host, |audit| audit.get(&id))?.unwrap_or_default();
                    Ok(JSValue::String(event))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    let list_host = Rc::clone(&audit_host);
    global
        .set_property(
            "__ic_edge_audit_list",
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let offset = arg_u64(args, 0).map_err(to_js_error)?;
                    let limit = arg_u64(args, 1).map_err(to_js_error)?;
                    let events = with_audit(&list_host, |audit| audit.list(offset, limit))?;
                    Ok(JSValue::String(events))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    global
        .set_property(
            "__ic_edge_audit_root",
            context
                .wrap_callback(move |_ctx, _this, _args| {
                    let root = with_audit(&audit_host, |audit| audit.root())?;
                    Ok(JSValue::String(root))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)
}

fn set_two_arg(
    context: &JSContextRef,
    global: &JSValueRef,
    name: &str,
    audit_host: Rc<RefCell<Option<Box<dyn AuditHost>>>>,
    operation: fn(&mut dyn AuditHost, &str, &str) -> Result<String>,
) -> Result<()> {
    global
        .set_property(
            name,
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let id = arg_string(args, 0).map_err(to_js_error)?;
                    let payload = arg_string(args, 1).map_err(to_js_error)?;
                    let event = with_audit(&audit_host, |audit| operation(audit, &id, &payload))?;
                    Ok(JSValue::String(event))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)
}

fn with_audit<T>(
    audit_host: &Rc<RefCell<Option<Box<dyn AuditHost>>>>,
    operation: impl FnOnce(&mut dyn AuditHost) -> Result<T>,
) -> std::result::Result<T, JSError> {
    let mut borrowed = audit_host.borrow_mut();
    let audit = borrowed
        .as_mut()
        .ok_or_else(|| to_js_error(Error::Runtime("audit is not configured".to_string())))?;
    operation(audit.as_mut()).map_err(to_js_error)
}

fn arg_string(args: &[JSValueRef], index: usize) -> Result<String> {
    args.get(index)
        .map(|value| value.to_string())
        .ok_or_else(|| Error::Runtime(format!("missing audit argument {index}")))
}

fn arg_u64(args: &[JSValueRef], index: usize) -> Result<u64> {
    arg_string(args, index)?
        .parse::<u64>()
        .map_err(|error| Error::Runtime(error.to_string()))
}

fn to_runtime_error(error: impl std::fmt::Display) -> Error {
    Error::Runtime(error.to_string())
}

fn to_js_error(error: Error) -> JSError {
    JSError::Internal(format!("{error:?}"))
}
