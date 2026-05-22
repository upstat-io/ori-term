//! Tests for `Term::resize` — terminal resize lifecycle including
//! image-cache remap, alt-cache handling, and reflow integration.

use std::sync::Arc;

use vte::ansi::Processor;

use crate::cell::{Cell, CellFlags};
use crate::effect::VoidEffectSink;
use crate::grid::StableRowIndex;
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing};
use crate::index::{Column, Line};
use crate::term::Term;
use crate::term::renderable::RenderableContent;
use crate::theme::Theme;

fn make_term() -> Term<VoidEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

/// Feed raw bytes through the VTE processor.
fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

/// Store a 2×2 RGBA test image and place it at the given cell row/col.
fn place_test_image(
    term: &mut Term<VoidEffectSink>,
    stable_row: u64,
    col: usize,
    rows: usize,
    cols: usize,
) -> ImageId {
    let id = term.image_cache_mut().next_image_id();
    let data = ImageData {
        id,
        width: 2,
        height: 2,
        data: Arc::new(vec![255; 16]),
        pixel_generation: 0,
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    term.image_cache_mut().store(data).unwrap();
    term.image_cache_mut().place(ImagePlacement {
        image_id: id,
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 0,
        source_h: 0,
        cell_col: col,
        cell_row: StableRowIndex(stable_row),
        cols,
        rows,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    id
}

// ── Term::resize integration ────────────────────────────────────────

#[test]
fn term_resize_changes_both_grids() {
    let mut term = make_term();
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);

    term.resize(10, 40, true);

    assert_eq!(term.grid().lines(), 10);
    assert_eq!(term.grid().cols(), 40);

    term.swap_alt();
    assert_eq!(term.grid().lines(), 10);
    assert_eq!(term.grid().cols(), 40);
}

#[test]
fn term_resize_preserves_content() {
    let mut term = make_term();
    feed(&mut term, b"hello");

    term.resize(10, 40, true);

    assert_eq!(term.grid()[Line(0)][Column(0)].ch, 'h');
    assert_eq!(term.grid()[Line(0)][Column(1)].ch, 'e');
    assert_eq!(term.grid()[Line(0)][Column(4)].ch, 'o');
}

#[test]
fn term_resize_marks_selection_dirty() {
    let mut term = make_term();
    term.clear_selection_dirty();

    term.resize(10, 40, true);

    assert!(term.is_selection_dirty());
}

#[test]
fn term_resize_marks_all_dirty() {
    let mut term = make_term();
    term.grid_mut().dirty_mut().drain().for_each(drop);

    term.resize(10, 40, true);

    assert!(term.grid().dirty().is_all_dirty());
}

#[test]
fn term_resize_zero_is_noop() {
    let mut term = make_term();
    term.resize(0, 40, true);
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);

    term.resize(10, 0, true);
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);
}

#[test]
fn term_resize_with_vte_wrapped_content() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);
    feed(&mut term, b"abcdefghijklmnopqrst");
    assert_eq!(term.grid().cursor().line(), 1);

    term.resize(5, 20, true);

    assert_eq!(term.grid().cols(), 20);
    assert_eq!(term.grid()[Line(0)][Column(0)].ch, 'a');
    assert_eq!(term.grid()[Line(0)][Column(9)].ch, 'j');
    assert_eq!(term.grid()[Line(0)][Column(10)].ch, 'k');
    assert_eq!(term.grid()[Line(0)][Column(19)].ch, 't');
}

// Stress resize: rapid dimension changes simulating window drag.

#[test]
fn stress_resize_rapid_dimension_changes() {
    let mut term = make_term();
    feed(&mut term, b"hello world\r\nsecond line\r\nthird line");

    let sizes: &[(usize, usize)] = &[
        (24, 80),
        (23, 79),
        (20, 60),
        (10, 40),
        (5, 20),
        (1, 1),
        (2, 2),
        (3, 3),
        (5, 5),
        (10, 10),
        (50, 200),
        (100, 300),
        (24, 80),
        (1, 1),
        (24, 80),
        (3, 100),
        (100, 3),
        (1, 200),
        (200, 1),
    ];
    let mut buf = RenderableContent::default();

    for &(rows, cols) in sizes {
        term.resize(rows, cols, true);
        term.renderable_content_into(&mut buf);
        assert_eq!(
            buf.lines, rows,
            "lines mismatch after resize to {rows}x{cols}"
        );
        assert_eq!(
            buf.cols, cols,
            "cols mismatch after resize to {rows}x{cols}"
        );
        assert_eq!(
            buf.cells.len(),
            rows * cols,
            "cell count mismatch after resize to {rows}x{cols}"
        );
        assert!(
            buf.cursor.line < rows,
            "cursor line {} >= rows {rows} after resize to {rows}x{cols}",
            buf.cursor.line
        );
        assert!(
            buf.cursor.column.0 < cols,
            "cursor col {} >= cols {cols} after resize to {rows}x{cols}",
            buf.cursor.column.0
        );
    }
}

#[test]
fn stress_resize_with_scrollback_and_reflow() {
    let mut term = Term::new(10, 40, 500, Theme::default(), VoidEffectSink);
    for i in 0..100 {
        let line = format!("line {i:04} with some padding text here\r\n");
        feed(&mut term, line.as_bytes());
    }

    let sizes: &[(usize, usize)] = &[
        (10, 40),
        (5, 20),
        (3, 10),
        (1, 5),
        (20, 80),
        (10, 40),
        (50, 120),
        (5, 10),
        (10, 40),
    ];
    let mut buf = RenderableContent::default();

    for &(rows, cols) in sizes {
        term.resize(rows, cols, true);
        term.renderable_content_into(&mut buf);
        assert_eq!(buf.lines, rows);
        assert_eq!(buf.cols, cols);
        assert_eq!(buf.cells.len(), rows * cols);
        assert!(buf.cursor.line < rows);
        assert!(buf.cursor.column.0 < cols);
    }
}

#[test]
fn stress_resize_alternating_grow_shrink() {
    let mut term = make_term();
    feed(&mut term, b"test content for resize cycles");
    let mut buf = RenderableContent::default();

    for i in 0..50 {
        let rows = if i % 2 == 0 { 10 } else { 30 };
        let cols = if i % 3 == 0 { 40 } else { 120 };
        term.resize(rows, cols, true);
        term.renderable_content_into(&mut buf);
        assert_eq!(buf.lines, rows);
        assert_eq!(buf.cols, cols);
        assert_eq!(buf.cells.len(), rows * cols);
    }
}

#[test]
fn stress_resize_with_wide_chars() {
    let mut term = Term::new(10, 20, 100, Theme::default(), VoidEffectSink);
    feed(&mut term, "日本語テスト".as_bytes());

    let sizes: &[(usize, usize)] = &[
        (10, 20),
        (10, 10),
        (10, 5),
        (10, 3),
        (10, 2),
        (10, 20),
        (10, 40),
        (10, 10),
        (10, 20),
    ];
    let mut buf = RenderableContent::default();

    for &(rows, cols) in sizes {
        term.resize(rows, cols, true);
        term.renderable_content_into(&mut buf);
        assert_eq!(buf.lines, rows);
        assert_eq!(buf.cols, cols);
        assert_eq!(buf.cells.len(), rows * cols);
    }
}

#[test]
fn stress_resize_vte_output_between_resizes() {
    let mut term = make_term();
    let mut buf = RenderableContent::default();

    for i in 0..20 {
        let line = format!("output line {i}\r\n");
        feed(&mut term, line.as_bytes());

        let rows = 10 + (i % 15);
        let cols = 40 + (i * 3 % 60);
        term.resize(rows, cols, true);
        term.renderable_content_into(&mut buf);
        assert_eq!(buf.lines, rows);
        assert_eq!(buf.cols, cols);
        assert_eq!(buf.cells.len(), rows * cols);
    }
}

// ── Image lifecycle on resize (section 07.5) ──

/// A placement whose starting column falls entirely outside the new
/// grid width must be dropped by `Term::resize`.
#[test]
fn term_resize_removes_out_of_bounds_image_placement() {
    let mut term = Term::new(24, 100, 1000, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);
    place_test_image(&mut term, 0, 90, 1, 10);
    assert_eq!(term.image_cache().placement_count(), 1);

    term.resize(24, 80, true);

    assert_eq!(
        term.image_cache().placement_count(),
        0,
        "placement starting at col=90 must be removed when new_cols=80"
    );
}

/// The alt-screen image cache (when allocated) also gets column-bounds
/// handling on resize. Alt grid never reflows, so only `on_resize`
/// runs on the alt cache — no remap.
#[test]
fn term_resize_updates_alt_cache_when_alt_exists() {
    let mut term = Term::new(24, 100, 1000, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);
    term.swap_alt();
    place_test_image(&mut term, 0, 90, 1, 10);
    assert_eq!(
        term.image_cache().placement_count(),
        1,
        "alt-mode placement should be visible before resize"
    );

    term.resize(24, 80, true);

    assert_eq!(
        term.image_cache().placement_count(),
        0,
        "alt-mode placement at col=90 must be removed when new_cols=80"
    );
}

/// When reflow runs, `Term::resize` remaps placements through the
/// `ReflowMapping` so they continue to point at the same content.
/// A soft-wrapped continuation row unwraps into its parent on grow —
/// the placement on the continuation row must follow the content.
#[test]
fn term_resize_remaps_image_placement_through_reflow() {
    let mut term = Term::new(3, 10, 100, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    for (col, ch) in "helloworld".chars().enumerate() {
        term.grid_mut()[Line(0)][Column(col)] = Cell {
            ch,
            ..Cell::default()
        };
    }
    term.grid_mut()[Line(0)][Column(9)]
        .flags
        .insert(CellFlags::WRAP);
    for (col, ch) in "again".chars().enumerate() {
        term.grid_mut()[Line(1)][Column(col)] = Cell {
            ch,
            ..Cell::default()
        };
    }

    place_test_image(&mut term, 1, 0, 1, 2);
    assert_eq!(
        term.image_cache()
            .placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX))[0]
            .cell_row
            .0,
        1
    );

    term.resize(3, 20, true);

    let placements = term
        .image_cache()
        .placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row.0, 0,
        "placement must follow unwrapped content onto output row 0"
    );
}

/// Regression guard: image-cache field isolation after alt-screen toggle.
/// See: §07.5
/// After removing the image-cache swap from `toggle_alt_common`, the
/// `image_cache` and `alt_image_cache` fields hold their semantic
/// contents regardless of `ALT_SCREEN` mode: primary placements live
/// in `self.image_cache`, alt placements live in `self.alt_image_cache`.
/// This test bypasses the `image_cache()` accessor and reads the
/// fields directly so it catches any future routing inversion the
/// accessor might hide.
#[test]
fn term_resize_routes_each_grid_through_its_own_image_cache() {
    let mut term = Term::new(24, 100, 1000, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    place_test_image(&mut term, 0, 5, 1, 2);
    assert_eq!(
        term.image_cache.placement_count(),
        1,
        "primary field must hold the primary-mode placement"
    );
    assert!(
        term.alt_image_cache.is_none(),
        "alt cache not yet allocated"
    );

    term.swap_alt();
    place_test_image(&mut term, 0, 90, 1, 10);
    assert_eq!(
        term.image_cache.placement_count(),
        1,
        "primary field still has its 1 placement (NOT swapped)"
    );
    assert_eq!(
        term.alt_image_cache.as_ref().unwrap().placement_count(),
        1,
        "alt field now holds the alt-mode placement (NOT swapped)"
    );

    term.resize(24, 80, true);

    assert_eq!(
        term.image_cache.placement_count(),
        1,
        "primary placement at col=5 must survive — primary grid's reflow must not hit the alt cache"
    );
    assert_eq!(
        term.alt_image_cache.as_ref().unwrap().placement_count(),
        0,
        "alt placement at col=90 must be removed by on_resize on the alt cache"
    );
}

/// Isolation check: primary-mode and alt-mode placements do NOT leak
/// into each other's cache. Regression guard for the image-cache
/// field isolation fix (spec-conformance §07.5): the old cache-swap
/// allowed alt-mode placements to appear in primary after swap back.
#[test]
fn alt_image_cache_isolation_check() {
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    place_test_image(&mut term, 0, 5, 1, 2);
    assert_eq!(term.image_cache().placement_count(), 1);

    term.swap_alt();
    assert_eq!(
        term.image_cache().placement_count(),
        0,
        "alt mode must not see the primary placement"
    );

    place_test_image(&mut term, 0, 10, 1, 2);
    assert_eq!(term.image_cache().placement_count(), 1);

    term.swap_alt();
    assert_eq!(
        term.image_cache().placement_count(),
        1,
        "primary must still hold only its original placement — alt-mode placement must not leak"
    );
}

/// `reflow: false` skips the remap step entirely — placements retain
/// their original `cell_row` when reflow does not run.
#[test]
fn term_resize_without_reflow_skips_remap() {
    let mut term = Term::new(3, 20, 100, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);
    place_test_image(&mut term, 1, 0, 1, 2);

    term.resize(5, 10, false);

    let placements = term
        .image_cache()
        .placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row.0, 1,
        "reflow=false must leave cell_row unchanged"
    );
}

/// Regression guard: resize reconciles BOTH the primary AND the
/// inactive-alt-screen image cache's placeholder anchors against
/// their respective grids. The fix is `reconcile_both_placeholder_anchors`
/// at `term/handler/helpers.rs:295` invoked from `term/resize/mod.rs:91`.
/// Without this symmetric reconcile, the inactive cache's anchors
/// referencing images whose backing cells were dropped by resize
/// would survive as orphans, and the next alt-screen entry would
/// render zombie placeholders.
/// This test:
/// 1. Allocates the alt cache via `swap_alt`.
/// 2. Inserts a placeholder anchor in the alt cache (no `U+10EEEE`
/// cells written — anchor is orphan-by-construction).
/// 3. Swaps back to primary (alt cache becomes inactive but retains
/// the orphan anchor).
/// 4. Triggers `resize`.
/// 5. Asserts the orphan anchor was reconciled away.
/// See: §13.6.1 item 8
#[test]
fn resize_from_primary_reconciles_inactive_alt_screen_placeholder_anchors() {
    let mut term = Term::new(24, 80, 100, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    // Enter alt screen — allocates `alt_grid` + `alt_image_cache`.
    term.swap_alt();

    // Insert an orphan placeholder anchor in the alt cache. We never
    // write a `U+10EEEE` cell, so the anchor has no surviving cell to
    // reference.
    let anchor_id = ImageId(99);
    term.image_cache_mut().add_placeholder_anchor(anchor_id);
    assert!(
        term.image_cache()
            .placeholder_anchors()
            .contains(&anchor_id),
        "anchor inserted into the active (alt) cache should be visible there"
    );

    // Swap back to primary. The alt cache field stays allocated and
    // still carries the orphan anchor — we are NOT testing the active
    // path here, we are testing the INACTIVE-cache reconcile.
    term.swap_alt();
    assert!(
        term.alt_image_cache
            .as_ref()
            .expect("alt cache field remains allocated after toggle back")
            .placeholder_anchors()
            .contains(&anchor_id),
        "orphan anchor must still be present in the inactive alt cache before resize"
    );

    // Trigger resize on the primary screen with a real dimension change
    // (24×80 → 24×100 so we don't hit `resize`'s same-size early-exit at
    // `term/resize/mod.rs:28`). The fix is that `Term::resize` invokes
    // `reconcile_both_placeholder_anchors` which walks BOTH grids and
    // reconciles BOTH caches; without this, the alt cache's orphan
    // anchor survives.
    term.resize(24, 100, true);

    // Pin: the orphan was reconciled away by the symmetric reconcile.
    assert!(
        term.alt_image_cache
            .as_ref()
            .expect("alt cache field still allocated post-resize")
            .placeholder_anchors()
            .is_empty(),
        "inactive alt cache's orphan placeholder anchor must be reconciled \
 away by resize — if this fails, `Term::resize` regressed to \
 calling `reconcile_placeholder_anchors_from_grid` (active grid \
 only) instead of `reconcile_both_placeholder_anchors`"
    );
}
