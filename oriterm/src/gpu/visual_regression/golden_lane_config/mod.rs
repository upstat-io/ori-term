//! Configuration for the deterministic golden image lane.
//!
//! Stores font configuration + comparison parameters. Cell metrics
//! are NOT stored here — they are DERIVED from `FontCollection::cell_metrics()`
//! after constructing the font with these parameters. See
//! `font/collection/mod.rs` for the SSOT.
//!
//! Adding independent cell metric fields (`cell_width_px`, `cell_height_px`)
//! would create a `` against `FontCollection::cell_metrics()`.

use crate::font::{FontSet, GlyphFormat, HintingMode};

/// Configuration for spec-conformance golden image tests.
/// Cell metrics (`cell_width_px`, `cell_height_px`) are intentionally absent.
/// Construct a `FontCollection` from the font parameters in this struct and
/// call `FontCollection::cell_metrics()` to obtain authoritative cell
/// dimensions. Adding independent cell metric fields here would create a
/// `` against `font/collection/mod.rs`.
/// `Debug` is implemented manually because `FontSet` intentionally does not
/// derive `Debug` (its owned byte buffers are large and uninformative).
#[derive(Clone)]
pub struct GoldenLaneConfig {
    // Font parameters (inputs to FontCollection construction).
    /// Font size in points (e.g. 12.0).
    pub font_size_pt: f32,
    /// Display DPI (e.g. 96.0).
    pub dpi: f32,
    /// Glyph rasterization format.
    pub glyph_format: GlyphFormat,
    /// Hinting mode for glyph rasterization.
    pub hinting_mode: HintingMode,

    // Viewport dimensions (in grid cells).
    /// Number of columns in the terminal grid.
    pub viewport_cols: u32,
    /// Number of rows in the terminal grid.
    pub viewport_rows: u32,

    // FrameInput determinism.
    /// Whether subpixel positioning is enabled.
    /// `false` snaps all glyphs to integer pixel boundaries, eliminating
    /// fractional-offset variation that causes cross-run pixel differences.
    pub subpixel_positioning: bool,

    // Comparison parameters.
    /// Maximum per-channel difference (0 = exact match).
    pub pixel_tolerance: u8,
    /// Maximum percentage of pixels allowed to differ (0.0 = zero mismatches).
    pub max_diff_percent: f64,

    // Subpixel pipeline selection (test-only).
    /// Force the NON-dual subpixel pipeline even when the device advertises
    /// `DUAL_SOURCE_BLENDING`. `false` (the `SPEC_DEFAULT` value) keeps the
    /// production selection (dual-source when available). `true` flips
    /// `GpuState::dual_source_blending` to `false` before pipeline creation so
    /// the `subpixel_fg.wgsl` known-bg branch (the F2 Porter-Duff "glyph over
    /// translucent bg" path) is exercised — the dual-source path composites
    /// over the framebuffer in hardware and never reaches that branch.
    pub force_non_dual_subpixel: bool,

    // Harness font override (test-only).
    /// Optional test-only `FontSet` override for the deterministic lane.
    /// `None` (the `SPEC_DEFAULT` value) keeps `FontSet::embedded()` — the
    /// canonical JetBrains Mono fixture used by every production golden.
    /// `Some(custom)` lets a test inject a font that advertises coverage of
    /// a built-in-rendered codepoint with an obviously-wrong glyph, so the
    /// precedence tests in §11.2 can prove the built-in Canvas renderer
    /// wins unconditionally over the configured font.
    pub font_override: Option<FontSet>,
}

impl std::fmt::Debug for GoldenLaneConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoldenLaneConfig")
            .field("font_size_pt", &self.font_size_pt)
            .field("dpi", &self.dpi)
            .field("glyph_format", &self.glyph_format)
            .field("hinting_mode", &self.hinting_mode)
            .field("viewport_cols", &self.viewport_cols)
            .field("viewport_rows", &self.viewport_rows)
            .field("subpixel_positioning", &self.subpixel_positioning)
            .field("pixel_tolerance", &self.pixel_tolerance)
            .field("max_diff_percent", &self.max_diff_percent)
            .field("force_non_dual_subpixel", &self.force_non_dual_subpixel)
            .field("font_override", &self.font_override.is_some())
            .finish()
    }
}

impl GoldenLaneConfig {
    /// Canonical spec-conformance defaults.
    /// - 12pt @ 96 DPI: matches existing visual_regression test font.
    /// - `HintingMode::None`: grayscale alpha for reproducibility.
    /// - `GlyphFormat::Alpha`: no subpixel color rendering.
    /// - `subpixel_positioning: false`: snaps glyphs to integer pixel
    /// boundaries for exact matching.
    /// - `pixel_tolerance: 0`: exact per-pixel match required.
    /// - `max_diff_percent: 0.0`: zero mismatches allowed.
    pub const SPEC_DEFAULT: Self = Self {
        font_size_pt: 12.0,
        dpi: 96.0,
        glyph_format: GlyphFormat::Alpha,
        hinting_mode: HintingMode::None,
        viewport_cols: 80,
        viewport_rows: 24,
        subpixel_positioning: false,
        pixel_tolerance: 0,
        max_diff_percent: 0.0,
        force_non_dual_subpixel: false,
        font_override: None,
    };

    /// Non-dual companion of `SPEC_DEFAULT`: identical except
    /// `glyph_format: GlyphFormat::SubpixelRgb` + `force_non_dual_subpixel:
    /// true`. The SSOT for golden tests that exercise the
    /// non-dual subpixel pipeline branch (the `subpixel_fg.wgsl` known-bg
    /// translucent-bg path) — pairs with `SPEC_DEFAULT` on dual-source-
    /// advertising adapters where `SPEC_DEFAULT` always selects the dual
    /// path. Subpixel-AA-exercising test categories (cell alpha, cursor
    /// opacity, weight tests, adjacency / raster) should declare paired
    /// `_non_dual` variants using this constant.
    pub const NON_DUAL_DEFAULT: Self = Self {
        glyph_format: GlyphFormat::SubpixelRgb,
        force_non_dual_subpixel: true,
        ..Self::SPEC_DEFAULT
    };

    /// Builder: return a config that carries the given `FontSet` as the
    /// harness's primary font, replacing `FontSet::embedded()` in the
    /// deterministic lane. Test-only — consumed by font-precedence tests
    /// that need to inject a custom font advertising coverage of built-in
    /// codepoints.
    #[must_use]
    pub fn with_font_override(mut self, font: FontSet) -> Self {
        self.font_override = Some(font);
        self
    }
}

#[cfg(test)]
mod tests;
