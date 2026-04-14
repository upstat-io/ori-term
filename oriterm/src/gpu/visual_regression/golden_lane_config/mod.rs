//! Configuration for the deterministic golden image lane.
//!
//! Stores font configuration + comparison parameters. Cell metrics
//! are NOT stored here — they are DERIVED from `FontCollection::cell_metrics()`
//! after constructing the font with these parameters. See
//! `font/collection/mod.rs` for the SSOT.
//!
//! Adding independent cell metric fields (`cell_width_px`, `cell_height_px`)
//! would create a `LEAK:shadow-home` against `FontCollection::cell_metrics()`.

use crate::font::{GlyphFormat, HintingMode};

/// Configuration for spec-conformance golden image tests.
///
/// Cell metrics (`cell_width_px`, `cell_height_px`) are intentionally absent.
/// Construct a `FontCollection` from the font parameters in this struct and
/// call `FontCollection::cell_metrics()` to obtain authoritative cell
/// dimensions. Adding independent cell metric fields here would create a
/// `LEAK:shadow-home` against `font/collection/mod.rs`.
#[derive(Clone, Debug)]
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
    ///
    /// `false` snaps all glyphs to integer pixel boundaries, eliminating
    /// fractional-offset variation that causes cross-run pixel differences.
    pub subpixel_positioning: bool,

    // Comparison parameters.
    /// Maximum per-channel difference (0 = exact match).
    pub pixel_tolerance: u8,
    /// Maximum percentage of pixels allowed to differ (0.0 = zero mismatches).
    pub max_diff_percent: f64,
}

impl GoldenLaneConfig {
    /// Canonical spec-conformance defaults.
    ///
    /// - 12pt @ 96 DPI: matches existing visual_regression test font.
    /// - `HintingMode::None`: grayscale alpha for reproducibility.
    /// - `GlyphFormat::Alpha`: no subpixel color rendering.
    /// - `subpixel_positioning: false`: snaps glyphs to integer pixel
    ///   boundaries for exact matching.
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
    };
}

#[cfg(test)]
mod tests;
