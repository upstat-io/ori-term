//! Resolved visual constraints for a single primitive.
//!
//! Computed at paint time from accumulated clip stacks. Each primitive
//! carries its own `ContentMask` so the GPU renderer reads it directly
//! without processing stack commands.

use crate::geometry::Rect;

/// Resolved visual constraints for a single primitive.
///
/// Computed at paint time from accumulated clip stacks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentMask {
    /// Viewport-space clip rect (intersection of all ancestor clips).
    pub clip: Rect,
}

impl ContentMask {
    /// No clipping — the entire viewport is visible.
    pub fn unclipped() -> Self {
        Self {
            clip: Rect::from_ltrb(
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            ),
        }
    }
}
