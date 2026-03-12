//! Allocation regression tests for hot-path zero-alloc invariants.
//!
//! Uses a counting global allocator to verify that `renderable_content_into()`
//! performs zero heap allocations after warmup. Placed in an integration test
//! file (separate binary) to isolate the `#[global_allocator]`.
//!
//! **Thread safety**: The counting allocator is process-wide. When tests run
//! in parallel, other test threads' allocations contribute noise (~5-20 per
//! measurement window). The thresholds account for this — real regressions
//! (e.g., per-cell allocation) would add ~1920 allocs per call on a 24x80
//! grid, far exceeding the tolerance. For exact-zero verification, run with
//! `--test-threads=1`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use oriterm_core::{Term, Theme, VoidListener};

// --- Counting allocator with enable/disable gate ---

/// When false, allocations are not counted. This narrows the measurement
/// window to only the code under test, reducing (but not eliminating)
/// noise from parallel test threads.
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

struct CountingAlloc;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            BYTES_ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[allow(unsafe_code)]
#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Measure allocations during `f()`. Enables counting, runs the closure,
/// disables counting, and returns the allocation count.
fn measure_allocs<F: FnOnce()>(f: F) -> u64 {
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    BYTES_ALLOCATED.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    f();
    COUNTING.store(false, Ordering::SeqCst);
    ALLOC_COUNT.load(Ordering::SeqCst)
}

fn make_term() -> Term<VoidListener> {
    Term::new(24, 80, 1000, Theme::default(), VoidListener)
}

/// Threshold for "zero-alloc" assertion. Accounts for noise from parallel
/// test threads (~5-20 allocs per measurement window). A real regression
/// adding per-cell allocation would produce ~1920 allocs on a 24x80 grid.
const ZERO_ALLOC_THRESHOLD: u64 = 50;

// --- Tests ---

/// After the first call to `renderable_content_into()`, subsequent calls on
/// the same `RenderableContent` buffer must perform zero heap allocations
/// (plain ASCII content, no images, no combining marks).
#[test]
fn snapshot_extraction_zero_alloc_steady_state() {
    let term = make_term();
    let mut out = term.renderable_content();

    // Warmup: first `_into` call establishes Vec capacities.
    term.renderable_content_into(&mut out);

    // Measure: second call should allocate nothing (threshold for thread noise).
    let allocs = measure_allocs(|| {
        term.renderable_content_into(&mut out);
    });

    assert!(
        allocs < ZERO_ALLOC_THRESHOLD,
        "renderable_content_into() allocated {allocs} times on steady-state call \
         (expected < {ZERO_ALLOC_THRESHOLD}, a real regression would be ~1920+)"
    );
}

/// After warmup, rendering 100 consecutive frames into the same buffer must
/// produce near-zero total allocations. This catches regressions that only
/// appear under repeated use (e.g., HashSet rehash, Vec realloc).
#[test]
fn hundred_frames_zero_alloc_after_warmup() {
    let mut term = make_term();
    let mut proc: vte::ansi::Processor = vte::ansi::Processor::new();

    // Write some content so the grid isn't empty.
    proc.advance(&mut term, b"Hello, terminal!\r\n");
    proc.advance(&mut term, b"Line two.\r\n");

    let mut out = term.renderable_content();

    // Warmup: two calls to ensure stable capacity.
    term.renderable_content_into(&mut out);
    term.renderable_content_into(&mut out);

    // Measure 100 frames.
    let allocs = measure_allocs(|| {
        for _ in 0..100 {
            term.renderable_content_into(&mut out);
        }
    });

    // 100 frames × threshold per frame. A real regression would be 100 × 1920 = 192,000.
    let threshold = ZERO_ALLOC_THRESHOLD * 100;
    assert!(
        allocs < threshold,
        "100 frames produced {allocs} allocations \
         (expected < {threshold}, a real regression would be ~192,000+)"
    );
}

/// Feed 100,000 lines through the terminal (bounded scrollback) and verify
/// total allocations are bounded. The scrollback buffer has a fixed capacity
/// (1000 rows), so old rows are recycled. Total allocations should be modest
/// — well under 50 MB — proving no quadratic blowup or unbounded growth.
#[test]
fn rss_stability_under_sustained_output() {
    let mut term = make_term();
    let mut proc: vte::ansi::Processor = vte::ansi::Processor::new();
    let line = "A".repeat(79) + "\r\n";

    // Warmup: fill scrollback to capacity to stabilize allocations.
    for _ in 0..2000 {
        proc.advance(&mut term, line.as_bytes());
    }

    // Snapshot once to warm up the RenderableContent.
    let mut out = term.renderable_content();
    term.renderable_content_into(&mut out);

    // Measure: feed another 100,000 lines and track total bytes.
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    BYTES_ALLOCATED.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    for _ in 0..100_000 {
        proc.advance(&mut term, line.as_bytes());
    }
    term.renderable_content_into(&mut out);
    COUNTING.store(false, Ordering::SeqCst);
    let allocated = BYTES_ALLOCATED.load(Ordering::SeqCst);

    // Total allocations should be bounded. The scrollback is capped at 1000
    // rows, so old rows are recycled via `Row::reset()`. VTE parsing of plain
    // text produces near-zero allocations (no OSC buffers, no title strings).
    // Under 50 MB total for 100k lines proves no quadratic blowup.
    assert!(
        allocated < 50_000_000,
        "100k lines caused {allocated} bytes of allocations (expected < 50 MB)"
    );
}
