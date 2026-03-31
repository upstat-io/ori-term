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
            subpixel_positioning: true, // UI text always uses subpixel positioning.
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
            subpixel_positioning: true, // UI text always uses subpixel positioning.
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
        scene_raster_keys(
            scene,
            hinted,
            scale,
            &mut self.ui_raster_keys,
            true, // UI text always uses subpixel positioning.
        );

        if self.ui_raster_keys.is_empty() {
            return;
        }

        // Partition: terminal-font glyphs always use self.font_collection;
        // UI-font glyphs use the size registry (or terminal font as fallback).
        let terminal_end = {
            let mut lo = 0;
            let mut hi = self.ui_raster_keys.len();
            while lo < hi {
                if self.ui_raster_keys[lo].font_realm == crate::font::FontRealm::Terminal {
                    lo += 1;
                } else {
                    hi -= 1;
                    self.ui_raster_keys.swap(lo, hi);
                }
            }
            lo
        };

        // Terminal-font glyphs: rasterize using the terminal collection directly.
        if terminal_end > 0 {
            ensure_glyphs_cached(
                self.ui_raster_keys[..terminal_end].iter().copied(),
                &mut self.atlas,
                &mut self.subpixel_atlas,
                &mut self.color_atlas,
                &mut self.empty_keys,
                &mut self.font_collection,
                &gpu.device,
                &gpu.queue,
            );
        }

        // UI-font glyphs: group by size_q6 and resolve via UI font registry.
        let ui_keys = &mut self.ui_raster_keys[terminal_end..];
        ui_keys.sort_unstable_by_key(|k| k.size_q6);

        let mut start = 0;
        while start < ui_keys.len() {
            let q6 = ui_keys[start].size_q6;
            let end = ui_keys[start..]
                .iter()
                .position(|k| k.size_q6 != q6)
                .map_or(ui_keys.len(), |p| start + p);

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
                &mut self.font_collection
            };
            ensure_glyphs_cached(
                ui_keys[start..end].iter().copied(),
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
