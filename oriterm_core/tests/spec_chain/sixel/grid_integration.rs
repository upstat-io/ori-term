//! Sixel grid-integration spec-conformance tests (§12.3, non-GPU portion).
//!
//! Drives DECSET/DECRST 80 (`SIXEL_SCROLLING`) and DECSET/DECRST 8452
//! (`SIXEL_CURSOR_RIGHT`) through the VTE mode-handler seam and observes
//! the cursor-advance + placement-creation behavior at
//! `Term::sixel_create_placement` in `oriterm_core/src/term/handler/
//! image/sixel.rs`. Pins §12.3 non-GPU checkboxes:
//! `SIXEL-MODE-80` (SIXEL_SCROLLING on/off), `SIXEL-MODE-8452`
//! (SIXEL_CURSOR_RIGHT), placement-creation attributes, orphan cleanup.
//!
//! Negative `z_index` (§11 occlusion rung) and golden-image z-order
//! live in GPU-crate pilots per `.claude/rules/crate-boundaries.md`
//! (§12.3 plan body: "grid-state assertions stay in `oriterm_core`,
//! pixel-golden assertions stay in `oriterm`").

use oriterm_test_support::spec_chain::SpecHarness;
use oriterm_test_support::spec_chain::sixel_fixtures::{
    dcs_n_bands_tall, dcs_n_cols_wide, placement_count,
};

// Test helpers.

/// Return the cursor's (line, col) on the active grid.
fn cursor_line_col(harness: &SpecHarness) -> (usize, usize) {
    let grid = harness.term().grid();
    (grid.cursor().line(), grid.cursor().col().0)
}

// SIXEL_SCROLLING (mode 80) — default ON.

/// Catalog row: `SIXEL-MODE-80` (default-on path).
///
/// Default harness: `SIXEL_SCROLLING` is in `TermMode::default()`. A
/// sixel image that spans two cell-rows (24 pixels tall, harness cell
/// height = 16) commits ONE placement and the cursor advances by
/// `rows.saturating_sub(1) == 1` linefeed — from (line=0, col=0) to
/// (line=1, col=0). Pins `sixel_create_placement`'s
/// `else if sixel_scrolling` branch in
/// `oriterm_core/src/term/handler/image/sixel.rs`.
#[test]
fn sixel_scrolling_default_on_advances_cursor_below_image() {
    let mut h = SpecHarness::new();
    // 4 sixel bands = 24 pixels tall; 24 / 16 cell_h = 2 cell-rows.
    h.feed(&dcs_n_bands_tall(4));

    assert_eq!(placement_count(&h), 1);
    assert_eq!(
        cursor_line_col(&h),
        (1, 0),
        "SIXEL_SCROLLING default ON + multi-row image → cursor advances \
         one linefeed per `rows.saturating_sub(1)`",
    );
}

/// Catalog row: `SIXEL-MODE-80` (off path).
///
/// DECRST 80 disables `SIXEL_SCROLLING`. With `SIXEL_CURSOR_RIGHT` also
/// off, `sixel_create_placement`'s cursor-advance branch falls to the
/// no-op `else` arm — cursor stays at pre-placement position even for
/// a multi-row image.
#[test]
fn sixel_scrolling_off_via_decrst_80_keeps_cursor_at_home() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?80l"); // DECRST 80 — sixel_scrolling off.
    h.feed(&dcs_n_bands_tall(4));

    assert_eq!(placement_count(&h), 1);
    assert_eq!(
        cursor_line_col(&h),
        (0, 0),
        "DECRST 80 + DECRST 8452 (default) → cursor stays at (0, 0)",
    );
}

// SIXEL_CURSOR_RIGHT (mode 8452) — default OFF.

/// Catalog row: `SIXEL-MODE-8452` (on path).
///
/// DECSET 8452 enables `SIXEL_CURSOR_RIGHT`. The `if cursor_right`
/// branch takes priority over `sixel_scrolling`: cursor col advances
/// by `cols` (clamped to max_col), row unchanged. Default harness
/// cell width = 8; a 10-pixel-wide sixel → cols = ceil(10/8) = 2; so
/// cursor col moves from 0 → 2 and row stays at 0.
#[test]
fn sixel_cursor_right_via_decset_8452_moves_cursor_right_of_image() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?8452h"); // DECSET 8452 — sixel_cursor_right on.
    h.feed(&dcs_n_cols_wide(10));

    assert_eq!(placement_count(&h), 1);
    assert_eq!(
        cursor_line_col(&h),
        (0, 2),
        "DECSET 8452 → cursor moves to the right of the image by `cols`",
    );
}

/// Catalog row: `SIXEL-MODE-8452` (priority over SIXEL_SCROLLING).
///
/// When both modes are set, `cursor_right` wins — the cursor moves
/// right (no linefeed) even with a multi-row image. Pins the
/// `if cursor_right` priority branch in `sixel_create_placement`.
#[test]
fn sixel_cursor_right_priority_over_sixel_scrolling() {
    let mut h = SpecHarness::new();
    // Both modes set explicitly (default for 80 is ON; set 8452 ON).
    h.feed(b"\x1b[?80h\x1b[?8452h");
    // Multi-row image (24px tall = 2 cell-rows), 10px wide (2 cell-cols).
    let mut dcs = b"\x1bPq#0;2;100;0;0".to_vec();
    for _ in 0..10 {
        dcs.push(b'~');
    }
    dcs.extend_from_slice(b"-");
    for _ in 0..10 {
        dcs.push(b'~');
    }
    dcs.extend_from_slice(b"\x1b\\");
    h.feed(&dcs);

    assert_eq!(placement_count(&h), 1);
    assert_eq!(
        cursor_line_col(&h),
        (0, 2),
        "cursor_right has priority over sixel_scrolling — row unchanged, \
         col advances by `cols`",
    );
}

// Placement creation attributes.

/// Catalog row: placement-creation — `z_index: 0` + `FixedPixels`
/// sizing.
///
/// Every sixel `handle_sixel_end` commits a placement via
/// `sixel_create_placement`. The snapshot-exposed
/// `RenderablePlacement::z_index` must be `0` so z-order is neutral
/// (non-negative draws above text per the GPU emit path; negative draws
/// below). `FixedPixels` sizing is observable through
/// `RenderablePlacement::display_width`/`display_height`: the decoded
/// sixel's pixel dimensions flow through verbatim — no cell-based
/// stretching (which would be the `CellCount` sizing branch). A
/// 5-column × 6-row sixel at `dcs_n_cols_wide(5)` renders at exactly
/// 5.0 × 6.0 pixels, proving FixedPixels is live.
#[test]
fn sixel_placement_creation_sets_z_index_zero_and_fixed_pixels_sizing() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));

    assert_eq!(placement_count(&h), 1);
    let snap = h.term().renderable_content();
    let p = snap.images.last().expect("placement present");
    assert_eq!(
        p.z_index, 0,
        "sixel placements use z_index: 0 (non-negative → above text)",
    );
    assert_eq!(
        p.display_width, 5.0,
        "FixedPixels sizing — display_width matches decoded sixel pixels",
    );
    assert_eq!(
        p.display_height, 6.0,
        "FixedPixels sizing — display_height matches decoded sixel pixels",
    );
}

/// Catalog row: placement-creation — `cell_col` + `cell_row` reflect
/// cursor position at DCS-end time.
///
/// The placement is anchored at the cursor's (col, stable_row) when
/// `handle_sixel_end` fires. Moving the cursor with CUP before the
/// DCS changes the placement's pixel-viewport origin (the UV/pixel
/// coords exposed by `RenderablePlacement` reflect cell position
/// scaled by cell dimensions).
#[test]
fn sixel_placement_anchors_at_cursor_position() {
    let mut h = SpecHarness::new();
    // Move cursor to (line=2, col=4) via CUP, then emit a sixel.
    h.feed(b"\x1b[3;5H"); // CUP is 1-indexed — (row=3, col=5) == (line=2, col=4).
    h.feed(&dcs_n_cols_wide(5));

    assert_eq!(placement_count(&h), 1);
    let snap = h.term().renderable_content();
    let p = snap.images.last().expect("placement present");
    // cell_pixel_width=8, cell_pixel_height=16 (harness defaults).
    assert_eq!(
        p.viewport_x, 32.0,
        "viewport_x = cell_col (4) * cell_pixel_width (8) = 32",
    );
    assert_eq!(
        p.viewport_y, 32.0,
        "viewport_y = cell_row (2) * cell_pixel_height (16) = 32",
    );
}

// Orphan cleanup via prune_scrollback.

/// Catalog row: orphan cleanup (consumes §07's `prune_scrollback` hook).
///
/// Place a sixel at stable_row=0, then drive enough linefeeds to push
/// that row off the end of the scrollback buffer. The VTE handler's
/// `linefeed` path in `oriterm_core/src/term/handler/mod.rs` calls
/// `prune_images_if_evicted` after each linefeed; once the placement's
/// stable_row falls below `total_evicted`, `ImageCache::prune_scrollback`
/// drops it. Harness scrollback is 1000 lines; grid is 24 lines;
/// 24 + 1000 + margin = 1100 linefeeds guarantees eviction.
#[test]
fn sixel_placement_pruned_when_scrollback_evicts_its_row() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(
        placement_count(&h),
        1,
        "placement committed at stable_row=0",
    );

    // 1100 linefeeds — 24 grid-fills + 1000 scrollback-fills + 76 margin.
    let lfs = vec![b'\n'; 1100];
    h.feed(&lfs);

    assert_eq!(
        placement_count(&h),
        0,
        "placement at evicted stable_row must be pruned via \
         prune_images_if_evicted → ImageCache::prune_scrollback",
    );
}

// Matrix completeness.

/// Structural matrix-completeness pin for §12.3's non-GPU scope.
///
/// GPU occlusion pilots (wide-CJK / ZWJ / subcell + negative-z pin)
/// live under `oriterm/src/gpu/visual_regression/spec_chain/pilots/`
/// per the crate-boundaries rule and are NOT enumerated here.
#[test]
fn grid_integration_category_matrix_completeness() {
    const CATEGORIES: &[&str] = &[
        "sixel_scrolling_default_on",
        "sixel_scrolling_off_decrst_80",
        "sixel_cursor_right_decset_8452",
        "sixel_cursor_right_priority",
        "placement_z_index_zero_and_fixed_pixels_sizing",
        "placement_anchor_at_cursor",
        "placement_pruned_on_scrollback_eviction",
    ];
    assert_eq!(
        CATEGORIES.len(),
        7,
        "§12.3 non-GPU scope has 7 grid-integration categories; \
         update this list if the plan changes",
    );
}
