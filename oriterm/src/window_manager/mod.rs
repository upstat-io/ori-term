//! Central window manager for tracking all OS windows.
//!
//! [`WindowManager`] is the single source of truth for what windows exist,
//! their kinds, parent-child hierarchy, and focus state. It stores only
//! metadata (IDs, kinds, relationships) — no winit handles, GPU resources,
//! or event loop references — so it is fully testable in headless `#[test]`.

mod hierarchy;
mod lifecycle;
pub(crate) mod platform;
pub mod types;

use std::collections::HashMap;

use winit::window::WindowId;

pub use types::{ManagedWindow, WindowKind};

/// Central registry for all OS windows in the application.
///
/// Tracks window kinds, parent-child ownership, and focus state.
/// Does not own any platform or GPU resources — those live in `App`
/// and `WindowContext`.
pub struct WindowManager {
    /// All managed windows, keyed by winit `WindowId`.
    windows: HashMap<WindowId, ManagedWindow>,
    /// The currently focused window (set on `Focused(true)` events).
    focused_id: Option<WindowId>,
}

impl WindowManager {
    /// Create an empty window manager.
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            focused_id: None,
        }
    }

    /// Get a managed window by winit ID.
    pub fn get(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    /// Get a mutable reference to a managed window by winit ID.
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut ManagedWindow> {
        self.windows.get_mut(&id)
    }

    /// Iterate all windows matching a kind predicate.
    pub fn windows_of_kind(
        &self,
        predicate: fn(&WindowKind) -> bool,
    ) -> impl Iterator<Item = &ManagedWindow> {
        self.windows.values().filter(move |w| predicate(&w.kind))
    }

    /// Iterate all main windows.
    pub fn main_windows(&self) -> impl Iterator<Item = &ManagedWindow> {
        self.windows_of_kind(WindowKind::is_main)
    }

    /// Iterate children of a specific window.
    pub fn children_of(&self, parent: WindowId) -> impl Iterator<Item = &ManagedWindow> {
        let children: Vec<WindowId> = self
            .windows
            .get(&parent)
            .map(|w| w.children.clone())
            .unwrap_or_default();
        children
            .into_iter()
            .filter_map(move |id| self.windows.get(&id))
    }

    /// Check if a window is registered.
    pub fn contains(&self, id: WindowId) -> bool {
        self.windows.contains_key(&id)
    }

    /// Total number of managed windows.
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Returns `true` if no windows are managed.
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Number of primary windows (main + tear-off).
    pub fn primary_window_count(&self) -> usize {
        self.windows
            .values()
            .filter(|w| w.kind.is_primary())
            .count()
    }

    /// Get the currently focused window ID.
    pub fn focused_id(&self) -> Option<WindowId> {
        self.focused_id
    }

    /// Set the currently focused window.
    pub fn set_focused(&mut self, id: Option<WindowId>) {
        self.focused_id = id;
    }

    /// The active main window. If a dialog is focused, returns its parent.
    ///
    /// Used for determining which terminal receives keyboard input
    /// (when no dialog is focused) and for resolving the session window
    /// associated with the current focus.
    pub fn active_main_window(&self) -> Option<WindowId> {
        let focused = self.focused_id?;
        let window = self.windows.get(&focused)?;
        match &window.kind {
            WindowKind::Main | WindowKind::TearOff => Some(focused),
            WindowKind::Dialog(_) => window.parent,
        }
    }

    /// Whether the focused window is a child dialog of `parent_id`.
    ///
    /// Used to suppress terminal focus-out events when focus moves from
    /// a main window to its owned dialog.
    pub fn focused_is_child_of(&self, parent_id: WindowId) -> bool {
        self.focused_id
            .and_then(|fid| self.windows.get(&fid))
            .is_some_and(|w| w.parent == Some(parent_id) && w.kind.is_dialog())
    }

    /// Whether a window is blocked by a modal child dialog.
    pub fn is_modal_blocked(&self, id: WindowId) -> bool {
        self.find_modal_child(id).is_some()
    }

    /// Find the modal child dialog of a window, if any.
    ///
    /// Returns the winit ID of the first modal dialog owned by `id`.
    /// Used to bring the modal to front when the blocked parent is clicked.
    pub fn find_modal_child(&self, id: WindowId) -> Option<WindowId> {
        let window = self.windows.get(&id)?;
        window.children.iter().copied().find(|child_id| {
            self.windows.get(child_id).is_some_and(|c| match &c.kind {
                WindowKind::Dialog(dk) => dk.is_modal(),
                _ => false,
            })
        })
    }
}

#[cfg(test)]
mod tests;
