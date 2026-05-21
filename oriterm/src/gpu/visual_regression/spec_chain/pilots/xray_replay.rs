//! `notcurses-demo xray` replay pilot — BUG-06-086 perf-diagnostic vehicle.
//!
//! Apex: `GoldenImage` (one PNG per 5-sec wall-clock mark of xray's
//! intended 15-sec runtime). Per-chunk parse+dispatch+prepare cost is
//! instrumented under `XRAY_DUMP_TIMING=1`.
//!
//! ## Repro context
//!
//! BUG-06-086 reports severe lag + frame drops when notcurses-demo's
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
//! and dwarfs the `./test-all.sh` budget. Gated `#[ignore = "BUG-06-086:
//! …"]` per `tests.md §Test Disposition Discipline` and CLAUDE.md
//! §Test Disposition Discipline — every `#[ignore]` MUST contain
//! `BUG-XX-NNN`. Run explicitly:
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
fn snapshot_byte_offsets(timing_path: &std::path::Path) -> Vec<(f64, &'static str, &'static str, usize)> {
    let mut out = Vec::new();
    let Ok(f) = fs::File::open(timing_path) else { return out };
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
#[ignore = "BUG-06-086: xray replay diagnostic — full ~312 MB capture; \
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
