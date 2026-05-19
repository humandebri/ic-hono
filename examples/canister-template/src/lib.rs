//! `examples/canister-template` shows the intended IC HTTP endpoint shape.
//! Uploaded bundles are evaluated by the runtime when handling HTTP requests.

mod audit_support;
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
use ic_edge_canister::{
    https_outcall_fetch_with_replication, CdkHttpRequest, CdkHttpResponse, OutcallReplication,
};
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
use ic_edge_runtime::EdgeRuntime;
#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
use ic_edge_runtime::StaticRuntime;
use ic_edge_store::{EdgeStore, StableEdgeStore};
use ic_edge_web::{limits, Body, Headers, Request};
#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
use runtime_cache::{store_runtime, take_runtime};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::RefCell;

const GENERATION_KEY: &str = "__runtime_generation";
const MODULE_MANIFEST_PREFIX: &str = "__module_manifest:";
const UPLOAD_BYTES_PREFIX: &str = "__upload_bytes:";
const UPLOAD_MANIFEST_PREFIX: &str = "__upload_manifest:";
const UPLOAD_TOTAL_PREFIX: &str = "__upload_total:";

thread_local! {
    static STORE: RefCell<StableEdgeStore> = RefCell::new(StableEdgeStore::new());
}

#[derive(CandidType)]
struct RuntimeInfo {
    backend: String,
    generation: u64,
    bundle_sha256: Option<String>,
}

#[derive(Deserialize)]
struct UploadManifest {
    schema_version: u8,
    bundle_sha256: String,
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
    STORE.with_borrow_mut(|store| upload_bundle_in_store(store, &module, &bytes))
}

#[ic_cdk::update]
fn begin_bundle_upload(
    module: String,
    total_bytes: u64,
    manifest_json: String,
) -> Result<(), String> {
    ensure_controller()?;
    let total_bytes = usize::try_from(total_bytes)
        .map_err(|_| "bundle size does not fit this canister".to_string())?;
    STORE.with_borrow_mut(|store| {
        begin_bundle_upload_in_store(store, &module, total_bytes, &manifest_json)
    })
}

#[ic_cdk::update]
fn append_bundle_chunk(module: String, offset: u64, bytes: Vec<u8>) -> Result<(), String> {
    ensure_controller()?;
    let offset = usize::try_from(offset)
        .map_err(|_| "chunk offset does not fit this canister".to_string())?;
    STORE.with_borrow_mut(|store| append_bundle_chunk_in_store(store, &module, offset, &bytes))
}

#[ic_cdk::update]
fn commit_bundle_upload(module: String) -> Result<(), String> {
    ensure_controller()?;
    STORE.with_borrow_mut(|store| commit_bundle_upload_in_store(store, &module))
}

#[ic_cdk::update]
fn abort_bundle_upload(module: String) -> Result<(), String> {
    ensure_controller()?;
    STORE.with_borrow_mut(|store| abort_bundle_upload_in_store(store, &module))
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
    STORE.with_borrow(read_env_names)
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
        bundle_sha256: STORE.with_borrow(|store| read_bundle_sha256(store, "app")),
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
    fetch_outcall_with_replication(url, OutcallReplication::NonReplicated).await
}

#[ic_cdk::update]
async fn fetch_outcall_replicated(url: String) -> CdkHttpResponse {
    fetch_outcall_with_replication(url, OutcallReplication::Replicated).await
}

async fn fetch_outcall_with_replication(
    url: String,
    replication: OutcallReplication,
) -> CdkHttpResponse {
    if let Err(error) = ensure_controller() {
        return forbidden_response(&error);
    }
    let mut headers = Headers::new();
    if let Err(error) = headers.set("user-agent", "ic-edge-runtime".to_string()) {
        return error_response(error);
    }
    let request = Request::new("GET".to_string(), url, headers, Body::empty());
    https_outcall_fetch_with_replication(
        request,
        "transform_strip_headers",
        Some(limits::DEFAULT_FETCH_RESPONSE_BYTES),
        replication,
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
    let seed = raw_rand()
        .await
        .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
    let time_nanos = ic_cdk::api::time();
    let generation = STORE.with_borrow(read_generation);
    let source = STORE
        .with_borrow(|store| store.get_module("app"))
        .map_err(|error| ic_edge_web::Error::Runtime(format!("{error:?}")))?;
    let source = String::from_utf8(source)
        .map_err(|error| ic_edge_web::Error::Runtime(error.to_string()))?;
    let env = env_script()?;
    let mut runtime = take_runtime(generation, &env, &source)?;
    runtime.install_random_seed(seed)?;
    runtime.install_time_nanos(time_nanos)?;
    runtime.install_ic_context(
        &ic_cdk::api::msg_caller().to_string(),
        &ic_cdk::api::canister_self().to_string(),
    )?;
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

fn forbidden_response(message: &str) -> CdkHttpResponse {
    CdkHttpResponse {
        status_code: 403,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: message.as_bytes().to_vec(),
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

fn upload_bundle_in_store(
    store: &mut StableEdgeStore,
    module: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let _ = store;
    let _ = module;
    let _ = bytes;
    Err("manifest is required".to_string())
}

fn begin_bundle_upload_in_store(
    store: &mut StableEdgeStore,
    module: &str,
    total_bytes: usize,
    manifest_json: &str,
) -> Result<(), String> {
    if total_bytes > limits::MAX_BUNDLE_BYTES {
        return Err("bundle exceeds v1 limit".to_string());
    }
    validate_manifest_json(manifest_json)?;
    store
        .put_kv(&upload_bytes_key(module), &[])
        .map_err(|error| format!("{error:?}"))?;
    store
        .put_kv(&upload_manifest_key(module), manifest_json.as_bytes())
        .map_err(|error| format!("{error:?}"))?;
    store
        .put_kv(
            &upload_total_key(module),
            total_bytes.to_string().as_bytes(),
        )
        .map_err(|error| format!("{error:?}"))
}

fn append_bundle_chunk_in_store(
    store: &mut StableEdgeStore,
    module: &str,
    offset: usize,
    bytes: &[u8],
) -> Result<(), String> {
    if bytes.len() > limits::MAX_BUNDLE_UPLOAD_CHUNK_BYTES {
        return Err("bundle chunk exceeds v1 limit".to_string());
    }
    let total = read_upload_total(store, module)?;
    let mut staged = read_upload_bytes(store, module)?;
    if offset != staged.len() {
        return Err("bundle chunk offset mismatch".to_string());
    }
    if staged.len().saturating_add(bytes.len()) > total {
        return Err("bundle upload exceeds declared size".to_string());
    }
    staged.extend_from_slice(bytes);
    store
        .put_kv(&upload_bytes_key(module), &staged)
        .map_err(|error| format!("{error:?}"))
}

fn commit_bundle_upload_in_store(store: &mut StableEdgeStore, module: &str) -> Result<(), String> {
    let total = read_upload_total(store, module)?;
    let staged = read_upload_bytes(store, module)?;
    let manifest_json = read_upload_manifest(store, module)?;
    let manifest = validate_manifest_json(&manifest_json)?;
    if staged.len() != total {
        return Err("bundle upload is incomplete".to_string());
    }
    if sha256_hex(&staged) != manifest.bundle_sha256 {
        return Err("bundle sha256 does not match manifest".to_string());
    }
    store
        .put_module(module, &staged)
        .map_err(|error| format!("{error:?}"))?;
    store
        .put_kv(&module_manifest_key(module), manifest_json.as_bytes())
        .map_err(|error| format!("{error:?}"))?;
    abort_bundle_upload_in_store(store, module)?;
    let generation = bump_generation(store).map_err(|error| format!("{error:?}"))?;
    history_support::record_snapshot(store, generation)
}

fn abort_bundle_upload_in_store(store: &mut StableEdgeStore, module: &str) -> Result<(), String> {
    store
        .delete_kv(&upload_bytes_key(module))
        .map_err(|error| format!("{error:?}"))?;
    store
        .delete_kv(&upload_manifest_key(module))
        .map_err(|error| format!("{error:?}"))?;
    store
        .delete_kv(&upload_total_key(module))
        .map_err(|error| format!("{error:?}"))
}

fn read_upload_total(store: &StableEdgeStore, module: &str) -> Result<usize, String> {
    let bytes = store
        .get_kv(&upload_total_key(module))
        .map_err(|error| format!("{error:?}"))?
        .ok_or_else(|| "bundle upload has not started".to_string())?;
    let value = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn read_upload_bytes(store: &StableEdgeStore, module: &str) -> Result<Vec<u8>, String> {
    store
        .get_kv(&upload_bytes_key(module))
        .map_err(|error| format!("{error:?}"))?
        .ok_or_else(|| "bundle upload has not started".to_string())
}

fn read_upload_manifest(store: &StableEdgeStore, module: &str) -> Result<String, String> {
    let bytes = store
        .get_kv(&upload_manifest_key(module))
        .map_err(|error| format!("{error:?}"))?
        .ok_or_else(|| "bundle upload has not started".to_string())?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn validate_manifest_json(manifest_json: &str) -> Result<UploadManifest, String> {
    let manifest: UploadManifest =
        serde_json::from_str(manifest_json).map_err(|error| error.to_string())?;
    if manifest.schema_version != 1 {
        return Err("unsupported manifest schema_version".to_string());
    }
    if manifest.bundle_sha256.len() != 64
        || !manifest
            .bundle_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("manifest bundle_sha256 must be lowercase sha256 hex".to_string());
    }
    Ok(manifest)
}

fn read_bundle_sha256(store: &StableEdgeStore, module: &str) -> Option<String> {
    let bytes = store.get_kv(&module_manifest_key(module)).ok().flatten()?;
    let manifest: UploadManifest = serde_json::from_slice(&bytes).ok()?;
    Some(manifest.bundle_sha256)
}

pub(crate) fn read_module_manifest(store: &StableEdgeStore, module: &str) -> Vec<u8> {
    store
        .get_kv(&module_manifest_key(module))
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub(crate) fn put_module_manifest(
    store: &mut StableEdgeStore,
    module: &str,
    manifest: &[u8],
) -> Result<(), String> {
    if manifest.is_empty() {
        return store
            .delete_kv(&module_manifest_key(module))
            .map_err(|error| format!("{error:?}"));
    }
    store
        .put_kv(&module_manifest_key(module), manifest)
        .map_err(|error| format!("{error:?}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn module_manifest_key(module: &str) -> String {
    format!("{MODULE_MANIFEST_PREFIX}{}:{module}", module.len())
}

fn upload_bytes_key(module: &str) -> String {
    format!("{UPLOAD_BYTES_PREFIX}{}:{module}", module.len())
}

fn upload_manifest_key(module: &str) -> String {
    format!("{UPLOAD_MANIFEST_PREFIX}{}:{module}", module.len())
}

fn upload_total_key(module: &str) -> String {
    format!("{UPLOAD_TOTAL_PREFIX}{}:{module}", module.len())
}

#[cfg(all(target_arch = "wasm32", feature = "quickjs-ic"))]
fn runtime_backend() -> &'static str {
    "quickjs-ic"
}

#[cfg(not(all(target_arch = "wasm32", feature = "quickjs-ic")))]
fn runtime_backend() -> &'static str {
    "static"
}
