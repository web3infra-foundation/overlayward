use std::ffi::CString;
use std::io;
use libc::{c_int, pid_t};

#[repr(C)]
pub struct OwCgroupConfig {
    pub path: *const libc::c_char,
    pub cpu_quota_us: u64,
    pub cpu_period_us: u64,
    pub memory_max: u64,
    pub memory_swap_max: u64,
}

extern "C" {
    pub fn ow_cgroup_create(config: *const OwCgroupConfig) -> c_int;
    pub fn ow_cgroup_attach(path: *const libc::c_char, pid: pid_t) -> c_int;
    pub fn ow_cgroup_destroy(path: *const libc::c_char) -> c_int;
}

pub fn create_cgroup(path: &str, memory_max: u64, cpu_quota: u64, cpu_period: u64) -> io::Result<()> {
    let c_path = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let config = OwCgroupConfig {
        path: c_path.as_ptr(),
        cpu_quota_us: cpu_quota,
        cpu_period_us: cpu_period,
        memory_max,
        memory_swap_max: 0,
    };
    let ret = unsafe { ow_cgroup_create(&config) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}

pub fn attach_cgroup(path: &str, pid: i32) -> io::Result<()> {
    let c_path = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { ow_cgroup_attach(c_path.as_ptr(), pid) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}

pub fn destroy_cgroup(path: &str) -> io::Result<()> {
    let c_path = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { ow_cgroup_destroy(c_path.as_ptr()) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}
