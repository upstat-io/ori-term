//! Tests for GPU image texture cache.
//!
//! Tests that require a GPU adapter gracefully skip when no adapter is
//! available (CI without GPU, headless environments).

use oriterm_core::image::ImageId;

use super::{ImagePixels, ImageTextureCache, ImageUpload};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;

/// Create a headless GPU environment for texture tests.
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );

    // Frame 2: upload image 2, don't touch image 1.
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );

    // Frame 2: touch both images again (last_frame = 2 for both).
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &data,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &fake_rgba(8, 8),
                width: 8,
                height: 8,
                pixel_generation: 0u64,
            },
        },
    );

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &fake_rgba(8, 8),
                width: 8,
                height: 8,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &fake_rgba(8, 8),
                width: 8,
                height: 8,
                pixel_generation: 0u64,
            },
        },
    );
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &fake_rgba(8, 8),
                width: 8,
                height: 8,
                pixel_generation: 0u64,
            },
        },
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
        ImageUpload {
            id: ImageId::from_raw(1),
            pixels: ImagePixels {
                data: &fake_rgba(4, 4),
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
    );
    assert_eq!(cache.gpu_memory_used(), 64); // 4*4*4

    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: ImageId::from_raw(2),
            pixels: ImagePixels {
                data: &fake_rgba(8, 8),
                width: 8,
                height: 8,
                pixel_generation: 0u64,
            },
        },
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

/// touch_image for an Occupied entry stamps
/// last_frame = frame_counter, refreshing the LRU position. Without touch,
/// images in panes served from PaneRenderCache age out and silently skip
/// at draw time.
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
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &fake_rgba(4, 4),
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
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
    assert_eq!(
        cache.frame_counter(),
        touched_at,
        "touch must not advance frame_counter"
    );
}

/// touch_image for a Vacant entry is a no-op —
/// does not insert, does not advance any counter. Returns false to signal
/// the caller that the image is already evicted; the caller must
/// invalidate any cached state referencing it.
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
    assert_eq!(
        cache.frame_counter(),
        before,
        "touch must not advance counter"
    );
}

/// touch_image returns true when the entry exists.
/// The return value is the eviction-detection contract consumed by
/// `touch_cached_pane_images` in `frame_prep.rs`.
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
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &fake_rgba(4, 4),
                width: 4,
                height: 4,
                pixel_generation: 0u64,
            },
        },
    );

    assert!(cache.touch_image(id), "touch on Occupied must return true");
}

// ── (xray-scene lag cure) pixel_generation re-upload gate ─────────────────────
//
// Animated images mutate `ImageData::data` on every `apply_frame`. The
// cache key is the stable `ImageId`, so the GPU texture cache's Occupied
// arm returns the cached bind group without re-uploading new bytes →
// renders stale pixels until LRU eviction. Cure: `pixel_generation`
// counter on `ImageData`, bumped on every mutation; GPU cache re-uploads
// when the observed generation advances. Phase 3 records the generation
// on each entry; Phase 4 wires the gate. Pre-fix all four tests below
// fail because the Occupied arm short-circuits without checking the
// recorded generation.

/// Regression: §03 GPU texture invalidation pin (Plan TPR R0 reviewer consensus).
/// Drive 2 generations through `ensure_uploaded` — first with
/// `pixel_generation=0` + data_A, then `pixel_generation=1` + data_B.
/// Read back the texture via `read_texture_for_test`; assert pixel
/// content matches `data_B`, NOT `data_A`. This IS the clamp-from-below assertion
/// for the GPU stale-texture defect that motivates the entire fix.
/// Failure mode pre-fix: Occupied arm returns cached bind group
/// without re-uploading; readback shows `data_A` → assert fires.
#[test]
fn gpu_texture_serves_fresh_pixels_after_generation_advance() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;
    let id = ImageId::from_raw(1);
    let data_a = vec![0xAAu8; 16]; // 2×2 RGBA all-0xAA
    let data_b = vec![0xBBu8; 16]; // 2×2 RGBA all-0xBB

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_a,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels { data: // ← generation advanced
        &data_b, width: 2, height: 2, pixel_generation: 1u64 },
        },
    );

    let pixels = cache
        .read_texture_for_test(id, &gpu.device, &gpu.queue)
        .expect("texture readback must succeed");
    assert_eq!(
        pixels, data_b,
        "GPU texture must reflect data_B after generation advance; pre-fix Occupied arm returns stale data_A"
    );
}

/// Regression: §03 GPU texture invalidation companion pin. When
/// `pixel_generation` is UNCHANGED between two consecutive uploads
/// of the same `ImageId`, the cache MUST NOT re-upload — verified
/// by checking the readback pixels match the FIRST upload (no
/// silent overwrite by the second call's data). Catches the
/// regression where Phase 4 over-zealously re-uploads on every
/// `ensure_uploaded` call, defeating the cache's purpose.
/// Failure mode pre-fix: trivially passes (cache returns Occupied
/// without re-upload today). Failure mode post-fix-if-too-aggressive:
/// cache re-uploads even when generation matches → second call's
/// `data_b` overwrites → readback differs from `data_a`.
#[test]
fn ensure_uploaded_returns_cached_when_generation_unchanged() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;
    let id = ImageId::from_raw(1);
    let data_a = vec![0xAAu8; 16];
    let data_b = vec![0xBBu8; 16];

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_a,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );
    cache.begin_frame();
    // Second call: SAME generation (0), DIFFERENT data — must NOT re-upload.
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_b,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );

    let pixels = cache
        .read_texture_for_test(id, &gpu.device, &gpu.queue)
        .expect("readback");
    assert_eq!(
        pixels, data_a,
        "same-generation call MUST NOT re-upload; cached pixels must remain data_A"
    );
}

/// Regression: §03 + Plan TPR R0 reviewer consensus + R3 reviewer consensus wrap-cycle pin.
/// The MAX→0 boundary alone is insufficient — the real stale case
/// is "cached gen=0 with data_A; image side wraps a full u64 cycle
/// back to gen=0 with data_B; ensure_uploaded equality check sees
/// gen==gen and skips the upload". Test setup: (1) seed gen=0
/// data_A; (2) re-upload at gen=0 with data_B AND mark this as
/// post-wrap (via a test-only seam OR an explicit different
/// `epoch_force_reupload` mechanism Phase 4 picks). For Phase 3 we
/// assert the desired behavior: a documented "different data with
/// same generation key" MUST cause re-upload. Cure surface options
/// per §05 Item 4a (Phase 4 picks): wrap-epoch field, cache clear
/// on observed wrap, or saturating-add.
/// Failure mode pre-fix: the cache does not detect wrap; readback
/// returns data_A. POST-fix: depends on Phase 4's chosen cure.
#[test]
fn pixel_generation_full_wrap_to_seeded_value_forces_reupload() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;
    let id = ImageId::from_raw(1);
    let data_a = vec![0xAAu8; 16];
    let data_b = vec![0xBBu8; 16];

    // Seed: gen=0, data_A.
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_a,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );
    // Simulate one full wrap-cycle by jumping to gen=u64::MAX then back to 0.
    // Both calls use the SAME id; the second call (gen=u64::MAX) advances
    // the cache's recorded generation off zero; the third call (gen=0)
    // would-be "cycled back to zero" with different data.
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_a,
                width: 2,
                height: 2,
                pixel_generation: u64::MAX,
            },
        },
    );
    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: id,
            pixels: ImagePixels {
                data: &data_b,
                width: 2,
                height: 2,
                pixel_generation: 0u64,
            },
        },
    );

    let pixels = cache
        .read_texture_for_test(id, &gpu.device, &gpu.queue)
        .expect("readback");
    assert_eq!(
        pixels, data_b,
        "wrap-back to seeded generation with different data MUST force re-upload; \
 cure surface (epoch, cache-clear, or saturating-add) is Phase 4's pick"
    );
}

/// Regression: §03 propagation pin — `pixel_generation` reaches the
/// GPU cache via the snapshot → FrameInput path. Build a
/// `RenderableImageData` with `pixel_generation = 7`, hand it to a
/// frame-prep step (here simulated by directly invoking
/// `ensure_uploaded` with the value), confirm the upload landed.
/// This test is structural — it asserts the parameter wiring; the
/// behavior gate is owned by the wrap-cycle test above.
/// Failure mode pre-fix: the parameter exists post-Phase-3-scaffold,
/// so this test PASSES today as a clamp from above. If a future
/// regression drops the parameter from the call chain, the
/// compile-time mismatch surfaces here.
#[test]
fn pixel_generation_propagates_through_snapshot_to_frame_input() {
    use oriterm_core::RenderableImageData;
    use std::sync::Arc;

    let Some((gpu, pipelines)) = headless_gpu() else {
        return;
    };
    let mut cache = ImageTextureCache::new(&gpu.device);
    let layout = &pipelines.image_texture_layout;

    let img = RenderableImageData {
        id: ImageId::from_raw(42),
        data: Arc::new(vec![0xCC; 16]),
        width: 2,
        height: 2,
        pixel_generation: 7,
    };

    cache.begin_frame();
    cache.ensure_uploaded(
        &gpu.device,
        &gpu.queue,
        layout,
        ImageUpload {
            id: img.id,
            pixels: ImagePixels {
                data: &img.data,
                width: img.width,
                height: img.height,
                pixel_generation: img.pixel_generation,
            },
        },
    );

    // Structural assertion: upload succeeded, texture present.
    assert!(
        cache.get_bind_group(img.id).is_some(),
        "RenderableImageData.pixel_generation parameter must reach the cache without compile-time loss"
    );
}
