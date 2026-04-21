//! Sixel + image lifecycle spec-conformance tests (§12.5, non-GPU portion).
//!
//! Drives every §07 lifecycle hook from the sixel side through the public
//! `Term` + `SpecHarness` surface. Each test observes the effect via
//! `Term::renderable_content().images` — the hot-path snapshot API that
//! the renderer consumes — never the private `ImageCache` helper.
//!
//! §07 is `status: complete`; §12.5 **consumes** its handlers
//! (`ImageCache::on_resize`, `ReflowMapping`/`remap_placements`,
//! `prune_scrollback`, `remove_placements_in_region`) through the public
//! seams: `Term::resize`, `Term::set_cell_dimensions`, ED/EL escape
//! sequences, DECSET 1049 alt-screen toggle, and the linefeed scrollback
//! path.
//!
//! Cross-stack hand-off lives in `cross_stack_handoff.rs`.

use oriterm_test_support::spec_chain::SpecHarness;
use oriterm_test_support::spec_chain::sixel_fixtures::{dcs_n_cols_wide, placement_count};

// Scrollback eviction — consumes §07's `prune_scrollback`.

/// Catalog row: sixel lifecycle — scrollback eviction.
///
/// Place a sixel at `stable_row=0`, then drive enough linefeeds to push
/// it past the 1000-row scrollback buffer. The VTE handler's linefeed
/// path calls `prune_images_if_evicted` → `ImageCache::prune_scrollback`,
/// which removes placements whose `stable_row < total_evicted`. Pins
/// §07's eviction handler via the public snapshot.
#[test]
fn sixel_placement_removed_when_scrollback_evicts_its_row() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(placement_count(&h), 1, "placement committed at row 0");

    // 1100 linefeeds: 24 visible + 1000 scrollback + 76 margin.
    let lfs = vec![b'\n'; 1100];
    h.feed(&lfs);

    assert_eq!(
        placement_count(&h),
        0,
        "placement at evicted stable_row must be pruned via \
         ImageCache::prune_scrollback",
    );
}

// ED (erase display) — consumes §07's `remove_placements_in_region`.

/// Catalog row: sixel lifecycle — ED CSI 2 J removes placement.
///
/// Place a sixel at `(line=0, col=0)`, then emit `CSI 2 J` (ED `All`).
/// `clear_images_after_ed` routes to
/// `ImageCache::remove_placements_in_region(first, last, None, None)`
/// covering the full visible grid. The sixel placement's
/// `stable_row=0` falls inside that region → removed.
#[test]
fn sixel_placement_removed_on_full_screen_ed() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(placement_count(&h), 1);

    h.feed(b"\x1b[2J"); // ED All.

    assert_eq!(
        placement_count(&h),
        0,
        "CSI 2 J must remove placements via remove_placements_in_region",
    );
}

// EL (erase line) — consumes §07's `remove_placements_in_region`.

/// Catalog row: sixel lifecycle — EL CSI 2 K removes placement on row.
///
/// Place a sixel at `(line=0, col=0)`, then emit `CSI 2 K` (EL `All`).
/// `clear_images_after_el` calls
/// `remove_placements_in_region(row, row, None, None)` on the cursor
/// line; the sixel's `stable_row=0` + `cell_col=0` falls inside →
/// removed.
#[test]
fn sixel_placement_removed_on_el_covering_its_row() {
    let mut h = SpecHarness::new();
    // Emit sixel then return cursor to (0,0) so EL targets the sixel row.
    h.feed(&dcs_n_cols_wide(5));
    h.feed(b"\x1b[H"); // CUP (1,1) = (line=0, col=0).
    assert_eq!(placement_count(&h), 1);

    h.feed(b"\x1b[2K"); // EL All.

    assert_eq!(
        placement_count(&h),
        0,
        "CSI 2 K must remove placements on the cursor line",
    );
}

// Alt-screen toggle — primary cache preserved across enter/exit.

/// Catalog row: sixel lifecycle — alt-screen preserves primary placement.
///
/// Enter alt screen (DECSET 1049) and observe that the snapshot returns
/// zero placements (the alt cache is separate and empty). Exit alt
/// screen (DECRST 1049) and observe that the primary placement is
/// intact — proves `alt_image_cache` is a distinct cache, not a shared
/// view.
#[test]
fn sixel_primary_placement_preserved_across_alt_screen_toggle() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(placement_count(&h), 1, "primary cache has 1 placement");

    h.feed(b"\x1b[?1049h"); // DECSET 1049 — enter alt screen.
    assert_eq!(
        placement_count(&h),
        0,
        "alt cache is distinct and empty (primary placement not shared)",
    );

    h.feed(b"\x1b[?1049l"); // DECRST 1049 — exit alt screen.
    assert_eq!(
        placement_count(&h),
        1,
        "primary placement preserved after alt-screen round-trip",
    );
}

// Resize shrink columns — consumes §07's `ImageCache::on_resize`.

/// Catalog row: sixel lifecycle — resize shrink removes out-of-bounds.
///
/// Place a sixel at `(line=0, col=90)` on a 100-col harness, then
/// resize to 80 cols. `Term::resize` → `ImageCache::on_resize(80, …)`
/// which removes placements whose `cell_col >= new_cols`.
#[test]
fn sixel_placement_removed_on_resize_shrinking_past_column() {
    let mut h = SpecHarness::with_size(24, 100);
    // Move to col=90 then emit sixel.
    h.feed(b"\x1b[1;91H");
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(placement_count(&h), 1);

    h.term_mut().resize(24, 80, false);

    assert_eq!(
        placement_count(&h),
        0,
        "placement at col=90 must be removed when cols shrink to 80 \
         via ImageCache::on_resize",
    );
}

// Resize WITHOUT reflow but growing — placement preserved.

/// Catalog row: sixel lifecycle — resize preserves in-bounds placement.
///
/// Place a sixel at `(line=0, col=0)` on an 80-col harness, then
/// resize to 120 cols. The placement's `cell_col=0` is in-bounds →
/// preserved. Paired positive-pin for the shrink test above.
#[test]
fn sixel_placement_preserved_on_resize_expanding_columns() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    assert_eq!(placement_count(&h), 1);

    h.term_mut().resize(24, 120, false);

    assert_eq!(
        placement_count(&h),
        1,
        "in-bounds placement preserved across column-grow resize",
    );
}

// Resize WITH reflow — consumes §07's `remap_placements`.

/// Catalog row: sixel lifecycle — reflow remaps in-bounds placement.
///
/// Place a sixel at `(line=0, col=0)`, then `term.resize(24, 40, true)`
/// with reflow enabled. The placement's `cell_col=0` stays in-bounds
/// under 40 cols, so `ImageCache::on_resize` does NOT remove it; the
/// `remap_placements` path (driven by `ReflowMapping`) preserves the
/// placement across the reflow — proven by the snapshot still listing
/// it with intact `display_width`/`display_height`. Positive pin for
/// reflow-remap observed via the public snapshot (avoids exposing
/// `ReflowMapping` as a public surface).
#[test]
fn sixel_placement_survives_resize_with_reflow_when_column_in_bounds() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    let before = h.term().renderable_content();
    assert_eq!(before.images.len(), 1);
    let original_width = before.images[0].display_width;
    let original_height = before.images[0].display_height;

    // Resize 80 cols → 40 cols with reflow enabled. Column 0 stays
    // in-bounds; remap_placements keeps the placement alive.
    h.term_mut().resize(24, 40, true);

    let after = h.term().renderable_content();
    assert_eq!(
        after.images.len(),
        1,
        "in-bounds placement survives resize-with-reflow via \
         ImageCache::remap_placements (NOT removed by on_resize, \
         NOT orphaned)",
    );
    assert_eq!(
        after.images[0].display_width, original_width,
        "FixedPixels display_width unchanged through reflow (image \
         dims are fixed; only stable_row remaps)",
    );
    assert_eq!(
        after.images[0].display_height, original_height,
        "FixedPixels display_height unchanged through reflow",
    );
}

/// Catalog row: sixel lifecycle — reflow REMAPS stable_row when text above wraps.
///
/// Stronger reflow pin than `…survives_resize_with_reflow…`: this test
/// sets up preceding text that wraps differently at 80 vs 40 cols, so
/// the sixel's `stable_row` MUST change under `remap_placements` or the
/// assertion fails. Removing `remap_placements` from `Term::resize` or
/// regressing its `ReflowMapping::first_output_row` lookup would leave
/// the placement at its original stable_row and fail this test.
///
/// Setup: write 40 characters at 80 cols (fits on line 0, cursor at col
/// 40 line 0), linefeed to line 1, place sixel on line 1 (stable_row=1).
/// Resize 80→40 cols with reflow: the 40-char prefix stays on line 0
/// (fits exactly), but the `\n` newline still advances to line 1.
/// However because the cursor was at col 40 BEFORE resize and reflow
/// compacts wrapped lines, `remap_placements` recomputes the sixel's
/// stable_row via `ReflowMapping::first_output_row`. The viewport_y
/// must be consistent with the post-reflow grid — crucially, the
/// placement must still be mapped to a valid cell-row after reflow
/// (not stranded at a stable_row that no longer exists in the grid).
#[test]
fn sixel_placement_remaps_stable_row_across_reflow_with_text_wrap() {
    let mut h = SpecHarness::with_size(24, 80);
    // Write 100 characters at 80 cols — wraps into 2 visible lines
    // (line 0 full 80 chars, line 1 has 20 chars). Cursor ends at line
    // 1 col 20.
    let text: Vec<u8> = (0..100).map(|i| b'A' + (i % 26) as u8).collect();
    h.feed(&text);
    // Move cursor to line 3 col 0, then place a sixel there.
    h.feed(b"\x1b[4;1H");
    h.feed(&dcs_n_cols_wide(5));

    let before = h.term().renderable_content();
    assert_eq!(before.images.len(), 1);
    // Sixel placed at CUP (4,1) → line 3 × cell_pixel_height 16 = 48.
    assert_eq!(
        before.images[0].viewport_y, 48.0,
        "before reflow: sixel at line 3, viewport_y = 3 * 16 = 48",
    );

    // Resize 80 → 40 cols with reflow. The 100-char prefix wraps from
    // 2 lines at 80 cols (80 + 20) to 3 lines at 40 cols (40 + 40 + 20),
    // pushing the sixel placement one row down: line 3 → line 4.
    // `remap_placements` recomputes the stable_row via
    // `ReflowMapping::first_output_row`, observable as viewport_y shift.
    h.term_mut().resize(24, 40, true);

    let after = h.term().renderable_content();
    assert_eq!(
        after.images.len(),
        1,
        "placement survives the reflow (remap_placements kept it alive)",
    );
    // After reflow: sixel at line 4 × cell_pixel_height 16 = 64. Exact
    // pin (not assert_ne) — if the reflow algorithm regresses to
    // no-op-remap the value stays 48; if it moves to the wrong row the
    // strict equality catches that too.
    assert_eq!(
        after.images[0].viewport_y, 64.0,
        "reflow remaps sixel from line 3 → line 4 (100-char text wraps \
         3-way at 40 cols, pushing sixel one row down); viewport_y \
         must be 4 * 16 = 64",
    );
}

// Font-size / DPI change — consumes `Term::set_cell_dimensions`.

/// Catalog row: sixel lifecycle — cell-dimension change preserves placement.
///
/// Place a sixel (`FixedPixels` sizing at harness defaults
/// `cell_pixel_width=8`, `cell_pixel_height=16`), then call
/// `Term::set_cell_dimensions(16, 32)`. The placement survives the
/// dimension change; its cell coverage recomputes via
/// `update_cell_coverage`, but the snapshot still lists it.
#[test]
fn sixel_placement_preserved_across_cell_dimension_change() {
    let mut h = SpecHarness::new();
    h.feed(&dcs_n_cols_wide(5));
    let before = h.term().renderable_content();
    assert_eq!(before.images.len(), 1);
    let original_width = before.images[0].display_width;

    h.term_mut().set_cell_dimensions(16, 32);

    let after = h.term().renderable_content();
    assert_eq!(
        after.images.len(),
        1,
        "placement preserved across set_cell_dimensions (handler only \
         updates cell coverage; FixedPixels display dims unchanged)",
    );
    assert_eq!(
        after.images[0].display_width, original_width,
        "FixedPixels display_width independent of cell dimensions",
    );
}

// Negative pin — no-sixel placements: on_resize is a no-op.

/// Negative pin — §07 handler fires only on relevant placements.
///
/// With zero sixel placements in the cache, `Term::resize` (which calls
/// `ImageCache::on_resize` via the internal resize path) must not spawn
/// any phantom placements. This proves `on_resize` is a no-op on an
/// empty cache — the handler doesn't fabricate work.
#[test]
fn resize_on_empty_cache_is_noop_no_phantom_placements() {
    let mut h = SpecHarness::new();
    assert_eq!(placement_count(&h), 0, "empty cache pre-condition");

    h.term_mut().resize(24, 40, false);
    assert_eq!(placement_count(&h), 0, "resize must not create placements");

    h.term_mut().resize(40, 100, true);
    assert_eq!(
        placement_count(&h),
        0,
        "resize with reflow must not create placements",
    );
}

// Matrix completeness.

/// Structural matrix-completeness pin for §12.5's non-GPU lifecycle scope.
///
/// Cross-stack hand-off lives in `cross_stack_handoff.rs` — the
/// handshake test is enumerated there, not here.
#[test]
fn lifecycle_category_matrix_completeness() {
    const CATEGORIES: &[&str] = &[
        "scrollback_eviction",
        "ed_full_screen",
        "el_full_line",
        "alt_screen_toggle",
        "resize_shrink_columns",
        "resize_expand_columns",
        "resize_with_reflow_preserves_in_bounds",
        "resize_with_reflow_remaps_stable_row",
        "cell_dimension_change",
        "negative_pin_empty_cache_resize_noop",
    ];
    assert_eq!(
        CATEGORIES.len(),
        10,
        "§12.5 non-GPU lifecycle scope has 10 categories — update this \
         matrix when adding/removing a lifecycle test",
    );
}
