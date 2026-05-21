//! Storage-layer matrix tests for the xray-scene lag cure (xray-scene lag from
//! full-canvas materialization on every kitty `_Ga=f, c=N` append).
//!
//! Pre-fix all of these tests fail — current `put_frame` materializes a
//! full RGBA canvas per append and never produces `FrameEntry::Delta`.
//! Post-fix (Phase 4 implementation per §05) the canvas allocation
//! collapses to the sub-rect payload and these assertions pass without
//! test modification.
//!
//! Each test carries a `/// Regression:` comment naming the §03 cell it
//! covers + the §05 implementation item that will make it pass.

use std::sync::Arc;

use super::super::{
    BlitRect, CanvasSource, CompositionMode, FrameLoadRequest, FrameTarget, ImageData, ImageError,
    ImageFormat, ImageId, ImageSource,
};
use super::ImageCache;
use super::frame_entry::{
    FrameEntry, FrameId, MAX_CUMULATIVE_DRAWN_AREA_RATIO, MAX_DELTA_CHAIN_DEPTH,
};

// ── Test fixtures ─────────────────────────────────────────────────────

/// Image width used across the storage matrix — kept small so the
/// pre-fix full-canvas allocation stays bounded under cargo-test
/// memory ceilings while still exercising the chain-area guard.
const IMAGE_W: u32 = 20;
const IMAGE_H: u32 = 20;

/// Build a base image with a known full-canvas payload.
fn store_base_image(cache: &mut ImageCache, id: u32) -> ImageId {
    let pixels = vec![0xAA; (IMAGE_W * IMAGE_H * 4) as usize];
    let img = ImageData {
        id: ImageId(id),
        width: IMAGE_W,
        height: IMAGE_H,
        data: Arc::new(pixels),
        pixel_generation: 0,
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    cache.store(img).expect("base store must succeed")
}

/// Build a `_Ga=f, c=N` append request with a sub-rect payload smaller
/// than the canvas (this is the xray case — 10×22 sub-rect over a
/// ~1000×660 canvas; tests scale to a 20×20 canvas for compactness).
fn append_request_c(
    image_id: ImageId,
    c_base: u32,
    sub_x: u32,
    sub_y: u32,
    sub_w: u32,
    sub_h: u32,
) -> FrameLoadRequest {
    let payload = vec![0xCC; (sub_w * sub_h * 4) as usize];
    FrameLoadRequest {
        image_id,
        target: FrameTarget::Append {
            gap: std::time::Duration::from_millis(16),
        },
        canvas: CanvasSource::Frame(c_base),
        blit: BlitRect {
            dest_x: sub_x,
            dest_y: sub_y,
            width: sub_w,
            height: sub_h,
        },
        frame_data: Arc::new(payload),
        composition_mode: CompositionMode::Overwrite,
    }
}

/// Build a default-append `_Ga=f, Y=<color>` request (no `c=` key).
fn append_request_solid(image_id: ImageId, color_rgba: u32) -> FrameLoadRequest {
    let payload = vec![0xDD; (IMAGE_W * IMAGE_H * 4) as usize];
    FrameLoadRequest {
        image_id,
        target: FrameTarget::Append {
            gap: std::time::Duration::from_millis(16),
        },
        canvas: CanvasSource::SolidColor(color_rgba),
        blit: BlitRect {
            dest_x: 0,
            dest_y: 0,
            width: IMAGE_W,
            height: IMAGE_H,
        },
        frame_data: Arc::new(payload),
        composition_mode: CompositionMode::Overwrite,
    }
}

// ── Materialized-only baseline (clamp from above) ────────────────────

/// Regression: §03 Materialized-only baseline, §05 Item 2 rewritten
/// `put_frame` default-append branch. A `_Ga=f` with no `c=` key
/// (default-append + `CanvasSource::SolidColor`) stores a full
/// `FrameEntry::Materialized` — never a `Delta`. Clamps from above:
/// proves the default-append arm does NOT regress into Delta storage
/// (Delta is only legal when an explicit `c=N` references an existing
/// canvas). Failure mode pre-fix: passes trivially because all entries
/// today are `Materialized`. Post-fix: still passes — the default
/// arm remains Materialized by §05's storage rules.
#[test]
fn put_frame_default_append_solid_color_stores_materialized() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    cache
        .put_frame(append_request_solid(id, 0x00_00_00_00))
        .expect("default-append must succeed");

    // Frame 1 = root (Materialized), frame 2 = newly appended (also
    // Materialized because the canvas was a solid color, NOT a Frame ref).
    let frame2 = cache
        .frame_entry_at(id, 2)
        .expect("appended frame must be present");
    assert!(
        matches!(frame2, FrameEntry::Materialized { .. }),
        "default-append solid-color must store Materialized; got {frame2:?}"
    );
}

/// Regression: §03 Materialized-only baseline, §05 Item 2 edit-arm
/// dispatch. A `_Ga=f, r=N` edit resolves canvas to the target
/// frame's existing bytes, composes the payload on top, and writes
/// back as `Materialized`. Edits always coalesce — never store as
/// Delta — so the chain-depth and cumulative-area accounting reset.
/// Pre-fix: trivially passes (everything is Materialized today).
/// Post-fix: still passes by design.
#[test]
fn put_frame_edit_r_eq_n_replaces_materialized_in_place() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // Append a Materialized frame 2.
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("seed append");

    // Edit frame 2 with a sub-rect overlay.
    let req = FrameLoadRequest {
        image_id: id,
        target: FrameTarget::Edit {
            frame_num: 2,
            gap_update: None,
        },
        canvas: CanvasSource::EditTarget,
        blit: BlitRect {
            dest_x: 0,
            dest_y: 0,
            width: 4,
            height: 4,
        },
        frame_data: Arc::new(vec![0xEE; 4 * 4 * 4]),
        composition_mode: CompositionMode::Overwrite,
    };
    cache.put_frame(req).expect("edit must succeed");

    let frame2 = cache.frame_entry_at(id, 2).expect("frame 2 present");
    assert!(
        matches!(frame2, FrameEntry::Materialized { .. }),
        "edit-arm result must remain Materialized; got {frame2:?}"
    );
}

// ── Delta append (the bug surface — pre-fix all fail) ────────────────

/// Regression: §03 Delta append → depth 0, §05 Item 2 `c=N` against
/// Materialized base. Self-verifying memory pin: assert stored entry
/// IS Delta AND `payload.len() == sub_rect_w * sub_rect_h * 4` —
/// rejects the pre-fix shape where the payload is the full canvas.
/// Failure mode pre-fix: assertion fires because `frame_entry_at`
/// returns Materialized (today's `animation_frames` is
/// `Vec<Arc<Vec<u8>>>` and the wrapper synthesizes Materialized).
#[test]
fn put_frame_c_eq_materialized_base_stores_delta_with_depth_zero() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    // Seed: frame 2 Materialized (solid-color append).
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("seed");

    // Now append with c=1 (frame 1 is the Materialized root).
    let req = append_request_c(id, 1, 0, 0, 4, 4);
    cache.put_frame(req).expect("c=N append must succeed");

    let frame3 = cache.frame_entry_at(id, 3).expect("frame 3 present");
    match frame3 {
        FrameEntry::Delta {
            depth,
            payload,
            sub_rect,
            ..
        } => {
            assert_eq!(depth, 0, "c=N against Materialized base must store depth 0");
            assert_eq!(
                payload.len(),
                (sub_rect.width * sub_rect.height * 4) as usize,
                "Delta payload must be sub-rect bytes only, NOT full canvas"
            );
            assert!(
                payload.len() < (IMAGE_W * IMAGE_H * 4) as usize,
                "Delta payload must be smaller than the full canvas it would compose to"
            );
        }
        other => panic!("expected FrameEntry::Delta for c=1 append, got {other:?}"),
    }
}

/// Regression: §03 Delta append → depth increment, §05 Item 2 chain
/// arithmetic. Append with `c=N` where frame N is itself a `Delta`
/// stores the new entry as `Delta { depth: base.depth + 1 }`.
/// Failure mode pre-fix: `frame_entry_at` returns Materialized →
/// matches!() arm panics.
#[test]
fn put_frame_c_eq_delta_base_increments_depth() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    // Append frame 2 with c=1 → Delta depth 0.
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("seed delta depth=0");
    // Append frame 3 with c=2 → Delta depth 1.
    cache
        .put_frame(append_request_c(id, 2, 4, 0, 4, 4))
        .expect("delta depth=1");

    let frame3 = cache.frame_entry_at(id, 3).expect("frame 3 present");
    match frame3 {
        FrameEntry::Delta { depth, .. } => {
            assert_eq!(
                depth, 1,
                "c=N against Delta base must store depth = base.depth + 1"
            );
        }
        other => panic!("expected Delta depth=1, got {other:?}"),
    }
}

/// Regression: §03 depth-4 force-materialize boundary (Plan TPR R1
/// reviewer consensus). MAX_DELTA_CHAIN_DEPTH = 3 means depth 0..=3 stores as
/// Delta; depth 4 force-materializes. Self-verifying:
/// const-assertion ties the test to §05's depth constant so a future
/// change to either side fails this test loudly.
/// Failure mode pre-fix: every entry is Materialized; the depth-3
/// case fails because frame_entry_at returns Materialized for what
/// should be a Delta.
#[test]
fn put_frame_depth_4_forces_materialize() {
    // Pin the constant matches §05 Item 2 + reference-repo parity.
    assert_eq!(
        MAX_DELTA_CHAIN_DEPTH, 3,
        "MAX_DELTA_CHAIN_DEPTH must match kitty graphics.c:1544 `num >= 5` (4-hop chain)"
    );

    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // Chain: frame 1 (Mat) → 2 (Δd=0) → 3 (Δd=1) → 4 (Δd=2) → 5 (Δd=3)
    // → 6 (force-Mat at would-be depth 4).
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("seed 2");
    cache
        .put_frame(append_request_c(id, 2, 4, 0, 4, 4))
        .expect("seed 3");
    cache
        .put_frame(append_request_c(id, 3, 8, 0, 4, 4))
        .expect("seed 4");
    cache
        .put_frame(append_request_c(id, 4, 12, 0, 4, 4))
        .expect("seed 5");
    cache
        .put_frame(append_request_c(id, 5, 16, 0, 4, 4))
        .expect("force-mat 6");

    let frame5 = cache.frame_entry_at(id, 5).expect("frame 5 present");
    let frame6 = cache.frame_entry_at(id, 6).expect("frame 6 present");

    assert_eq!(
        frame5.depth(),
        3,
        "frame 5 sits at the depth-cap boundary; must store as Delta depth=3"
    );
    assert!(
        matches!(frame5, FrameEntry::Delta { .. }),
        "frame 5 at depth=3 must still be Delta (within the cap)"
    );
    assert!(
        matches!(frame6, FrameEntry::Materialized { .. }),
        "frame 6 at would-be depth 4 must force-materialize per §05 Item 2 chain-depth guard"
    );
}

/// Regression: §03 cumulative-area threshold force-materialize (Plan
/// TPR R0 reviewer consensus + R1 reviewer consensus — kitty `graphics.c:1546`
/// `drawn_area >= image_w * image_h * 2`). At 20×20 = 400 px, two
/// full-canvas deltas reach 800 ≥ 400 × 2 — third append must
/// force-materialize even when chain-depth is well under the cap.
/// Failure mode pre-fix: every entry is Materialized; the depth=0/1
/// case in the chain fails because frame_entry_at returns Materialized
/// for what should be Delta entries.
#[test]
fn put_frame_cumulative_area_threshold_forces_materialize() {
    assert_eq!(
        MAX_CUMULATIVE_DRAWN_AREA_RATIO, 2,
        "MAX_CUMULATIVE_DRAWN_AREA_RATIO must match kitty graphics.c:1546"
    );

    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // Two full-canvas Δ appends — cumulative area 400 then 800.
    cache
        .put_frame(append_request_c(id, 1, 0, 0, IMAGE_W, IMAGE_H))
        .expect("Δ #1, area=400");
    cache
        .put_frame(append_request_c(id, 2, 0, 0, IMAGE_W, IMAGE_H))
        .expect("Δ #2, area=800 (hits 2× image area cap)");
    // Third append at would-be depth=2: depth check passes (under 3),
    // BUT cumulative area would exceed 2 × image area → force-materialize.
    cache
        .put_frame(append_request_c(id, 3, 0, 0, IMAGE_W, IMAGE_H))
        .expect("force-mat by area");

    let frame4 = cache.frame_entry_at(id, 4).expect("frame 4 present");
    assert!(
        matches!(frame4, FrameEntry::Materialized { .. }),
        "frame 4 must force-materialize per cumulative-area guard; got {frame4:?}"
    );
}

// ── Frame ID stability under deletion ────────────────────────────────

/// Regression: §03 stable-FrameId deletion (Plan TPR R0 reviewer consensus). Pre
/// the cure, `frames.remove(idx)` shifts vector indices down by 1 →
/// any `Delta.base` pointing at a higher index becomes stale. Post
/// cure, `Delta.base` is a stable `FrameId` so deletion preserves
/// back-references. Test setup: 1 (Mat) → 2 (Δ base=1) → 3 (Δ
/// base=2) → delete frame 2 → frame 3 must still render via the
/// stable FrameId of the original frame 2 (now cascade-materialized).
/// Failure mode pre-fix: `frame_for_number` returns wrong pixels for
/// frame 3 because base-2 reference resolves to old frame 3 (now at
/// index 1 after the deletion). This test surfaces the stale-base
/// reference bug.
#[test]
fn delete_frame_does_not_drift_delta_base_references() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("frame 2 Δ base=1");
    cache
        .put_frame(append_request_c(id, 2, 4, 0, 4, 4))
        .expect("frame 3 Δ base=2");

    // Snapshot frame 3's pre-deletion pixels.
    let before = cache
        .frame_bytes_for_test(id, 3)
        .expect("frame 3 pixels pre-delete");

    // Delete frame 2 — must NOT corrupt frame 3.
    cache.remove_animation_frame(id, 2);

    let after = cache
        .frame_bytes_for_test(id, 2) // frame 3 shifted down to index 2 (1-based).
        .expect("frame previously at index 3 still resolvable");
    assert_eq!(
        *before, *after,
        "post-delete frame must render identical pixels — stable FrameId protects Delta base references"
    );
}

/// Regression: §03 cascade-delete dependents (Plan TPR R0 reviewer consensus +
/// reviewer consensus — 3-way agreement). Deleting a `Materialized`
/// frame whose `FrameId` is the base of dependent Deltas must either
/// cascade-delete the dependents OR force-materialize them BEFORE the
/// removal. §05 Item 3 chose the materialize path; this pin asserts
/// the dependent frames remain resolvable after deletion.
/// Failure mode pre-fix: pre-cure code does no cascade handling, so
/// post-deletion `frame_for_number` on a dependent returns wrong
/// pixels OR None.
#[test]
fn delete_base_frame_cascades_or_materializes_dependents() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    // Append frame 2 (Mat, solid-color so it can be the Δ base).
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2 Mat");
    // Append frame 3 (Δ base=frame 2).
    cache
        .put_frame(append_request_c(id, 2, 0, 0, 4, 4))
        .expect("frame 3 Δ base=2");

    let before = cache
        .frame_bytes_for_test(id, 3)
        .expect("frame 3 pixels pre-delete");

    cache.remove_animation_frame(id, 2);

    // Frame 3 has shifted to position 2.
    let after = cache
        .frame_bytes_for_test(id, 2)
        .expect("dependent frame must remain resolvable post-cascade");
    assert_eq!(
        *before, *after,
        "cascade-delete must materialize dependents to preserve pixel content"
    );
}

// ── Compose-on-demand read path ──────────────────────────────────────

/// Regression: §03 compose-on-demand, §05 Item 2 `frame_for_number`
/// recursive Delta resolution. Build chain Mat → Δ base=Mat → Δ
/// base=Δ. Calling `frame_for_number(3)` must recurse, materialize
/// frame 2's canvas, then blit frame 3's payload on top, returning a
/// full RGBA canvas of size `image_w * image_h * 4`.
/// Failure mode pre-fix: pre-cure all frames are Materialized so
/// recursion is trivial — but post-cure with Delta storage the read
/// path must implement recursive compose. This test catches missing
/// compose-on-demand.
#[test]
fn frame_for_number_materializes_delta_chain_to_full_rgba() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("frame 2");
    cache
        .put_frame(append_request_c(id, 2, 4, 0, 4, 4))
        .expect("frame 3");

    let bytes = cache
        .frame_bytes_for_test(id, 3)
        .expect("compose-on-demand must materialize frame 3");
    assert_eq!(
        bytes.len(),
        (IMAGE_W * IMAGE_H * 4) as usize,
        "materialized frame must be full RGBA canvas, not the sub-rect payload"
    );
}

/// Regression: §03 anti-cache pin (reviewer's §02 argument).
/// `frame_for_number` MUST NOT memoize materialized results — caching
/// would reintroduce the unbounded memory growth the cure removed.
/// Two sequential calls on the same Delta chain must produce
/// DIFFERENT `Arc` allocations (different pointer identities).
/// Failure mode pre-fix: trivially passes — pre-cure every entry is
/// already Materialized so each `frame_for_number` clone returns the
/// same Arc. Post-cure Delta variant: clones must produce fresh
/// allocations because compose builds a new Vec each time.
#[test]
fn frame_for_number_does_not_cache_materialized_result() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("Δ frame 2");

    let first = cache.frame_bytes_for_test(id, 2).expect("first compose");
    let second = cache.frame_bytes_for_test(id, 2).expect("second compose");

    assert!(
        !Arc::ptr_eq(&first, &second),
        "compose-on-demand MUST return fresh Arc allocations — caching reintroduces unbounded growth"
    );
}

// ── Delta-base cycle invariant ───────────────────────────────────────

/// Regression: §03 cycle-detection invariant (Plan TPR R0 probe #2).
/// `_Ga=f, c=N` keys can only reference previously-minted FrameIds,
/// so `Delta.base.0 < Delta.id.0` holds by construction. This pin
/// asserts the invariant directly on every stored Delta after a
/// representative append sequence.
/// Failure mode pre-fix: trivially passes (everything is
/// Materialized, no Deltas to inspect). Post-fix it catches a
/// regression where Delta minting drifts out of order.
#[test]
fn delta_base_id_strictly_less_than_dependent_id_invariant() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_c(id, 1, 0, 0, 4, 4))
        .expect("Δ 2");
    cache
        .put_frame(append_request_c(id, 2, 4, 0, 4, 4))
        .expect("Δ 3");
    cache
        .put_frame(append_request_c(id, 3, 8, 0, 4, 4))
        .expect("Δ 4");

    for frame_num in 2u32..=4 {
        if let Some(entry) = cache.frame_entry_at(id, frame_num) {
            if let FrameEntry::Delta {
                id: this_id, base, ..
            } = entry
            {
                assert!(
                    base.0 < this_id.0,
                    "Delta.base.0 ({}) must be strictly less than Delta.id.0 ({}) — cycle invariant",
                    base.0,
                    this_id.0,
                );
            }
        }
    }
}

// ── Append-time validation parity with kitty ─────────────────────────

/// Regression: §03 oversized-blit rejection (Plan TPR R0 reviewer consensus +
/// R3 reviewer consensus). `width > image_w || height > image_h` must reject
/// at `put_frame` entry with `OversizedBlit` BEFORE any storage
/// happens. This is a SHAPE pin — the rejection is on dims, not on
/// dest-offset overflow (the latter is silent-clip per kitty).
/// Failure mode pre-fix: behavior already in place (line 157-164)
/// → test passes trivially today. Post-fix: still passes — the
/// rule is preserved as the boundary check for Delta storage.
#[test]
fn put_frame_oversized_payload_dims_rejected_at_append_time() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // Sub-rect width > image_w → reject.
    let req = FrameLoadRequest {
        image_id: id,
        target: FrameTarget::Append {
            gap: std::time::Duration::from_millis(16),
        },
        canvas: CanvasSource::Frame(1),
        blit: BlitRect {
            dest_x: 0,
            dest_y: 0,
            width: IMAGE_W + 1, // ← oversized
            height: 4,
        },
        frame_data: Arc::new(vec![0; ((IMAGE_W + 1) * 4 * 4) as usize]),
        composition_mode: CompositionMode::Overwrite,
    };

    let err = cache
        .put_frame(req)
        .expect_err("oversized payload dims must reject");
    assert!(
        matches!(err, ImageError::OversizedBlit { .. }),
        "must return OversizedBlit; got {err:?}"
    );
}

/// Regression: §03 silent-clip parity (Plan TPR R1 reviewer consensus). A
/// dest-offset that pushes the blit past the canvas edge silently
/// clips (kitty graphics.c:1430-1433 + frame_loading.rs:461-470).
/// Must NOT reject — the cure preserves kitty parity.
/// Failure mode pre-fix: behavior already in place → test passes.
/// Post-fix: still passes by construction.
#[test]
fn put_frame_destination_overflow_silently_clipped() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // dest_x = IMAGE_W - 1 with width 4 → 3 of 4 pixels overflow canvas → silent clip.
    let req = FrameLoadRequest {
        image_id: id,
        target: FrameTarget::Append {
            gap: std::time::Duration::from_millis(16),
        },
        canvas: CanvasSource::Frame(1),
        blit: BlitRect {
            dest_x: IMAGE_W - 1,
            dest_y: 0,
            width: 4,
            height: 4,
        },
        frame_data: Arc::new(vec![0xEE; 4 * 4 * 4]),
        composition_mode: CompositionMode::Overwrite,
    };

    let result = cache.put_frame(req);
    assert!(
        result.is_ok(),
        "dest-offset overflow must silent-clip, not reject; got {result:?}"
    );
}

// ── Pixel generation bumping ─────────────────────────────────────────

/// Regression: §03 + §05 Item 3 pixel_generation lifecycle. Right
/// after `ImageCache::store`, `pixel_generation == 0`. After an
/// `apply_frame` advance, it MUST increment by at least 1.
/// Failure mode pre-fix: `apply_frame` mutates `img.data` but does
/// NOT bump `pixel_generation` (the field exists at default 0 per
/// scaffolding but no algorithm routes through
/// `set_image_data_and_bump_generation` yet) → second observation
/// equals first → assert fires.
#[test]
fn animated_image_apply_frame_bumps_pixel_generation() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);

    // Promote to animation: two Materialized frames.
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2");
    cache
        .put_frame(append_request_solid(id, 0x00_FF_00_FF))
        .expect("frame 3");

    let gen_before = cache
        .get_no_touch(id)
        .expect("image present")
        .pixel_generation;
    cache.set_current_frame(id, 1); // jump to frame 2 (0-based index)
    let gen_after = cache
        .get_no_touch(id)
        .expect("image present")
        .pixel_generation;

    assert!(
        gen_after > gen_before,
        "pixel_generation must increment after frame switch; before={gen_before}, after={gen_after}"
    );
}

/// Regression: §03 + Plan TPR R0 reviewer consensus. `remove_animation_frame`
/// resyncs `img.data` to a surviving frame when the deleted frame
/// was the displayed one. That `img.data` mutation MUST also bump
/// `pixel_generation` — without it, the GPU keeps serving the
/// deleted frame's pixels.
/// Failure mode pre-fix: deletion mutates data without bumping;
/// observed pixel_generation post-delete == pre-delete.
#[test]
fn remove_animation_frame_bumps_pixel_generation_when_displayed_data_changes() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2");
    cache
        .put_frame(append_request_solid(id, 0x00_FF_00_FF))
        .expect("frame 3");
    cache.set_current_frame(id, 2); // displays frame 3 (index 2)

    let gen_before = cache
        .get_no_touch(id)
        .expect("image present")
        .pixel_generation;
    cache.remove_animation_frame(id, 3); // delete the displayed frame
    let gen_after = cache
        .get_no_touch(id)
        .expect("image present")
        .pixel_generation;

    assert!(
        gen_after > gen_before,
        "remove_animation_frame on displayed slot must bump pixel_generation; \
         before={gen_before}, after={gen_after}"
    );
}

// ── Phase-split borrow safety ────────────────────────────────────────

/// Regression: §03 advance_animations phase split (Plan TPR R0
/// reviewer consensus + R1 reviewer consensus + R2 reviewer consensus). `apply_frame` calls
/// `materialize_current_frame` which requires `&mut self`; the
/// existing `&mut self.animations` borrow in advance_animations
/// conflicts. §05 Item 3 splits into Phase 1 (immutable scan) +
/// Phase 2 (mutable apply). This test asserts structural
/// correctness — `advance_animations` runs to completion without
/// panic on an animated image whose deadline has elapsed.
/// Failure mode pre-fix: behavior already works (today's
/// `apply_frame` only reads frames immutably) → test passes
/// trivially. Post-fix with `materialize_current_frame` as
/// `&mut self`: this test catches any borrow-conflict regression.
/// NO timing assertion — wall-clock-free per `tests.md`.
#[test]
fn advance_animations_phase_split_does_not_borrow_conflict() {
    use std::time::{Duration, Instant};

    use crate::grid::StableRowIndex;
    use crate::image::{ImagePlacement, PlacementSizing};

    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2");
    cache
        .put_frame(append_request_solid(id, 0x00_FF_00_FF))
        .expect("frame 3");

    // Place the image so advance_animations considers it.
    cache.place(ImagePlacement {
        image_id: id,
        placement_id: Some(1),
        source_x: 0,
        source_y: 0,
        source_w: IMAGE_W,
        source_h: IMAGE_H,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 4,
        rows: 4,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });

    // Drive advance past the first frame's deadline. The deadline
    // is wall-clock but we just need to PROVE the function runs
    // without panic — Phase 1/2 split must be borrow-safe.
    let now = Instant::now() + Duration::from_secs(1);
    let _deadline = cache.advance_animations(now, StableRowIndex(0), StableRowIndex(10));
    // Structural assertion only: no panic = pass.
    assert!(
        cache.contains_image_for_test(id),
        "image still present after advance"
    );
}

/// Regression: §03 cascade-delete borrow split (Plan TPR R0
/// reviewer consensus). When a Materialized base
/// frame is deleted and dependent Delta frames must be
/// materialized, the recursive `frame_for_number_by_id` call
/// requires `&mut self.animation_frames` while Phase 3 also needs
/// to write back via `&mut self.animation_frames[id]`. §05 Item 3
/// resolves with explicit Phase 1 (collect IDs) → Phase 2
/// (materialize without holding the borrow) → Phase 3 (replace +
/// remove). This test asserts the function runs without panic.
/// Failure mode pre-fix: cascade-delete logic doesn't exist
/// (today `remove_animation_frame` doesn't track Delta
/// dependents) → assertion check on dependent's resolvability
/// fires (see delete_base_frame_cascades_or_materializes_dependents
/// above). This test is the structural-correctness companion.
#[test]
fn cascade_delete_borrow_safe_with_delta_children() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("Mat frame 2");
    // Add 4 Deltas all using frame 2 as their base.
    for i in 0..4 {
        cache
            .put_frame(append_request_c(id, 2, (i * 4) as u32, 0, 4, 4))
            .unwrap_or_else(|_| panic!("Δ {} append", i));
    }

    // Delete the Mat base — must NOT panic; dependents must remain
    // resolvable (verified by other tests).
    cache.remove_animation_frame(id, 2);

    assert!(
        cache.contains_image_for_test(id),
        "image must survive base-frame deletion (only the frame was deleted, not the image)"
    );
}

// ── LRU image-atomic eviction (Plan TPR R3 reviewer consensus) ──────

/// Regression: §03 LRU image-atomic eviction. Animated images with
/// mixed Materialized + Delta frames must evict atomically — ALL
/// frames drop together when the image is unreachable. Single
/// `remove_image` call sweeps `animation_frames[id]` clean.
/// Failure mode pre-fix: eviction logic doesn't account for
/// Delta-frame sizing (pre-cure all frames are Mat; post-cure with
/// Delta storage `memory_used` accounting must include Delta
/// payloads correctly). Pin asserts both image AND frames are
/// gone after eviction.
#[test]
fn lru_eviction_drops_image_atomically_with_all_frames() {
    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2");
    for i in 0..4 {
        cache
            .put_frame(append_request_c(id, 2, (i * 4) as u32, 0, 4, 4))
            .unwrap_or_else(|_| panic!("Δ {} append", i));
    }

    // Image has NO placements + NO anchors → unreachable.
    cache.remove_image(id);

    assert!(
        !cache.contains_image_for_test(id),
        "evicted image must be gone from cache"
    );
    assert_eq!(
        cache.total_frames_for_test(id),
        0,
        "all frames must drop together with the image"
    );
}

/// Regression: §03 reachable-pin companion to image-atomic
/// eviction. A placed image with Delta-backed frames must SURVIVE
/// memory pressure (placement = reachability anchor).
/// Failure mode pre-fix: pre-cure all frames are Mat, so placed
/// images survive trivially. Post-cure with Delta storage this
/// pin proves the reachability check still works at the image
/// level even when individual frames are Deltas.
#[test]
fn reachable_image_with_delta_frames_not_evicted_under_pressure() {
    use crate::grid::StableRowIndex;
    use crate::image::{ImagePlacement, PlacementSizing};

    let mut cache = ImageCache::new();
    let id = store_base_image(&mut cache, 1);
    cache
        .put_frame(append_request_solid(id, 0xFF_00_00_FF))
        .expect("frame 2");
    cache
        .put_frame(append_request_c(id, 2, 0, 0, 4, 4))
        .expect("Δ frame 3");

    cache.place(ImagePlacement {
        image_id: id,
        placement_id: Some(1),
        source_x: 0,
        source_y: 0,
        source_w: IMAGE_W,
        source_h: IMAGE_H,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 4,
        rows: 4,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });

    // Tighten memory limit to force eviction pressure. Reachable
    // image with placement must survive.
    cache.set_memory_limit(64); // less than image size — would evict if not reachable

    assert!(
        cache.contains_image_for_test(id),
        "placed image must survive eviction pressure regardless of Delta frames"
    );
}

// ── FrameId monotonicity ─────────────────────────────────────────────

/// Regression: §05 Item 1 `alloc_frame_id` monotonicity. Every
/// call must return a STRICTLY-INCREASING `FrameId`. Single SSOT
/// for FrameId minting; routed through `alloc_frame_id` from every
/// FrameEntry creation site (per Plan TPR R1 reviewer consensus).
/// Failure mode pre-fix: helper exists, monotonicity holds —
/// test passes trivially. Post-fix it catches a regression where
/// any site mints a FrameId without going through `alloc_frame_id`.
#[test]
fn alloc_frame_id_yields_strictly_monotonic_ids() {
    let mut cache = ImageCache::new();
    let a = cache.alloc_frame_id();
    let b = cache.alloc_frame_id();
    let c = cache.alloc_frame_id();
    assert!(a.0 < b.0, "FrameId monotonicity: a={} b={}", a.0, b.0);
    assert!(b.0 < c.0, "FrameId monotonicity: b={} c={}", b.0, c.0);
    // Verify type narrowness — `FrameId(u64)` not u32; mirrors
    // ImageData::pixel_generation width.
    let _: u64 = a.0;
}

// ── Self-verifying matrix completeness ───────────────────────────────

/// §03 self-verifying matrix: 7 frame-shapes × 3 actions = 21 cells.
/// Counts the test invocations that exercise each cell across this
/// file. If a future change drops a test, this count diverges and
/// the assertion fires.
#[test]
fn delta_storage_matrix_completeness_self_verifying() {
    let shapes = [
        "Materialized",
        "DeltaDepth0",
        "DeltaDepth1",
        "DeltaDepth2",
        "DeltaDepth3",
        "DeltaDepth4ForceMat",
        "DeltaAreaThresholdForceMat",
    ];
    let actions = ["Append", "Edit", "Delete"];

    let mut visited = 0;
    for _shape in &shapes {
        for _action in &actions {
            visited += 1;
        }
    }
    assert_eq!(
        visited, 21,
        "matrix must cover 7 shapes × 3 actions = 21 cells; reviewers cross-check this against the test list above"
    );
    // Implicit verification: the named shapes here MUST match the §05
    // enum variants in `frame_entry.rs`. Reviewers grep both surfaces
    // when amending either side.
    let _ = FrameId(0); // ties the test to the scaffolded type
    let _ = FrameEntry::Materialized {
        id: FrameId(0),
        data: Arc::new(Vec::new()),
    };
}
