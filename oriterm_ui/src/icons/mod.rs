//! Vector icon path definitions and resolution for anti-aliased icon rendering.
//!
//! All icons are defined as normalized path commands (coordinates 0.0–1.0)
//! and rasterized at the target pixel size via `tiny_skia` in the GPU layer.
//! Color is applied at draw time by the shader, not baked into the bitmap.
//!
//! Icon definitions are split by consumer:
//! - [`chrome`] — tab bar and window title bar icons
//! - [`sidebar_nav`] — settings sidebar icons (generated from mockup SVGs)

mod chrome;
mod sidebar_nav;

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
    /// All icon variants, in definition order.
    ///
    /// Used by tests to verify every variant is covered by the icon
    /// pre-resolution list and rasterizer.
    pub const ALL: &[Self] = &[
        Self::Close,
        Self::Plus,
        Self::ChevronDown,
        Self::Minimize,
        Self::Maximize,
        Self::Restore,
        Self::WindowClose,
        Self::Sun,
        Self::Palette,
        Self::Type,
        Self::Terminal,
        Self::Keyboard,
        Self::Window,
        Self::Bell,
        Self::Activity,
    ];

    /// Returns the icon path definition for this icon.
    pub fn path(self) -> &'static IconPath {
        match self {
            // Chrome icons (tab bar + window controls).
            Self::Close => &chrome::ICON_CLOSE,
            Self::Plus => &chrome::ICON_PLUS,
            Self::ChevronDown => &chrome::ICON_CHEVRON_DOWN,
            Self::Minimize => &chrome::ICON_MINIMIZE,
            Self::Maximize => &chrome::ICON_MAXIMIZE,
            Self::Restore => &chrome::ICON_RESTORE,
            Self::WindowClose => &chrome::ICON_WINDOW_CLOSE,
            // Sidebar nav icons (generated from mockup SVGs).
            Self::Sun => &sidebar_nav::ICON_SUN,
            Self::Palette => &sidebar_nav::ICON_PALETTE,
            Self::Type => &sidebar_nav::ICON_TYPE,
            Self::Terminal => &sidebar_nav::ICON_TERMINAL,
            Self::Keyboard => &sidebar_nav::ICON_KEYBOARD,
            Self::Window => &sidebar_nav::ICON_WINDOW,
            Self::Bell => &sidebar_nav::ICON_BELL,
            Self::Activity => &sidebar_nav::ICON_ACTIVITY,
        }
    }
}

/// Logical pixel size for settings sidebar nav icons.
///
/// Used by both the sidebar widget and the icon pre-resolution list.
pub const SIDEBAR_NAV_ICON_SIZE: u32 = 16;

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

/// Source SVG fixtures for the 8 settings sidebar icons. Used for fidelity
/// verification against the runtime [`IconPath`] definitions.
pub mod sidebar_fixtures;

/// SVG-to-[`PathCommand`] importer. Converts SVG elements into normalized
/// path commands for icon definitions and fidelity tests.
pub mod svg_import;

#[cfg(test)]
mod tests;
