//! Footer icon definitions (unsaved indicator).
//!
//! Alert-circle icon from Feather Icons, normalized from a 24×24 viewBox
//! to 0.0–1.0 coordinates. Circle approximated with 4 cubic Bézier segments.

// Generated coordinate data — separators in 6-decimal literals reduce readability.
#![expect(clippy::unreadable_literal, reason = "generated icon coordinates")]

use super::{IconPath, IconStyle, PathCommand};

/// Stroke width matching other Feather-style icons.
const ALERT_STROKE: f32 = 1.0;

/// Bézier control-point offset for a quarter-circle: `radius * 0.5523`.
///
/// For `r = 10/24 ≈ 0.416667`, kappa = `0.416667 * 0.5523 ≈ 0.230125`.
const K: f32 = 0.230125;

/// Center and radius of the alert circle in normalized coords.
const CX: f32 = 0.500000;
const CY: f32 = 0.500000;
const R: f32 = 0.416667;

/// Alert circle — circle with vertical bar and dot (Feather `alert-circle`).
///
/// SVG source (24×24 viewBox):
/// ```svg
/// <circle cx="12" cy="12" r="10"/>
/// <path d="M12 8v4M12 16h.01"/>
/// ```
pub static ICON_ALERT_CIRCLE: IconPath = IconPath {
    commands: &[
        // Circle: 4 cubic Bézier quarter-arcs (clockwise from top).
        PathCommand::MoveTo(CX, CY - R),
        // Top → Right.
        PathCommand::CubicTo(CX + K, CY - R, CX + R, CY - K, CX + R, CY),
        // Right → Bottom.
        PathCommand::CubicTo(CX + R, CY + K, CX + K, CY + R, CX, CY + R),
        // Bottom → Left.
        PathCommand::CubicTo(CX - K, CY + R, CX - R, CY + K, CX - R, CY),
        // Left → Top.
        PathCommand::CubicTo(CX - R, CY - K, CX - K, CY - R, CX, CY - R),
        PathCommand::Close,
        // Vertical bar: M12 8 v4 → normalized y: 8/24..12/24.
        PathCommand::MoveTo(0.500000, 0.333333),
        PathCommand::LineTo(0.500000, 0.500000),
        // Dot: M12 16 h.01 → normalized (0.5, 0.6667) to (0.5004, 0.6667).
        PathCommand::MoveTo(0.500000, 0.666667),
        PathCommand::LineTo(0.500417, 0.666667),
    ],
    style: IconStyle::Stroke(ALERT_STROKE),
};
