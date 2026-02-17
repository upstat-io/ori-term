//! Frame input types for the Extract phase of the render pipeline.
//!
//! [`FrameInput`] composes `oriterm_core::RenderableContent` (the terminal
//! snapshot) with rendering context: viewport pixel dimensions, cell metrics,
//! and semantic palette colors. The Prepare phase consumes a `FrameInput` and
//! produces a [`PreparedFrame`](super::prepared_frame::PreparedFrame).

// FrameInput is consumed starting in Section 5.8; suppress until then.
#![expect(dead_code, reason = "GPU infrastructure used starting in Section 5.8")]

use oriterm_core::{RenderableContent, Rgb};

use crate::font::CellMetrics;

/// Placeholder for selection range — replaced in Section 9.
pub type SelectionRange = ();

/// Placeholder for search match — replaced in Section 11.
pub type SearchMatch = ();

/// Pixel dimensions of the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl ViewportSize {
    /// Create a viewport size from pixel dimensions.
    ///
    /// Dimensions are clamped to a minimum of 1 to avoid zero-size surfaces.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }
}

/// Semantic colors needed beyond per-cell resolved colors.
///
/// Per-cell fg/bg are already resolved in `RenderableCell`. This captures
/// only the three global colors the renderer needs: clear color, cursor
/// fill, and text-under-cursor inversion color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePalette {
    /// Window clear color (terminal background).
    pub background: Rgb,
    /// Default foreground (used for cursor text inversion).
    pub foreground: Rgb,
    /// Cursor rectangle fill color.
    pub cursor_color: Rgb,
}

/// Complete input for one render frame.
///
/// Composes the terminal snapshot ([`RenderableContent`]) with the rendering
/// context needed to convert logical cells into pixel geometry. Built during
/// the Extract phase, consumed by the Prepare phase.
pub struct FrameInput {
    /// Terminal content snapshot (cells, cursor, damage, mode).
    pub content: RenderableContent,
    /// Viewport pixel dimensions.
    pub viewport: ViewportSize,
    /// Cell pixel dimensions from font metrics.
    pub cell_size: CellMetrics,
    /// Semantic colors for background clear and cursor.
    pub palette: FramePalette,
    /// Active selection range (placeholder until Section 9).
    pub selection: Option<SelectionRange>,
    /// Active search matches (placeholder until Section 11).
    pub search_matches: Vec<SearchMatch>,
}

impl FrameInput {
    /// Number of grid columns based on viewport and cell size.
    pub fn columns(&self) -> usize {
        self.cell_size.columns(self.viewport.width)
    }

    /// Number of grid rows based on viewport and cell size.
    pub fn rows(&self) -> usize {
        self.cell_size.rows(self.viewport.height)
    }

    /// Whether the entire viewport needs a full repaint.
    pub fn needs_full_repaint(&self) -> bool {
        self.content.all_dirty
    }
}

#[cfg(test)]
mod tests;
