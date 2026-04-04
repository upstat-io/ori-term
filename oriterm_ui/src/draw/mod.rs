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

pub use border::{Border, BorderSides};
pub use damage::DamageTracker;
pub use gradient::{Gradient, GradientStop};
pub use rect_style::RectStyle;
pub use scene::{
    ContentMask, IconPrimitive, ImagePrimitive, LinePrimitive, Quad, Scene, TextRun, build_scene,
};
pub use shadow::Shadow;

// Re-export from oriterm_core for crate-internal use.
pub(crate) use oriterm_core::maybe_shrink_vec;

#[cfg(test)]
mod tests;
