//! Text types for UI rendering.
//!
//! Provides style descriptors, shaped glyph output, and measurement results
//! for non-grid UI text (labels, tab titles, overlays). These types are
//! GPU-agnostic — shaping and rasterization live in the `oriterm` crate.

pub mod editing;

use std::borrow::Cow;

use crate::color::Color;

/// CSS-style text transformation applied before shaping.
///
/// Applied once in the shared text pipeline — widgets set this on
/// [`TextStyle`] rather than mutating source strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextTransform {
    /// No transformation (default).
    #[default]
    None,
    /// Convert all characters to uppercase (`text-transform: uppercase`).
    Uppercase,
    /// Convert all characters to lowercase (`text-transform: lowercase`).
    Lowercase,
}

impl TextTransform {
    /// Apply the transform to `text`, borrowing when no change is needed.
    pub fn apply(self, text: &str) -> Cow<'_, str> {
        match self {
            Self::None => Cow::Borrowed(text),
            Self::Uppercase => Cow::Owned(text.to_uppercase()),
            Self::Lowercase => Cow::Owned(text.to_lowercase()),
        }
    }
}

/// Numeric font weight for UI text (CSS-style, 100–900).
///
/// Thin (100) through Black (900) in 100-step increments. Values outside
/// 100–900 are clamped. Named constants match the CSS `font-weight` keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMIBOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    /// Create a font weight from a numeric value, clamped to 100–900.
    pub const fn new(weight: u16) -> Self {
        Self(if weight < 100 {
            100
        } else if weight > 900 {
            900
        } else {
            weight
        })
    }

    /// The numeric weight value (100–900).
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Horizontal text alignment within a bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    /// Left-aligned (default for LTR text).
    #[default]
    Left,
    /// Horizontally centered.
    Center,
    /// Right-aligned.
    Right,
}

/// Which font source a text style should use for shaping.
///
/// `Ui` selects the embedded UI font (IBM Plex Mono). `Terminal` selects the
/// user's configured terminal font collection, which includes emoji fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontSource {
    /// Embedded UI font (IBM Plex Mono). Default for settings, menus, etc.
    #[default]
    Ui,
    /// Terminal font with emoji fallback. Use for tab titles, status text,
    /// and anything that may contain user/OSC-provided content.
    Terminal,
}

/// How text that exceeds its container width is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextOverflow {
    /// Clip at the container edge (no visual indicator).
    #[default]
    Clip,
    /// Truncate with ellipsis (U+2026 `...`).
    Ellipsis,
    /// Wrap at word boundaries to the next line.
    Wrap,
}

/// Style descriptor for UI text rendering.
///
/// Input to the shaping pipeline. The shaper uses these parameters to select
/// the correct font face and size, then produces a [`ShapedText`] block.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font family name. `None` uses the default UI font.
    pub font_family: Option<String>,
    /// Font size in logical pixels (CSS-like: 13.0 = body, 18.0 = title).
    pub size: f32,
    /// Font weight.
    pub weight: FontWeight,
    /// Text color.
    pub color: Color,
    /// Horizontal alignment within the layout box.
    pub align: TextAlign,
    /// Overflow handling when text exceeds available width.
    pub overflow: TextOverflow,
    /// Extra spacing between characters in pixels. `0.0` = normal.
    pub letter_spacing: f32,
    /// Case transformation applied before shaping.
    pub text_transform: TextTransform,
    /// Line-height multiplier override (`size * multiplier` = line box height).
    ///
    /// `None` uses natural font metrics. `Some(1.5)` sets the line box height
    /// to `size * 1.5`. Use [`normalized_line_height`](Self::normalized_line_height)
    /// to filter invalid values.
    pub line_height: Option<f32>,
    /// Which font source to use for shaping. Default: `Ui` (embedded font).
    pub font_source: FontSource,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: None,
            size: 12.0,
            weight: FontWeight::NORMAL,
            color: Color::WHITE,
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            letter_spacing: 0.0,
            text_transform: TextTransform::None,
            line_height: None,
            font_source: FontSource::Ui,
        }
    }
}

impl TextStyle {
    /// Create a text style with the given size and color, using defaults for
    /// all other fields.
    pub fn new(size: f32, color: Color) -> Self {
        Self {
            font_family: None,
            size,
            weight: FontWeight::NORMAL,
            color,
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            letter_spacing: 0.0,
            text_transform: TextTransform::None,
            line_height: None,
            font_source: FontSource::Ui,
        }
    }

    /// Use the terminal font (with emoji fallback) instead of the UI font.
    #[must_use]
    pub fn with_terminal_font(mut self) -> Self {
        self.font_source = FontSource::Terminal;
        self
    }

    /// Set the font weight.
    #[must_use]
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the text alignment.
    #[must_use]
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set the overflow behavior.
    #[must_use]
    pub fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Set letter spacing in pixels.
    #[must_use]
    pub fn with_letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    /// Set the text transform.
    #[must_use]
    pub fn with_text_transform(mut self, transform: TextTransform) -> Self {
        self.text_transform = transform;
        self
    }

    /// Set the line-height multiplier (e.g. `1.5` for 150% of font size).
    #[must_use]
    pub fn with_line_height(mut self, multiplier: f32) -> Self {
        self.line_height = Some(multiplier);
        self
    }

    /// Returns the line-height multiplier if valid (finite and positive), or `None`.
    ///
    /// Invalid overrides (`<= 0.0`, `NaN`, infinities) normalize to `None`,
    /// falling back to natural font metrics.
    pub fn normalized_line_height(&self) -> Option<f32> {
        self.line_height.filter(|m| m.is_finite() && *m > 0.0)
    }
}

/// A shaped glyph — unified output for both terminal grid and UI text.
///
/// Output of the shaping pipeline, input to the GPU renderer. Uses pixel-based
/// `x_advance` positioning. Grid-column mapping is a terminal-specific concern
/// stored in parallel arrays outside this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Glyph ID within the font face (0 for advance-only entries like spaces).
    pub glyph_id: u16,
    /// Font face index as a raw `u16`. Avoids dependency on `oriterm`'s
    /// `FaceIdx` type — the renderer maps this back.
    pub face_index: u16,
    /// `SyntheticFlags` bits (0 = none, 1 = bold, 2 = italic).
    pub synthetic: u8,
    /// Horizontal advance in pixels (cursor moves right by this amount).
    pub x_advance: f32,
    /// Shaper X offset from the glyph origin in pixels.
    pub x_offset: f32,
    /// Shaper Y offset from the baseline in pixels.
    pub y_offset: f32,
}

/// Pre-shaped text block ready for rendering.
///
/// Contains the shaped glyph sequence and layout metrics. Produced by the
/// shaper in the `oriterm` crate, consumed by the draw list converter to
/// emit GPU glyph instances.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    /// Shaped glyphs in visual order.
    pub glyphs: Vec<ShapedGlyph>,
    /// Total advance width in pixels.
    pub width: f32,
    /// Line height in pixels.
    pub height: f32,
    /// Baseline offset from the top of the text block in pixels.
    pub baseline: f32,
    /// Font size in 26.6 fixed-point physical pixels.
    ///
    /// Stamped by the shaper from the `FontCollection` that produced this run.
    /// Used by scene conversion to construct `RasterKey`s with the correct size,
    /// enabling mixed-size text in one frame (e.g. 18px titles + 10px sidebar).
    /// Test/mock code can pass `0`.
    pub size_q6: u32,
    /// Requested font weight (CSS numeric value, 100–900).
    ///
    /// Stamped by the shaper from the `TextStyle.weight` that produced this run.
    /// Used by scene conversion to construct `RasterKey`s with the correct weight,
    /// preventing atlas collisions between different weight requests.
    /// Test/mock code can pass `400`.
    pub weight: u16,
}

impl ShapedText {
    /// Create a shaped text block from pre-computed data.
    #[expect(
        clippy::too_many_arguments,
        reason = "weight parameter added for CSS font-weight threading; grouping into a struct would obscure a simple data constructor"
    )]
    pub fn new(
        glyphs: Vec<ShapedGlyph>,
        width: f32,
        height: f32,
        baseline: f32,
        size_q6: u32,
        weight: u16,
    ) -> Self {
        Self {
            glyphs,
            width,
            height,
            baseline,
            size_q6,
            weight,
        }
    }

    /// Whether this text block contains no glyphs.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Number of shaped glyphs.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }
}

/// Text measurement result — dimensions without glyph data.
///
/// Lighter than [`ShapedText`] when only layout dimensions are needed
/// (e.g. for hit testing or container sizing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetrics {
    /// Total text width in pixels.
    pub width: f32,
    /// Total text height in pixels.
    pub height: f32,
    /// Number of lines (1 for single-line text, more with wrapping).
    pub line_count: u32,
}

#[cfg(test)]
mod tests;
