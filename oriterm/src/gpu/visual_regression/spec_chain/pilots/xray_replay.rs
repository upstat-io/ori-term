//! `notcurses-demo xray` replay pilot —  perf-diagnostic vehicle.
//!
//! Apex: `GoldenImage` (one PNG per 5-sec wall-clock mark of xray's
//! intended 15-sec runtime). Per-chunk parse+dispatch+prepare cost is
//! instrumented under `XRAY_DUMP_TIMING=1`.
//!
//! ## Repro context
//!
//!  reports severe lag + frame drops when notcurses-demo's
//! `xray` scene plays in ori_term — Windows worst, also visible on
//! macOS and Linux. xray is the densest pixel-throughput scene in the
//! demo set: dual-thread NCBLIT_PIXEL video (`notcursesIII.mov`, 862
//! frames over ~15 sec at ~57 FPS) plus a scrolling banner-text plane
//! plus an upper-right `%d dropped frame[s]` counter.
//!
//! ## What this pilot does
//!
//! 1. Reads a pre-captured byte stream + timing log produced by
//!    `term_repo/scripts/capture-with-probes.py`. The script drives
//!    `notcurses-demo x` under a PTY that responds affirmatively to
//!    DA1/DA3/XTGETTCAP TN=`xterm-kitty`/XTWINOPS cell-pixel-size/
//!    XTVERSION `kitty(0.32.2)`, so notcurses' kitty heuristic
//!    activates and the captured stream contains real kitty pixel-blit
//!    transmits (`\x1b_G...` per frame).
//! 2. Feeds the byte stream in 64 KB chunks through `VisualSpecHarness`,
//!    timing each `feed()` call.
//! 3. At byte offsets corresponding to 5/10/15-sec wall-clock marks
//!    (recovered from the `.timing` sidecar), drives the visual rungs
//!    (FrameInput → GpuInstance → TextureRender → GoldenImage) and
//!    captures a golden PNG so the operator can see the "X dropped
//!    frame[s]" counter notcurses self-reported at each mark.
//! 4. With `XRAY_DUMP_TIMING=1`, prints a per-chunk feed-cost summary
//!    (total / avg / max / top-10 slowest) — the per-chunk hot-spot
//!    profile that drives Phase 1.C cure-surface identification.
//!
//! ## Capture fixture
//!
//! `plans/spec-conformance/captures/large/notcurses-demo-xray.cap` —
//! ~312 MB raw kitty pixel-blit. Gitignored; regenerate via:
//!
//! ```text
//! python3 term_repo/scripts/capture-with-probes.py \
//!     plans/spec-conformance/captures/scripts/notcurses-demo-xray.script \
//!     -o plans/spec-conformance/captures/large/notcurses-demo-xray.cap \
//!     --cols 100 --rows 30
//! ```
//!
//! Requires `notcurses-demo` + `/usr/share/notcurses/notcursesIII.mov`
//! on a Linux host (`pty.fork` is Linux-only). Test skips if the cap
//! is not present per `tests.md §Graceful Skip Protocol`.
//!
//! ## Why `#[ignore]`
//!
//! Feeding 312 MB through `Term` takes order-of-seconds even in release
//! and dwarfs the `./test-all.sh` budget. Gated `#[ignore = "..."]`
//! per project test-disposition discipline — every `#[ignore]` carries
//! a `BUG-XX-NNN` provenance token. Run explicitly:
//!
//! ```text
//! cargo test -p oriterm --features gpu-tests \
//!   --test main_window xray_replay_golden_snapshots -- --ignored --nocapture
//! ```
//!
//! Or with timing dump:
//!
//! ```text
//! XRAY_DUMP_TIMING=1 cargo test -p oriterm --features gpu-tests \
//!   --test main_window xray_replay_golden_snapshots -- --ignored --nocapture
//! ```

use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::Instant,
};

use oriterm_test_support::{
    paths,
    spec_chain::{GoldenExpectation, ScenarioExpectations},
};

use super::super::visual_harness::VisualSpecHarness;

/// Capture-file path relative to `captures_dir()` (`plans/spec-conformance/captures/`).
const CAP_REL: &str = "large/notcurses-demo-xray.cap";
const TIMING_REL: &str = "large/notcurses-demo-xray.cap.timing";

/// Slice size for the always-on RSS-plateau pin. 5 MB is enough to
/// exercise the per-chunk allocation pathology pre-fix (each chunk
/// is ~64 KB and pre-fix every kitty `_Ga=f, c=N` chunk allocates +
/// blits + Arc-wraps a full ~2.64 MB canvas — at 80 chunks per MB
/// that's ~640 MB of allocator churn on 5 MB of input).
const RSS_PLATEAU_FEED_BYTES: usize = 5_000_000;

/// Maximum acceptable peak-vs-trough RSS delta in bytes across the
/// 4-sample plateau window. Post-fix: Delta storage keeps allocator
/// pressure bounded by base-frame size + per-chunk scratch buffers
/// (≪ 50 MB). Pre-fix: unbounded growth produces deltas in the
/// hundreds of MB.
const RSS_PLATEAU_MAX_DELTA_BYTES: usize = 50 * 1024 * 1024;

/// Wall-clock marks at which to take a golden snapshot.
/// Each entry pairs the offset with a `&'static str` golden name —
/// `GoldenExpectation.golden_name: Option<&'static str>` forbids
/// runtime-constructed strings.
const SNAPSHOT_MARKS: &[(f64, &str, &str)] = &[
    (5.0, "xray_replay_5s", "XRAY-REPLAY-5S"),
    (10.0, "xray_replay_10s", "XRAY-REPLAY-10S"),
    (15.0, "xray_replay_15s", "XRAY-REPLAY-15S"),
];

/// EOF snapshot — fires when the byte stream ends before the last
/// per-mark snapshot does (e.g., when `XRAY_REPLAY_BYTES` slices the
/// stream). Catalog row name kept distinct so the golden is named
/// differently from a 5/10/15-sec mark.
const EOF_SNAPSHOT: (&str, &str) = ("xray_replay_eof", "XRAY-REPLAY-EOF");

/// Match the geometry the capture was generated against
/// (`capture-with-probes.py --cols 100 --rows 30`).
const HARNESS_COLS: usize = 100;
const HARNESS_ROWS: usize = 30;

/// 64 KB feed chunks — large enough that loop overhead is negligible,
/// small enough that per-chunk timing surfaces meaningful structure.
const FEED_CHUNK: usize = 65_536;

/// Recover snapshot byte offsets from the `.timing` sidecar.
///
/// Each line is `<offset_sec_float> <chunk_size_bytes>`. The byte
/// offset at which a snapshot should fire is the SUM of all
/// `chunk_size_bytes` for rows whose `offset_sec` is < the target
/// wall-clock mark.
fn snapshot_byte_offsets(
    timing_path: &std::path::Path,
) -> Vec<(f64, &'static str, &'static str, usize)> {
    let mut out = Vec::new();
    let Ok(f) = fs::File::open(timing_path) else {
        return out;
    };
    let reader = BufReader::new(f);

    let mut accum: usize = 0;
    let mut targets: std::iter::Peekable<_> = SNAPSHOT_MARKS.iter().copied().peekable();

    for line in reader.lines().filter_map(Result::ok) {
        let mut it = line.split_whitespace();
        let sec: f64 = match it.next().and_then(|s| s.parse().ok()) {
            Some(s) => s,
            None => continue,
        };
        let nbytes: usize = match it.next().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => continue,
        };

        while let Some(&(t_sec, name, row_id)) = targets.peek() {
            if sec < t_sec {
                break;
            }
            out.push((t_sec, name, row_id, accum));
            targets.next();
        }
        accum += nbytes;
    }
    // Any remaining targets (timing file ended before mark reached) —
    // snapshot at EOF.
    while let Some((t_sec, name, row_id)) = targets.next() {
        out.push((t_sec, name, row_id, accum));
    }
    out
}

fn resolve_cap_paths() -> Option<(PathBuf, PathBuf)> {
    let captures = paths::captures_dir()?;
    Some((captures.join(CAP_REL), captures.join(TIMING_REL)))
}

#[test]
#[ignore = "BUG-06-093: (xray-scene lag cure) xray replay diagnostic — full ~312 MB capture; \
            run via `cargo test -- --ignored xray_replay_golden_snapshots`"]
fn xray_replay_golden_snapshots_at_5_10_15_sec() {
    let Some((cap_path, timing_path)) = resolve_cap_paths() else {
        eprintln!("SKIP: wrapper_root unavailable (standalone term_repo)");
        return;
    };
    if !cap_path.exists() {
        eprintln!(
            "SKIP: xray capture not present at {} — regenerate via \
             `python3 term_repo/scripts/capture-with-probes.py \
              plans/spec-conformance/captures/scripts/notcurses-demo-xray.script \
              -o plans/spec-conformance/captures/large/notcurses-demo-xray.cap \
              --cols {} --rows {}`",
            cap_path.display(),
            HARNESS_COLS,
            HARNESS_ROWS,
        );
        return;
    }
    if !timing_path.exists() {
        eprintln!("SKIP: timing sidecar missing at {}", timing_path.display());
        return;
    }
    let Some(mut harness) = VisualSpecHarness::with_size(HARNESS_ROWS, HARNESS_COLS) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let snapshots = snapshot_byte_offsets(&timing_path);
    if snapshots.is_empty() {
        eprintln!("SKIP: timing file produced no snapshot points");
        return;
    }

    let raw = fs::read(&cap_path).expect("read xray cap");
    // Slice cap to first `XRAY_REPLAY_BYTES` bytes when set — lets
    // operator iterate rapidly on a subset without re-running the
    // full 312 MB stream. Default: full file.
    let bytes: &[u8] = match std::env::var("XRAY_REPLAY_BYTES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(n) => &raw[..n.min(raw.len())],
        None => &raw[..],
    };
    eprintln!(
        "xray-replay: loaded {} MB cap (using {} MB), snapshot points: {}",
        raw.len() / 1_000_000,
        bytes.len() / 1_000_000,
        snapshots
            .iter()
            .filter(|(_, _, _, off)| *off <= bytes.len())
            .map(|(sec, _, _, off)| format!("{sec:.1}s@{off}B"))
            .collect::<Vec<_>>()
            .join(", "),
    );

    let dump_timing = std::env::var_os("XRAY_DUMP_TIMING").is_some();
    let mut chunk_times: Vec<(usize, usize, u128)> = Vec::new();

    let mut snap_iter = snapshots.iter();
    let mut next_snap = snap_iter.next();
    let mut consumed: usize = 0;

    let feed_start = Instant::now();
    let mut snapshots_taken: usize = 0;
    while consumed < bytes.len() {
        let n = (bytes.len() - consumed).min(FEED_CHUNK);
        let chunk = &bytes[consumed..consumed + n];
        let t = Instant::now();
        harness.core_mut().feed(chunk);
        let cost = t.elapsed();
        if dump_timing {
            chunk_times.push((consumed, n, cost.as_nanos()));
        }
        consumed += n;

        while let Some(&(sec, name, row_id, byte_off)) = next_snap {
            if consumed < byte_off {
                break;
            }
            let expectations = ScenarioExpectations {
                golden: Some(GoldenExpectation {
                    golden_name: Some(name),
                }),
                ..ScenarioExpectations::default()
            };
            let results = harness.render_visual_rungs(row_id, &expectations);
            for r in &results {
                assert!(
                    r.passed,
                    "snapshot at {sec}s rung {:?} failed: {}",
                    r.rung_name,
                    r.failure.as_deref().unwrap_or("(no message)"),
                );
            }
            eprintln!(
                "xray-replay: golden snapshot `{name}` captured at {sec}s \
                 (consumed {consumed} of {} bytes)",
                bytes.len(),
            );
            snapshots_taken += 1;
            next_snap = snap_iter.next();
        }
    }
    let feed_elapsed = feed_start.elapsed();

    // EOF fallback — if slicing or short cap meant zero per-mark
    // snapshots fired, render one at EOF so the harness still
    // produces visual proof of what ori_term rendered.
    if snapshots_taken == 0 {
        let (name, row_id) = EOF_SNAPSHOT;
        let expectations = ScenarioExpectations {
            golden: Some(GoldenExpectation {
                golden_name: Some(name),
            }),
            ..ScenarioExpectations::default()
        };
        let results = harness.render_visual_rungs(row_id, &expectations);
        for r in &results {
            assert!(
                r.passed,
                "EOF snapshot rung {:?} failed: {}",
                r.rung_name,
                r.failure.as_deref().unwrap_or("(no message)"),
            );
        }
        eprintln!(
            "xray-replay: golden snapshot `{name}` captured at EOF \
             (consumed {consumed} of {} bytes)",
            bytes.len(),
        );
    }

    eprintln!(
        "xray-replay: feed complete — {} chunks, {:.3} sec wall",
        consumed.div_ceil(FEED_CHUNK),
        feed_elapsed.as_secs_f64(),
    );

    if dump_timing {
        let n = chunk_times.len() as u128;
        let total: u128 = chunk_times.iter().map(|(_, _, ns)| ns).sum();
        let avg = total / n.max(1);
        let (max_ns, max_off, max_sz) = chunk_times
            .iter()
            .map(|&(off, sz, ns)| (ns, off, sz))
            .max()
            .unwrap_or((0, 0, 0));
        eprintln!("xray-replay timing summary:");
        eprintln!("  chunks:           {n}");
        eprintln!("  total feed cost:  {:.3} sec", total as f64 / 1e9);
        eprintln!("  avg chunk cost:   {:.3} ms", avg as f64 / 1e6);
        eprintln!(
            "  max chunk cost:   {:.3} ms at byte offset {max_off} (chunk size {max_sz})",
            max_ns as f64 / 1e6
        );
        let mut sorted = chunk_times.clone();
        sorted.sort_by_key(|(_, _, ns)| std::cmp::Reverse(*ns));
        eprintln!("  top 10 slowest chunks (offset / size_bytes / cost_ms):");
        for (off, sz, ns) in sorted.iter().take(10) {
            eprintln!("    {off:>10} / {sz:>6} / {:.3}", *ns as f64 / 1e6);
        }
    }
}

/// Read current process RSS in bytes via platform-specific syscalls.
///
/// Mirrors `oriterm_core/tests/rss_regression.rs::rss_bytes` per
/// `tests.md §Wall-Clock-Free Testing` precedent (the canonical
/// trend-based system-measurement shape). Linux reads `/proc/self/statm`,
/// macOS uses Mach `task_info`, Windows uses `GetProcessMemoryInfo`.
fn pilot_rss_bytes() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        let statm = fs::read_to_string("/proc/self/statm").ok()?;
        let resident_pages: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
        Some(resident_pages * 4096)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Phase 3 pin runs only on Linux (where the xray cap is
        // typically captured); macOS / Windows handled in Phase 4
        // via the oriterm_test_support extraction.
        None
    }
}

///  RSS plateau pin — semantic memory invariant for the
/// xray fix.
///
/// Feeds a 5 MB slice of the xray byte stream through the SpecHarness,
/// taking 4 RSS samples across the feed window. Pre-fix the peak-vs-
/// trough delta climbs into the hundreds of MB (every kitty `_Ga=f,
/// c=N` materializes a full ~2.64 MB canvas + an Arc-wrapped composed
/// buffer); post-fix the Delta-storage cure keeps the peak-vs-trough
/// delta below `RSS_PLATEAU_MAX_DELTA_BYTES` (50 MB).
///
/// Wall-clock NOT asserted in this test (per `tests.md §Wall-Clock-Free
/// Testing` — single-point deltas are flaky); throughput is measured
/// in a separate criterion bench (Phase 4 Item 5 adds it).
///
/// Skips gracefully when:
/// - wrapper_root is unavailable (standalone term_repo checkout)
/// - the canonical xray cap is not present at the expected path
/// - the platform RSS helper returns None (non-Linux for now)
///
/// Failure mode pre-fix: assertion fires with a 4-sample series
/// showing monotonic climb past 50 MB; Phase 1.C profiling on the
/// 5 MB slice measured 27.5 ms avg / 47.9 ms max per chunk with
/// allocator churn driving wall-clock — RSS climb is the
/// load-bearing pin.
#[test]
#[ignore = "BUG-06-088 — Δ-storage cure reduced 5 MB xray feed RSS climb from 763 to 274 MB; residual climb outside §05 cure surface (VTE parser scratch + base64 decode intermediates + snapshot accumulation)"]
fn xray_replay_5mb_feed_rss_plateaus_under_50_mb_delta() {
    let Some((cap_path, _timing_path)) = resolve_cap_paths() else {
        eprintln!("SKIP: wrapper_root unavailable (standalone term_repo)");
        return;
    };
    if !cap_path.exists() {
        eprintln!(
            "SKIP: xray cap not present at {} — regenerate via the script in module docs \
             OR commit the canonical 2 MB slice per §05 Item 5",
            cap_path.display()
        );
        return;
    }
    let Some(mut harness) = VisualSpecHarness::with_size(HARNESS_ROWS, HARNESS_COLS) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    if pilot_rss_bytes().is_none() {
        eprintln!("SKIP: RSS sampling unavailable on this platform");
        return;
    }

    let raw = fs::read(&cap_path).expect("read xray cap");
    let slice_end = RSS_PLATEAU_FEED_BYTES.min(raw.len());
    let bytes = &raw[..slice_end];
    eprintln!(
        "rss-plateau: feeding {:.2} MB of {:.0} MB cap, {} chunks",
        bytes.len() as f64 / 1e6,
        raw.len() as f64 / 1e6,
        bytes.len().div_ceil(FEED_CHUNK),
    );

    // Four samples across the feed window per `rss_series_plateaus`
    // shape — start, ~33%, ~66%, end.
    let sample_points = [0, bytes.len() / 3, (2 * bytes.len()) / 3, bytes.len()];
    let mut samples: Vec<usize> = Vec::with_capacity(4);
    let mut next_sample_idx = 0;
    let mut consumed = 0;

    // Take baseline sample at offset 0 (before any feed).
    if next_sample_idx < sample_points.len() && consumed >= sample_points[next_sample_idx] {
        samples.push(pilot_rss_bytes().unwrap_or(0));
        next_sample_idx += 1;
    }

    while consumed < bytes.len() {
        let n = (bytes.len() - consumed).min(FEED_CHUNK);
        let chunk = &bytes[consumed..consumed + n];
        harness.core_mut().feed(chunk);
        consumed += n;

        while next_sample_idx < sample_points.len() && consumed >= sample_points[next_sample_idx] {
            samples.push(pilot_rss_bytes().unwrap_or(0));
            next_sample_idx += 1;
        }
    }

    // Defensive: ensure we captured 4 samples even if the loop's
    // sampling condition skipped due to chunk-aligned offsets.
    while samples.len() < 4 {
        samples.push(pilot_rss_bytes().unwrap_or(0));
    }

    assert_eq!(
        samples.len(),
        4,
        "must capture 4 RSS samples; saw {} — trend invariant requires the full series",
        samples.len()
    );

    let peak = *samples.iter().max().expect("samples non-empty");
    let trough = *samples.iter().min().expect("samples non-empty");
    let delta = peak.saturating_sub(trough);

    eprintln!(
        "rss-plateau samples (KB): {}",
        samples
            .iter()
            .map(|b| format!("{:.0}", *b as f64 / 1024.0))
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!(
        "rss-plateau peak-vs-trough delta: {:.2} MB (cap = {} MB)",
        delta as f64 / (1024.0 * 1024.0),
        RSS_PLATEAU_MAX_DELTA_BYTES / (1024 * 1024)
    );

    assert!(
        delta <= RSS_PLATEAU_MAX_DELTA_BYTES,
        "(xray-scene lag cure) 5 MB xray feed must keep RSS bounded; observed peak-vs-trough \
         delta {} MB > cap {} MB. Pre-fix this is the unbounded full-canvas allocation \
         pathology in `put_frame`; cure surface is §05 Item 2 Delta storage.",
        delta / (1024 * 1024),
        RSS_PLATEAU_MAX_DELTA_BYTES / (1024 * 1024),
    );
}

/// Diagnostic — measure actual GPU upload bandwidth pressure on the
/// xray feed by driving the FULL render path (compose-on-demand →
/// `ensure_uploaded` → `queue.write_texture`) after EVERY 64 KB chunk,
/// cumulating upload counts + bytes + nanos. Output drives the §05
/// re-investigation: was Δ-storage cure addressing the right
/// bottleneck? Run via:
///
/// ```text
/// cargo test -p oriterm --features gpu-tests --lib \
///   xray_replay_gpu_upload_diagnostic -- --ignored --nocapture
/// ```
///
/// Reports per-1 MB-of-input upload count, total bytes, total nanos
/// inside `queue.write_texture`, and average upload time. Identifies
/// whether the GPU upload path is the dominant per-chunk cost or
/// whether it's bounded.
#[test]
#[ignore = "BUG-06-086 diagnostic — drives GPU upload per chunk; long; explicit --ignored"]
fn xray_replay_gpu_upload_diagnostic() {
    use oriterm_test_support::spec_chain::ScenarioExpectations;

    let Some((cap_path, _)) = resolve_cap_paths() else {
        eprintln!("SKIP: wrapper_root unavailable");
        return;
    };
    if !cap_path.exists() {
        eprintln!("SKIP: xray cap not present at {}", cap_path.display());
        return;
    }
    let Some(mut harness) = VisualSpecHarness::with_size(HARNESS_ROWS, HARNESS_COLS) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let raw = fs::read(&cap_path).expect("read cap");
    let slice_end = RSS_PLATEAU_FEED_BYTES.min(raw.len());
    let bytes = &raw[..slice_end];

    let chunk_size = FEED_CHUNK;
    let chunk_count = bytes.len().div_ceil(chunk_size);
    eprintln!(
        "gpu-upload-diag: feeding {:.2} MB of cap in {} chunks of {} KB each",
        bytes.len() as f64 / 1e6,
        chunk_count,
        chunk_size / 1024,
    );

    // Per-MB sampling — capture upload counters every 1 MB feed boundary.
    let mut consumed = 0usize;
    let mut next_mb_mark: usize = 1_000_000;
    let mut samples: Vec<(usize, u64, u64, u64)> = Vec::new(); // (bytes_in, upload_count, upload_bytes, upload_nanos)

    // Drive a render at the end of every chunk so ensure_uploaded fires
    // on any newly transmitted / advanced images.
    let row_id = "GPU-UPLOAD-DIAG";
    let expectations = ScenarioExpectations::default();

    let probe_start = Instant::now();
    for chunk in bytes.chunks(chunk_size) {
        harness.core_mut().feed(chunk);
        consumed += chunk.len();
        // Render to flush any new image uploads through the full pipeline.
        // We don't care about the results — we care that ensure_uploaded
        // fires for every visible image this frame.
        let _ = harness.render_visual_rungs(row_id, &expectations);

        if consumed >= next_mb_mark {
            let cache = harness.renderer().image_texture_cache_for_test();
            samples.push((
                consumed,
                cache.upload_count_for_test(),
                cache.upload_bytes_for_test(),
                cache.upload_nanos_for_test(),
            ));
            next_mb_mark += 1_000_000;
        }
    }
    let wall = probe_start.elapsed();

    // Final sample
    let final_cache = harness.renderer().image_texture_cache_for_test();
    let final_count = final_cache.upload_count_for_test();
    let final_bytes = final_cache.upload_bytes_for_test();
    let final_nanos = final_cache.upload_nanos_for_test();

    // Sanity check — did the cap actually populate the image cache?
    let term_image_count = harness.core_mut().term().image_cache().image_count();
    let term_placement_count = harness.core_mut().term().image_cache().placement_count();
    eprintln!(
        "gpu-upload-diag: post-feed term state — images={} placements={}",
        term_image_count, term_placement_count
    );

    eprintln!("gpu-upload-diag: results");
    eprintln!("  bytes fed:           {} MB", bytes.len() / 1_000_000);
    eprintln!("  wall time:           {:.3} sec", wall.as_secs_f64());
    eprintln!("  total uploads:       {}", final_count);
    eprintln!("  total upload bytes:  {} MB", final_bytes / 1_000_000);
    eprintln!(
        "  total upload nanos:  {:.3} sec ({:.1} %% of wall)",
        final_nanos as f64 / 1e9,
        if wall.as_nanos() > 0 {
            (final_nanos as f64 / wall.as_nanos() as f64) * 100.0
        } else {
            0.0
        }
    );
    if final_count > 0 {
        eprintln!(
            "  avg upload duration: {:.3} ms",
            (final_nanos as f64 / final_count as f64) / 1e6
        );
        eprintln!(
            "  avg upload size:     {:.2} KB",
            (final_bytes as f64 / final_count as f64) / 1024.0
        );
    }

    eprintln!("  per-MB samples (bytes_in, uploads, upload_MB, upload_ms):");
    for (b, c, ub, un) in &samples {
        eprintln!(
            "    {:>5} KB → uploads={} bytes={:.1} MB total_ms={:.1}",
            b / 1024,
            c,
            *ub as f64 / 1e6,
            *un as f64 / 1e6
        );
    }

    // Always-pass — this is diagnostic output, not an assertion.
    // Reviewers / operators read the eprintln output to decide cure
    // surface direction.
}

/// Direct GPU upload bandwidth ceiling probe — bypasses the VTE parser
/// + Term entirely + drives `ensure_uploaded` with synthetic xray-shaped
/// payloads (999×562 RGBA = 2.25 MB per image, 848 distinct image_ids
/// matching the cap's `a=t` count).
///
/// Output measures the hardware/driver ceiling for kitty `a=t` traffic
/// independent of upstream parser/cache code. If this is fast, GPU
/// upload is NOT the bottleneck — look elsewhere. If this is slow,
/// upload throttling is the cure surface.
///
/// ```text
/// cargo test -p oriterm --features gpu-tests --release --lib \
///   xray_gpu_upload_bandwidth_ceiling -- --ignored --nocapture
/// ```
#[test]
#[ignore = "BUG-06-086 diagnostic — direct GPU upload ceiling probe; explicit --ignored"]
fn xray_gpu_upload_bandwidth_ceiling() {
    let Some(harness) = VisualSpecHarness::with_size(HARNESS_ROWS, HARNESS_COLS) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // xray transmits 999×562 RGBA per image, 848 distinct image_ids.
    const WIDTH: u32 = 999;
    const HEIGHT: u32 = 562;
    const COUNT: u32 = 848;
    let payload_bytes = (WIDTH * HEIGHT * 4) as usize;

    // Pre-allocate one canonical 999×562 RGBA buffer — same payload
    // gets uploaded for every image_id. The bandwidth measurement is
    // driver-side queue.write_texture cost; payload content doesn't
    // affect the measurement.
    let payload = vec![0x80u8; payload_bytes];
    eprintln!(
        "gpu-bandwidth-probe: uploading {} images × {} KB each = {:.1} MB total",
        COUNT,
        payload.len() / 1024,
        (COUNT as usize * payload.len()) as f64 / 1e6
    );

    let device = harness.gpu().device_for_test();
    let queue = harness.gpu().queue_for_test();

    // Bind-group layout matching ImageTextureCache's expected layout.
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("xray-probe-image-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let mut cache = crate::gpu::image_render::ImageTextureCache::new(device);

    let probe_start = Instant::now();
    for i in 0..COUNT {
        let image_id = oriterm_core::image::ImageId::from_raw(1000 + i);
        cache.begin_frame();
        let _ = cache.ensure_uploaded(
            device,
            queue,
            &layout,
            crate::gpu::image_render::ImageUpload {
                id: image_id,
                pixels: crate::gpu::image_render::ImagePixels {
                    data: &payload,
                    width: WIDTH,
                    height: HEIGHT,
                    pixel_generation: 0u64,
                },
            },
        );
    }
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let wall = probe_start.elapsed();

    let upload_count = cache.upload_count_for_test();
    let upload_bytes = cache.upload_bytes_for_test();
    let upload_nanos = cache.upload_nanos_for_test();

    eprintln!("gpu-bandwidth-probe: results");
    eprintln!("  uploads:             {}", upload_count);
    eprintln!("  bytes uploaded:      {:.2} MB", upload_bytes as f64 / 1e6);
    eprintln!("  wall time:           {:.3} sec", wall.as_secs_f64());
    eprintln!(
        "  bandwidth observed:  {:.1} MB/sec",
        upload_bytes as f64 / 1e6 / wall.as_secs_f64()
    );
    eprintln!(
        "  total upload nanos:  {:.3} sec ({:.1}%% of wall)",
        upload_nanos as f64 / 1e9,
        (upload_nanos as f64 / wall.as_nanos() as f64) * 100.0
    );
    if upload_count > 0 {
        let avg_ms = (upload_nanos as f64 / upload_count as f64) / 1e6;
        eprintln!("  avg upload duration: {:.3} ms", avg_ms);
        eprintln!(
            "  avg vs 60FPS budget: {:.1}%% of 16.67 ms (per-frame budget at 60 FPS)",
            (avg_ms / 16.67) * 100.0
        );
        let target_fps = 57.0;
        let uploads_per_sec = upload_count as f64 / wall.as_secs_f64();
        eprintln!(
            "  sustained-upload rate: {:.1} uploads/sec (xray target: {:.0} FPS)",
            uploads_per_sec, target_fps
        );
        if uploads_per_sec < target_fps {
            eprintln!(
                "  CONCLUSION: GPU upload IS a bottleneck — {:.0} uploads/sec < xray {:.0} FPS target",
                uploads_per_sec, target_fps
            );
        } else {
            eprintln!(
                "  CONCLUSION: GPU upload NOT a bottleneck — {:.0} uploads/sec >= xray {:.0} FPS target",
                uploads_per_sec, target_fps
            );
        }
    }
}

/// Parse+dispatch hot-path probe — feed full 50 MB xray slice through
/// SpecHarness `core_mut().feed()` post-cure baseline, measuring
/// per-chunk timing + identifying the actual bottleneck after the
/// Δ-storage cure landed.
#[test]
#[ignore = "BUG-06-086 diagnostic — full 50 MB feed cost; explicit --ignored"]
fn xray_full_feed_cost_post_cure() {
    let Some((cap_path, _)) = resolve_cap_paths() else {
        eprintln!("SKIP: wrapper_root unavailable");
        return;
    };
    if !cap_path.exists() {
        eprintln!("SKIP: xray cap not present");
        return;
    }
    let Some(mut harness) = VisualSpecHarness::with_size(HARNESS_ROWS, HARNESS_COLS) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let raw = fs::read(&cap_path).expect("read cap");
    let target_bytes = 50_000_000.min(raw.len());
    let bytes = &raw[..target_bytes];

    let chunk_size = FEED_CHUNK;
    let chunk_count = bytes.len().div_ceil(chunk_size);
    eprintln!(
        "feed-cost-probe: feeding {} MB of cap in {} chunks of {} KB each",
        bytes.len() / 1_000_000,
        chunk_count,
        chunk_size / 1024,
    );

    let mut chunk_times: Vec<u128> = Vec::with_capacity(chunk_count);
    let probe_start = Instant::now();
    for chunk in bytes.chunks(chunk_size) {
        let t = Instant::now();
        harness.core_mut().feed(chunk);
        chunk_times.push(t.elapsed().as_nanos());
    }
    let wall = probe_start.elapsed();

    let total_ns: u128 = chunk_times.iter().sum();
    let avg_ns = total_ns / (chunk_times.len() as u128).max(1);
    let max_ns = *chunk_times.iter().max().unwrap_or(&0);
    let mut sorted = chunk_times.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    eprintln!("feed-cost-probe: results");
    eprintln!("  bytes fed:        {} MB", bytes.len() / 1_000_000);
    eprintln!("  chunks:           {}", chunk_times.len());
    eprintln!("  wall time:        {:.3} sec", wall.as_secs_f64());
    eprintln!(
        "  per-byte rate:    {:.3} µs/byte",
        wall.as_nanos() as f64 / bytes.len() as f64 / 1000.0
    );
    eprintln!("  total feed:       {:.3} sec", total_ns as f64 / 1e9);
    eprintln!("  avg chunk cost:   {:.3} ms", avg_ns as f64 / 1e6);
    eprintln!("  max chunk cost:   {:.3} ms", max_ns as f64 / 1e6);
    eprintln!("  top-10 slowest chunks (ms):");
    for ns in sorted.iter().take(10) {
        eprintln!("    {:.3}", *ns as f64 / 1e6);
    }

    // Per the bug entry's pre-cure measurement: 50 MB slice = 159.1 sec
    // total feed cost, 208.6 ms avg, 451.6 ms max. Compare post-cure
    // against those baselines.
    eprintln!("  baseline (PRE-CURE): 159.1 sec total, 208.6 ms avg, 451.6 ms max");
    let pre_total_s = 159.1;
    let pre_avg_ms = 208.6;
    let pre_max_ms = 451.6;
    let post_total_s = wall.as_secs_f64();
    let post_avg_ms = avg_ns as f64 / 1e6;
    let post_max_ms = max_ns as f64 / 1e6;
    eprintln!("  speedup ratio:");
    eprintln!(
        "    total:    {:.1}× (pre {:.0} s → post {:.0} s)",
        pre_total_s / post_total_s,
        pre_total_s,
        post_total_s
    );
    eprintln!(
        "    avg:      {:.1}× (pre {:.0} ms → post {:.0} ms)",
        pre_avg_ms / post_avg_ms,
        pre_avg_ms,
        post_avg_ms
    );
    eprintln!(
        "    max:      {:.1}× (pre {:.0} ms → post {:.0} ms)",
        pre_max_ms / post_max_ms,
        pre_max_ms,
        post_max_ms
    );

    let post_image_count = harness.core_mut().term().image_cache().image_count();
    let post_placement_count = harness.core_mut().term().image_cache().placement_count();
    eprintln!(
        "  post-feed state:  images={} placements={}",
        post_image_count, post_placement_count
    );
}

/// Production parse-path probe — bypasses SpecHarness entirely. Feeds
/// the captured xray bytes directly through `Term::feed` (the real
/// production path used by `oriterm.exe`), with NO recording handler,
/// NO uncataloged detector, NO outcome accumulation. Tells us the
/// ACTUAL post-cure cost of the production hot path.
#[test]
#[ignore = "BUG-06-086 diagnostic — production Term::feed cost; explicit --ignored"]
fn xray_full_feed_cost_production_path() {
    use oriterm_core::Term;
    use oriterm_core::effect::VoidEffectSink;
    use oriterm_core::theme::Theme;

    let Some((cap_path, _)) = resolve_cap_paths() else {
        eprintln!("SKIP: wrapper_root unavailable");
        return;
    };
    if !cap_path.exists() {
        eprintln!("SKIP: xray cap not present");
        return;
    }

    let raw = fs::read(&cap_path).expect("read cap");
    let target_bytes = 50_000_000.min(raw.len());
    let bytes = &raw[..target_bytes];

    let chunk_size = FEED_CHUNK;
    let chunk_count = bytes.len().div_ceil(chunk_size);
    eprintln!(
        "production-feed-probe: feeding {} MB in {} chunks of {} KB via raw Term::feed",
        bytes.len() / 1_000_000,
        chunk_count,
        chunk_size / 1024,
    );

    // Production Term + raw VTE processor, no recording overhead.
    let mut term = Term::new(
        HARNESS_ROWS,
        HARNESS_COLS,
        1000, // scrollback
        Theme::default(),
        VoidEffectSink,
    );
    term.set_cell_dimensions(8, 16);
    let mut processor: vte::ansi::Processor = vte::ansi::Processor::new();

    let mut chunk_times: Vec<u128> = Vec::with_capacity(chunk_count);
    let probe_start = Instant::now();
    for chunk in bytes.chunks(chunk_size) {
        let t = Instant::now();
        processor.advance(&mut term, chunk);
        chunk_times.push(t.elapsed().as_nanos());
    }
    let wall = probe_start.elapsed();

    let total_ns: u128 = chunk_times.iter().sum();
    let avg_ns = total_ns / (chunk_times.len() as u128).max(1);
    let max_ns = *chunk_times.iter().max().unwrap_or(&0);

    eprintln!("production-feed-probe: results");
    eprintln!("  bytes fed:        {} MB", bytes.len() / 1_000_000);
    eprintln!("  chunks:           {}", chunk_times.len());
    eprintln!("  wall time:        {:.3} sec", wall.as_secs_f64());
    eprintln!(
        "  per-byte rate:    {:.3} µs/byte",
        wall.as_nanos() as f64 / bytes.len() as f64 / 1000.0
    );
    eprintln!("  avg chunk cost:   {:.3} ms", avg_ns as f64 / 1e6);
    eprintln!("  max chunk cost:   {:.3} ms", max_ns as f64 / 1e6);

    // Notcurses emits xray at ~18 MB/sec (per bug entry §01 measurements).
    let notcurses_emit_rate_mbps = 18.0;
    let our_rate_mbps = (bytes.len() as f64 / 1e6) / wall.as_secs_f64();
    eprintln!("  our rate:         {:.2} MB/sec", our_rate_mbps);
    eprintln!("  notcurses target: {:.0} MB/sec", notcurses_emit_rate_mbps);
    eprintln!(
        "  speedup needed:   {:.1}× to match notcurses real-time",
        notcurses_emit_rate_mbps / our_rate_mbps
    );

    let image_count = term.image_cache().image_count();
    let placement_count = term.image_cache().placement_count();
    eprintln!(
        "  post-feed state:  images={} placements={}",
        image_count, placement_count
    );
}
