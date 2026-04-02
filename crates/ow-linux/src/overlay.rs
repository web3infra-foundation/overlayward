//! Linux kernel overlayfs mount/unmount wrappers.
//!
//! DS-1 uses kernel native overlayfs (`mount -t overlay`).
//! DS-2 will migrate to rfuse3 user-space implementation.

use std::path::Path;
use nix::mount::{mount, umount2, MntFlags, MsFlags};
use tracing::{info, debug};

/// Mount an overlayfs.
///
/// `lower_dirs` order: bottom to top (first is the lowest layer).
/// overlayfs requires lowerdir from top to bottom, so this function reverses.
pub fn overlay_mount(
    lower_dirs: &[impl AsRef<Path>],
    upper_dir: &Path,
    work_dir: &Path,
    mountpoint: &Path,
) -> std::io::Result<()> {
    let lowerdir: String = lower_dirs
        .iter()
        .rev()
        .map(|p| p.as_ref().to_str().unwrap())
        .collect::<Vec<_>>()
        .join(":");

    let options = format!(
        "lowerdir={},upperdir={},workdir={}",
        lowerdir,
        upper_dir.display(),
        work_dir.display(),
    );

    debug!(
        mountpoint = %mountpoint.display(),
        layers = lower_dirs.len(),
        "mounting overlayfs"
    );

    mount(
        Some("overlay"),
        mountpoint,
        Some("overlay"),
        MsFlags::empty(),
        Some(options.as_str()),
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    info!(mountpoint = %mountpoint.display(), "overlayfs mounted");
    Ok(())
}

/// Unmount an overlayfs.
pub fn overlay_unmount(mountpoint: &Path) -> std::io::Result<()> {
    debug!(mountpoint = %mountpoint.display(), "unmounting overlayfs");

    umount2(mountpoint, MntFlags::MNT_DETACH)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    info!(mountpoint = %mountpoint.display(), "overlayfs unmounted");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn lowerdir_order_is_reversed() {
        let layers = vec!["/bottom", "/middle", "/top"];
        let lowerdir: String = layers
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join(":");
        assert_eq!(lowerdir, "/top:/middle:/bottom");
    }
}
