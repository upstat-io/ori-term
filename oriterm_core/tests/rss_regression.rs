//! RSS (resident set size) regression tests for terminal core.
//!
//! Exercises `Term` with sustained output and measures actual process RSS
//! to verify memory plateaus. Placed in a separate integration test binary
//! so process memory is isolated from other test suites.
//!
//! Linux-only: reads `/proc/self/statm` for RSS measurement. Other
//! platforms skip these tests at compile time.

#![cfg(target_os = "linux")]

use oriterm_core::{Term, Theme, VoidListener};

/// Read the current process RSS in bytes from `/proc/self/statm`.
///
/// Field layout: `size resident shared text lib data dt` (pages).
/// Page size is 4096 on x86_64 Linux (the only Linux target we build for).
fn rss_bytes() -> usize {
    let statm =
        std::fs::read_to_string("/proc/self/statm").expect("failed to read /proc/self/statm");
    let resident_pages: usize = statm
        .split_whitespace()
        .nth(1)
        .expect("missing resident field")
        .parse()
        .expect("non-numeric resident field");
    resident_pages * 4096
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
