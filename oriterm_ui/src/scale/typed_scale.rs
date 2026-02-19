//! Type-safe scale factor between coordinate spaces.
//!
//! [`Scale`] carries source and destination coordinate space markers so
//! the compiler prevents accidentally applying a logical-to-physical
//! scale where a physical-to-screen scale was needed.

use std::fmt;
use std::marker::PhantomData;

use crate::geometry::{Point, Rect, Size};

/// A scale factor that converts from coordinate space `Src` to `Dst`.
///
/// The `f32` value is the multiplicative factor: `dst = src * factor`.
pub struct Scale<Src, Dst> {
    factor: f32,
    _src: PhantomData<Src>,
    _dst: PhantomData<Dst>,
}

// Manual trait impls to avoid spurious bounds.

impl<Src, Dst> Copy for Scale<Src, Dst> {}

impl<Src, Dst> Clone for Scale<Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Src, Dst> fmt::Debug for Scale<Src, Dst> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scale")
            .field("factor", &self.factor)
            .finish()
    }
}

impl<Src, Dst> PartialEq for Scale<Src, Dst> {
    fn eq(&self, other: &Self) -> bool {
        self.factor == other.factor
    }
}

impl<Src, Dst> Scale<Src, Dst> {
    /// Creates a uniform scale factor.
    pub const fn uniform(factor: f32) -> Self {
        Self {
            factor,
            _src: PhantomData,
            _dst: PhantomData,
        }
    }

    /// Returns the raw scale factor value.
    pub const fn factor(self) -> f32 {
        self.factor
    }

    /// Transforms a point from `Src` to `Dst` coordinates.
    pub fn transform_point(self, point: Point<Src>) -> Point<Dst> {
        Point::new(point.x * self.factor, point.y * self.factor)
    }

    /// Transforms a size from `Src` to `Dst` coordinates.
    pub fn transform_size(self, size: Size<Src>) -> Size<Dst> {
        Size::new(size.width() * self.factor, size.height() * self.factor)
    }

    /// Transforms a rect from `Src` to `Dst` coordinates.
    pub fn transform_rect(self, rect: Rect<Src>) -> Rect<Dst> {
        Rect::from_origin_size(
            self.transform_point(rect.origin),
            self.transform_size(rect.size),
        )
    }

    /// Returns the inverse scale (`Dst` → `Src`).
    pub fn inverse(self) -> Scale<Dst, Src> {
        Scale::uniform(1.0 / self.factor)
    }
}
