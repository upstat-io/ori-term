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

/// Hook called on each [`FontCollection`] after construction or rebuild.
type PostRebuildHook = Box<dyn Fn(&mut FontCollection)>;

/// Standard preloaded logical pixel sizes used by the settings UI.
///
/// Created eagerly at registry construction to avoid first-frame hitches.
/// All widget-used sizes must be listed here — the registry does not create
/// collections lazily at render time (the measurer has `&self` access only).
/// Use [`UiFontSizes::ensure_size`] in `&mut` contexts to register additional
/// sizes before creating an immutable measurer.
pub(crate) const PRELOAD_SIZES: &[f32] = &[9.0, 9.5, 10.0, 11.0, 11.5, 12.0, 13.0, 16.0, 18.0];

/// Exact-size UI font collection registry.
///
/// Stores one [`FontCollection`] per requested logical size, keyed by the
/// physical `size_q6` (26.6 fixed-point of physical pixel size). Collections
/// are created eagerly for all sizes in [`PRELOAD_SIZES`] at construction.
pub(crate) struct UiFontSizes {
    font_set: FontSet,
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
    weight: u16,
    bold_weight: u16,
    /// Keyed by physical `size_q6`.
    collections: BTreeMap<u32, FontCollection>,
    /// All logical pixel sizes that have been inserted.
    ///
    /// Tracked so [`set_dpi`] can rebuild every collection at the correct
    /// logical size after the physical DPI changes.
    logical_sizes: Vec<f32>,
    /// The `size_q6` key for the default body text collection (13px logical).
    default_q6: u32,
    /// Applied to each collection after rebuild (DPI change, `ensure_size`).
    ///
    /// Captures font config state (features, fallback metadata, codepoint
    /// mappings) so that [`rebuild_all`] preserves user font configuration
    /// across monitor DPI changes.
    post_rebuild_hook: Option<PostRebuildHook>,
}

impl UiFontSizes {
    /// Create a registry, preloading collections for the given logical sizes.
    ///
    /// `dpi` is the physical DPI (encodes scale factor: e.g. 192 at 2×).
    /// Each logical size is converted to `size_pt = logical × 72 / 96` and
    /// passed to [`FontCollection::new`] with the given `dpi`.
    #[expect(
        clippy::too_many_arguments,
        reason = "font registry requires all parameters: font data, DPI, format, hinting, weight, bold_weight, sizes"
    )]
    pub(crate) fn new(
        font_set: FontSet,
        dpi: f32,
        format: GlyphFormat,
        hinting: HintingMode,
        weight: u16,
        bold_weight: u16,
        preload_logical_sizes: &[f32],
    ) -> Result<Self, FontError> {
        let mut collections = BTreeMap::new();
        let mut logical_sizes = Vec::with_capacity(preload_logical_sizes.len());
        let mut default_q6 = 0;

        for &logical_px in preload_logical_sizes {
            let size_pt = logical_to_pt(logical_px);
            let fc = FontCollection::new(
                font_set.clone(),
                size_pt,
                dpi,
                format,
                weight,
                bold_weight,
                hinting,
            )?;
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
            let fc = FontCollection::new(
                font_set.clone(),
                size_pt,
                dpi,
                format,
                weight,
                bold_weight,
                hinting,
            )?;
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
            bold_weight,
            collections,
            logical_sizes,
            default_q6,
            post_rebuild_hook: None,
        })
    }

    /// Inject fallback fonts into all collections in the registry.
    ///
    /// Used to add the terminal font's emoji fallback so emoji render at
    /// the correct UI text size through `FontSource::Ui`.
    pub(crate) fn inject_fallbacks(&mut self, data: &[super::collection::loading::FontData]) {
        for fc in self.collections.values_mut() {
            fc.append_fallback_data(data);
        }
    }

    // Accessors

    /// The default body text collection (13px logical).
    pub(crate) fn default_collection(&self) -> Option<&FontCollection> {
        self.collections.get(&self.default_q6)
    }

    /// Mutable access to the default body text collection.
    pub(crate) fn default_collection_mut(&mut self) -> Option<&mut FontCollection> {
        self.collections.get_mut(&self.default_q6)
    }

    /// Iterate over all collections in the registry.
    pub(crate) fn collections_mut(&mut self) -> impl Iterator<Item = &mut FontCollection> {
        self.collections.values_mut()
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

    /// Set a hook applied to each collection after rebuild.
    ///
    /// Used to reapply font config (features, fallback metadata, codepoint
    /// mappings) after DPI-triggered rebuilds or `ensure_size` additions.
    pub(crate) fn set_post_rebuild_hook(&mut self, hook: PostRebuildHook) {
        self.post_rebuild_hook = Some(hook);
    }

    /// Number of collections in the registry.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.collections.len()
    }

    // Size selection

    /// Select a collection by logical pixel size and scale factor.
    ///
    /// Returns `None` if no collection exists for this exact physical size.
    /// All widget-used sizes must be in [`PRELOAD_SIZES`] — a missing size
    /// is a configuration error (the caller falls back to the default
    /// collection, but text renders at the wrong size).
    pub(crate) fn select(&self, logical_size: f32, scale: f32) -> Option<&FontCollection> {
        let physical_px = logical_size * scale;
        let q6 = size_key(physical_px);
        let result = self.collections.get(&q6);
        if result.is_none() {
            log::warn!(
                "UiFontSizes: no collection for {logical_size}px \
                 (physical {physical_px:.1}px, q6={q6}). \
                 Add this size to PRELOAD_SIZES to avoid fallback."
            );
        }
        result
    }

    /// Register a collection for a logical size not in [`PRELOAD_SIZES`].
    ///
    /// Call this from `&mut self` contexts (e.g. font config changes, DPI
    /// updates, settings UI adding a new size option) before creating an
    /// immutable `UiFontMeasurer`. The measurer has `&self` access only and
    /// cannot create collections at render time.
    #[allow(
        dead_code,
        reason = "public API for runtime size registration; exercised in tests"
    )]
    pub(crate) fn ensure_size(&mut self, logical_size: f32, scale: f32) -> Result<(), FontError> {
        let physical_px = logical_size * scale;
        let q6 = size_key(physical_px);
        if !self.collections.contains_key(&q6) {
            let size_pt = logical_to_pt(logical_size);
            let mut fc = FontCollection::new(
                self.font_set.clone(),
                size_pt,
                self.dpi,
                self.format,
                self.weight,
                self.bold_weight,
                self.hinting,
            )?;
            if let Some(ref hook) = self.post_rebuild_hook {
                hook(&mut fc);
            }
            self.collections.insert(q6, fc);
            self.logical_sizes.push(logical_size);
        }
        Ok(())
    }

    /// Look up a collection by its physical `size_q6` key.
    pub(crate) fn select_by_q6(&self, size_q6: u32) -> Option<&FontCollection> {
        self.collections.get(&size_q6)
    }

    /// Mutable lookup by physical `size_q6` key.
    pub(crate) fn select_by_q6_mut(&mut self, size_q6: u32) -> Option<&mut FontCollection> {
        self.collections.get_mut(&size_q6)
    }

    // Configuration changes

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

    /// Create a standalone [`FontCollection`] at the default body text size.
    ///
    /// Used by [`WindowRenderer::new_ui_only`] which needs a `FontCollection`
    /// in the terminal font slot for atlas seeding. Applies the post-rebuild
    /// hook so the returned collection has font config applied.
    pub(crate) fn create_default_collection(&self) -> Result<FontCollection, FontError> {
        let size_pt = logical_to_pt(DEFAULT_LOGICAL_SIZE);
        let mut fc = FontCollection::new(
            self.font_set.clone(),
            size_pt,
            self.dpi,
            self.format,
            self.weight,
            self.bold_weight,
            self.hinting,
        )?;
        if let Some(ref hook) = self.post_rebuild_hook {
            hook(&mut fc);
        }
        Ok(fc)
    }

    // Internals

    /// Rebuild all collections from scratch after a DPI change.
    ///
    /// Reapplies the post-rebuild hook (font config) to each new collection.
    fn rebuild_all(&mut self) -> Result<(), FontError> {
        self.collections.clear();
        let logical_sizes = std::mem::take(&mut self.logical_sizes);
        let mut new_default_q6 = 0;

        for &logical_px in &logical_sizes {
            let size_pt = logical_to_pt(logical_px);
            let mut fc = FontCollection::new(
                self.font_set.clone(),
                size_pt,
                self.dpi,
                self.format,
                self.weight,
                self.bold_weight,
                self.hinting,
            )?;
            if let Some(ref hook) = self.post_rebuild_hook {
                hook(&mut fc);
            }
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
