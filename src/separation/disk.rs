//! One-time destination filesystem admission; no reservation or monitoring.
use anyhow::{Context, Result, ensure};
use std::path::Path;
pub fn estimate(frames: u64, source_rate: u32, output_rate: u32) -> Result<u64> {
    let bytes = |rate| -> Result<u64> {
        crate::stems::coordinate_frames(frames, rate, source_rate)?
            .checked_mul(8)
            .context("Audio size overflow")
    };
    bytes(44100)?
        .checked_mul(8)
        .and_then(|v| v.checked_add(bytes(output_rate).ok()?.checked_mul(8)?))
        .and_then(|v| v.checked_add(frames.checked_mul(8)?))
        .and_then(|v| v.checked_add(64 * 1024 * 1024))
        .context("Processing space estimate overflow")
}
pub fn admit(path: &Path, needed: u64) -> Result<()> {
    let available = available(path)?;
    ensure!(
        available >= needed,
        "Insufficient free space: approximately {:.2} GB needed, {:.2} GB available at {}",
        needed as f64 / 1e9,
        available as f64 / 1e9,
        path.display()
    );
    Ok(())
}
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // statvfs integer widths differ across Unix targets.
pub fn available(path: &Path) -> Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut result = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: null-terminated path and valid writable statvfs storage.
    let status = unsafe { libc::statvfs(path.as_ptr(), result.as_mut_ptr()) };
    if status != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful statvfs initialized the structure.
    let value = unsafe { result.assume_init() };
    (value.f_bavail as u64)
        .checked_mul(value.f_frsize as u64)
        .context("Free space overflow")
}
#[cfg(windows)]
pub fn available(path: &Path) -> Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            available: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
    }
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut bytes = 0;
    // SAFETY: path is terminated and available points to valid output storage.
    if unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_estimate_and_filesystem() {
        assert_eq!(estimate(14_400_000, 48000, 48000).unwrap(), 1_950_628_864);
        assert!(estimate(u64::MAX, 8000, 192000).is_err());
        assert!(available(&std::env::temp_dir()).unwrap() > 0);
    }
}
