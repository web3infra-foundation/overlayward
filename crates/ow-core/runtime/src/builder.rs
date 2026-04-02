use std::sync::Arc;
use std::time::Duration;
use ow_core_traits::*;

#[derive(Debug, Clone)]
pub struct ContainerSpec {
    pub image: ImageSpec,
    pub isolation: IsolationConfig,
    pub filesystem: FilesystemConfig,
    pub network: NetworkConfig,
    pub process: ProcessConfig,
}

#[derive(Debug)]
pub struct BuildOutput {
    pub isolation: IsolationHandle,
    pub filesystem: FilesystemHandle,
    pub network: NetworkHandle,
    pub process: ProcessHandle,
}

pub struct ContainerBuilder {
    backend: Arc<dyn PlatformBackend>,
    isolation: Option<IsolationHandle>,
    filesystem: Option<FilesystemHandle>,
    network: Option<NetworkHandle>,
    process: Option<ProcessHandle>,
}

impl ContainerBuilder {
    pub fn new(backend: Arc<dyn PlatformBackend>) -> Self {
        Self {
            backend,
            isolation: None,
            filesystem: None,
            network: None,
            process: None,
        }
    }

    pub async fn build(&mut self, spec: &ContainerSpec) -> Result<BuildOutput> {
        let image = self.backend.prepare_image(&spec.image).await?;

        self.isolation = Some(match self.backend.create_isolation(&image, &spec.isolation).await {
            Ok(h) => h,
            Err(e) => { self.rollback().await; return Err(e); }
        });

        // Bridge prepare_image output to FilesystemConfig (M2: rootfs_path is JSON layer paths)
        let fs_config = FilesystemConfig {
            rootfs_path: image.rootfs_path.clone(),
            readonly: spec.filesystem.readonly,
        };

        let iso_ref = self.isolation.as_ref().unwrap();
        self.filesystem = Some(match self.backend.setup_filesystem(iso_ref, &fs_config).await {
            Ok(h) => h,
            Err(e) => { self.rollback().await; return Err(e); }
        });

        let iso_ref = self.isolation.as_ref().unwrap();
        self.network = Some(match self.backend.setup_network(iso_ref, &spec.network).await {
            Ok(h) => h,
            Err(e) => { self.rollback().await; return Err(e); }
        });

        let (iso_ref, fs_ref, net_ref) = (
            self.isolation.as_ref().unwrap(),
            self.filesystem.as_ref().unwrap(),
            self.network.as_ref().unwrap(),
        );
        self.process = Some(match self.backend.start_process(iso_ref, fs_ref, net_ref, &spec.process).await {
            Ok(h) => h,
            Err(e) => { self.rollback().await; return Err(e); }
        });

        Ok(BuildOutput {
            isolation: self.isolation.take().unwrap(),
            filesystem: self.filesystem.take().unwrap(),
            network: self.network.take().unwrap(),
            process: self.process.take().unwrap(),
        })
    }

    pub async fn rollback(&mut self) {
        if let Some(h) = self.process.take() {
            if let Err(e) = self.backend.stop_process(&h, Duration::from_secs(10)).await {
                tracing::warn!("rollback stop_process failed: {}", e);
            }
        }
        if let Some(h) = self.network.take() {
            if let Err(e) = self.backend.teardown_network(&h).await {
                tracing::warn!("rollback teardown_network failed: {}", e);
            }
        }
        if let Some(h) = self.filesystem.take() {
            if let Err(e) = self.backend.teardown_filesystem(&h).await {
                tracing::warn!("rollback teardown_filesystem failed: {}", e);
            }
        }
        if let Some(h) = self.isolation.take() {
            if let Err(e) = self.backend.destroy_isolation(&h).await {
                tracing::warn!("rollback destroy_isolation failed: {}", e);
            }
        }
    }
}

impl Drop for ContainerBuilder {
    fn drop(&mut self) {
        let leaked = self.isolation.is_some() as u8
            + self.filesystem.is_some() as u8
            + self.network.is_some() as u8
            + self.process.is_some() as u8;
        if leaked > 0 {
            tracing::warn!(
                "ContainerBuilder dropped with {} uncleaned steps — resources may leak",
                leaked
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ow_core_traits::*;
    use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
    use std::time::Duration;

    struct MockBackend {
        create_isolation_count: AtomicU32,
        setup_filesystem_count: AtomicU32,
        setup_network_count: AtomicU32,
        start_process_count: AtomicU32,
        destroy_isolation_count: AtomicU32,
        teardown_filesystem_count: AtomicU32,
        teardown_network_count: AtomicU32,
        stop_process_count: AtomicU32,
        fail_at_step: Option<String>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                create_isolation_count: AtomicU32::new(0),
                setup_filesystem_count: AtomicU32::new(0),
                setup_network_count: AtomicU32::new(0),
                start_process_count: AtomicU32::new(0),
                destroy_isolation_count: AtomicU32::new(0),
                teardown_filesystem_count: AtomicU32::new(0),
                teardown_network_count: AtomicU32::new(0),
                stop_process_count: AtomicU32::new(0),
                fail_at_step: None,
            }
        }

        fn failing_at(step: &str) -> Self {
            let mut m = Self::new();
            m.fail_at_step = Some(step.to_string());
            m
        }
    }

    #[async_trait::async_trait]
    impl PlatformBackend for MockBackend {
        async fn prepare_image(&self, _spec: &ImageSpec) -> Result<PreparedImage> {
            Ok(PreparedImage { rootfs_path: "/tmp/rootfs".into(), reference: "mock".into() })
        }
        async fn create_isolation(&self, _img: &PreparedImage, _cfg: &IsolationConfig) -> Result<IsolationHandle> {
            self.create_isolation_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_at_step.as_deref() == Some("isolation") {
                return Err(OwError::Other("injected isolation failure".into()));
            }
            Ok(IsolationHandle::new(42u32))
        }
        async fn setup_filesystem(&self, _iso: &IsolationHandle, _cfg: &FilesystemConfig) -> Result<FilesystemHandle> {
            self.setup_filesystem_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_at_step.as_deref() == Some("filesystem") {
                return Err(OwError::Other("injected filesystem failure".into()));
            }
            Ok(FilesystemHandle::new(43u32))
        }
        async fn setup_network(&self, _iso: &IsolationHandle, _cfg: &NetworkConfig) -> Result<NetworkHandle> {
            self.setup_network_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_at_step.as_deref() == Some("network") {
                return Err(OwError::Other("injected network failure".into()));
            }
            Ok(NetworkHandle::new(44u32))
        }
        async fn start_process(&self, _iso: &IsolationHandle, _fs: &FilesystemHandle, _net: &NetworkHandle, _cfg: &ProcessConfig) -> Result<ProcessHandle> {
            self.start_process_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_at_step.as_deref() == Some("process") {
                return Err(OwError::Other("injected process failure".into()));
            }
            Ok(ProcessHandle::new(45u32))
        }
        async fn stop_process(&self, _h: &ProcessHandle, _t: Duration) -> Result<ExitStatus> {
            self.stop_process_count.fetch_add(1, Ordering::SeqCst);
            Ok(ExitStatus::success())
        }
        async fn teardown_network(&self, _h: &NetworkHandle) -> Result<()> {
            self.teardown_network_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn teardown_filesystem(&self, _h: &FilesystemHandle) -> Result<()> {
            self.teardown_filesystem_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn destroy_isolation(&self, _h: &IsolationHandle) -> Result<()> {
            self.destroy_isolation_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn build_success_returns_completed_steps() {
        let backend = Arc::new(MockBackend::new());
        let mut builder = ContainerBuilder::new(backend.clone());
        let spec = test_spec();
        let result = builder.build(&spec).await;
        assert!(result.is_ok());
        assert_eq!(backend.create_isolation_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.start_process_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn build_failure_at_filesystem_rolls_back_isolation() {
        let backend = Arc::new(MockBackend::failing_at("filesystem"));
        let mut builder = ContainerBuilder::new(backend.clone());
        let spec = test_spec();
        let result = builder.build(&spec).await;
        assert!(result.is_err());
        assert_eq!(backend.create_isolation_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.destroy_isolation_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.teardown_filesystem_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn build_failure_at_process_rolls_back_all_prior_steps() {
        let backend = Arc::new(MockBackend::failing_at("process"));
        let mut builder = ContainerBuilder::new(backend.clone());
        let spec = test_spec();
        let result = builder.build(&spec).await;
        assert!(result.is_err());
        assert_eq!(backend.destroy_isolation_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.teardown_filesystem_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.teardown_network_count.load(Ordering::SeqCst), 1);
        assert_eq!(backend.stop_process_count.load(Ordering::SeqCst), 0);
    }

    fn test_spec() -> ContainerSpec {
        ContainerSpec {
            image: ImageSpec { reference: "alpine:latest".into() },
            isolation: IsolationConfig {
                hostname: "test".into(),
                namespaces: vec!["pid".into(), "mnt".into()],
            },
            filesystem: FilesystemConfig { rootfs_path: "/tmp/rootfs".into(), readonly: false },
            network: NetworkConfig { enabled: false },
            process: ProcessConfig {
                args: vec!["/bin/echo".into(), "hello".into()],
                env: vec!["PATH=/usr/bin".into()],
                working_dir: "/".into(),
            },
        }
    }
}
