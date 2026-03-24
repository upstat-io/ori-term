//! Resolved visual constraints for a single primitive.
//!
//! Computed at paint time from accumulated clip stacks. Each primitive
//! carries its own `ContentMask` so the GPU renderer reads it directly
//! without processing stack commands.

use crate::geometry::Rect;

/// Resolved visual constraints for a single primitive.
///
/// Computed at paint time from accumulated clip and opacity stacks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentMask {
    /// Viewport-space clip rect (intersection of all ancestor clips).
    pub clip: Rect,
    /// Cumulative subtree opacity (`0.0..=1.0`).
    pub opacity: f32,
}

impl ContentMask {
    /// No clipping, full opacity — the entire viewport is visible.
    pub fn unclipped() -> Self {
        Self {
            clip: Rect::from_ltrb(
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            ),
            opacity: 1.0,
        }
    }
}
