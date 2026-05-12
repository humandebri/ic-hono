//! `crates/ic-edge-runtime` installs QuickJS wasm crypto callbacks.
//! The callbacks keep crypto primitives out of the JS polyfill body.

use hmac::{Hmac, Mac};
use ic_edge_web::{Error, Result};
use quickjs_wasm_rs::{JSContextRef, JSError, JSValue, JSValueRef};
use sha2::{Digest, Sha256};

pub fn install_callbacks(context: &JSContextRef) -> Result<()> {
    let global = context.global_object().map_err(to_runtime_error)?;
    global
        .set_property(
            "__ic_edge_crypto_digest",
            context
                .wrap_callback(|_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let data = bytes_arg(args, 1).map_err(to_js_error)?;
                    Ok(digest_json(&name, &data)
                        .map(JSValue::String)
                        .map_err(to_js_error)?)
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    global
        .set_property(
            "__ic_edge_crypto_sign",
            context
                .wrap_callback(|_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let key = bytes_arg(args, 1).map_err(to_js_error)?;
                    let data = bytes_arg(args, 2).map_err(to_js_error)?;
                    Ok(sign_json(&name, &key, &data)
                        .map(JSValue::String)
                        .map_err(to_js_error)?)
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    global
        .set_property(
            "__ic_edge_crypto_verify",
            context
                .wrap_callback(|_ctx, _this, args| {
                    let name = arg_string(args, 0).map_err(to_js_error)?;
                    let key = bytes_arg(args, 1).map_err(to_js_error)?;
                    let signature = bytes_arg(args, 2).map_err(to_js_error)?;
                    let data = bytes_arg(args, 3).map_err(to_js_error)?;
                    Ok(verify_hmac(&name, &key, &signature, &data)
                        .map(JSValue::Bool)
                        .map_err(to_js_error)?)
                })
                .map_err(to_runtime_error)?,
        )
        .map_err(to_runtime_error)?;
    Ok(())
}

pub fn install_random_seed(context: &JSContextRef, seed: Vec<u8>) -> Result<()> {
    let mut counter = 0u64;
    let callback = context
        .wrap_callback(move |_ctx, _this, args| {
            let length = args
                .first()
                .map(|value| value.to_string().parse::<usize>())
                .transpose()?
                .unwrap_or(0);
            let mut output = Vec::with_capacity(length);
            while output.len() < length {
                let mut hasher = Sha256::new();
                hasher.update(&seed);
                hasher.update(counter.to_le_bytes());
                output.extend_from_slice(&hasher.finalize());
                counter = counter.wrapping_add(1);
            }
            output.truncate(length);
            serde_json::to_string(&output)
                .map(JSValue::String)
                .map_err(|error| JSError::Internal(error.to_string()).into())
        })
        .map_err(to_runtime_error)?;
    context
        .global_object()
        .map_err(to_runtime_error)?
        .set_property("__ic_edge_crypto_random", callback)
        .map_err(to_runtime_error)
}

fn arg_string(args: &[JSValueRef], index: usize) -> Result<String> {
    args.get(index)
        .map(|value| value.to_string())
        .ok_or_else(|| Error::Runtime(format!("missing crypto argument {index}")))
}

fn bytes_arg(args: &[JSValueRef], index: usize) -> Result<Vec<u8>> {
    let value = arg_string(args, index)?;
    serde_json::from_str(&value).map_err(|error| Error::Runtime(error.to_string()))
}

fn digest_json(name: &str, data: &[u8]) -> Result<String> {
    if !name.eq_ignore_ascii_case("SHA-256") {
        return Err(Error::Runtime(format!("unsupported digest {name}")));
    }
    let digest = Sha256::digest(data).to_vec();
    serde_json::to_string(&digest).map_err(|error| Error::Runtime(error.to_string()))
}

fn sign_json(name: &str, key: &[u8], data: &[u8]) -> Result<String> {
    if !is_hmac_sha256(name) {
        return Err(Error::Runtime(format!("unsupported sign algorithm {name}")));
    }
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|error| Error::Runtime(error.to_string()))?;
    mac.update(data);
    let signature = mac.finalize().into_bytes().to_vec();
    serde_json::to_string(&signature).map_err(|error| Error::Runtime(error.to_string()))
}

fn verify_hmac(name: &str, key: &[u8], signature: &[u8], data: &[u8]) -> Result<bool> {
    if !is_hmac_sha256(name) {
        return Err(Error::Runtime(format!(
            "unsupported verify algorithm {name}"
        )));
    }
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|error| Error::Runtime(error.to_string()))?;
    mac.update(data);
    Ok(mac.verify_slice(signature).is_ok())
}

fn is_hmac_sha256(name: &str) -> bool {
    name.eq_ignore_ascii_case("HMAC") || name.eq_ignore_ascii_case("HMAC-SHA-256")
}

fn to_runtime_error(error: impl std::fmt::Display) -> Error {
    Error::Runtime(error.to_string())
}

fn to_js_error(error: Error) -> JSError {
    JSError::Internal(format!("{error:?}"))
}
