//! Re-exports [`Transform2D`] from [`geometry`](crate::geometry).
//!
//! The canonical definition lives in `geometry::transform2d`. This module
//! exists for backward compatibility so `compositor::Transform2D` continues
//! to resolve.

pub use crate::geometry::Transform2D;
