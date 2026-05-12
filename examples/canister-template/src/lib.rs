//! `examples/canister-template` shows the intended IC HTTP endpoint shape.
//! Uploaded bundles are evaluated by the runtime when handling HTTP requests.

mod cache_support;
mod env_support;
mod history_support;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
mod runtime_cache;
#[cfg(test)]
mod tests;

use candid::CandidType;
use env_support::{env_assignment, insert_env_name, read_env_names, valid_env_name};
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
use ic_cdk::management_canister::raw_rand;
use ic_cdk::management_canister::{HttpRequestResult, TransformArgs};
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
use ic_edge_canister::handle_cdk_http;
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
use ic_edge_canister::handle_cdk_http_async;
use ic_edge_canister::{https_outcall_fetch, CdkHttpRequest, CdkHttpResponse};
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
use ic_edge_runtime::EdgeRuntime;
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
use ic_edge_runtime::StaticRuntime;
use ic_edge_store::{EdgeStore, StableEdgeStore};
use ic_edge_web::{limits, Body, Headers, Request};
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
use runtime_cache::{store_runtime, take_runtime};
use std::cell::RefCell;

const GENERATION_KEY: &str = "__runtime_generation";

thread_local! {
    static STORE: RefCell<StableEdgeStore> = RefCell::new(StableEdgeStore::new());
}

#[derive(CandidType)]
struct RuntimeInfo {
    backend: String,
    generation: u64,
}

#[ic_cdk::query]
fn http_request(_request: CdkHttpRequest) -> CdkHttpResponse {
    CdkHttpResponse {
        status_code: 200,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: Vec::new(),
        upgrade: Some(true),
    }
}
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
#[ic_cdk::update]
fn http_request_update(request: CdkHttpRequest) -> CdkHttpResponse {
    handle_uploaded_bundle(request).unwrap_or_else(error_response)
}
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
#[ic_cdk::update]
async fn http_request_update(request: CdkHttpRequest) -> CdkHttpResponse {
    handle_uploaded_bundle_async(request)
        .await
        .unwrap_or_else(error_response)
}

#[ic_cdk::update]
fn upload_bundle(module: String, bytes: Vec<u8>) -> Result<(), String> {
    ensure_controller()?;
    if bytes.len() > limits::MAX_BUNDLE_BYTES {
        return Err("bundle exceeds v1 limit".to_string());
    }
    STORE.with_borrow_mut(|store| {
        store
            .put_module(&module, &bytes)
            .map_err(|error| format!("{error:?}"))?;
        let generation = bump_generation(store).map_err(|error| format!("{error:?}"))?;
        history_support::record_snapshot(store, generation)
    })
}

#[ic_cdk::update]
fn set_env(name: String, value: String) -> Result<(), String> {
    ensure_controller()?;
    if !valid_env_name(&name) {
        return Err("env name must use A-Z, 0-9, and _".to_string());
    }
    if value.len() > limits::MAX_ENV_VALUE_BYTES {
        return Err("env value exceeds v1 limit".to_string());
    }
    STORE.with_borrow_mut(|store| {
        let names = store
            .get_kv("__env_names")
            .ok()
            .flatten()
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .unwrap_or_default();
        let names = insert_env_name(&names, &name);
        if names.lines().filter(|item| !item.is_empty()).count() > limits::MAX_ENV_NAMES {
            return Err("env name count exceeds v1 limit".to_string());
        }
        store
            .put_kv(&format!("env:{name}"), value.as_bytes())
            .map_err(|error| format!("{error:?}"))?;
        store
            .put_kv("__env_names", names.as_bytes())
            .map_err(|error| format!("{error:?}"))?;
        let generation = bump_generation(store).map_err(|error| format!("{error:?}"))?;
        history_support::record_snapshot(store, generation)
    })
}
#[ic_cdk::query]
fn env_names() -> Vec<String> {
    STORE.with_borrow(|store| read_env_names(store))
}
#[ic_cdk::query]
fn bundle_size(module: String) -> Option<u64> {
    STORE
        .with_borrow(|store| store.get_module(&module).ok())
        .map(|bytes| bytes.len() as u64)
}
#[ic_cdk::query]
fn runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        backend: runtime_backend().to_string(),
        generation: STORE.with_borrow(read_generation),
    }
}
#[ic_cdk::query]
fn runtime_history() -> Vec<history_support::RuntimeSnapshotInfo> {
    STORE.with_borrow(history_support::runtime_history)
}
#[ic_cdk::update]
fn rollback_runtime(generation: u64) -> Result<(), String> {
    ensure_controller()?;
    STORE.with_borrow_mut(|store| history_support::rollback(store, generation))
}
#[ic_cdk::query(hidden = true)]
fn transform_strip_headers(args: TransformArgs) -> HttpRequestResult {
    ic_edge_canister::transform_strip_headers(args)
}
#[ic_cdk::update]
async fn fetch_outcall(url: String) -> CdkHttpResponse {
    let mut headers = Headers::new();
    if let Err(error) = headers.set("user-agent", "ic-edge-runtime".to_string()) {
        return error_response(error);
    }
    let request = Request::new("GET".to_string(), url, headers, Body::empty());
    https_outcall_fetch(
        request,
        "transform_strip_headers",
        Some(limits::DEFAULT_FETCH_RESPONSE_BYTES),
    )
    .await
    .map(|response| ic_edge_canister::IcHttpResponse {
        status_code: response.status,
        headers: response
            .headers
            .entries()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
        body: response.body.bytes().to_vec(),
    })
    .map(CdkHttpResponse::from)
    .unwrap_or_else(error_response)
}

#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
fn handle_uploaded_bundle(request: CdkHttpRequest) -> ic_edge_web::Result<CdkHttpResponse> {
    let source = STORE
        .with_borrow(|store| store.get_module("app"))
        .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
    let source = String::from_utf8(source)
        .map_err(|error| ic_edge_web::Error::Runtime(error.to_string()))?;
    let mut runtime = new_runtime()?;
    runtime.eval_module("env", &env_script()?)?;
    runtime.eval_module("app", &source)?;
    handle_cdk_http(&mut runtime, request)
}

#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
async fn handle_uploaded_bundle_async(
    request: CdkHttpRequest,
) -> ic_edge_web::Result<CdkHttpResponse> {
    let generation = STORE.with_borrow(read_generation);
    let source = STORE
        .with_borrow(|store| store.get_module("app"))
        .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
    let source = String::from_utf8(source)
        .map_err(|error| ic_edge_web::Error::Runtime(error.to_string()))?;
    let env = env_script()?;
    let seed = raw_rand()
        .await
        .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
    let mut runtime = take_runtime(generation, &env, &source)?;
    runtime.install_random_seed(seed)?;
    let response = handle_cdk_http_async(&mut runtime, request).await;
    store_runtime(generation, runtime);
    response
}

#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
fn new_runtime() -> ic_edge_web::Result<StaticRuntime> {
    Ok(StaticRuntime::new())
}

fn error_response(error: ic_edge_web::Error) -> CdkHttpResponse {
    CdkHttpResponse {
        status_code: 500,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: format!("{error:?}").into_bytes(),
        upgrade: None,
    }
}

#[cfg(target_arch = "wasm32")]
fn ensure_controller() -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    if ic_cdk::api::is_controller(&caller) {
        Ok(())
    } else {
        Err("caller is not a controller".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_controller() -> Result<(), String> {
    Ok(())
}

fn env_script() -> ic_edge_web::Result<String> {
    STORE.with_borrow(|store| {
        let mut script = "globalThis.process ||= {}; globalThis.process.env ||= {};".to_string();
        for name in read_env_names(store) {
            let key = format!("env:{name}");
            let value = store
                .get_kv(&key)
                .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
            let value =
                value.ok_or_else(|| ic_edge_web::Error::Runtime(format!("missing env {name}")))?;
            let value = String::from_utf8(value)
                .map_err(|error| ic_edge_web::Error::Runtime(error.to_string()))?;
            script.push_str(&env_assignment(&name, &value));
        }
        Ok(script)
    })
}

fn read_generation(store: &StableEdgeStore) -> u64 {
    store
        .get_kv(GENERATION_KEY)
        .ok()
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn bump_generation(store: &mut StableEdgeStore) -> ic_edge_store::Result<u64> {
    let next = read_generation(store).saturating_add(1);
    store.put_kv(GENERATION_KEY, next.to_string().as_bytes())?;
    Ok(next)
}

#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
fn runtime_backend() -> &'static str {
    "quickjs-ic"
}

#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
fn runtime_backend() -> &'static str {
    "static"
}
