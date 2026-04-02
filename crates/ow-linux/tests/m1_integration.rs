//! M1 Delivery Gate: echo hello in container.
//! Requires: Linux, root, --test-threads=1

use std::sync::Arc;
use ow_core_traits::*;
use ow_core_runtime::builder::{ContainerBuilder, ContainerSpec};
use ow_linux::LinuxBackend;

#[tokio::test]
async fn m1_echo_hello_in_container() {
    let spec = ContainerSpec {
        image: ImageSpec { reference: "/".into() },
        isolation: IsolationConfig {
            hostname: "m1-test".into(),
            namespaces: vec!["pid".into(), "mnt".into(), "uts".into(), "ipc".into()],
        },
        filesystem: FilesystemConfig {
            rootfs_path: "/".into(),
            readonly: false,
        },
        network: NetworkConfig { enabled: false },
        process: ProcessConfig {
            args: vec!["/bin/echo".into(), "hello".into()],
            env: vec!["PATH=/usr/bin:/bin".into()],
            working_dir: "/".into(),
        },
    };

    let backend = Arc::new(LinuxBackend::new());
    let mut builder = ContainerBuilder::new(backend.clone());

    let output = builder.build(&spec).await
        .expect("container build should succeed");

    let proc = output.process.downcast_ref::<ow_linux::process::LinuxProcess>()
        .expect("should be LinuxProcess");

    // Wait for the short-lived process to finish, then drain log tasks
    let mut status: libc::c_int = 0;
    unsafe { libc::waitpid(proc.pid, &mut status, 0); }
    proc.drain_logs().await;

    let logs = proc.log_store.tail(None);
    let stdout_lines: Vec<&str> = logs.iter()
        .filter(|l| l.stream == ow_core_runtime::log_store::LogStream::Stdout)
        .map(|l| l.content.as_str())
        .collect();
    assert!(
        stdout_lines.iter().any(|l| l.contains("hello")),
        "M1 gate failed: stdout should contain 'hello', got: {:?}",
        stdout_lines
    );

    backend.teardown_network(&output.network).await.ok();
    backend.teardown_filesystem(&output.filesystem).await.ok();
    backend.destroy_isolation(&output.isolation).await.ok();
}
