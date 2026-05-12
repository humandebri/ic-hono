//! `crates/ic-edge-loader` records bundled module metadata.
//! v1 starts with a single bundle before adding a full module graph.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    pub entrypoint: String,
    pub bundle_path: String,
}

impl BundleManifest {
    pub fn single_bundle(entrypoint: String, bundle_path: String) -> Self {
        Self {
            entrypoint,
            bundle_path,
        }
    }
}
