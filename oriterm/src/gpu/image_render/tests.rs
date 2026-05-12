//! Tests for GPU image texture cache.
//!
//! Tests that require a GPU adapter gracefully skip when no adapter is
//! available (CI without GPU, headless environments).

use oriterm_core::image::ImageId;

use super::ImageTextureCache;
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;

/// Create a headless GPU environment for texture tests.
///
/// Returns `None` when no adapter is available.
fn headless_gpu() -> Option<(GpuState, GpuPipelines)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    Some((gpu, pipelines))
}

/// Generate fake RGBA pixel data for a `w × h` image.
fn fake_rgba(w: u32, h: u32) -> Vec<u8> {
    vec![128u8; (w as usize) * (h as usize) * 4]
}

// -- Upload and retrieval --

#[test]
fn ensure_uploaded_creates_texture_and_returns_bind_group() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    cache.begin_frame();

    let data = fake_rgba(4, 4);
    let _bg = cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        &pipelines.image_texture_layout,
        ImageId::from_raw(1),
        &data,
        4,
        4,
    );

    assert_eq!(cache.texture_count(), 1);
    assert_eq!(cache.gpu_memory_used(), 4 * 4 * 4);
    assert!(cache.get_bind_group(ImageId::from_raw(1)).is_some());
}

#[test]
fn ensure_uploaded_deduplicates_same_id() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    cache.begin_frame();

    let data = fake_rgba(4, 4);
    let layout = &pipelines.image_texture_layout;

    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &data,
        4,
        4,
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &data,
        4,
        4,
    );

    // Second call is a no-op — only one texture, counted once.
    assert_eq!(cache.texture_count(), 1);
    assert_eq!(cache.gpu_memory_used(), 4 * 4 * 4);
}

// -- Frame-based eviction --

#[test]
fn evict_unused_removes_old_textures() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;
    let data = fake_rgba(2, 2);

    // Frame 1: upload image 1.
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &data,
        2,
        2,
    );

    // Frame 2: upload image 2, don't touch image 1.
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &data,
        2,
        2,
    );

    // Frame 3: advance without touching either.
    cache.begin_frame();

    // Evict textures unused for 1 frame — image 1 was last used at frame 1,
    // image 2 at frame 2, current is 3. Threshold=1 → cutoff=2.
    // Image 1 (last_frame=1 < 2) gets evicted. Image 2 (last_frame=2) survives.
    cache.evict_unused(1);

    assert_eq!(cache.texture_count(), 1);
    assert!(cache.get_bind_group(ImageId::from_raw(1)).is_none());
    assert!(cache.get_bind_group(ImageId::from_raw(2)).is_some());
}

#[test]
fn evict_unused_keeps_recently_used() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;
    let data = fake_rgba(2, 2);

    // Frame 1: upload both images (last_frame = 1 for both).
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &data,
        2,
        2,
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &data,
        2,
        2,
    );

    // Frame 2: touch both images again (last_frame = 2 for both).
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &data,
        2,
        2,
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &data,
        2,
        2,
    );

    // Evict with threshold=1 (cutoff = 2 - 1 = 1).
    // Both at last_frame=2, cutoff=1 → neither evicted (2 >= 1).
    cache.evict_unused(1);
    assert_eq!(cache.texture_count(), 2);
}

// -- Memory limit eviction --

#[test]
fn evict_over_limit_removes_lru() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;

    // 8×8 image = 256 bytes. Set limit to 300 (fits one, not two).
    cache.set_gpu_memory_limit(300);

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &fake_rgba(8, 8),
        8,
        8,
    );

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &fake_rgba(8, 8),
        8,
        8,
    );

    // Over limit: 512 > 300. Evict the oldest (image 1).
    cache.evict_over_limit();

    assert_eq!(cache.texture_count(), 1);
    assert!(cache.get_bind_group(ImageId::from_raw(1)).is_none());
    assert!(cache.get_bind_group(ImageId::from_raw(2)).is_some());
    assert!(cache.gpu_memory_used() <= 300);
}

#[test]
fn set_gpu_memory_limit_triggers_eviction() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &fake_rgba(8, 8),
        8,
        8,
    );
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &fake_rgba(8, 8),
        8,
        8,
    );
    assert_eq!(cache.gpu_memory_used(), 512);

    // Lower limit — should evict immediately.
    cache.set_gpu_memory_limit(256);

    assert_eq!(cache.texture_count(), 1);
    assert!(cache.gpu_memory_used() <= 256);
}

// -- Memory tracking --

#[test]
fn gpu_memory_tracks_uploads_and_removals() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(1),
        &fake_rgba(4, 4),
        4,
        4,
    );
    assert_eq!(cache.gpu_memory_used(), 64); // 4*4*4

    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageId::from_raw(2),
        &fake_rgba(8, 8),
        8,
        8,
    );
    assert_eq!(cache.gpu_memory_used(), 64 + 256); // 320

    cache.remove(ImageId::from_raw(1));
    assert_eq!(cache.gpu_memory_used(), 256);
    assert_eq!(cache.texture_count(), 1);

    cache.remove(ImageId::from_raw(2));
    assert_eq!(cache.gpu_memory_used(), 0);
    assert_eq!(cache.texture_count(), 0);
}

#[test]
fn remove_nonexistent_is_noop() {
    let Some((gpu, _)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    cache.remove(ImageId::from_raw(999));
    assert_eq!(cache.gpu_memory_used(), 0);
    assert_eq!(cache.texture_count(), 0);
}

/// Regression: BUG-06-062 — touch_image for an Occupied entry stamps
/// last_frame = frame_counter, refreshing the LRU position. Without touch,
/// images in panes served from PaneRenderCache age out and silently skip
/// at draw time.
/// See: bug-tracker/plans/BUG-06-062/00-overview.md
#[test]
fn touch_image_stamps_last_frame_for_occupied_entry() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);

    cache.begin_frame();
    let id = ImageId::from_raw(1);
    let _bg = cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        &pipelines.image_texture_layout,
        id,
        &fake_rgba(4, 4),
        4,
        4,
    );

    // Advance to a later frame; touch should stamp the new value.
    for _ in 0..5 {
        cache.begin_frame();
    }
    let touched_at = cache.frame_counter();
    cache.touch_image(id);

    // evict_unused with a threshold of 0 evicts every entry whose last_frame
    // is strictly less than current frame_counter. After touch, last_frame ==
    // touched_at == current frame_counter, so threshold=0 must NOT evict.
    cache.evict_unused(0);
    assert!(
        cache.get_bind_group(id).is_some(),
        "touch_image must stamp last_frame so evict_unused does not drop the entry"
    );
    assert_eq!(cache.frame_counter(), touched_at, "touch must not advance frame_counter");
}

/// Regression: BUG-06-062 — touch_image for a Vacant entry is a no-op —
/// does not insert, does not advance any counter. Returns false to signal
/// the caller that the image is already evicted; the caller must
/// invalidate any cached state referencing it.
/// See: bug-tracker/plans/BUG-06-062/00-overview.md
#[test]
fn touch_image_is_noop_for_vacant_entry() {
    let Some((gpu, _)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    cache.begin_frame();
    let before = cache.frame_counter();
    assert!(
        !cache.touch_image(ImageId::from_raw(9999)),
        "touch on Vacant must return false (eviction-signal contract)"
    );
    assert_eq!(cache.texture_count(), 0, "touch must not insert");
    assert_eq!(cache.gpu_memory_used(), 0, "touch must not allocate");
    assert_eq!(cache.frame_counter(), before, "touch must not advance counter");
}

/// Regression: BUG-06-062 — touch_image returns true when the entry exists.
/// The return value is the eviction-detection contract consumed by
/// `touch_cached_pane_images` in `frame_prep.rs`.
/// See: bug-tracker/plans/BUG-06-062/00-overview.md
#[test]
fn touch_image_returns_true_for_occupied_entry() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    cache.begin_frame();
    let id = ImageId::from_raw(42);
    let _bg = cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        &pipelines.image_texture_layout,
        id,
        &fake_rgba(4, 4),
        4,
        4,
    );

    assert!(
        cache.touch_image(id),
        "touch on Occupied must return true"
    );
}
