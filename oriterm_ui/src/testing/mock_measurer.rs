//! Mock text measurer for deterministic widget testing.
//!
//! Uses fixed metrics: each character is `char_width` pixels wide,
//! line height is `line_height` pixels, baseline at 80% of line height.
//! No Unicode width handling — all characters are treated as single-width.

use crate::text::{ShapedGlyph, ShapedText, TextMetrics, TextStyle};
use crate::widgets::text_measurer::TextMeasurer;

/// Deterministic text measurer for widget tests.
///
/// Every character occupies exactly `char_width` pixels. Text wraps
/// by dividing total width by `max_width` and rounding up. Shaping
/// produces one glyph per character with sequential IDs.
pub struct MockMeasurer {
    pub char_width: f32,
    pub line_height: f32,
}

impl MockMeasurer {
    /// Standard mock: 8px per char, 16px line height (const for static usage).
    pub const STANDARD: Self = Self {
        char_width: 8.0,
        line_height: 16.0,
    };

    /// Standard mock: 8px per char, 16px line height.
    pub fn new() -> Self {
        Self::STANDARD
    }
}

impl TextMeasurer for MockMeasurer {
    fn measure(&self, text: &str, _style: &TextStyle, max_width: f32) -> TextMetrics {
        let full_width = self.char_width * text.len() as f32;
        if max_width.is_finite() && full_width > max_width {
            // Simple wrapping: number of lines = ceil(full_width / max_width).
            let line_count = (full_width / max_width).ceil() as u32;
            TextMetrics {
                width: max_width,
                height: self.line_height * line_count as f32,
                line_count,
            }
        } else {
            TextMetrics {
                width: full_width,
                height: self.line_height,
                line_count: 1,
            }
        }
    }

    fn shape(&self, text: &str, _style: &TextStyle, _max_width: f32) -> ShapedText {
        let glyphs: Vec<ShapedGlyph> = text
            .chars()
            .enumerate()
            .map(|(i, _)| ShapedGlyph {
                glyph_id: (i as u16) + 1,
                face_index: 0,
                synthetic: 0,
                x_advance: self.char_width,
                x_offset: 0.0,
                y_offset: 0.0,
            })
            .collect();
        let width = self.char_width * text.len() as f32;
        let baseline = self.line_height * 0.8;
        ShapedText::new(glyphs, width, self.line_height, baseline)
    }
}
