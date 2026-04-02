pub mod ffi;
pub mod isolation;
pub mod filesystem;
pub mod process;
pub mod overlay;

use std::collections::HashMap;
use std::io::BufRead;
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;
use std::time::Duration;
use async_trait::async_trait;
use ow_core_traits::*;
use ow_core_image::{ImageManager, ImageConfig};
use crate::isolation::LinuxIsolation;
use crate::filesystem::LinuxFilesystem;
use crate::process::LinuxProcess;
use ow_core_runtime::log_store::LogStore;

/// Tracks overlay mount state for a container (for teardown).
#[derive(Debug)]
struct OverlayInfo {
    mountpoint: std::path::PathBuf,
}

pub struct LinuxBackend {
    image_manager: Option<ImageManager>,
    overlay_mounts: Mutex<HashMap<String, OverlayInfo>>,
}

impl LinuxBackend {
    /// Create a LinuxBackend without ImageManager (M1 compatible).
    pub fn new() -> Self {
        Self {
            image_manager: None,
            overlay_mounts: Mutex::new(HashMap::new()),
        }
    }

    /// Create a LinuxBackend with ImageManager (M2+).
    pub fn with_image_manager(config: ImageConfig) -> std::result::Result<Self, OwError> {
        let image_manager = ImageManager::new(config)
            .map_err(|e| OwError::Other(e.to_string()))?;
        Ok(Self {
            image_manager: Some(image_manager),
            overlay_mounts: Mutex::new(HashMap::new()),
        })
    }

    /// Teardown overlay mount + container directories for a container.
    pub fn teardown_overlay(&self, container_id: &str) -> std::result::Result<(), OwError> {
        let mut mounts = self.overlay_mounts.lock().unwrap();
        if let Some(info) = mounts.remove(container_id) {
            overlay::overlay_unmount(&info.mountpoint)
                .map_err(|e| OwError::Other(
                    format!("overlay unmount failed for {container_id}: {e}")
                ))?;
            tracing::info!(container_id, "overlay unmounted");
        }
        if let Some(ref manager) = self.image_manager {
            manager.cleanup_container(container_id)
                .map_err(|e| OwError::Other(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for LinuxBackend {
    fn default() -> Self { Self::new() }
}

/// Pre-fork data prepared from handles, avoiding heap allocs after fork.
struct ChildParams {
    ns_fds: crate::ffi::namespace::OwNsResult,
    rootfs_source: std::ffi::CString,
    rootfs_mount: std::ffi::CString,
    #[allow(dead_code)] // owns CStrings that c_arg_ptrs points into
    c_args: Vec<std::ffi::CString>,
    c_arg_ptrs: Vec<*const libc::c_char>,
    err_pipe_wr: libc::c_int,
    cgroup_fd: libc::c_int,
    pseudo_fs: crate::filesystem::PseudoFsParams,
}

// c_arg_ptrs contains raw pointers derived from c_args which are owned by ChildParams
unsafe impl Send for ChildParams {}

impl ChildParams {
    fn new(
        ns_fds: crate::ffi::namespace::OwNsResult,
        rootfs_source: std::ffi::CString,
        rootfs_mount: std::ffi::CString,
        c_args: Vec<std::ffi::CString>,
        err_pipe_wr: libc::c_int,
        cgroup_fd: libc::c_int,
        pseudo_fs: crate::filesystem::PseudoFsParams,
    ) -> Self {
        let c_arg_ptrs: Vec<*const libc::c_char> = c_args.iter()
            .map(|a| a.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();
        Self { ns_fds, rootfs_source, rootfs_mount, c_args, c_arg_ptrs, err_pipe_wr, cgroup_fd, pseudo_fs }
    }
}

/// Write step+errno to error pipe and abort. Async-signal-safe.
#[inline(always)]
unsafe fn child_abort(fd: libc::c_int, step: u8) -> ! {
    let errno = *libc::__errno_location();
    let mut buf = [0u8; 5];
    buf[0] = step;
    buf[1..5].copy_from_slice(&errno.to_ne_bytes());
    libc::write(fd, buf.as_ptr() as *const libc::c_void, 5);
    libc::_exit(127);
}

/// Runs in child1 after fork. Only async-signal-safe calls + pre-allocated data.
unsafe fn child_entry(params: &ChildParams) -> ! {
    let efd = params.err_pipe_wr;

    // Become process group leader so stop_process can kill(-pid) the entire
    // group (child1 shim + child2 workload) instead of just the shim.
    libc::setpgid(0, 0);

    // Enter non-PID namespaces via raw FFI (no allocation)
    // Steps: 1=NEWNS, 2=NEWUTS, 3=NEWIPC
    // For mount namespace: unshare a fresh one instead of setns into the holder's,
    // because setns(CLONE_NEWNS) enters a shared mount table where pivot_root
    // can fail if the root mount identity hasn't changed.
    if libc::unshare(libc::CLONE_NEWNS) != 0 {
        child_abort(efd, 1);
    }
    for ns_type in &[libc::CLONE_NEWUTS, libc::CLONE_NEWIPC] {
        let step: u8 = if *ns_type == libc::CLONE_NEWUTS { 2 } else { 3 };
        if crate::ffi::namespace::ow_ns_enter(&params.ns_fds, *ns_type) != 0 {
            child_abort(efd, step);
        }
    }

    // Stop mount propagation to parent namespace (required for pivot_root)
    if libc::mount(
        std::ptr::null(),
        b"/\0".as_ptr() as *const libc::c_char,
        std::ptr::null(),
        libc::MS_SLAVE | libc::MS_REC,
        std::ptr::null(),
    ) != 0 {
        child_abort(efd, 10); // step 10: make mounts slave
    }

    // Bind-mount rootfs, then pivot_root(".", ".") (runc-style, Linux ≥3.16)
    let mount_ptr = params.rootfs_mount.as_ptr();
    libc::mkdir(mount_ptr, 0o755);
    if libc::mount(
        params.rootfs_source.as_ptr(),
        mount_ptr,
        std::ptr::null(),
        libc::MS_BIND | libc::MS_REC,
        std::ptr::null(),
    ) != 0 {
        child_abort(efd, 4); // step 4: bind mount
    }

    // step 11: mount pseudo-filesystems into rootfs (before pivot_root)
    let pseudo_result = crate::filesystem::setup_pseudo_filesystems_raw(&params.pseudo_fs);
    if pseudo_result != 0 {
        child_abort(efd, 20 + pseudo_result as u8);
    }

    if libc::chdir(mount_ptr) != 0 {
        child_abort(efd, 5);
    }
    let dot = b".\0".as_ptr() as *const libc::c_char;
    if libc::syscall(libc::SYS_pivot_root, dot, dot) != 0 {
        child_abort(efd, 5); // step 5: pivot_root
    }
    // Detach old root (now at "." after pivot)
    libc::umount2(dot, libc::MNT_DETACH);
    libc::chdir(b"/\0".as_ptr() as *const libc::c_char);

    // Enter PID namespace
    if crate::ffi::namespace::ow_ns_enter(&params.ns_fds, libc::CLONE_NEWPID) != 0 {
        child_abort(efd, 6); // step 6: PID ns
    }

    // Double fork so child2 is PID 1 in the new PID namespace.
    // Error pipe stays open: child1 closes its copy after fork; child2 keeps it
    // with O_CLOEXEC so exec auto-closes on success, or child_abort reports failure.
    let pid2 = libc::fork();
    if pid2 < 0 { child_abort(efd, 7); }
    if pid2 > 0 {
        // child1: release fds it doesn't need, then wait for child2
        libc::close(efd);
        if params.cgroup_fd >= 0 { libc::close(params.cgroup_fd); }
        let mut status: libc::c_int = 0;
        libc::waitpid(pid2, &mut status, 0);
        if libc::WIFEXITED(status) {
            libc::_exit(libc::WEXITSTATUS(status));
        } else {
            libc::_exit(128 + libc::WTERMSIG(status));
        }
    }

    // child2: attach self to cgroup before exec
    if params.cgroup_fd >= 0 {
        if libc::write(params.cgroup_fd, b"0\n".as_ptr() as *const libc::c_void, 2) < 0 {
            child_abort(efd, 8); // step 8: cgroup attach
        }
        libc::close(params.cgroup_fd);
    }

    // exec the workload — efd has O_CLOEXEC, auto-closes on success
    libc::execvp(params.c_arg_ptrs[0], params.c_arg_ptrs.as_ptr());
    child_abort(efd, 9); // step 9: execvp failed
}

fn namespace_flags(names: &[String]) -> i32 {
    let mut flags = 0i32;
    for name in names {
        flags |= match name.as_str() {
            "pid" => libc::CLONE_NEWPID,
            "mnt" | "mount" => libc::CLONE_NEWNS,
            "uts" => libc::CLONE_NEWUTS,
            "ipc" => libc::CLONE_NEWIPC,
            "net" | "network" => libc::CLONE_NEWNET,
            "cgroup" => libc::CLONE_NEWCGROUP,
            "user" => libc::CLONE_NEWUSER,
            _ => 0,
        };
    }
    flags
}

fn spawn_log_reader(
    pipe: os_pipe::PipeReader,
    store: LogStore,
    stream: ow_core_runtime::log_store::LogStream,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let reader = std::io::BufReader::new(pipe);
        for line in reader.lines().flatten() {
            store.push(stream, line);
        }
    })
}

#[async_trait]
impl PlatformBackend for LinuxBackend {
    async fn prepare_image(&self, spec: &ImageSpec) -> Result<PreparedImage> {
        if let Some(ref manager) = self.image_manager {
            // M2: pull via ImageManager, encode layer paths as JSON
            let (_manifest_path, layer_paths) = manager
                .pull(&spec.reference)
                .await
                .map_err(|e| OwError::Other(e.to_string()))?;

            let layer_info = serde_json::to_string(&layer_paths)
                .map_err(|e| OwError::Other(e.to_string()))?;

            tracing::info!(reference = %spec.reference, layers = layer_paths.len(), "image pulled, ready for mount");

            Ok(PreparedImage {
                rootfs_path: layer_info,
                reference: spec.reference.clone(),
            })
        } else {
            // M1 compatible: rootfs_path is a local directory
            Ok(PreparedImage {
                rootfs_path: spec.reference.clone(),
                reference: spec.reference.clone(),
            })
        }
    }

    async fn create_isolation(
        &self,
        _image: &PreparedImage,
        config: &IsolationConfig,
    ) -> Result<IsolationHandle> {
        let clone_flags = namespace_flags(&config.namespaces);
        let iso = isolation::create_isolation(&config.hostname, clone_flags, 256 * 1024 * 1024)
            .map_err(OwError::Io)?;
        Ok(isolation::to_handle(iso))
    }

    async fn setup_filesystem(
        &self,
        isolation: &IsolationHandle,
        config: &FilesystemConfig,
    ) -> Result<FilesystemHandle> {
        let iso = isolation.downcast_ref::<LinuxIsolation>()
            .ok_or_else(|| OwError::Other("invalid isolation handle".into()))?;

        // Detect if rootfs_path is JSON (M2 OCI layers) or plain path (M1 compatible)
        if let Ok(layer_paths) = serde_json::from_str::<Vec<std::path::PathBuf>>(&config.rootfs_path) {
            // M2 path: overlayfs mount
            let data_root = self.image_manager.as_ref()
                .ok_or_else(|| OwError::Other("ImageManager not initialized for overlay mount".into()))?
                .data_root();
            let fs = filesystem::setup_overlayfs_rootfs(
                &iso.container_id,
                &layer_paths,
                data_root,
            ).map_err(OwError::Io)?;
            // Record overlay info for teardown
            let mut mounts = self.overlay_mounts.lock().unwrap();
            mounts.insert(iso.container_id.clone(), OverlayInfo {
                mountpoint: fs.rootfs_mount_point.clone(),
            });
            Ok(filesystem::to_handle(fs))
        } else {
            // M1 compatible path: simple directory
            let fs = filesystem::setup_filesystem(&iso.container_id, &config.rootfs_path)
                .map_err(OwError::Io)?;
            Ok(filesystem::to_handle(fs))
        }
    }

    async fn setup_network(
        &self,
        _isolation: &IsolationHandle,
        _config: &NetworkConfig,
    ) -> Result<NetworkHandle> {
        Ok(NetworkHandle::new(()))
    }

    async fn start_process(
        &self,
        isolation: &IsolationHandle,
        fs: &FilesystemHandle,
        _network: &NetworkHandle,
        config: &ProcessConfig,
    ) -> Result<ProcessHandle> {
        let iso = isolation.downcast_ref::<LinuxIsolation>()
            .ok_or_else(|| OwError::Other("invalid isolation handle".into()))?;
        let linux_fs = fs.downcast_ref::<LinuxFilesystem>()
            .ok_or_else(|| OwError::Other("invalid filesystem handle".into()))?;

        // Pre-allocate all data needed by the child before fork
        let mount_point = linux_fs.rootfs_mount_point.to_str()
            .ok_or_else(|| OwError::Other("rootfs path not UTF-8".into()))?;
        let rootfs_source = linux_fs.rootfs_source.to_str()
            .ok_or_else(|| OwError::Other("rootfs source not UTF-8".into()))?;
        let c_args: Vec<std::ffi::CString> = config.args.iter()
            .map(|a| std::ffi::CString::new(a.as_str())
                .map_err(|_| OwError::Other(format!("arg contains null byte: {}", a))))
            .collect::<Result<_>>()?;

        // Open cgroup.procs fd for child to self-attach (survives fork + pivot_root)
        let cgroup_procs = iso.cgroup_path.join("cgroup.procs");
        let cgroup_fd = {
            let c_path = std::ffi::CString::new(
                cgroup_procs.to_str()
                    .ok_or_else(|| OwError::Other("cgroup path not UTF-8".into()))?
            ).map_err(|_| OwError::Other("cgroup path contains null".into()))?;
            unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY) }
        };
        if cgroup_fd < 0 {
            return Err(OwError::Io(std::io::Error::last_os_error()));
        }

        // Error pipe: child writes errno on setup failure; O_CLOEXEC auto-closes on exec
        let mut err_pipe = [0 as libc::c_int; 2];
        if unsafe { libc::pipe2(err_pipe.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            unsafe { libc::close(cgroup_fd); }
            return Err(OwError::Io(std::io::Error::last_os_error()));
        }

        // Pre-allocate pseudo-fs params before fork
        let pseudo_fs = filesystem::PseudoFsParams::new(
            std::path::Path::new(mount_point)
        );

        let params = ChildParams::new(
            iso.ns_result,
            std::ffi::CString::new(rootfs_source)
                .map_err(|_| OwError::Other("rootfs source contains null".into()))?,
            std::ffi::CString::new(mount_point)
                .map_err(|_| OwError::Other("rootfs mount contains null".into()))?,
            c_args,
            err_pipe[1],
            cgroup_fd,
            pseudo_fs,
        );

        let log_store = LogStore::new();
        let (stdout_read, stdout_write) = os_pipe::pipe().map_err(OwError::Io)?;
        let (stderr_read, stderr_write) = os_pipe::pipe().map_err(OwError::Io)?;

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(err_pipe[0]);
                libc::close(err_pipe[1]);
                libc::close(cgroup_fd);
            }
            return Err(OwError::Io(err));
        }
        if pid == 0 {
            unsafe { libc::close(err_pipe[0]); }
            drop(stdout_read);
            drop(stderr_read);
            unsafe {
                libc::dup2(stdout_write.as_raw_fd(), 1);
                libc::dup2(stderr_write.as_raw_fd(), 2);
            }
            drop(stdout_write);
            drop(stderr_write);
            unsafe { child_entry(&params); }
        }

        // Parent: close write ends
        unsafe {
            libc::close(err_pipe[1]);
            libc::close(cgroup_fd);
        }
        drop(stdout_write);
        drop(stderr_write);

        // Block until child closes error pipe (setup done) or writes step+errno (setup failed).
        // This is intentionally a blocking read — the wait is bounded by namespace/mount
        // setup time (sub-millisecond) and avoids spawn_blocking scheduling overhead.
        let mut err_buf = [0u8; 5];
        let n = unsafe {
            libc::read(err_pipe[0], err_buf.as_mut_ptr() as *mut libc::c_void, 5)
        };
        unsafe { libc::close(err_pipe[0]); }

        if n > 0 {
            let step = err_buf[0];
            let errno = i32::from_ne_bytes([err_buf[1], err_buf[2], err_buf[3], err_buf[4]]);
            unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0); }
            let desc = match step {
                1 => "unshare(CLONE_NEWNS)",
                2 => "setns(CLONE_NEWUTS)",
                3 => "setns(CLONE_NEWIPC)",
                4 => "bind mount",
                5 => "pivot_root",
                6 => "setns(CLONE_NEWPID)",
                7 => "fork(child2)",
                8 => "cgroup attach",
                9 => "execvp",
                10 => "mount(MS_SLAVE|MS_REC)",
                21 => "pseudo-fs mount: /proc",
                22 => "pseudo-fs mount: /dev",
                23 => "pseudo-fs mount: /sys (bind)",
                24 => "pseudo-fs mount: /sys (remount ro)",
                25 => "pseudo-fs mount: /tmp",
                _ => "unknown",
            };
            return Err(OwError::Other(format!(
                "container setup failed at step {} ({}): {}",
                step, desc, std::io::Error::from_raw_os_error(errno)
            )));
        }

        let stdout_task = spawn_log_reader(stdout_read, log_store.clone(), ow_core_runtime::log_store::LogStream::Stdout);
        let stderr_task = spawn_log_reader(stderr_read, log_store.clone(), ow_core_runtime::log_store::LogStream::Stderr);

        Ok(process::to_handle(LinuxProcess::new(
            pid,
            log_store,
            vec![stdout_task, stderr_task],
        )))
    }

    async fn stop_process(
        &self,
        handle: &ProcessHandle,
        timeout: Duration,
    ) -> Result<ExitStatus> {
        let proc = handle.downcast_ref::<LinuxProcess>()
            .ok_or_else(|| OwError::Other("invalid process handle".into()))?;

        let pid = proc.pid;

        // Check if already exited before sending signals
        let mut status: libc::c_int = 0;
        let ret = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        let exit_status = if ret > 0 {
            status
        } else if ret < 0 {
            // ECHILD or other error — process already reaped or not our child
            proc.drain_logs().await;
            return Ok(ExitStatus::exited(-1));
        } else {
            // ret == 0: shim still alive. Give fast workloads (e.g. `cat`) a brief
            // grace period to exit naturally before sending SIGTERM.
            let grace_deadline = tokio::time::Instant::now() + Duration::from_millis(500);
            let mut s: libc::c_int = 0;
            let natural_exit = loop {
                let r = unsafe { libc::waitpid(pid, &mut s, libc::WNOHANG) };
                if r > 0 { break Some(s); }
                if r < 0 {
                    proc.drain_logs().await;
                    return Ok(ExitStatus::exited(-1));
                }
                if tokio::time::Instant::now() >= grace_deadline { break None; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            if let Some(natural_status) = natural_exit {
                proc.drain_logs().await;
                return Ok(if libc::WIFEXITED(natural_status) {
                    ExitStatus::exited(libc::WEXITSTATUS(natural_status))
                } else if libc::WIFSIGNALED(natural_status) {
                    ExitStatus::signaled(libc::WTERMSIG(natural_status))
                } else {
                    ExitStatus::exited(-1)
                });
            }
            // Still running after grace period — kill entire process group
            unsafe { libc::kill(-pid, libc::SIGTERM); }
            let deadline = tokio::time::Instant::now() + timeout;
            loop {
                let mut s: libc::c_int = 0;
                let r = unsafe { libc::waitpid(pid, &mut s, libc::WNOHANG) };
                if r > 0 { break s; }
                if r < 0 {
                    // ECHILD — process already reaped, can't determine real status
                    proc.drain_logs().await;
                    return Ok(ExitStatus::exited(-1));
                }
                if tokio::time::Instant::now() >= deadline {
                    unsafe { libc::kill(-pid, libc::SIGKILL); }
                    unsafe { libc::waitpid(pid, &mut s, 0); }
                    break s;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };

        proc.drain_logs().await;

        if libc::WIFEXITED(exit_status) {
            Ok(ExitStatus::exited(libc::WEXITSTATUS(exit_status)))
        } else if libc::WIFSIGNALED(exit_status) {
            Ok(ExitStatus::signaled(libc::WTERMSIG(exit_status)))
        } else {
            Ok(ExitStatus::exited(-1))
        }
    }

    async fn teardown_network(&self, _handle: &NetworkHandle) -> Result<()> {
        Ok(())
    }

    async fn teardown_filesystem(&self, handle: &FilesystemHandle) -> Result<()> {
        let fs = handle.downcast_ref::<LinuxFilesystem>()
            .ok_or_else(|| OwError::Other("invalid filesystem handle".into()))?;
        // M2: unmount overlay first (if any), then clean directories
        self.teardown_overlay(&fs.container_id)?;
        filesystem::teardown_filesystem(fs).map_err(OwError::Io)
    }

    async fn destroy_isolation(&self, handle: &IsolationHandle) -> Result<()> {
        let iso = handle.downcast_ref::<LinuxIsolation>()
            .ok_or_else(|| OwError::Other("invalid isolation handle".into()))?;
        isolation::destroy_isolation(iso).map_err(OwError::Io)
    }
}
