//! RSS (resident set size) regression tests for the mux pipeline.
//!
//! Drives a real pane via [`EmbeddedMux::spawn_pane`] with `shell:
//! Some("yes")` (or `cat` for the no-flood baseline) and samples
//! process RSS at intervals while the IO thread parses output. Pins the
//! bounded-growth invariant against the pre-fix unbounded byte channel.
//!
//! Linux-only — matches `tests/e2e.rs`. The integration-test-binary
//! isolation gives clean process-level RSS (no contamination from
//! sibling unit tests in the same `cargo test -p oriterm_mux` run).
//!
//! Trend pattern from `oriterm_core/tests/rss_regression.rs::rss_series_plateaus`:
//! take ≥6 samples across the workload and assert the post-warmup window
//! does NOT show monotonic growth past a 256 KiB OS-noise tolerance.
//!
//! Regression: BUG-11-002.
//! See: bug-tracker/plans/BUG-11-002/section-03-tdd-matrix.md.

#![cfg(target_os = "linux")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use oriterm_core::Theme;
use oriterm_mux::EmbeddedMux;
use oriterm_mux::backend::MuxBackend;
use oriterm_mux::domain::SpawnConfig;

const MB: usize = 1_048_576;
const NOISE_TOLERANCE: usize = 256 * 1024;

/// Read the current process RSS in bytes via `/proc/self/statm`.
fn rss_bytes() -> usize {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let resident_pages: usize = statm
        .split_whitespace()
        .nth(1)
        .expect("statm rss field")
        .parse()
        .expect("parse rss pages");
    resident_pages * 4096
}

/// SpawnConfig that runs a flooding command directly (no shell integration).
fn flood_spawn_config() -> SpawnConfig {
    SpawnConfig {
        cols: 80,
        rows: 24,
        shell: Some("yes".into()),
        cwd: None,
        env: Vec::new(),
        scrollback: 1000,
        // Disable shell integration so CommandBuilder runs `yes` directly
        // rather than injecting bash/zsh hooks.
        shell_integration: false,
    }
}

/// SpawnConfig for a quiet pane — `cat` blocks reading from PTY stdin.
fn quiet_spawn_config() -> SpawnConfig {
    SpawnConfig {
        cols: 80,
        rows: 24,
        shell: Some("cat".into()),
        cwd: None,
        env: Vec::new(),
        scrollback: 1000,
        shell_integration: false,
    }
}

/// Pump events for `dur`, calling `poll_events` and yielding briefly between
/// calls so the IO thread (background) gets scheduling time.
fn pump_for<B: MuxBackend>(mux: &mut B, dur: Duration) {
    let deadline = Instant::now() + dur;
    while Instant::now() < deadline {
        mux.poll_events();
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Format a measurement series as MB strings for assertion messages.
fn fmt_mb(samples: &[usize]) -> Vec<String> {
    samples
        .iter()
        .map(|&b| format!("{:.1}", b as f64 / MB as f64))
        .collect()
}

/// Repro test — RSS plateaus under sustained PTY flood.
///
/// Regression: BUG-11-002. Pre-fix the byte channel was unbounded, so
/// `yes`'s ~10 MB/s output grew the queue heap proportionally to flood
/// duration. Post-fix the bounded channel back-pressures through the
/// kernel PTY buffer to the child, capping per-pane queue heap at
/// `BYTE_CHANNEL_MEMORY_BUDGET = 1 MiB` and producing a stable plateau.
#[test]
fn mux_rss_plateaus_under_sustained_flood() {
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut mux = EmbeddedMux::new(wakeup);

    let _pane_id = mux
        .spawn_pane(&flood_spawn_config(), Theme::Dark)
        .expect("spawn pane");

    // Take 6 samples: a short warmup window then 5 wider windows so the
    // trend assertion has room to discriminate growth from noise.
    let mut measurements = Vec::with_capacity(6);
    for i in 0..6 {
        let win = if i == 0 {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(800)
        };
        pump_for(&mut mux, win);
        measurements.push(rss_bytes());
    }

    // After 2 warmup samples, the remaining 4 must NOT all grow past the
    // OS-noise tolerance — a real leak would scale with input volume,
    // crossing the threshold every step.
    let post_warmup = &measurements[2..];
    let all_increasing = post_warmup
        .windows(2)
        .all(|w| w[1] > w[0] + NOISE_TOLERANCE);

    assert!(
        !all_increasing,
        "RSS grew monotonically under sustained flood — bounded channel back-pressure failed. \
         samples: {:?} (MB: {:?})",
        measurements,
        fmt_mb(&measurements),
    );
}

/// Negative pin — RSS does NOT exhibit unbounded-growth pattern.
///
/// Regression: BUG-11-002. Forbid-output pin per
/// `.claude/rules/tests.md §Negative Testing Protocol`. Pre-fix the
/// unbounded queue grew the heap by ~128 KiB per queued message; over a
/// multi-second `yes` flood at ~10 MB/s output the heap would grow by
/// hundreds of MB. Asserts the absolute post-warmup max-min delta is
/// well under that range.
#[test]
fn mux_rss_negative_no_unbounded_growth() {
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut mux = EmbeddedMux::new(wakeup);
    let _pane_id = mux
        .spawn_pane(&flood_spawn_config(), Theme::Dark)
        .expect("spawn pane");

    let mut measurements = Vec::with_capacity(6);
    for i in 0..6 {
        let win = if i == 0 {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(800)
        };
        pump_for(&mut mux, win);
        measurements.push(rss_bytes());
    }

    // Post-warmup max - min must stay under 50 MB. Pre-fix this would
    // exceed 100 MB during a 4 s window.
    let post_warmup = &measurements[2..];
    let max = *post_warmup.iter().max().expect("≥1 post-warmup sample");
    let min = *post_warmup.iter().min().expect("≥1 post-warmup sample");
    let delta = max - min;
    assert!(
        delta < 50 * MB,
        "RSS post-warmup delta = {:.1} MB (allowed < 50 MB). samples: {:?} (MB: {:?})",
        delta as f64 / MB as f64,
        measurements,
        fmt_mb(&measurements),
    );
}

/// Baseline — empty pane RSS does not grow without flood input.
///
/// Regression: BUG-11-002 §03 edge case "empty pane". Spawns `cat`
/// (which blocks reading PTY stdin and produces no output) and verifies
/// the no-flood baseline doesn't exhibit growth — proves the trend
/// shape is sensitive to actual workload, not test harness overhead.
#[test]
fn mux_rss_bounded_empty_pane() {
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut mux = EmbeddedMux::new(wakeup);
    let _pane_id = mux
        .spawn_pane(&quiet_spawn_config(), Theme::Dark)
        .expect("spawn pane");

    let mut measurements = Vec::with_capacity(4);
    for _ in 0..4 {
        pump_for(&mut mux, Duration::from_millis(500));
        measurements.push(rss_bytes());
    }

    let all_increasing = measurements
        .windows(2)
        .all(|w| w[1] > w[0] + NOISE_TOLERANCE);
    assert!(
        !all_increasing,
        "Empty pane (no flood) RSS grew monotonically: {:?} (MB: {:?})",
        measurements,
        fmt_mb(&measurements),
    );
}

/// Cross-pane interaction — multiple panes flooding simultaneously.
///
/// Regression: BUG-11-002 §03 edge case "multiple panes". Spawns 4
/// flooding panes; each bounded byte channel caps at 1 MiB, so the
/// expected worst-case queue heap is ~4 MiB regardless of flood
/// duration. Pins the N-pane scaling property: bounded growth scales
/// with pane count, not with input volume.
#[test]
fn mux_rss_plateaus_with_multiple_panes() {
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut mux = EmbeddedMux::new(wakeup);

    for _ in 0..4 {
        mux.spawn_pane(&flood_spawn_config(), Theme::Dark)
            .expect("spawn pane");
    }

    let mut measurements = Vec::with_capacity(6);
    for i in 0..6 {
        let win = if i == 0 {
            Duration::from_millis(300)
        } else {
            Duration::from_millis(700)
        };
        pump_for(&mut mux, win);
        measurements.push(rss_bytes());
    }

    let post_warmup = &measurements[2..];
    let all_increasing = post_warmup
        .windows(2)
        .all(|w| w[1] > w[0] + NOISE_TOLERANCE);
    assert!(
        !all_increasing,
        "RSS grew monotonically with 4 flooding panes: {:?} (MB: {:?})",
        measurements,
        fmt_mb(&measurements),
    );
}

/// Resize during flood — bounded channels do not deadlock the resize path.
///
/// Regression: BUG-11-002 §03 cross-feature interaction "resize during
/// flood". Sends a Resize command mid-flood; verifies the IO thread
/// completes the resize (snapshot reports rows=30, cols=100) AND RSS
/// still plateaus. Pins that the bounded byte channel doesn't block
/// the resize command path — `cmd_tx` remains unbounded by design
/// (BUG-11-025 owns the Resize-coalescing-via-AtomicU64 follow-up).
#[test]
fn mux_rss_plateaus_through_resize() {
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut mux = EmbeddedMux::new(wakeup);

    let pane_id = mux
        .spawn_pane(&flood_spawn_config(), Theme::Dark)
        .expect("spawn pane");

    let mut measurements = Vec::with_capacity(5);
    let mut resize_observed = false;
    for i in 0..5 {
        if i == 2 {
            mux.resize_pane_grid(pane_id, 30, 100);
        }
        let win = if i == 0 {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(700)
        };
        pump_for(&mut mux, win);

        // After the resize, poll the snapshot (per pump tick) until it
        // reports the new dimensions. Sets `resize_observed` once seen
        // so the assertion at the end of the test fails fast if the
        // resize was silently dropped.
        if i >= 2 && !resize_observed {
            if let Some(snap) = mux.refresh_pane_snapshot(pane_id) {
                if snap.cells.len() == 30 && snap.cols == 100 {
                    resize_observed = true;
                }
            }
        }

        measurements.push(rss_bytes());
    }

    assert!(
        resize_observed,
        "Mid-flood resize never landed in a published snapshot — IO thread \
         may be deadlocked or the resize command was dropped. samples: {:?}",
        measurements,
    );

    let post_warmup = &measurements[1..];
    let all_increasing = post_warmup
        .windows(2)
        .all(|w| w[1] > w[0] + NOISE_TOLERANCE);
    assert!(
        !all_increasing,
        "RSS grew through resize-mid-flood: {:?} (MB: {:?})",
        measurements,
        fmt_mb(&measurements),
    );
}
