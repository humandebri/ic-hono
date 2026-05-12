//! `crates/ic-edge-runtime` implements the host QuickJS runtime.
//! It is target-gated until QuickJS can be ported into canister wasm.

use crate::quickjs_cache_host::{install as install_cache_host, MemoryCacheHost};
use crate::{
    crypto_polyfill, json_polyfill, web_cache_polyfill, web_dispatch_polyfill, web_polyfill,
    web_url_polyfill, CacheHost, EdgeRuntime,
};
use ic_edge_web::{limits, Body, Error, Headers, Request, Response, Result};
use rquickjs::{Context, Exception, FromJs, Function, Runtime};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

/// Synchronous host fetch implementation for host QuickJS tests and examples.
pub trait HostFetch {
    /// Performs an external fetch for a JavaScript `fetch()` request.
    fn fetch(&mut self, request: Request) -> Result<Response>;
}

/// Host QuickJS runtime for bundled Worker-compatible apps.
pub struct QuickJsRuntime {
    _runtime: Runtime,
    context: Context,
    cache_host: Rc<RefCell<Box<dyn CacheHost>>>,
}

/// Evaluates a bundle and verifies the v1 IIFE app contract.
pub fn validate_bundle_contract(source: &str) -> Result<()> {
    let mut runtime = QuickJsRuntime::new()?;
    runtime.install_contract_env()?;
    runtime.eval_module("contract", source)?;
    let valid = runtime
        .context
        .with(|ctx| {
            ctx.eval::<bool, _>(
                "typeof globalThis.__ic_edge_bundle === 'object' &&
                 typeof globalThis.__ic_edge_bundle.default === 'object' &&
                 typeof globalThis.__ic_edge_bundle.default.fetch === 'function'",
            )
        })
        .map_err(to_runtime_error)?;
    if valid {
        Ok(())
    } else {
        Err(Error::Runtime(
            "bundle contract requires __ic_edge_bundle.default.fetch function".to_string(),
        ))
    }
}

impl QuickJsRuntime {
    /// Creates a runtime with Web API, Cache, crypto, and dispatch polyfills installed.
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new().map_err(to_runtime_error)?;
        let context = Context::full(&runtime).map_err(to_runtime_error)?;
        let instance = Self {
            _runtime: runtime,
            context,
            cache_host: Rc::new(RefCell::new(Box::new(MemoryCacheHost::default()))),
        };
        instance.install_web_polyfill()?;
        Ok(instance)
    }

    /// Installs a custom Cache API persistence backend.
    pub fn install_cache<C>(&self, cache: C)
    where
        C: CacheHost + 'static,
    {
        *self.cache_host.borrow_mut() = Box::new(cache);
    }

    /// Installs a host fetch callback for JavaScript `fetch()`.
    pub fn install_fetch<F>(&self, fetcher: F) -> Result<()>
    where
        F: HostFetch + 'static,
    {
        let fetcher = Rc::new(RefCell::new(fetcher));
        self.context
            .with(|ctx| {
                let global = ctx.globals();
                let callback_fetcher = Rc::clone(&fetcher);
                let callback = Function::new(ctx.clone(), move |method, url, headers, body| {
                    host_fetch(&callback_fetcher, method, url, headers, body)
                })?;
                global.set("__ic_edge_host_fetch", callback)
            })
            .map_err(to_runtime_error)
    }

    /// Takes the last captured `console.error` text.
    pub fn take_console_error(&self) -> Result<Option<String>> {
        self.context
            .with(|ctx| ctx.globals().get("__ic_edge_console_error"))
            .map_err(to_runtime_error)
    }

    fn install_web_polyfill(&self) -> Result<()> {
        self.install_crypto_host()?;
        install_cache_host(&self.context, Rc::clone(&self.cache_host))?;
        self.context
            .with(|ctx| {
                ctx.eval::<(), _>(crypto_polyfill::SOURCE)?;
                ctx.eval::<(), _>(json_polyfill::SOURCE)?;
                ctx.eval::<(), _>(web_polyfill::SOURCE)?;
                ctx.eval::<(), _>(web_url_polyfill::SOURCE)?;
                ctx.eval::<(), _>(web_cache_polyfill::SOURCE)?;
                ctx.eval::<(), _>(web_dispatch_polyfill::SOURCE)
            })
            .map_err(to_runtime_error)
    }

    fn install_contract_env(&self) -> Result<()> {
        self.context
            .with(|ctx| {
                ctx.eval::<(), _>(
                    "globalThis.process ||= {};
                     globalThis.process.env = new Proxy(globalThis.process.env || {}, {
                       get(target, prop) {
                         if (prop in target) return target[prop]
                         if (typeof prop !== 'string') return undefined
                         if (prop.endsWith('_URL')) return 'https://example.test'
                         if (prop.endsWith('_MODEL')) return 'test-model'
                         if (prop.endsWith('_API_KEY')) return 'sk-ic-edge-contract-smoke'
                         return 'ic-edge-contract-smoke'
                       }
                     })",
                )
            })
            .map_err(to_runtime_error)
    }

    fn install_crypto_host(&self) -> Result<()> {
        crate::crypto_host::install(&self.context)
    }

    fn drain_jobs(&self) -> Result<()> {
        while self._runtime.is_job_pending() {
            self._runtime
                .execute_pending_job()
                .map_err(|error| Error::Runtime(error.to_string()))?;
        }
        Ok(())
    }

    fn take_output(&self) -> Result<Response> {
        self.context
            .with(|ctx| {
                let global = ctx.globals();
                let error: Option<String> = global.get("__ic_edge_error")?;
                if let Some(error) = error {
                    return Err(rquickjs::Error::new_from_js_message(
                        "Promise", "Response", error,
                    ));
                }
                let output: String = global.get("__ic_edge_output")?;
                RuntimeResponse::from_json(&output).map_err(|error| {
                    rquickjs::Error::new_from_js_message(
                        "Response",
                        "Response",
                        format!("{error:?}"),
                    )
                })
            })
            .map_err(to_runtime_error)
    }
}

impl EdgeRuntime for QuickJsRuntime {
    fn eval_module(&mut self, _name: &str, source: &str) -> Result<()> {
        self.context.with(|ctx| match ctx.eval::<(), _>(source) {
            Ok(()) => Ok(()),
            Err(error) if error.is_exception() => {
                let exception = Exception::from_js(&ctx, ctx.catch())
                    .map_err(|err| Error::Runtime(err.to_string()))?;
                Err(Error::Runtime(
                    exception
                        .message()
                        .unwrap_or_else(|| "JavaScript exception".to_string()),
                ))
            }
            Err(error) => Err(to_runtime_error(error)),
        })?;
        self.context
            .with(|ctx| {
                ctx.eval::<(), _>(
                    "if (globalThis.__ic_edge_bundle?.default) globalThis.__ic_edge_app = globalThis.__ic_edge_bundle.default",
                )
            })
            .map_err(to_runtime_error)
    }

    fn call_app_fetch(&mut self, request: Request) -> Result<Response> {
        self.context
            .with(|ctx| {
                let global = ctx.globals();
                let dispatch: Function = global.get("__ic_edge_dispatch")?;
                let headers_json = serde_json::to_string(&HeaderPairs::from_headers(
                    &request.headers,
                ))
                .map_err(|error| {
                    rquickjs::Error::new_from_js_message("Headers", "String", error.to_string())
                })?;
                let body_json = serde_json::to_string(request.body.bytes()).map_err(|error| {
                    rquickjs::Error::new_from_js_message("Body", "JSON", error.to_string())
                })?;
                dispatch.call::<_, ()>((request.method, request.url, headers_json, body_json))
            })
            .map_err(to_runtime_error)?;
        self.drain_jobs()?;
        self.take_output()
    }
}

fn host_fetch<F>(
    fetcher: &Rc<RefCell<F>>,
    method: String,
    url: String,
    headers_json: String,
    body_json: String,
) -> rquickjs::Result<String>
where
    F: HostFetch,
{
    let pairs: Vec<(String, String)> = serde_json::from_str(&headers_json).map_err(|error| {
        rquickjs::Error::new_from_js_message("Headers", "Headers", error.to_string())
    })?;
    let body = serde_json::from_str(&body_json)
        .map_err(|error| rquickjs::Error::new_from_js_message("Body", "Body", error.to_string()))?;
    let request = Request::new(
        method,
        url,
        Headers::from_pairs(pairs).map_err(to_js_error)?,
        Body::from_bytes(body),
    );
    let response = fetcher.borrow_mut().fetch(request).map_err(to_js_error)?;
    RuntimeResponse::from_response(response).map_err(to_js_error)
}

fn to_runtime_error(error: rquickjs::Error) -> Error {
    Error::Runtime(error.to_string())
}

fn to_js_error(error: Error) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("Rust", "JavaScript", format!("{error:?}"))
}

#[derive(Debug, Serialize)]
struct HeaderPairs(Vec<(String, String)>);

impl HeaderPairs {
    fn from_headers(headers: &Headers) -> Self {
        Self(
            headers
                .entries()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RuntimeResponse {
    status: Option<RuntimeStatus>,
    headers: Vec<(String, String)>,
    body: RuntimeBody,
}

impl RuntimeResponse {
    fn from_json(value: &str) -> Result<Response> {
        let decoded: Self =
            serde_json::from_str(value).map_err(|error| Error::Runtime(error.to_string()))?;
        let body = decoded.body.into_bytes();
        if body.len() > limits::MAX_JS_RESPONSE_BODY_BYTES {
            return Err(Error::Runtime(
                "JS response body exceeds v1 limit".to_string(),
            ));
        }
        Response::new(
            decoded.status.map_or(Ok(200), |status| status.to_u16())?,
            Headers::from_pairs(decoded.headers)?,
            Body::from_bytes(body),
        )
    }

    fn from_response(response: Response) -> Result<String> {
        let encoded = Self {
            status: Some(RuntimeStatus::Code(response.status)),
            headers: response
                .headers
                .entries()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: RuntimeBody::Bytes(response.body.bytes().to_vec()),
        };
        serde_json::to_string(&encoded).map_err(|error| Error::Runtime(error.to_string()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum RuntimeBody {
    Bytes(Vec<u8>),
    Text(String),
}

impl RuntimeBody {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Text(text) => text.into_bytes(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum RuntimeStatus {
    Code(u16),
    Text(String),
}

impl RuntimeStatus {
    fn to_u16(&self) -> Result<u16> {
        match self {
            Self::Code(status) => Ok(*status),
            Self::Text(status) => status
                .parse::<u16>()
                .map_err(|error| Error::Runtime(error.to_string())),
        }
    }
}
