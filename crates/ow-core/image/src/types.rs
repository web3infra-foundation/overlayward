use std::path::PathBuf;

/// ImageManager configuration. Constructed by the caller (LinuxBackend) and passed in.
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Overlayward data root directory.
    /// Default: /var/lib/overlayward (root) or ~/.local/share/overlayward (non-root)
    pub data_root: PathBuf,

    /// Default OCI registry. Default: "registry-1.docker.io"
    pub default_registry: String,

    /// Auth config file path. Default: {data_root}/config/auth.toml
    pub auth_path: Option<PathBuf>,

    /// Whether running as root. Detected by LinuxBackend, passed in here.
    pub is_root: bool,
}

impl ImageConfig {
    /// Create default config for root user.
    pub fn for_root() -> Self {
        Self {
            data_root: PathBuf::from("/var/lib/overlayward"),
            default_registry: "registry-1.docker.io".to_string(),
            auth_path: None,
            is_root: true,
        }
    }

    /// Create default config for non-root user.
    pub fn for_user() -> Self {
        let data_root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("overlayward");
        Self {
            data_root,
            default_registry: "registry-1.docker.io".to_string(),
            auth_path: None,
            is_root: false,
        }
    }
}

/// Internal rich type returned by ImageManager::prepare_rootfs().
/// NOT the same as ow-core-traits::PreparedImage (which is the cross-platform interface type).
#[derive(Debug, Clone)]
pub struct PreparedImage {
    /// Overlay mount point path (container rootfs)
    pub rootfs_path: PathBuf,
    /// Image reference (e.g. "library/alpine:latest")
    pub reference: String,
    /// Layer directory list (bottom to top), for overlayfs lowerdir
    pub layer_paths: Vec<PathBuf>,
    /// Container upper (write) layer directory
    pub upper_dir: PathBuf,
    /// overlayfs work directory
    pub work_dir: PathBuf,
}

/// Local image info (returned by list)
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub reference: String,
    pub tag: String,
    pub digest: String,
    pub size: u64,
    pub created: Option<String>,
}
