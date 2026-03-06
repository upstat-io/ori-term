//! Draw list conversion: append UI and overlay draw commands to the prepared frame.

use super::super::state::GpuState;
use super::{CombinedAtlasLookup, WindowRenderer};
use crate::font::size_key;

use super::helpers::{ensure_glyphs_cached, ui_text_raster_keys};

impl WindowRenderer {
    /// Append UI draw commands **with text** from a [`DrawList`].
    ///
    /// This method:
    /// 1. Rasterizes uncached UI text glyphs into atlases.
    /// 2. Converts text commands with a real [`TextContext`] so glyph
    ///    instances are emitted into the mono/subpixel/color writers.
    ///
    /// Use this for overlays containing visible text (dialog title, message,
    /// button labels). Call after [`prepare`](Self::prepare) and before
    /// [`render_frame`](Self::render_frame).
    pub fn append_ui_draw_list_with_text(
        &mut self,
        draw_list: &oriterm_ui::draw::DrawList,
        scale: f32,
        opacity: f32,
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

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };

        // Use UI-specific glyph writers so text renders AFTER UI rect
        // backgrounds (draws 7–9) instead of behind them (draws 2–4).
        // Per-text bg_hint is baked into each Text command by the layer stack.
        let mut text_ctx = super::super::draw_list_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.ui_glyphs,
            subpixel_writer: &mut self.prepared.ui_subpixel_glyphs,
            color_writer: &mut self.prepared.ui_color_glyphs,
            size_q6,
            hinted,
        };
        super::super::draw_list_convert::convert_draw_list(
            draw_list,
            &mut self.prepared.ui_rects,
            Some(&mut text_ctx),
            scale,
            opacity,
        );
    }

    /// Append overlay draw commands **with text** into the overlay tier.
    ///
    /// Identical to [`append_ui_draw_list_with_text`](Self::append_ui_draw_list_with_text)
    /// but writes to the overlay buffers (draws 10–13) instead of the chrome
    /// buffers (draws 6–9). This ensures overlay content renders ON TOP of
    /// all chrome text (tab bar titles), not behind it.
    pub fn append_overlay_draw_list_with_text(
        &mut self,
        draw_list: &oriterm_ui::draw::DrawList,
        scale: f32,
        opacity: f32,
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

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };

        let mut text_ctx = super::super::draw_list_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.overlay_glyphs,
            subpixel_writer: &mut self.prepared.overlay_subpixel_glyphs,
            color_writer: &mut self.prepared.overlay_color_glyphs,
            size_q6,
            hinted,
        };
        super::super::draw_list_convert::convert_draw_list(
            draw_list,
            &mut self.prepared.overlay_rects,
            Some(&mut text_ctx),
            scale,
            opacity,
        );
    }
}
