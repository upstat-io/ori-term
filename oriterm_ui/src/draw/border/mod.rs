//! Border style for rectangular UI elements.

use crate::color::Color;

/// A single border edge with width and color.
///
/// Width is in logical pixels. The border is rendered as the space between
/// the outer SDF and an inner SDF inset by `width`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// Border thickness in logical pixels.
    pub width: f32,
    /// Border color.
    pub color: Color,
}

/// Per-side border specification for rectangular UI elements.
///
/// Each side can independently have its own width and color, or be absent.
/// Use the constructors for common patterns and the `widths()`/`colors()`
/// accessors for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderSides {
    /// Top border.
    pub top: Option<Border>,
    /// Right border.
    pub right: Option<Border>,
    /// Bottom border.
    pub bottom: Option<Border>,
    /// Left border.
    pub left: Option<Border>,
}

impl BorderSides {
    /// Creates a uniform border with the same width and color on all four sides.
    #[must_use]
    pub fn uniform(width: f32, color: Color) -> Self {
        let side = Some(Border { width, color });
        Self {
            top: side,
            right: side,
            bottom: side,
            left: side,
        }
    }

    /// Creates a border on the top side only.
    #[must_use]
    pub fn only_top(width: f32, color: Color) -> Self {
        Self {
            top: Some(Border { width, color }),
            ..Default::default()
        }
    }

    /// Creates a border on the right side only.
    #[must_use]
    pub fn only_right(width: f32, color: Color) -> Self {
        Self {
            right: Some(Border { width, color }),
            ..Default::default()
        }
    }

    /// Creates a border on the bottom side only.
    #[must_use]
    pub fn only_bottom(width: f32, color: Color) -> Self {
        Self {
            bottom: Some(Border { width, color }),
            ..Default::default()
        }
    }

    /// Creates a border on the left side only.
    #[must_use]
    pub fn only_left(width: f32, color: Color) -> Self {
        Self {
            left: Some(Border { width, color }),
            ..Default::default()
        }
    }

    /// Returns `true` when no side has a visible border.
    ///
    /// A side is invisible if absent or has an invalid width (zero, negative,
    /// NaN, or infinite).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !Self::is_visible_side(self.top)
            && !Self::is_visible_side(self.right)
            && !Self::is_visible_side(self.bottom)
            && !Self::is_visible_side(self.left)
    }

    /// Returns `Some(border)` only when all four sides are present, have valid
    /// widths, and are identical (same width and color).
    ///
    /// Used by the scene conversion fast path to avoid per-side processing.
    #[must_use]
    pub fn as_uniform(&self) -> Option<Border> {
        let t = self.top?;
        let r = self.right?;
        let b = self.bottom?;
        let l = self.left?;

        if !Self::is_valid_width(t.width) {
            return None;
        }

        // Exact comparison is correct: widths are user-specified literals, not computed.
        #[expect(clippy::float_cmp, reason = "comparing user-specified border widths")]
        let widths_equal = t.width == r.width && r.width == b.width && b.width == l.width;
        if widths_equal && t.color == r.color && r.color == b.color && b.color == l.color {
            Some(t)
        } else {
            None
        }
    }

    /// Per-side widths as `[top, right, bottom, left]`, normalized for rendering.
    ///
    /// Invalid widths (zero, negative, NaN, infinite) are normalized to `0.0`.
    #[must_use]
    pub fn widths(&self) -> [f32; 4] {
        [
            Self::normalized_width(self.top),
            Self::normalized_width(self.right),
            Self::normalized_width(self.bottom),
            Self::normalized_width(self.left),
        ]
    }

    /// Per-side colors as `[top, right, bottom, left]`.
    ///
    /// Absent sides use `Color::TRANSPARENT`.
    #[must_use]
    pub fn colors(&self) -> [Color; 4] {
        [
            Self::side_color(self.top),
            Self::side_color(self.right),
            Self::side_color(self.bottom),
            Self::side_color(self.left),
        ]
    }

    /// Whether a width value is valid for rendering (finite and positive).
    fn is_valid_width(w: f32) -> bool {
        w.is_finite() && w > 0.0
    }

    /// Whether a side is visible (present with a valid width).
    fn is_visible_side(side: Option<Border>) -> bool {
        side.is_some_and(|b| Self::is_valid_width(b.width))
    }

    /// Extracts the normalized width from a side, returning 0.0 for absent or invalid.
    fn normalized_width(side: Option<Border>) -> f32 {
        match side {
            Some(b) if Self::is_valid_width(b.width) => b.width,
            _ => 0.0,
        }
    }

    /// Extracts the color from a side, returning transparent for absent or
    /// invisible sides (zero, negative, NaN, or infinite width).
    fn side_color(side: Option<Border>) -> Color {
        match side {
            Some(b) if Self::is_valid_width(b.width) => b.color,
            _ => Color::TRANSPARENT,
        }
    }
}

#[cfg(test)]
mod tests;
