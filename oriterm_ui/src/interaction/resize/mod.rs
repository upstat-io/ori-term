//! Reusable resize geometry for resizable UI regions.
//!
//! Provides edge/corner hit testing, cursor icon mapping, and resize
//! computation — extracted from floating pane drag logic for general use.

use winit::window::CursorIcon;

/// Edge or corner of a resizable region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Result of hit-testing a point against a resizable region's zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitZone {
    /// Over the title bar (drag to move).
    TitleBar,
    /// Over an edge or corner (drag to resize).
    Edge(ResizeEdge),
    /// Over the interior (no drag, just click-through).
    Interior,
}

/// Thresholds for hit-testing a resizable region.
#[derive(Debug, Clone, Copy)]
pub struct HitTestConfig {
    /// Distance from edge to count as edge hover (pixels).
    pub edge_threshold: f32,
    /// Distance from corner to count as corner hover (pixels).
    pub corner_size: f32,
    /// Height of the title bar zone (pixels).
    pub title_bar_height: f32,
}

/// Axis-aligned rectangle for resize operations.
///
/// Framework-agnostic — avoids coupling to any specific `Rect` type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeRect {
    /// Left edge x coordinate.
    pub x: f32,
    /// Top edge y coordinate.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// Result of a resize computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeResult {
    /// New x coordinate.
    pub x: f32,
    /// New y coordinate.
    pub y: f32,
    /// New width.
    pub width: f32,
    /// New height.
    pub height: f32,
    /// Whether the origin moved (top/left edge drag shifts position).
    pub needs_move: bool,
}

/// Maps a resize edge to the appropriate cursor icon.
pub fn resize_cursor(edge: ResizeEdge) -> CursorIcon {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorIcon::NsResize,
        ResizeEdge::Left | ResizeEdge::Right => CursorIcon::EwResize,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorIcon::NwseResize,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorIcon::NeswResize,
    }
}

/// Hit-tests a point against a resizable region's zones.
///
/// Returns `None` if `(px, py)` is outside `rect`. Otherwise returns the
/// zone: corners (highest priority), then edges, then title bar, then interior.
pub fn hit_test_floating_zone(
    px: f32,
    py: f32,
    rect: &ResizeRect,
    config: &HitTestConfig,
) -> Option<HitZone> {
    // Bounds check.
    if px < rect.x || px > rect.x + rect.width || py < rect.y || py > rect.y + rect.height {
        return None;
    }

    let dx_left = px - rect.x;
    let dx_right = (rect.x + rect.width) - px;
    let dy_top = py - rect.y;
    let dy_bottom = (rect.y + rect.height) - py;

    // Corners (highest priority).
    if dx_left < config.corner_size && dy_top < config.corner_size {
        return Some(HitZone::Edge(ResizeEdge::TopLeft));
    }
    if dx_right < config.corner_size && dy_top < config.corner_size {
        return Some(HitZone::Edge(ResizeEdge::TopRight));
    }
    if dx_left < config.corner_size && dy_bottom < config.corner_size {
        return Some(HitZone::Edge(ResizeEdge::BottomLeft));
    }
    if dx_right < config.corner_size && dy_bottom < config.corner_size {
        return Some(HitZone::Edge(ResizeEdge::BottomRight));
    }

    // Edges.
    if dx_left < config.edge_threshold {
        return Some(HitZone::Edge(ResizeEdge::Left));
    }
    if dx_right < config.edge_threshold {
        return Some(HitZone::Edge(ResizeEdge::Right));
    }
    if dy_top < config.edge_threshold {
        return Some(HitZone::Edge(ResizeEdge::Top));
    }
    if dy_bottom < config.edge_threshold {
        return Some(HitZone::Edge(ResizeEdge::Bottom));
    }

    // Title bar (top region excluding edges).
    if dy_top < config.title_bar_height {
        return Some(HitZone::TitleBar);
    }

    Some(HitZone::Interior)
}

/// Compute the new rect after a resize drag.
///
/// Given the initial rect, the drag edge, the mouse delta `(dx, dy)`, and
/// a minimum size constraint, returns the resulting rect and whether the
/// origin moved.
pub fn compute_resize(
    initial: &ResizeRect,
    edge: ResizeEdge,
    dx: f32,
    dy: f32,
    min_size: f32,
) -> ResizeResult {
    let (x, y, w, h) = (initial.x, initial.y, initial.width, initial.height);
    let mut rx = x;
    let mut ry = y;
    let mut rw = w;
    let mut rh = h;
    let mut moved = false;

    match edge {
        ResizeEdge::Right => {
            rw = (w + dx).max(min_size);
        }
        ResizeEdge::Bottom => {
            rh = (h + dy).max(min_size);
        }
        ResizeEdge::Left => {
            let new_w = (w - dx).max(min_size);
            rx = x + w - new_w;
            rw = new_w;
            moved = true;
        }
        ResizeEdge::Top => {
            let new_h = (h - dy).max(min_size);
            ry = y + h - new_h;
            rh = new_h;
            moved = true;
        }
        ResizeEdge::TopLeft => {
            let new_w = (w - dx).max(min_size);
            let new_h = (h - dy).max(min_size);
            rx = x + w - new_w;
            ry = y + h - new_h;
            rw = new_w;
            rh = new_h;
            moved = true;
        }
        ResizeEdge::TopRight => {
            rw = (w + dx).max(min_size);
            let new_h = (h - dy).max(min_size);
            ry = y + h - new_h;
            rh = new_h;
            moved = true;
        }
        ResizeEdge::BottomLeft => {
            let new_w = (w - dx).max(min_size);
            rx = x + w - new_w;
            rw = new_w;
            rh = (h + dy).max(min_size);
            moved = true;
        }
        ResizeEdge::BottomRight => {
            rw = (w + dx).max(min_size);
            rh = (h + dy).max(min_size);
        }
    }

    ResizeResult {
        x: rx,
        y: ry,
        width: rw,
        height: rh,
        needs_move: moved,
    }
}

#[cfg(test)]
mod tests;
