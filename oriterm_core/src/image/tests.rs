use std::sync::Arc;

use crate::grid::StableRowIndex;

use super::{
    ImageCache, ImageData, ImageError, ImageFormat, ImageId, ImagePlacement, ImageSource,
    decode::{detect_format, rgb_to_rgba},
};

/// Helper: create ImageData with the given ID and byte count of fake RGBA data.
fn make_image(id: u32, bytes: usize) -> ImageData {
    ImageData {
        id: ImageId(id),
        width: 100,
        height: 100,
        data: Arc::new(vec![0u8; bytes]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
    }
}

/// Helper: create a placement at the given cell position.
fn make_placement(image_id: u32, col: usize, row: u64) -> ImagePlacement {
    ImagePlacement {
        image_id: ImageId(image_id),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 100,
        source_h: 100,
        cell_col: col,
        cell_row: StableRowIndex(row),
        cols: 10,
        rows: 5,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
    }
}

// -- ImageCache basics --

#[test]
fn store_and_retrieve_roundtrip() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 1024);
    let id = cache.store(img).unwrap();
    assert_eq!(id, ImageId(1));

    let retrieved = cache.get(id).unwrap();
    assert_eq!(retrieved.width, 100);
    assert_eq!(retrieved.data.len(), 1024);
    assert_eq!(cache.memory_used(), 1024);
    assert_eq!(cache.image_count(), 1);
}

#[test]
fn placement_at_cell_and_viewport_query() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 512);
    cache.store(img).unwrap();

    let p = make_placement(1, 5, 10);
    cache.place(p);
    assert_eq!(cache.placement_count(), 1);

    // Placement at rows 10..14 (5 rows). Query viewport 8..20.
    let visible = cache.placements_in_viewport(StableRowIndex(8), StableRowIndex(20));
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].cell_col, 5);

    // Query viewport 0..5 — placement is outside.
    let visible = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(5));
    assert!(visible.is_empty());

    // Query viewport 14..14 — just the bottom row of placement.
    let visible = cache.placements_in_viewport(StableRowIndex(14), StableRowIndex(14));
    assert_eq!(visible.len(), 1);
}

#[test]
fn memory_limit_triggers_lru_eviction() {
    let mut cache = ImageCache::new();
    // Set small limit: 2048 bytes.
    cache.set_memory_limit(2048);

    let img1 = make_image(1, 1024);
    cache.store(img1).unwrap();
    assert_eq!(cache.image_count(), 1);

    let img2 = make_image(2, 1024);
    cache.store(img2).unwrap();
    assert_eq!(cache.image_count(), 2);
    assert_eq!(cache.memory_used(), 2048);

    // This should evict img1 (oldest, no placements).
    let img3 = make_image(3, 1024);
    cache.store(img3).unwrap();
    assert_eq!(cache.image_count(), 2);
    assert!(cache.get_no_touch(ImageId(1)).is_none());
    assert!(cache.get_no_touch(ImageId(2)).is_some());
    assert!(cache.get_no_touch(ImageId(3)).is_some());
}

#[test]
fn eviction_prefers_unused_images() {
    let mut cache = ImageCache::new();
    cache.set_memory_limit(2048);

    // Store two images.
    let img1 = make_image(1, 1024);
    cache.store(img1).unwrap();
    let img2 = make_image(2, 1024);
    cache.store(img2).unwrap();

    // Place img1 — it has a placement, img2 does not.
    cache.place(make_placement(1, 0, 0));

    // Store a third image — should evict img2 (no placements) not img1.
    let img3 = make_image(3, 1024);
    cache.store(img3).unwrap();
    assert!(
        cache.get_no_touch(ImageId(1)).is_some(),
        "placed image should survive"
    );
    assert!(
        cache.get_no_touch(ImageId(2)).is_none(),
        "unused image evicted first"
    );
    assert!(cache.get_no_touch(ImageId(3)).is_some());
}

#[test]
fn remove_by_id_clears_image_and_placements() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 512);
    cache.store(img).unwrap();
    cache.place(make_placement(1, 0, 0));
    cache.place(make_placement(1, 10, 5));
    assert_eq!(cache.placement_count(), 2);

    cache.remove_image(ImageId(1));
    assert_eq!(cache.image_count(), 0);
    assert_eq!(cache.placement_count(), 0);
    assert_eq!(cache.memory_used(), 0);
}

#[test]
fn remove_specific_placement() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 512);
    cache.store(img).unwrap();

    let mut p1 = make_placement(1, 0, 0);
    p1.placement_id = Some(10);
    let mut p2 = make_placement(1, 5, 5);
    p2.placement_id = Some(20);
    cache.place(p1);
    cache.place(p2);

    cache.remove_placement(ImageId(1), 10);
    assert_eq!(cache.placement_count(), 1);
    assert_eq!(
        cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(100))[0].placement_id,
        Some(20)
    );
}

#[test]
fn remove_by_position() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 512);
    cache.store(img).unwrap();
    cache.place(make_placement(1, 5, 10));
    cache.place(make_placement(1, 20, 30));

    cache.remove_by_position(5, StableRowIndex(10));
    assert_eq!(cache.placement_count(), 1);
}

#[test]
fn prune_scrollback_removes_stale_placements() {
    let mut cache = ImageCache::new();
    let img1 = make_image(1, 512);
    let img2 = make_image(2, 512);
    cache.store(img1).unwrap();
    cache.store(img2).unwrap();

    cache.place(make_placement(1, 0, 5)); // Old row.
    cache.place(make_placement(2, 0, 50)); // Recent row.

    // Evict everything before stable row 20.
    cache.prune_scrollback(StableRowIndex(20));
    assert_eq!(cache.placement_count(), 1);
    assert_eq!(
        cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(100))[0].image_id,
        ImageId(2)
    );

    // img1 had no remaining placements — should be orphaned and removed.
    assert!(cache.get_no_touch(ImageId(1)).is_none());
    assert!(cache.get_no_touch(ImageId(2)).is_some());
}

#[test]
fn remove_placements_in_region() {
    let mut cache = ImageCache::new();
    let img = make_image(1, 512);
    cache.store(img).unwrap();

    // Placement at col=5, row=10, spanning 10 cols x 5 rows.
    cache.place(make_placement(1, 5, 10));
    // Placement at col=20, row=20 (different region).
    cache.place(make_placement(1, 20, 20));

    // Erase region rows 8..16, cols 0..15 — should hit first placement.
    cache.remove_placements_in_region(StableRowIndex(8), StableRowIndex(16), Some(0), Some(15));
    assert_eq!(cache.placement_count(), 1);
    assert_eq!(
        cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(100))[0].cell_col,
        20
    );
}

#[test]
fn clear_removes_everything() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.store(make_image(2, 256)).unwrap();
    cache.place(make_placement(1, 0, 0));
    cache.place(make_placement(2, 5, 5));

    cache.clear();
    assert_eq!(cache.image_count(), 0);
    assert_eq!(cache.placement_count(), 0);
    assert_eq!(cache.memory_used(), 0);
}

#[test]
fn oversized_single_image_rejected() {
    let mut cache = ImageCache::new();
    cache.set_max_single_image(1000);

    let img = make_image(1, 2000);
    let result = cache.store(img);
    assert_eq!(result, Err(ImageError::OversizedImage));
    assert_eq!(cache.image_count(), 0);
}

#[test]
fn dirty_flag_set_on_mutation_cleared_by_take() {
    let mut cache = ImageCache::new();
    assert!(!cache.is_dirty());

    cache.store(make_image(1, 512)).unwrap();
    assert!(cache.is_dirty());

    let was_dirty = cache.take_dirty();
    assert!(was_dirty);
    assert!(!cache.is_dirty());

    // Place sets dirty again.
    cache.place(make_placement(1, 0, 0));
    assert!(cache.is_dirty());
    cache.take_dirty();

    // Remove sets dirty.
    cache.remove_image(ImageId(1));
    assert!(cache.is_dirty());
}

#[test]
fn next_image_id_auto_increments() {
    let mut cache = ImageCache::new();
    let id1 = cache.next_image_id();
    let id2 = cache.next_image_id();
    assert_eq!(id1, ImageId(2_147_483_647));
    assert_eq!(id2, ImageId(2_147_483_648));
}

#[test]
fn get_updates_lru_counter() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.store(make_image(2, 512)).unwrap();

    // After store, img2 has higher last_accessed than img1.
    let img1_before = cache.get_no_touch(ImageId(1)).unwrap().last_accessed;
    let img2_before = cache.get_no_touch(ImageId(2)).unwrap().last_accessed;
    assert!(img2_before > img1_before);

    // Access img1 — makes it more recently used than img2.
    let _ = cache.get(ImageId(1));

    let img1 = cache.get_no_touch(ImageId(1)).unwrap();
    let img2 = cache.get_no_touch(ImageId(2)).unwrap();
    assert!(img1.last_accessed > img2.last_accessed);
}

// -- Format detection --

#[test]
fn detect_png_magic() {
    let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(detect_format(&data), Some(ImageFormat::Png));
}

#[test]
fn detect_jpeg_magic() {
    let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00];
    assert_eq!(detect_format(&data), Some(ImageFormat::Jpeg));
}

#[test]
fn detect_gif_magic() {
    assert_eq!(detect_format(b"GIF89a..."), Some(ImageFormat::Gif));
    assert_eq!(detect_format(b"GIF87a..."), Some(ImageFormat::Gif));
}

#[test]
fn detect_bmp_magic() {
    assert_eq!(detect_format(b"BM\x00\x00\x00"), Some(ImageFormat::Bmp));
}

#[test]
fn detect_webp_magic() {
    let mut data = Vec::from(b"RIFF" as &[u8]);
    data.extend_from_slice(&[0, 0, 0, 0]); // size
    data.extend_from_slice(b"WEBP");
    assert_eq!(detect_format(&data), Some(ImageFormat::WebP));
}

#[test]
fn detect_unknown_format() {
    assert_eq!(detect_format(&[0, 0, 0, 0, 0]), None);
    assert_eq!(detect_format(&[1, 2, 3]), None); // Too short.
    assert_eq!(detect_format(&[]), None);
}

// -- RGB to RGBA conversion --

#[test]
fn rgb_to_rgba_conversion() {
    let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255]; // R, G, B pixels
    let rgba = rgb_to_rgba(&rgb).unwrap();
    assert_eq!(rgba, [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]);
}

#[test]
fn rgb_to_rgba_invalid_length() {
    assert!(rgb_to_rgba(&[1, 2]).is_none());
    assert!(rgb_to_rgba(&[1, 2, 3, 4]).is_none());
}

#[test]
fn rgb_to_rgba_empty() {
    let rgba = rgb_to_rgba(&[]).unwrap();
    assert!(rgba.is_empty());
}

// -- ImageError display --

#[test]
fn image_error_display() {
    assert_eq!(
        ImageError::OversizedImage.to_string(),
        "image exceeds maximum size limit"
    );
    assert_eq!(
        ImageError::InvalidFormat.to_string(),
        "unrecognized image format"
    );
    assert_eq!(
        ImageError::DecodeFailed("bad data".into()).to_string(),
        "image decode failed: bad data"
    );
    assert_eq!(
        ImageError::MemoryLimitExceeded.to_string(),
        "image memory limit exceeded"
    );
}

// -- Decode stub --

#[test]
fn decode_without_feature_returns_error() {
    // This tests the stub when image-protocol feature is not enabled,
    // or the real decode when it is. Either way, calling with garbage
    // data should not panic.
    let result = super::decode::decode_to_rgba(&[0, 1, 2, 3]);
    assert!(result.is_err());
}

// -- Edge cases --

#[test]
fn memory_limit_exceeded_when_single_image_fills_limit() {
    let mut cache = ImageCache::new();
    cache.set_memory_limit(1024);

    // Store one image that fills the limit.
    cache.store(make_image(1, 1024)).unwrap();

    // Try to store another — eviction removes img1, then img2 fits.
    let result = cache.store(make_image(2, 1024));
    assert!(result.is_ok());
    assert_eq!(cache.image_count(), 1);
    assert!(cache.get_no_touch(ImageId(2)).is_some());
}

#[test]
fn set_memory_limit_lower_triggers_eviction() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 1024)).unwrap();
    cache.store(make_image(2, 1024)).unwrap();
    assert_eq!(cache.memory_used(), 2048);

    // Lower limit below current usage.
    cache.set_memory_limit(1024);
    assert!(cache.memory_used() <= 1024);
    assert_eq!(cache.image_count(), 1);
}

#[test]
fn remove_nonexistent_image_is_noop() {
    let mut cache = ImageCache::new();
    cache.remove_image(ImageId(999)); // No panic, no dirty flag.
    assert!(!cache.is_dirty());
}

#[test]
fn remove_nonexistent_placement_is_noop() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();
    cache.take_dirty();
    cache.remove_placement(ImageId(1), 999); // No matching placement.
    assert!(!cache.is_dirty());
}

#[test]
fn viewport_query_with_multi_row_placement() {
    let mut cache = ImageCache::new();
    cache.store(make_image(1, 512)).unwrap();

    // Placement spanning rows 100..109 (10 rows).
    let mut p = make_placement(1, 0, 100);
    p.rows = 10;
    cache.place(p);

    // Query that overlaps just the bottom.
    let v = cache.placements_in_viewport(StableRowIndex(108), StableRowIndex(120));
    assert_eq!(v.len(), 1);

    // Query that overlaps just the top.
    let v = cache.placements_in_viewport(StableRowIndex(95), StableRowIndex(101));
    assert_eq!(v.len(), 1);

    // Query entirely below.
    let v = cache.placements_in_viewport(StableRowIndex(110), StableRowIndex(120));
    assert!(v.is_empty());
}
