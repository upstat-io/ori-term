//! Drawing primitives for UI rendering.
//!
//! GPU-agnostic types for describing what to draw. The actual conversion
//! to GPU instances lives in the `oriterm` crate's `draw_list_convert` module.

mod border;
mod draw_list;
mod gradient;
mod rect_style;
mod shadow;

pub use border::Border;
pub use draw_list::{DrawCommand, DrawList};
pub use gradient::{Gradient, GradientStop};
pub use rect_style::RectStyle;
pub use shadow::Shadow;

#[cfg(test)]
mod tests;
