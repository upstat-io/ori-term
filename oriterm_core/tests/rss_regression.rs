//! RSS (resident set size) regression tests for terminal core.
//!
//! Exercises `Term` with sustained output and measures actual process RSS
//! to verify memory plateaus. Placed in a separate integration test binary
//! so process memory is isolated from other test suites.
//!
//! Cross-platform: Linux reads `/proc/self/statm`, macOS uses `task_info`,
//! Windows uses `GetProcessMemoryInfo`. Other platforms skip at compile time.

#![cfg(any(target_os = "linux", target_os = "macos", windows))]

use oriterm_core::{Term, Theme, VoidListener};

/// Read the current process RSS in bytes.
///
/// Platform-specific: Linux reads `/proc/self/statm`, macOS uses Mach
/// `task_info`, Windows uses `GetProcessMemoryInfo`.
fn rss_bytes() -> usize {
    platform_rss().expect("failed to read process RSS")
}

#[cfg(target_os = "linux")]
fn platform_rss() -> Option<usize> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * 4096)
}

#[cfg(target_os = "macos")]
fn platform_rss() -> Option<usize> {
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

    #[allow(unsafe_code)]
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        if ok != 0 {
            Some(counters.WorkingSetSize)
        } else {
            None
        }
    }
}

fn make_term(scrollback: usize) -> Term<VoidListener> {
    Term::new(24, 80, scrollback, Theme::default(), VoidListener)
}

const MB: usize = 1_048_576;

/// RSS must plateau under sustained output.
///
/// After filling scrollback to capacity, feeding another 100k lines must not
/// cause RSS to grow significantly. The scrollback ring buffer recycles rows,
/// so memory should be stable once the buffer is full.
#[test]
fn rss_plateaus_under_sustained_output() {
    let mut term = make_term(1000);
    let mut parser: vte::ansi::Processor = vte::ansi::Processor::new();
    let line = "A".repeat(79) + "\r\n";

    // Phase 1: Fill scrollback to capacity (warmup).
    for _ in 0..2000 {
        parser.advance(&mut term, line.as_bytes());
    }

    // Snapshot to warm up RenderableContent buffers.
    let mut out = term.renderable_content();
    term.renderable_content_into(&mut out);

    let rss_after_warmup = rss_bytes();

    // Phase 2: Sustained output — 100k more lines.
    for _ in 0..100_000 {
        parser.advance(&mut term, line.as_bytes());
    }
    term.renderable_content_into(&mut out);

    let rss_after_sustained = rss_bytes();

    // RSS growth should be minimal — under 2 MB. The scrollback is bounded
    // (1000 rows), so old rows are recycled. Any significant growth indicates
    // a leak (unbounded buffer, stale cache, etc.).
    let growth = rss_after_sustained.saturating_sub(rss_after_warmup);
    assert!(
        growth < 2 * MB,
        "RSS grew {:.1} MB after 100k lines (warmup: {:.1} MB, after: {:.1} MB). \
         Expected < 2 MB growth with bounded scrollback.",
        growth as f64 / MB as f64,
        rss_after_warmup as f64 / MB as f64,
        rss_after_sustained as f64 / MB as f64,
    );
}

/// RSS for an empty terminal (core only, no GPU) must be under 10 MB.
///
/// This covers the Grid, scrollback capacity allocation, VTE parser, and
/// RenderableContent scratch buffers. The full app (with GPU textures, font
/// caches, etc.) will be higher — those targets are validated via
/// `--profile` mode at runtime.
#[test]
fn rss_bounded_empty_terminal() {
    let term = make_term(1000);
    let mut out = term.renderable_content();
    term.renderable_content_into(&mut out);

    let rss = rss_bytes();

    // Core-only RSS should be well under 10 MB. The grid allocates 1024 rows
    // × ~1920 bytes = ~1.9 MB, plus VTE state and test harness overhead.
    assert!(
        rss < 10 * MB,
        "Empty terminal RSS is {:.1} MB (expected < 10 MB)",
        rss as f64 / MB as f64,
    );
}

/// RSS must not grow monotonically across measurement intervals.
///
/// Simulates the `--profile` measurement protocol: measure at intervals
/// while feeding output, verify the series eventually plateaus.
#[test]
fn rss_series_plateaus() {
    let mut term = make_term(5000);
    let mut parser: vte::ansi::Processor = vte::ansi::Processor::new();
    let line = "B".repeat(79) + "\r\n";
    let mut out = term.renderable_content();

    let mut measurements = Vec::new();

    // Take 6 measurements: after 0, 10k, 20k, 30k, 40k, 50k lines.
    for i in 0..6 {
        if i > 0 {
            for _ in 0..10_000 {
                parser.advance(&mut term, line.as_bytes());
            }
            term.renderable_content_into(&mut out);
        }
        measurements.push(rss_bytes());
    }

    // After the first 2 measurements (warmup), the remaining 4 should not
    // show monotonic growth. At least one measurement must be <= its predecessor.
    let post_warmup = &measurements[2..];
    let all_increasing = post_warmup.windows(2).all(|w| w[1] > w[0] + 256 * 1024); // 256 KB tolerance for OS noise

    assert!(
        !all_increasing,
        "RSS grew monotonically after warmup: {:?} (in MB: {:?}). \
         Expected plateau with bounded scrollback.",
        measurements,
        measurements
            .iter()
            .map(|&b| format!("{:.1}", b as f64 / MB as f64))
            .collect::<Vec<_>>(),
    );
}
