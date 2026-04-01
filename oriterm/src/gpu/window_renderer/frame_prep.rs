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

impl WindowRenderer {
    /// Whether visual-only state (selection) changed since the last frame.
    ///
    /// Visual changes need a full instance rebuild but NOT re-shaping.
    fn has_visual_change(&self, input: &FrameInput) -> bool {
        let new_sel = input
            .selection
            .as_ref()
            .and_then(|s| s.damage_snapshot(input.rows()));
        new_sel != self.prepared.prev_selection_snapshot
    }

    /// Run the Prepare phase: shape text and build GPU instance buffers.
    ///
    /// Fills `self.prepared` via buffer reuse (no per-frame allocation after
    /// the first frame).
    ///
    /// The `origin` offset positions the grid on screen (from layout). The
    /// `cursor_blink_visible` flag gates cursor emission (from application
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
        reason = "origin + cursor blink + content_changed are pipeline context"
    )]
    pub fn prepare(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        origin: (f32, f32),
        cursor_blink_visible: bool,
        content_changed: bool,
    ) {
        // Cursor-blink-only fast path: when content hasn't changed and no
        // visual state (selection, search, hover) differs from the last
        // prepared frame, skip shaping, glyph caching, and the full instance
        // rebuild. Just update cursor/URL/prompt overlays.
        let cols = input.columns();
        let cached_valid = self.shaping.frame.rows() > 0 && self.shaping.frame.cols() == cols;
        let visual_changed = self.has_visual_change(input);
        if !content_changed && !visual_changed && cached_valid && self.prepared.has_terminal_data()
        {
            self.atlas.begin_frame();
            self.subpixel_atlas.begin_frame();
            self.color_atlas.begin_frame();
            self.prepared.clear_ephemeral_tiers();
            prepare::update_cursor_only(input, &mut self.prepared, origin, cursor_blink_visible);
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
            cursor_blink_visible,
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

        // Evict textures not used in the last 60 frames (~1 second at 60fps).
        self.image_texture_cache.evict_unused(60);
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
