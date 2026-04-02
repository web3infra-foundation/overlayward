use std::path::{Path, PathBuf};
use std::io;

pub struct LinuxFilesystem {
    pub rootfs_source: PathBuf,
    pub rootfs_mount_point: PathBuf,
    pub container_id: String,
}

/// M1 compatible: setup filesystem from a local directory path.
pub fn setup_filesystem(container_id: &str, rootfs_source: &str) -> io::Result<LinuxFilesystem> {
    let mount_point = format!("/tmp/overlayward/{}/rootfs", container_id);
    std::fs::create_dir_all(&mount_point)?;

    Ok(LinuxFilesystem {
        rootfs_source: PathBuf::from(rootfs_source),
        rootfs_mount_point: PathBuf::from(mount_point),
        container_id: container_id.to_string(),
    })
}

/// M2: setup overlayfs rootfs from OCI image layers.
/// Executes overlayfs mount in the parent process.
pub fn setup_overlayfs_rootfs(
    container_id: &str,
    layer_paths: &[PathBuf],
    data_root: &Path,
) -> io::Result<LinuxFilesystem> {
    let container_dir = data_root.join("containers").join(container_id);
    let rootfs = container_dir.join("rootfs");
    let upper = container_dir.join("upper");
    let work = container_dir.join("work");

    for dir in [&rootfs, &upper, &work] {
        std::fs::create_dir_all(dir)?;
    }

    // overlayfs mount (parent process)
    crate::overlay::overlay_mount(layer_paths, &upper, &work, &rootfs)?;

    Ok(LinuxFilesystem {
        rootfs_source: rootfs.clone(),
        rootfs_mount_point: rootfs,
        container_id: container_id.to_string(),
    })
}

pub fn teardown_filesystem(fs: &LinuxFilesystem) -> io::Result<()> {
    match std::fs::remove_dir_all(&fs.rootfs_mount_point) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn to_handle(fs: LinuxFilesystem) -> ow_core_traits::FilesystemHandle {
    ow_core_traits::FilesystemHandle::new(fs)
}

/// Pre-allocated pseudo-filesystem mount parameters for fork-safe child execution.
/// Constructed before fork, used in child1 with only async-signal-safe libc calls.
pub struct PseudoFsParams {
    mounts: Vec<PseudoMount>,
}

struct PseudoMount {
    target: std::ffi::CString,
    source: std::ffi::CString,
    fstype: *const libc::c_char,
    flags: libc::c_ulong,
    data: *const libc::c_char,
}

unsafe impl Send for PseudoMount {}

impl PseudoFsParams {
    /// Construct before fork. Pre-allocates all paths and parameters.
    pub fn new(rootfs: &Path) -> Self {
        let rootfs_str = rootfs.to_str().expect("rootfs path must be valid UTF-8");
        let mounts = vec![
            // /proc — procfs
            PseudoMount {
                target: std::ffi::CString::new(format!("{rootfs_str}/proc")).unwrap(),
                source: std::ffi::CString::new("proc").unwrap(),
                fstype: b"proc\0".as_ptr() as *const libc::c_char,
                flags: 0,
                data: std::ptr::null(),
            },
            // /dev — bind mount from host
            PseudoMount {
                target: std::ffi::CString::new(format!("{rootfs_str}/dev")).unwrap(),
                source: std::ffi::CString::new("/dev").unwrap(),
                fstype: std::ptr::null(),
                flags: (libc::MS_BIND | libc::MS_REC) as libc::c_ulong,
                data: std::ptr::null(),
            },
            // /sys — step 1: bind mount from host (MS_RDONLY is silently ignored on initial bind)
            PseudoMount {
                target: std::ffi::CString::new(format!("{rootfs_str}/sys")).unwrap(),
                source: std::ffi::CString::new("/sys").unwrap(),
                fstype: std::ptr::null(),
                flags: (libc::MS_BIND | libc::MS_REC) as libc::c_ulong,
                data: std::ptr::null(),
            },
            // /sys — step 2: remount read-only (required by Linux bind-mount semantics)
            PseudoMount {
                target: std::ffi::CString::new(format!("{rootfs_str}/sys")).unwrap(),
                source: std::ffi::CString::new("/sys").unwrap(),
                fstype: std::ptr::null(),
                flags: (libc::MS_BIND | libc::MS_REC | libc::MS_REMOUNT | libc::MS_RDONLY) as libc::c_ulong,
                data: std::ptr::null(),
            },
            // /tmp — tmpfs
            PseudoMount {
                target: std::ffi::CString::new(format!("{rootfs_str}/tmp")).unwrap(),
                source: std::ffi::CString::new("tmpfs").unwrap(),
                fstype: b"tmpfs\0".as_ptr() as *const libc::c_char,
                flags: 0,
                data: b"size=64m\0".as_ptr() as *const libc::c_char,
            },
        ];
        Self { mounts }
    }
}

/// Mount pseudo-filesystems in child1 (after bind-mount rootfs, before pivot_root).
/// Only uses pre-allocated data + async-signal-safe libc calls.
///
/// # Safety
/// Must be called after fork() in the child process. params must be constructed before fork.
pub unsafe fn setup_pseudo_filesystems_raw(params: &PseudoFsParams) -> i32 {
    for (i, m) in params.mounts.iter().enumerate() {
        libc::mkdir(m.target.as_ptr(), 0o755); // ignore EEXIST
        if libc::mount(
            m.source.as_ptr(),
            m.target.as_ptr(),
            m.fstype,
            m.flags,
            m.data as *const libc::c_void,
        ) != 0 {
            return (i as i32) + 1;
        }
    }
    0
}
