use std::ffi::CString;
use std::io;
use libc::c_int;

extern "C" {
    pub fn ow_pivot_root(new_root: *const libc::c_char, put_old: *const libc::c_char) -> c_int;
    pub fn ow_bind_mount(src: *const libc::c_char, dst: *const libc::c_char, readonly: c_int) -> c_int;
    pub fn ow_tmpfs_mount(dst: *const libc::c_char, size: u64) -> c_int;
}

pub fn pivot_root(new_root: &str, put_old: &str) -> io::Result<()> {
    let c_new = CString::new(new_root).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let c_old = CString::new(put_old).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { ow_pivot_root(c_new.as_ptr(), c_old.as_ptr()) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}

pub fn bind_mount(src: &str, dst: &str, readonly: bool) -> io::Result<()> {
    let c_src = CString::new(src).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let c_dst = CString::new(dst).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { ow_bind_mount(c_src.as_ptr(), c_dst.as_ptr(), readonly as c_int) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}

pub fn tmpfs_mount(dst: &str, size: u64) -> io::Result<()> {
    let c_dst = CString::new(dst).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { ow_tmpfs_mount(c_dst.as_ptr(), size) };
    if ret != 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}
