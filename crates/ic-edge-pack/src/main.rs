//! `crates/ic-edge-pack` provides the first CLI surface.
//! It runs a local bundler and checks the runtime bundle contract.

use ic_edge_pack::{
    artifact_manifest, bytecode_path_for_bundle, default_out_file, esbuild_args,
    manifest_for_request, upload_bytecode, verified_artifact_manifest, write_artifact_manifest,
    PackRequest,
};
use ic_edge_runtime::validate_bundle_contract as validate_runtime_bundle_contract;
use ic_edge_store::{EdgeStore, MemoryEdgeStore};
use ic_edge_web::limits;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BYTECODE_COMPILER_WASM: &[u8] = include_bytes!("../assets/ic-edge-bytecode-compiler.wasm");

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("init") => init_command(&args[1..]),
        Some("pack") => pack_command(&args[1..]),
        Some("upload") => upload_command(&args[1..]),
        _ => Err(
            "usage: ic-edge init hono [directory]\n       ic-edge pack <entrypoint> [--out <bundle.js>]\n       ic-edge upload <app.qjbc> [--module <name>] [--canister <name>] [--environment <name>]"
                .to_string(),
        ),
    }
}

fn init_command(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("hono") => init_hono(args.get(1).map_or_else(|| Path::new("."), Path::new)),
        Some(value) => Err(format!("unknown template: {value}")),
        None => Err("missing template".to_string()),
    }
}

fn init_hono(directory: &Path) -> Result<String, String> {
    fs::create_dir_all(directory.join("src")).map_err(|error| error.to_string())?;
    write_new_file(
        &directory.join("package.json"),
        r#"{
  "name": "ic-edge-hono-app",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "ic-edge pack src/app.ts --out dist/app.bundle.js"
  },
  "dependencies": {
    "hono": "^4.7.0"
  },
  "devDependencies": {
    "esbuild": "^0.25.0",
    "typescript": "^5.8.0"
  }
}
"#,
    )?;
    write_new_file(
        &directory.join("src/app.ts"),
        r#"import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { TrieRouter } from 'hono/router/trie-router'

declare global {
  interface RequestInit {
    ic?: { replicated?: boolean }
  }

  interface Request {
    ic?: { replicated: boolean }
  }
}

const app = new Hono({ router: new TrieRouter() })

app.use('*', cors())

app.get('/', (c) => c.text('ok'))

app.post('/echo', async (c) => {
  return c.json(await c.req.json())
})

app.get('/users/:id', (c) => {
  return c.json({
    id: c.req.param('id'),
    q: c.req.query('q'),
  })
})

export default app
"#,
    )?;
    write_new_file(
        &directory.join("README.md"),
        r#"# ic-edge Hono app

```bash
npm install
npm run build
ic-edge upload dist/app.qjbc --canister edge --environment local
```
"#,
    )?;
    Ok(format!("initialized hono app in {}", directory.display()))
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Err(format!("refusing to overwrite {}", path.display()));
    }
    fs::write(path, contents).map_err(|error| error.to_string())
}

fn pack_command(args: &[String]) -> Result<String, String> {
    let entrypoint = args
        .first()
        .ok_or_else(|| "missing entrypoint".to_string())?
        .to_string();
    let out_file =
        parse_out_file(args)?.unwrap_or_else(|| default_out_file(&PathBuf::from(&entrypoint)));
    let manifest = manifest_for_request(PackRequest {
        entrypoint,
        out_file: out_file.to_string_lossy().to_string(),
    });
    let esbuild_args = run_esbuild(&manifest.entrypoint, &manifest.bundle_path)?;
    validate_bundle_contract(&manifest.bundle_path)?;
    let bytecode_path = bytecode_path_for_bundle(Path::new(&manifest.bundle_path));
    compile_bundle_bytecode(Path::new(&manifest.bundle_path), &bytecode_path)?;
    let artifact = artifact_manifest(
        &manifest.entrypoint,
        Path::new(&manifest.bundle_path),
        &bytecode_path,
        esbuild_args,
    )?;
    write_artifact_manifest(&artifact)?;
    Ok(format!("packed {}", bytecode_path.display()))
}

fn upload_command(args: &[String]) -> Result<String, String> {
    let bytecode_path = args
        .first()
        .ok_or_else(|| "missing bytecode path".to_string())?;
    if Path::new(bytecode_path)
        .extension()
        .and_then(|value| value.to_str())
        != Some("qjbc")
    {
        return Err("upload expects a .qjbc bytecode artifact".to_string());
    }
    let module = parse_module(args)?.unwrap_or_else(|| "app".to_string());
    let manifest = verified_artifact_manifest(Path::new(bytecode_path))?;
    let bytes = fs::read(bytecode_path).map_err(|error| error.to_string())?;
    if bytes.len() > limits::MAX_BUNDLE_BYTES {
        return Err("bytecode exceeds v1 limit".to_string());
    }
    if let Some(canister) = parse_canister(args)? {
        upload_to_canister(
            &canister,
            parse_environment(args)?,
            &module,
            &bytes,
            &manifest,
        )?;
        return Ok(format!(
            "uploaded {} bytecode bytes to canister {canister} module {module}",
            bytes.len()
        ));
    }
    let mut store = MemoryEdgeStore::new();
    upload_bytecode(&mut store, &module, &bytes).map_err(|error| format!("{error:?}"))?;
    let stored_len = store
        .get_module(&module)
        .map_err(|error| format!("{error:?}"))?
        .len();
    Ok(format!(
        "uploaded {stored_len} bytecode bytes to module {module}"
    ))
}

fn upload_to_canister(
    canister: &str,
    environment: Option<String>,
    module: &str,
    bytes: &[u8],
    manifest: &ic_edge_pack::BundleArtifactManifest,
) -> Result<(), String> {
    let manifest_json = serde_json::to_string(manifest).map_err(|error| error.to_string())?;
    call_canister(
        canister,
        environment.as_deref(),
        "begin_bytecode_upload",
        &candid_begin_upload_argument(module, bytes.len(), &manifest_json)?,
    )?;
    let mut offset = 0usize;
    while offset < bytes.len() {
        let end = offset
            .saturating_add(limits::MAX_BUNDLE_UPLOAD_CHUNK_BYTES)
            .min(bytes.len());
        call_canister(
            canister,
            environment.as_deref(),
            "append_bytecode_chunk",
            &candid_append_chunk_argument(module, offset, &bytes[offset..end])?,
        )
        .map_err(|error| {
            let _ = call_canister(
                canister,
                environment.as_deref(),
                "abort_bytecode_upload",
                &candid_module_argument(module),
            );
            error
        })?;
        offset = end;
    }
    call_canister(
        canister,
        environment.as_deref(),
        "commit_bytecode_upload",
        &candid_module_argument(module),
    )
    .map_err(|error| abort_after_upload_error(canister, environment.as_deref(), module, error))
}

fn abort_after_upload_error(
    canister: &str,
    environment: Option<&str>,
    module: &str,
    error: String,
) -> String {
    let _ = call_canister(
        canister,
        environment,
        "abort_bytecode_upload",
        &candid_module_argument(module),
    );
    error
}

fn call_canister(
    canister: &str,
    environment: Option<&str>,
    method: &str,
    argument: &str,
) -> Result<(), String> {
    let argument_path = env::temp_dir().join(format!("ic-edge-{method}.did"));
    fs::write(&argument_path, argument).map_err(|error| error.to_string())?;
    let mut command = Command::new("icp");
    command.args(["canister", "call", canister, method, "--args-file"]);
    command.arg(&argument_path);
    if let Some(environment) = environment {
        command.args(["--environment", environment]);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        parse_canister_call_result(&output.stdout, &output.stderr)
    } else {
        Err(command_output_error(&output.stdout, &output.stderr))
    }
}

fn parse_canister_call_result(stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(stdout);
    if stdout.contains("variant { Err") {
        return Err(extract_candid_err(&stdout).unwrap_or_else(|| stdout.trim().to_string()));
    }
    if stdout.contains("variant { Ok") {
        return Ok(());
    }
    Err(command_output_error(stdout.as_bytes(), stderr))
}

fn extract_candid_err(stdout: &str) -> Option<String> {
    let start = stdout.find("Err")?;
    let after_err = &stdout[start..];
    let quote_start = after_err.find('"')?;
    let quoted = &after_err[quote_start + 1..];
    let quote_end = quoted.find('"')?;
    Some(quoted[..quote_end].to_string())
}

fn command_output_error(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    "empty icp canister call response".to_string()
}

fn candid_begin_upload_argument(
    module: &str,
    total_bytes: usize,
    manifest_json: &str,
) -> Result<String, String> {
    let total =
        u64::try_from(total_bytes).map_err(|_| "bytecode length does not fit nat64".to_string())?;
    Ok(format!("({module:?}, {total} : nat64, {manifest_json:?})"))
}

fn candid_append_chunk_argument(
    module: &str,
    offset: usize,
    bytes: &[u8],
) -> Result<String, String> {
    let offset = u64::try_from(offset).map_err(|_| "offset does not fit nat64".to_string())?;
    Ok(format!(
        "({module:?}, {offset} : nat64, {})",
        candid_blob(bytes)
    ))
}

fn candid_module_argument(module: &str) -> String {
    format!("({module:?})")
}

#[cfg(test)]
fn candid_upload_argument(module: &str, bytes: &[u8]) -> String {
    format!("({module:?}, {})", candid_blob(bytes))
}

fn candid_blob(bytes: &[u8]) -> String {
    let escaped = bytes
        .iter()
        .map(|byte| format!("\\{byte:02x}"))
        .collect::<String>();
    format!("blob \"{escaped}\"")
}

fn run_esbuild(entrypoint: &str, bundle_path: &str) -> Result<Vec<String>, String> {
    let esbuild = find_esbuild(Path::new(entrypoint));
    let args = esbuild_args(entrypoint, bundle_path);
    let output = Command::new(esbuild)
        .args(&args)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(args)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn find_esbuild(entrypoint: &Path) -> PathBuf {
    let mut cursor = if entrypoint.is_dir() {
        entrypoint.to_path_buf()
    } else {
        entrypoint
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    loop {
        let candidate = cursor.join("node_modules/.bin/esbuild");
        if candidate.exists() {
            return candidate;
        }
        if !cursor.pop() {
            return PathBuf::from("esbuild");
        }
    }
}

fn validate_bundle_contract(bundle_path: &str) -> Result<(), String> {
    let source = fs::read_to_string(bundle_path).map_err(|error| error.to_string())?;
    validate_runtime_bundle_contract(&source).map_err(|error| format!("{error:?}"))
}

#[cfg(not(test))]
fn compile_bundle_bytecode(bundle_path: &Path, bytecode_path: &Path) -> Result<(), String> {
    let compiler_wasm = bytecode_compiler_wasm()?;
    let wasmtime = env::var("IC_EDGE_WASMTIME").unwrap_or_else(|_| "wasmtime".to_string());
    let mut command = Command::new(wasmtime);
    preopen_parent_dir(&mut command, bundle_path);
    preopen_parent_dir(&mut command, bytecode_path);
    let output = command
        .arg(&compiler_wasm)
        .arg(bundle_path)
        .arg(bytecode_path)
        .output()
        .map_err(|error| format!("failed to run wasmtime; install wasmtime CLI: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_error(&output.stdout, &output.stderr))
    }
}

#[cfg(test)]
fn compile_bundle_bytecode(bundle_path: &Path, bytecode_path: &Path) -> Result<(), String> {
    let bytes = fs::read(bundle_path).map_err(|error| error.to_string())?;
    fs::write(bytecode_path, bytes).map_err(|error| error.to_string())
}

#[cfg(not(test))]
fn preopen_parent_dir(command: &mut Command, path: &Path) {
    command.arg("--dir").arg(preopen_dir_for_path(path));
}

#[cfg(not(test))]
fn bytecode_compiler_wasm() -> Result<PathBuf, String> {
    bytecode_compiler_wasm_with_override(env::var("IC_EDGE_BYTECODE_COMPILER_WASM").ok())
}

fn bytecode_compiler_wasm_with_override(override_path: Option<String>) -> Result<PathBuf, String> {
    if let Some(path) = override_path.filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    materialize_bundled_bytecode_compiler_wasm()
}

fn materialize_bundled_bytecode_compiler_wasm() -> Result<PathBuf, String> {
    let path = env::temp_dir().join(format!(
        "ic-edge-bytecode-compiler-{}.wasm",
        env!("CARGO_PKG_VERSION")
    ));
    let needs_write = fs::read(&path)
        .map(|bytes| bytes != BYTECODE_COMPILER_WASM)
        .unwrap_or(true);
    if needs_write {
        fs::write(&path, BYTECODE_COMPILER_WASM).map_err(|error| error.to_string())?;
    }
    Ok(path)
}

fn preopen_dir_for_path(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn parse_out_file(args: &[String]) -> Result<Option<PathBuf>, String> {
    let Some(arg) = args.get(1) else {
        return Ok(None);
    };
    match arg.as_str() {
        "--out" => {
            let value = args
                .get(2)
                .ok_or_else(|| "missing --out value".to_string())?;
            Ok(Some(PathBuf::from(value)))
        }
        value => Err(format!("unknown argument: {value}")),
    }
}

fn parse_module(args: &[String]) -> Result<Option<String>, String> {
    parse_string_arg(args, "--module")
}

fn parse_canister(args: &[String]) -> Result<Option<String>, String> {
    parse_string_arg(args, "--canister")
}

fn parse_environment(args: &[String]) -> Result<Option<String>, String> {
    parse_string_arg(args, "--environment")
}

fn parse_string_arg(args: &[String], name: &str) -> Result<Option<String>, String> {
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            value if value == name => {
                let arg = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing {name} value"))?;
                return Ok(Some(arg.to_string()));
            }
            "--module" | "--canister" | "--environment" => index += 2,
            value => return Err(format!("unknown argument: {value}")),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
