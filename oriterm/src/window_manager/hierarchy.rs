//! Parent-child window relationships and cascading cleanup.
//!
//! Follows Chromium's transient window pattern: when a parent window closes,
//! all its children are closed first. Children always have a valid parent
//! (or `None` for root windows).

use winit::window::WindowId;

use super::WindowManager;
use crate::window_manager::types::ManagedWindow;

impl WindowManager {
    /// Register a new window. If the window has a parent, it is added to the
    /// parent's children list.
    pub fn register(&mut self, window: ManagedWindow) {
        let id = window.winit_id;
        if let Some(parent_id) = window.parent {
            if let Some(parent) = self.windows.get_mut(&parent_id) {
                parent.children.push(id);
            }
        }
        self.windows.insert(id, window);
    }

    /// Unregister a window and all its descendants.
    ///
    /// Returns the removed windows ordered children-first (depth-first),
    /// with the requested window last. The caller is responsible for
    /// actually closing the OS windows in this order.
    pub fn unregister(&mut self, id: WindowId) -> Vec<ManagedWindow> {
        let mut removed = Vec::new();
        self.collect_descendants(id, &mut removed);

        // Remove from parent's children list.
        if let Some(window) = self.windows.get(&id) {
            if let Some(parent_id) = window.parent {
                if let Some(parent) = self.windows.get_mut(&parent_id) {
                    parent.children.retain(|c| *c != id);
                }
            }
        }

        // Remove self last.
        if let Some(window) = self.windows.remove(&id) {
            removed.push(window);
        }

        // Clear focus if any removed window was focused.
        if removed.iter().any(|w| self.focused_id == Some(w.winit_id)) {
            self.focused_id = None;
        }

        removed
    }

    /// Change a window's parent.
    ///
    /// Removes from old parent's children list, adds to new parent's children
    /// list. Pass `None` to make the window a root window.
    pub fn reparent(&mut self, id: WindowId, new_parent: Option<WindowId>) {
        // Remove from old parent.
        let old_parent = self.windows.get(&id).and_then(|w| w.parent);
        if let Some(old_id) = old_parent {
            if let Some(parent) = self.windows.get_mut(&old_id) {
                parent.children.retain(|c| *c != id);
            }
        }

        // Add to new parent.
        if let Some(new_id) = new_parent {
            if let Some(parent) = self.windows.get_mut(&new_id) {
                parent.children.push(id);
            }
        }

        // Update the window's parent field.
        if let Some(window) = self.windows.get_mut(&id) {
            window.parent = new_parent;
        }
    }

    /// Recursively collect all descendants of `id`, removing them from the
    /// registry. Children are collected depth-first so they appear before
    /// their parents in the output.
    fn collect_descendants(&mut self, id: WindowId, out: &mut Vec<ManagedWindow>) {
        let children = match self.windows.get(&id) {
            Some(window) => window.children.clone(),
            None => return,
        };

        // Clear the parent's children vec to avoid stale references.
        if let Some(window) = self.windows.get_mut(&id) {
            window.children.clear();
        }

        for child_id in children {
            self.collect_descendants(child_id, out);
            if let Some(child) = self.windows.remove(&child_id) {
                out.push(child);
            }
        }
    }
}
