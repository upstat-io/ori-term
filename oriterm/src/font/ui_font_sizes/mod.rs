//! Per-size UI font collection registry.
//!
//! [`UiFontSizes`] maps logical pixel sizes to exact-size [`FontCollection`]
//! instances. Widgets declare desired text sizes via `TextStyle`; the
//! measurer and renderer select the collection whose physical rasterization
//! size matches exactly — no nearest-pool approximation.

use std::collections::BTreeMap;

use super::collection::{FontCollection, FontSet, size_key};
use super::{FontError, GlyphFormat, HintingMode};

/// Default logical pixel size for body text (CSS `font-size: 13px`).
const DEFAULT_LOGICAL_SIZE: f32 = 13.0;

/// Standard preloaded logical pixel sizes used by the settings UI.
///
/// Created eagerly at registry construction to avoid first-frame hitches.
/// Any other size is created lazily on first request.
pub(crate) const PRELOAD_SIZES: &[f32] = &[9.0, 10.0, 11.0, 11.5, 12.0, 13.0, 16.0, 18.0];

/// Exact-size UI font collection registry.
///
/// Stores one [`FontCollection`] per requested logical size, keyed by the
/// physical `size_q6` (26.6 fixed-point of physical pixel size). Collections
/// are created eagerly for common sizes and lazily for uncommon sizes.
pub(crate) struct UiFontSizes {
    font_set: FontSet,
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
    weight: u16,
    /// Keyed by physical `size_q6`.
    collections: BTreeMap<u32, FontCollection>,
    /// All logical pixel sizes that have been inserted.
    ///
    /// Tracked so [`set_dpi`] can rebuild every collection at the correct
    /// logical size after the physical DPI changes.
    logical_sizes: Vec<f32>,
    /// The `size_q6` key for the default body text collection (13px logical).
    default_q6: u32,
}

impl UiFontSizes {
    /// Create a registry, preloading collections for the given logical sizes.
    ///
    /// `dpi` is the physical DPI (encodes scale factor: e.g. 192 at 2×).
    /// Each logical size is converted to `size_pt = logical × 72 / 96` and
    /// passed to [`FontCollection::new`] with the given `dpi`.
    #[expect(
        clippy::too_many_arguments,
        reason = "font registry requires all parameters: font data, DPI, format, hinting, weight, sizes"
    )]
    pub(crate) fn new(
        font_set: FontSet,
        dpi: f32,
        format: GlyphFormat,
        hinting: HintingMode,
        weight: u16,
        preload_logical_sizes: &[f32],
    ) -> Result<Self, FontError> {
        let mut collections = BTreeMap::new();
        let mut logical_sizes = Vec::with_capacity(preload_logical_sizes.len());
        let mut default_q6 = 0;

        for &logical_px in preload_logical_sizes {
            let size_pt = logical_to_pt(logical_px);
            let fc = FontCollection::new(font_set.clone(), size_pt, dpi, format, weight, hinting)?;
            let q6 = size_key(fc.size_px());
            if (logical_px - DEFAULT_LOGICAL_SIZE).abs() < 0.01 {
                default_q6 = q6;
            }
            collections.insert(q6, fc);
            logical_sizes.push(logical_px);
        }

        // If the default wasn't in the preload list, create it now.
        if default_q6 == 0 {
            let size_pt = logical_to_pt(DEFAULT_LOGICAL_SIZE);
            let fc = FontCollection::new(font_set.clone(), size_pt, dpi, format, weight, hinting)?;
            default_q6 = size_key(fc.size_px());
            collections.insert(default_q6, fc);
            logical_sizes.push(DEFAULT_LOGICAL_SIZE);
        }

        Ok(Self {
            font_set,
            dpi,
            format,
            hinting,
            weight,
            collections,
            logical_sizes,
            default_q6,
        })
    }

    // ── Accessors ──

    /// The default body text collection (13px logical).
    pub(crate) fn default_collection(&self) -> Option<&FontCollection> {
        self.collections.get(&self.default_q6)
    }

    /// Mutable access to the default body text collection.
    pub(crate) fn default_collection_mut(&mut self) -> Option<&mut FontCollection> {
        self.collections.get_mut(&self.default_q6)
    }

    /// The `size_q6` key for the default body text collection.
    #[allow(dead_code, reason = "used in later subsections for atlas grouping")]
    pub(crate) fn default_q6(&self) -> u32 {
        self.default_q6
    }

    /// Current hinting mode.
    pub(crate) fn hinting_mode(&self) -> HintingMode {
        self.hinting
    }

    /// Current rasterization format.
    #[allow(
        dead_code,
        reason = "symmetry with hinting_mode; used in later subsections"
    )]
    pub(crate) fn format(&self) -> GlyphFormat {
        self.format
    }

    /// Number of collections in the registry.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.collections.len()
    }

    // ── Size selection ──

    /// Select a collection by logical pixel size and scale factor.
    ///
    /// Returns `None` if no collection exists for this exact physical size.
    /// Use [`select_mut`](Self::select_mut) to lazily create missing sizes.
    #[allow(dead_code, reason = "used in 01.2 UiFontMeasurer size-aware shaping")]
    pub(crate) fn select(&self, logical_size: f32, scale: f32) -> Option<&FontCollection> {
        let physical_px = logical_size * scale;
        let q6 = size_key(physical_px);
        self.collections.get(&q6)
    }

    /// Select or lazily create a collection for the given logical size.
    ///
    /// If no collection exists at this physical size, creates one and
    /// retains it for future use.
    #[allow(dead_code, reason = "used in 01.2 UiFontMeasurer size-aware shaping")]
    pub(crate) fn select_mut(
        &mut self,
        logical_size: f32,
        scale: f32,
    ) -> Result<&mut FontCollection, FontError> {
        let physical_px = logical_size * scale;
        let q6 = size_key(physical_px);
        if !self.collections.contains_key(&q6) {
            let size_pt = logical_to_pt(logical_size);
            let fc = FontCollection::new(
                self.font_set.clone(),
                size_pt,
                self.dpi,
                self.format,
                self.weight,
                self.hinting,
            )?;
            self.collections.insert(q6, fc);
            self.logical_sizes.push(logical_size);
        }
        Ok(self
            .collections
            .get_mut(&q6)
            .expect("just inserted or exists"))
    }

    /// Look up a collection by its physical `size_q6` key.
    #[allow(dead_code, reason = "used in 01.3 scene conversion size threading")]
    pub(crate) fn select_by_q6(&self, size_q6: u32) -> Option<&FontCollection> {
        self.collections.get(&size_q6)
    }

    /// Mutable lookup by physical `size_q6` key.
    #[allow(dead_code, reason = "used in 01.3 scene conversion size threading")]
    pub(crate) fn select_by_q6_mut(&mut self, size_q6: u32) -> Option<&mut FontCollection> {
        self.collections.get_mut(&size_q6)
    }

    // ── Configuration changes ──

    /// Rebuild all collections at a new DPI (e.g. after scale factor change).
    ///
    /// Recreates every collection at its original logical size but the new
    /// physical DPI. No-ops if the DPI is unchanged.
    pub(crate) fn set_dpi(&mut self, dpi: f32) -> Result<(), FontError> {
        if (self.dpi - dpi).abs() < 0.01 {
            return Ok(());
        }
        self.dpi = dpi;
        self.rebuild_all()
    }

    /// Update hinting mode for all collections.
    ///
    /// Propagates the change to every collection's glyph cache. The caller
    /// is responsible for clearing GPU atlases afterward.
    pub(crate) fn set_hinting(&mut self, hinting: HintingMode) {
        if self.hinting == hinting {
            return;
        }
        self.hinting = hinting;
        for fc in self.collections.values_mut() {
            fc.set_hinting(hinting);
        }
    }

    /// Update rasterization format for all collections.
    ///
    /// Propagates the change to every collection's glyph cache. The caller
    /// is responsible for clearing GPU atlases afterward.
    pub(crate) fn set_format(&mut self, format: GlyphFormat) {
        if self.format == format {
            return;
        }
        self.format = format;
        for fc in self.collections.values_mut() {
            fc.set_format(format);
        }
    }

    /// Create a standalone [`FontCollection`] at the default body text size.
    ///
    /// Used by [`WindowRenderer::new_ui_only`] which needs a `FontCollection`
    /// in the terminal font slot for atlas seeding.
    pub(crate) fn create_default_collection(&self) -> Result<FontCollection, FontError> {
        let size_pt = logical_to_pt(DEFAULT_LOGICAL_SIZE);
        FontCollection::new(
            self.font_set.clone(),
            size_pt,
            self.dpi,
            self.format,
            self.weight,
            self.hinting,
        )
    }

    // ── Internals ──

    /// Rebuild all collections from scratch after a DPI change.
    fn rebuild_all(&mut self) -> Result<(), FontError> {
        self.collections.clear();
        let logical_sizes = std::mem::take(&mut self.logical_sizes);
        let mut new_default_q6 = 0;

        for &logical_px in &logical_sizes {
            let size_pt = logical_to_pt(logical_px);
            let fc = FontCollection::new(
                self.font_set.clone(),
                size_pt,
                self.dpi,
                self.format,
                self.weight,
                self.hinting,
            )?;
            let q6 = size_key(fc.size_px());
            if (logical_px - DEFAULT_LOGICAL_SIZE).abs() < 0.01 {
                new_default_q6 = q6;
            }
            self.collections.insert(q6, fc);
        }

        self.logical_sizes = logical_sizes;
        self.default_q6 = new_default_q6;
        Ok(())
    }
}

/// Convert logical pixels to points: `pt = px × 72 / 96`.
fn logical_to_pt(logical_px: f32) -> f32 {
    logical_px * 72.0 / 96.0
}

#[cfg(test)]
mod tests;
