//! `crates/ic-edge-pack` keeps CLI behavior tests out of the binary module.
//! Tests cover pack/init/upload contracts without invoking the real IC network.

use super::*;

#[test]
fn pack_runs_esbuild_and_checks_contract() {
    let root = env::temp_dir().join(format!("ic-edge-pack-test-{}", std::process::id()));
    let src = root.join("src");
    let bin = root.join("node_modules/.bin");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(src.join("app.ts"), "export default {}").unwrap();
    let esbuild = bin.join("esbuild");
    fs::write(
        &esbuild,
        "#!/usr/bin/env sh\nfor arg in \"$@\"; do case \"$arg\" in --outfile=*) out=\"${arg#--outfile=}\";; esac; done\nmkdir -p \"$(dirname \"$out\")\"\nprintf '%s\n' \"$@\" > \"$out.args\"\nprintf 'var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response(\"ok\") } }))();\\n//# sourceMappingURL=app.bundle.js.map\\n' > \"$out\"\nprintf '{\"version\":3,\"sources\":[\"app.ts\"],\"mappings\":\"\"}' > \"$out.map\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&esbuild, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let entrypoint = src.join("app.ts").to_string_lossy().to_string();
    let out_file = root
        .join("dist/app.bundle.js")
        .to_string_lossy()
        .to_string();
    let output = run(vec![
        "pack".to_string(),
        entrypoint,
        "--out".to_string(),
        out_file.clone(),
    ])
    .unwrap();
    let bytecode_file = root.join("dist/app.qjbc").to_string_lossy().to_string();
    assert_eq!(output, format!("packed {bytecode_file}"));
    let args = fs::read_to_string(format!("{out_file}.args")).unwrap();
    assert!(!args.lines().any(|arg| arg == "--minify"));
    assert!(args.lines().any(|arg| arg == "--sourcemap=external"));
    assert!(Path::new(&format!("{out_file}.map")).exists());
    assert!(Path::new(&bytecode_file).exists());
    let manifest_path = format!("{bytecode_file}.ic-edge-manifest.json");
    let manifest = fs::read_to_string(manifest_path).unwrap();
    assert!(manifest.contains("\"schema_version\": 2"));
    assert!(manifest.contains("\"format\": \"quickjs-bytecode\""));
    assert!(manifest.contains("\"bytecode_sha256\""));
    assert!(manifest.contains("\"source_bundle_sha256\""));
    assert!(manifest.contains("\"source_map_sha256\""));
}

#[test]
fn validates_runtime_bundle_contract_shape() {
    let root = env::temp_dir().join(format!("ic-edge-contract-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let valid = root.join("valid.js");
    let comment_only = root.join("comment-only.js");
    let missing_fetch = root.join("missing-fetch.js");
    fs::write(
        &valid,
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();",
    )
    .unwrap();
    fs::write(
        &comment_only,
        "var text = '__ic_edge_bundle default:'; // default: __ic_edge_bundle",
    )
    .unwrap();
    fs::write(
        &missing_fetch,
        "var __ic_edge_bundle = (() => ({ default: {} }))();",
    )
    .unwrap();
    assert!(validate_bundle_contract(&valid.to_string_lossy()).is_ok());
    assert!(validate_bundle_contract(&comment_only.to_string_lossy()).is_err());
    assert!(validate_bundle_contract(&missing_fetch.to_string_lossy()).is_err());
}

#[test]
fn init_hono_creates_template_without_overwriting() {
    let root = env::temp_dir().join(format!("ic-edge-init-test-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let output = run(vec![
        "init".to_string(),
        "hono".to_string(),
        root.to_string_lossy().to_string(),
    ])
    .unwrap();
    assert_eq!(
        output,
        format!("initialized hono app in {}", root.display())
    );
    assert!(root.join("package.json").exists());
    assert!(root.join("src/app.ts").exists());
    let app_source = fs::read_to_string(root.join("src/app.ts")).unwrap();
    assert!(app_source.contains("interface RequestInit"));
    assert!(app_source.contains("ic?: { replicated?: boolean }"));
    assert!(run(vec![
        "init".to_string(),
        "hono".to_string(),
        root.to_string_lossy().to_string(),
    ])
    .is_err());
}

#[test]
fn rejects_unknown_command() {
    assert!(run(vec!["upload".to_string()]).is_err());
}

#[test]
fn rejects_unknown_upload_argument() {
    assert!(run(vec![
        "upload".to_string(),
        "bundle.js".to_string(),
        "--bad".to_string()
    ])
    .is_err());
}

#[test]
fn creates_candid_upload_argument() {
    assert_eq!(
        candid_upload_argument("app", &[0, 15, 255]),
        "(\"app\", blob \"\\00\\0f\\ff\")"
    );
    assert_eq!(
        candid_begin_upload_argument("app", 3, "{\"schema_version\":1}").unwrap(),
        "(\"app\", 3 : nat64, \"{\\\"schema_version\\\":1}\")"
    );
    assert_eq!(
        candid_append_chunk_argument("app", 2, &[255]).unwrap(),
        "(\"app\", 2 : nat64, blob \"\\ff\")"
    );
    assert_eq!(candid_module_argument("app"), "(\"app\")");
}

#[test]
fn upload_requires_manifest() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-manifest-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.qjbc");
    fs::write(
        &bundle,
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();",
    )
    .unwrap();
    let error = run(vec![
        "upload".to_string(),
        bundle.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert!(error.contains("manifest is required"));
}

#[test]
fn upload_rejects_js_source_artifact() {
    let root = env::temp_dir().join(format!("ic-edge-upload-js-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("bundle.js");
    fs::write(&source, "var __ic_edge_bundle = {};").unwrap();

    let error = run(vec![
        "upload".to_string(),
        source.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert_eq!(error, "upload expects a .qjbc bytecode artifact");
}

#[test]
fn upload_rejects_manifest_hash_mismatch() {
    let root = env::temp_dir().join(format!("ic-edge-upload-hash-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.js");
    let bytecode = root.join("bundle.qjbc");
    let source_map = root.join("bundle.js.map");
    fs::write(
        &bundle,
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();",
    )
    .unwrap();
    fs::write(&bytecode, b"bytecode").unwrap();
    fs::write(
        &source_map,
        "{\"version\":3,\"sources\":[],\"mappings\":\"\"}",
    )
    .unwrap();
    write_test_manifest(&bytecode, &bundle, &source_map, "0".repeat(64));

    let error = run(vec![
        "upload".to_string(),
        bytecode.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert_eq!(error, "bytecode sha256 does not match manifest");
}

#[test]
fn upload_rejects_missing_source_map() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-sourcemap-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.js");
    let bytecode = root.join("bundle.qjbc");
    let source_map = root.join("bundle.js.map");
    let source =
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();";
    fs::write(&bundle, source).unwrap();
    fs::write(&bytecode, source).unwrap();
    write_test_manifest(
        &bytecode,
        &bundle,
        &source_map,
        ic_edge_pack::sha256_hex(source.as_bytes()),
    );

    let error = run(vec![
        "upload".to_string(),
        bytecode.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert_eq!(error, "source map is required");
}

#[test]
fn upload_rejects_source_map_hash_mismatch() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-sourcemap-hash-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.js");
    let bytecode = root.join("bundle.qjbc");
    let source_map = root.join("bundle.js.map");
    let source =
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();";
    fs::write(&bundle, source).unwrap();
    fs::write(&bytecode, source).unwrap();
    fs::write(
        &source_map,
        "{\"version\":3,\"sources\":[],\"mappings\":\"\"}",
    )
    .unwrap();
    write_test_manifest(
        &bytecode,
        &bundle,
        &source_map,
        ic_edge_pack::sha256_hex(source.as_bytes()),
    );

    let error = run(vec![
        "upload".to_string(),
        bytecode.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert_eq!(error, "source map sha256 does not match manifest");
}

#[test]
fn upload_accepts_manifest_hashes_for_local_store() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-valid-manifest-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.js");
    let bytecode = root.join("bundle.qjbc");
    let source_map = root.join("bundle.js.map");
    let source =
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();";
    let map = "{\"version\":3,\"sources\":[],\"mappings\":\"\"}";
    fs::write(&bundle, source).unwrap();
    fs::write(&bytecode, source).unwrap();
    fs::write(&source_map, map).unwrap();
    write_test_manifest_with_hashes(
        &bytecode,
        &bundle,
        &source_map,
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(map.as_bytes()),
    );

    let output = run(vec![
        "upload".to_string(),
        bytecode.to_string_lossy().to_string(),
    ])
    .unwrap();
    assert!(output.starts_with("uploaded "));
    assert!(output.ends_with(" bytecode bytes to module app"));
}

#[test]
fn upload_accepts_equivalent_bytecode_path_spelling() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-equivalent-path-test-{}",
        std::process::id()
    ));
    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let bundle = dist.join("bundle.js");
    let bytecode = dist.join("bundle.qjbc");
    let source_map = dist.join("bundle.js.map");
    let source =
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();";
    let map = "{\"version\":3,\"sources\":[],\"mappings\":\"\"}";
    fs::write(&bundle, source).unwrap();
    fs::write(&bytecode, source).unwrap();
    fs::write(&source_map, map).unwrap();
    write_test_manifest_with_hashes(
        &bytecode,
        &bundle,
        &source_map,
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(map.as_bytes()),
    );

    let equivalent = dist.join("../dist/bundle.qjbc");
    let output = run(vec![
        "upload".to_string(),
        equivalent.to_string_lossy().to_string(),
    ])
    .unwrap();
    assert!(output.starts_with("uploaded "));
}

#[test]
fn upload_rejects_manifest_for_different_bytecode_path() {
    let root = env::temp_dir().join(format!(
        "ic-edge-upload-different-path-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle = root.join("bundle.js");
    let bytecode = root.join("bundle.qjbc");
    let other_bytecode = root.join("other.qjbc");
    let source_map = root.join("bundle.js.map");
    let source =
        "var __ic_edge_bundle = (() => ({ default: { fetch: () => new Response('ok') } }))();";
    let map = "{\"version\":3,\"sources\":[],\"mappings\":\"\"}";
    fs::write(&bundle, source).unwrap();
    fs::write(&bytecode, source).unwrap();
    fs::write(&other_bytecode, source).unwrap();
    fs::write(&source_map, map).unwrap();
    write_test_manifest_with_bundle_path(
        &bytecode,
        &other_bytecode,
        &bundle,
        &source_map,
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(source.as_bytes()),
        ic_edge_pack::sha256_hex(map.as_bytes()),
    );

    let error = run(vec![
        "upload".to_string(),
        bytecode.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    assert_eq!(error, "manifest bytecode_path does not match upload path");
}

fn write_test_manifest(bytecode: &Path, bundle: &Path, source_map: &Path, bytecode_sha256: String) {
    write_test_manifest_with_hashes(
        bytecode,
        bundle,
        source_map,
        bytecode_sha256,
        ic_edge_pack::sha256_hex(fs::read(bundle).unwrap().as_slice()),
        "0".repeat(64),
    );
}

fn write_test_manifest_with_hashes(
    bytecode: &Path,
    bundle: &Path,
    source_map: &Path,
    bytecode_sha256: String,
    source_bundle_sha256: String,
    source_map_sha256: String,
) {
    write_test_manifest_with_bundle_path(
        bytecode,
        bytecode,
        bundle,
        source_map,
        bytecode_sha256,
        source_bundle_sha256,
        source_map_sha256,
    );
}

fn write_test_manifest_with_bundle_path(
    manifest_file_for_bytecode: &Path,
    manifest_bytecode_path: &Path,
    manifest_bundle_path: &Path,
    source_map: &Path,
    bytecode_sha256: String,
    source_bundle_sha256: String,
    source_map_sha256: String,
) {
    let manifest = ic_edge_pack::BundleArtifactManifest {
        schema_version: 2,
        format: "quickjs-bytecode".to_string(),
        global_name: "__ic_edge_bundle".to_string(),
        entrypoint: "src/app.ts".to_string(),
        bundle_path: manifest_bundle_path.to_string_lossy().to_string(),
        source_bundle_sha256,
        bytecode_path: manifest_bytecode_path.to_string_lossy().to_string(),
        bytecode_sha256,
        source_map_path: source_map.to_string_lossy().to_string(),
        source_map_sha256,
        quickjs_wasm_rs_version: "3.1.0".to_string(),
        esbuild_args: Vec::new(),
    };
    let path = ic_edge_pack::artifact_manifest_path(manifest_file_for_bytecode);
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    fs::write(path, format!("{json}\n")).unwrap();
}

#[test]
fn abort_after_upload_error_preserves_original_error() {
    let error = abort_after_upload_error(
        "missing-canister",
        Some("local"),
        "app",
        "commit failed".to_string(),
    );
    assert_eq!(error, "commit failed");
}

#[test]
fn parses_canister_call_result_variants() {
    assert!(parse_canister_call_result(b"(variant { Ok })\n", b"").is_ok());
    assert_eq!(
        parse_canister_call_result(
            b"(variant { Err = \"bundle chunk offset mismatch\" })\n",
            b"",
        )
        .unwrap_err(),
        "bundle chunk offset mismatch"
    );
    assert_eq!(
        parse_canister_call_result(b"", b"ERR failed").unwrap_err(),
        "ERR failed"
    );
    assert_eq!(
        parse_canister_call_result(b"unexpected", b"").unwrap_err(),
        "unexpected"
    );
}

#[test]
fn preopen_dir_uses_current_dir_for_bare_output_file() {
    assert_eq!(
        preopen_dir_for_path(Path::new("app.bundle.js")),
        Path::new(".")
    );
}

#[test]
fn preopen_dir_uses_parent_for_relative_and_absolute_paths() {
    assert_eq!(
        preopen_dir_for_path(Path::new("dist/app.bundle.js")),
        Path::new("dist")
    );
    let temp = env::temp_dir();
    let absolute = temp.join("app.bundle.js");
    assert_eq!(preopen_dir_for_path(&absolute), temp.as_path());
}

#[test]
fn bytecode_compiler_wasm_prefers_explicit_override() {
    let path =
        bytecode_compiler_wasm_with_override(Some("/tmp/custom-helper.wasm".to_string())).unwrap();
    assert_eq!(path, PathBuf::from("/tmp/custom-helper.wasm"));
}

#[test]
fn bytecode_compiler_wasm_materializes_bundled_asset() {
    let path = bytecode_compiler_wasm_with_override(None).unwrap();
    assert_eq!(fs::read(path).unwrap(), BYTECODE_COMPILER_WASM);
}

#[test]
fn parses_canister_upload_target() {
    let args = vec![
        "bundle.js".to_string(),
        "--module".to_string(),
        "app".to_string(),
        "--canister".to_string(),
        "edge".to_string(),
        "--environment".to_string(),
        "local".to_string(),
    ];
    assert_eq!(parse_module(&args).unwrap(), Some("app".to_string()));
    assert_eq!(parse_canister(&args).unwrap(), Some("edge".to_string()));
    assert_eq!(parse_environment(&args).unwrap(), Some("local".to_string()));
}
