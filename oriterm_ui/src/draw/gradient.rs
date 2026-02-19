//! Linear gradient types for rectangular UI elements.
//!
//! Types exist for API completeness. The gradient shader is deferred to a
//! later section — the converter falls back to the first stop's color.

use crate::color::Color;

/// A single color stop in a gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// Position along the gradient axis in `[0.0, 1.0]`.
    pub position: f32,
    /// Color at this stop.
    pub color: Color,
}

/// A linear gradient defined by an angle and color stops.
///
/// `angle` is in degrees: 0 = bottom-to-top, 90 = left-to-right.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// Gradient angle in degrees.
    pub angle: f32,
    /// Color stops (must contain at least two).
    pub stops: Vec<GradientStop>,
}
