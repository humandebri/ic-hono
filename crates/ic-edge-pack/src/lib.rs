//! `crates/ic-edge-pack` defines packer inputs and outputs.
//! The executable CLI can wrap this API after the bundle format settles.

use ic_edge_loader::BundleManifest;
use ic_edge_store::{EdgeStore, Result as StoreResult};
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

/// Returns `dist/<entrypoint-stem>.bundle.js`.
pub fn default_out_file(entrypoint: &Path) -> PathBuf {
    let file_name = entrypoint
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("app");
    PathBuf::from("dist").join(format!("{file_name}.bundle.js"))
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
