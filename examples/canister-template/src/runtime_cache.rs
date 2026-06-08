//! `examples/canister-template` caches the QuickJS runtime between update calls.
//! The cache is generation-scoped so upload/env changes rebuild the runtime.

use ic_edge_canister::OutcallReplication;
use ic_edge_runtime::{AsyncEdgeRuntime, AsyncHostFetch, HostFetchOptions, QuickJsRuntime};
use ic_edge_web::{Request, Response};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;

thread_local! {
    static RUNTIME_CACHE: RefCell<Option<CachedRuntime>> = const { RefCell::new(None) };
}

struct CachedRuntime {
    generation: u64,
    runtime: QuickJsRuntime,
}

pub(super) fn take_cached_runtime(generation: u64) -> Option<QuickJsRuntime> {
    if let Some(cached) = RUNTIME_CACHE.take() {
        if cached.generation == generation {
            return Some(cached.runtime);
        }
    }
    None
}

pub(super) fn build_runtime(
    env_source: &str,
    app_bytecode: &[u8],
) -> ic_edge_web::Result<QuickJsRuntime> {
    let mut runtime = QuickJsRuntime::new()?;
    runtime.install_async_fetch(OutcallFetch);
    runtime.install_cache(crate::cache_support::StableCacheHost);
    runtime.install_audit(crate::audit_support::StableAuditHost);
    runtime.eval_module("env", env_source)?;
    runtime.eval_bytecode(app_bytecode)?;
    Ok(runtime)
}

pub(super) fn store_runtime(generation: u64, runtime: QuickJsRuntime) {
    RUNTIME_CACHE.set(Some(CachedRuntime {
        generation,
        runtime,
    }));
}

struct OutcallFetch;

impl AsyncHostFetch for OutcallFetch {
    fn fetch<'a>(
        &'a mut self,
        request: Request,
        options: HostFetchOptions,
    ) -> Pin<Box<dyn Future<Output = ic_edge_web::Result<Response>> + 'a>> {
        Box::pin(async move {
            let replication = if options.replicated {
                OutcallReplication::Replicated
            } else {
                OutcallReplication::NonReplicated
            };
            crate::https_outcall_fetch_with_replication(
                request,
                "transform_strip_headers",
                Some(64 * 1024),
                replication,
            )
            .await
        })
    }
}
