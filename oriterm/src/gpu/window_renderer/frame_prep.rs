//! Frame preparation and lifecycle methods for [`WindowRenderer`].
//!
//! Owns the prepare phase (shape → cache → prepare → upload) and
//! post-frame buffer shrinking.

use super::super::frame_input::FrameInput;
use super::super::pipelines::GpuPipelines;
use super::super::prepare;
use super::super::state::GpuState;
use super::helpers::{CombinedAtlasLookup, ensure_glyphs_cached, grid_raster_keys, shape_frame};
use super::{EMPTY_KEYS_CAP, WindowRenderer};

/// Number of frames an image texture may go unused before eviction.
///
/// At a 60Hz refresh this is ~1s of idle retention; higher refresh rates
/// evict proportionally faster, which is acceptable for a frame-cache
/// retention policy (the goal is "evict if not actively used", not a
/// fixed wall-clock budget).
const IMAGE_TEXTURE_EVICT_FRAME_THRESHOLD: u64 = 60;

impl WindowRenderer {
    /// Whether any frame-level dispatch-fingerprint input changed since
    /// the last frame. Single SSOT consumer of the dispatch fingerprint
    /// — replaces the prior `has_geometry_change` plus the
    /// fingerprint-covered subset of `has_visual_change`. Inputs hashed
    /// via [`super::super::prepare::compute_dispatch_fingerprint`]:
    /// viewport, full `CellMetrics`, content dims, origin, blink/palette
    /// /dim opacities, subpixel positioning, search state.
    ///
    /// Bitwise-exact comparison via `.to_bits()` — replaces the prior
    /// `> 0.001` and `< f32::EPSILON` thresholds with one rule. Effect
    /// is extra rebuilds on tiny float deltas, never stale reuse.
    pub(crate) fn has_dispatch_change(&self, input: &FrameInput, origin: (f32, f32)) -> bool {
        let fingerprint = prepare::compute_dispatch_fingerprint(input, origin);
        self.prepared.prev_dispatch_fingerprint != Some(fingerprint)
    }

    /// Whether per-row dirty-tracking inputs changed since the last
    /// frame. These fields are intentionally NOT in the dispatch
    /// fingerprint because they're handled by `build_dirty_set` inside
    /// the incremental prepare pass. But the cursor-only fast path
    /// BYPASSES prepare entirely — bypasses `build_dirty_set` entirely
    /// — so selection/hover/cursor changes MUST gate the fast path
    /// independently of the fingerprint, or stale decorations replay
    /// from the cached terminal tier.
    ///
    /// Cursor gating: the resolved cursor `(line, column, shape, visible)`
    /// is compared via visibility-canonicalized `Option<RenderableCursor>`
    /// `PartialEq`. Hidden-to-hidden cursor position changes canonicalize to
    /// `None == None` (no-op), avoiding WASTE invalidation on invisible-
    /// cursor frames.
    pub(crate) fn has_row_state_change(&self, input: &FrameInput, cursor_opacity: f32) -> bool {
        prepare::evaluate_row_state_change(&self.prepared, input, cursor_opacity)
    }

    /// Run the Prepare phase: shape text and build GPU instance buffers.
    ///
    /// Fills `self.prepared` via buffer reuse (no per-frame allocation after
    /// the first frame).
    ///
    /// The `origin` offset positions the grid on screen (from layout). The
    /// `cursor_opacity` controls cursor emission opacity (from application
    /// blink state) — when `false`, no cursor instances are emitted even
    /// if the terminal reports the cursor as visible.
    ///
    /// When `content_changed` is false the shaping phase is skipped entirely,
    /// reusing the previous frame's [`ShapedFrame`]. Decorations (cursor,
    /// selection, URL hover) only affect the prepare phase, so they work
    /// correctly with cached shaping data.
    ///
    /// Three phases:
    /// 1. **Shape** — segment rows into runs and shape via rustybuzz.
    /// 2. **Cache** — rasterize and upload any missing shaped glyphs.
    /// 3. **Prepare** — emit GPU instances from shaped glyph positions.
    #[expect(
        clippy::too_many_arguments,
        reason = "origin + cursor opacity + content_changed are pipeline context"
    )]
    pub fn prepare(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        origin: (f32, f32),
        cursor_opacity: f32,
        content_changed: bool,
    ) {
        // INVARIANT: cursor-blink-only fast path runs only when content +
        // dispatch fingerprint + row-state are all unchanged.
        let cols = input.columns();
        let cached_valid = self.shaping.frame.rows() > 0 && self.shaping.frame.cols() == cols;
        let dispatch_changed = self.has_dispatch_change(input, origin);
        let row_state_changed = self.has_row_state_change(input, cursor_opacity);

        // SSOT for "did this frame invalidate the content-cache tier?".
        // True when prepare must rebuild instances; false when the fast
        // path can reuse the cached terminal tier (cursor-blink-only frames).
        let can_reuse_content_cache = !content_changed
            && !dispatch_changed
            && !row_state_changed
            && cached_valid
            && self.prepared.has_terminal_data();
        self.cache_invalidated_this_frame = !can_reuse_content_cache;

        if can_reuse_content_cache {
            self.atlas.begin_frame();
            self.subpixel_atlas.begin_frame();
            self.color_atlas.begin_frame();
            self.prepared.clear_ephemeral_tiers();
            prepare::update_cursor_only(input, &mut self.prepared, origin, cursor_opacity);
            return;
        }

        self.atlas.begin_frame();
        self.subpixel_atlas.begin_frame();
        self.color_atlas.begin_frame();

        // Phase A: Shape all rows, or reuse cached shaping when content
        // hasn't changed (mouse hover, cursor blink, selection changes
        // only affect the prepare phase).
        if content_changed || !cached_valid {
            shape_frame(input, &self.font_collection, &mut self.shaping);
        }

        // Phase B + B2: Ensure shaped glyphs + builtin glyphs cached. SSOT
        // helper shared with `prepare_pane_into` so the Phase-B+B2 sequence
        // never drifts between the two render paths.
        self.cache_glyphs_and_builtins(input, gpu);

        // Phase C: Fill prepared frame via combined atlas lookup bridge.
        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };
        prepare::prepare_frame_shaped_into(
            input,
            &bridge,
            &self.shaping.frame,
            &mut self.prepared,
            origin,
            cursor_opacity,
        );

        // Phase D: Ensure image textures uploaded.
        self.upload_image_textures(input, gpu, pipelines);

        log::trace!(
            "frame: cells={} bg_inst={} glyph_inst={} cursor_inst={} images={}",
            input.content.cells.len(),
            self.prepared.backgrounds.len(),
            self.prepared.glyphs.len(),
            self.prepared.cursors.len(),
            self.prepared.image_quads_below.len() + self.prepared.image_quads_above.len(),
        );
    }

    /// Run Phase B + B2 of the prepare pipeline.
    ///
    /// Caches shaped glyphs (routes to mono, subpixel, or color atlas) and
    /// builtin geometric glyphs + decoration patterns. Builtins always go to
    /// the mono atlas (alpha-only bitmaps).
    ///
    /// Single SSOT consumed by both single-pane [`prepare`] and multi-pane
    /// `prepare_pane_into` so the Phase-B+B2 sequence never drifts between
    /// the two render paths (`LEAK:algorithmic-duplication`).
    ///
    /// Pre-condition: [`shape_frame`] (Phase A) has populated
    /// `self.shaping.frame` for the current frame.
    pub(super) fn cache_glyphs_and_builtins(&mut self, input: &FrameInput, gpu: &GpuState) {
        // Phase B: shaped glyphs.
        ensure_glyphs_cached(
            grid_raster_keys(
                &self.shaping.frame,
                self.font_collection.hinting_mode().hint_flag(),
                self.subpixel_positioning,
            ),
            &mut self.atlas,
            &mut self.subpixel_atlas,
            &mut self.color_atlas,
            &mut self.empty_keys,
            &mut self.font_collection,
            &gpu.device,
            &gpu.queue,
        );

        // Phase B2: builtin geometric glyphs + decoration patterns.
        super::super::builtin_glyphs::ensure_builtins_cached(
            input,
            self.shaping.frame.size_q6(),
            &mut self.atlas,
            &mut self.empty_keys,
            &gpu.device,
            &gpu.queue,
        );
    }

    /// Begin the per-frame image-texture-cache lifecycle.
    ///
    /// Advances the LRU frame counter on [`ImageTextureCache`]. Must be
    /// paired with a corresponding [`finish_image_frame`] call to bracket
    /// the per-pane [`ensure_pane_images_uploaded`] uploads. Called ONCE
    /// per visual frame regardless of pane count — the multi-pane path
    /// invokes this in `begin_multi_pane_frame`, NOT per-pane.
    pub(super) fn begin_image_frame(&mut self) {
        debug_assert!(
            !self.image_frame_active,
            "begin_image_frame called while another image frame is active"
        );
        self.image_frame_active = true;
        self.image_texture_cache.begin_frame();
    }

    /// Upload all image textures referenced by this pane / frame.
    ///
    /// Per-pane component of the image-texture-cache lifecycle: ensures
    /// each image in `input.content.image_data` is uploaded (touches
    /// existing entries' LRU position via [`ImageTextureCache::ensure_uploaded`]).
    /// Does NOT advance the frame counter and does NOT evict — those
    /// are bracket-only operations owned by [`begin_image_frame`] and
    /// [`finish_image_frame`].
    pub(super) fn ensure_pane_images_uploaded(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
    ) {
        debug_assert!(
            self.image_frame_active,
            "ensure_pane_images_uploaded called outside begin_image_frame/finish_image_frame bracket"
        );
        for img_data in &input.content.image_data {
            self.image_texture_cache.ensure_uploaded(
                &gpu.device,
                &gpu.queue,
                &pipelines.image_texture_layout,
                img_data.id,
                &img_data.data,
                img_data.width,
                img_data.height,
            );
        }
    }

    /// Finish the per-frame image-texture-cache lifecycle: evict stale + over-limit.
    ///
    /// Runs the eviction passes that bound GPU memory. Called ONCE per
    /// visual frame to keep `frame_counter`-based eviction deltas consistent
    /// (a per-pane finish would tighten the effective retention window
    /// to `THRESHOLD / pane_count`).
    pub(super) fn finish_image_frame(&mut self) {
        debug_assert!(
            self.image_frame_active,
            "finish_image_frame called outside begin_image_frame bracket"
        );
        self.image_texture_cache
            .evict_unused(IMAGE_TEXTURE_EVICT_FRAME_THRESHOLD);
        self.image_texture_cache.evict_over_limit();
        self.image_frame_active = false;
    }

    /// Refresh LRU `last_frame` for images visible in a cached pane.
    ///
    /// Returns `true` iff every image quad referenced by `cached` was
    /// found in `image_texture_cache` and successfully touched. Returns
    /// `false` when one or more referenced images have been evicted
    /// between the original `prepare_pane_into` call (which uploaded them)
    /// and now (typically by `evict_over_limit` firing for another pane's
    /// uploads in the same frame). The caller MUST treat `false` as a
    /// cache invalidation — the cached `PreparedFrame`'s `ImageQuad`s
    /// would silently skip at draw time (via the
    /// [`get_bind_group`](ImageTextureCache::get_bind_group) `None` arm
    /// in `render_helpers.rs::draw_image_quads`) — and re-route through
    /// `prepare_pane_into` to re-upload the evicted textures.
    ///
    /// When the multi-pane redraw loop serves a pane from
    /// `PaneRenderCache` (i.e. `prepare_pane_into` is skipped because the
    /// pane is clean), the cached `PreparedFrame` already contains its
    /// image-quad instances, but the underlying `GpuImageTexture` would
    /// not have its `last_frame` advanced. Over `THRESHOLD` cached
    /// frames, [`evict_unused`] would drop the texture. Called per
    /// `cache_hit` immediately before `prepared.extend_from(cached)`.
    pub(crate) fn touch_cached_pane_images(
        &mut self,
        cached: &super::super::prepared_frame::PreparedFrame,
    ) -> bool {
        debug_assert!(
            self.image_frame_active,
            "touch_cached_pane_images called outside begin_image_frame bracket"
        );
        let mut all_present = true;
        for quad in cached
            .image_quads_below
            .iter()
            .chain(cached.image_quads_above.iter())
        {
            if !self.image_texture_cache.touch_image(quad.image_id) {
                all_present = false;
            }
        }
        all_present
    }

    /// Upload image textures for a single-pane frame.
    ///
    /// Thin wrapper composing the per-frame lifecycle for the single-pane
    /// render path: `begin_image_frame` → `ensure_pane_images_uploaded` →
    /// `finish_image_frame`. Multi-pane callers invoke the three helpers
    /// directly across the `begin_multi_pane_frame` / per-pane loop /
    /// `finish_multi_pane_frame` boundaries.
    fn upload_image_textures(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
    ) {
        self.begin_image_frame();
        self.ensure_pane_images_uploaded(input, gpu, pipelines);
        self.finish_image_frame();
    }

    /// Update the GPU memory limit for image textures.
    ///
    /// Triggers immediate eviction if current usage exceeds the new limit.
    pub fn set_image_gpu_memory_limit(&mut self, limit: usize) {
        self.image_texture_cache.set_gpu_memory_limit(limit);
    }

    /// Shrink grow-only buffers if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste to 2× actual usage.
    /// Also caps `empty_keys` at 10,000 entries to prevent unbounded growth
    /// from pathological glyph-missing scenarios.
    pub fn maybe_shrink_buffers(&mut self) {
        self.prepared.maybe_shrink();
        self.shaping.maybe_shrink();
        if self.empty_keys.len() > EMPTY_KEYS_CAP {
            self.empty_keys.clear();
        }
    }
}
