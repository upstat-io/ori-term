//! DPI scale-factor change handler.
//!
//! Extracted from `mod.rs` to keep the parent under the 500-line limit.

use winit::window::WindowId;

use crate::app::App;

use super::DEFAULT_DPI;
use super::config_reload;

impl App {
    /// Re-rasterize fonts and update rendering settings for a new DPI scale.
    ///
    /// Called when the window moves between monitors with different scale
    /// factors. Recalculates font size at physical DPI, updates hinting
    /// and subpixel mode, and clears/recaches glyph atlases.
    ///
    /// `winit_id` identifies the window whose DPI changed. Only that
    /// window's renderer is affected — other windows keep their DPI.
    pub(super) fn handle_dpi_change(&mut self, winit_id: WindowId, scale_factor: f64) {
        let Some(gpu) = &self.gpu else { return };
        let Some(ctx) = self.windows.get_mut(&winit_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_mut() else {
            return;
        };
        let scale = scale_factor as f32;
        let physical_dpi = DEFAULT_DPI * scale;

        // Re-rasterize at new physical DPI. This recomputes cell metrics
        // and clears the glyph cache + GPU atlases.
        renderer.set_font_size(self.config.font.size, physical_dpi, gpu);

        // Update hinting and subpixel mode for the new scale factor.
        let hinting = config_reload::resolve_hinting(&self.config.font, scale_factor);
        let opacity = f64::from(self.config.window.effective_opacity());
        let format = config_reload::resolve_subpixel_mode(&self.config.font, scale_factor, opacity)
            .glyph_format();
        renderer.set_hinting_and_format(hinting, format, gpu);

        // Re-resolve atlas filtering (may change with scale factor).
        let atlas_filter = config_reload::resolve_atlas_filtering(&self.config.font, scale_factor);
        if let Some(pipelines) = &self.pipelines {
            renderer.set_atlas_filtering(atlas_filter, gpu, &pipelines.atlas_layout);
        }

        ctx.pane_cache.invalidate_all();
        ctx.text_cache.clear();
        ctx.root.invalidation_mut().invalidate_all();
        ctx.root.damage_mut().reset();
        ctx.root.mark_dirty();

        // Mark all grid lines dirty so the frame extraction re-reads every
        // cell with the new cell metrics. Without this, the terminal content
        // appears stale until PTY output marks individual lines dirty.
        if let Some(pane_id) = self.active_pane_id_for_window(winit_id) {
            if let Some(mux) = self.mux.as_mut() {
                mux.mark_all_dirty(pane_id);
            }
        }

        // Propagate the new cell metrics to every pane in the
        // affected window so `FixedPixels` image placements refresh
        // their cell coverage. Re-read through `self.windows` because
        // we no longer hold the `ctx` borrow (mux access above
        // required relinquishing it).
        if let Some(ctx) = self.windows.get(&winit_id) {
            if let Some(renderer) = ctx.renderer.as_ref() {
                let cell = renderer.cell_metrics();
                let cell_w = cell.width.round().max(1.0) as u16;
                let cell_h = cell.height.round().max(1.0) as u16;
                self.broadcast_cell_metrics_to_window(winit_id, cell_w, cell_h);
            }
        }
    }
}
