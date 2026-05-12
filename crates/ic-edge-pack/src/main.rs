//! `crates/ic-edge-pack` provides the first CLI surface.
//! It runs a local bundler and checks the runtime bundle contract.

use ic_edge_pack::{default_out_file, manifest_for_request, upload_bundle, PackRequest};
use ic_edge_store::{EdgeStore, MemoryEdgeStore};
use ic_edge_web::limits;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
            "usage: ic-edge init hono [directory]\n       ic-edge pack <entrypoint> [--out <file>]\n       ic-edge upload <bundle.js> [--module <name>] [--canister <name>] [--environment <name>]"
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
ic-edge upload dist/app.bundle.js --canister edge --environment local
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
    run_esbuild(&manifest.entrypoint, &manifest.bundle_path)?;
    validate_bundle_contract(&manifest.bundle_path)?;
    Ok(format!("packed {}", manifest.bundle_path))
}

fn upload_command(args: &[String]) -> Result<String, String> {
    let bundle_path = args
        .first()
        .ok_or_else(|| "missing bundle path".to_string())?;
    let module = parse_module(args)?.unwrap_or_else(|| "app".to_string());
    let bytes = fs::read(bundle_path).map_err(|error| error.to_string())?;
    if bytes.len() > limits::MAX_BUNDLE_BYTES {
        return Err("bundle exceeds v1 limit".to_string());
    }
    if let Some(canister) = parse_canister(args)? {
        upload_to_canister(&canister, parse_environment(args)?, &module, &bytes)?;
        return Ok(format!(
            "uploaded {} bytes to canister {canister} module {module}",
            bytes.len()
        ));
    }
    let mut store = MemoryEdgeStore::new();
    upload_bundle(&mut store, &module, &bytes).map_err(|error| format!("{error:?}"))?;
    let stored_len = store
        .get_module(&module)
        .map_err(|error| format!("{error:?}"))?
        .len();
    Ok(format!("uploaded {stored_len} bytes to module {module}"))
}

fn upload_to_canister(
    canister: &str,
    environment: Option<String>,
    module: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let argument_path = env::temp_dir().join("ic-edge-upload.did");
    fs::write(&argument_path, candid_upload_argument(module, bytes))
        .map_err(|error| error.to_string())?;
    let mut command = Command::new("icp");
    command.args(["canister", "call", canister, "upload_bundle", "--args-file"]);
    command.arg(&argument_path);
    if let Some(environment) = environment {
        command.args(["--environment", &environment]);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn candid_upload_argument(module: &str, bytes: &[u8]) -> String {
    let escaped = bytes
        .iter()
        .map(|byte| format!("\\{byte:02x}"))
        .collect::<String>();
    format!("({module:?}, blob \"{escaped}\")")
}

fn run_esbuild(entrypoint: &str, bundle_path: &str) -> Result<(), String> {
    let esbuild = find_esbuild(Path::new(entrypoint));
    let output = Command::new(esbuild)
        .args([
            entrypoint,
            "--bundle",
            "--format=iife",
            "--global-name=__ic_edge_bundle",
            "--platform=neutral",
            "--conditions=browser,worker,import",
            "--target=es2018",
            &format!("--outfile={bundle_path}"),
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
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
    if !source.contains("__ic_edge_bundle") {
        return Err("bundle contract missing __ic_edge_bundle global".to_string());
    }
    if !source.contains("default:") {
        return Err("bundle contract missing default export".to_string());
    }
    Ok(())
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
