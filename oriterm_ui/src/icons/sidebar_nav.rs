//! Settings sidebar icon definitions derived from mockup SVGs.
//!
//! Generated from `mockups/settings-brutal.html` via the `svg_import` module.
//! Each icon uses a 24×24 viewBox normalized to 0.0–1.0 coordinates. Circles,
//! rounded rects, and SVG arcs are lowered to cubic Bézier segments.

// Generated coordinate data — separators in 6-decimal literals reduce readability.
#![expect(clippy::unreadable_literal, reason = "generated icon coordinates")]

use super::{IconPath, IconStyle, PathCommand};

/// Stroke width for settings nav icons (logical pixels).
///
/// The mockup SVG spec is `stroke-width="2"` in a 24×24 viewBox at 16px,
/// giving a nominal `2.0 × (16.0 / 24.0) ≈ 1.333`. However, the mockup
/// renders with SVG-default butt linecaps while the rasterizer uses round
/// linecaps (which add semicircles at every endpoint, increasing visual
/// weight). A stroke of `1.0` with round caps matches the mockup's visual
/// weight at the target size.
pub(super) const NAV_STROKE: f32 = 1.0;

/// Sun icon — Appearance settings page.
///
/// Circle with 8 rays (4 cardinal + 4 diagonal).
pub static ICON_SUN: IconPath = IconPath {
    commands: &[
        // Center circle (cubic Bézier approximation).
        PathCommand::MoveTo(0.500000, 0.291667),
        PathCommand::CubicTo(0.615059, 0.291667, 0.708333, 0.384941, 0.708333, 0.500000),
        PathCommand::CubicTo(0.708333, 0.615059, 0.615059, 0.708333, 0.500000, 0.708333),
        PathCommand::CubicTo(0.384941, 0.708333, 0.291667, 0.615059, 0.291667, 0.500000),
        PathCommand::CubicTo(0.291667, 0.384941, 0.384941, 0.291667, 0.500000, 0.291667),
        PathCommand::Close,
        // Top ray.
        PathCommand::MoveTo(0.500000, 0.041667),
        PathCommand::LineTo(0.500000, 0.125000),
        // Bottom ray.
        PathCommand::MoveTo(0.500000, 0.875000),
        PathCommand::LineTo(0.500000, 0.958333),
        // Top-left diagonal.
        PathCommand::MoveTo(0.175833, 0.175833),
        PathCommand::LineTo(0.235000, 0.235000),
        // Bottom-right diagonal.
        PathCommand::MoveTo(0.765000, 0.765000),
        PathCommand::LineTo(0.824167, 0.824167),
        // Left ray.
        PathCommand::MoveTo(0.041667, 0.500000),
        PathCommand::LineTo(0.125000, 0.500000),
        // Right ray.
        PathCommand::MoveTo(0.875000, 0.500000),
        PathCommand::LineTo(0.958333, 0.500000),
        // Bottom-left diagonal.
        PathCommand::MoveTo(0.175833, 0.824167),
        PathCommand::LineTo(0.235000, 0.765000),
        // Top-right diagonal.
        PathCommand::MoveTo(0.765000, 0.235000),
        PathCommand::LineTo(0.824167, 0.175833),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Palette icon — Colors settings page.
///
/// Asymmetric palette silhouette with 4 color dots.
pub static ICON_PALETTE: IconPath = IconPath {
    commands: &[
        // Palette body.
        PathCommand::MoveTo(0.500000, 0.083333),
        PathCommand::CubicTo(0.270000, 0.083333, 0.083333, 0.270000, 0.083333, 0.500000),
        PathCommand::CubicTo(0.083333, 0.730000, 0.270000, 0.916667, 0.500000, 0.916667),
        PathCommand::CubicTo(0.545833, 0.916667, 0.583333, 0.879167, 0.583333, 0.833333),
        PathCommand::CubicTo(0.583333, 0.812500, 0.575000, 0.791667, 0.562500, 0.779167),
        PathCommand::CubicTo(0.550000, 0.762500, 0.541667, 0.745833, 0.541667, 0.725000),
        PathCommand::CubicTo(0.541667, 0.679167, 0.579167, 0.641667, 0.625000, 0.641667),
        PathCommand::LineTo(0.725000, 0.641667),
        PathCommand::CubicTo(0.854167, 0.641667, 0.958333, 0.537500, 0.958333, 0.408333),
        PathCommand::CubicTo(0.958333, 0.241667, 0.754167, 0.083333, 0.500000, 0.083333),
        PathCommand::Close,
        // Left dot (r=1.5 at 6.5, 11.5).
        PathCommand::MoveTo(0.270833, 0.416667),
        PathCommand::CubicTo(0.305351, 0.416667, 0.333333, 0.444649, 0.333333, 0.479167),
        PathCommand::CubicTo(0.333333, 0.513684, 0.305351, 0.541667, 0.270833, 0.541667),
        PathCommand::CubicTo(0.236316, 0.541667, 0.208333, 0.513684, 0.208333, 0.479167),
        PathCommand::CubicTo(0.208333, 0.444649, 0.236316, 0.416667, 0.270833, 0.416667),
        PathCommand::Close,
        // Top-left dot (r=1.5 at 9.5, 7.5).
        PathCommand::MoveTo(0.395833, 0.250000),
        PathCommand::CubicTo(0.430351, 0.250000, 0.458333, 0.277982, 0.458333, 0.312500),
        PathCommand::CubicTo(0.458333, 0.347018, 0.430351, 0.375000, 0.395833, 0.375000),
        PathCommand::CubicTo(0.361316, 0.375000, 0.333333, 0.347018, 0.333333, 0.312500),
        PathCommand::CubicTo(0.333333, 0.277982, 0.361316, 0.250000, 0.395833, 0.250000),
        PathCommand::Close,
        // Top-right dot (r=1.5 at 14.5, 7.5).
        PathCommand::MoveTo(0.604167, 0.250000),
        PathCommand::CubicTo(0.638684, 0.250000, 0.666667, 0.277982, 0.666667, 0.312500),
        PathCommand::CubicTo(0.666667, 0.347018, 0.638684, 0.375000, 0.604167, 0.375000),
        PathCommand::CubicTo(0.569649, 0.375000, 0.541667, 0.347018, 0.541667, 0.312500),
        PathCommand::CubicTo(0.541667, 0.277982, 0.569649, 0.250000, 0.604167, 0.250000),
        PathCommand::Close,
        // Right dot (r=1.5 at 17.5, 11.5).
        PathCommand::MoveTo(0.729167, 0.416667),
        PathCommand::CubicTo(0.763684, 0.416667, 0.791667, 0.444649, 0.791667, 0.479167),
        PathCommand::CubicTo(0.791667, 0.513684, 0.763684, 0.541667, 0.729167, 0.541667),
        PathCommand::CubicTo(0.694649, 0.541667, 0.666667, 0.513684, 0.666667, 0.479167),
        PathCommand::CubicTo(0.666667, 0.444649, 0.694649, 0.416667, 0.729167, 0.416667),
        PathCommand::Close,
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Type/font icon — Font settings page.
///
/// Serif T letterform with crossbar and bottom serif.
pub static ICON_TYPE: IconPath = IconPath {
    commands: &[
        // Top crossbar with vertical returns.
        PathCommand::MoveTo(0.166667, 0.291667),
        PathCommand::LineTo(0.166667, 0.166667),
        PathCommand::LineTo(0.833333, 0.166667),
        PathCommand::LineTo(0.833333, 0.291667),
        // Bottom serif.
        PathCommand::MoveTo(0.375000, 0.833333),
        PathCommand::LineTo(0.625000, 0.833333),
        // Vertical stem.
        PathCommand::MoveTo(0.500000, 0.166667),
        PathCommand::LineTo(0.500000, 0.833333),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Terminal prompt icon — Terminal settings page.
///
/// Chevron prompt (>) with underline cursor.
pub static ICON_TERMINAL: IconPath = IconPath {
    commands: &[
        // Prompt chevron.
        PathCommand::MoveTo(0.166667, 0.708333),
        PathCommand::LineTo(0.416667, 0.458333),
        PathCommand::LineTo(0.166667, 0.208333),
        // Input line.
        PathCommand::MoveTo(0.500000, 0.791667),
        PathCommand::LineTo(0.833333, 0.791667),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Keyboard icon — Keybindings settings page.
///
/// Rounded-rect frame with key dots and spacebar.
pub static ICON_KEYBOARD: IconPath = IconPath {
    commands: &[
        // Outer frame (rounded rect, rx=2).
        PathCommand::MoveTo(0.166667, 0.166667),
        PathCommand::LineTo(0.833333, 0.166667),
        PathCommand::CubicTo(0.879357, 0.166667, 0.916667, 0.203976, 0.916667, 0.250000),
        PathCommand::LineTo(0.916667, 0.750000),
        PathCommand::CubicTo(0.916667, 0.796024, 0.879357, 0.833333, 0.833333, 0.833333),
        PathCommand::LineTo(0.166667, 0.833333),
        PathCommand::CubicTo(0.120643, 0.833333, 0.083333, 0.796024, 0.083333, 0.750000),
        PathCommand::LineTo(0.083333, 0.250000),
        PathCommand::CubicTo(0.083333, 0.203976, 0.120643, 0.166667, 0.166667, 0.166667),
        PathCommand::Close,
        // Row 1 keys (4 dots).
        PathCommand::MoveTo(0.250000, 0.333333),
        PathCommand::LineTo(0.250417, 0.333333),
        PathCommand::MoveTo(0.416667, 0.333333),
        PathCommand::LineTo(0.417083, 0.333333),
        PathCommand::MoveTo(0.583333, 0.333333),
        PathCommand::LineTo(0.583750, 0.333333),
        PathCommand::MoveTo(0.750000, 0.333333),
        PathCommand::LineTo(0.750417, 0.333333),
        // Row 2 keys (3 dots).
        PathCommand::MoveTo(0.333333, 0.500000),
        PathCommand::LineTo(0.333750, 0.500000),
        PathCommand::MoveTo(0.500000, 0.500000),
        PathCommand::LineTo(0.500417, 0.500000),
        PathCommand::MoveTo(0.666667, 0.500000),
        PathCommand::LineTo(0.667083, 0.500000),
        // Space bar.
        PathCommand::MoveTo(0.333333, 0.666667),
        PathCommand::LineTo(0.666667, 0.666667),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Window frame icon — Window settings page.
///
/// Rounded-rect window frame with title bar divider.
pub static ICON_WINDOW: IconPath = IconPath {
    commands: &[
        // Outer frame (rounded rect, rx=2).
        PathCommand::MoveTo(0.208333, 0.125000),
        PathCommand::LineTo(0.791667, 0.125000),
        PathCommand::CubicTo(0.837690, 0.125000, 0.875000, 0.162310, 0.875000, 0.208333),
        PathCommand::LineTo(0.875000, 0.791667),
        PathCommand::CubicTo(0.875000, 0.837690, 0.837690, 0.875000, 0.791667, 0.875000),
        PathCommand::LineTo(0.208333, 0.875000),
        PathCommand::CubicTo(0.162310, 0.875000, 0.125000, 0.837690, 0.125000, 0.791667),
        PathCommand::LineTo(0.125000, 0.208333),
        PathCommand::CubicTo(0.125000, 0.162310, 0.162310, 0.125000, 0.208333, 0.125000),
        PathCommand::Close,
        // Title bar divider.
        PathCommand::MoveTo(0.125000, 0.375000),
        PathCommand::LineTo(0.875000, 0.375000),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Bell icon — Bell settings page.
///
/// Bell body with arc top, brim, and clapper.
pub static ICON_BELL: IconPath = IconPath {
    commands: &[
        // Bell body (arcs converted to cubics).
        PathCommand::MoveTo(0.750000, 0.333333),
        PathCommand::CubicTo(0.750000, 0.195262, 0.638071, 0.083333, 0.500000, 0.083333),
        PathCommand::CubicTo(0.361929, 0.083333, 0.250000, 0.195262, 0.250000, 0.333333),
        PathCommand::CubicTo(0.250000, 0.625000, 0.125000, 0.708333, 0.125000, 0.708333),
        // Brim.
        PathCommand::LineTo(0.875000, 0.708333),
        PathCommand::CubicTo(0.875000, 0.708333, 0.750000, 0.625000, 0.750000, 0.333333),
        // Clapper.
        PathCommand::MoveTo(0.572083, 0.875000),
        PathCommand::CubicTo(0.557175, 0.900700, 0.529711, 0.916519, 0.500000, 0.916519),
        PathCommand::CubicTo(0.470289, 0.916519, 0.442825, 0.900700, 0.427917, 0.875000),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Activity/pulse icon — Rendering settings page.
///
/// Heartbeat-style waveform line.
pub static ICON_ACTIVITY: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.916667, 0.500000),
        PathCommand::LineTo(0.750000, 0.500000),
        PathCommand::LineTo(0.625000, 0.875000),
        PathCommand::LineTo(0.375000, 0.125000),
        PathCommand::LineTo(0.250000, 0.500000),
        PathCommand::LineTo(0.083333, 0.500000),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
