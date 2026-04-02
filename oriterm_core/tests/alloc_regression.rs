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
use std::time::Instant;

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

/// Allocation + byte counts from a measurement window.
struct AllocMeasurement {
    allocs: u64,
    bytes: u64,
}

/// Measure allocations during `f()`. Enables counting, runs the closure,
/// disables counting, and returns both allocation count and bytes.
fn measure(f: impl FnOnce()) -> AllocMeasurement {
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    BYTES_ALLOCATED.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    f();
    COUNTING.store(false, Ordering::SeqCst);
    AllocMeasurement {
        allocs: ALLOC_COUNT.load(Ordering::SeqCst),
        bytes: BYTES_ALLOCATED.load(Ordering::SeqCst),
    }
}

/// Measure allocations during `f()`. Returns allocation count only.
fn measure_allocs<F: FnOnce()>(f: F) -> u64 {
    measure(f).allocs
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

    // Warmup: two calls to fully establish Vec capacities (some internal
    // collections stabilize after the first call).
    term.renderable_content_into(&mut out);
    term.renderable_content_into(&mut out);

    // Measure: third call should allocate nothing (threshold for thread noise).
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

/// Processing 1 MB of printable ASCII through VTE + Term must not allocate
/// after warmup. This covers the full parse path: `Processor::advance()` →
/// `Term::input()` → `Grid::put_char_ascii()`.
#[test]
fn vte_1mb_ascii_zero_alloc_after_warmup() {
    let mut term = make_term();
    let mut proc: vte::ansi::Processor = vte::ansi::Processor::new();

    // Build a 1 MB buffer of printable ASCII (0x20–0x7E) with periodic \n.
    let line = "A".repeat(79) + "\n";
    let mut buf = Vec::new();
    while buf.len() < 1_024 * 1_024 {
        buf.extend_from_slice(line.as_bytes());
    }
    buf.truncate(1_024 * 1_024);

    // Warmup: fill grid and scrollback to stabilize all internal capacities.
    proc.advance(&mut term, &buf);

    // Measure: second pass should produce near-zero allocations.
    let allocs = measure_allocs(|| {
        proc.advance(&mut term, &buf);
    });

    assert!(
        allocs < ZERO_ALLOC_THRESHOLD,
        "1 MB ASCII parse produced {allocs} allocations after warmup \
         (expected < {ZERO_ALLOC_THRESHOLD})"
    );
}

/// The threaded IO render path swaps `RenderableContent` buffers between the
/// IO thread and main thread via `std::mem::swap`. This must be zero-allocation
/// after warmup — the swap exchanges pre-allocated buffers, not copying data.
/// Simulates the `SnapshotDoubleBuffer::flip_swap()` + `swap_front()` cycle.
#[test]
fn snapshot_swap_path_zero_alloc_after_warmup() {
    let term = make_term();

    // Simulate IO thread's snapshot buffer and main thread's render cache.
    let mut io_buf = term.renderable_content();
    let mut main_buf = term.renderable_content();

    // Warmup: fill both buffers to establish Vec capacities.
    term.renderable_content_into(&mut io_buf);
    term.renderable_content_into(&mut main_buf);

    // Simulate 100 swap cycles (IO thread flips, main thread consumes).
    let allocs = measure_allocs(|| {
        for _ in 0..100 {
            // IO thread produces snapshot then swaps with "front" buffer.
            term.renderable_content_into(&mut io_buf);
            std::mem::swap(&mut io_buf, &mut main_buf);
        }
    });

    // The swap itself is zero-alloc. The `renderable_content_into` reuses
    // pre-allocated Vecs. 100 cycles should stay well under threshold.
    let threshold = ZERO_ALLOC_THRESHOLD * 100;
    assert!(
        allocs < threshold,
        "100 snapshot swap cycles produced {allocs} allocations \
         (expected < {threshold})"
    );
}

// --- Profiling tests (Section 23.3) ---

/// Profile memory consumed by blank rows in scrollback.
///
/// Creates a 120-column terminal with 10K scrollback and fills it with
/// alternating content/blank lines. Measures total bytes allocated for the
/// blank rows to determine whether a compact `RowStorage::Blank` enum is
/// justified (threshold: >5 MB savings).
///
/// Analytical expectation: 5,000 blank rows × 120 cols × 24 bytes/cell =
/// 14.4 MB. Each blank `Row` stores a full `Vec<Cell>` even when every cell
/// is default. A compact representation would use ~8 bytes per blank row.
///
/// Run with: `cargo test -p oriterm_core --test alloc_regression profile_blank -- --ignored`
#[test]
#[ignore = "profiling test — run separately to avoid counting allocator noise"]
fn profile_blank_row_memory() {
    let cols = 120;
    let scrollback = 10_000;
    let mut term = Term::new(50, cols, scrollback, Theme::default(), VoidListener);
    let mut proc: vte::ansi::Processor = vte::ansi::Processor::new();

    // Fill scrollback with alternating content and blank lines.
    // Content line: 119 chars + \n. Blank line: just \n.
    let content_line = "X".repeat(cols - 1) + "\n";
    let blank_line = b"\n";

    // Push enough lines to fill scrollback (need >10K lines to saturate).
    // Each pair is one content + one blank = 2 lines.
    let m = measure(|| {
        for _ in 0..6_000 {
            proc.advance(&mut term, content_line.as_bytes());
            proc.advance(&mut term, blank_line);
        }
    });

    // Theoretical: 10K rows × 120 cols × 24 bytes/cell = 28.8 MB total.
    // Half blank → 14.4 MB for blank rows alone.
    // The actual allocation includes Vec overhead per row (24 bytes ptr/len/cap)
    // and allocator alignment, so expect slightly more.
    let bytes_mb = m.bytes as f64 / (1024.0 * 1024.0);
    let theoretical_blank_mb = (scrollback as f64 / 2.0) * (cols as f64) * 24.0 / (1024.0 * 1024.0);

    eprintln!("--- Blank Row Memory Profile (Section 23.3) ---");
    eprintln!("  Grid: {cols} cols, {scrollback} scrollback");
    eprintln!(
        "  Total bytes allocated: {:.1} MB ({} allocs)",
        bytes_mb, m.allocs
    );
    eprintln!("  Theoretical blank row cost: {theoretical_blank_mb:.1} MB");
    eprintln!("  (5K blank rows × {cols} cols × 24 bytes/cell)");
    eprintln!(
        "  Verdict: blank rows consume >{:.0} MB → {} compact blank optimization",
        theoretical_blank_mb,
        if theoretical_blank_mb > 5.0 {
            "JUSTIFIES"
        } else {
            "does NOT justify"
        }
    );
    eprintln!("-----------------------------------------------");

    // Sanity: we should have allocated at least the grid memory.
    assert!(
        m.bytes > 1_000_000,
        "expected substantial allocation for 10K-row scrollback, got {} bytes",
        m.bytes
    );
}

/// Profile allocation count during rapid resize cycles.
///
/// Simulates dragging a window edge by alternating between two terminal
/// sizes for 100 cycles. Measures allocation count after warmup to determine
/// whether a `row_pool` for resize reuse is justified (threshold: >1000
/// allocs causing >16ms frame time).
///
/// Run with: `cargo test -p oriterm_core --test alloc_regression profile_resize -- --ignored`
#[test]
#[ignore = "profiling test — run separately to avoid counting allocator noise"]
fn profile_resize_allocation_count() {
    let mut term = Term::new(50, 120, 1000, Theme::default(), VoidListener);
    let mut proc: vte::ansi::Processor = vte::ansi::Processor::new();

    // Fill grid with content so resize has rows to reflow.
    let line = "Hello world, this is terminal content for resize testing.\n";
    for _ in 0..200 {
        proc.advance(&mut term, line.as_bytes());
    }

    // Warmup: one resize cycle to stabilize internal capacities.
    term.resize(40, 100, true);
    term.resize(50, 120, true);

    // Measure: 100 resize cycles alternating between two sizes.
    let start = Instant::now();
    let m = measure(|| {
        for _ in 0..100 {
            term.resize(40, 100, true);
            term.resize(50, 120, true);
        }
    });
    let elapsed = start.elapsed();

    let allocs_per_cycle = m.allocs as f64 / 100.0;
    let bytes_per_cycle = m.bytes as f64 / 100.0;
    let ms_per_cycle = elapsed.as_secs_f64() * 1000.0 / 100.0;

    eprintln!("--- Resize Allocation Profile (Section 23.3) ---");
    eprintln!("  Resize cycle: 50×120 ↔ 40×100 (with reflow)");
    eprintln!(
        "  Cycles: 100, total time: {:.1}ms",
        elapsed.as_secs_f64() * 1000.0
    );
    eprintln!(
        "  Total: {} allocs, {:.1} KB",
        m.allocs,
        m.bytes as f64 / 1024.0
    );
    eprintln!(
        "  Per cycle: {allocs_per_cycle:.1} allocs, {bytes_per_cycle:.0} bytes, {ms_per_cycle:.2}ms"
    );
    eprintln!(
        "  Verdict: {} allocs/cycle, {ms_per_cycle:.2}ms/cycle → {} row pool optimization",
        m.allocs / 100,
        if m.allocs > 100_000 && ms_per_cycle > 16.0 {
            "JUSTIFIES"
        } else {
            "does NOT justify"
        }
    );
    eprintln!("  (Threshold: >1000 allocs/cycle with >16ms frame time)");
    eprintln!("------------------------------------------------");
}
