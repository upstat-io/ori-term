//! Font metric and rasterization-config value types.
//!
//! Cell dimensions, stroke metrics, hinting mode, glyph format, subpixel
//! mode, and the shared [`FontRasterConfig`] consumed by `FontCollection`
//! and the UI font-size registry. Extracted from `mod.rs` to keep that
//! file under the 500-line limit.

/// Cell dimensions in pixels, derived from the font metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellMetrics {
    /// Cell width in pixels (fractional for subpixel accuracy).
    pub width: f32,
    /// Cell height in pixels (fractional for subpixel accuracy).
    pub height: f32,
    /// Distance from cell top to text baseline, in pixels.
    pub baseline: f32,
    /// Distance from baseline to underline stroke, in pixels.
    ///
    /// Positive values are below the baseline (typical). Extracted from the
    /// font's `post` table via swash `underline_offset`, negated so that a
    /// larger value means further below baseline.
    pub underline_offset: f32,
    /// Thickness of underline and strikethrough strokes, in pixels.
    ///
    /// Extracted from the font's OS/2 `stroke_size` via swash. Clamped to
    /// a minimum of 1.0 to ensure visibility at small sizes.
    pub stroke_size: f32,
    /// Distance from baseline to strikeout stroke, in pixels.
    ///
    /// Positive values are above the baseline (typical). Extracted from the
    /// font's OS/2 table via swash `strikeout_offset`.
    pub strikeout_offset: f32,
}

/// Stroke-positioning metrics for [`CellMetrics::new`].
///
/// Groups the three decoration-stroke offsets/thickness derived from the
/// font's `post`/OS-2 tables, distinct from the cell's geometric dimensions.
#[derive(Debug, Clone, Copy)]
pub struct StrokeMetrics {
    /// Distance from baseline to underline stroke, in pixels.
    pub underline_offset: f32,
    /// Thickness of underline/strikethrough strokes (clamped to ≥ 1.0).
    pub stroke_size: f32,
    /// Distance from baseline to strikeout stroke, in pixels.
    pub strikeout_offset: f32,
}

impl StrokeMetrics {
    /// Construct stroke metrics from underline offset, stroke size, and
    /// strikeout offset.
    pub fn new(underline_offset: f32, stroke_size: f32, strikeout_offset: f32) -> Self {
        Self {
            underline_offset,
            stroke_size,
            strikeout_offset,
        }
    }
}

impl CellMetrics {
    /// Create cell metrics from font-derived dimensions.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if any dimension is non-positive or non-finite.
    pub fn new(width: f32, height: f32, baseline: f32, stroke: StrokeMetrics) -> Self {
        let StrokeMetrics {
            underline_offset,
            stroke_size,
            strikeout_offset,
        } = stroke;
        debug_assert!(
            width > 0.0 && width.is_finite(),
            "cell width must be positive"
        );
        debug_assert!(
            height > 0.0 && height.is_finite(),
            "cell height must be positive"
        );
        debug_assert!(baseline.is_finite(), "baseline must be finite");
        Self {
            width,
            height,
            baseline,
            underline_offset,
            stroke_size: stroke_size.max(1.0),
            strikeout_offset,
        }
    }

    /// Number of columns that fit in the viewport width.
    pub fn columns(&self, viewport_width: u32) -> usize {
        (f64::from(viewport_width) / f64::from(self.width)).floor() as usize
    }

    /// Number of rows that fit in the viewport height.
    pub fn rows(&self, viewport_height: u32) -> usize {
        (f64::from(viewport_height) / f64::from(self.height)).floor() as usize
    }
}

/// Glyph hinting mode — controls grid-fitting of outlines to pixel boundaries.
///
/// Hinting snaps glyph outlines to the pixel grid for sharper rendering at
/// small sizes. On high-DPI displays (2x+) the extra pixels make hinting
/// unnecessary, so disabling it preserves outline shape fidelity.
///
/// swash only supports a boolean hint flag — no "light" mode — so two
/// variants is the honest representation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum HintingMode {
    /// Full hinting (snaps to pixel grid). Crispest text on non-high-DPI.
    #[default]
    Full,
    /// No hinting (preserves outline shape). Best on high-DPI (2x+) where
    /// subpixel precision isn't needed for sharpness.
    None,
}

impl HintingMode {
    /// Convert to the boolean flag expected by swash's `ScalerBuilder::hint()`.
    pub fn hint_flag(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Auto-detect hinting mode from display scale factor.
    ///
    /// `scale_factor < 2.0` → `Full` (non-high-DPI needs grid-fitting).
    /// `scale_factor >= 2.0` → `None` (Retina/4K has enough pixels).
    pub fn from_scale_factor(scale_factor: f64) -> Self {
        if scale_factor < 2.0 {
            Self::Full
        } else {
            Self::None
        }
    }
}

/// Rasterization output format.
///
/// Determines pixel layout in [`RasterizedGlyph::bitmap`](super::RasterizedGlyph::bitmap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphFormat {
    /// 1 byte/pixel grayscale alpha coverage.
    Alpha,
    /// 4 bytes/pixel RGBA per-channel subpixel coverage (R-G-B order).
    SubpixelRgb,
    /// 4 bytes/pixel RGBA per-channel subpixel coverage (B-G-R order).
    SubpixelBgr,
    /// 4 bytes/pixel RGBA premultiplied color (for color emoji).
    Color,
}

impl GlyphFormat {
    /// Bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Alpha => 1,
            Self::SubpixelRgb | Self::SubpixelBgr | Self::Color => 4,
        }
    }

    /// Whether this format is a subpixel variant.
    pub fn is_subpixel(self) -> bool {
        matches!(self, Self::SubpixelRgb | Self::SubpixelBgr)
    }
}

/// Rasterization configuration shared by [`FontCollection`] and the UI
/// font-size registry: glyph format, weights, and hinting mode.
///
/// Bundles the four font-config inputs threaded through `FontCollection::new`
/// and `UiFontSizes::new`, distinct from the per-collection font data and
/// sizing (`font_set`, `size_pt`, `dpi`).
#[derive(Debug, Clone, Copy)]
pub struct FontRasterConfig {
    /// Glyph rasterization format (alpha / subpixel / color).
    pub format: GlyphFormat,
    /// Regular text weight (CSS 100–900).
    pub weight: u16,
    /// Bold text weight (CSS 100–900).
    pub bold_weight: u16,
    /// Glyph hinting mode.
    pub hinting: HintingMode,
}

/// LCD subpixel rendering mode.
///
/// Controls whether glyphs are rasterized with per-channel coverage for
/// ~3x effective horizontal resolution on LCD displays. Automatically
/// disabled on high-DPI (scale >= 2.0) where subpixels are invisible.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SubpixelMode {
    /// RGB subpixel order (vast majority of displays).
    #[default]
    Rgb,
    /// BGR subpixel order (rare panels).
    Bgr,
    /// Disabled — grayscale alpha rendering only.
    None,
}

impl SubpixelMode {
    /// Auto-detect subpixel mode from display scale factor.
    ///
    /// `scale_factor < 2.0` → `Rgb` (subpixels visible on non-HiDPI).
    /// `scale_factor >= 2.0` → `None` (Retina/4K — subpixels invisible).
    pub fn from_scale_factor(scale_factor: f64) -> Self {
        if scale_factor < 2.0 {
            Self::Rgb
        } else {
            Self::None
        }
    }

    /// Auto-detect subpixel mode considering both scale and background opacity.
    ///
    /// Subpixel rendering over transparent backgrounds produces visible color
    /// fringing because the per-channel blending assumes an opaque background.
    /// When `opacity < 1.0`, forces grayscale regardless of scale factor.
    pub fn for_display(scale_factor: f64, opacity: f64) -> Self {
        if opacity < 1.0 {
            Self::None
        } else {
            Self::from_scale_factor(scale_factor)
        }
    }

    /// Convert to the [`GlyphFormat`] used for rasterization.
    ///
    /// Returns `Alpha` when subpixel is disabled, otherwise the matching
    /// subpixel format.
    pub fn glyph_format(self) -> GlyphFormat {
        match self {
            Self::Rgb => GlyphFormat::SubpixelRgb,
            Self::Bgr => GlyphFormat::SubpixelBgr,
            Self::None => GlyphFormat::Alpha,
        }
    }

    /// Whether subpixel rendering is enabled.
    #[allow(dead_code, reason = "convenience predicate for caller code")]
    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::None)
    }
}
