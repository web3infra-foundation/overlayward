//! OCI image management for Overlayward.
//! Wraps rkforge pull/storage/registry as library API.

pub mod error;
pub mod types;
pub mod store;
pub mod registry_client;
pub mod manager;

pub use error::{ImageError, ImageResult};
pub use types::{ImageConfig, ImageInfo, PreparedImage};
pub use manager::ImageManager;
pub use store::ImageStore;
