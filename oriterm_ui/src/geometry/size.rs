//! A 2D size parameterized by coordinate space.
//!
//! Dimensions are clamped so that values below a small epsilon threshold
//! are treated as zero. This prevents floating-point noise from producing
//! non-empty sizes that distort layout (Chrome's `SizeF` pattern).

use std::fmt;
use std::marker::PhantomData;

use super::units::Logical;

/// Epsilon threshold below which a dimension is clamped to zero.
///
/// Matches Chromium's `kTrivial = 8 * std::numeric_limits<float>::epsilon()`.
const TRIVIAL: f32 = 8.0 * f32::EPSILON;

/// Clamps a dimension value: anything at or below [`TRIVIAL`] becomes `0.0`.
fn clamp_dimension(v: f32) -> f32 {
    if v > TRIVIAL { v } else { 0.0 }
}

/// A 2D size in pixels, parameterized by coordinate space.
///
/// Both dimensions are epsilon-clamped on construction and mutation so
/// that near-zero noise values collapse to exactly `0.0`.
///
/// The type parameter `U` tags the coordinate space. Defaults to
/// [`Logical`] so unparameterized `Size` works identically to the
/// previous untagged type. All fields are private — use accessor methods.
#[must_use]
pub struct Size<U = Logical> {
    width: f32,
    height: f32,
    _unit: PhantomData<U>,
}

// Manual trait impls to avoid spurious `U: Trait` bounds from derive.

impl<U> Copy for Size<U> {}

impl<U> Clone for Size<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> fmt::Debug for Size<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Size")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl<U> PartialEq for Size<U> {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }
}

impl<U> Default for Size<U> {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            _unit: PhantomData,
        }
    }
}

impl<U> Size<U> {
    /// Creates a new size, clamping near-zero dimensions to `0.0`.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: clamp_dimension(width),
            height: clamp_dimension(height),
            _unit: PhantomData,
        }
    }

    /// Width in pixels.
    pub fn width(self) -> f32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(self) -> f32 {
        self.height
    }

    /// Sets the width, clamping near-zero to `0.0`.
    pub fn set_width(&mut self, width: f32) {
        self.width = clamp_dimension(width);
    }

    /// Sets the height, clamping near-zero to `0.0`.
    pub fn set_height(&mut self, height: f32) {
        self.height = clamp_dimension(height);
    }

    /// Returns `true` if either dimension is zero.
    pub fn is_empty(self) -> bool {
        self.width == 0.0 || self.height == 0.0
    }

    /// Area in pixels squared.
    pub fn area(self) -> f32 {
        self.width * self.height
    }

    /// Returns a new size with both dimensions scaled.
    pub fn scale(self, sx: f32, sy: f32) -> Self {
        Self::new(self.width * sx, self.height * sy)
    }
}
