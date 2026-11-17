//! Windows job ownership closes every assigned process, including pipe-holding descendants.
#[cfg(windows)]
mod windows {
    use anyhow::{Context, Result};
    use std::{ffi::c_void, os::windows::io::AsRawHandle, process::Child};
    type Handle = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimits {
        process_time: i64,
        job_time: i64,
        flags: u32,
        min_working_set: usize,
        max_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_operations: u64,
        write_operations: u64,
        other_operations: u64,
        read_bytes: u64,
        write_bytes: u64,
        other_bytes: u64,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimits {
        basic: BasicLimits,
        io: IoCounters,
        process_memory: usize,
        job_memory: usize,
        peak_process_memory: usize,
        peak_job_memory: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            class: u32,
            information: *const c_void,
            size: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    pub struct ProcessTree(Handle);
    impl ProcessTree {
        pub fn assign(child: &Child) -> Result<Self> {
            // SAFETY: null optional attributes/name; returned handle is owned below.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let owner = Self(handle);
            let limits = ExtendedLimits {
                basic: BasicLimits {
                    flags: 0x2000,
                    ..BasicLimits::default()
                },
                ..ExtendedLimits::default()
            };
            // SAFETY: structure layout matches JOBOBJECT_EXTENDED_LIMIT_INFORMATION (class 9).
            if unsafe {
                SetInformationJobObject(
                    handle,
                    9,
                    &limits as *const _ as *const c_void,
                    std::mem::size_of::<ExtendedLimits>() as u32,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error())
                    .context("Could not configure inference process-tree cleanup");
            }
            // SAFETY: both live handles are valid and remain owned by their Rust objects.
            if unsafe { AssignProcessToJobObject(handle, child.as_raw_handle()) } == 0 {
                return Err(std::io::Error::last_os_error())
                    .context("Could not own the inference process tree");
            }
            Ok(owner)
        }
    }
    impl Drop for ProcessTree {
        fn drop(&mut self) {
            // SAFETY: this object uniquely owns the job handle; close kills assigned processes.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}
#[cfg(windows)]
pub use windows::ProcessTree;
