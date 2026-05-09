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
    pub(crate) fn has_row_state_change(&self, input: &FrameInput) -> bool {
        let cur = prepare::resolve_cursor_state(input).into_visible();
        if cur != self.prepared.prev_resolved_cursor {
            return true;
        }
        let new_sel = input
            .selection
            .as_ref()
            .and_then(|s| s.damage_snapshot(input.rows()));
        if new_sel != self.prepared.prev_selection_snapshot {
            return true;
        }
        if input.hovered_cell != self.prepared.prev_hovered_cell {
            return true;
        }
        false
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
        let row_state_changed = self.has_row_state_change(input);

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

        // Phase B: Ensure shaped glyphs cached (routes to mono, subpixel, or color atlas).
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

        // Phase B2: Ensure built-in geometric glyphs + decoration patterns cached.
        // Built-ins always go to the mono atlas (alpha-only bitmaps).
        super::super::builtin_glyphs::ensure_builtins_cached(
            input,
            self.shaping.frame.size_q6(),
            &mut self.atlas,
            &mut self.empty_keys,
            &gpu.device,
            &gpu.queue,
        );

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

    /// Upload image textures for the current frame.
    ///
    /// Ensures all images referenced by the prepared frame have GPU textures.
    /// Evicts textures that haven't been used recently.
    fn upload_image_textures(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
    ) {
        self.image_texture_cache.begin_frame();

        // Upload textures for all visible images.
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

        self.image_texture_cache
            .evict_unused(IMAGE_TEXTURE_EVICT_FRAME_THRESHOLD);
        self.image_texture_cache.evict_over_limit();
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
