//! Border style for rectangular UI elements.

use crate::color::Color;

/// A uniform border drawn inside a rectangle's edges.
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
