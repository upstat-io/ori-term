//! Shared scrollbar styling, geometry, and rendering.
//!
//! Provides [`ScrollbarStyle`] (rest/hover/drag colors with separate visual
//! thickness and hit slop), [`ScrollbarAxis`], and pure geometry/draw helpers
//! that any scrollable widget can delegate to.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::theme::UiTheme;

/// Visual axis for a scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollbarAxis {
    /// Vertical scrollbar (right edge of viewport).
    Vertical,
    /// Horizontal scrollbar (bottom edge of viewport).
    Horizontal,
}

/// Overlay scrollbar appearance with explicit per-state colors.
///
/// Separates visual `thickness` from pointer `hit_slop` so the rendered
/// bar stays narrow (e.g. 6px) while the clickable area extends further
/// for comfortable drag acquisition.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbarStyle {
    /// Visible scrollbar width/height (logical pixels).
    pub thickness: f32,
    /// Extra invisible pointer hit area on each side (logical pixels).
    pub hit_slop: f32,
    /// Inset from the viewport edge (logical pixels).
    pub edge_inset: f32,
    /// Corner radius of the thumb.
    pub thumb_radius: f32,
    /// Minimum thumb length along the scroll axis (logical pixels).
    pub min_thumb_length: f32,
    /// Visible thickness when hovered or dragged (logical pixels).
    ///
    /// When the cursor is over the scrollbar area, the rendered track and
    /// thumb expand to this width for easier grab acquisition. Set equal
    /// to `thickness` to disable expansion.
    pub hover_thickness: f32,

    // Thumb colors per interaction state.
    /// Thumb color at rest (no hover, no drag).
    pub thumb_color: Color,
    /// Thumb color when the cursor hovers the track/thumb area.
    pub thumb_hover_color: Color,
    /// Thumb color while being dragged.
    pub thumb_drag_color: Color,

    // Track colors per interaction state.
    /// Track color at rest.
    pub track_color: Color,
    /// Track color on hover.
    pub track_hover_color: Color,
    /// Track color while the thumb is being dragged.
    pub track_drag_color: Color,
}

impl ScrollbarStyle {
    /// Create a style from theme tokens matching the brutal design mockup.
    ///
    /// - Rest thumb: `theme.border` (`#2a2a36` dark)
    /// - Hover/drag thumb: `theme.fg_faint` (`#8c8ca0` dark)
    /// - Track: transparent in all states
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            thickness: 6.0,
            hit_slop: 4.0,
            edge_inset: 2.0,
            thumb_radius: 3.0,
            min_thumb_length: 20.0,
            hover_thickness: 10.0,
            thumb_color: theme.border,
            thumb_hover_color: theme.fg_faint,
            thumb_drag_color: theme.fg_faint,
            track_color: Color::TRANSPARENT,
            track_hover_color: Color::TRANSPARENT,
            track_drag_color: Color::TRANSPARENT,
        }
    }
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::default())
    }
}

// Geometry

/// Input measurements for scrollbar geometry computation.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarMetrics {
    /// Which axis this bar represents.
    pub axis: ScrollbarAxis,
    /// Total content extent along the axis (logical pixels).
    pub content_extent: f32,
    /// Visible viewport extent along the axis (logical pixels).
    pub view_extent: f32,
    /// Current scroll offset along the axis (logical pixels, >= 0).
    pub scroll_offset: f32,
}

/// Computed scrollbar rects for rendering and hit testing.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarRects {
    /// Visible track rect.
    pub track: Rect,
    /// Expanded track rect for pointer hit testing.
    pub track_hit: Rect,
    /// Visible thumb rect.
    pub thumb: Rect,
    /// Expanded thumb rect for pointer hit testing.
    pub thumb_hit: Rect,
}

/// Current visual state of a scrollbar (drives color selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarVisualState {
    /// No interaction.
    #[default]
    Rest,
    /// Cursor over the track/thumb area.
    Hovered,
    /// Thumb is being dragged.
    Dragging,
}

/// Whether a scrollbar should be visible for the given metrics.
///
/// Returns `true` when content overflows the viewport.
pub fn should_show(metrics: &ScrollbarMetrics) -> bool {
    metrics.content_extent > metrics.view_extent
}

/// Compute visible and hit-test rects for an overlay scrollbar.
///
/// `viewport` is the scroll container's visible bounds.
/// `reserve_far_edge` is extra space to reserve at the far end of the track
/// (e.g. the other axis's scrollbar thickness when both axes are active).
pub fn compute_rects(
    viewport: Rect,
    metrics: &ScrollbarMetrics,
    style: &ScrollbarStyle,
    reserve_far_edge: f32,
) -> ScrollbarRects {
    let (track, track_hit) = track_rects(viewport, metrics.axis, style, reserve_far_edge);

    let ratio = if metrics.content_extent > 0.0 {
        metrics.view_extent / metrics.content_extent
    } else {
        1.0
    };

    let track_length = match metrics.axis {
        ScrollbarAxis::Vertical => track.height(),
        ScrollbarAxis::Horizontal => track.width(),
    };

    let thumb_len = (track_length * ratio)
        .max(style.min_thumb_length)
        .min(track_length);

    let scroll_range = (metrics.content_extent - metrics.view_extent).max(0.0);
    let scroll_ratio = if scroll_range > 0.0 {
        metrics.scroll_offset / scroll_range
    } else {
        0.0
    };
    let thumb_offset = scroll_ratio * (track_length - thumb_len);

    let (thumb, thumb_hit) = match metrics.axis {
        ScrollbarAxis::Vertical => {
            let visible = Rect::new(
                track.x(),
                track.y() + thumb_offset,
                style.thickness,
                thumb_len,
            );
            let hit = Rect::new(
                track_hit.x(),
                track.y() + thumb_offset,
                track_hit.width(),
                thumb_len,
            );
            (visible, hit)
        }
        ScrollbarAxis::Horizontal => {
            let visible = Rect::new(
                track.x() + thumb_offset,
                track.y(),
                thumb_len,
                style.thickness,
            );
            let hit = Rect::new(
                track.x() + thumb_offset,
                track_hit.y(),
                thumb_len,
                track_hit.height(),
            );
            (visible, hit)
        }
    };

    ScrollbarRects {
        track,
        track_hit,
        thumb,
        thumb_hit,
    }
}

/// Draw the scrollbar track and thumb with the given visual state.
///
/// When hovered or dragging, the rendered rects expand from `thickness`
/// to `hover_thickness` toward the viewport interior (the edge-side
/// position stays fixed). This makes the scrollbar easier to grab
/// without shifting the outer edge.
pub fn draw_overlay(
    scene: &mut Scene,
    rects: &ScrollbarRects,
    style: &ScrollbarStyle,
    state: ScrollbarVisualState,
) {
    let (track_color, thumb_color) = match state {
        ScrollbarVisualState::Rest => (style.track_color, style.thumb_color),
        ScrollbarVisualState::Hovered => (style.track_hover_color, style.thumb_hover_color),
        ScrollbarVisualState::Dragging => (style.track_drag_color, style.thumb_drag_color),
    };

    let expanded = state != ScrollbarVisualState::Rest
        && (style.hover_thickness - style.thickness).abs() > 0.01;

    // Draw track if visible.
    if track_color.a > 0.0 {
        let track = if expanded {
            expand_rect_inward(rects.track, style.hover_thickness - style.thickness)
        } else {
            rects.track
        };
        scene.push_quad(
            track,
            RectStyle::filled(track_color).with_radius(style.thumb_radius),
        );
    }

    // Draw thumb.
    let thumb = if expanded {
        expand_rect_inward(rects.thumb, style.hover_thickness - style.thickness)
    } else {
        rects.thumb
    };
    scene.push_quad(
        thumb,
        RectStyle::filled(thumb_color).with_radius(style.thumb_radius),
    );
}

/// Expand a rect inward (toward the viewport center) by `extra` pixels.
///
/// For a vertical scrollbar at the right edge, this extends the rect to the
/// left. For a horizontal scrollbar at the bottom, this extends upward.
/// The far-edge (right/bottom) stays fixed.
fn expand_rect_inward(r: Rect, extra: f32) -> Rect {
    // Determine orientation from aspect ratio: tall = vertical bar, wide = horizontal.
    if r.height() > r.width() {
        // Vertical bar: extend left.
        Rect::new(r.x() - extra, r.y(), r.width() + extra, r.height())
    } else {
        // Horizontal bar: extend upward.
        Rect::new(r.x(), r.y() - extra, r.width(), r.height() + extra)
    }
}

/// Convert a pointer position along the track into a scroll offset.
///
/// Useful for click-to-jump and drag-tracking. Returns the offset in
/// content coordinates, clamped to `[0, content_extent - view_extent]`.
pub fn pointer_to_offset(
    pointer_along_axis: f32,
    rects: &ScrollbarRects,
    metrics: &ScrollbarMetrics,
) -> f32 {
    let (track_start, track_length) = match metrics.axis {
        ScrollbarAxis::Vertical => (rects.track.y(), rects.track.height()),
        ScrollbarAxis::Horizontal => (rects.track.x(), rects.track.width()),
    };

    let ratio = if track_length > 0.0 {
        ((pointer_along_axis - track_start) / track_length).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let max = (metrics.content_extent - metrics.view_extent).max(0.0);
    (ratio * max).clamp(0.0, max)
}

/// Convert a drag delta (in pointer pixels) into a scroll offset delta.
///
/// Maps pixel movement on the track into content-space movement. Uses
/// the usable track space (`track_length - thumb_length`) so the thumb
/// tracks the pointer 1:1 without drifting.
pub fn drag_delta_to_offset(
    delta_pixels: f32,
    rects: &ScrollbarRects,
    metrics: &ScrollbarMetrics,
) -> f32 {
    let (track_length, thumb_length) = match metrics.axis {
        ScrollbarAxis::Vertical => (rects.track.height(), rects.thumb.height()),
        ScrollbarAxis::Horizontal => (rects.track.width(), rects.thumb.width()),
    };
    let max = (metrics.content_extent - metrics.view_extent).max(0.0);
    let usable = track_length - thumb_length;
    if usable > 0.0 {
        delta_pixels * max / usable
    } else {
        0.0
    }
}

// Private helpers

/// Compute track visible and hit rects.
fn track_rects(
    viewport: Rect,
    axis: ScrollbarAxis,
    style: &ScrollbarStyle,
    reserve_far_edge: f32,
) -> (Rect, Rect) {
    match axis {
        ScrollbarAxis::Vertical => {
            let x = viewport.right() - style.thickness - style.edge_inset;
            let hit_x = x - style.hit_slop;
            let h = viewport.height() - reserve_far_edge;
            let visible = Rect::new(x, viewport.y(), style.thickness, h);
            let hit = Rect::new(hit_x, viewport.y(), style.thickness + style.hit_slop, h);
            (visible, hit)
        }
        ScrollbarAxis::Horizontal => {
            let y = viewport.bottom() - style.thickness - style.edge_inset;
            let hit_y = y - style.hit_slop;
            let w = viewport.width() - reserve_far_edge;
            let visible = Rect::new(viewport.x(), y, w, style.thickness);
            let hit = Rect::new(viewport.x(), hit_y, w, style.thickness + style.hit_slop);
            (visible, hit)
        }
    }
}

#[cfg(test)]
mod tests;
