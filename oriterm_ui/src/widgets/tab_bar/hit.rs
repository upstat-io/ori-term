//! Tab bar hit-test result and geometry-based hit testing.
//!
//! [`TabBarHit`] identifies which element of the tab bar the cursor is over.
//! [`hit_test`] maps a logical-pixel position (relative to the tab bar origin)
//! to the highest-priority element at that location.

use super::constants::{
    CLOSE_BUTTON_RIGHT_PAD, CLOSE_BUTTON_WIDTH, DROPDOWN_BUTTON_WIDTH, NEW_TAB_BUTTON_WIDTH,
    TAB_BAR_HEIGHT,
};
use super::layout::TabBarLayout;

/// Which element of the tab bar the cursor is targeting.
///
/// Variants are ordered by priority: window controls > close button > tab >
/// new tab > dropdown > drag area. The hit-test function checks in this order
/// and returns the first match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TabBarHit {
    /// No element — cursor outside or below the tab bar.
    #[default]
    None,
    /// Tab body at the given index.
    Tab(usize),
    /// Close button on the tab at the given index.
    CloseTab(usize),
    /// New-tab (+) button.
    NewTab,
    /// Dropdown (settings/scheme) button.
    Dropdown,
    /// Window control: minimize.
    Minimize,
    /// Window control: maximize/restore.
    Maximize,
    /// Window control: close window.
    CloseWindow,
    /// Empty tab bar area (for window dragging or double-click maximize).
    DragArea,
}

impl TabBarHit {
    /// Whether the hit targets a specific tab (body or close button).
    pub fn is_tab(&self, index: usize) -> bool {
        matches!(self, Self::Tab(i) | Self::CloseTab(i) if *i == index)
    }

    /// Returns the tab index if the hit targets a tab (body or close button).
    pub fn tab_index(&self) -> Option<usize> {
        match self {
            Self::Tab(i) | Self::CloseTab(i) => Some(*i),
            _ => None,
        }
    }

    /// Whether the hit targets any window control button.
    pub fn is_window_control(&self) -> bool {
        matches!(self, Self::Minimize | Self::Maximize | Self::CloseWindow)
    }
}

/// Maps a logical-pixel position to the tab bar element at that location.
///
/// Coordinates are relative to the tab bar origin (0,0 = top-left of the
/// tab bar area). Priority order:
///
/// 1. Outside bounds → `None`
/// 2. Window controls zone → `Minimize` / `Maximize` / `CloseWindow`
/// 3. Tab strip (close button first) → `CloseTab(idx)` / `Tab(idx)`
/// 4. New-tab button → `NewTab`
/// 5. Dropdown button → `Dropdown`
/// 6. Remaining space → `DragArea`
pub fn hit_test(x: f32, y: f32, layout: &TabBarLayout) -> TabBarHit {
    // Outside tab bar bounds.
    if !(0.0..TAB_BAR_HEIGHT).contains(&y) {
        return TabBarHit::None;
    }

    // Window controls zone (rightmost).
    let controls_x = layout.controls_x();
    if x >= controls_x {
        return hit_test_controls(x - controls_x, y);
    }

    // Tab strip: close button checked first (higher priority than tab body).
    if let Some(idx) = layout.tab_index_at(x) {
        let tab_right = layout.tab_x(idx) + layout.tab_width_at(idx);
        let close_left = tab_right - CLOSE_BUTTON_WIDTH - CLOSE_BUTTON_RIGHT_PAD;
        let close_right = tab_right - CLOSE_BUTTON_RIGHT_PAD;
        if x >= close_left && x < close_right {
            return TabBarHit::CloseTab(idx);
        }
        return TabBarHit::Tab(idx);
    }

    // New-tab button (immediately after last tab).
    let new_tab_x = layout.new_tab_x();
    if x >= new_tab_x && x < new_tab_x + NEW_TAB_BUTTON_WIDTH {
        return TabBarHit::NewTab;
    }

    // Dropdown button (after new-tab).
    let dropdown_x = layout.dropdown_x();
    if x >= dropdown_x && x < dropdown_x + DROPDOWN_BUTTON_WIDTH {
        return TabBarHit::Dropdown;
    }

    // Empty space in the tab bar.
    TabBarHit::DragArea
}

/// Hit-test within the window controls zone (Windows: rectangular buttons).
///
/// `offset_x` is relative to the start of the controls zone. Buttons are
/// laid out left-to-right: Minimize, Maximize, Close.
#[cfg(target_os = "windows")]
fn hit_test_controls(offset_x: f32, _y: f32) -> TabBarHit {
    use crate::widgets::window_chrome::constants::CONTROL_BUTTON_WIDTH;

    match (offset_x / CONTROL_BUTTON_WIDTH) as usize {
        0 => TabBarHit::Minimize,
        1 => TabBarHit::Maximize,
        _ => TabBarHit::CloseWindow,
    }
}

/// Hit-test within the window controls zone (Linux/macOS: circular buttons).
///
/// `offset_x` is relative to the start of the controls zone.
/// Uses circular hit regions centered vertically in the tab bar.
/// Misses between circles fall through to `DragArea`.
#[cfg(not(target_os = "windows"))]
fn hit_test_controls(offset_x: f32, y: f32) -> TabBarHit {
    use super::constants::{
        CONTROL_BUTTON_DIAMETER, CONTROL_BUTTON_MARGIN, CONTROL_BUTTON_SPACING,
    };

    let cy = TAB_BAR_HEIGHT / 2.0;
    let r = CONTROL_BUTTON_DIAMETER / 2.0;
    let r_sq = r * r;
    let step = CONTROL_BUTTON_DIAMETER + CONTROL_BUTTON_SPACING;

    let buttons = [
        (CONTROL_BUTTON_MARGIN + r, TabBarHit::Minimize),
        (CONTROL_BUTTON_MARGIN + step + r, TabBarHit::Maximize),
        (
            CONTROL_BUTTON_MARGIN + 2.0 * step + r,
            TabBarHit::CloseWindow,
        ),
    ];

    for (cx, hit) in buttons {
        let dx = offset_x - cx;
        let dy = y - cy;
        if dx * dx + dy * dy <= r_sq {
            return hit;
        }
    }

    // Within controls zone but not on a button circle.
    TabBarHit::DragArea
}
