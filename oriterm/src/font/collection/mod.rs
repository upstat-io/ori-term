//! Font collection: face data, rasterization, metrics, glyph cache.
//!
//! [`FontSet`] loads raw font bytes from discovery results. [`FontCollection`]
//! validates faces, computes cell metrics, resolves characters to glyph IDs,
//! and rasterizes glyphs into bitmaps ready for GPU atlas upload.

mod codepoint_map;
pub(crate) mod colr_v1;
mod face;
mod loading;
mod metadata;
mod rasterize;
mod resolve;
mod shaping;

use std::collections::HashMap;
use std::sync::Arc;

use swash::scale::ScaleContext;

use super::{
    CellMetrics, FaceIdx, FontError, GlyphFormat, GlyphStyle, HintingMode, RasterKey,
    ResolvedGlyph, SyntheticFlags,
};
use codepoint_map::CodepointMap;
pub(crate) use codepoint_map::parse_hex_range;
pub(crate) use face::size_key;
pub(crate) use face::{FaceData, font_ref};
use face::{build_face, compute_metrics};
pub(crate) use loading::{FontByteCache, FontSet};
pub(crate) use metadata::parse_features;
use metadata::{
    FallbackMeta, MAX_FONT_SIZE, MIN_FONT_SIZE, default_features, effective_size_for,
    face_variations, resolve_ui_weight,
};

/// A rasterized glyph bitmap ready for atlas upload.
#[derive(Debug, Clone)]
pub(crate) struct RasterizedGlyph {
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Horizontal bearing (pixels from origin to left edge of bitmap).
    pub bearing_x: i32,
    /// Vertical bearing (pixels from baseline to top edge; positive = above).
    pub bearing_y: i32,
    /// Horizontal advance width in pixels.
    #[allow(
        dead_code,
        reason = "read in tests; production use pending shaped-advance integration"
    )]
    pub advance: f32,
    /// Pixel format of the bitmap data.
    pub format: GlyphFormat,
    /// Raw bitmap bytes. Layout depends on format:
    /// - Alpha: `width * height` bytes (1 byte/pixel).
    /// - SubpixelRgb/Bgr: `width * height * 4` bytes (RGBA per-channel).
    /// - Color: `width * height * 4` bytes (RGBA premultiplied).
    pub bitmap: Vec<u8>,
}

/// Font collection: validated faces, cell metrics, glyph cache, rasterization.
///
/// Owns all font face data and provides the bridge between font discovery
/// and the GPU renderer. Resolves characters to glyph IDs, rasterizes
/// bitmaps, and pre-caches ASCII glyphs.
pub(crate) struct FontCollection {
    // Faces
    primary: [Option<FaceData>; 4],
    /// Medium (500) face for UI text weight fidelity. Substituted into the
    /// Regular shaping/raster slot when `requested_weight` is in 500..700.
    medium: Option<FaceData>,
    fallbacks: Vec<FaceData>,
    fallback_meta: Vec<FallbackMeta>,
    // Metrics
    size_px: f32,
    metrics: CellMetrics,
    #[allow(
        dead_code,
        reason = "used for diagnostics and future dynamic fallback loading"
    )]
    cap_height_px: f32,
    // Rasterization
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
    glyph_cache: HashMap<RasterKey, RasterizedGlyph>,
    scale_context: ScaleContext,
    // Config
    weight: u16,
    family_name: String,
    /// Collection-wide OpenType features applied to all primary faces.
    ///
    /// Default: `["liga", "calt"]` (standard ligatures + contextual alternates).
    features: Vec<rustybuzz::Feature>,
    /// Codepoint-to-face overrides. Checked before the normal fallback chain.
    codepoint_map: CodepointMap,
}

impl FontCollection {
    /// Build a font collection from a loaded font set.
    ///
    /// Validates all faces and computes cell metrics from the Regular font.
    /// ASCII glyphs are not pre-cached here — the GPU renderer's
    /// `pre_cache_atlas()` fills both the `HashMap` and the atlas in one pass.
    #[expect(
        clippy::too_many_arguments,
        reason = "font collection requires all parameters: font data, sizing, format, weight, hinting"
    )]
    pub fn new(
        font_set: FontSet,
        size_pt: f32,
        dpi: f32,
        format: GlyphFormat,
        weight: u16,
        hinting: HintingMode,
    ) -> Result<Self, FontError> {
        let size_px = (size_pt * dpi / 72.0).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);

        // Validate Regular (required).
        let regular_face =
            build_face(Arc::clone(&font_set.regular.data), font_set.regular.index)
                .ok_or_else(|| FontError::InvalidFont("Regular font is invalid".into()))?;

        // Compute metrics from Regular (with weight variations for variable fonts).
        let regular_vars = face_variations(
            FaceIdx::REGULAR,
            SyntheticFlags::NONE,
            weight,
            &regular_face.axes,
        );
        let font_metrics = compute_metrics(
            &font_set.regular.data,
            font_set.regular.index,
            size_px,
            &regular_vars.settings,
        )
        .ok_or_else(|| FontError::InvalidFont("Regular font metrics unavailable".into()))?;
        let primary_cap = font_metrics.cap_height;

        // Validate optional primary variants.
        let bold = font_set
            .bold
            .as_ref()
            .and_then(|fd| build_face(Arc::clone(&fd.data), fd.index));
        let italic = font_set
            .italic
            .as_ref()
            .and_then(|fd| build_face(Arc::clone(&fd.data), fd.index));
        let bold_italic = font_set
            .bold_italic
            .as_ref()
            .and_then(|fd| build_face(Arc::clone(&fd.data), fd.index));
        let medium = font_set
            .medium
            .as_ref()
            .and_then(|fd| build_face(Arc::clone(&fd.data), fd.index));

        // Validate fallbacks and compute cap-height normalization.
        let mut fallbacks = Vec::new();
        let mut fallback_meta = Vec::new();
        for fd in &font_set.fallbacks {
            if let Some(face) = build_face(Arc::clone(&fd.data), fd.index) {
                let fb_metrics =
                    compute_metrics(&fd.data, fd.index, size_px, &[]).unwrap_or(font_metrics);
                let scale_factor = if fb_metrics.cap_height > 0.0 && primary_cap > 0.0 {
                    primary_cap / fb_metrics.cap_height
                } else {
                    1.0
                };
                fallbacks.push(face);
                fallback_meta.push(FallbackMeta {
                    scale_factor,
                    size_offset: 0.0,
                    features: None,
                });
            }
        }

        let metrics = CellMetrics::new(
            font_metrics.cell_width,
            font_metrics.cell_height,
            font_metrics.baseline,
            font_metrics.underline_offset,
            font_metrics.stroke_size,
            font_metrics.strikeout_offset,
        );

        let collection = Self {
            primary: [Some(regular_face), bold, italic, bold_italic],
            medium,
            fallbacks,
            fallback_meta,
            size_px,
            metrics,
            cap_height_px: primary_cap,
            dpi,
            format,
            hinting,
            glyph_cache: HashMap::new(),
            scale_context: ScaleContext::new(),
            weight,
            family_name: font_set.family_name,
            features: default_features(),
            codepoint_map: CodepointMap::new(),
        };

        Ok(collection)
    }

    // ── Accessors ──

    /// Cell metrics for the GPU renderer.
    pub fn cell_metrics(&self) -> CellMetrics {
        self.metrics
    }

    /// Font size in pixels.
    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    /// Family name of the primary font.
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    /// Rasterization format.
    pub fn format(&self) -> GlyphFormat {
        self.format
    }

    /// Number of cached glyphs.
    #[allow(dead_code, reason = "font fields consumed in later sections")]
    pub fn cache_len(&self) -> usize {
        self.glyph_cache.len()
    }

    /// Effective pixel size for a face, accounting for cap-height normalization.
    ///
    /// Primary faces return `size_px`. Fallback faces are scaled by their
    /// cap-height ratio plus any user-configured `size_offset`.
    pub fn effective_size(&self, face_idx: FaceIdx) -> f32 {
        effective_size_for(face_idx, self.size_px, &self.fallback_meta)
    }

    /// OpenType features for a given face.
    ///
    /// Primary faces (0–3) use collection-wide defaults. Fallback faces use
    /// their per-fallback override if configured, otherwise collection defaults.
    pub fn features_for_face(&self, face_idx: FaceIdx) -> &[rustybuzz::Feature] {
        if let Some(fb_i) = face_idx.fallback_index() {
            if let Some(meta) = self.fallback_meta.get(fb_i) {
                if let Some(ref fb_features) = meta.features {
                    return fb_features;
                }
            }
        }
        &self.features
    }

    /// Current hinting mode.
    pub fn hinting_mode(&self) -> HintingMode {
        self.hinting
    }

    /// Whether the collection has a real Bold face (not synthetic).
    pub fn has_bold(&self) -> bool {
        self.primary[GlyphStyle::Bold as usize].is_some()
    }

    /// Whether the Regular primary face has a `wght` variation axis.
    pub fn has_wght_axis(&self) -> bool {
        self.primary[0]
            .as_ref()
            .is_some_and(|fd| face::has_axis(&fd.axes, *b"wght"))
    }

    /// Whether a specific face has a `wght` variation axis.
    ///
    /// Used by the UI shaping pipeline to decide whether a fallback face
    /// needs synthetic bold (it does if the face lacks a `wght` axis and
    /// the requested weight is >= 700).
    pub fn face_has_wght_axis(&self, face_idx: FaceIdx) -> bool {
        self.face_data(face_idx)
            .is_some_and(|fd| face::has_axis(&fd.axes, *b"wght"))
    }

    /// Configured font weight (CSS 100–900, typically 400).
    pub fn weight(&self) -> u16 {
        self.weight
    }

    /// Resolve a UI text weight request against this collection's font capabilities.
    ///
    /// Returns face slot, `wght` axis value, and synthetic bold flag based on
    /// the weight realization policy. UI-text only — terminal grid uses
    /// face-slot-based bold selection.
    #[allow(dead_code, reason = "consumed by UI weight pipeline in 02.3")]
    pub fn resolve_ui_weight_info(&self, requested_weight: u16) -> metadata::UiWeightResolution {
        resolve_ui_weight(requested_weight, self.has_wght_axis(), self.has_bold())
    }

    /// Look up a glyph ID and advance width directly from the cmap table.
    ///
    /// Bypasses rustybuzz shaping, returning the raw cmap glyph ID and advance
    /// from swash metrics. Used when rustybuzz cannot create a `Face` for a
    /// font (e.g. some color emoji fonts) — the cmap ID is sufficient for
    /// simple characters and single-codepoint emoji.
    ///
    /// Returns `None` if the face doesn't exist or the character isn't covered.
    pub fn cmap_glyph(&self, ch: char, face_idx: FaceIdx) -> Option<(u16, f32)> {
        let fd = self.face_data(face_idx)?;
        let gid = face::glyph_id(fd, ch);
        if gid == 0 {
            return None;
        }
        let size = effective_size_for(face_idx, self.size_px, &self.fallback_meta);
        let vars = face_variations(face_idx, SyntheticFlags::NONE, self.weight, &fd.axes);
        let fr = font_ref(fd);
        let coords = face::normalize_coords(&fr, &vars.settings);
        let advance = fr.glyph_metrics(&coords).scale(size).advance_width(gid);
        Some((gid, advance))
    }

    /// Access face data by index (0–3 = primary, 4+ = fallback).
    fn face_data(&self, face_idx: FaceIdx) -> Option<&FaceData> {
        if let Some(fb_i) = face_idx.fallback_index() {
            self.fallbacks.get(fb_i)
        } else {
            self.primary.get(face_idx.as_usize())?.as_ref()
        }
    }

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
        self.glyph_cache.clear();
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
        self.glyph_cache.clear();
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
        self.glyph_cache.clear();
        true
    }
}

#[cfg(test)]
mod tests;
