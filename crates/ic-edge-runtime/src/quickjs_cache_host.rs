//! `crates/ic-edge-runtime` installs host QuickJS Cache API callbacks.
//! The default host cache is in-memory and scoped to one runtime instance.

use crate::CacheHost;
use ic_edge_web::{limits, Error, Result};
use rquickjs::{Context, Function};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(crate) fn install(
    context: &Context,
    cache_host: Rc<RefCell<Box<dyn CacheHost>>>,
) -> Result<()> {
    context
        .with(|ctx| {
            let global = ctx.globals();
            let match_host = Rc::clone(&cache_host);
            global.set(
                "__ic_edge_cache_match",
                Function::new(ctx.clone(), move |name: String, key: String| {
                    match_host
                        .borrow_mut()
                        .match_entry(&name, &key)
                        .map(|value| value.unwrap_or_default())
                        .map_err(to_js_error)
                })?,
            )?;
            let put_host = Rc::clone(&cache_host);
            global.set(
                "__ic_edge_cache_put",
                Function::new(
                    ctx.clone(),
                    move |name: String, key: String, value: String| {
                        put_host
                            .borrow_mut()
                            .put_entry(&name, &key, &value)
                            .map_err(to_js_error)
                    },
                )?,
            )?;
            let delete_host = Rc::clone(&cache_host);
            global.set(
                "__ic_edge_cache_delete",
                Function::new(ctx.clone(), move |name: String, key: String| {
                    delete_host
                        .borrow_mut()
                        .delete_entry(&name, &key)
                        .map_err(to_js_error)
                })?,
            )
        })
        .map_err(|error| Error::Runtime(error.to_string()))
}

#[derive(Default)]
pub(crate) struct MemoryCacheHost {
    entries: BTreeMap<String, String>,
}

impl CacheHost for MemoryCacheHost {
    fn match_entry(&mut self, cache_name: &str, key: &str) -> Result<Option<String>> {
        Ok(self.entries.get(&cache_key(cache_name, key)).cloned())
    }

    fn put_entry(&mut self, cache_name: &str, key: &str, response_json: &str) -> Result<()> {
        if response_json.len() > limits::MAX_CACHE_ENTRY_BYTES {
            return Err(Error::Runtime("cache entry exceeds v1 limit".to_string()));
        }
        let old_size = self
            .entries
            .get(&cache_key(cache_name, key))
            .map(|value| value.len())
            .unwrap_or(0);
        let total =
            self.entries.values().map(String::len).sum::<usize>() - old_size + response_json.len();
        if total > limits::MAX_CACHE_TOTAL_BYTES {
            return Err(Error::Runtime("cache total exceeds v1 limit".to_string()));
        }
        self.entries
            .insert(cache_key(cache_name, key), response_json.to_string());
        Ok(())
    }

    fn delete_entry(&mut self, cache_name: &str, key: &str) -> Result<bool> {
        Ok(self.entries.remove(&cache_key(cache_name, key)).is_some())
    }
}

fn cache_key(cache_name: &str, key: &str) -> String {
    format!("{cache_name}\nGET\n{key}")
}

fn to_js_error(error: Error) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("Rust", "JavaScript", format!("{error:?}"))
}
