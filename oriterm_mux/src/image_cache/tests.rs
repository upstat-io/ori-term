//! Unit tests for the bounded LRU image cache.
//!
//! See:
//! §"Cache pins (cluster-1 + R2 fixes — apply BOTH sides via shared ImageCache)"

use std::sync::Arc;

use oriterm_core::{ImageId, RenderableImageData};

use crate::PaneId;

use super::ImageCache;

fn mk_image(id: u32, bytes: usize) -> Arc<RenderableImageData> {
    Arc::new(RenderableImageData {
        id: ImageId::from_raw(id),
        data: Arc::new(vec![0xAB; bytes]),
        width: 1,
        height: bytes as u32,
        pixel_generation: 0,
    })
}

/// All entries reachable — no eviction even past cap.
#[test]
fn reachability_bounded_eviction_keeps_all_referenced() {
    let mut cache = ImageCache::with_memory_cap(100);
    let pane = PaneId::from_raw(1);
    // 3 × 80-byte entries = 240 bytes, far past 100-byte cap.
    let evicted_a = cache.insert(pane, ImageId::from_raw(1), mk_image(1, 80), |_, _| true);
    let evicted_b = cache.insert(pane, ImageId::from_raw(2), mk_image(2, 80), |_, _| true);
    let evicted_c = cache.insert(pane, ImageId::from_raw(3), mk_image(3, 80), |_, _| true);
    assert!(evicted_a.is_empty());
    assert!(evicted_b.is_empty());
    assert!(evicted_c.is_empty());
    assert_eq!(cache.len(), 3);
    assert!(cache.memory_used() > cache.memory_cap());
}

/// LRU eviction evicts oldest unreferenced entry when cap exceeded.
#[test]
fn evicts_oldest_unreferenced_when_cap_exceeded() {
    let mut cache = ImageCache::with_memory_cap(100);
    let pane = PaneId::from_raw(1);
    // Insert 3 × 80-byte entries with NONE referenced — cap=100 forces eviction
    // back to ≤100 bytes.
    cache.insert(pane, ImageId::from_raw(1), mk_image(1, 80), |_, _| false);
    cache.insert(pane, ImageId::from_raw(2), mk_image(2, 80), |_, _| false);
    let evicted = cache.insert(pane, ImageId::from_raw(3), mk_image(3, 80), |_, _| false);
    // 240 bytes inserted; cap=100; eviction must drop entries down to ≤100.
    assert!(cache.memory_used() <= cache.memory_cap());
    // The evicted set is non-empty.
    assert!(!evicted.is_empty(), "expected at least one eviction");
    // The MOST RECENT entry (id=3) must survive.
    assert!(cache.get(pane, ImageId::from_raw(3)).is_some());
}

/// Drop-pane removes every entry with matching PaneId.
#[test]
fn drop_pane_removes_all_entries() {
    let mut cache = ImageCache::with_memory_cap(1_000_000);
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);
    cache.insert(pane_a, ImageId::from_raw(1), mk_image(1, 100), |_, _| false);
    cache.insert(pane_a, ImageId::from_raw(2), mk_image(2, 100), |_, _| false);
    cache.insert(pane_b, ImageId::from_raw(1), mk_image(1, 100), |_, _| false);
    assert_eq!(cache.len(), 3);
    cache.drop_pane(pane_a);
    assert_eq!(cache.len(), 1);
    assert!(cache.get(pane_a, ImageId::from_raw(1)).is_none());
    assert!(cache.get(pane_a, ImageId::from_raw(2)).is_none());
    assert!(cache.get(pane_b, ImageId::from_raw(1)).is_some());
    assert_eq!(cache.memory_used(), 100);
}

/// `get` bumps LRU — a recently-touched entry is NOT evicted preferentially.
#[test]
fn get_bumps_lru_and_protects_recent_entry() {
    let mut cache = ImageCache::with_memory_cap(100);
    let pane = PaneId::from_raw(1);
    cache.insert(pane, ImageId::from_raw(1), mk_image(1, 50), |_, _| false);
    cache.insert(pane, ImageId::from_raw(2), mk_image(2, 50), |_, _| false);
    // Touch id=1 — should move it to the back of LRU.
    let _ = cache.get(pane, ImageId::from_raw(1));
    // Insert id=3 — pushes total to 150; cap=100; eviction must drop oldest
    // (which is now id=2, not id=1).
    let evicted = cache.insert(pane, ImageId::from_raw(3), mk_image(3, 50), |_, _| false);
    assert!(evicted.contains(&(pane, ImageId::from_raw(2))));
    assert!(cache.get(pane, ImageId::from_raw(1)).is_some());
    assert!(cache.get(pane, ImageId::from_raw(2)).is_none());
    assert!(cache.get(pane, ImageId::from_raw(3)).is_some());
}

/// Cross-pane same raw ImageId — keyed by `(PaneId, ImageId)`, NOT global.
/// ImageId is per-Term-instance scoped (see `oriterm_core::image::mod.rs`), so a
/// global-key cache would corrupt multi-pane rendering.
#[test]
fn cross_pane_same_image_id_stores_distinct_entries() {
    let mut cache = ImageCache::with_memory_cap(1_000_000);
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);
    let id = ImageId::from_raw(2_147_483_647); // AUTO_ID_START — collision case
    let img_a = mk_image(2_147_483_647, 50);
    let img_b = mk_image(2_147_483_647, 80);
    cache.insert(pane_a, id, img_a.clone(), |_, _| false);
    cache.insert(pane_b, id, img_b.clone(), |_, _| false);
    assert_eq!(cache.len(), 2);
    let a = cache.get(pane_a, id).unwrap();
    let b = cache.get(pane_b, id).unwrap();
    assert!(Arc::ptr_eq(&a, &img_a));
    assert!(Arc::ptr_eq(&b, &img_b));
}

/// Replacing an existing entry releases the old bytes from `memory_used`.
#[test]
fn replace_existing_releases_old_bytes() {
    let mut cache = ImageCache::with_memory_cap(1_000_000);
    let pane = PaneId::from_raw(1);
    let id = ImageId::from_raw(1);
    cache.insert(pane, id, mk_image(1, 200), |_, _| false);
    assert_eq!(cache.memory_used(), 200);
    cache.insert(pane, id, mk_image(1, 50), |_, _| false);
    assert_eq!(cache.memory_used(), 50);
    assert_eq!(cache.len(), 1);
}
