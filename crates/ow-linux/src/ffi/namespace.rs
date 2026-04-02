use std::io;
use libc::{c_int, pid_t};

#[repr(C)]
pub struct OwNsConfig {
    pub clone_flags: c_int,
    pub userns_fd: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OwNsResult {
    pub init_pid: pid_t,
    pub ns_fds: [c_int; 7],
}

impl OwNsResult {
    pub fn zeroed() -> Self {
        Self {
            init_pid: 0,
            ns_fds: [-1; 7],
        }
    }
}

extern "C" {
    pub fn ow_ns_create(config: *const OwNsConfig, result: *mut OwNsResult) -> c_int;
    pub fn ow_ns_enter(ns: *const OwNsResult, ns_type: c_int) -> c_int;
    pub fn ow_ns_destroy(ns: *const OwNsResult) -> c_int;
}

pub fn create_namespaces(clone_flags: i32) -> io::Result<OwNsResult> {
    let config = OwNsConfig {
        clone_flags,
        userns_fd: -1,
    };
    let mut result = OwNsResult::zeroed();

    let ret = unsafe { ow_ns_create(&config, &mut result) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(result)
}

pub fn enter_namespace(ns: &OwNsResult, ns_type: i32) -> io::Result<()> {
    let ret = unsafe { ow_ns_enter(ns, ns_type) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn destroy_namespaces(ns: &OwNsResult) -> io::Result<()> {
    let ret = unsafe { ow_ns_destroy(ns) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
