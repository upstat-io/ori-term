//! Mark mode types — keyboard-driven cursor navigation and selection.
//!
//! Pure type definitions shared between the UI framework and app layer.
//! The dispatch logic (`handle_mark_mode_key`) stays in `oriterm` because
//! it depends on `SnapshotGrid` and `MarkCursor` from `oriterm_mux`.

pub mod motion;

use oriterm_core::Selection;

/// Result of processing a key event in mark mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkAction {
    /// Key was handled (consumed by mark mode).
    ///
    /// `scroll_delta` is `Some(delta)` when the viewport needs to scroll
    /// to keep the mark cursor visible. The caller applies the scroll via
    /// `MuxBackend::scroll_display`.
    Handled { scroll_delta: Option<isize> },
    /// Key was not recognized by mark mode (fall through).
    Ignored,
    /// Exit mark mode. `copy` indicates whether to copy the selection.
    Exit {
        /// Whether to copy the selection to the clipboard on exit.
        copy: bool,
    },
}

/// Selection state update from mark mode.
pub enum SelectionUpdate {
    /// Set or replace the selection.
    Set(Selection),
    /// Clear the selection.
    Clear,
}
