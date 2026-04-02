//! Selection, search, and command-zone selection methods for [`Pane`].
//!
//! Extracted from `mod.rs` to keep file sizes under the 500-line limit.

use oriterm_core::{SearchState, Selection, SelectionPoint};

use super::Pane;

impl Pane {
    // -- Selection --

    /// Active text selection, if any.
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Replace the active selection.
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = Some(selection);
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Update the endpoint of an active selection during drag.
    pub fn update_selection_end(&mut self, end: SelectionPoint) {
        if let Some(sel) = &mut self.selection {
            sel.end = end;
        }
    }

    /// Check whether terminal output has invalidated the selection.
    ///
    /// Reads the lock-free `io_selection_dirty` atomic (set by the IO thread
    /// after VTE parsing) instead of locking the terminal.
    pub fn check_selection_invalidation(&mut self) {
        if !self
            .io_selection_dirty
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        // Terminal output changed — invalidate any active selection.
        self.selection = None;
    }

    // -- Search --

    /// Active search state, if any.
    pub fn search(&self) -> Option<&SearchState> {
        self.search.as_ref()
    }

    /// Mutable access to the active search state.
    pub fn search_mut(&mut self) -> Option<&mut SearchState> {
        self.search.as_mut()
    }

    /// Activate search.
    pub fn open_search(&mut self) {
        if self.search.is_none() {
            self.search = Some(SearchState::new());
        }
        self.search_active
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Close search.
    pub fn close_search(&mut self) {
        self.search = None;
        self.search_active
            .store(false, std::sync::atomic::Ordering::Release);
    }

    /// Whether search is currently active.
    ///
    /// Reads the lock-free `search_active` atomic, which is kept in sync
    /// by `open_search()` / `close_search()`. Does not require terminal
    /// access or a reply channel to the IO thread.
    pub fn is_search_active(&self) -> bool {
        self.search_active
            .load(std::sync::atomic::Ordering::Acquire)
    }

    // Command zone selection is handled by the IO thread via
    // `PaneIoCommand::SelectCommandOutput` / `SelectCommandInput`.
}
