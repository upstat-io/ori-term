//! Timing diagnostic for kitty `a=f` (Animate Frame) handler using exact
//! byte patterns captured from notcurses-demo xray PTY trace
//! (`oriterm-perf-last-tap.bin`).
//!
//! Run via:
//! ```text
//! cargo test -p oriterm_core --release \
//!     --lib term::handler::image::kitty::frame::tests::xray_af_timing \
//!     -- --nocapture --ignored
//! ```
//!
//! Marked `#[ignore]` so it does not run in the default test suite — it is
//! a diagnostic harness, not a correctness pin. The realistic xray-shape
//! roundtrip + 14 existing prepare boundary tests cover correctness.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vte::ansi::Processor;

use crate::effect::{QueueingEffectSink, VoidEffectSink};
use crate::image::{ImageData, ImageId, ImageSource};
use crate::term::Term;
use crate::theme::Theme;

/// Exact a=f APC sequence from PTY tap offset 10,243,967:
/// `\e_Ga=f,x=231,y=667,s=11,v=23,i=12699316,X=1,r=2,c=1,q=2;<1350-byte base64 of all-zero RGBA>\e\\`
///
/// 1410 bytes total. Edit frame 2 of image_id 12699316 with an 11×23 pixel
/// cell-sized transparent slice at canvas (231, 667).
const XRAY_AF_BYTES: &[u8] = include_bytes!("xray_af_sample.bin");

/// xray frame canvas dimensions: 999 × 562 RGBA (the image_id that a=f
/// targets — the full kitty sprite frame). Per-cell pixel size 11×23
/// (notcurses cellpxx/cellpxy from the xray run) is encoded inside
/// the a=f byte sequence (s=11, v=23 control keys).
const CANVAS_W: u32 = 999;
const CANVAS_H: u32 = 562;

/// image_id that the captured a=f byte sequence targets (from x=231,
/// y=667 — fits a canvas of CANVAS_W × CANVAS_H scaled to notcurses cell
/// grid).
const IMAGE_ID: u32 = 12699316;

fn build_canvas_bytes() -> Arc<Vec<u8>> {
    Arc::new(vec![0u8; (CANVAS_W * CANVAS_H * 4) as usize])
}

/// Pre-populate the Term's ImageCache with image_id 12699316 plus two
/// animation frames so the a=f Edit-frame-2 path can hit the in-place
/// fast path.
fn setup_term_with_animation() -> Term<VoidEffectSink> {
    let mut term = Term::new(50, 200, 1000, Theme::default(), VoidEffectSink);
    let cache = term.image_cache_mut();
    let initial = ImageData {
        id: ImageId(IMAGE_ID),
        width: CANVAS_W,
        height: CANVAS_H,
        data: build_canvas_bytes(),
        pixel_generation: 0,
        format: crate::image::ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    let frames = vec![build_canvas_bytes(), build_canvas_bytes()];
    let durations = vec![Duration::from_millis(33), Duration::from_millis(33)];
    cache
        .store_animated(initial, frames, durations, Some(0))
        .expect("test setup: store_animated must succeed");
    term
}

/// Run `N` a=f calls + measure timing. Returns (wall_time, kitty_stats).
fn time_af_calls(term: &mut Term<VoidEffectSink>, n: u32) -> (Duration, crate::term::KittyHandlerStats) {
    let mut proc: Processor = Processor::new();
    // Warm-up
    for _ in 0..10 {
        proc.advance(term, XRAY_AF_BYTES);
    }
    let _ = term.take_kitty_handler_stats();

    let start = Instant::now();
    for _ in 0..n {
        proc.advance(term, XRAY_AF_BYTES);
    }
    let elapsed = start.elapsed();
    let stats = term.take_kitty_handler_stats();
    (elapsed, stats)
}

#[test]
#[ignore]
fn xray_af_timing() {
    let mut term = setup_term_with_animation();
    const N: u32 = 1000;
    let (elapsed, stats) = time_af_calls(&mut term, N);

    let per_call_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let f_idx = 5; // KittyAction::Frame discriminant per Term::record_kitty_action_sample
    let f_total_ns = stats.per_action_ns[f_idx];
    let f_calls = stats.per_action_calls[f_idx];
    let per_call_f_ns = if f_calls > 0 {
        f_total_ns / u64::from(f_calls)
    } else {
        0
    };

    eprintln!();
    eprintln!("=== xray a=f timing diagnostic ({N} calls) ===");
    eprintln!("total wall-clock:  {elapsed:?}");
    eprintln!("per call (wall):   {per_call_ns} ns ({:.2} µs)", per_call_ns as f64 / 1000.0);
    eprintln!();
    eprintln!("--- handle_kitty_graphics total (instrumented) ---");
    eprintln!("total kitty ns:    {} ns", stats.total_ns);
    eprintln!("total calls:       {}", stats.total_calls);
    eprintln!("per kitty call:    {} ns ({:.2} µs)",
        if stats.total_calls > 0 { stats.total_ns / u64::from(stats.total_calls) } else { 0 },
        if stats.total_calls > 0 {
            (stats.total_ns / u64::from(stats.total_calls)) as f64 / 1000.0
        } else { 0.0 },
    );
    eprintln!();
    eprintln!("--- per-action Frame (F) bucket ---");
    eprintln!("F calls:           {f_calls}");
    eprintln!("F total ns:        {f_total_ns}");
    eprintln!("F per-call ns:     {per_call_f_ns} ({:.2} µs)", per_call_f_ns as f64 / 1000.0);
    eprintln!();
    eprintln!("--- substep breakdown (cumulative across {N} F calls) ---");
    eprintln!("prepare_ns:        {} ({:.2} µs/call)", stats.prepare_ns,
        stats.prepare_ns as f64 / N as f64 / 1000.0);
    eprintln!("format_ns:         {} ({:.2} µs/call)", stats.format_ns,
        stats.format_ns as f64 / N as f64 / 1000.0);
    eprintln!("store_ns:          {} ({:.2} µs/call)", stats.store_ns,
        stats.store_ns as f64 / N as f64 / 1000.0);
    eprintln!();
}

/// Verify the hypothesis: in production, the canvas Arc is shared with
/// renderer/snapshot pipeline, so `Arc::make_mut` in `try_edit_in_place`
/// clones the 2.25 MB canvas per Edit (~45 µs at DRAM bandwidth).
///
/// This test pre-clones the frame Arc to simulate an outstanding strong
/// ref held by the renderer side, then measures per-call cost. Expected
/// per-call cost: ~45 µs (vs 0.30 µs without extra refs).
#[test]
#[ignore]
fn xray_af_timing_with_shared_canvas_arc() {
    let mut term = setup_term_with_animation();

    // Manufacture a shared-Arc situation: clone the per-frame Arc<Vec>
    // refs and hold them outside the cache to simulate what the GPU
    // renderer / snapshot publisher does in production — keeps refs to
    // canvas data while the IO thread is mutating frames.
    let outstanding_refs: Vec<Arc<Vec<u8>>> = {
        let cache = term.image_cache();
        let frames = cache
            .animation_frame_arcs_for_test(ImageId(IMAGE_ID))
            .expect("test setup: image must have animation frames");
        frames
    };
    eprintln!();
    eprintln!("outstanding Arc refs to canvas frames: {}", outstanding_refs.len());

    const N: u32 = 1000;
    let (elapsed, stats) = time_af_calls(&mut term, N);

    let per_call_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let f_idx = 5;
    let f_total_ns = stats.per_action_ns[f_idx];
    let f_calls = stats.per_action_calls[f_idx];
    let per_call_f_ns = if f_calls > 0 {
        f_total_ns / u64::from(f_calls)
    } else {
        0
    };

    eprintln!("=== xray a=f WITH shared canvas Arc ({N} calls) ===");
    eprintln!("total wall-clock:   {elapsed:?}");
    eprintln!("per call (wall):    {per_call_ns} ns ({:.2} µs)", per_call_ns as f64 / 1000.0);
    eprintln!("F per-call ns:      {per_call_f_ns} ({:.2} µs)", per_call_f_ns as f64 / 1000.0);
    eprintln!();
    eprintln!("hypothesis check: if per-call >> 1µs, Arc::make_mut is cloning");
    eprintln!();

    // Keep outstanding_refs alive so the borrow checker keeps the Arcs
    // valid for the duration of the test. The renderer pattern in prod.
    drop(outstanding_refs);
}

/// Append a=f instead of Edit — drives the slow path through
/// resolve_canvas_bytes which clones the 2.25 MB canvas per call.
/// Generates a synthetic a=f sequence with r=0 (Append).
#[test]
#[ignore]
fn xray_af_timing_append_path() {
    let mut term = setup_term_with_animation();

    // Replace the embedded fixture's r=2 with r=0 to force Append path.
    // The byte sequence stays identical except the r= field.
    // Original control: a=f,x=231,y=667,s=11,v=23,i=12699316,X=1,r=2,c=1,q=2;
    // Modified: replace ",r=2," with ",r=0,"
    let mut append_bytes: Vec<u8> = XRAY_AF_BYTES.to_vec();
    if let Some(pos) = find_subslice(&append_bytes, b",r=2,") {
        append_bytes.splice(pos..pos + 5, b",r=0,".iter().copied());
    } else {
        panic!("expected ',r=2,' in fixture");
    }

    let mut proc: Processor = Processor::new();
    // Warm-up
    for _ in 0..10 {
        proc.advance(&mut term, &append_bytes);
    }
    let _ = term.take_kitty_handler_stats();

    const N: u32 = 100; // Append accumulates frames; smaller N to keep RAM bounded.
    let start = Instant::now();
    for _ in 0..N {
        proc.advance(&mut term, &append_bytes);
    }
    let elapsed = start.elapsed();
    let stats = term.take_kitty_handler_stats();

    let per_call_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let f_idx = 5;
    let f_calls = stats.per_action_calls[f_idx];
    let per_call_f_ns = if f_calls > 0 {
        stats.per_action_ns[f_idx] / u64::from(f_calls)
    } else { 0 };

    eprintln!();
    eprintln!("=== xray a=f APPEND path timing ({N} calls) ===");
    eprintln!("total wall-clock:   {elapsed:?}");
    eprintln!("per call (wall):    {per_call_ns} ns ({:.2} µs)", per_call_ns as f64 / 1000.0);
    eprintln!("F per-call:         {per_call_f_ns} ns ({:.2} µs)", per_call_f_ns as f64 / 1000.0);
    eprintln!();
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Simulate the snapshot publisher: every call re-acquires an Arc ref
/// to the canvas (mimicking a 60 Hz snapshot loop that captures latest
/// state). Each a=f Edit therefore sees Arc strong_count > 1 and
/// Arc::make_mut clones.
#[test]
#[ignore]
fn xray_af_timing_with_per_call_snapshot() {
    let mut term = setup_term_with_animation();
    let mut proc: Processor = Processor::new();
    // Warm-up
    for _ in 0..10 {
        proc.advance(&mut term, XRAY_AF_BYTES);
    }
    let _ = term.take_kitty_handler_stats();

    const N: u32 = 1000;
    let start = Instant::now();
    for _ in 0..N {
        // Snapshot: acquire a fresh Arc clone before each call.
        let _snap = term
            .image_cache()
            .animation_frame_arcs_for_test(ImageId(IMAGE_ID))
            .expect("snapshot");
        proc.advance(&mut term, XRAY_AF_BYTES);
        // _snap is dropped here, releasing the strong ref.
    }
    let elapsed = start.elapsed();
    let stats = term.take_kitty_handler_stats();
    let per_call_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let f_idx = 5;
    let f_calls = stats.per_action_calls[f_idx];
    let per_call_f_ns = if f_calls > 0 {
        stats.per_action_ns[f_idx] / u64::from(f_calls)
    } else { 0 };

    eprintln!();
    eprintln!("=== xray a=f with per-call snapshot ref ({N} calls) ===");
    eprintln!("total wall-clock:   {elapsed:?}");
    eprintln!("per call (wall):    {per_call_ns} ns ({:.2} µs)", per_call_ns as f64 / 1000.0);
    eprintln!("F per-call:         {per_call_f_ns} ns ({:.2} µs)", per_call_f_ns as f64 / 1000.0);
    eprintln!();
}

/// Simulate the rate-limited snapshot publisher cure: snapshot acquires
/// new Arc refs only once per K a=f calls (mimicking a 60 Hz publish gate
/// instead of per-message publish). Inside one batch, Arcs are uniquely
/// held by the cache, so subsequent a=f Edits hit the in-place fast path.
///
/// Per-call cost should converge toward `(clone_cost / K) + fast_path_cost`.
/// With K=10 and clone_cost ~45 µs, per-call should be ~4.5 µs — a 10×
/// reduction vs per-call snapshot.
#[test]
#[ignore]
fn xray_af_timing_with_rate_limited_snapshot() {
    let mut term = setup_term_with_animation();
    let mut proc: Processor = Processor::new();
    for _ in 0..10 {
        proc.advance(&mut term, XRAY_AF_BYTES);
    }
    let _ = term.take_kitty_handler_stats();

    const N: u32 = 1000;
    const BATCH: u32 = 10;
    let start = Instant::now();
    let mut snap_opt: Option<Vec<Arc<Vec<u8>>>> = None;
    for i in 0..N {
        if i % BATCH == 0 {
            drop(snap_opt.take());
            snap_opt = Some(
                term.image_cache()
                    .animation_frame_arcs_for_test(ImageId(IMAGE_ID))
                    .expect("snapshot"),
            );
        }
        proc.advance(&mut term, XRAY_AF_BYTES);
    }
    drop(snap_opt);
    let elapsed = start.elapsed();
    let stats = term.take_kitty_handler_stats();
    let per_call_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let f_idx = 5;
    let f_calls = stats.per_action_calls[f_idx];
    let per_call_f_ns = if f_calls > 0 {
        stats.per_action_ns[f_idx] / u64::from(f_calls)
    } else { 0 };

    eprintln!();
    eprintln!("=== xray a=f with rate-limited snapshot batch={BATCH} ({N} calls) ===");
    eprintln!("total wall-clock:   {elapsed:?}");
    eprintln!("per call (wall):    {per_call_ns} ns ({:.2} µs)", per_call_ns as f64 / 1000.0);
    eprintln!("F per-call:         {per_call_f_ns} ns ({:.2} µs)", per_call_f_ns as f64 / 1000.0);
    eprintln!("expected: ~(clone_cost/{BATCH}) + fast_path = ~4.5 µs/call");
    eprintln!();
}

/// Replay a 306 KB chunk captured from the actual notcurses xray PTY tap
/// at offset ~50 MB (mid-run, post-image-transmit). The chunk contains 171
/// APC commands (85 a=a animation control + 84 a=f frame Edits + 1 a=p
/// place + 1 a=d delete) — representative of ~1 frame's worth of work.
///
/// Measures total wall + per-substep timing so we can identify the
/// dominant cost contributor to the 29 ms median drain cycle observed
/// post-rate-limit cure. Goal per user 2026-05-23: drive drain duration
/// under the 17 ms xray budget so the dropped-frame counter reaches 0.
///
/// Run via:
/// ```text
/// cargo test -p oriterm_core --release \
///     --lib term::handler::image::kitty::frame::tests::xray_drain_chunk \
///     -- --nocapture --ignored
/// ```
const XRAY_DRAIN_CHUNK: &[u8] = include_bytes!("xray_drain_chunk.bin");

fn setup_term_with_drain_images() -> Term<QueueingEffectSink> {
    let mut term = Term::new(50, 200, 1000, Theme::default(), QueueingEffectSink::new());
    let cache = term.image_cache_mut();
    // Three image_ids referenced by the drain chunk, each with 3+ frames
    // so the r=2 Edit-frame path resolves cleanly.
    for image_id in [13537382u32, 13537383, 13537384] {
        let initial = ImageData {
            id: ImageId(image_id),
            width: CANVAS_W,
            height: CANVAS_H,
            data: build_canvas_bytes(),
            pixel_generation: 0,
            format: crate::image::ImageFormat::Rgba,
            source: ImageSource::Direct,
            last_accessed: 0,
            image_number: None,
        };
        let frames = vec![build_canvas_bytes(), build_canvas_bytes(), build_canvas_bytes()];
        let durations = vec![
            Duration::from_millis(33),
            Duration::from_millis(33),
            Duration::from_millis(33),
        ];
        cache
            .store_animated(initial, frames, durations, Some(0))
            .expect("test setup: store_animated must succeed");
    }
    term
}

#[test]
#[ignore]
fn xray_drain_chunk_timing() {
    let mut term = setup_term_with_drain_images();
    let mut proc: Processor = Processor::new();

    for _ in 0..3 {
        proc.advance(&mut term, XRAY_DRAIN_CHUNK);
    }
    let _ = term.take_kitty_handler_stats();

    const N: u32 = 10;
    let start = Instant::now();
    for _ in 0..N {
        proc.advance(&mut term, XRAY_DRAIN_CHUNK);
    }
    let elapsed = start.elapsed();
    let stats = term.take_kitty_handler_stats();

    let per_iter_ms = elapsed.as_secs_f64() * 1000.0 / f64::from(N);
    let bytes_per_iter = XRAY_DRAIN_CHUNK.len();

    eprintln!();
    eprintln!("=== xray drain chunk timing ({N} iterations of {bytes_per_iter}B chunk) ===");
    eprintln!("total wall:        {elapsed:?}");
    eprintln!("per iter wall:     {per_iter_ms:.2} ms");
    eprintln!("xray frame budget: 17.4 ms (57 FPS)");
    eprintln!("budget headroom:   {:.1}× ({:+.2} ms)",
        17.4 / per_iter_ms, 17.4 - per_iter_ms);
    eprintln!();
    eprintln!("--- handle_kitty_graphics aggregate ---");
    eprintln!("total kitty calls:  {}", stats.total_calls);
    eprintln!("total kitty ns:     {} ({:.3} ms total)",
        stats.total_ns, stats.total_ns as f64 / 1e6);
    eprintln!("kitty per call:     {} ns ({:.2} µs)",
        if stats.total_calls > 0 { stats.total_ns / u64::from(stats.total_calls) } else { 0 },
        if stats.total_calls > 0 {
            (stats.total_ns / u64::from(stats.total_calls)) as f64 / 1000.0
        } else { 0.0 });
    eprintln!();
    eprintln!("--- per-action breakdown (cumulative across {N} iterations) ---");
    // Index order from oriterm_core/src/term/handler/image/kitty/mod.rs:135:
    //   0=Query, 1=Transmit, 2=TransmitAndPlace, 3=Place, 4=Delete, 5=Frame, 6=Animate, 7=Compose
    let labels = ["Query", "Transmit", "TransmitAndPlace", "Place", "Delete", "Frame", "Animate", "Compose"];
    for (i, label) in labels.iter().enumerate() {
        let calls = stats.per_action_calls[i];
        let ns = stats.per_action_ns[i];
        if calls > 0 {
            eprintln!("  {label:18} calls={calls:6} total={:.3} ms  per-call={:.2} µs",
                ns as f64 / 1e6,
                ns as f64 / f64::from(calls) / 1000.0);
        }
    }
    eprintln!();
    eprintln!("--- substep breakdown (cumulative) ---");
    eprintln!("  prepare (b64+zlib+fmt): {} ns ({:.3} ms)  per-F-call={:.2} µs",
        stats.prepare_ns,
        stats.prepare_ns as f64 / 1e6,
        if stats.per_action_calls[5] > 0 {
            stats.prepare_ns as f64 / f64::from(stats.per_action_calls[5]) / 1000.0
        } else { 0.0 });
    eprintln!("  format decode:          {} ns ({:.3} ms)",
        stats.format_ns,
        stats.format_ns as f64 / 1e6);
    eprintln!("  store/blit:             {} ns ({:.3} ms)",
        stats.store_ns,
        stats.store_ns as f64 / 1e6);
    eprintln!();
    let kitty_ms = stats.total_ns as f64 / 1e6;
    let total_ms = elapsed.as_secs_f64() * 1000.0;
    eprintln!("--- attribution ---");
    eprintln!("kitty handler total:    {kitty_ms:.3} ms ({:.1}%)",
        kitty_ms / total_ms * 100.0);
    eprintln!("vte parser + other:     {:.3} ms ({:.1}%)",
        total_ms - kitty_ms,
        (total_ms - kitty_ms) / total_ms * 100.0);
    eprintln!();
}

/// Measure cost of `vte::Parser::advance` on a chunk that's almost entirely
/// kitty APC payload bytes. Hypothesis: `advance_apc_string` is called
/// per-byte (no slice fast-path like `advance_ground`), and the per-byte
/// trait dispatch via `Perform::apc_put` consumes ~4 ns/byte. For a
/// 306 KB drain chunk that's ~1.2 ms of pure per-byte VTE traversal,
/// scaling to ~6 ms across a full drain.
///
/// This is the **last** dominant cost contributor inside `handle_bytes`
/// after the perf-tap I/O was eliminated. Goal: identify whether a
/// memchr-based ApcString fast-path could collapse this cost.
#[test]
#[ignore]
fn xray_drain_chunk_vte_per_byte_cost() {
    let mut term = setup_term_with_drain_images();
    let mut proc: Processor = Processor::new();

    for _ in 0..3 {
        proc.advance(&mut term, XRAY_DRAIN_CHUNK);
    }

    const N: u32 = 10;
    let bytes_per_iter = XRAY_DRAIN_CHUNK.len();
    let start = Instant::now();
    for _ in 0..N {
        proc.advance(&mut term, XRAY_DRAIN_CHUNK);
    }
    let elapsed = start.elapsed();

    let per_iter_ns = elapsed.as_nanos() as u64 / u64::from(N);
    let per_byte_ns = per_iter_ns as f64 / bytes_per_iter as f64;

    eprintln!();
    eprintln!("=== VTE per-byte cost ({N} iter × {bytes_per_iter}B) ===");
    eprintln!("total wall:        {elapsed:?}");
    eprintln!("per iter:          {per_iter_ns} ns ({:.2} ms)", per_iter_ns as f64 / 1e6);
    eprintln!("per byte:          {per_byte_ns:.2} ns");
    eprintln!();
    eprintln!("hypothesis: ApcString state's per-byte dispatch (advance_apc_string");
    eprintln!("→ change_state → performer.apc_put) costs ~4 ns/byte without a memchr");
    eprintln!("fast-path like advance_ground. For a 1.5 MB drain that's ~6 ms.");
    eprintln!();
}

#[test]
fn xray_af_fixture_round_trip_smoke() {
    // Smoke test: confirm setup + parse + dispatch path doesn't panic.
    let mut term = setup_term_with_animation();
    let mut proc: Processor = Processor::new();
    proc.advance(&mut term, XRAY_AF_BYTES);
    let stats = term.take_kitty_handler_stats();
    assert!(stats.total_calls > 0, "handle_kitty_graphics must be invoked");
    // Index 5 = Frame per action_label mapping.
    assert!(stats.per_action_calls[5] > 0, "Frame action must be observed");
}
