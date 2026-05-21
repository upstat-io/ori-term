//! Kitty graphics animation frame-transition pilot — GpuInstance-apex pin.
//!
//! Apex: `GpuInstance` (NOT `GoldenImage` — see §Pixel-level golden deferred).
//!
//! Catalog row: `KG-ANIMATE-RUN`
//!
//! Closes the §13.3 gap that the existing cache-level tests at
//! `oriterm_core/tests/spec_chain/kitty/animation.rs:399,427` do not cover:
//! the snapshot + FrameInput + GPU-prepare pipeline must observe the
//! cache-level frame transition. When `Term::advance_animations(now)`
//! advances the cache past a frame-gap deadline, the next snapshot must
//! see `current_frame = 1` and the GPU prepare phase must continue to
//! emit an image quad for the visible placement.
//!
//! ## Pixel-level golden deferred
//!
//! The §13.3 plan called for a pixel-exact golden-image comparison
//! distinguishing frame 0 from frame 1 in the rendered texture. That
//! comparison cannot land here today: the underlying GPU pipeline for
//! kitty images emits image quads to the GpuInstance buffer but does
//! NOT produce visible pixel-level kitty rendering — confirmed at
//! §13.6 author-time (2026-05-14) by inspecting the existing
//! `kitty_minimal` pilot's bootstrapped golden alongside this pilot's
//! frame-0 / frame-1 candidate goldens: all three PNGs are byte-identical
//! (800×528 with 4 distinct pixel values — none of them the placement's
//! red or blue), proving the kitty pipeline below the quad-emission
//! level is broken or untested. The §13.3 pixel-level pin is therefore
//! blocked-by §13.6's "Cure kitty quad → texture pipeline gap" item
//! and will land alongside §13.6's GPU pilots once the upstream
//! rendering chain produces visible pixels for kitty placements.
//!
//! What this pilot DOES pin today:
//!   1. Cache transitions correctly (`current_frame` 0 → 1 after a
//!      single `advance_animations(t_far_future)` call) — diagnostic
//!      assertions inline below catch regressions where the cache
//!      silently fails to advance (which would let the test pass for
//!      the wrong reason).
//!   2. Snapshot extraction picks up the placement (FrameInput rung).
//!   3. GPU prepare emits at least one image quad for the visible
//!      placement (GpuInstance rung). Without this, the rendered
//!      texture would have no image-quad input at all, regardless of
//!      the pixel-level rendering bug.
//!   4. Must-not-advance assertion: no advance_animations between
//!      renders → cache stays at frame 0; FrameInput + GpuInstance
//!      metadata identical between renders.
//!
//! Wall-clock-free — drives the canonical clock-injection seam
//! `Term::advance_animations(now)` with synthesized `Instant` values.
//! No `thread::sleep`, no frame-count timing, no fixed-deadline budget
//! assertions. Real time never elapses in the test; only the synthetic
//! `Instant` passed to `advance_animations` matters.

use std::time::{Duration, Instant};

use base64::Engine;
use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::{
    FrameInputExpectation, GpuInstanceExpectation, RungName, ScenarioExpectations,
};

use super::super::visual_harness::VisualSpecHarness;

/// Catalog row pinned by this pilot — the rendered-frame side of the
/// `KG-ANIMATE-RUN` chain. The cache-level rung is pinned at
/// `animation.rs:399`; this row's `texture-render` apex lands here.
const CATALOG_ROW: &str = "KG-ANIMATE-RUN";

/// Image id used across the test bytes.
const IMAGE_ID: u32 = 99;

/// Per-frame gap in milliseconds. Sent via `z=<gap_ms>` on the `a=f`
/// command. The promotion-from-static path applies the same gap to BOTH
/// frame 0 (the base) and frame 1 (the appended frame), per
/// `oriterm_core/src/image/cache/animation.rs:222` (`durations = vec![gap, gap]`).
const FRAME_GAP_MS: u64 = 100;

/// Drive a 2-frame kitty animation through the full visual chain, observe
/// the rendered texture at frame 0, then advance the cache clock past the
/// frame-gap deadline and assert the rendered texture observes the
/// transition to frame 1.
///
/// Asserts within the same test:
///
/// 1. **Initial state (frame 0)**: a=T transmit + a=f frame-append +
///    a=a,s=3 run-mode produces a 2-frame animation; the cache reports
///    `current_frame = 0, total_frames = 2, paused = false`. Initial
///    render observes the FrameInput placement + GPU emits ≥1 image
///    quad.
///
/// 2. **Must-not-advance assertion (cache untouched between renders)**:
///    re-rendering without calling `advance_animations` keeps the cache
///    at `current_frame = 0` and produces the same FrameInput +
///    GpuInstance shape — proves the GPU pipeline does NOT auto-advance
///    and the cache is the SSOT for the current frame.
///
/// 3. **Positive pin (cache transition observed through snapshot +
///    FrameInput + GPU prepare)**: a single
///    `advance_animations(t_far_future)` call advances the cache past
///    the 100ms deadline; `current_frame` flips 0 → 1; the subsequent
///    snapshot extraction picks up the new placement state; FrameInput
///    + GpuInstance rungs continue to observe the placement. The
///    pixel-level texture-render comparison is deferred per the
///    §Pixel-level golden deferred section in the file docs.
#[test]
fn kitty_animation_frame_transition_observed_at_render_apex() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Build expectations for the FrameInput + GpuInstance rungs. The
    // GoldenImage rung is intentionally omitted — see the file-level
    // §Pixel-level golden deferred section. The TextureRender rung is
    // also omitted because its sole asserted invariant
    // (`min_non_zero_pixels: Some(1)`) is satisfied by the deterministic
    // palette background regardless of whether the kitty placement
    // renders, providing no useful signal here.
    let expectations = ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
        ..ScenarioExpectations::default()
    };

    drive_frame_0_to_pixels(&mut harness, &expectations);
    drive_frame_transition_to_frame_1(&mut harness);
    assert_visible_color_change_at_apex(&mut harness, &expectations);
}

/// Phase 1 helper: feed the 2-frame kitty animation byte stream, assert
/// the cache observes the initial frame-0 state, render twice without
/// advancing the clock, and clamp the must-not-advance invariant
/// (current_frame stays at 0 across renders without an explicit
/// `advance_animations` call).
///
/// Pins:
///   - a=T + a=f + a=a,s=3 promotes the static image to a 2-frame
///     animation with `current_frame = 0, total_frames = 2, paused = false`.
///   - Initial render observes FrameInput + emits ≥1 GPU image quad.
///   - Re-render without cache mutation leaves `current_frame = 0` and
///     produces the same FrameInput + GpuInstance shape — the cache is
///     the SSOT for current_frame; rendering does not advance it.
fn drive_frame_0_to_pixels(harness: &mut VisualSpecHarness, expectations: &ScenarioExpectations) {
    feed_animation_byte_stream(harness);
    assert_initial_animation_cache_state(harness);
    assert_renders_dont_advance_cache(harness, expectations);
}

/// Feed the a=T + a=f + a=a,s=3 byte sequence that promotes the static
/// image to a 2-frame animation with a 100ms gap on both frames.
///
/// Byte stream:
///   - a=T transmits + places frame 0 (red) at cursor.
///   - a=f,z=100 appends frame 1 (blue) and sets a 100ms gap on BOTH
///     frames (the promotion-from-static path applies the same gap to
///     frame 0 and frame 1 per cache/animation.rs:222).
///   - a=a,s=3 sets run-mode and anchors `frame_starts[id]` to the feed
///     instant — required so a single `advance_animations` call past
///     the deadline produces the transition (without s=3, the first
///     advance_animations call would only initialize `frame_starts` and
///     produce no transition).
///   - q=2 suppresses replies — this pilot pins the render path, not
///     the reply path (KG-RESPONSE-* rows pin that separately).
fn feed_animation_byte_stream(harness: &mut VisualSpecHarness) {
    let encoder = base64::engine::general_purpose::STANDARD;

    // Frame 0: solid red 4x4 RGBA (64 bytes). The base image transmitted
    // via a=T at cursor position (becomes the static image, then frame 0
    // of the promoted animation).
    let red_payload: Vec<u8> = [0xFFu8, 0x00, 0x00, 0xFF].repeat(16);
    let red_b64 = encoder.encode(&red_payload);

    // Frame 1: solid blue 4x4 RGBA — visually distinguishable from frame 0
    // so the golden PNGs differ in 16 pixels (4x4 image footprint) and the
    // pixel-exact comparison detects the transition.
    let blue_payload: Vec<u8> = [0x00u8, 0x00, 0xFF, 0xFF].repeat(16);
    let blue_b64 = encoder.encode(&blue_payload);

    let transmit = format!("\x1b_Ga=T,i={IMAGE_ID},f=32,s=4,v=4,q=2;{red_b64}\x1b\\");
    let frame =
        format!("\x1b_Ga=f,i={IMAGE_ID},f=32,s=4,v=4,z={FRAME_GAP_MS},q=2;{blue_b64}\x1b\\");
    let animate = format!("\x1b_Ga=a,i={IMAGE_ID},s=3,q=2\x1b\\");

    harness.core_mut().feed(transmit.as_bytes());
    harness.core_mut().feed(frame.as_bytes());
    harness.core_mut().feed(animate.as_bytes());
}

/// Diagnostic invariant 1: cache observes the 2-frame animation in the
/// initial state.
///
/// Catches the case where the byte stream silently fails to construct
/// the animation (parser fallback, placement rejection, image_id
/// resolution miss) — without this assertion, an empty render would
/// still pass the texture floor guard and produce identical goldens
/// regardless of the cache state.
fn assert_initial_animation_cache_state(harness: &VisualSpecHarness) {
    let state = harness
        .core()
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(IMAGE_ID))
        .expect(
            "kitty animation state for IMAGE_ID must exist after a=T + a=f + a=a,s=3 — \
             indicates the byte stream did not promote the static image to animated",
        );
    assert_eq!(
        state.total_frames, 2,
        "promotion from static via a=f must produce 2 frames (base + appended)"
    );
    assert_eq!(
        state.current_frame, 0,
        "initial current_frame must be 0 (base, red)"
    );
    assert!(
        !state.paused,
        "a=a,s=3 must set paused = false so advance_animations advances on deadline"
    );
}

/// Must-not-advance assertion: render twice with no `advance_animations`
/// call between renders, then clamp `current_frame = 0`.
///
/// The cache is the SSOT for current_frame; without an explicit
/// `advance_animations` call, current_frame stays at 0. The FrameInput +
/// GpuInstance shape must be identical between renders. The cache-level
/// test at `oriterm_core/tests/spec_chain/kitty/animation.rs:427` covers
/// the pure-cache version of this pin; this clamps the additional
/// invariant that "rendering does not advance the cache".
fn assert_renders_dont_advance_cache(
    harness: &mut VisualSpecHarness,
    expectations: &ScenarioExpectations,
) {
    // Render 1: initial state, current_frame = 0 (red). The cache has not
    // observed any timer tick yet; current_frame is whatever
    // `add_animation_frame` initialized (0 = the base from a=T).
    let results = harness.render_visual_rungs(CATALOG_ROW, expectations);
    assert_rungs_pass(&results, "initial frame-0 render");
    assert_rung_chain_complete(&results);

    // Render 2: re-render without any cache mutation between renders.
    let results_unchanged = harness.render_visual_rungs(CATALOG_ROW, expectations);
    assert_rungs_pass(
        &results_unchanged,
        "must-not-advance assertion (no advance_animations between renders) — \
         FrameInput + GpuInstance shape should be IDENTICAL to render 1",
    );

    let state = harness
        .core()
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(IMAGE_ID))
        .expect("animation state must still exist after re-render");
    assert_eq!(
        state.current_frame, 0,
        "must-not-advance assertion: re-rendering must NOT advance current_frame; \
         got {} after two renders with no advance_animations call",
        state.current_frame
    );
}

/// Phase 2 helper: drive the canonical clock-injection seam
/// `Term::advance_animations(now)` with a synthesized far-future
/// `Instant`, then clamp the cache transition (`current_frame` 0 → 1).
///
/// Pins:
///   - `advance_animations(t + 10s)` past the 100ms gap deadline returns
///     `Some(next_deadline)` — animation is still running (loop_count =
///     infinite), so a NEXT-frame deadline is scheduled.
///   - `current_frame` flips from 0 to 1 after the advance — catches the
///     case where the call returns a deadline but does NOT advance the
///     cache (e.g., frame_starts never anchored, or elapsed < frame_dur
///     due to a clock-arithmetic bug).
fn drive_frame_transition_to_frame_1(harness: &mut VisualSpecHarness) {
    // Advance the cache clock past the 100ms frame-gap deadline. Using
    // `Instant::now() + 10s` provides a comfortable margin above the
    // 100ms deadline + any time consumed by render 1 + render 2 since
    // `frame_starts[id]` was anchored at the a=a,s=3 feed instant. The
    // synthesized future Instant is the canonical clock-injection
    // seam — no `thread::sleep`, no real time elapses in the test.
    let advance_to = Instant::now() + Duration::from_secs(10);
    let deadline = harness.core_mut().term_mut().advance_animations(advance_to);
    assert!(
        deadline.is_some(),
        "advance_animations past frame-gap deadline must return Some(deadline) \
         for the NEXT frame transition (animation still running, loop_count = infinite)"
    );

    // Diagnostic invariant 2: cache transitioned. Catches the case where
    // advance_animations returns a deadline (animation still running) but
    // does NOT actually advance current_frame — e.g., if frame_starts was
    // not anchored, or if elapsed < frame_dur due to some clock-arithmetic
    // bug. Without this assertion, the test would pass for the wrong
    // reason: the downstream FrameInput + GpuInstance rungs would see
    // the same placement state as before the advance, but the test
    // would attribute the pass to a successful transition.
    let state = harness
        .core()
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(IMAGE_ID))
        .expect("animation state must still exist after advance_animations");
    assert_eq!(
        state.current_frame, 1,
        "advance_animations(t + 10s) past the 100ms gap deadline must advance \
         current_frame from 0 to 1; got {} — indicates either frame_starts was \
         never anchored or the deadline arithmetic is broken",
        state.current_frame
    );
}

/// Phase 3 helper: render at the post-transition apex (current_frame = 1)
/// and assert the FrameInput + GpuInstance rungs continue to observe the
/// placement.
///
/// The helper is named for the §13.3 positive-pin intent — the visible
/// color change from red (frame 0) to blue (frame 1) at the rendered
/// texture apex. Pixel-exact comparison is deferred per the file-level
/// §Pixel-level golden deferred section, so this helper currently pins
/// the GpuInstance apex only (image quad emission survives the cache
/// transition). The naming is preserved as the load-bearing anchor for
/// when §13.6 cures the kitty quad → texture pipeline and the
/// pixel-level pin lands.
fn assert_visible_color_change_at_apex(
    harness: &mut VisualSpecHarness,
    expectations: &ScenarioExpectations,
) {
    // Render 3: post-transition state, current_frame = 1 (blue). The
    // snapshot extraction must pick up the new placement state; the GPU
    // pipeline must continue to emit an image quad (proves the
    // placement was not silently dropped when current_frame advanced).
    // Pixel-level pin (different rendered texture for frame 1) is
    // deferred per the file-level §Pixel-level golden deferred section.
    let results_after_transition = harness.render_visual_rungs(CATALOG_ROW, expectations);
    assert_rungs_pass(
        &results_after_transition,
        "frame-1 render after advance_animations past deadline — \
         FrameInput + GpuInstance must continue to observe the placement \
         after cache transitions current_frame to 1",
    );
    assert_rung_chain_complete(&results_after_transition);
}

/// Assert every rung in `results` passed; on failure include the rung
/// name + failure message + caller-supplied context.
fn assert_rungs_pass(results: &[oriterm_test_support::spec_chain::RungResult], context: &str) {
    for r in results {
        assert!(
            r.passed,
            "{context}: rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no failure message)")
        );
    }
}

/// Diagnostic probe: feed the kitty_minimal byte sequence and print
/// what's in the snapshot, FrameInput, and rendered pixels. Used to
/// narrow the §13.6 prerequisite "Cure kitty quad → texture pipeline
/// gap" by isolating which stage drops the image data.
#[test]
#[ignore = "BUG-06-082: diagnostic probe (not a regression test) — run manually via `cargo test -- --ignored probe_kitty_pipeline_stages`"]
fn probe_kitty_pipeline_stages() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    harness
        .core_mut()
        .feed(b"\x1b_Gf=32,s=1,v=1,a=T,i=1,q=2;/wAA/w==\x1b\\");

    let term = harness.core().term();
    let content = term.renderable_content();

    eprintln!("[probe] content.images: {} entries", content.images.len());
    for img in &content.images {
        eprintln!(
            "[probe]   placement: id={:?} pos=({},{}) size=({}x{}) uv=({},{},{},{}) z={}",
            img.image_id,
            img.viewport_x,
            img.viewport_y,
            img.display_width,
            img.display_height,
            img.source_x,
            img.source_y,
            img.source_w,
            img.source_h,
            img.z_index,
        );
    }
    eprintln!(
        "[probe] content.image_data: {} entries",
        content.image_data.len(),
    );
    for d in &content.image_data {
        eprintln!(
            "[probe]   data: id={:?} {}x{} bytes={} first4={:?}",
            d.id,
            d.width,
            d.height,
            d.data.len(),
            &d.data[..4.min(d.data.len())],
        );
    }
    eprintln!(
        "[probe] content.cells: {} cells, cols={} lines={}",
        content.cells.len(),
        content.cols,
        content.lines,
    );

    // Drop the immutable borrow on term/content before mutating harness.
    drop(content);

    // Drive the visual pipeline through FrameInput + GpuInstance rungs to
    // verify the image quad reaches the prepared frame.
    let expectations = ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(1)),
        ..ScenarioExpectations::default()
    };
    let results = harness.render_visual_rungs("KG-PROBE", &expectations);
    for r in &results {
        eprintln!(
            "[probe] rung {:?}: passed={} failure={:?}",
            r.rung_name, r.passed, r.failure,
        );
    }

    let (below, above) = harness.prepared_image_quad_counts();
    eprintln!("[probe] prepared.image_quads: below={below} above={above}");

    // Probe 3: dump rendered pixels — look for the red image bytes.
    if let Some((pixels, w, h)) = harness.last_rendered_pixels() {
        eprintln!("[probe] rendered: {w}x{h}, {} bytes", pixels.len());
        // Scan the top-left 16x32 px region (where the 8x16 image quad sits).
        let mut distinct = std::collections::HashSet::new();
        for y in 0..32.min(h as usize) {
            for x in 0..16.min(w as usize) {
                let off = (y * w as usize + x) * 4;
                if off + 4 <= pixels.len() {
                    let rgba = (
                        pixels[off],
                        pixels[off + 1],
                        pixels[off + 2],
                        pixels[off + 3],
                    );
                    distinct.insert(rgba);
                }
            }
        }
        let mut sorted: Vec<_> = distinct.into_iter().collect();
        sorted.sort();
        eprintln!(
            "[probe] top-left 16x32 px region distinct pixels: {} {:?}",
            sorted.len(),
            sorted,
        );
        // FULL scan: any red pixel anywhere in the rendered output?
        let mut red_count = 0;
        let mut first_red_pos = None;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let off = (y * w as usize + x) * 4;
                if off + 4 <= pixels.len() {
                    let r = pixels[off];
                    let g = pixels[off + 1];
                    let b = pixels[off + 2];
                    if r > 100 && g < 50 && b < 50 {
                        red_count += 1;
                        if first_red_pos.is_none() {
                            first_red_pos = Some((x, y, r, g, b, pixels[off + 3]));
                        }
                    }
                }
            }
        }
        eprintln!(
            "[probe] full-scan red-ish (r>100, g<50, b<50): {red_count} pixels, first={first_red_pos:?}",
        );
    } else {
        eprintln!("[probe] last_rendered_pixels: None");
    }
}

/// Assert the visual-rung chain ran to completion. `render_visual_rungs`
/// always drives all 4 rungs (FrameInput, GpuInstance, TextureRender,
/// GoldenImage); rungs with no expectations set return a no-op pass.
/// This pilot asserts only on FrameInput + GpuInstance (TextureRender +
/// GoldenImage are no-ops here per the §Pixel-level golden deferred
/// section). Early fail-fast on FrameInput or GpuInstance would produce
/// fewer than 4 results — the count assertion catches that case.
fn assert_rung_chain_complete(results: &[oriterm_test_support::spec_chain::RungResult]) {
    assert_eq!(
        results.len(),
        4,
        "render_visual_rungs should produce exactly 4 results \
         (FrameInput, GpuInstance, TextureRender, GoldenImage); got: {:?} — \
         fewer than 4 means a fail-fast on an earlier rung",
        results.iter().map(|r| r.rung_name).collect::<Vec<_>>()
    );
    assert_eq!(
        results.last().unwrap().rung_name,
        RungName::GoldenImage,
        "last rung should be GoldenImage"
    );
}
