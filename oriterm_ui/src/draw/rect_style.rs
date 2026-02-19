//! Visual style for rectangular UI elements.

use crate::color::Color;

use super::border::Border;
use super::gradient::Gradient;
use super::shadow::Shadow;

/// Visual properties for a styled rectangle.
///
/// All fields are optional — an unstyled rect is invisible. Use the builder
/// methods to construct incrementally.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RectStyle {
    /// Solid fill color.
    pub fill: Option<Color>,
    /// Border drawn inside the rect edges.
    pub border: Option<Border>,
    /// Per-corner radii `[top_left, top_right, bottom_right, bottom_left]`.
    pub corner_radius: [f32; 4],
    /// Drop shadow behind the rect.
    pub shadow: Option<Shadow>,
    /// Linear gradient fill (deferred — falls back to first stop color).
    pub gradient: Option<Gradient>,
}

impl RectStyle {
    /// Creates a style with a solid fill color.
    #[must_use]
    pub fn filled(color: Color) -> Self {
        Self {
            fill: Some(color),
            ..Default::default()
        }
    }

    /// Adds a border.
    #[must_use]
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = Some(Border { width, color });
        self
    }

    /// Sets a uniform corner radius on all four corners.
    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.corner_radius = [radius; 4];
        self
    }

    /// Sets per-corner radii `[top_left, top_right, bottom_right, bottom_left]`.
    #[must_use]
    pub fn with_per_corner_radius(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.corner_radius = [tl, tr, br, bl];
        self
    }

    /// Adds a drop shadow.
    #[must_use]
    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
}
