//! Tab bar hit-test result for cursor targeting.
//!
//! [`TabBarHit`] identifies which element of the tab bar the cursor is over.
//! Used by both the rendering layer (hover highlighting) and the event layer
//! (click dispatch in Section 16.3).

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
}
