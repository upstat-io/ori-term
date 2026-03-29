//! Window chrome and tab bar icon definitions.
//!
//! Close, plus, chevron, minimize, maximize, restore, and window close
//! icons for tab bar buttons and title bar controls.

use super::{IconPath, IconStyle, PathCommand};

/// Stroke width for tab bar icons (logical pixels).
const TAB_STROKE: f32 = 1.0;

/// Stroke width for window chrome icons (logical pixels).
const CHROME_STROKE: f32 = 1.0;

/// Tab close button: two diagonal lines forming ×.
///
/// Fills most of the bitmap (0.1–0.9) with a small margin for stroke caps.
/// The widget's `CLOSE_ICON_INSET` handles positioning within the button.
pub static ICON_CLOSE: IconPath = IconPath {
    commands: &[
        // Top-left to bottom-right diagonal.
        PathCommand::MoveTo(0.1, 0.1),
        PathCommand::LineTo(0.9, 0.9),
        // Top-right to bottom-left diagonal.
        PathCommand::MoveTo(0.9, 0.1),
        PathCommand::LineTo(0.1, 0.9),
    ],
    style: IconStyle::Stroke(TAB_STROKE),
};

/// New tab button: horizontal + vertical lines forming +.
///
/// Fills most of the bitmap (0.1–0.9) with a small margin for stroke caps.
/// The widget centers the icon rect in the button.
pub static ICON_PLUS: IconPath = IconPath {
    commands: &[
        // Horizontal arm.
        PathCommand::MoveTo(0.1, 0.5),
        PathCommand::LineTo(0.9, 0.5),
        // Vertical arm.
        PathCommand::MoveTo(0.5, 0.1),
        PathCommand::LineTo(0.5, 0.9),
    ],
    style: IconStyle::Stroke(TAB_STROKE),
};

/// Dropdown chevron: two lines forming a downward-pointing V (▾).
///
/// Fills the bitmap width (0.1–0.9) with proportional vertical extent.
/// The widget centers the icon rect in the dropdown button.
pub static ICON_CHEVRON_DOWN: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.15, 0.35),
        PathCommand::LineTo(0.5, 0.75),
        PathCommand::LineTo(0.85, 0.35),
    ],
    style: IconStyle::Stroke(TAB_STROKE),
};

/// Window minimize: single horizontal dash centered vertically.
///
/// Derived from: `SYMBOL_SIZE = 10.0` on `CONTROL_BUTTON_WIDTH`.
/// Half = 5/10 = 0.5 of symbol region → stroke from 0.0 to 1.0 at y=0.5.
pub static ICON_MINIMIZE: IconPath = IconPath {
    commands: &[PathCommand::MoveTo(0.0, 0.5), PathCommand::LineTo(1.0, 0.5)],
    style: IconStyle::Stroke(CHROME_STROKE),
};

/// Window maximize: square outline.
pub static ICON_MAXIMIZE: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.0, 0.0),
        PathCommand::LineTo(1.0, 0.0),
        PathCommand::LineTo(1.0, 1.0),
        PathCommand::LineTo(0.0, 1.0),
        PathCommand::Close,
    ],
    style: IconStyle::Stroke(CHROME_STROKE),
};

/// Window restore: two overlapping square outlines.
///
/// Back window offset up-right by 2/10 = 0.2 of symbol size.
/// Front window at origin, slightly smaller (8/10 = 0.8).
pub static ICON_RESTORE: IconPath = IconPath {
    commands: &[
        // Back window (offset up-right).
        PathCommand::MoveTo(0.2, 0.0),
        PathCommand::LineTo(1.0, 0.0),
        PathCommand::LineTo(1.0, 0.8),
        PathCommand::LineTo(0.8, 0.8),
        // Front window (offset down-left).
        PathCommand::MoveTo(0.0, 0.2),
        PathCommand::LineTo(0.8, 0.2),
        PathCommand::LineTo(0.8, 1.0),
        PathCommand::LineTo(0.0, 1.0),
        PathCommand::Close,
    ],
    style: IconStyle::Stroke(CHROME_STROKE),
};

/// Filled downward triangle for dropdown select triggers.
///
/// Mockup SVG path: `M0 0l5 6 5-6z` in a 10×6 viewbox.
/// Centered vertically in a 10×10 square: top at y=0.2, bottom at y=0.8.
pub static ICON_DROPDOWN_ARROW: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.0, 0.2),
        PathCommand::LineTo(0.5, 0.8),
        PathCommand::LineTo(1.0, 0.2),
        PathCommand::Close,
    ],
    style: IconStyle::Fill,
};

/// Filled upward triangle for number input stepper (up arrow).
///
/// Mirror of `ICON_STEPPER_DOWN`: point at top-center, base at bottom.
/// Centered in a 10×10 square: bottom at y=0.8, top at y=0.2.
pub static ICON_STEPPER_UP: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.5, 0.2),
        PathCommand::LineTo(0.0, 0.8),
        PathCommand::LineTo(1.0, 0.8),
        PathCommand::Close,
    ],
    style: IconStyle::Fill,
};

/// Filled downward triangle for number input stepper (down arrow).
///
/// Same geometry as `ICON_DROPDOWN_ARROW` — point at bottom-center, base at top.
/// Centered in a 10×10 square: top at y=0.2, bottom at y=0.8.
pub static ICON_STEPPER_DOWN: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.0, 0.2),
        PathCommand::LineTo(0.5, 0.8),
        PathCommand::LineTo(1.0, 0.2),
        PathCommand::Close,
    ],
    style: IconStyle::Fill,
};

/// Window close button: × with full-extent diagonals (corner to corner).
///
/// Slightly different proportions than tab close — fills the entire
/// symbol region for a bolder appearance on the title bar.
pub static ICON_WINDOW_CLOSE: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.0, 0.0),
        PathCommand::LineTo(1.0, 1.0),
        PathCommand::MoveTo(1.0, 0.0),
        PathCommand::LineTo(0.0, 1.0),
    ],
    style: IconStyle::Stroke(CHROME_STROKE),
};
