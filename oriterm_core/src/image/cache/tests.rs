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
        pixel_generation: 0,
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
        pixel_generation: 0,
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

// ── Snapshot-read recency contract ──────────────────────────────────
//
// Render-time reads (the SOLE production read path via `get_no_touch`)
// MUST NOT bump recency. Snapshot construction is a pure-read path per
// `term/snapshot/mod.rs:27-34` — mutating recency from inside snapshot
// construction would violate the no-side-logic invariant for snapshot
// logic` AND defeat eviction's signal (everything looks "recently
// accessed" because every visible image is touched on every frame).

/// Positive pin: `ImageCache::get_no_touch` does NOT bump `last_accessed`.
/// Read the image 100 times via `get_no_touch` and assert the stored
/// `last_accessed` is identical to the value stamped at `store()` time.
#[test]
fn get_no_touch_does_not_bump_last_accessed() {
    let mut cache = ImageCache::new();
    let id = ImageId(1);
    let stored = cache
        .store(make_image(id.0, 4))
        .expect("store should succeed within memory limits");

    let baseline = cache
        .get_no_touch(stored)
        .expect("image must exist immediately after store")
        .last_accessed;
    assert!(
        baseline > 0,
        "store should stamp last_accessed > 0 (first store bumps access_counter); got {baseline}"
    );

    for _ in 0..100 {
        let observed = cache
            .get_no_touch(stored)
            .expect("image must still exist")
            .last_accessed;
        assert_eq!(
            observed, baseline,
            "get_no_touch MUST NOT bump last_accessed; saw {observed} after read, baseline was {baseline}"
        );
    }
}

/// Companion pin: 100 sequential `renderable_content_into` calls do NOT
/// mutate `ImageCache` recency state. Routes through the production
/// snapshot path (the SSOT for `Term::renderable_content` per
/// `snapshot/mod.rs:35-82`). Catches the regression where someone
/// introduces a `&mut ImageCache` borrow in snapshot construction
/// (which would silently bump recency on every frame).
#[test]
fn snapshot_construction_does_not_mutate_image_cache_recency() {
    let mut term = Term::new(24, 80, 100, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

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
    term.image_cache_mut()
        .place(make_placement(id.0, 0, 0, 2, 2));

    let baseline_last_accessed = term
        .image_cache()
        .get_no_touch(id)
        .expect("image must exist after store + place")
        .last_accessed;

    // Drive 100 snapshot constructions through the production path.
    // Snapshot construction must be a pure read of `ImageCache`; mutating
    // the cache from inside snapshot extraction would violate the
    // snapshot-purity invariant documented at `term/snapshot/mod.rs:27-34`.
    let mut out = RenderableContent::default();
    for _ in 0..100 {
        term.renderable_content_into(&mut out);
        assert_eq!(
            out.images.len(),
            1,
            "placement should still be visible in every snapshot"
        );
    }

    let post_snapshots = term
        .image_cache()
        .get_no_touch(id)
        .expect("image must still exist after 100 snapshots")
        .last_accessed;
    assert_eq!(
        post_snapshots, baseline_last_accessed,
        "100 snapshot constructions MUST NOT bump the image's last_accessed; \
         saw {post_snapshots}, baseline was {baseline_last_accessed} — a mutation here \
         means snapshot construction acquired a &mut ImageCache borrow and broke \
         the pure-read invariant documented at term/snapshot/mod.rs:27-34"
    );
}

// ── LRU recency on placement create (§13.6.1) ──────────────────────

/// Regression: spec-conformance §13.6.1 — `ImageCache::place` bumps the
/// underlying image's `last_accessed` stamp so re-placement of an
/// existing image refreshes its eviction priority among the unplaced
/// pool. The bump lives in `ImageCache::place` itself so kitty, sixel,
/// and iterm2 all flow through one canonical recency-update path —
/// scattering the bump across per-protocol handlers would silently
/// regress eviction to FIFO-on-store-order for any new protocol whose
/// handler omits the bump.
#[test]
fn access_on_placement_create_bumps_last_accessed() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();
    cache.store(make_image(2, 256)).unwrap();

    let pre_id1 = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    let pre_id2 = cache.get_no_touch(ImageId(2)).unwrap().last_accessed;
    assert!(
        pre_id1 < pre_id2,
        "post-store baseline: id1 stored first, must have lower last_accessed; \
         saw id1={pre_id1}, id2={pre_id2}"
    );

    // Place id1 — must bump id1's recency past id2.
    cache.place(make_placement(1, 0, 0, 1, 1));

    let post_id1 = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    let post_id2 = cache.get_no_touch(ImageId(2)).unwrap().last_accessed;
    assert!(
        post_id1 > post_id2,
        "ImageCache::place(id1) must bump id1.last_accessed past id2 \
         (canonical recency-update SSOT for all protocols); saw id1={post_id1}, \
         id2={post_id2}. Regression: per-protocol handlers may be bumping \
         outside `place()`, or `place()` lost its recency-bump."
    );
    assert_eq!(
        post_id2, pre_id2,
        "place(id1) MUST NOT perturb id2's last_accessed; saw post={post_id2}, \
         pre={pre_id2}. Regression: place() now mutates more than the targeted image."
    );
}

/// Regression: spec-conformance §13.6.1 — repeated `place()` of the same
/// image keeps bumping its recency. Without this, a stale image whose
/// only refresh is "place again" would never escape eviction.
#[test]
fn access_on_repeated_placement_keeps_bumping_last_accessed() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();

    let after_store = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    cache.place(make_placement(1, 0, 0, 1, 1));
    let after_first_place = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    cache.place(make_placement(1, 5, 0, 1, 1));
    let after_second_place = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;

    assert!(after_first_place > after_store, "first place must bump");
    assert!(
        after_second_place > after_first_place,
        "second place must bump again (monotonic LRU); \
         saw first={after_first_place}, second={after_second_place}"
    );
}

/// Regression: spec-conformance §13.6.1 — `place()` of a placement whose
/// `image_id` has no corresponding `ImageData` is a no-op for recency
/// (the placement is added — orphan-placement pruning handles cleanup —
/// but no image data exists to stamp). Pin guards against a panic on
/// orphan placements as well as against silent off-by-one indexing.
#[test]
fn place_with_no_underlying_image_does_not_panic() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();
    let pre_id1 = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;

    // Place a placement whose image_id has never been stored.
    cache.place(make_placement(99, 0, 0, 1, 1));

    let post_id1 = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    assert_eq!(
        post_id1, pre_id1,
        "orphan place() must not perturb other images' last_accessed"
    );
    assert!(
        cache.get_no_touch(ImageId(99)).is_none(),
        "orphan place() must not synthesize an ImageData entry for image_id=99"
    );
}
