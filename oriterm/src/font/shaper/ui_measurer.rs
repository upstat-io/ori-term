//! Size-aware text measurer for UI widgets.
//!
//! Wraps the [`UiFontSizes`] registry and delegates to the shaping functions
//! in [`ui_text`](super::ui_text). Each `TextStyle.size` selects the exact
//! [`FontCollection`] for that logical pixel size, so 18px titles and 10px
//! sidebar text rasterize at their true sizes.

use oriterm_ui::text::{ShapedText, TextMetrics, TextStyle};
use oriterm_ui::widgets::TextMeasurer;

use crate::font::collection::FontCollection;
use crate::font::ui_font_sizes::UiFontSizes;

use super::ui_text;

/// Text measurer backed by a [`UiFontSizes`] registry.
///
/// Created per-frame from the renderer's font state and passed to widget
/// layout/draw/event contexts. The `scale` factor converts between the widget
/// layout coordinate system (logical pixels) and the font's rasterization
/// coordinate system (physical pixels).
///
/// For each `TextStyle`, [`collection_for_style`](Self::collection_for_style)
/// selects the exact-size collection from the registry. If the registry is
/// absent or lacks the requested size, it falls back to `fallback`.
///
/// - [`measure()`](TextMeasurer::measure) returns metrics in logical pixels
///   (physical ÷ scale) so widget layout computes correct proportions.
/// - [`shape()`](TextMeasurer::shape) returns [`ShapedText`] with physical-pixel
///   advances so glyph bitmaps render at native resolution without stretching.
pub struct UiFontMeasurer<'a> {
    sizes: Option<&'a UiFontSizes>,
    fallback: &'a FontCollection,
    scale: f32,
}

impl<'a> UiFontMeasurer<'a> {
    /// Create a size-aware text measurer.
    ///
    /// `sizes` is the per-size UI font registry (`None` if no UI fonts loaded).
    /// `fallback` is the default collection used when `sizes` is absent or
    /// lacks the requested size. `scale` is the display scale factor
    /// (logical → physical pixel ratio).
    pub fn new(sizes: Option<&'a UiFontSizes>, fallback: &'a FontCollection, scale: f32) -> Self {
        Self {
            sizes,
            fallback,
            scale,
        }
    }

    /// Select the collection matching a text style's size.
    ///
    /// Looks up the exact physical size in the registry. Falls back to
    /// `self.fallback` if the registry is absent or the size isn't loaded.
    fn collection_for_style(&self, style: &TextStyle) -> &FontCollection {
        self.sizes
            .and_then(|sizes| sizes.select(style.size, self.scale))
            .unwrap_or(self.fallback)
    }
}

impl TextMeasurer for UiFontMeasurer<'_> {
    fn measure(&self, text: &str, style: &TextStyle, _max_width: f32) -> TextMetrics {
        let collection = self.collection_for_style(style);
        let phys_spacing = style.letter_spacing.max(0.0) * self.scale;
        let shaped = ui_text::shape_text(text, style, f32::INFINITY, phys_spacing, collection);
        let height = match style.normalized_line_height() {
            Some(multiplier) => style.size * multiplier,
            None => shaped.height / self.scale,
        };
        TextMetrics {
            width: shaped.width / self.scale,
            height,
            line_count: 1,
        }
    }

    fn shape(&self, text: &str, style: &TextStyle, max_width: f32) -> ShapedText {
        let collection = self.collection_for_style(style);
        let phys_spacing = style.letter_spacing.max(0.0) * self.scale;
        // Widget passes logical max_width; convert to physical for truncation.
        let mut shaped = ui_text::shape_text(
            text,
            style,
            max_width * self.scale,
            phys_spacing,
            collection,
        );

        if let Some(multiplier) = style.normalized_line_height() {
            let target_logical = style.size * multiplier;
            let target_physical = target_logical * self.scale;
            // shaped.height is still physical at this point (from ui_text::shape_text).
            let half_leading = (target_physical - shaped.height) / 2.0;
            // Shift baseline in physical space (scene_convert consumes it as physical).
            shaped.baseline += half_leading;
            shaped.height = target_logical;
        } else {
            shaped.height /= self.scale;
        }

        // Convert width to logical pixels for widget positioning.
        // Glyph advances and baseline remain in physical pixels for rendering.
        shaped.width /= self.scale;
        shaped
    }
}
