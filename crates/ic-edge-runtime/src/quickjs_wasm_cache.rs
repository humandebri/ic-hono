//! `crates/ic-edge-runtime` installs wasm QuickJS Cache API callbacks.
//! The canister host supplies durable cache behavior through `CacheHost`.

use crate::CacheHost;
use ic_edge_web::{Error, Result};
use quickjs_wasm_rs::{JSContextRef, JSError, JSValue, JSValueRef};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn install(
    context: &JSContextRef,
    cache_host: Rc<RefCell<Option<Box<dyn CacheHost>>>>,
) -> Result<()> {
    let global = context.global_object().map_err(to_runtime_error)?;
    let match_host = Rc::clone(&cache_host);
    global
        .set_property(
            "__ic_edge_cache_match",
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let key = arg_string(args, 1).map_err(to_js_error)?;
                    let value = with_cache(&match_host, |cache| cache.match_entry(&name, &key))?
                        .unwrap_or_default();
                    Ok(JSValue::String(value))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    let put_host = Rc::clone(&cache_host);
    global
        .set_property(
            "__ic_edge_cache_put",
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let key = arg_string(args, 1).map_err(to_js_error)?;
                    let value = arg_string(args, 2).map_err(to_js_error)?;
                    with_cache(&put_host, |cache| cache.put_entry(&name, &key, &value))?;
                    Ok(JSValue::Bool(true))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    let delete_host = Rc::clone(&cache_host);
    global
        .set_property(
            "__ic_edge_cache_delete",
            context
                .wrap_callback(move |_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let key = arg_string(args, 1).map_err(to_js_error)?;
                    let deleted =
                        with_cache(&delete_host, |cache| cache.delete_entry(&name, &key))?;
                    Ok(JSValue::Bool(deleted))
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)
}

fn with_cache<T>(
    cache_host: &Rc<RefCell<Option<Box<dyn CacheHost>>>>,
    operation: impl FnOnce(&mut dyn CacheHost) -> Result<T>,
) -> std::result::Result<T, JSError> {
    let mut borrowed = cache_host.borrow_mut();
    let cache = borrowed
        .as_mut()
        .ok_or_else(|| to_js_error(Error::Runtime("cache is not configured".to_string())))?;
    operation(cache.as_mut()).map_err(to_js_error)
}

fn arg_string(args: &[JSValueRef], index: usize) -> Result<String> {
    args.get(index)
        .map(|value| value.to_string())
        .ok_or_else(|| Error::Runtime(format!("missing cache argument {index}")))
}

fn to_runtime_error(error: impl std::fmt::Display) -> Error {
    Error::Runtime(error.to_string())
}

fn to_js_error(error: Error) -> JSError {
    JSError::Internal(format!("{error:?}"))
}
