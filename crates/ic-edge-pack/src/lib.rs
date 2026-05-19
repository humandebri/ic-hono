//! `crates/ic-edge-pack` defines packer inputs and outputs.
//! The executable CLI can wrap this API after the bundle format settles.

use ic_edge_loader::BundleManifest;
use ic_edge_store::{EdgeStore, Result as StoreResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Request describing one local bundle operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRequest {
    /// Source entrypoint file.
    pub entrypoint: String,
    /// Bundle output file.
    pub out_file: String,
}

/// Converts a pack request into the runtime bundle manifest.
pub fn manifest_for_request(request: PackRequest) -> BundleManifest {
    BundleManifest::single_bundle(request.entrypoint, request.out_file)
}

/// Manifest written next to the bundle artifact.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct BundleArtifactManifest {
    pub schema_version: u8,
    pub format: String,
    pub global_name: String,
    pub entrypoint: String,
    pub bundle_path: String,
    pub bundle_sha256: String,
    pub source_map_path: String,
    pub source_map_sha256: String,
    pub esbuild_args: Vec<String>,
}

/// Returns `<bundle>.ic-edge-manifest.json`.
pub fn artifact_manifest_path(bundle_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.ic-edge-manifest.json", bundle_path.display()))
}

/// Returns esbuild arguments for the v1 IIFE artifact.
pub fn esbuild_args(entrypoint: &str, bundle_path: &str) -> Vec<String> {
    vec![
        entrypoint.to_string(),
        "--bundle".to_string(),
        "--format=iife".to_string(),
        "--global-name=__ic_edge_bundle".to_string(),
        "--platform=neutral".to_string(),
        "--conditions=browser,worker,import".to_string(),
        "--target=es2018".to_string(),
        "--sourcemap=external".to_string(),
        format!("--outfile={bundle_path}"),
    ]
}

/// Builds a manifest from generated bundle and source map files.
pub fn artifact_manifest(
    entrypoint: &str,
    bundle_path: &Path,
    esbuild_args: Vec<String>,
) -> Result<BundleArtifactManifest, String> {
    let source_map_path = source_map_path(bundle_path);
    Ok(BundleArtifactManifest {
        schema_version: 1,
        format: "iife".to_string(),
        global_name: "__ic_edge_bundle".to_string(),
        entrypoint: entrypoint.to_string(),
        bundle_path: bundle_path.to_string_lossy().to_string(),
        bundle_sha256: file_sha256_hex(bundle_path)?,
        source_map_path: source_map_path.to_string_lossy().to_string(),
        source_map_sha256: file_sha256_hex(&source_map_path)?,
        esbuild_args,
    })
}

/// Writes a deterministic pretty JSON artifact manifest.
pub fn write_artifact_manifest(manifest: &BundleArtifactManifest) -> Result<PathBuf, String> {
    let path = artifact_manifest_path(Path::new(&manifest.bundle_path));
    let json = serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(&path, format!("{json}\n")).map_err(|error| error.to_string())?;
    Ok(path)
}

/// Reads and validates bundle provenance before upload.
pub fn verified_artifact_manifest(bundle_path: &Path) -> Result<BundleArtifactManifest, String> {
    let manifest_path = artifact_manifest_path(bundle_path);
    let manifest_json = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("manifest is required: {error}"))?;
    let manifest: BundleArtifactManifest =
        serde_json::from_str(&manifest_json).map_err(|error| error.to_string())?;
    validate_artifact_manifest(bundle_path, &manifest)?;
    Ok(manifest)
}

/// Validates manifest fields and file hashes.
pub fn validate_artifact_manifest(
    bundle_path: &Path,
    manifest: &BundleArtifactManifest,
) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err("unsupported manifest schema_version".to_string());
    }
    if manifest.format != "iife" || manifest.global_name != "__ic_edge_bundle" {
        return Err("manifest does not describe an ic-edge IIFE bundle".to_string());
    }
    let bundle_path = fs::canonicalize(bundle_path).map_err(|error| error.to_string())?;
    let manifest_bundle_path =
        fs::canonicalize(&manifest.bundle_path).map_err(|error| error.to_string())?;
    if manifest_bundle_path != bundle_path {
        return Err("manifest bundle_path does not match upload path".to_string());
    }
    if manifest.source_map_path.is_empty() {
        return Err("source map is required".to_string());
    }
    let source_map =
        fs::canonicalize(&manifest.source_map_path).map_err(|_| "source map is required")?;
    if file_sha256_hex(&bundle_path)? != manifest.bundle_sha256 {
        return Err("bundle sha256 does not match manifest".to_string());
    }
    if file_sha256_hex(&source_map)? != manifest.source_map_sha256 {
        return Err("source map sha256 does not match manifest".to_string());
    }
    Ok(())
}

/// Returns `dist/<entrypoint-stem>.bundle.js`.
pub fn default_out_file(entrypoint: &Path) -> PathBuf {
    let file_name = entrypoint
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("app");
    PathBuf::from("dist").join(format!("{file_name}.bundle.js"))
}

fn source_map_path(bundle_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.map", bundle_path.display()))
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(sha256_hex(&bytes))
}

/// Computes lowercase SHA-256 hex for artifact validation.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Stores a bundle under `module_path`.
pub fn upload_bundle(
    store: &mut impl EdgeStore,
    module_path: &str,
    bytes: &[u8],
) -> StoreResult<()> {
    store.put_module(module_path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_single_bundle_manifest() {
        let manifest = manifest_for_request(PackRequest {
            entrypoint: "src/app.ts".to_string(),
            out_file: "dist/app.bundle.js".to_string(),
        });
        assert_eq!(manifest.entrypoint, "src/app.ts");
        assert_eq!(manifest.bundle_path, "dist/app.bundle.js");
    }

    #[test]
    fn computes_artifact_manifest_path() {
        assert_eq!(
            artifact_manifest_path(Path::new("dist/app.bundle.js")),
            PathBuf::from("dist/app.bundle.js.ic-edge-manifest.json")
        );
    }

    #[test]
    fn creates_default_bundle_path() {
        let out_file = default_out_file(Path::new("src/app.ts"));
        assert_eq!(out_file, PathBuf::from("dist/app.bundle.js"));
    }

    #[test]
    fn uploads_bundle_to_store() {
        let mut store = ic_edge_store::MemoryEdgeStore::new();
        upload_bundle(&mut store, "app", b"bundle").unwrap();
        assert_eq!(store.get_module("app").unwrap(), b"bundle");
    }
}
