use std::time::Duration;
use async_trait::async_trait;
use crate::error::Result;
use crate::handles::*;

#[derive(Debug, Clone)]
pub struct ImageSpec {
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct IsolationConfig {
    pub hostname: String,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FilesystemConfig {
    pub rootfs_path: String,
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub working_dir: String,
}

#[derive(Debug, Clone)]
pub struct PreparedImage {
    pub rootfs_path: String,
    pub reference: String,
}

#[async_trait]
pub trait PlatformBackend: Send + Sync {
    async fn prepare_image(&self, spec: &ImageSpec) -> Result<PreparedImage>;
    async fn create_isolation(&self, image: &PreparedImage, config: &IsolationConfig) -> Result<IsolationHandle>;
    async fn setup_filesystem(&self, isolation: &IsolationHandle, config: &FilesystemConfig) -> Result<FilesystemHandle>;
    async fn setup_network(&self, isolation: &IsolationHandle, config: &NetworkConfig) -> Result<NetworkHandle>;
    async fn start_process(&self, isolation: &IsolationHandle, fs: &FilesystemHandle, network: &NetworkHandle, config: &ProcessConfig) -> Result<ProcessHandle>;
    async fn stop_process(&self, handle: &ProcessHandle, timeout: Duration) -> Result<ExitStatus>;
    async fn teardown_network(&self, handle: &NetworkHandle) -> Result<()>;
    async fn teardown_filesystem(&self, handle: &FilesystemHandle) -> Result<()>;
    async fn destroy_isolation(&self, handle: &IsolationHandle) -> Result<()>;
}
