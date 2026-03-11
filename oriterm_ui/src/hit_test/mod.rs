//! Platform-independent hit testing for frameless window chrome.
//!
//! Translates a point in window coordinates into a semantic region
//! (`Client`, `Caption`, or `ResizeBorder`). The OS-specific window
//! procedure calls this pure function and maps the result to native
//! constants (e.g. `WM_NCHITTEST` on Windows).

use crate::geometry::{Point, Rect, Size};

/// Window chrome layout parameters for hit testing.
///
/// Bundles the window-level geometry needed to classify a point as client
/// area, caption, or resize border.
pub struct WindowChrome<'a> {
    /// Total window size in physical pixels.
    pub window_size: Size,
    /// Resize border width in physical pixels.
    pub border_width: f32,
    /// Caption (title/tab bar) height in physical pixels.
    pub caption_height: f32,
    /// Rects within the caption that intercept clicks (buttons, tabs).
    pub interactive_rects: &'a [Rect],
    /// Whether the window is maximized (suppresses resize borders).
    pub is_maximized: bool,
}

/// The semantic region a point falls in within a frameless window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestResult {
    /// Standard client area (terminal grid, buttons, tabs).
    Client,
    /// Draggable caption area (title bar / tab bar background).
    Caption,
    /// Resizable border or corner.
    ResizeBorder(ResizeDirection),
}

/// Direction for a resize border hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDirection {
    /// Top edge.
    Top,
    /// Bottom edge.
    Bottom,
    /// Left edge.
    Left,
    /// Right edge.
    Right,
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
}

impl ResizeDirection {
    /// Maps to winit's compass-based resize direction.
    pub fn to_winit(self) -> winit::window::ResizeDirection {
        match self {
            Self::Top => winit::window::ResizeDirection::North,
            Self::Bottom => winit::window::ResizeDirection::South,
            Self::Left => winit::window::ResizeDirection::West,
            Self::Right => winit::window::ResizeDirection::East,
            Self::TopLeft => winit::window::ResizeDirection::NorthWest,
            Self::TopRight => winit::window::ResizeDirection::NorthEast,
            Self::BottomLeft => winit::window::ResizeDirection::SouthWest,
            Self::BottomRight => winit::window::ResizeDirection::SouthEast,
        }
    }
}

/// Determines the semantic region for a point within a frameless window.
///
/// Priority hierarchy:
///
/// 1. Resize edges/corners (unless maximized) -> `ResizeBorder`.
/// 2. Interactive rects within caption (buttons, tabs) -> `Client`.
/// 3. Caption area -> `Caption` (draggable).
/// 4. Everything else -> `Client`.
///
/// Resize borders have highest priority so that corners near window
/// control buttons remain resizable (e.g. top-right corner overlapping
/// the close button still allows diagonal resize).
///
/// Corners take priority over edges: a point in the top-left corner
/// returns `TopLeft`, not `Top` or `Left`.
///
/// All coordinates must be in the same unit space — caller ensures this.
/// The function is unit-agnostic: it works with physical pixels, logical
/// pixels, or any consistent coordinate system.
pub fn hit_test(point: Point, chrome: &WindowChrome<'_>) -> HitTestResult {
    // 1. Check resize borders first (suppressed when maximized).
    //    Resize always wins over buttons/caption so corners near window
    //    controls remain resizable.
    if !chrome.is_maximized {
        if let Some(direction) = resize_direction(point, chrome.window_size, chrome.border_width) {
            return HitTestResult::ResizeBorder(direction);
        }
    }

    // 2. Check interactive rects — buttons/tabs within caption are
    //    clickable, not draggable.
    for rect in chrome.interactive_rects {
        if rect.contains(point) {
            return HitTestResult::Client;
        }
    }

    // 3. Check caption area.
    if point.y < chrome.caption_height {
        return HitTestResult::Caption;
    }

    // 4. Everything else is client area.
    HitTestResult::Client
}

/// Returns the resize direction if the point is within `border_width` of
/// any window edge. Corners take priority over edges.
fn resize_direction(point: Point, window_size: Size, border_width: f32) -> Option<ResizeDirection> {
    let w = window_size.width();
    let h = window_size.height();

    let on_left = point.x < border_width;
    let on_right = point.x >= w - border_width;
    let on_top = point.y < border_width;
    let on_bottom = point.y >= h - border_width;

    // Corners first (higher priority than edges).
    match (on_left, on_right, on_top, on_bottom) {
        (true, _, true, _) => Some(ResizeDirection::TopLeft),
        (_, true, true, _) => Some(ResizeDirection::TopRight),
        (true, _, _, true) => Some(ResizeDirection::BottomLeft),
        (_, true, _, true) => Some(ResizeDirection::BottomRight),
        (true, _, _, _) => Some(ResizeDirection::Left),
        (_, true, _, _) => Some(ResizeDirection::Right),
        (_, _, true, _) => Some(ResizeDirection::Top),
        (_, _, _, true) => Some(ResizeDirection::Bottom),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
