//! Tests for kitty unicode-placeholder (`U=1`) anchor management.
//!
//! Covers `add_placeholder_anchor`, `set_placeholder_anchor_grid`,
//! `anchor_placeholder_with_grid`, `placeholder_anchors`,
//! `placeholder_anchor_grid_for`, and `reconcile_placeholder_anchors`.
//! Tests for cross-method interactions with `remove_image` /
//! `ImageCache::clear` / store-triggered replacement also live here, because
//! the invariant under test is the anchor-grid map-sync contract owned by
//! this module.

use std::sync::Arc;

use crate::image::{ImageCache, ImageData, ImageFormat, ImageId, ImageSource};

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

/// Regression: spec-conformance §13.6.1 — `set_placeholder_anchor_grid`
/// records `(cols, rows)` on the anchor, readable via
/// `placeholder_anchor_grid_for`. Distinct from
/// `add_placeholder_anchor` which only marks the eviction immunity flag.
#[test]
fn placeholder_anchor_grid_round_trip() {
    let mut cache = ImageCache::new();
    cache.add_placeholder_anchor(ImageId(5));
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(5)),
        None,
        "anchor without explicit grid must report None — caller defaults to (1, 1)"
    );

    cache.set_placeholder_anchor_grid(ImageId(5), 4, 2);
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(5)),
        Some((4, 2)),
        "grid round-trip"
    );

    // Repeated set with same value is idempotent — no panic, value unchanged.
    cache.set_placeholder_anchor_grid(ImageId(5), 4, 2);
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(5)), Some((4, 2)));

    // Re-set with different dims overwrites.
    cache.set_placeholder_anchor_grid(ImageId(5), 7, 1);
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(5)), Some((7, 1)));
}

/// Regression: spec-conformance §13.6.1 — `set_placeholder_anchor_grid`
/// with zero cols or rows is a no-op. The cache rejects invalid grids at
/// the entry point so emit can safely divide by cols/rows without
/// per-call guards.
#[test]
fn placeholder_anchor_grid_rejects_zero_dims() {
    let mut cache = ImageCache::new();
    cache.add_placeholder_anchor(ImageId(7));

    cache.set_placeholder_anchor_grid(ImageId(7), 0, 4);
    cache.set_placeholder_anchor_grid(ImageId(7), 4, 0);
    cache.set_placeholder_anchor_grid(ImageId(7), 0, 0);
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(7)),
        None,
        "zero dims must be rejected at the cache boundary"
    );

    // A valid set after rejections still records.
    cache.set_placeholder_anchor_grid(ImageId(7), 3, 2);
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(7)), Some((3, 2)));
}

/// Regression: spec-conformance §13.6.1 — `remove_image` MUST drop the
/// `placeholder_anchor_grid` entry alongside the `placeholder_anchors`
/// removal. Direct image-removal paths (`d=I`, `d=F`, LRU eviction,
/// orphan pruning, store-triggered replacement) all flow through
/// `remove_image` and would otherwise leak grid entries. Without this
/// fix, the grid `HashMap` grows unboundedly across image create+delete
/// cycles, and a reused `ImageId` could read a stale `(cols, rows)` and
/// render broken UV slices.
#[test]
fn remove_image_clears_placeholder_anchor_grid_entry() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();
    cache.add_placeholder_anchor(ImageId(1));
    cache.set_placeholder_anchor_grid(ImageId(1), 4, 2);
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(1)), Some((4, 2)));

    cache.remove_image(ImageId(1));

    assert!(
        cache.get_no_touch(ImageId(1)).is_none(),
        "image data must be gone"
    );
    assert!(
        !cache.placeholder_anchors().contains(&ImageId(1)),
        "anchor must be dropped"
    );
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(1)),
        None,
        "grid entry MUST be dropped alongside the anchor — stale grid \
         entries are a LEAK regression"
    );
}

/// Regression: spec-conformance §13.6.1 round-0 TPR cluster —
/// `ImageCache::clear()` MUST clear `placeholder_anchor_grid` alongside
/// `placeholder_anchors`. Cache-reset paths (`\x1b_Gd=A` deletes all
/// images; primary/alt screen reset) would otherwise leave stale grid
/// entries behind.
#[test]
fn clear_image_cache_clears_placeholder_anchor_grid() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();
    cache.store(make_image(2, 256)).unwrap();
    cache.add_placeholder_anchor(ImageId(1));
    cache.set_placeholder_anchor_grid(ImageId(1), 3, 3);
    cache.add_placeholder_anchor(ImageId(2));
    cache.set_placeholder_anchor_grid(ImageId(2), 5, 1);

    cache.clear();

    assert_eq!(cache.image_count(), 0);
    assert!(cache.placeholder_anchors().is_empty());
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(1)),
        None,
        "clear() MUST drop grid entries"
    );
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(2)), None);
}

/// Regression: spec-conformance §13.6.1 round-0 TPR cluster — store-
/// triggered replacement (storing an image with an ImageId that already
/// exists) routes through `remove_image` then `store`. The grid entry
/// for the old image MUST drop; the new image starts with no recorded
/// grid until `set_placeholder_anchor_grid` is called for it. Without
/// this guarantee, a reused ImageId would inherit the prior store's
/// grid dimensions and slice UVs incorrectly.
#[test]
fn store_triggered_replacement_drops_old_placeholder_anchor_grid() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 256)).unwrap();
    cache.add_placeholder_anchor(ImageId(1));
    cache.set_placeholder_anchor_grid(ImageId(1), 4, 4);

    // Re-store image with same ID — eviction path runs remove_image first.
    cache.store(make_image(1, 256)).unwrap();

    assert!(cache.get_no_touch(ImageId(1)).is_some(), "new image stored");
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(1)),
        None,
        "replacement-via-remove_image MUST drop the prior grid entry — \
         reused ImageId starts with no recorded grid until the new \
         transmit registers one"
    );
}

/// Regression: spec-conformance §13.6.1 — `reconcile_placeholder_anchors`
/// drops grid entries for any image_id no longer in the survivor set,
/// keeping the anchor set and grid map synchronized. Without this,
/// stale grid entries accumulate after grid edits erase placeholder
/// cells.
#[test]
fn reconcile_placeholder_anchors_drops_grid_for_evicted_ids() {
    let mut cache = ImageCache::new();
    cache.add_placeholder_anchor(ImageId(1));
    cache.set_placeholder_anchor_grid(ImageId(1), 3, 3);
    cache.add_placeholder_anchor(ImageId(2));
    cache.set_placeholder_anchor_grid(ImageId(2), 5, 1);

    let mut survivors = std::collections::HashSet::new();
    survivors.insert(ImageId(2));
    cache.reconcile_placeholder_anchors(&survivors);

    assert!(
        !cache.placeholder_anchors().contains(&ImageId(1)),
        "anchor for evicted image_id=1 must drop"
    );
    assert_eq!(
        cache.placeholder_anchor_grid_for(ImageId(1)),
        None,
        "grid for evicted image_id=1 must drop alongside the anchor — \
         stale grid entries are a LEAK:scattered-knowledge regression"
    );
    assert!(cache.placeholder_anchors().contains(&ImageId(2)));
    assert_eq!(cache.placeholder_anchor_grid_for(ImageId(2)), Some((5, 1)));
}
