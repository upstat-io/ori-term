//! Drawing primitives for UI rendering.
//!
//! GPU-agnostic types for describing what to draw. The actual conversion
//! to GPU instances lives in the `oriterm` crate's GPU module.

mod border;
pub mod damage;
mod gradient;
mod rect_style;
pub mod scene;
mod shadow;

pub use border::Border;
pub use damage::DamageTracker;
pub use gradient::{Gradient, GradientStop};
pub use rect_style::RectStyle;
pub use scene::{
    ContentMask, IconPrimitive, ImagePrimitive, LinePrimitive, Quad, Scene, TextRun, build_scene,
};
pub use shadow::Shadow;

/// Shrinks a Vec if capacity vastly exceeds usage (> 4x len and > 4096 elements).
///
/// Standard buffer shrink discipline shared by `Scene` and `DamageTracker`.
pub(crate) fn maybe_shrink_vec<T>(v: &mut Vec<T>) {
    let cap = v.capacity();
    let len = v.len();
    if cap > 4 * len && cap > 4096 {
        v.shrink_to(len * 2);
    }
}

#[cfg(test)]
mod tests;
