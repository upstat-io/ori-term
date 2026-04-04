//! Platform-specific resident set size (RSS) measurement.
//!
//! Returns the process's RSS in bytes. Used by [`PerfStats`](crate::app)
//! for periodic memory watermark logging in `--profile` mode.

/// Read the current process's resident set size in bytes.
///
/// Returns `None` if the platform API fails or is unavailable.
pub fn rss_bytes() -> Option<usize> {
    platform_rss()
}

/// Format bytes as a human-readable string (e.g. "42.3 MB").
pub fn format_bytes(bytes: usize) -> String {
    const MB: f64 = 1_048_576.0;
    format!("{:.1} MB", bytes as f64 / MB)
}

#[cfg(target_os = "linux")]
fn platform_rss() -> Option<usize> {
    // /proc/self/statm fields: size resident shared text lib data dt (in pages).
    // Page size is 4096 on x86_64 Linux (the only Linux target we build for).
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * 4096)
}

#[cfg(target_os = "macos")]
fn platform_rss() -> Option<usize> {
    // SAFETY: FFI calls to Mach kernel APIs. `mach_task_self_` is the current
    // task port (extern static). `task_info()` fills a zeroed struct — no
    // aliasing or lifetime issues. All types come from `libc` with correct ABI.
    #[allow(unsafe_code, deprecated)]
    unsafe {
        let mut info: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
        let kr = libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            std::ptr::addr_of_mut!(info).cast(),
            &mut count,
        );
        if kr == libc::KERN_SUCCESS {
            Some(info.resident_size as usize)
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn platform_rss() -> Option<usize> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: FFI calls to Win32 APIs. `GetCurrentProcess()` returns a
    // pseudo-handle (no cleanup needed). `GetProcessMemoryInfo` fills a
    // zeroed struct with `cb` set to the correct size.
    #[allow(unsafe_code)]
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(
            GetCurrentProcess(),
            &raw mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        if ok != 0 {
            Some(counters.WorkingSetSize)
        } else {
            None
        }
    }
}

// Fallback for other platforms (FreeBSD, etc.).
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_rss() -> Option<usize> {
    None
}
