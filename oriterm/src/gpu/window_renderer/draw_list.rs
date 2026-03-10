//! Draw list conversion: append UI and overlay draw commands to the prepared frame.

use super::super::prepared_frame::OverlayDrawRange;
use super::super::state::GpuState;
use super::{CombinedAtlasLookup, WindowRenderer};
use crate::font::size_key;

use super::helpers::{ensure_glyphs_cached, ui_text_raster_keys};

impl WindowRenderer {
    /// Append UI draw commands **with text** from a [`DrawList`].
    ///
    /// Rasterizes uncached UI text glyphs, converts text commands into glyph
    /// instances, and processes clip commands. Writes to the chrome tier
    /// (draws 6–9).
    pub fn append_ui_draw_list_with_text(
        &mut self,
        draw_list: &oriterm_ui::draw::DrawList,
        scale: f32,
        opacity: f32,
        gpu: &GpuState,
    ) {
        self.cache_ui_glyphs(draw_list, scale, gpu);
        let size_q6 = self.ui_size_q6();
        let hinted = self.ui_hinted();

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };
        let vw = self.prepared.viewport.width;
        let vh = self.prepared.viewport.height;

        let mut text_ctx = super::super::draw_list_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.ui_glyphs,
            subpixel_writer: &mut self.prepared.ui_subpixel_glyphs,
            color_writer: &mut self.prepared.ui_color_glyphs,
            size_q6,
            hinted,
        };
        let mut clip_ctx = super::super::draw_list_convert::ClipContext {
            clips: &mut self.prepared.ui_clips,
            stack: &mut self.clip_stack,
            viewport_w: vw,
            viewport_h: vh,
        };
        super::super::draw_list_convert::convert_draw_list(
            draw_list,
            &mut self.prepared.ui_rects,
            Some(&mut text_ctx),
            Some(&mut clip_ctx),
            scale,
            opacity,
        );
    }

    /// Append overlay draw commands **with text** into the overlay tier.
    ///
    /// Each call records an [`OverlayDrawRange`] so the render pass can draw
    /// each overlay as a complete unit (rects → glyphs) before moving to the
    /// next. This ensures correct z-ordering between stacked overlays.
    pub fn append_overlay_draw_list_with_text(
        &mut self,
        draw_list: &oriterm_ui::draw::DrawList,
        scale: f32,
        opacity: f32,
        gpu: &GpuState,
    ) {
        self.cache_ui_glyphs(draw_list, scale, gpu);
        let size_q6 = self.ui_size_q6();
        let hinted = self.ui_hinted();

        // Snapshot buffer positions before conversion.
        let rect_start = self.prepared.overlay_rects.len() as u32;
        let mono_start = self.prepared.overlay_glyphs.len() as u32;
        let sub_start = self.prepared.overlay_subpixel_glyphs.len() as u32;
        let color_start = self.prepared.overlay_color_glyphs.len() as u32;

        // Use scratch clips for this overlay (cleared per call).
        self.overlay_scratch_clips.clear();

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };
        let vw = self.prepared.viewport.width;
        let vh = self.prepared.viewport.height;

        let mut text_ctx = super::super::draw_list_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.overlay_glyphs,
            subpixel_writer: &mut self.prepared.overlay_subpixel_glyphs,
            color_writer: &mut self.prepared.overlay_color_glyphs,
            size_q6,
            hinted,
        };
        let mut clip_ctx = super::super::draw_list_convert::ClipContext {
            clips: &mut self.overlay_scratch_clips,
            stack: &mut self.clip_stack,
            viewport_w: vw,
            viewport_h: vh,
        };
        super::super::draw_list_convert::convert_draw_list(
            draw_list,
            &mut self.prepared.overlay_rects,
            Some(&mut text_ctx),
            Some(&mut clip_ctx),
            scale,
            opacity,
        );

        // Record the range for this overlay.
        let range = OverlayDrawRange {
            rects: (rect_start, self.prepared.overlay_rects.len() as u32),
            mono: (mono_start, self.prepared.overlay_glyphs.len() as u32),
            subpixel: (
                sub_start,
                self.prepared.overlay_subpixel_glyphs.len() as u32,
            ),
            color: (color_start, self.prepared.overlay_color_glyphs.len() as u32),
            clips: self.overlay_scratch_clips.clone(),
        };
        self.prepared.overlay_draw_ranges.push(range);
    }

    /// Cache UI text glyphs referenced by the draw list.
    fn cache_ui_glyphs(
        &mut self,
        draw_list: &oriterm_ui::draw::DrawList,
        scale: f32,
        gpu: &GpuState,
    ) {
        let ui_fc = self
            .ui_font_collection
            .as_mut()
            .unwrap_or(&mut self.font_collection);
        let size_q6 = size_key(ui_fc.size_px());
        let hinted = ui_fc.hinting_mode().hint_flag();

        self.ui_raster_keys.clear();
        ui_text_raster_keys(draw_list, size_q6, hinted, scale, &mut self.ui_raster_keys);
        ensure_glyphs_cached(
            self.ui_raster_keys.iter().copied(),
            &mut self.atlas,
            &mut self.subpixel_atlas,
            &mut self.color_atlas,
            &mut self.empty_keys,
            ui_fc,
            &gpu.queue,
        );
    }

    /// UI font size in 26.6 fixed-point.
    fn ui_size_q6(&self) -> u32 {
        let ui_fc = self
            .ui_font_collection
            .as_ref()
            .unwrap_or(&self.font_collection);
        size_key(ui_fc.size_px())
    }

    /// Whether UI font hinting is enabled.
    fn ui_hinted(&self) -> bool {
        let ui_fc = self
            .ui_font_collection
            .as_ref()
            .unwrap_or(&self.font_collection);
        ui_fc.hinting_mode().hint_flag()
    }
}
