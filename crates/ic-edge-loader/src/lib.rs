//! `crates/ic-edge-loader` records bundled module metadata.
//! v1 starts with a single bundle before adding a full module graph.

/// Manifest for the v1 single-bundle packaging contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    /// Source entrypoint passed to the bundler.
    pub entrypoint: String,
    /// Output bundle path consumed by the runtime.
    pub bundle_path: String,
}

impl BundleManifest {
    /// Creates a manifest for one IIFE bundle.
    pub fn single_bundle(entrypoint: String, bundle_path: String) -> Self {
        Self {
            entrypoint,
            bundle_path,
        }
    }
}
