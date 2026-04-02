//! Font configuration changes: size, hinting, glyph format.
//!
//! Each method delegates to [`FontCollection`] for metrics/cache invalidation,
//! then clears GPU atlases and re-caches ASCII glyphs.

use crate::font::{FontCollection, FontRealm, GlyphFormat, HintingMode, UiFontSizes};
use crate::gpu::state::GpuState;

use super::WindowRenderer;
use super::helpers::{pre_cache_atlas, prewarm_ui_font_sizes};

impl WindowRenderer {
    // ── Font configuration ──

    /// Replace the entire font collection (family, weight, features changed).
    ///
    /// Clears all GPU atlases and re-caches ASCII glyphs with the new fonts.
    pub fn replace_font_collection(&mut self, collection: FontCollection, gpu: &GpuState) {
        self.font_collection = collection;
        self.clear_and_recache(gpu);
    }

    /// Replace the UI font sizes registry (font family/weight/features changed).
    ///
    /// Stores the new registry without clearing atlases — call
    /// [`replace_font_collection`] afterward to clear and re-prewarm both
    /// terminal and UI atlases in one pass.
    pub fn replace_ui_font_sizes(&mut self, sizes: UiFontSizes) {
        self.ui_font_sizes = Some(sizes);
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

    /// Change both hinting mode and glyph format for the **terminal** font,
    /// clearing atlases once.
    ///
    /// Used during scale factor changes where both settings typically change
    /// together. Avoids the double clear-and-recache that would happen from
    /// calling [`set_hinting_mode`] and [`set_glyph_format`] separately.
    ///
    /// UI fonts are intentionally unaffected — they always use
    /// `GlyphFormat::Alpha` / `HintingMode::None` (set at construction and
    /// in `rebuild_ui_font_sizes`).
    pub fn set_hinting_and_format(
        &mut self,
        mode: HintingMode,
        format: GlyphFormat,
        gpu: &GpuState,
    ) {
        let hinting_changed = self.font_collection.set_hinting(mode);
        let format_changed = self.font_collection.set_format(format);
        if hinting_changed || format_changed {
            self.clear_and_recache(gpu);
        }
    }

    /// Set whether subpixel glyph positioning is enabled.
    ///
    /// When disabled, all glyphs snap to integer pixel X boundaries (no
    /// fractional subpixel phase). Takes effect at the next prepare pass.
    pub fn set_subpixel_positioning(&mut self, enabled: bool) {
        self.subpixel_positioning = enabled;
    }

    /// Returns whether subpixel positioning is enabled.
    pub fn subpixel_positioning(&self) -> bool {
        self.subpixel_positioning
    }

    /// Change the atlas texture filtering mode, recreating all bind groups.
    ///
    /// Snapshots atlas generations so `rebuild_stale_atlas_bind_groups()` does
    /// not immediately re-rebuild the bind groups we just created.
    pub fn set_atlas_filtering(
        &mut self,
        filtering: crate::gpu::bind_groups::AtlasFiltering,
        gpu: &GpuState,
        layout: &wgpu::BindGroupLayout,
    ) {
        let filter = filtering.to_filter_mode();
        self.atlas_bind_group = crate::gpu::bind_groups::AtlasBindGroup::new(
            &gpu.device,
            layout,
            self.atlas.view(),
            filter,
        );
        self.subpixel_atlas_bind_group = crate::gpu::bind_groups::AtlasBindGroup::new(
            &gpu.device,
            layout,
            self.subpixel_atlas.view(),
            filter,
        );
        self.color_atlas_bind_group = crate::gpu::bind_groups::AtlasBindGroup::new(
            &gpu.device,
            layout,
            self.color_atlas.view(),
            filter,
        );
        // Snapshot so rebuild_stale doesn't immediately re-rebuild.
        self.atlas_generation = self.atlas.generation();
        self.subpixel_atlas_generation = self.subpixel_atlas.generation();
        self.color_atlas_generation = self.color_atlas.generation();
        self.atlas_filtering = filtering;
    }

    /// Clear all atlases and empty-key set, then re-cache ASCII.
    ///
    /// `clear()` resets the packer and cache but the underlying texture
    /// persists at its current size. Bind group rebuild is handled lazily
    /// by `rebuild_stale_atlas_bind_groups()` at the next render if the
    /// atlas grows during re-caching.
    ///
    /// Prewarms both terminal and UI font atlases so neither has first-frame
    /// atlas misses after a font configuration change.
    fn clear_and_recache(&mut self, gpu: &GpuState) {
        self.atlas.clear();
        self.subpixel_atlas.clear();
        self.color_atlas.clear();
        self.empty_keys.clear();
        self.icon_cache.clear();
        // Invalidate the shaped frame cache so the next prepare() re-shapes
        // all text with the new font. Without this, stale glyph IDs from the
        // old font are served until something forces content_changed=true.
        self.shaping.frame = super::super::prepare::ShapedFrame::new(0, 0);

        let format = self.font_collection.format();
        if format.is_subpixel() {
            pre_cache_atlas(
                &mut self.subpixel_atlas,
                &mut self.font_collection,
                FontRealm::Terminal,
                &gpu.device,
                &gpu.queue,
            );
        } else {
            pre_cache_atlas(
                &mut self.atlas,
                &mut self.font_collection,
                FontRealm::Terminal,
                &gpu.device,
                &gpu.queue,
            );
        }

        // Re-prewarm UI font sizes — the atlas was just cleared.
        if let Some(ref mut sizes) = self.ui_font_sizes {
            prewarm_ui_font_sizes(
                sizes,
                &mut self.atlas,
                &mut self.subpixel_atlas,
                &gpu.device,
                &gpu.queue,
            );
        }
    }
}
