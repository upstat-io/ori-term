//! Converts a [`DrawList`] into GPU instance buffer records.
//!
//! This module bridges `oriterm_ui`'s GPU-agnostic [`DrawCommand`]s and
//! oriterm's [`InstanceWriter`]. Each command becomes one or more
//! [`push_ui_rect`](super::instance_writer::InstanceWriter::push_ui_rect)
//! calls. Clip and image commands are deferred (logged as no-ops).

use oriterm_ui::color::Color;
use oriterm_ui::draw::{DrawCommand, DrawList, RectStyle};
use oriterm_ui::geometry::{Point, Rect};

use super::instance_writer::{InstanceWriter, ScreenRect};

/// Convert all commands in a [`DrawList`] to UI rect instances.
///
/// Shadow commands emit an expanded shadow rect before the main rect.
/// Line commands are converted to thin rectangles.
/// Image and clip commands are logged as no-ops.
#[allow(
    dead_code,
    reason = "public API for Section 07 — not yet wired into render loop"
)]
pub fn convert_draw_list(draw_list: &DrawList, writer: &mut InstanceWriter) {
    for cmd in draw_list.commands() {
        match cmd {
            DrawCommand::Rect { rect, style } => convert_rect(*rect, style, writer),
            DrawCommand::Line {
                from,
                to,
                width,
                color,
            } => {
                convert_line(*from, *to, *width, *color, writer);
            }
            DrawCommand::Image { .. } => {
                log::trace!("DrawCommand::Image deferred — not yet implemented");
            }
            DrawCommand::PushClip { .. } => {
                log::trace!("DrawCommand::PushClip deferred — not yet implemented");
            }
            DrawCommand::PopClip => {
                log::trace!("DrawCommand::PopClip deferred — not yet implemented");
            }
        }
    }
}

/// Convert a styled rect command to one or two UI rect instances.
fn convert_rect(rect: Rect, style: &RectStyle, writer: &mut InstanceWriter) {
    // Resolve fill color: prefer gradient first stop, then solid fill.
    let fill = style
        .gradient
        .as_ref()
        .and_then(|g| g.stops.first().map(|s| s.color))
        .or(style.fill)
        .unwrap_or(Color::TRANSPARENT);

    // Shadow instance (if present): expanded rect behind the main rect.
    if let Some(shadow) = &style.shadow {
        let expand = shadow.spread + shadow.blur_radius;
        let shadow_rect = ScreenRect {
            x: rect.x() + shadow.offset_x - expand,
            y: rect.y() + shadow.offset_y - expand,
            w: rect.width() + expand * 2.0,
            h: rect.height() + expand * 2.0,
        };
        writer.push_ui_rect(
            shadow_rect,
            shadow.color.to_array(),
            [0.0; 4],
            uniform_radius(&style.corner_radius) + expand,
            0.0,
        );
    }

    // Main rect instance.
    let screen = to_screen_rect(rect);
    let (border_color, border_width) = style
        .border
        .map_or(([0.0; 4], 0.0), |b| (b.color.to_array(), b.width));

    writer.push_ui_rect(
        screen,
        fill.to_array(),
        border_color,
        uniform_radius(&style.corner_radius),
        border_width,
    );
}

/// Convert a line segment to a thin axis-aligned rect instance.
fn convert_line(from: Point, to: Point, width: f32, color: Color, writer: &mut InstanceWriter) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len < f32::EPSILON {
        return;
    }

    // Perpendicular offset for line thickness.
    let nx = -dy / len * width * 0.5;
    let ny = dx / len * width * 0.5;

    // Axis-aligned bounding box of the thick line.
    let corners_x = [from.x + nx, from.x - nx, to.x + nx, to.x - nx];
    let corners_y = [from.y + ny, from.y - ny, to.y + ny, to.y - ny];

    let min_x = corners_x[0]
        .min(corners_x[1])
        .min(corners_x[2])
        .min(corners_x[3]);
    let min_y = corners_y[0]
        .min(corners_y[1])
        .min(corners_y[2])
        .min(corners_y[3]);
    let max_x = corners_x[0]
        .max(corners_x[1])
        .max(corners_x[2])
        .max(corners_x[3]);
    let max_y = corners_y[0]
        .max(corners_y[1])
        .max(corners_y[2])
        .max(corners_y[3]);

    let screen = ScreenRect {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    };

    writer.push_ui_rect(screen, color.to_array(), [0.0; 4], 0.0, 0.0);
}

/// Convert a geometry [`Rect`] to a [`ScreenRect`] for the instance writer.
fn to_screen_rect(rect: Rect) -> ScreenRect {
    ScreenRect {
        x: rect.x(),
        y: rect.y(),
        w: rect.width(),
        h: rect.height(),
    }
}

/// Pick a uniform radius from the per-corner array.
///
/// The SDF shader currently supports a single radius value. When per-corner
/// radii differ, use the maximum (visually reasonable until a 4-corner SDF
/// is implemented).
fn uniform_radius(radii: &[f32; 4]) -> f32 {
    radii[0].max(radii[1]).max(radii[2]).max(radii[3])
}

#[cfg(test)]
mod tests;
