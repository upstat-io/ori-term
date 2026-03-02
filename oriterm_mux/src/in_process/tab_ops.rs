//! Tab CRUD operations for `InProcessMux`.
//!
//! Extracted from `mod.rs` to keep file sizes under the 500-line limit.
//! Covers tab creation, closure, splitting, zoom, resize, and undo/redo.

use std::collections::HashSet;
use std::io;
use std::sync::Arc;

use oriterm_core::Theme;

use crate::domain::SpawnConfig;
use crate::layout::SplitDirection;
use crate::mux_event::MuxNotification;
use crate::pane::Pane;
use crate::session::MuxTab;
use crate::{PaneId, TabId, WindowId};

use super::InProcessMux;

impl InProcessMux {
    /// Create a new tab with a single pane in the given window.
    ///
    /// Returns `(TabId, PaneId, Pane)` — the caller stores the `Pane`.
    pub fn create_tab(
        &mut self,
        window_id: WindowId,
        config: &SpawnConfig,
        theme: Theme,
        wakeup: &Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<(TabId, PaneId, Pane)> {
        let tab_id = self.tab_alloc.alloc();
        let (pane_id, pane) = self.spawn_pane(tab_id, config, theme, wakeup)?;

        let mux_tab = MuxTab::new(tab_id, pane_id);
        self.session.add_tab(mux_tab);

        if let Some(win) = self.session.get_window_mut(window_id) {
            win.add_tab(tab_id);
        }

        self.notifications
            .push(MuxNotification::TabLayoutChanged(tab_id));
        self.notifications
            .push(MuxNotification::WindowTabsChanged(window_id));

        Ok((tab_id, pane_id, pane))
    }

    /// Close a tab and all its panes.
    ///
    /// Returns the list of `PaneId`s that the caller should drop from its map.
    pub fn close_tab(&mut self, tab_id: TabId) -> Vec<PaneId> {
        let pane_ids = match self.session.get_tab(tab_id) {
            Some(tab) => tab.all_panes(),
            None => return Vec::new(),
        };

        // Unregister all panes.
        for &pid in &pane_ids {
            self.pane_registry.unregister(pid);
            self.notifications.push(MuxNotification::PaneClosed(pid));
        }

        // Find the owning window before removing the tab (window_for_tab
        // needs the tab to still exist in windows, but not in session.tabs).
        let window_id = self.session.window_for_tab(tab_id);

        // Remove the tab from the session.
        self.session.remove_tab(tab_id);

        // Update the owning window (cascades to window removal if empty).
        if let Some(wid) = window_id {
            if self.handle_window_after_tab_removal(wid, tab_id) {
                self.notifications.push(MuxNotification::LastWindowClosed);
            }
        }

        pane_ids
    }

    /// Split an existing pane, creating a new pane as its sibling.
    ///
    /// Returns `(PaneId, Pane)` for the newly created pane.
    #[allow(
        clippy::too_many_arguments,
        reason = "split requires source pane + direction on top of spawn params"
    )]
    pub fn split_pane(
        &mut self,
        tab_id: TabId,
        source_pane: PaneId,
        direction: SplitDirection,
        config: &SpawnConfig,
        theme: Theme,
        wakeup: &Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<(PaneId, Pane)> {
        let (new_pane_id, pane) = self.spawn_pane(tab_id, config, theme, wakeup)?;

        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            self.pane_registry.unregister(new_pane_id);
            return Err(io::Error::other("tab not found after spawn"));
        };
        let new_tree = tab
            .tree()
            .split_at(source_pane, direction, new_pane_id, 0.5);
        tab.set_tree(new_tree);

        self.notifications
            .push(MuxNotification::TabLayoutChanged(tab_id));

        Ok((new_pane_id, pane))
    }

    /// Set the ratio of a specific divider identified by the pane pair.
    ///
    /// The divider is the split where `first` contains `pane_before` and
    /// `second` contains `pane_after`. Emits `TabLayoutChanged`.
    pub fn set_divider_ratio(
        &mut self,
        tab_id: TabId,
        pane_before: PaneId,
        pane_after: PaneId,
        new_ratio: f32,
    ) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        let new_tree = tab
            .tree()
            .set_divider_ratio(pane_before, pane_after, new_ratio);
        if new_tree != *tab.tree() {
            tab.set_tree(new_tree);
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
        }
    }

    /// Resize a pane by adjusting the nearest qualifying split border.
    ///
    /// `axis` is the split direction to match, `pane_in_first` selects the
    /// qualifying child side, and `delta` adjusts the ratio. See
    /// [`SplitTree::resize_toward`] for the algorithm. Emits
    /// `TabLayoutChanged` if a qualifying split was found.
    #[expect(
        clippy::too_many_arguments,
        reason = "resize requires tab + pane + axis + side + delta"
    )]
    pub fn resize_pane(
        &mut self,
        tab_id: TabId,
        pane_id: PaneId,
        axis: SplitDirection,
        pane_in_first: bool,
        delta: f32,
    ) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        if let Some(new_tree) = tab
            .tree()
            .try_resize_toward(pane_id, axis, pane_in_first, delta)
        {
            tab.set_tree(new_tree);
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
        }
    }

    /// Toggle zoom on the active pane in a tab.
    ///
    /// If already zoomed, unzooms. Otherwise zooms the active pane.
    pub fn toggle_zoom(&mut self, tab_id: TabId) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        if tab.zoomed_pane().is_some() {
            tab.set_zoomed_pane(None);
        } else {
            tab.set_zoomed_pane(Some(tab.active_pane()));
        }
        self.notifications
            .push(MuxNotification::TabLayoutChanged(tab_id));
    }

    /// Clear zoom on a tab if it is currently zoomed.
    ///
    /// Emits `TabLayoutChanged` when zoom was active. For callers that will
    /// emit their own notification, use [`unzoom_silent`] instead.
    #[cfg(test)]
    pub fn unzoom(&mut self, tab_id: TabId) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        if tab.zoomed_pane().is_some() {
            tab.set_zoomed_pane(None);
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
        }
    }

    /// Clear zoom without emitting a `TabLayoutChanged` notification.
    ///
    /// Used by operations that will emit their own layout notification
    /// after the subsequent mutation, avoiding a redundant recomputation.
    pub fn unzoom_silent(&mut self, tab_id: TabId) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        if tab.zoomed_pane().is_some() {
            tab.set_zoomed_pane(None);
        }
    }

    /// Reset all split ratios to 0.5 (equal sizing).
    pub fn equalize_panes(&mut self, tab_id: TabId) {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return;
        };
        let new_tree = tab.tree().equalize();
        if new_tree != *tab.tree() {
            tab.set_tree(new_tree);
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
        }
    }

    /// Undo the last split tree mutation on the given tab.
    ///
    /// Returns `true` if an undo was applied.
    pub fn undo_split(&mut self, tab_id: TabId, live_panes: &HashSet<PaneId>) -> bool {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return false;
        };
        if tab.undo_tree(live_panes) {
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
            true
        } else {
            false
        }
    }

    /// Redo the last undone split tree mutation on the given tab.
    ///
    /// Returns `true` if a redo was applied.
    pub fn redo_split(&mut self, tab_id: TabId, live_panes: &HashSet<PaneId>) -> bool {
        let Some(tab) = self.session.get_tab_mut(tab_id) else {
            return false;
        };
        if tab.redo_tree(live_panes) {
            self.notifications
                .push(MuxNotification::TabLayoutChanged(tab_id));
            true
        } else {
            false
        }
    }
}
