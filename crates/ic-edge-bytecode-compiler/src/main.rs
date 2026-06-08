//! `crates/ic-edge-bytecode-compiler` runs under WASI to match canister QuickJS.
//! It compiles an esbuild IIFE bundle into QuickJS bytecode for upload.

#[cfg(target_arch = "wasm32")]
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

#[cfg(target_arch = "wasm32")]
fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: ic-edge-bytecode-compiler <bundle.js> <out.qjbc>".to_string());
    }
    let source = std::fs::read_to_string(&args[0]).map_err(|error| error.to_string())?;
    let bytecode = quickjs_wasm_rs::JSContextRef::default()
        .compile_global("app.js", &source)
        .map_err(|error| error.to_string())?;
    if let Some(parent) = std::path::Path::new(&args[1]).parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&args[1], bytecode).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("ic-edge-bytecode-compiler must be built for wasm32-wasip1 and run with wasmtime");
    std::process::exit(2);
}
