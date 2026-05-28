//! Command enum for the Terminal IO thread.
//!
//! Each variant replaces a `pane.terminal().lock()` call site on the main
//! thread. The IO thread processes commands in order, mutates `Term<T>`, and
//! produces a fresh snapshot after state changes.

use std::fmt;

use crossbeam_channel::Sender;

use oriterm_core::encode::mouse::MouseEvent;
use oriterm_core::{CursorShape, Palette, Selection, Theme};

use crate::backend::ImageConfig;
use crate::pane::MarkCursor;

/// Commands sent from the main thread to the Terminal IO thread.
///
/// Commands are processed in FIFO order. Variants with a `reply` field use a
/// oneshot-style channel for request-response patterns (the IO thread sends
/// the result back via the provided `Sender`).
pub enum PaneIoCommand {
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
    /// Dispatch a semantic mouse input to the IO thread's `Term` so
    /// encoding + emission flow through the Decision-10-Option-A apex
    /// (`Term::handle_mouse_input` reads `TermMode`, encodes, pushes
    /// `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent,
    /// .. })` via `effect_sink`; the `effect_router` lands the bytes at
    /// `MuxEvent::PtyWrite { kind, .. }`; the in-process pump carries
    /// them to `pane.write_input`). Replaces the historical App-side
    /// `encode_mouse_event + write_pane_input` direct path so the kind
    /// discriminator survives the boundary.
    HandleMouseInput(MouseEvent),
    /// Dispatch a semantic focus change to the IO thread's `Term` so
    /// emission flows through the Decision-10-Option-A apex
    /// (`Term::handle_focus_event` reads `TermMode::FOCUS_IN_OUT`,
    /// emits `Effect::Pty(PtyEffect::Write { kind:
    /// PtyWriteKind::FocusEvent, .. })` via `effect_sink`).
    /// `focused: true` → focus-in (CSI I); `false` → focus-out (CSI O).
    HandleFocusEvent(bool),
    /// Update the ENQ answerback string on the IO thread's `Term`.
    ///
    /// Empty (default) suppresses emission per ECMA-48 §8.3.40 + `WezTerm`
    /// parity (`term/src/terminalstate/performer.rs:473-478`). Non-empty
    /// bytes are written to the PTY verbatim on each `ENQ` byte received.
    /// Routed via `MuxBackend::set_answerback` from `Config.behavior.answerback`
    /// at every spawn site + on hot-reload. The handler arm DOES NOT mark
    /// `grid_dirty` because answerback affects only the ENQ outbound write,
    /// not any visible state — matches the `SetImageConfig` non-visual
    /// precedent at `handler.rs:120-126`.
    SetAnswerback(Vec<u8>),
}

impl PaneIoCommand {
    /// SSOT classification for variants that carry a reply
    /// `Sender<...>`.
    ///
    /// Production code does NOT use this for runtime dispatch — the
    /// `apply_pending_resize()` flush is unconditional per the §05
    /// Round-3 design. Sole consumer: the §03 exhaustiveness pin
    /// `is_reply_bearing_predicate_matches_reply_field_presence`,
    /// which fails compilation when a future contributor adds a new
    /// `reply: Sender<_>` variant without updating this predicate.
    #[cfg(test)]
    pub(super) fn is_reply_bearing(&self) -> bool {
        matches!(
            self,
            Self::SnapshotNow { .. }
                | Self::ExtractText { .. }
                | Self::ExtractHtml { .. }
                | Self::EnterMarkMode { .. }
                | Self::SelectCommandOutput { .. }
                | Self::SelectCommandInput { .. }
        )
    }
}

impl fmt::Debug for PaneIoCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::HandleMouseInput(event) => f
                .debug_struct("HandleMouseInput")
                .field("button", &event.button)
                .field("kind", &event.kind)
                .field("col", &event.col)
                .field("line", &event.line)
                .finish(),
            Self::HandleFocusEvent(focused) => {
                write!(f, "HandleFocusEvent({focused})")
            }
            Self::SetAnswerback(bytes) => write!(f, "SetAnswerback({} bytes)", bytes.len()),
        }
    }
}

#[cfg(test)]
mod tests;
