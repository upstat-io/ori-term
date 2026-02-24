//! Real text measurer for UI widgets backed by a [`FontCollection`].
//!
//! Lightweight adapter that wraps `&FontCollection` and delegates to the
//! existing shaping functions in [`ui_text`](super::ui_text). This replaces
//! the `NullMeasurer` stub so dialog titles, messages, and button labels
//! render with actual text.

use oriterm_ui::text::{ShapedText, TextMetrics, TextStyle};
use oriterm_ui::widgets::TextMeasurer;

use crate::font::collection::FontCollection;

use super::ui_text;

/// Text measurer backed by a real [`FontCollection`].
///
/// Created per-frame from the renderer's active UI font collection and passed
/// to widget layout/draw/event contexts. The `scale` factor converts between
/// the widget layout coordinate system (logical pixels) and the font's
/// rasterization coordinate system (physical pixels).
///
/// - [`measure()`](TextMeasurer::measure) returns metrics in logical pixels
///   (physical ÷ scale) so widget layout computes correct proportions.
/// - [`shape()`](TextMeasurer::shape) returns [`ShapedText`] with physical-pixel
///   advances so glyph bitmaps render at native resolution without stretching.
pub struct UiFontMeasurer<'a> {
    collection: &'a FontCollection,
    scale: f32,
}

impl<'a> UiFontMeasurer<'a> {
    /// Wrap a font collection for use as a text measurer.
    ///
    /// `scale` is the display scale factor (logical → physical pixel ratio).
    /// Pass `1.0` when no scaling is needed.
    pub fn new(collection: &'a FontCollection, scale: f32) -> Self {
        Self { collection, scale }
    }
}

impl TextMeasurer for UiFontMeasurer<'_> {
    fn measure(&self, text: &str, style: &TextStyle, _max_width: f32) -> TextMetrics {
        // Shaping produces physical-pixel metrics; convert to logical for layout.
        let phys = ui_text::measure_text_styled(text, style, self.collection);
        TextMetrics {
            width: phys.width / self.scale,
            height: phys.height / self.scale,
            line_count: phys.line_count,
        }
    }

    fn shape(&self, text: &str, style: &TextStyle, max_width: f32) -> ShapedText {
        // Widget passes logical max_width; convert to physical for truncation.
        let mut shaped = ui_text::shape_text(text, style, max_width * self.scale, self.collection);
        // Convert layout metrics to logical pixels for widget centering/positioning.
        // Glyph advances and baseline remain in physical pixels for rendering.
        shaped.width /= self.scale;
        shaped.height /= self.scale;
        shaped
    }
}
