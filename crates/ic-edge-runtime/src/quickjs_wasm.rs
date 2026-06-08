//! `crates/ic-edge-runtime` runs QuickJS inside canister wasm.
//! It uses a prebuilt QuickJS wasm binding to avoid a C toolchain in canister builds.

use crate::{
    audit_polyfill, crypto_polyfill, fetch_queue_polyfill, json_polyfill, quickjs_wasm_audit,
    quickjs_wasm_cache, quickjs_wasm_crypto,
    quickjs_wasm_types::{HeaderPairs, RuntimeFetchRequest, RuntimeResponse},
    web_cache_polyfill, web_dispatch_polyfill, web_polyfill, web_url_polyfill, AsyncEdgeRuntime,
    AsyncHostFetch, AuditHost, CacheHost, EdgeRuntime,
};
use ic_edge_web::{limits, Error, Request, Response, Result};
use quickjs_wasm_rs::JSContextRef;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Wasm QuickJS runtime used inside canisters with the `quickjs-ic` feature.
pub struct QuickJsRuntime {
    context: JSContextRef,
    async_fetcher: Option<Box<dyn AsyncHostFetch>>,
    cache_host: Rc<RefCell<Option<Box<dyn CacheHost>>>>,
    audit_host: Rc<RefCell<Option<Box<dyn AuditHost>>>>,
}

impl QuickJsRuntime {
    /// Creates a canister-compatible QuickJS runtime.
    pub fn new() -> Result<Self> {
        let runtime = Self {
            context: JSContextRef::default(),
            async_fetcher: None,
            cache_host: Rc::new(RefCell::new(None)),
            audit_host: Rc::new(RefCell::new(None)),
        };
        runtime.install_web_polyfill()?;
        Ok(runtime)
    }

    /// Installs the async host fetch bridge.
    pub fn install_async_fetch<F>(&mut self, fetcher: F)
    where
        F: AsyncHostFetch + 'static,
    {
        self.async_fetcher = Some(Box::new(fetcher));
    }

    /// Installs a Cache API persistence backend.
    pub fn install_cache<C>(&mut self, cache: C)
    where
        C: CacheHost + 'static,
    {
        *self.cache_host.borrow_mut() = Some(Box::new(cache));
    }

    /// Installs an audit persistence backend.
    pub fn install_audit<A>(&mut self, audit: A)
    where
        A: AuditHost + 'static,
    {
        *self.audit_host.borrow_mut() = Some(Box::new(audit));
    }

    /// Installs the per-request random seed used by `crypto.getRandomValues`.
    pub fn install_random_seed(&self, seed: Vec<u8>) -> Result<()> {
        quickjs_wasm_crypto::install_random_seed(&self.context, seed)
    }

    /// Installs the per-request IC time in nanoseconds.
    pub fn install_time_nanos(&self, time_nanos: u64) -> Result<()> {
        let script = format!(
            "globalThis.ic ||= {{}}; globalThis.ic.time = () => BigInt({})",
            json_string(&time_nanos.to_string())?
        );
        self.context
            .eval_global("ic-time.js", &script)
            .map_err(to_runtime_error)?;
        Ok(())
    }

    /// Installs per-request IC identity values.
    pub fn install_ic_context(&self, caller: &str, canister_id: &str) -> Result<()> {
        self.context
            .eval_global("ic-context-init.js", "globalThis.ic ||= {}")
            .map_err(to_runtime_error)?;
        let caller_script = format!(
            "globalThis.ic.caller = function() {{ return {} }}",
            json_string(caller)?
        );
        self.context
            .eval_global("ic-caller.js", &caller_script)
            .map_err(to_runtime_error)?;
        let canister_script = format!(
            "globalThis.ic.canisterId = function() {{ return {} }}",
            json_string(canister_id)?
        );
        self.context
            .eval_global("ic-canister-id.js", &canister_script)
            .map_err(to_runtime_error)?;
        Ok(())
    }

    fn install_web_polyfill(&self) -> Result<()> {
        quickjs_wasm_crypto::install_callbacks(&self.context)?;
        quickjs_wasm_cache::install(&self.context, Rc::clone(&self.cache_host))?;
        quickjs_wasm_audit::install(&self.context, Rc::clone(&self.audit_host))?;
        self.eval_global("crypto.js", crypto_polyfill::SOURCE)?;
        self.eval_global("json.js", json_polyfill::SOURCE)?;
        self.eval_global("web.js", web_polyfill::SOURCE)?;
        self.eval_global("audit.js", audit_polyfill::SOURCE)?;
        self.eval_global("web-url.js", web_url_polyfill::SOURCE)?;
        self.eval_global("web-cache.js", web_cache_polyfill::SOURCE)?;
        self.eval_global("web-dispatch.js", web_dispatch_polyfill::SOURCE)?;
        self.eval_global("fetch-queue.js", fetch_queue_polyfill::SOURCE)?;
        Ok(())
    }

    fn eval_global(&self, name: &str, source: &str) -> Result<()> {
        self.context
            .eval_global(name, source)
            .map(|_| ())
            .map_err(to_runtime_error)
    }

    /// Evaluates QuickJS bytecode produced by the ic-edge bytecode compiler.
    pub fn eval_bytecode(&mut self, bytecode: &[u8]) -> Result<()> {
        self.context
            .eval_binary(bytecode)
            .map_err(to_runtime_error)?;
        self.context
            .eval_global(
                "app-default.js",
                "if (globalThis.__ic_edge_bundle?.default) globalThis.__ic_edge_app = globalThis.__ic_edge_bundle.default",
            )
            .map_err(to_runtime_error)?;
        let has_fetch = self
            .context
            .eval_global(
                "app-fetch-contract.js",
                "typeof globalThis.__ic_edge_app?.fetch === 'function'",
            )
            .map_err(to_runtime_error)?;
        if has_fetch.to_string() != "true" {
            return Err(Error::Runtime(
                "bytecode default export must expose fetch".to_string(),
            ));
        }
        Ok(())
    }

    fn drain_jobs(&self) -> Result<()> {
        self.context.execute_pending().map_err(to_runtime_error)
    }

    fn take_output(&self) -> Result<Response> {
        let error = self.eval_string(
            "take-error.js",
            "globalThis.__ic_edge_error === undefined ? '' : String(globalThis.__ic_edge_error)",
        )?;
        if !error.is_empty() {
            return Err(Error::Runtime(error));
        }
        let output = self.eval_string(
            "take-output.js",
            "globalThis.__ic_edge_output === undefined ? '' : String(globalThis.__ic_edge_output)",
        )?;
        RuntimeResponse::from_json(&output)
    }

    fn has_output(&self) -> Result<bool> {
        let error = self.eval_string(
            "has-error.js",
            "globalThis.__ic_edge_error === undefined ? '' : String(globalThis.__ic_edge_error)",
        )?;
        if !error.is_empty() {
            return Err(Error::Runtime(error));
        }
        let output = self
            .context
            .eval_global("has-output.js", "globalThis.__ic_edge_output !== undefined")
            .map_err(to_runtime_error)?;
        Ok(output.to_string() == "true")
    }

    fn dispatch_request(&self, request: Request) -> Result<()> {
        let headers_json = serde_json::to_string(&HeaderPairs::from_headers(&request.headers))
            .map_err(|error| Error::Runtime(error.to_string()))?;
        let body_json = serde_json::to_string(request.body.bytes())
            .map_err(|error| Error::Runtime(error.to_string()))?;
        let script = format!(
            "globalThis.__ic_edge_dispatch({}, {}, {}, {})",
            json_string(&request.method)?,
            json_string(&request.url)?,
            json_string(&headers_json)?,
            json_string(&body_json)?
        );
        self.context
            .eval_global("dispatch.js", &script)
            .map_err(to_runtime_error)?;
        Ok(())
    }

    fn take_fetch_requests(&self) -> Result<Vec<RuntimeFetchRequest>> {
        let output = self.eval_string(
            "take-fetch-requests.js",
            "String(globalThis.__ic_edge_take_fetch_requests())",
        )?;
        serde_json::from_str(&output).map_err(|error| {
            let prefix: String = output.chars().take(120).collect();
            Error::Runtime(format!("{error}; fetch prefix: {prefix}"))
        })
    }

    fn resolve_fetch(&self, id: u64, response: Response) -> Result<()> {
        let response = RuntimeResponse::from_response(response);
        let response_json =
            serde_json::to_string(&response).map_err(|error| Error::Runtime(error.to_string()))?;
        let script = format!(
            "globalThis.__ic_edge_resolve_fetch({}, {})",
            id,
            json_string(&response_json)?
        );
        self.context
            .eval_global("resolve-fetch.js", &script)
            .map_err(to_runtime_error)?;
        Ok(())
    }

    fn reject_fetch(&self, id: u64, message: &str) -> Result<()> {
        let script = format!(
            "globalThis.__ic_edge_reject_fetch({}, {})",
            id,
            json_string(message)?
        );
        self.context
            .eval_global("reject-fetch.js", &script)
            .map_err(to_runtime_error)?;
        Ok(())
    }

    fn eval_string(&self, name: &str, source: &str) -> Result<String> {
        let script = format!("new TextEncoder().encode({source}).buffer");
        let bytes: Vec<u8> = self
            .context
            .eval_global(name, &script)
            .map_err(to_runtime_error)?
            .try_into()
            .map_err(to_runtime_error)?;
        String::from_utf8(bytes).map_err(|error| Error::Runtime(error.to_string()))
    }
}

impl EdgeRuntime for QuickJsRuntime {
    fn eval_module(&mut self, _name: &str, source: &str) -> Result<()> {
        self.context
            .eval_global("app.js", source)
            .map_err(to_runtime_error)?;
        self.context
            .eval_global(
                "app-default.js",
                "if (globalThis.__ic_edge_bundle?.default) globalThis.__ic_edge_app = globalThis.__ic_edge_bundle.default",
            )
            .map_err(to_runtime_error)?;
        Ok(())
    }

    fn call_app_fetch(&mut self, request: Request) -> Result<Response> {
        self.dispatch_request(request)?;
        self.drain_jobs()?;
        self.take_output()
    }
}

impl AsyncEdgeRuntime for QuickJsRuntime {
    fn eval_module(&mut self, name: &str, source: &str) -> Result<()> {
        EdgeRuntime::eval_module(self, name, source)
    }

    fn call_app_fetch<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + 'a>> {
        Box::pin(async move {
            self.dispatch_request(request)?;
            let mut fetch_count = 0usize;
            loop {
                self.drain_jobs()?;
                if self.has_output()? {
                    return self.take_output();
                }

                let requests = self.take_fetch_requests()?;
                if requests.is_empty() {
                    return Err(Error::Runtime(
                        "app.fetch did not produce a response".to_string(),
                    ));
                }
                fetch_count += requests.len();
                if fetch_count > limits::MAX_FETCHES_PER_REQUEST {
                    return Err(Error::Runtime("fetch count exceeds v1 limit".to_string()));
                }

                for fetch_request in requests {
                    let fetch_id = fetch_request.id();
                    let options = fetch_request.options();
                    let request = fetch_request.to_request()?;
                    let result = match self.async_fetcher.as_mut() {
                        Some(fetcher) => fetcher.fetch(request, options).await,
                        None => Err(Error::Runtime("fetch is not configured".to_string())),
                    };
                    match result {
                        Ok(response) => self.resolve_fetch(fetch_id, response)?,
                        Err(error) => self.reject_fetch(fetch_id, &format!("{error:?}"))?,
                    }
                }
            }
        })
    }
}

fn json_string(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(|error| Error::Runtime(error.to_string()))
}

fn to_runtime_error(error: impl std::fmt::Display) -> Error {
    Error::Runtime(error.to_string())
}
