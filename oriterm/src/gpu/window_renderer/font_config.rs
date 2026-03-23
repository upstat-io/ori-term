//! Font configuration changes: size, hinting, glyph format.
//!
//! Each method delegates to [`FontCollection`] for metrics/cache invalidation,
//! then clears GPU atlases and re-caches ASCII glyphs.

use crate::font::{FontCollection, GlyphFormat, HintingMode};
use crate::gpu::state::GpuState;

use super::WindowRenderer;
use super::helpers::pre_cache_atlas;

impl WindowRenderer {
    // ── Font configuration ──

    /// Replace the entire font collection (family, weight, features changed).
    ///
    /// Clears all GPU atlases and re-caches ASCII glyphs with the new fonts.
    /// Returns the old cell metrics so callers can detect size changes.
    pub fn replace_font_collection(&mut self, collection: FontCollection, gpu: &GpuState) {
        self.font_collection = collection;
        self.clear_and_recache(gpu);
    }

    /// Change font size, recomputing metrics, clearing atlases, and re-caching.
    ///
    /// Delegates to [`FontCollection::set_size`] for metrics + glyph cache,
    /// then clears all GPU atlases, re-populates the appropriate atlas with
    /// ASCII glyphs, and rebuilds bind groups for the new texture state.
    ///
    /// The `dpi` parameter is the physical DPI (encodes scale). If it changed
    /// (window moved to a different-DPI monitor), the UI font registry is
    /// rebuilt at the new physical sizes.
    pub fn set_font_size(&mut self, size_pt: f32, dpi: f32, gpu: &GpuState) {
        if let Err(e) = self.font_collection.set_size(size_pt, dpi) {
            log::error!("font set_size failed: {e}");
        }
        // UI font sizes are keyed by logical pixels, not terminal size.
        // Only rebuild if DPI changed (physical sizes differ).
        if let Some(sizes) = &mut self.ui_font_sizes {
            if let Err(e) = sizes.set_dpi(dpi) {
                log::error!("UI font registry DPI update failed: {e}");
            }
        }
        self.clear_and_recache(gpu);
    }

    /// Change hinting mode, clearing atlases and re-caching.
    ///
    /// No-ops if the mode is unchanged. Mirrors [`set_font_size`] but only
    /// invalidates the glyph cache and atlases — cell metrics are unaffected
    /// because swash's `Metrics` API (used for cell dimensions) is independent
    /// of the hint flag.
    ///
    /// Prefer [`set_hinting_and_format`] when both change together (e.g.
    /// scale factor change) to avoid a double clear-and-recache.
    #[allow(
        dead_code,
        reason = "individual setter kept for single-property changes"
    )]
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
    ///
    /// Prefer [`set_hinting_and_format`] when both change together (e.g.
    /// scale factor change) to avoid a double clear-and-recache.
    #[allow(
        dead_code,
        reason = "individual setter kept for single-property changes"
    )]
    pub fn set_glyph_format(&mut self, format: GlyphFormat, gpu: &GpuState) {
        if !self.font_collection.set_format(format) {
            return;
        }
        self.clear_and_recache(gpu);
    }

    /// Change both hinting mode and glyph format, clearing atlases once.
    ///
    /// Used during scale factor changes where both settings typically change
    /// together. Avoids the double clear-and-recache that would happen from
    /// calling [`set_hinting_mode`] and [`set_glyph_format`] separately.
    pub fn set_hinting_and_format(
        &mut self,
        mode: HintingMode,
        format: GlyphFormat,
        gpu: &GpuState,
    ) {
        let hinting_changed = self.font_collection.set_hinting(mode);
        let format_changed = self.font_collection.set_format(format);
        // Keep UI font registry in sync with the terminal font's rendering settings.
        if let Some(sizes) = &mut self.ui_font_sizes {
            sizes.set_hinting(mode);
            sizes.set_format(format);
        }
        if hinting_changed || format_changed {
            self.clear_and_recache(gpu);
        }
    }

    /// Clear all atlases and empty-key set, then re-cache ASCII.
    ///
    /// `clear()` resets the packer and cache but the underlying texture
    /// persists at its current size. Bind group rebuild is handled lazily
    /// by `rebuild_stale_atlas_bind_groups()` at the next render if the
    /// atlas grows during re-caching.
    fn clear_and_recache(&mut self, gpu: &GpuState) {
        self.atlas.clear();
        self.subpixel_atlas.clear();
        self.color_atlas.clear();
        self.empty_keys.clear();
        self.icon_cache.clear();

        let format = self.font_collection.format();
        if format.is_subpixel() {
            pre_cache_atlas(
                &mut self.subpixel_atlas,
                &mut self.font_collection,
                &gpu.device,
                &gpu.queue,
            );
        } else {
            pre_cache_atlas(
                &mut self.atlas,
                &mut self.font_collection,
                &gpu.device,
                &gpu.queue,
            );
        }
    }
}
