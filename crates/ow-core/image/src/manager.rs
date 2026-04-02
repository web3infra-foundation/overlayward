use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::{ImageError, ImageResult};
use crate::store::ImageStore;
use crate::registry_client::RegistryClient;
use crate::types::{ImageConfig, ImageInfo, PreparedImage};
use rkforge::config::auth::AuthConfig;

/// OCI image manager. Wraps rkforge pull/storage/images modules.
pub struct ImageManager {
    store: ImageStore,
    #[allow(dead_code)]
    registry: RegistryClient,
    config: ImageConfig,
}

impl ImageManager {
    /// Create ImageManager and initialize rkforge CONFIG.
    /// Uses first-writer-wins + field conflict detection for the global CONFIG.
    pub fn new(config: ImageConfig) -> ImageResult<Self> {
        let store = ImageStore::new(&config.data_root)?;
        let registry = RegistryClient::new(
            config.default_registry.clone(),
            config.auth_path.as_deref(),
        )?;

        // Initialize rkforge global CONFIG
        let rkforge_config = rkforge::config::image::Config {
            layers_store_root: store.blobs_root().to_path_buf(),
            build_dir: config.data_root.join("build"),
            metadata_dir: config.data_root.join("images"),
            default_registry: config.default_registry.clone(),
            is_root: config.is_root,
        };

        // Create directories that rkforge expects
        std::fs::create_dir_all(&rkforge_config.layers_store_root)
            .map_err(|e| ImageError::StorageError {
                path: rkforge_config.layers_store_root.clone(),
                source: e,
            })?;
        std::fs::create_dir_all(&rkforge_config.build_dir)
            .map_err(|e| ImageError::StorageError {
                path: rkforge_config.build_dir.clone(),
                source: e,
            })?;
        std::fs::create_dir_all(&rkforge_config.metadata_dir)
            .map_err(|e| ImageError::StorageError {
                path: rkforge_config.metadata_dir.clone(),
                source: e,
            })?;

        rkforge::config::image::init_config(rkforge_config);

        info!(data_root = %config.data_root.display(), "ImageManager initialized");

        Ok(Self {
            store,
            registry,
            config,
        })
    }

    /// Pull an image to local storage. Returns (manifest_path, layer_paths).
    pub async fn pull(&self, reference: &str) -> ImageResult<(PathBuf, Vec<PathBuf>)> {
        info!(reference, "pulling image");

        let auth_config = match &self.config.auth_path {
            Some(path) => AuthConfig::load_from(path),
            None => AuthConfig::load(),
        }
        .map_err(|e| ImageError::PullFailed {
            reference: reference.to_string(),
            source: e,
        })?;

        let (manifest_path, layer_paths) =
            rkforge::pull::pull_or_get_image_with_config(reference, None::<&str>, &auth_config)
                .await
                .map_err(|e| ImageError::PullFailed {
                    reference: reference.to_string(),
                    source: e,
                })?;

        info!(
            reference,
            layers = layer_paths.len(),
            "image pull complete"
        );

        Ok((manifest_path, layer_paths))
    }

    /// Pull image and prepare rootfs directories for a container.
    /// Does NOT mount — mount is done by LinuxBackend.
    pub async fn prepare_rootfs(
        &self,
        reference: &str,
        container_id: &str,
    ) -> ImageResult<PreparedImage> {
        let (_manifest_path, layer_paths) = self.pull(reference).await?;
        let (rootfs, upper, work) = self.store.container_dirs(container_id)?;

        info!(
            container_id,
            reference,
            layers = layer_paths.len(),
            rootfs = %rootfs.display(),
            "rootfs prepared (mount pending)"
        );

        Ok(PreparedImage {
            rootfs_path: rootfs,
            reference: reference.to_string(),
            layer_paths,
            upper_dir: upper,
            work_dir: work,
        })
    }

    /// List all locally cached images.
    pub async fn list(&self) -> ImageResult<Vec<ImageInfo>> {
        let repos = rkforge::config::meta::Repositories::load().map_err(|e| {
            ImageError::StorageError {
                path: self.store.repositories_path().to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            }
        })?;

        let entries = repos.entries();
        let mut images = Vec::with_capacity(entries.len());

        for (image_ref, digest) in entries {
            let (repo, tag) = split_image_ref(image_ref);
            images.push(ImageInfo {
                reference: repo.to_string(),
                tag: tag.to_string(),
                digest: digest.clone(),
                size: 0,
                created: None,
            });
        }

        Ok(images)
    }

    /// Remove a local image.
    pub async fn remove(&self, reference: &str) -> ImageResult<()> {
        let args = rkforge::images::RmiArgs {
            image_ref: reference.to_string(),
            force: false,
        };

        rkforge::images::remove_image(args).map_err(|e| ImageError::ManifestError {
            reference: reference.to_string(),
            source: e,
        })?;

        info!(reference, "image removed");
        Ok(())
    }

    /// Clean up container directories (called on container delete).
    pub fn cleanup_container(&self, container_id: &str) -> ImageResult<()> {
        self.store.remove_container_dirs(container_id)
    }

    /// Build — not implemented in DS-1.
    pub async fn build(&self, _context: &Path, _dockerfile: &Path) -> ImageResult<PreparedImage> {
        Err(ImageError::Unsupported(
            "image build not implemented in DS-1".to_string(),
        ))
    }

    /// Expose data_root for LinuxBackend.
    pub fn data_root(&self) -> &Path {
        &self.config.data_root
    }
}

fn split_image_ref(image_ref: &str) -> (&str, &str) {
    match image_ref.rsplit_once(':') {
        Some((repo, tag)) => (repo, tag),
        None => (image_ref, "latest"),
    }
}
