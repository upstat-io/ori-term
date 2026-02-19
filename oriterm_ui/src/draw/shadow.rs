//! Drop shadow for rectangular UI elements.

use crate::color::Color;

/// A box shadow cast by a rectangle.
///
/// Rendered as a separate expanded UI rect instance behind the main rect,
/// using the shadow color as fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    /// Horizontal shadow offset in logical pixels.
    pub offset_x: f32,
    /// Vertical shadow offset in logical pixels.
    pub offset_y: f32,
    /// Blur radius in logical pixels (0 = hard shadow).
    pub blur_radius: f32,
    /// Spread distance in logical pixels (expands/shrinks the shadow rect).
    pub spread: f32,
    /// Shadow color (typically semi-transparent black).
    pub color: Color,
}
