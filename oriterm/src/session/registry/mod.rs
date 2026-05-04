//! GUI-side registry of tabs and windows.
//!
//! Owns all windows and tabs for the GUI session, providing lookup
//! and lifecycle management.

use std::collections::HashMap;

use oriterm_mux::PaneId;

use super::id::{IdAllocator, TabId, WindowId};
use super::tab::Tab;
use super::window::Window;

/// (Window, tab-index) location of a pane within the session.
///
/// `window_id` is the OWNING window — the one that contains the tab, which
/// contains the pane. `tab_index` is the position of the owning tab within
/// that window's tab list. Resolved purely from session model state — does
/// NOT depend on which window is "active" or "focused" at the application
/// level (those concerns live in `oriterm/src/app/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneTabPosition {
    pub window_id: WindowId,
    pub tab_index: usize,
}

/// GUI-side registry of tabs and windows.
///
/// Owns ID allocation for tabs and windows — the mux no longer allocates
/// these IDs. Each GUI instance has its own independent allocator.
#[derive(Debug)]
pub struct SessionRegistry {
    /// All tabs, keyed by tab ID.
    tabs: HashMap<TabId, Tab>,
    /// All windows, keyed by window ID.
    windows: HashMap<WindowId, Window>,
    /// Tab ID allocator.
    tab_alloc: IdAllocator<TabId>,
    /// Window ID allocator.
    window_alloc: IdAllocator<WindowId>,
}

impl SessionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            windows: HashMap::new(),
            tab_alloc: IdAllocator::new(),
            window_alloc: IdAllocator::new(),
        }
    }

    /// Allocate a new tab ID.
    pub fn alloc_tab_id(&mut self) -> TabId {
        self.tab_alloc.alloc()
    }

    /// Allocate a new window ID.
    pub fn alloc_window_id(&mut self) -> WindowId {
        self.window_alloc.alloc()
    }

    /// Register a tab.
    pub fn add_tab(&mut self, tab: Tab) {
        self.tabs.insert(tab.id(), tab);
    }

    /// Remove a tab by ID.
    pub fn remove_tab(&mut self, tab_id: TabId) -> Option<Tab> {
        self.tabs.remove(&tab_id)
    }

    /// Look up a tab by ID.
    pub fn get_tab(&self, tab_id: TabId) -> Option<&Tab> {
        self.tabs.get(&tab_id)
    }

    /// Mutable access to a tab.
    pub fn get_tab_mut(&mut self, tab_id: TabId) -> Option<&mut Tab> {
        self.tabs.get_mut(&tab_id)
    }

    /// Register a window.
    pub fn add_window(&mut self, window: Window) {
        self.windows.insert(window.id(), window);
    }

    /// Remove a window by ID.
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<Window> {
        self.windows.remove(&window_id)
    }

    /// Look up a window by ID.
    pub fn get_window(&self, window_id: WindowId) -> Option<&Window> {
        self.windows.get(&window_id)
    }

    /// Mutable access to a window.
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&window_id)
    }

    /// Find which window contains a given tab.
    pub fn window_for_tab(&self, tab_id: TabId) -> Option<WindowId> {
        self.windows
            .values()
            .find(|w| w.tabs().contains(&tab_id))
            .map(Window::id)
    }

    /// Number of registered tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Number of registered windows.
    #[allow(dead_code, reason = "used in tests; part of session registry API")]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Iterate over all windows.
    #[allow(dead_code, reason = "used in tests; part of session registry API")]
    pub fn windows(&self) -> &HashMap<WindowId, Window> {
        &self.windows
    }

    /// Find which window contains a given pane.
    pub fn window_for_pane(&self, pane_id: PaneId) -> Option<WindowId> {
        let tab_id = self.tab_for_pane(pane_id)?;
        self.window_for_tab(tab_id)
    }

    /// Find which tab contains a given pane.
    pub fn tab_for_pane(&self, pane_id: PaneId) -> Option<TabId> {
        self.tabs
            .values()
            .find(|t| t.all_panes().contains(&pane_id))
            .map(Tab::id)
    }

    /// Resolve a pane's owning (window, tab-index) position.
    ///
    /// Returns `None` if the pane is not registered in any tab, if the
    /// owning tab is not in any window's tab list, or if the tab-index
    /// lookup against the owning window's tab list fails. Used by the
    /// application shell to route per-pane UI updates (bell pulses,
    /// notification icons) to the correct OWNING window — independent of
    /// which window is focused.
    pub fn pane_position(&self, pane_id: PaneId) -> Option<PaneTabPosition> {
        let tab_id = self.tab_for_pane(pane_id)?;
        let window_id = self.window_for_tab(tab_id)?;
        let win = self.get_window(window_id)?;
        let tab_index = win.tabs().iter().position(|&t| t == tab_id)?;
        Some(PaneTabPosition {
            window_id,
            tab_index,
        })
    }

    /// True when this pane is the only pane across all tabs and windows.
    pub fn is_last_pane(&self, pane_id: PaneId) -> bool {
        if self.tabs.len() != 1 {
            return false;
        }
        let Some((_, tab)) = self.tabs.iter().next() else {
            return false;
        };
        tab.all_panes() == [pane_id]
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
