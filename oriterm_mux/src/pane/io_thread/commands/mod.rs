//! Command enum for the Terminal IO thread.
//!
//! Each variant replaces a `pane.terminal().lock()` call site on the main
//! thread. The IO thread processes commands in order, mutates `Term<T>`, and
//! produces a fresh snapshot after state changes.

use std::fmt;

use crossbeam_channel::Sender;

use oriterm_core::{CursorShape, Palette, Selection, Theme};

use crate::backend::ImageConfig;
use crate::pane::MarkCursor;

/// Commands sent from the main thread to the Terminal IO thread.
///
/// Commands are processed in FIFO order. Variants with a `reply` field use a
/// oneshot-style channel for request-response patterns (the IO thread sends
/// the result back via the provided `Sender`).
#[allow(
    dead_code,
    reason = "variants used incrementally as sections 02-06 are implemented"
)]
pub enum PaneIoCommand {
    /// Resize grid and notify PTY. The IO thread performs `Grid::resize()`
    /// with reflow, then sends SIGWINCH via `PtyControl`.
    Resize { rows: u16, cols: u16 },
    /// Change viewport scroll offset by `delta` lines.
    ScrollDisplay(isize),
    /// Reset to live view (`display_offset = 0`).
    ScrollToBottom,
    /// Scroll to nearest prompt above viewport.
    ScrollToPreviousPrompt,
    /// Scroll to nearest prompt below viewport.
    ScrollToNextPrompt,
    /// Change theme and palette (boxed — `Palette` is ~1.6 KB).
    SetTheme(Theme, Box<Palette>),
    /// Change cursor shape (from config or DECSCUSR).
    SetCursorShape(CursorShape),
    /// Toggle bold-is-bright rendering.
    SetBoldIsBright(bool),
    /// Force all lines dirty (after config change, etc.).
    MarkAllDirty,
    /// Produce a fresh snapshot now and reply when done.
    ///
    /// IO thread barrier — because commands are processed FIFO, sending
    /// `SnapshotNow` after one or more state-mutating commands and waiting
    /// for the reply guarantees that:
    /// 1. all earlier commands have been processed,
    /// 2. a fresh snapshot reflecting their effects is sitting in the
    ///    `SnapshotDoubleBuffer` ready to be swapped to the main thread.
    ///
    /// Used by `MuxBackend::sync_pane_snapshot` to provide a deterministic
    /// "scroll then read" pattern for tests and any callers that need a
    /// guaranteed-fresh snapshot. Production render code uses the async
    /// push pipeline and does not need this.
    SnapshotNow {
        /// Reply channel — sender notifies the caller when the snapshot
        /// has been published to the double buffer.
        reply: Sender<()>,
    },
    /// Update image protocol configuration.
    SetImageConfig(ImageConfig),
    /// Update cell pixel dimensions. Separate from `SetImageConfig`
    /// because cell metrics are runtime state derived from font
    /// rasterization, not user TOML config — mixing them into
    /// `ImageConfig` would let a config reload overwrite live metrics
    /// with zero/stale values (violating SSOT between static config
    /// and hardware/font state).
    SetCellDimensions { width: u16, height: u16 },
    /// Extract plain text from a selection region (response via oneshot).
    ExtractText {
        selection: Selection,
        reply: Sender<Option<String>>,
    },
    /// Extract HTML + plain text from a selection region (response via oneshot).
    ExtractHtml {
        selection: Selection,
        font_family: String,
        font_size: f32,
        reply: Sender<Option<(String, String)>>,
    },
    /// Open search mode on the terminal.
    OpenSearch,
    /// Close search mode on the terminal.
    CloseSearch,
    /// Set search query (triggers match computation on IO thread).
    SearchSetQuery(String),
    /// Navigate to next search match.
    SearchNextMatch,
    /// Navigate to previous search match.
    SearchPrevMatch,
    /// Enter mark mode — reply with the initial cursor position.
    EnterMarkMode { reply: Sender<MarkCursor> },
    /// Full terminal reset (RIS).
    Reset,
    /// Select the command output zone nearest to viewport center.
    SelectCommandOutput { reply: Sender<Option<Selection>> },
    /// Select the command input zone nearest to viewport center.
    SelectCommandInput { reply: Sender<Option<Selection>> },
    /// Shut down the IO thread (sent during pane close).
    Shutdown,
}

impl fmt::Debug for PaneIoCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resize { rows, cols } => f
                .debug_struct("Resize")
                .field("rows", rows)
                .field("cols", cols)
                .finish(),
            Self::ScrollDisplay(delta) => write!(f, "ScrollDisplay({delta})"),
            Self::ScrollToBottom => write!(f, "ScrollToBottom"),
            Self::ScrollToPreviousPrompt => write!(f, "ScrollToPreviousPrompt"),
            Self::ScrollToNextPrompt => write!(f, "ScrollToNextPrompt"),
            Self::SetTheme(..) => write!(f, "SetTheme(..)"),
            Self::SetCursorShape(shape) => write!(f, "SetCursorShape({shape:?})"),
            Self::SetBoldIsBright(b) => write!(f, "SetBoldIsBright({b})"),
            Self::MarkAllDirty => write!(f, "MarkAllDirty"),
            Self::SnapshotNow { .. } => write!(f, "SnapshotNow {{ .. }}"),
            Self::SetImageConfig(..) => write!(f, "SetImageConfig(..)"),
            Self::SetCellDimensions { width, height } => f
                .debug_struct("SetCellDimensions")
                .field("width", width)
                .field("height", height)
                .finish(),
            Self::ExtractText { .. } => write!(f, "ExtractText {{ .. }}"),
            Self::ExtractHtml { .. } => write!(f, "ExtractHtml {{ .. }}"),
            Self::OpenSearch => write!(f, "OpenSearch"),
            Self::CloseSearch => write!(f, "CloseSearch"),
            Self::SearchSetQuery(q) => write!(f, "SearchSetQuery({q:?})"),
            Self::SearchNextMatch => write!(f, "SearchNextMatch"),
            Self::SearchPrevMatch => write!(f, "SearchPrevMatch"),
            Self::EnterMarkMode { .. } => write!(f, "EnterMarkMode {{ .. }}"),
            Self::Reset => write!(f, "Reset"),
            Self::SelectCommandOutput { .. } => write!(f, "SelectCommandOutput {{ .. }}"),
            Self::SelectCommandInput { .. } => write!(f, "SelectCommandInput {{ .. }}"),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

#[cfg(test)]
mod tests;
