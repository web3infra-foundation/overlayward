use crate::error::{ImageError, ImageResult};

/// Wraps rkforge registry auth configuration.
/// M2: manages auth loading, injects into rkforge via init_config.
/// DS-2+: will extend to construct oci-client::Client instances directly.
pub struct RegistryClient {
    default_registry: String,
}

impl RegistryClient {
    pub fn new(default_registry: String, _auth_path: Option<&std::path::Path>) -> ImageResult<Self> {
        // M2: auth is handled internally by rkforge through its config.
        // RegistryClient primarily tracks the default registry.
        Ok(Self {
            default_registry,
        })
    }

    pub fn default_registry(&self) -> &str {
        &self.default_registry
    }
}
