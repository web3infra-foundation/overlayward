//! M2 integration tests: OCI image → overlayfs rootfs → container exec
//!
//! Run conditions: root + network + overlayfs kernel module + M1 delivery gate PASS
//! cargo test -p ow-linux --test m2_image_pipeline -- --test-threads=1

use std::sync::{Arc, OnceLock};
use std::time::Duration;
use ow_core_image::{ImageConfig, ImageManager};
use ow_core_runtime::builder::{ContainerBuilder, ContainerSpec};
use ow_core_traits::*;
use tempfile::TempDir;

/// Module-level shared TempDir to avoid rkforge CONFIG path invalidation.
static SHARED_DATA_ROOT: OnceLock<TempDir> = OnceLock::new();

fn shared_data_root() -> &'static std::path::Path {
    SHARED_DATA_ROOT
        .get_or_init(|| TempDir::new().expect("failed to create shared test data_root"))
        .path()
}

fn test_image_config() -> ImageConfig {
    ImageConfig {
        data_root: shared_data_root().to_path_buf(),
        default_registry: "registry-1.docker.io".to_string(),
        auth_path: None,
        is_root: true,
    }
}

/// M2 Delivery Gate Part A: image pipeline validation
/// pull → cache → list → overlayfs mount → rootfs content correct → unmount → cleanup
#[tokio::test]
async fn m2_delivery_gate_image_pipeline() {
    let config = test_image_config();
    let manager = ImageManager::new(config).unwrap();

    // Step 1: Pull alpine
    let (manifest_path, layer_paths) = manager.pull("alpine:latest").await.unwrap();
    assert!(manifest_path.exists(), "manifest should exist after pull");
    assert!(!layer_paths.is_empty(), "should have at least one layer");

    // Step 2: Repeated pull hits cache
    let (_, layers2) = manager.pull("alpine:latest").await.unwrap();
    assert_eq!(layer_paths, layers2, "cache hit");

    // Step 3: list contains the pulled image
    let images = manager.list().await.unwrap();
    assert!(images.iter().any(|i| i.reference.contains("alpine")));

    // Step 4: Prepare + Mount
    let prepared = manager.prepare_rootfs("alpine:latest", "test-m2-a").await.unwrap();
    ow_linux::overlay::overlay_mount(
        &prepared.layer_paths, &prepared.upper_dir,
        &prepared.work_dir, &prepared.rootfs_path,
    ).unwrap();

    // Step 5: Verify rootfs content
    let version = std::fs::read_to_string(prepared.rootfs_path.join("etc/alpine-release")).unwrap();
    assert!(version.trim().starts_with("3."), "got: {version}");

    // Step 6: Cleanup
    ow_linux::overlay::overlay_unmount(&prepared.rootfs_path).unwrap();
    manager.cleanup_container("test-m2-a").unwrap();
}

/// M2 Delivery Gate Part B: end-to-end exec via PlatformBackend trait methods
/// prepare_image → create_isolation → setup_filesystem → start_process → stop_process → teardown
#[tokio::test]
async fn m2_delivery_gate_exec() {
    let config = test_image_config();
    let backend = Arc::new(
        ow_linux::LinuxBackend::with_image_manager(config)
            .expect("LinuxBackend init should succeed")
    );

    // Step 1: prepare_image (pull)
    let image_spec = ImageSpec { reference: "alpine:latest".to_string() };
    let prepared = backend.prepare_image(&image_spec).await.unwrap();

    // Step 2: create_isolation
    let isolation_config = IsolationConfig {
        hostname: "m2-test".to_string(),
        namespaces: vec!["pid".into(), "mnt".into(), "uts".into()],
    };
    let isolation = backend.create_isolation(&prepared, &isolation_config).await.unwrap();

    // Step 3: setup_filesystem (overlayfs mount)
    let fs_config = FilesystemConfig {
        rootfs_path: prepared.rootfs_path.clone(),
        readonly: false,
    };
    let fs = backend.setup_filesystem(&isolation, &fs_config).await.unwrap();

    // Step 4: setup_network (no-op stub)
    let net_config = NetworkConfig { enabled: false };
    let net = backend.setup_network(&isolation, &net_config).await.unwrap();

    // Step 5: start_process — exec `cat /etc/alpine-release`
    let proc_config = ProcessConfig {
        args: vec!["cat".into(), "/etc/alpine-release".into()],
        env: vec!["PATH=/usr/bin:/bin".into()],
        working_dir: "/".to_string(),
    };
    let process = backend.start_process(&isolation, &fs, &net, &proc_config).await.unwrap();

    // Step 6: Wait for process
    let exit = backend.stop_process(&process, Duration::from_secs(10)).await.unwrap();
    assert!(exit.is_success(), "cat should exit 0, got: {:?}", exit);

    // Step 7: Verify rootfs content via overlayfs mountpoint
    let container_id = &isolation.downcast_ref::<ow_linux::isolation::LinuxIsolation>()
        .expect("should be LinuxIsolation")
        .container_id;
    let overlay_rootfs = shared_data_root()
        .join("containers")
        .join(container_id)
        .join("rootfs");
    let version = std::fs::read_to_string(overlay_rootfs.join("etc/alpine-release"))
        .expect("should be able to read alpine-release from overlayfs rootfs");
    assert!(version.trim().starts_with("3."), "alpine version mismatch: {version}");

    // Step 7b: stdout verification via LogStore
    let proc = process.downcast_ref::<ow_linux::process::LinuxProcess>()
        .expect("should be LinuxProcess");
    proc.drain_logs().await;
    let logs = proc.log_store.tail(None);
    let stdout_lines: Vec<&str> = logs.iter()
        .filter(|l| l.stream == ow_core_runtime::log_store::LogStream::Stdout)
        .map(|l| l.content.as_str())
        .collect();
    assert!(
        stdout_lines.iter().any(|l| l.starts_with("3.")),
        "stdout should contain alpine version starting with '3.', got: {:?}",
        stdout_lines
    );

    // Step 8: Teardown (reverse order)
    backend.teardown_network(&net).await.unwrap();
    backend.teardown_filesystem(&fs).await.unwrap();
    backend.destroy_isolation(&isolation).await.unwrap();
}

/// M2 Delivery Gate Part C: ContainerBuilder bridge
/// Verifies that ContainerBuilder.build() correctly bridges prepare_image().rootfs_path
/// into FilesystemConfig, so that spec.filesystem.rootfs_path is ignored and the
/// OCI layer paths JSON from prepare_image is used instead.
#[tokio::test]
async fn m2_delivery_gate_builder() {
    let config = test_image_config();
    let backend = Arc::new(
        ow_linux::LinuxBackend::with_image_manager(config)
            .expect("LinuxBackend init should succeed")
    );

    // spec.filesystem.rootfs_path intentionally left empty — builder must use
    // the path returned by prepare_image(), not this field.
    let spec = ContainerSpec {
        image: ImageSpec { reference: "alpine:latest".to_string() },
        isolation: IsolationConfig {
            hostname: "m2-builder-test".to_string(),
            namespaces: vec!["pid".into(), "mnt".into(), "uts".into()],
        },
        filesystem: FilesystemConfig {
            rootfs_path: String::new(),
            readonly: false,
        },
        network: NetworkConfig { enabled: false },
        process: ProcessConfig {
            args: vec!["cat".into(), "/etc/alpine-release".into()],
            env: vec!["PATH=/usr/bin:/bin".into()],
            working_dir: "/".to_string(),
        },
    };

    let mut builder = ContainerBuilder::new(backend.clone());
    let output = builder.build(&spec).await
        .expect("ContainerBuilder.build() should succeed");

    // Wait for process and verify success
    let exit = backend.stop_process(&output.process, Duration::from_secs(10)).await
        .expect("stop_process should succeed");
    assert!(exit.is_success(), "cat should exit 0, got: {:?}", exit);

    // Verify stdout via LogStore
    let proc = output.process.downcast_ref::<ow_linux::process::LinuxProcess>()
        .expect("should be LinuxProcess");
    proc.drain_logs().await;
    let logs = proc.log_store.tail(None);
    let stdout_lines: Vec<&str> = logs.iter()
        .filter(|l| l.stream == ow_core_runtime::log_store::LogStream::Stdout)
        .map(|l| l.content.as_str())
        .collect();
    assert!(
        stdout_lines.iter().any(|l| l.starts_with("3.")),
        "stdout should contain alpine version starting with '3.', got: {:?}",
        stdout_lines
    );

    // Teardown
    backend.teardown_network(&output.network).await.unwrap();
    backend.teardown_filesystem(&output.filesystem).await.unwrap();
    backend.destroy_isolation(&output.isolation).await.unwrap();
}
