//! Font collection configuration setters.
//!
//! Methods that reconfigure font size, hinting, format, features, and codepoint
//! mappings. Separated from the main [`FontCollection`] module to stay under the
//! 500-line file limit.

use super::FontCollection;
use super::face::compute_metrics;
use super::metadata::{MAX_FONT_SIZE, MIN_FONT_SIZE, face_variations};
use crate::font::{CellMetrics, FaceIdx, FontError, GlyphFormat, HintingMode, SyntheticFlags};

impl FontCollection {
    // ── Configuration setters ──

    /// Replace collection-wide OpenType features.
    ///
    /// Overrides the default `["liga", "calt"]` features. Primary faces (0–3)
    /// use these features; fallback faces use their per-fallback override if
    /// configured, otherwise these collection features.
    pub fn set_features(&mut self, features: Vec<rustybuzz::Feature>) {
        self.features = features;
    }

    /// Update a fallback font's metadata (`size_offset` and features).
    ///
    /// `fallback_index` is the 0-based position in the fallback array (not
    /// the global `FaceIdx`). Out-of-range indices are ignored.
    pub fn set_fallback_meta(
        &mut self,
        fallback_index: usize,
        size_offset: f32,
        features: Option<Vec<rustybuzz::Feature>>,
    ) {
        if let Some(meta) = self.fallback_meta.get_mut(fallback_index) {
            meta.size_offset = size_offset;
            meta.features = features;
        }
    }

    // ── Codepoint map ──

    /// Add a codepoint-to-face override.
    ///
    /// Codepoints in `start..=end` will resolve to `face_idx` before
    /// consulting the normal primary + fallback chain. If the mapped face
    /// doesn't contain the codepoint, normal resolution is used.
    pub fn add_codepoint_mapping(&mut self, start: u32, end: u32, face_idx: FaceIdx) {
        self.codepoint_map.add(start, end, face_idx);
    }

    /// Whether the codepoint map has any entries.
    #[allow(dead_code, reason = "diagnostic predicate for logging and future UI")]
    pub fn has_codepoint_mappings(&self) -> bool {
        !self.codepoint_map.is_empty()
    }

    // ── Public operations ──

    /// Change font size, recomputing all derived metrics and caches.
    ///
    /// Recomputes cell metrics from the Regular face at the new size,
    /// recalculates cap-height normalization for fallback fonts, and clears
    /// the glyph cache. The caller (`WindowRenderer::set_font_size`) is
    /// responsible for re-populating the atlas afterward.
    pub fn set_size(&mut self, size_pt: f32, dpi: f32) -> Result<(), FontError> {
        let size_px = (size_pt * dpi / 72.0).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);

        // Recompute metrics from Regular face (with weight variations).
        let regular = self.primary[0]
            .as_ref()
            .ok_or_else(|| FontError::InvalidFont("Regular face required".into()))?;
        let regular_vars = face_variations(
            FaceIdx::REGULAR,
            SyntheticFlags::NONE,
            self.weight,
            &regular.axes,
        );
        let fm = compute_metrics(
            &regular.bytes,
            regular.face_index,
            size_px,
            &regular_vars.settings,
        )
        .ok_or_else(|| FontError::InvalidFont("Regular font metrics unavailable".into()))?;
        let primary_cap = fm.cap_height;

        // Recalculate cap-height normalization for fallbacks.
        for (fb, meta) in self.fallbacks.iter().zip(self.fallback_meta.iter_mut()) {
            let fb_m = compute_metrics(&fb.bytes, fb.face_index, size_px, &[]).unwrap_or(fm);
            meta.scale_factor = if fb_m.cap_height > 0.0 && primary_cap > 0.0 {
                primary_cap / fb_m.cap_height
            } else {
                1.0
            };
        }

        self.size_px = size_px;
        self.dpi = dpi;
        self.metrics = CellMetrics::new(
            fm.cell_width,
            fm.cell_height,
            fm.baseline,
            fm.underline_offset,
            fm.stroke_size,
            fm.strikeout_offset,
        );
        self.cap_height_px = primary_cap;
        self.cache_clear();
        Ok(())
    }

    /// Change hinting mode and clear the glyph cache.
    ///
    /// No-ops if the mode is unchanged. The caller (`WindowRenderer::set_hinting_mode`)
    /// is responsible for clearing GPU atlases and re-populating afterward.
    ///
    /// Returns `true` if the mode actually changed.
    pub fn set_hinting(&mut self, mode: HintingMode) -> bool {
        if self.hinting == mode {
            return false;
        }
        self.hinting = mode;
        self.cache_clear();
        true
    }

    /// Change rasterization format and clear the glyph cache.
    ///
    /// No-ops if the format is unchanged. The caller
    /// (`WindowRenderer::set_glyph_format`) is responsible for clearing GPU
    /// atlases and re-populating afterward.
    ///
    /// Returns `true` if the format actually changed.
    pub fn set_format(&mut self, format: GlyphFormat) -> bool {
        if self.format == format {
            return false;
        }
        self.format = format;
        self.cache_clear();
        true
    }
}
