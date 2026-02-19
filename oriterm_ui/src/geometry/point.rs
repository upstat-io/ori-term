//! A 2D point parameterized by coordinate space.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Sub};

use super::units::Logical;

/// A point in 2D space, using `f32` pixels.
///
/// The type parameter `U` tags the coordinate space (e.g. [`Logical`],
/// [`Physical`](super::units::Physical)). Defaults to [`Logical`] so
/// unparameterized `Point` works identically to the previous untagged type.
#[must_use]
pub struct Point<U = Logical> {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
    /// Coordinate space marker (zero-sized).
    #[doc(hidden)]
    pub _unit: PhantomData<U>,
}

// Manual trait impls to avoid spurious `U: Trait` bounds from derive.

impl<U> Copy for Point<U> {}

impl<U> Clone for Point<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> fmt::Debug for Point<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl<U> PartialEq for Point<U> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<U> Default for Point<U> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl<U> Point<U> {
    /// Creates a new point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _unit: PhantomData,
        }
    }

    /// Returns a new point offset by `(dx, dy)`.
    pub fn offset(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    /// Returns a new point with both coordinates scaled.
    pub fn scale(self, sx: f32, sy: f32) -> Self {
        Self::new(self.x * sx, self.y * sy)
    }

    /// Euclidean distance to another point.
    pub fn distance_to(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }
}

impl<U> Add for Point<U> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<U> Sub for Point<U> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
