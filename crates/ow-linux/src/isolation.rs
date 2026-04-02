use std::path::PathBuf;
use std::sync::Once;
use crate::ffi::namespace::{self, OwNsResult};
use crate::ffi::cgroup;
use std::io;

pub struct LinuxIsolation {
    pub ns_result: OwNsResult,
    pub cgroup_path: PathBuf,
    pub container_id: String,
}

static CGROUP_INIT: Once = Once::new();

fn ensure_cgroup_parent() -> io::Result<()> {
    let mut init_err: Option<io::Error> = None;
    CGROUP_INIT.call_once(|| {
        let parent = "/sys/fs/cgroup/overlayward";
        if let Err(e) = std::fs::create_dir_all(parent) {
            init_err = Some(e);
            return;
        }
        let _ = std::fs::write(
            format!("{}/cgroup.subtree_control", parent),
            "+memory +cpu",
        );
    });
    match init_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub fn create_isolation(container_id: &str, clone_flags: i32, memory_max: u64) -> io::Result<LinuxIsolation> {
    let ns_result = namespace::create_namespaces(clone_flags)?;

    ensure_cgroup_parent()?;
    let cgroup_path = format!("/sys/fs/cgroup/overlayward/{}", container_id);
    if let Err(e) = cgroup::create_cgroup(&cgroup_path, memory_max, 0, 0) {
        let _ = namespace::destroy_namespaces(&ns_result);
        return Err(e);
    }

    if let Err(e) = cgroup::attach_cgroup(&cgroup_path, ns_result.init_pid) {
        let _ = cgroup::destroy_cgroup(&cgroup_path);
        let _ = namespace::destroy_namespaces(&ns_result);
        return Err(e);
    }

    Ok(LinuxIsolation {
        ns_result,
        cgroup_path: PathBuf::from(cgroup_path),
        container_id: container_id.to_string(),
    })
}

pub fn destroy_isolation(iso: &LinuxIsolation) -> io::Result<()> {
    let r1 = namespace::destroy_namespaces(&iso.ns_result);
    // Cgroup rmdir may fail with EBUSY if processes haven't fully exited.
    // Retry a few times with brief sleep.
    let cgroup_str = iso.cgroup_path.to_str().unwrap_or("");
    let mut r2 = Ok(());
    for i in 0..10 {
        match cgroup::destroy_cgroup(cgroup_str) {
            Ok(()) => { r2 = Ok(()); break; }
            Err(e) if e.raw_os_error() == Some(libc::EBUSY) && i < 9 => {
                // Debug: check what's still in the cgroup
                if i == 0 {
                    if let Ok(procs) = std::fs::read_to_string(
                        iso.cgroup_path.join("cgroup.procs")
                    ) {
                        eprintln!("cgroup still has procs: {:?}", procs.trim());
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => { r2 = Err(e); break; }
        }
    }
    r1.and(r2)
}

pub fn to_handle(iso: LinuxIsolation) -> ow_core_traits::IsolationHandle {
    ow_core_traits::IsolationHandle::new(iso)
}
