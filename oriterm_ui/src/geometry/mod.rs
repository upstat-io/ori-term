//! Geometry primitives for layout and rendering.
//!
//! Modeled after Chromium's `ui/gfx/geometry/`. All values are `f32` pixels,
//! parameterized by coordinate space ([`Logical`], [`Physical`], [`Screen`]).
//! Pure data types with no platform dependencies, fully testable.

mod insets;
mod layer_id;
mod point;
mod rect;
mod size;
mod transform2d;
pub mod units;

pub use insets::Insets;
pub use layer_id::LayerId;
pub use point::Point;
pub use rect::Rect;
pub use size::Size;
pub use transform2d::Transform2D;
pub use units::{Logical, Physical, Screen};

#[cfg(test)]
mod tests;
