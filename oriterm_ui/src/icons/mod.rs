//! Vector icon path definitions and resolution for anti-aliased icon rendering.
//!
//! All icons are defined as normalized path commands (coordinates 0.0–1.0)
//! and rasterized at the target pixel size via `tiny_skia` in the GPU layer.
//! Color is applied at draw time by the shader, not baked into the bitmap.
//!
//! The [`IconResolver`] trait bridges the library and binary crates: widgets
//! call `resolve()` at draw time to get atlas coordinates, and the binary
//! crate provides the concrete implementation backed by `IconCache`.

/// A single path drawing command in normalized 0.0–1.0 coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Move the pen to (x, y) without drawing.
    MoveTo(f32, f32),
    /// Draw a straight line from the current position to (x, y).
    LineTo(f32, f32),
    /// Draw a cubic Bézier curve to (x, y) with control points (cx1, cy1) and (cx2, cy2).
    CubicTo(f32, f32, f32, f32, f32, f32),
    /// Close the current sub-path back to the last `MoveTo`.
    Close,
}

/// How an icon path should be rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconStyle {
    /// Stroke the path with a fixed line width in **logical pixels**.
    ///
    /// The rasterizer multiplies by the display scale factor to get the
    /// physical pixel width — e.g. 1.5 logical → 2.25px at 1.5× scale.
    /// This keeps stroke weight visually consistent across DPI settings,
    /// matching the behavior of the old `push_line()` code path.
    Stroke(f32),
    /// Fill the interior of the path.
    Fill,
}

/// A complete icon defined as a sequence of path commands and a rendering style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconPath {
    /// Drawing commands in normalized 0.0–1.0 coordinate space.
    pub commands: &'static [PathCommand],
    /// Whether to stroke or fill the path.
    pub style: IconStyle,
}

/// Type-safe icon identifier for cache lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    /// Tab close button (×).
    Close,
    /// New tab button (+).
    Plus,
    /// Dropdown indicator (▾).
    ChevronDown,
    /// Window minimize (━).
    Minimize,
    /// Window maximize (□).
    Maximize,
    /// Window restore (⧉ — two overlapping squares).
    Restore,
    /// Window close button (×, slightly larger proportions than tab close).
    WindowClose,
    /// Sun icon — Appearance settings page.
    Sun,
    /// Palette icon — Colors settings page.
    Palette,
    /// Type/font icon — Font settings page.
    Type,
    /// Terminal prompt icon — Terminal settings page.
    Terminal,
    /// Keyboard icon — Keybindings settings page.
    Keyboard,
    /// Window frame icon — Window settings page.
    Window,
    /// Bell icon — Bell settings page.
    Bell,
    /// Activity/pulse icon — Rendering settings page.
    Activity,
}

impl IconId {
    /// Returns the icon path definition for this icon.
    pub fn path(self) -> &'static IconPath {
        match self {
            Self::Close => &ICON_CLOSE,
            Self::Plus => &ICON_PLUS,
            Self::ChevronDown => &ICON_CHEVRON_DOWN,
            Self::Minimize => &ICON_MINIMIZE,
            Self::Maximize => &ICON_MAXIMIZE,
            Self::Restore => &ICON_RESTORE,
            Self::WindowClose => &ICON_WINDOW_CLOSE,
            Self::Sun => &ICON_SUN,
            Self::Palette => &ICON_PALETTE,
            Self::Type => &ICON_TYPE,
            Self::Terminal => &ICON_TERMINAL,
            Self::Keyboard => &ICON_KEYBOARD,
            Self::Window => &ICON_WINDOW,
            Self::Bell => &ICON_BELL,
            Self::Activity => &ICON_ACTIVITY,
        }
    }
}

/// Atlas coordinates for a resolved icon bitmap.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedIcon {
    /// Atlas page (texture array layer) containing the icon bitmap.
    pub atlas_page: u32,
    /// Normalized UV coordinates `[u_left, v_top, u_width, v_height]`.
    pub uv: [f32; 4],
}

/// Pre-resolved icon atlas entries for one frame.
///
/// Built by the GPU renderer before the draw phase. Each icon is keyed by
/// `(IconId, size_px)`. Widgets look up their icon from this map during
/// `draw()` — no mutation or trait objects needed.
#[derive(Debug, Default)]
pub struct ResolvedIcons {
    entries: Vec<((IconId, u32), ResolvedIcon)>,
}

impl ResolvedIcons {
    /// Create an empty map.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert a resolved icon entry.
    pub fn insert(&mut self, id: IconId, size_px: u32, icon: ResolvedIcon) {
        // Small list (≤14 entries), linear scan is fine.
        let key = (id, size_px);
        for entry in &mut self.entries {
            if entry.0 == key {
                entry.1 = icon;
                return;
            }
        }
        self.entries.push((key, icon));
    }

    /// Look up a resolved icon by ID and pixel size.
    pub fn get(&self, id: IconId, size_px: u32) -> Option<ResolvedIcon> {
        let key = (id, size_px);
        self.entries.iter().find(|e| e.0 == key).map(|e| e.1)
    }

    /// Remove all entries. Call at the start of each frame.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Stroke width for tab bar icons (logical pixels).
const TAB_STROKE: f32 = 1.0;

/// Stroke width for window chrome icons (logical pixels).
const CHROME_STROKE: f32 = 1.0;

/// Tab close button: two diagonal lines forming ×.
///
/// Fills most of the bitmap (0.1–0.9) with a small margin for stroke caps.
/// The widget's `CLOSE_ICON_INSET` handles positioning within the button.
static ICON_CLOSE: IconPath = IconPath {
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
static ICON_PLUS: IconPath = IconPath {
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
static ICON_CHEVRON_DOWN: IconPath = IconPath {
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
static ICON_MINIMIZE: IconPath = IconPath {
    commands: &[PathCommand::MoveTo(0.0, 0.5), PathCommand::LineTo(1.0, 0.5)],
    style: IconStyle::Stroke(CHROME_STROKE),
};

/// Window maximize: square outline.
static ICON_MAXIMIZE: IconPath = IconPath {
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
static ICON_RESTORE: IconPath = IconPath {
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

/// Window close button: × with full-extent diagonals (corner to corner).
///
/// Slightly different proportions than tab close — fills the entire
/// symbol region for a bolder appearance on the title bar.
static ICON_WINDOW_CLOSE: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.0, 0.0),
        PathCommand::LineTo(1.0, 1.0),
        PathCommand::MoveTo(1.0, 0.0),
        PathCommand::LineTo(0.0, 1.0),
    ],
    style: IconStyle::Stroke(CHROME_STROKE),
};

/// Stroke width for settings nav icons (logical pixels).
const NAV_STROKE: f32 = 1.0;

/// Sun icon: circle with 4 rays at cardinal + 4 at diagonal positions.
///
/// Circle at center (radius ~0.2 of unit), rays extend outward.
/// Simplified from the mockup's 8-ray SVG sun.
static ICON_SUN: IconPath = IconPath {
    commands: &[
        // Top ray.
        PathCommand::MoveTo(0.5, 0.04),
        PathCommand::LineTo(0.5, 0.21),
        // Bottom ray.
        PathCommand::MoveTo(0.5, 0.79),
        PathCommand::LineTo(0.5, 0.96),
        // Left ray.
        PathCommand::MoveTo(0.04, 0.5),
        PathCommand::LineTo(0.21, 0.5),
        // Right ray.
        PathCommand::MoveTo(0.79, 0.5),
        PathCommand::LineTo(0.96, 0.5),
        // Top-left diagonal.
        PathCommand::MoveTo(0.17, 0.17),
        PathCommand::LineTo(0.29, 0.29),
        // Top-right diagonal.
        PathCommand::MoveTo(0.83, 0.17),
        PathCommand::LineTo(0.71, 0.29),
        // Bottom-left diagonal.
        PathCommand::MoveTo(0.17, 0.83),
        PathCommand::LineTo(0.29, 0.71),
        // Bottom-right diagonal.
        PathCommand::MoveTo(0.83, 0.83),
        PathCommand::LineTo(0.71, 0.71),
        // Center circle (approximated as octagon for stroke rendering).
        PathCommand::MoveTo(0.5, 0.29),
        PathCommand::LineTo(0.65, 0.35),
        PathCommand::LineTo(0.71, 0.5),
        PathCommand::LineTo(0.65, 0.65),
        PathCommand::LineTo(0.5, 0.71),
        PathCommand::LineTo(0.35, 0.65),
        PathCommand::LineTo(0.29, 0.5),
        PathCommand::LineTo(0.35, 0.35),
        PathCommand::Close,
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Palette icon: circle with 4 color dots inside.
///
/// Outer circle with 4 dots at compass positions (simulating color spots).
static ICON_PALETTE: IconPath = IconPath {
    commands: &[
        // Outer circle (approximated as octagon).
        PathCommand::MoveTo(0.5, 0.08),
        PathCommand::LineTo(0.79, 0.21),
        PathCommand::LineTo(0.92, 0.5),
        PathCommand::LineTo(0.79, 0.79),
        PathCommand::LineTo(0.5, 0.92),
        PathCommand::LineTo(0.21, 0.79),
        PathCommand::LineTo(0.08, 0.5),
        PathCommand::LineTo(0.21, 0.21),
        PathCommand::Close,
        // Dot top-left.
        PathCommand::MoveTo(0.27, 0.46),
        PathCommand::LineTo(0.33, 0.46),
        // Dot top.
        PathCommand::MoveTo(0.40, 0.31),
        PathCommand::LineTo(0.46, 0.31),
        // Dot top-right.
        PathCommand::MoveTo(0.58, 0.31),
        PathCommand::LineTo(0.64, 0.31),
        // Dot right.
        PathCommand::MoveTo(0.71, 0.46),
        PathCommand::LineTo(0.77, 0.46),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Type/font icon: capital T with serifs.
///
/// Horizontal crossbar at top, vertical stem centered, serifs at bottom.
static ICON_TYPE: IconPath = IconPath {
    commands: &[
        // Top crossbar.
        PathCommand::MoveTo(0.17, 0.17),
        PathCommand::LineTo(0.83, 0.17),
        // Vertical stem.
        PathCommand::MoveTo(0.5, 0.17),
        PathCommand::LineTo(0.5, 0.83),
        // Bottom serif.
        PathCommand::MoveTo(0.33, 0.83),
        PathCommand::LineTo(0.67, 0.83),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Terminal prompt icon: > cursor and input line.
///
/// Chevron on left, horizontal line on right (classic terminal prompt).
static ICON_TERMINAL: IconPath = IconPath {
    commands: &[
        // Prompt chevron >.
        PathCommand::MoveTo(0.17, 0.21),
        PathCommand::LineTo(0.42, 0.46),
        PathCommand::LineTo(0.17, 0.71),
        // Input line.
        PathCommand::MoveTo(0.50, 0.79),
        PathCommand::LineTo(0.83, 0.79),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Keyboard icon: rounded rectangle with key indicators inside.
///
/// Outer frame with 3 rows of key marks.
static ICON_KEYBOARD: IconPath = IconPath {
    commands: &[
        // Outer frame.
        PathCommand::MoveTo(0.08, 0.17),
        PathCommand::LineTo(0.92, 0.17),
        PathCommand::LineTo(0.92, 0.83),
        PathCommand::LineTo(0.08, 0.83),
        PathCommand::Close,
        // Top row keys (3 dots).
        PathCommand::MoveTo(0.25, 0.33),
        PathCommand::LineTo(0.29, 0.33),
        PathCommand::MoveTo(0.50, 0.33),
        PathCommand::LineTo(0.54, 0.33),
        PathCommand::MoveTo(0.71, 0.33),
        PathCommand::LineTo(0.75, 0.33),
        // Middle row keys (2 dots).
        PathCommand::MoveTo(0.33, 0.50),
        PathCommand::LineTo(0.37, 0.50),
        PathCommand::MoveTo(0.63, 0.50),
        PathCommand::LineTo(0.67, 0.50),
        // Space bar.
        PathCommand::MoveTo(0.33, 0.67),
        PathCommand::LineTo(0.67, 0.67),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Window frame icon: square with title bar line.
///
/// Outer rectangle with a horizontal divider near top (title bar).
static ICON_WINDOW: IconPath = IconPath {
    commands: &[
        // Outer frame.
        PathCommand::MoveTo(0.12, 0.12),
        PathCommand::LineTo(0.88, 0.12),
        PathCommand::LineTo(0.88, 0.88),
        PathCommand::LineTo(0.12, 0.88),
        PathCommand::Close,
        // Title bar line.
        PathCommand::MoveTo(0.12, 0.33),
        PathCommand::LineTo(0.88, 0.33),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Bell icon: bell shape with clapper.
///
/// Bell body (trapezoidal top widening down) with a clapper dot at bottom.
static ICON_BELL: IconPath = IconPath {
    commands: &[
        // Bell top.
        PathCommand::MoveTo(0.50, 0.08),
        PathCommand::LineTo(0.50, 0.17),
        // Bell body (left side).
        PathCommand::MoveTo(0.25, 0.71),
        PathCommand::LineTo(0.25, 0.50),
        PathCommand::CubicTo(0.25, 0.29, 0.38, 0.17, 0.50, 0.17),
        // Bell body (right side).
        PathCommand::CubicTo(0.62, 0.17, 0.75, 0.29, 0.75, 0.50),
        PathCommand::LineTo(0.75, 0.71),
        // Bell brim.
        PathCommand::MoveTo(0.17, 0.71),
        PathCommand::LineTo(0.83, 0.71),
        // Clapper.
        PathCommand::MoveTo(0.42, 0.83),
        PathCommand::LineTo(0.58, 0.83),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

/// Activity/pulse icon: EKG-style heartbeat line.
///
/// Horizontal baseline with a sharp peak in the middle.
static ICON_ACTIVITY: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.04, 0.50),
        PathCommand::LineTo(0.25, 0.50),
        PathCommand::LineTo(0.38, 0.12),
        PathCommand::LineTo(0.50, 0.88),
        PathCommand::LineTo(0.62, 0.25),
        PathCommand::LineTo(0.75, 0.50),
        PathCommand::LineTo(0.96, 0.50),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};

#[cfg(test)]
mod tests;
