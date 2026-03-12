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

// macOS: would require `libc` crate for `mach_task_basic_info`. Returns None
// until we add the dependency.
#[cfg(target_os = "macos")]
fn platform_rss() -> Option<usize> {
    None
}

// Windows: would require `Win32_System_ProcessStatus` feature on `windows-sys`.
// Returns None until we add the feature.
#[cfg(windows)]
fn platform_rss() -> Option<usize> {
    None
}
