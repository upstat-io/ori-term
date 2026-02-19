//! Font configuration changes: size, hinting, glyph format.
//!
//! Each method delegates to [`FontCollection`] for metrics/cache invalidation,
//! then clears GPU atlases and re-caches ASCII glyphs.

use crate::font::{GlyphFormat, HintingMode};
use crate::gpu::state::GpuState;

use super::GpuRenderer;
use super::helpers::pre_cache_atlas;

impl GpuRenderer {
    // ── Font configuration ──

    /// Change font size, recomputing metrics, clearing atlases, and re-caching.
    ///
    /// Delegates to [`FontCollection::set_size`] for metrics + glyph cache,
    /// then clears all GPU atlases, re-populates the appropriate atlas with
    /// ASCII glyphs, and rebuilds bind groups for the new texture state.
    #[allow(dead_code, reason = "font size change wired in later section")]
    pub fn set_font_size(&mut self, size_pt: f32, dpi: f32, gpu: &GpuState) {
        self.font_collection.set_size(size_pt, dpi);
        self.clear_and_recache(gpu);
    }

    /// Change hinting mode, clearing atlases and re-caching.
    ///
    /// No-ops if the mode is unchanged. Mirrors [`set_font_size`] but only
    /// invalidates the glyph cache and atlases — cell metrics are unaffected
    /// because swash's `Metrics` API (used for cell dimensions) is independent
    /// of the hint flag.
    pub fn set_hinting_mode(&mut self, mode: HintingMode, gpu: &GpuState) {
        if !self.font_collection.set_hinting(mode) {
            return;
        }
        self.clear_and_recache(gpu);
    }

    /// Change rasterization format (e.g. `Alpha` → `SubpixelRgb`), clearing
    /// atlases and re-caching.
    ///
    /// No-ops if the format is unchanged. Typically called once at startup
    /// after the display scale factor is known to enable LCD subpixel
    /// rendering on non-high-DPI displays.
    pub fn set_glyph_format(&mut self, format: GlyphFormat, gpu: &GpuState) {
        if !self.font_collection.set_format(format) {
            return;
        }
        self.clear_and_recache(gpu);
    }

    /// Clear all atlases, re-cache ASCII, and rebuild bind groups.
    fn clear_and_recache(&mut self, gpu: &GpuState) {
        self.atlas.clear();
        self.subpixel_atlas.clear();
        self.color_atlas.clear();

        let format = self.font_collection.format();
        if format.is_subpixel() {
            pre_cache_atlas(
                &mut self.subpixel_atlas,
                &mut self.font_collection,
                &gpu.queue,
            );
        } else {
            pre_cache_atlas(&mut self.atlas, &mut self.font_collection, &gpu.queue);
        }

        self.atlas_bind_group
            .rebuild(&gpu.device, &self.atlas_layout, self.atlas.view());
        self.subpixel_atlas_bind_group.rebuild(
            &gpu.device,
            &self.atlas_layout,
            self.subpixel_atlas.view(),
        );
        self.color_atlas_bind_group.rebuild(
            &gpu.device,
            &self.atlas_layout,
            self.color_atlas.view(),
        );
    }
}
