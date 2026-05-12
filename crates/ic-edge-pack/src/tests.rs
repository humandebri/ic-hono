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
        "#!/usr/bin/env sh\nfor arg in \"$@\"; do case \"$arg\" in --outfile=*) out=\"${arg#--outfile=}\";; esac; done\nmkdir -p \"$(dirname \"$out\")\"\nprintf 'var __ic_edge_bundle = (() => ({ default: {} }))();' > \"$out\"\n",
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
    assert_eq!(output, format!("packed {out_file}"));
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
