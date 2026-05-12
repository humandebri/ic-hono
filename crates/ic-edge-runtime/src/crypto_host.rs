//! `crates/ic-edge-runtime` provides host crypto callbacks for QuickJS.
//! It keeps cryptographic primitives out of the JavaScript polyfill source.

use hmac::{Hmac, Mac};
use rquickjs::{Context, Function};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub fn install(context: &Context) -> ic_edge_web::Result<()> {
    context
        .with(|ctx| {
            let global = ctx.globals();
            global.set(
                "__ic_edge_crypto_random",
                Function::new(ctx.clone(), crypto_random)?,
            )?;
            global.set(
                "__ic_edge_crypto_digest",
                Function::new(ctx.clone(), crypto_digest)?,
            )?;
            global.set(
                "__ic_edge_crypto_sign",
                Function::new(ctx.clone(), crypto_sign)?,
            )?;
            global.set(
                "__ic_edge_crypto_verify",
                Function::new(ctx.clone(), crypto_verify)?,
            )
        })
        .map_err(|error| ic_edge_web::Error::Runtime(error.to_string()))
}

fn crypto_random(length: usize) -> rquickjs::Result<String> {
    let mut bytes = vec![0_u8; length];
    getrandom::getrandom(&mut bytes).map_err(|error| {
        rquickjs::Error::new_from_js_message("Crypto", "Random", error.to_string())
    })?;
    bytes_to_json(&bytes)
}

fn crypto_digest(algorithm: String, data_json: String) -> rquickjs::Result<String> {
    if !algorithm.eq_ignore_ascii_case("SHA-256") {
        return Err(unsupported_crypto("digest", &algorithm));
    }
    let data = json_to_bytes(&data_json)?;
    bytes_to_json(&Sha256::digest(data).to_vec())
}

fn crypto_sign(algorithm: String, key_json: String, data_json: String) -> rquickjs::Result<String> {
    if !algorithm.eq_ignore_ascii_case("HMAC") {
        return Err(unsupported_crypto("sign", &algorithm));
    }
    let key = json_to_bytes(&key_json)?;
    let data = json_to_bytes(&data_json)?;
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|error| {
        rquickjs::Error::new_from_js_message("CryptoKey", "HMAC", error.to_string())
    })?;
    mac.update(&data);
    bytes_to_json(&mac.finalize().into_bytes())
}

fn crypto_verify(
    algorithm: String,
    key_json: String,
    signature_json: String,
    data_json: String,
) -> rquickjs::Result<bool> {
    let expected = crypto_sign(algorithm, key_json, data_json)?;
    Ok(expected == signature_json)
}

fn unsupported_crypto(operation: &'static str, algorithm: &str) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message(
        "Crypto",
        operation,
        format!("unsupported algorithm: {algorithm}"),
    )
}

fn json_to_bytes(value: &str) -> rquickjs::Result<Vec<u8>> {
    serde_json::from_str(value).map_err(|error| {
        rquickjs::Error::new_from_js_message("JSON", "Uint8Array", error.to_string())
    })
}

fn bytes_to_json(bytes: &[u8]) -> rquickjs::Result<String> {
    serde_json::to_string(bytes).map_err(|error| {
        rquickjs::Error::new_from_js_message("Uint8Array", "JSON", error.to_string())
    })
}
