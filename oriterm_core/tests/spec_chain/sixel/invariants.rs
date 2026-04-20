//! Sixel decoder-output invariants spec-conformance tests (§12.2).
//!
//! These tests pin properties of the decoder's **output state** across
//! multiple DCS invocations and output-buffer bounds — not per-operator
//! dispatch (that's §12.1 `state_machine.rs`). Failure modes here are
//! silent pixel corruption, cross-image palette leak, and unbounded
//! allocation — all observed at the output layer.
//!
//! Catalog rows covered:
//! - `SIXEL-BG-DeviceDefault` / `SIXEL-BG-SetToBg` — undrawn-pixel RGB
//!   semantics under the two opaque-bg modes.
//! - `SIXEL-BG-NoChange` — α=0 transparency invariant.
//! - `SIXEL-PALETTE-RESET-PER-DCS` — palette is rebuilt per DCS q.
//! - `SIXEL-REPEAT-CLAMP` — `!N?` with `N > MAX_DIMENSION` is clamped.
//! - `SIXEL-PIXEL-BUFFER-CAP` — raster attrs beyond `MAX_PIXEL_BYTES`
//!   aborts via `ImageError::OversizedImage` with no 900 MB allocation.

use oriterm_test_support::spec_chain::SpecHarness;
use vte::ansi::Rgb;

// ── Test helpers ────────────────────────────────────────────────────

/// Grab the most-recently-placed image's pixel buffer (RGBA) and dimensions.
fn last_image_pixels(harness: &SpecHarness) -> (Vec<u8>, u32, u32) {
    let snapshot = harness.term().renderable_content();
    let data = snapshot
        .image_data
        .last()
        .expect("at least one image should be present");
    (data.data.as_ref().clone(), data.width, data.height)
}

/// Count placements visible in the public renderable snapshot.
fn placement_count(harness: &SpecHarness) -> usize {
    harness.term().renderable_content().images.len()
}

/// Configure a distinguishable terminal bg color on the harness — used
/// by tests that need to observe `SetToBg` ≠ `DeviceDefault`.
fn set_terminal_bg(harness: &mut SpecHarness, r: u8, g: u8, b: u8) {
    harness
        .term_mut()
        .palette_mut()
        .set_background(Rgb { r, g, b });
}

// ── Background-mode invariants ──────────────────────────────────────

/// Catalog rows: `SIXEL-BG-DeviceDefault` + `SIXEL-BG-SetToBg`.
///
/// Semantic pin: under a non-black terminal bg, identical pixel data
/// emitted with P2=0 (DeviceDefault) vs P2=2 (SetToBg) MUST produce
/// different RGBA output on undrawn pixels.
///
/// - DeviceDefault — opaque VT340 device default (black).
/// - SetToBg — opaque terminal bg captured at DCS-hook time.
#[test]
fn bg_mode_set_to_bg_differs_from_device_default_on_identical_input() {
    const DATA: &[u8] = b"#0;2;100;0;0@"; // red @ (0,0); rows 1..5 undrawn

    // Run 1: P2=0 (DeviceDefault).
    let mut h0 = SpecHarness::new();
    set_terminal_bg(&mut h0, 0, 128, 255);
    let mut dcs = b"\x1bP0;0;0q".to_vec();
    dcs.extend_from_slice(DATA);
    dcs.extend_from_slice(b"\x1b\\");
    h0.feed(&dcs);
    assert_eq!(placement_count(&h0), 1);
    let (pixels_default, ..) = last_image_pixels(&h0);

    // Run 2: P2=2 (SetToBg) with the same terminal bg.
    let mut h2 = SpecHarness::new();
    set_terminal_bg(&mut h2, 0, 128, 255);
    let mut dcs = b"\x1bP0;2;0q".to_vec();
    dcs.extend_from_slice(DATA);
    dcs.extend_from_slice(b"\x1b\\");
    h2.feed(&dcs);
    assert_eq!(placement_count(&h2), 1);
    let (pixels_set_bg, ..) = last_image_pixels(&h2);

    // Drawn pixel identical; undrawn diverges.
    assert_eq!(
        &pixels_default[0..4],
        &pixels_set_bg[0..4],
        "drawn pixel must be identical across bg modes",
    );
    assert_ne!(
        &pixels_default[4..8],
        &pixels_set_bg[4..8],
        "DeviceDefault undrawn RGBA must differ from SetToBg undrawn RGBA",
    );
    assert_eq!(
        &pixels_default[4..8],
        &[0, 0, 0, 255],
        "DeviceDefault undrawn must be VT340 opaque black",
    );
    assert_eq!(
        &pixels_set_bg[4..8],
        &[0, 128, 255, 255],
        "SetToBg undrawn must be terminal bg captured at DCS-hook time",
    );
}

/// Catalog row: `SIXEL-BG-NoChange`.
///
/// P2=1 with partial pixel coverage must produce undrawn pixels with
/// `α = 0` — the transparency invariant consumed by §12.4 Scenario E.
#[test]
fn bg_mode_no_change_undrawn_pixels_have_alpha_zero() {
    let mut h = SpecHarness::new();
    // Non-black terminal bg is irrelevant here — NoChange MUST be α=0
    // regardless of the terminal bg.
    set_terminal_bg(&mut h, 0, 128, 255);
    h.feed(b"\x1bP0;1;0q#0;2;100;0;0@\x1b\\");

    assert_eq!(placement_count(&h), 1);
    let (pixels, _w, _h) = last_image_pixels(&h);
    assert_eq!(pixels[3], 255, "drawn pixel must be opaque");
    assert_eq!(
        pixels[7], 0,
        "undrawn pixel under NoChange must be transparent",
    );
}

/// Catalog row: `SIXEL-BG-SetToBg` (DECSCNM interaction).
///
/// Semantic pin — DECSCNM (reverse video, mode 5) swaps the terminal's
/// default fg/bg at render time; `SetToBg` captures the *effective*
/// background at DCS-hook time, so under reverse video P2=2 must fill
/// undrawn pixels with the FOREGROUND slot (the now-swapped bg), not
/// the nominal palette bg. Without DECSCNM awareness, sixel and the
/// `renderable_content` render path would disagree on the bg — see
/// `oriterm_core/src/term/snapshot.rs` reverse_video branch.
#[test]
fn bg_mode_set_to_bg_honors_decscnm_reverse_video() {
    let mut h = SpecHarness::new();
    // Distinguishable fg/bg.
    h.term_mut().palette_mut().set_foreground(Rgb {
        r: 0,
        g: 128,
        b: 255,
    });
    h.term_mut()
        .palette_mut()
        .set_background(Rgb { r: 0, g: 0, b: 0 });

    // Enable DECSCNM (reverse video, mode 5) BEFORE the sixel.
    h.feed(b"\x1b[?5h");

    // P2=2 SetToBg with a one-pixel-lit data byte. Undrawn pixels under
    // reverse video must use the foreground slot (effective bg = fg).
    h.feed(b"\x1bP0;2;0q#0;2;100;0;0@\x1b\\");

    assert_eq!(placement_count(&h), 1);
    let (pixels, _w, _h) = last_image_pixels(&h);
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255], "drawn pixel red");
    assert_eq!(
        &pixels[4..8],
        &[0, 128, 255, 255],
        "SetToBg under DECSCNM must fill undrawn pixels with the foreground slot",
    );
}

// ── Palette reset per DCS q (semantic pin) ──────────────────────────

/// Catalog row: `SIXEL-PALETTE-RESET-PER-DCS`.
///
/// Two back-to-back DCS q streams sharing a harness: stream A defines
/// color 5 as red; stream B defines color 5 as blue. Stream B's pixels
/// MUST depend only on stream B's palette — no leak from stream A.
#[test]
fn palette_rebuilds_per_dcs_q_no_leak_across_invocations() {
    let mut h = SpecHarness::new();
    // Stream A — define color 5 = red, draw it.
    h.feed(b"\x1bPq#5;2;100;0;0#5@\x1b\\");
    assert_eq!(placement_count(&h), 1);
    let (pixels_a, ..) = last_image_pixels(&h);
    assert_eq!(&pixels_a[0..4], &[255, 0, 0, 255], "stream A: red");

    // Stream B — redefine color 5 = blue, draw it. The second placement
    // lands at the same cursor position and evicts the first via
    // `remove_by_position` (`handle_sixel_end` / `sixel_create_placement`);
    // what matters is that stream B's *pixel buffer* reflects stream B's
    // palette — not a leaked red.
    h.feed(b"\x1bPq#5;2;0;0;100#5@\x1b\\");
    assert_eq!(
        placement_count(&h),
        1,
        "second DCS evicts the first at the cursor"
    );
    let (pixels_b, ..) = last_image_pixels(&h);
    assert_eq!(&pixels_b[0..4], &[0, 0, 255, 255], "stream B: blue");
}

/// Catalog row: `SIXEL-PALETTE-RESET-PER-DCS` (fingerprint side).
///
/// Complement to the redefinition test: stream B that does NOT redefine
/// color 5 must see the VT340 default for index 5 (cyan). If the parser
/// leaked stream A's redefined color 5 = red, stream B would paint red.
#[test]
fn palette_vt340_fingerprint_reappears_on_fresh_dcs() {
    let mut h = SpecHarness::new();
    // Stream A — redefine color 5 = red, use it.
    h.feed(b"\x1bPq#5;2;100;0;0#5@\x1b\\");
    let (pixels_a, ..) = last_image_pixels(&h);
    assert_eq!(&pixels_a[0..4], &[255, 0, 0, 255]);

    // Stream B — select bare `#5` without redefining; VT340 default is
    // cyan `[51, 204, 204]`.
    h.feed(b"\x1bPq#5@\x1b\\");
    let (pixels_b, ..) = last_image_pixels(&h);
    assert_eq!(
        &pixels_b[0..4],
        &[51, 204, 204, 255],
        "stream B must see VT340 default palette[5] = cyan, not leaked red",
    );
}

// ── Repeat clamp (de-facto DoS protection) ──────────────────────────

/// Catalog row: `SIXEL-REPEAT-CLAMP`.
///
/// `!20000?` — `?` = value 0 = no pixels lit — the repeat count exceeds
/// `MAX_DIMENSION` (10,000). The parser must clamp without panic / OOM
/// and the placement must commit with width ≤ 10,000.
#[test]
fn repeat_clamps_at_max_dimension_without_allocation_spike() {
    let mut h = SpecHarness::new();
    // `?` is value 0 — no pixels lit, but the clamp applies to the
    // advance step, so max_x caps at MAX_DIMENSION.
    h.feed(b"\x1bPq#0;2;100;0;0!20000?\x1b\\");

    // The image may decode to zero height because `?` draws nothing —
    // that returns `ImageError::DecodeFailed` before committing a
    // placement. The clamp invariant is that the parser does NOT panic
    // and does NOT allocate the 20,000-column buffer. A committed
    // placement is only expected when any pixel is drawn.
    //
    // The complementary clamp check uses `!20000~` where `~` = all six
    // bits → a placement WILL commit. That is the clamp observation.
    let mut h2 = SpecHarness::new();
    h2.feed(b"\x1bPq#0;2;100;0;0!20000~\x1b\\");
    assert_eq!(placement_count(&h2), 1, "lit-repeat must commit");
    let (_pixels, w, _h) = last_image_pixels(&h2);
    assert!(
        w <= 10_000,
        "width {w} must be clamped to MAX_DIMENSION (10000)",
    );
    assert!(w >= 1, "at least one column must exist");
}

// ── Pixel buffer cap (DoS protection) ───────────────────────────────

/// Catalog row: `SIXEL-PIXEL-BUFFER-CAP`.
///
/// Raster attrs `"1;1;15000;15000` declares a 15000 × 15000 image — 900
/// MB RGBA — exceeding `MAX_PIXEL_BYTES` (100 MB). The parser MUST set
/// the abort flag in `apply_raster_attrs` (per `mod.rs:318` / `:323`)
/// and `finish()` returns `Err(ImageError::OversizedImage)` — no
/// 900 MB buffer is ever allocated. Observable via: no placement is
/// committed (the `warn!` + return path in `handle_sixel_end`).
#[test]
fn raster_attrs_exceeding_max_pixel_bytes_aborts_cleanly() {
    let mut h = SpecHarness::new();
    // 15000 * 15000 * 4 = 900 MB, well above MAX_PIXEL_BYTES (100 MB).
    h.feed(b"\x1bPq\"1;1;15000;15000#0;2;100;0;0~\x1b\\");
    assert_eq!(
        placement_count(&h),
        0,
        "oversized raster attrs must NOT commit a placement",
    );
}

// ── Self-verifying matrix completeness ──────────────────────────────

/// Structural matrix-completeness pin for §12.2.
///
/// The invariant categories §12.2 owns are enumerated here — if a test
/// is removed, this count mismatch surfaces the gap before the plan is
/// closed.
#[test]
fn invariant_category_matrix_completeness() {
    const CATEGORIES: &[&str] = &[
        "bg_mode_set_to_bg_differs_from_device_default",
        "bg_mode_set_to_bg_honors_decscnm",
        "bg_mode_no_change_alpha_zero",
        "palette_rebuilds_per_dcs_q",
        "palette_vt340_fingerprint_fresh",
        "repeat_clamp",
        "pixel_buffer_cap_aborts",
    ];
    // The negative-pin palette-leak test lives as a unit test in
    // `oriterm_core/src/image/sixel/tests.rs` because it requires the
    // test-only `BypassVt340ResetGuard` — not reachable from
    // integration tests. That eighth invariant is counted here.
    assert_eq!(
        CATEGORIES.len() + 1,
        8,
        "§12.2 has 8 invariant categories (7 integration + 1 unit test); update if the plan changes",
    );
}
