//! Tests for `ImageCache` — lifecycle operations, resize handling,
//! and reflow remapping.
//!
//! Unit tests for cache basics (store, evict, viewport, dirty flag,
//! animation) live in `image/tests.rs`. This file covers the lifecycle
//! methods in `cache/lifecycle.rs` — `on_resize`, `remap_placements`,
//! and the `Term::renderable_content` regression guard.

use std::sync::Arc;

use crate::effect::VoidEffectSink;
use crate::grid::{ReflowMapping, StableRowIndex};
use crate::image::{
    ImageCache, ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing,
};
use crate::term::{RenderableContent, Term};
use crate::theme::Theme;

/// Helper: create `ImageData` with the given ID and byte count of fake RGBA data.
fn make_image(id: u32, bytes: usize) -> ImageData {
    ImageData {
        id: ImageId(id),
        width: 100,
        height: 100,
        data: Arc::new(vec![0u8; bytes]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    }
}

/// Helper: create a placement at the given cell position with a
/// specific span width (`cols`) and height (`rows`).
fn make_placement(image_id: u32, col: usize, row: u64, cols: usize, rows: usize) -> ImagePlacement {
    ImagePlacement {
        image_id: ImageId(image_id),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 0,
        source_h: 0,
        cell_col: col,
        cell_row: StableRowIndex(row),
        cols,
        rows,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    }
}

/// Helper: create a FixedPixels placement.
fn make_fixed_pixels_placement(
    image_id: u32,
    col: usize,
    row: u64,
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
) -> ImagePlacement {
    let mut p = make_placement(image_id, col, row, cols, rows);
    p.sizing = PlacementSizing::FixedPixels { width, height };
    p
}

// ── on_resize: column-bounds removal ────────────────────────────────

#[test]
fn on_resize_removes_placement_fully_outside_new_cols() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.place(make_placement(1, 90, 0, 10, 1)); // col 90, spans 10

    cache.on_resize(80, 24);

    assert_eq!(
        cache.placement_count(),
        0,
        "placement starting at col=90 must be removed when new_cols=80"
    );
}

#[test]
fn on_resize_preserves_placement_within_new_cols() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.place(make_placement(1, 5, 0, 10, 1)); // col 5, spans 10 (fully inside 80)

    cache.on_resize(80, 24);

    assert_eq!(cache.placement_count(), 1);
}

#[test]
fn on_resize_preserves_partially_overlapping_placement() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.place(make_placement(1, 75, 0, 10, 1)); // col 75..85, partial overlap

    cache.on_resize(80, 24);

    // cell_col=75 < 80 → survives. Right edge (col 84) is outside but
    // the renderer clips; storage layer does not remove partial overlaps.
    assert_eq!(cache.placement_count(), 1);
}

#[test]
fn on_resize_prunes_orphaned_image_after_all_placements_removed() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.place(make_placement(1, 90, 0, 10, 1));
    assert_eq!(cache.image_count(), 1);

    cache.on_resize(80, 24);

    assert_eq!(cache.placement_count(), 0);
    assert_eq!(
        cache.image_count(),
        0,
        "image with no remaining placements must be pruned"
    );
}

#[test]
fn on_resize_preserves_deferred_kitty_image_without_placements() {
    let mut cache = ImageCache::new();
    // Kitty deferred-placement pattern: store without ever placing.
    cache.store(make_image(1, 512)).unwrap();
    assert_eq!(cache.image_count(), 1);
    assert_eq!(cache.placement_count(), 0);

    cache.on_resize(80, 24);

    assert_eq!(
        cache.image_count(),
        1,
        "deferred image without placements must survive resize"
    );
}

// ── on_resize: FixedPixels ──────────────────────────────────────────

#[test]
fn on_resize_fixed_pixels_within_bounds_survives() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // FixedPixels at col=0, 200x100 px (cols=25, rows=7 at 8x16).
    // Fits in new_cols=80 since cell_col=0 < 80.
    cache.place(make_fixed_pixels_placement(1, 0, 0, 200, 100, 25, 7));

    cache.on_resize(80, 24);

    assert_eq!(cache.placement_count(), 1);
}

#[test]
fn on_resize_fixed_pixels_out_of_bounds_removed() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // cell_col=8, new_cols=8 → cell_col >= new_cols → remove.
    cache.place(make_fixed_pixels_placement(1, 8, 0, 64, 16, 8, 1));

    cache.on_resize(8, 24);

    assert_eq!(cache.placement_count(), 0);
}

// ── remap_placements: reflow-aware row remapping ────────────────────

#[test]
fn remap_placements_updates_stable_row_index_after_reflow() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // Place image at StableRowIndex(5) (old_total_evicted=0, absolute row 5).
    cache.place(make_placement(1, 0, 5, 10, 1));

    // Mapping: row 5 → output row 10 (0..=4 → themselves, 5 → 10, 6..=10 → 11..=15)
    let mut first_output_row = vec![0, 1, 2, 3, 4, 10];
    first_output_row.extend(11..=15); // rows 6..10 → 11..15
    let mapping = ReflowMapping {
        first_output_row,
        old_total_evicted: 0,
    };

    cache.remap_placements(&mapping);

    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row,
        StableRowIndex(10),
        "placement at row 5 must remap to StableRowIndex(0 + 10) = 10"
    );
}

#[test]
fn remap_placements_skips_already_evicted_placement() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // Placement at StableRowIndex(2), but old_total_evicted=5 (was
    // already evicted before reflow was even considered).
    cache.place(make_placement(1, 0, 2, 10, 1));

    let mapping = ReflowMapping {
        first_output_row: vec![0, 1, 2, 3, 4],
        old_total_evicted: 5,
    };

    // Must not panic (checked_sub prevents underflow). Placement
    // unchanged because it's below old_total_evicted.
    cache.remap_placements(&mapping);

    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row,
        StableRowIndex(2),
        "placement below old_total_evicted must be left unchanged (prune_scrollback cleans up)"
    );
}

#[test]
fn remap_placements_handles_unwrap() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // Placement on the continuation row (row 1) of a 2-row wrapped
    // logical line. After unwrap (width increase), both source rows
    // collapse into output row 0.
    cache.place(make_placement(1, 0, 1, 10, 1));

    let mapping = ReflowMapping {
        first_output_row: vec![0, 0, 1, 2], // rows 0+1 → 0 (unwrap), rows 2,3 → 1,2
        old_total_evicted: 0,
    };

    cache.remap_placements(&mapping);

    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row,
        StableRowIndex(0),
        "continuation-row placement must follow unwrapped content to output row 0"
    );
}

#[test]
fn remap_placements_handles_row_split() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    // Placement on source row 0 that will split into 2 output rows
    // on width decrease. Maps to the FIRST output row (0).
    cache.place(make_placement(1, 0, 0, 10, 1));

    let mapping = ReflowMapping {
        first_output_row: vec![0, 2, 3], // row 0 → 0 (rows 0 occupies 0..=1), row 1 → 2, row 2 → 3
        old_total_evicted: 0,
    };

    cache.remap_placements(&mapping);

    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].cell_row,
        StableRowIndex(0),
        "row-split placement must map to first output row (0)"
    );
}

#[test]
fn remap_placements_preserves_kitty_deferred_images() {
    let mut cache = ImageCache::new();
    // Store without placing (Kitty deferred pattern).
    cache.store(make_image(1, 512)).unwrap();
    assert_eq!(cache.image_count(), 1);

    let mapping = ReflowMapping {
        first_output_row: vec![0, 1, 2],
        old_total_evicted: 0,
    };

    cache.remap_placements(&mapping);

    assert_eq!(
        cache.image_count(),
        1,
        "deferred image (no placements) must survive remap unchanged"
    );
}

// ── Negative rendering pin ──────────────────────────────────────────

/// A placement removed by `Term::resize` must NOT appear in
/// `RenderableContent::images`. Actively asserts the removal holds
/// all the way through the snapshot pipeline, not just at the cache
/// level.
#[test]
fn removed_placement_not_in_renderable_content() {
    let mut term = Term::new(24, 100, 1000, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    // Place image at col=90 (outside the post-resize 80-wide grid).
    let id = term.image_cache_mut().next_image_id();
    let data = ImageData {
        id,
        width: 2,
        height: 2,
        data: Arc::new(vec![255; 16]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    term.image_cache_mut().store(data).unwrap();
    term.image_cache_mut()
        .place(make_placement(id.0, 90, 0, 10, 1));

    // Verify the placement is visible in the snapshot BEFORE resize.
    let mut before = RenderableContent::default();
    term.renderable_content_into(&mut before);
    assert_eq!(
        before.images.len(),
        1,
        "placement should be visible before resize"
    );

    // Resize: drops the col=90 placement.
    term.resize(24, 80, true);

 // Regression guard: the removed placement must NOT appear in the
    // rendered snapshot.
    let mut after = RenderableContent::default();
    term.renderable_content_into(&mut after);
    assert!(
        after.images.is_empty(),
        "removed placement must not be present in RenderableContent::images"
    );
}
