//! Scene conversion: append Scene primitives to the prepared frame.

use oriterm_ui::draw::Scene;

use super::super::prepared_frame::OverlayDrawRange;
use super::super::state::GpuState;
use super::{CombinedAtlasLookup, WindowRenderer};

use super::helpers::{ensure_glyphs_cached, scene_raster_keys};

impl WindowRenderer {
    /// Append Scene primitives **with text** to the chrome tier (draws 6-9).
    ///
    /// Rasterizes uncached UI text glyphs, converts typed Scene arrays into
    /// GPU instance buffer records.
    pub fn append_ui_scene_with_text(
        &mut self,
        scene: &Scene,
        scale: f32,
        opacity: f32,
        gpu: &GpuState,
    ) {
        self.cache_scene_glyphs(scene, scale, gpu);
        let hinted = self.ui_hinted();

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };

        let mut text_ctx = super::super::scene_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.ui_glyphs,
            subpixel_writer: &mut self.prepared.ui_subpixel_glyphs,
            color_writer: &mut self.prepared.ui_color_glyphs,
            hinted,
        };
        super::super::scene_convert::convert_scene(
            scene,
            &mut self.prepared.ui_rects,
            Some(&mut text_ctx),
            scale,
            opacity,
        );
    }

    /// Append overlay Scene primitives **with text** into the overlay tier.
    ///
    /// Each call records an [`OverlayDrawRange`] so the render pass can draw
    /// each overlay as a complete unit (rects then glyphs) before moving to
    /// the next. This ensures correct z-ordering between stacked overlays.
    pub fn append_overlay_scene_with_text(
        &mut self,
        scene: &Scene,
        scale: f32,
        opacity: f32,
        gpu: &GpuState,
    ) {
        self.cache_scene_glyphs(scene, scale, gpu);
        let hinted = self.ui_hinted();

        // Snapshot buffer positions before conversion.
        let rect_start = self.prepared.overlay_rects.len() as u32;
        let mono_start = self.prepared.overlay_glyphs.len() as u32;
        let sub_start = self.prepared.overlay_subpixel_glyphs.len() as u32;
        let color_start = self.prepared.overlay_color_glyphs.len() as u32;

        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };

        let mut text_ctx = super::super::scene_convert::TextContext {
            atlas: &bridge,
            mono_writer: &mut self.prepared.overlay_glyphs,
            subpixel_writer: &mut self.prepared.overlay_subpixel_glyphs,
            color_writer: &mut self.prepared.overlay_color_glyphs,
            hinted,
        };
        super::super::scene_convert::convert_scene(
            scene,
            &mut self.prepared.overlay_rects,
            Some(&mut text_ctx),
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
        };
        self.prepared.overlay_draw_ranges.push(range);
    }

    /// Cache UI text glyphs referenced by the Scene's text runs and icons.
    ///
    /// Each text run carries its own `size_q6`; keys are grouped by size so
    /// each group rasterizes against the matching [`FontCollection`] from the
    /// [`UiFontSizes`] registry.
    fn cache_scene_glyphs(&mut self, scene: &Scene, scale: f32, gpu: &GpuState) {
        let hinted = self.ui_hinted();

        self.ui_raster_keys.clear();
        scene_raster_keys(scene, hinted, scale, &mut self.ui_raster_keys);

        if self.ui_raster_keys.is_empty() {
            return;
        }

        // Sort keys by size_q6 for grouped processing per font size.
        self.ui_raster_keys.sort_unstable_by_key(|k| k.size_q6);

        // Process each size group with its matching FontCollection.
        let mut start = 0;
        while start < self.ui_raster_keys.len() {
            let q6 = self.ui_raster_keys[start].size_q6;
            let end = self.ui_raster_keys[start..]
                .iter()
                .position(|k| k.size_q6 != q6)
                .map_or(self.ui_raster_keys.len(), |p| start + p);

            // Check the registry with a shared borrow first, then re-borrow
            // mutably only the matching field. This avoids holding `&mut sizes`
            // and `&mut self.font_collection` in the same borrow scope.
            let in_registry = self
                .ui_font_sizes
                .as_ref()
                .is_some_and(|s| s.select_by_q6(q6).is_some());

            let ui_fc = if in_registry {
                self.ui_font_sizes
                    .as_mut()
                    .unwrap()
                    .select_by_q6_mut(q6)
                    .unwrap()
            } else {
                // Shaping fell back to the default collection (or no UI
                // registry exists). Rasterize against the same fallback.
                if self.ui_font_sizes.is_some() {
                    log::warn!(
                        "UI text size_q6={q6} not in font registry; \
                         falling back to terminal font for rasterization"
                    );
                }
                &mut self.font_collection
            };
            ensure_glyphs_cached(
                self.ui_raster_keys[start..end].iter().copied(),
                &mut self.atlas,
                &mut self.subpixel_atlas,
                &mut self.color_atlas,
                &mut self.empty_keys,
                ui_fc,
                &gpu.device,
                &gpu.queue,
            );
            start = end;
        }
    }

    /// Whether UI font hinting is enabled.
    fn ui_hinted(&self) -> bool {
        if let Some(sizes) = &self.ui_font_sizes {
            return sizes.hinting_mode().hint_flag();
        }
        self.font_collection.hinting_mode().hint_flag()
    }
}
