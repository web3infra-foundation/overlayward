use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("image not found: {reference}")]
    NotFound { reference: String },

    #[error("pull failed for {reference}: {source}")]
    PullFailed {
        reference: String,
        source: anyhow::Error,
    },

    #[error("storage error at {path}: {source}")]
    StorageError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("manifest parse error for {reference}: {source}")]
    ManifestError {
        reference: String,
        source: anyhow::Error,
    },

    #[error("overlay mount failed for container {container_id}: {source}")]
    OverlayMountFailed {
        container_id: String,
        source: std::io::Error,
    },

    #[error("config not initialized — call ImageManager::new() first")]
    ConfigNotInitialized,

    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type ImageResult<T> = Result<T, ImageError>;
