use std::path::{Path, PathBuf};
use std::fs;
use crate::error::{ImageError, ImageResult};

/// Manages Overlayward's local image storage directory structure.
pub struct ImageStore {
    blobs_dir: PathBuf,
    manifests_dir: PathBuf,
    repositories_path: PathBuf,
    containers_dir: PathBuf,
}

impl ImageStore {
    /// Create ImageStore and ensure directory structure exists.
    pub fn new(data_root: &Path) -> ImageResult<Self> {
        let blobs_dir = data_root.join("images/blobs/sha256");
        let manifests_dir = data_root.join("images/manifests/sha256");
        let repositories_path = data_root.join("images/repositories.toml");
        let containers_dir = data_root.join("containers");

        for dir in [&blobs_dir, &manifests_dir, &containers_dir] {
            fs::create_dir_all(dir).map_err(|e| ImageError::StorageError {
                path: dir.clone(),
                source: e,
            })?;
        }

        Ok(Self {
            blobs_dir,
            manifests_dir,
            repositories_path,
            containers_dir,
        })
    }

    /// Get the path for a given blob digest.
    /// digest format: "sha256:abc123..." → blobs_dir/abc123...
    pub fn blob_path(&self, digest: &str) -> PathBuf {
        let hash = digest.strip_prefix("sha256:").unwrap_or(digest);
        self.blobs_dir.join(hash)
    }

    pub fn manifests_dir(&self) -> &Path {
        &self.manifests_dir
    }

    pub fn repositories_path(&self) -> &Path {
        &self.repositories_path
    }

    pub fn blob_exists(&self, digest: &str) -> bool {
        self.blob_path(digest).exists()
    }

    /// Create container rootfs/upper/work directories, returns (rootfs_path, upper_path, work_path).
    pub fn container_dirs(&self, container_id: &str) -> ImageResult<(PathBuf, PathBuf, PathBuf)> {
        let container_dir = self.containers_dir.join(container_id);
        let rootfs = container_dir.join("rootfs");
        let upper = container_dir.join("upper");
        let work = container_dir.join("work");

        for dir in [&rootfs, &upper, &work] {
            fs::create_dir_all(dir).map_err(|e| ImageError::StorageError {
                path: dir.clone(),
                source: e,
            })?;
        }

        Ok((rootfs, upper, work))
    }

    /// Clean up container directories (called on remove).
    pub fn remove_container_dirs(&self, container_id: &str) -> ImageResult<()> {
        let container_dir = self.containers_dir.join(container_id);
        if container_dir.exists() {
            fs::remove_dir_all(&container_dir).map_err(|e| ImageError::StorageError {
                path: container_dir,
                source: e,
            })?;
        }
        Ok(())
    }

    /// Blobs root directory (rkforge needs this as layers_store_root).
    pub fn blobs_root(&self) -> &Path {
        &self.blobs_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_store() -> (TempDir, ImageStore) {
        let tmp = TempDir::new().unwrap();
        let store = ImageStore::new(tmp.path()).unwrap();
        (tmp, store)
    }

    #[test]
    fn new_creates_directory_structure() {
        let (tmp, _store) = setup_store();
        assert!(tmp.path().join("images/blobs/sha256").is_dir());
        assert!(tmp.path().join("images/manifests/sha256").is_dir());
        assert!(tmp.path().join("containers").is_dir());
    }

    #[test]
    fn blob_path_returns_correct_path() {
        let (tmp, store) = setup_store();
        let path = store.blob_path("sha256:abc123");
        assert_eq!(path, tmp.path().join("images/blobs/sha256/abc123"));
    }

    #[test]
    fn container_dirs_creates_upper_work_rootfs() {
        let (_tmp, store) = setup_store();
        let (rootfs, upper, work) = store.container_dirs("test-container-1").unwrap();
        assert!(rootfs.parent().unwrap().is_dir());
        assert!(upper.is_dir());
        assert!(work.is_dir());
        assert!(rootfs.to_str().unwrap().contains("test-container-1"));
    }
}
