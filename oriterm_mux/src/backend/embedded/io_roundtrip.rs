//! IO-thread roundtrip helpers.
//!
//! The `extract_text`, `extract_html`, `select_command_output`,
//! `select_command_input`, and `enter_mark_mode` trait methods all
//! share the same `crossbeam_channel::bounded(1) → send_io_command →
//! recv_timeout(100ms)` shape. `pane_io_roundtrip` factors that
//! protocol into one place per impl-hygiene §Algorithmic DRY ("3+
//! instances, any size: always extract"). The 5 callers become
//! 1-line `pub(super) fn` inherent helpers on `EmbeddedMux` that
//! the trait impl delegates to.

use std::time::Duration;

use crossbeam_channel::Sender;
use oriterm_core::Selection;

use crate::backend::embedded::EmbeddedMux;
use crate::pane::io_thread::PaneIoCommand;
use crate::pane::{MarkCursor, Pane};
use crate::PaneId;

/// IO-thread reply timeout for synchronous query commands (ms).
const IO_REPLY_TIMEOUT: Duration = Duration::from_millis(100);

/// Send a `reply: Sender<T>`-carrying command to the pane's IO thread and
/// block for the reply up to `IO_REPLY_TIMEOUT`.
///
/// `build` constructs the `PaneIoCommand` variant from a fresh reply
/// sender; callers pass a closure that wires their payload into the
/// command. Returns `None` on timeout, channel-disconnect, or sender drop.
fn pane_io_roundtrip<T>(pane: &Pane, build: impl FnOnce(Sender<T>) -> PaneIoCommand) -> Option<T> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    pane.send_io_command(build(tx));
    rx.recv_timeout(IO_REPLY_TIMEOUT).ok()
}

impl EmbeddedMux {
    /// Synchronously extract the plain-text content of `sel` from the pane's
    /// IO-thread grid. Returns `None` if the pane is missing or the IO
    /// thread doesn't reply within `IO_REPLY_TIMEOUT`.
    pub(super) fn extract_text_impl(
        &self,
        pane_id: PaneId,
        sel: &Selection,
    ) -> Option<String> {
        let pane = self.panes.get(&pane_id)?;
        pane_io_roundtrip(pane, |reply| PaneIoCommand::ExtractText {
            selection: *sel,
            reply,
        })
        .flatten()
    }

    /// Synchronously extract HTML for `sel` with caller-supplied font
    /// family + size. Returns `(html, plain_text)` or `None` on missing
    /// pane / IO-thread timeout.
    pub(super) fn extract_html_impl(
        &self,
        pane_id: PaneId,
        sel: &Selection,
        font_family: &str,
        font_size: f32,
    ) -> Option<(String, String)> {
        let pane = self.panes.get(&pane_id)?;
        pane_io_roundtrip(pane, |reply| PaneIoCommand::ExtractHtml {
            selection: *sel,
            font_family: font_family.to_string(),
            font_size,
            reply,
        })
        .flatten()
    }

    /// Synchronously ask the IO thread for the selection covering the
    /// last command's stdout (shell-integration / OSC 133).
    pub(super) fn select_command_output_impl(&self, pane_id: PaneId) -> Option<Selection> {
        let pane = self.panes.get(&pane_id)?;
        pane_io_roundtrip(pane, |reply| PaneIoCommand::SelectCommandOutput { reply }).flatten()
    }

    /// Synchronously ask the IO thread for the selection covering the
    /// last command's input line.
    pub(super) fn select_command_input_impl(&self, pane_id: PaneId) -> Option<Selection> {
        let pane = self.panes.get(&pane_id)?;
        pane_io_roundtrip(pane, |reply| PaneIoCommand::SelectCommandInput { reply }).flatten()
    }

    /// Synchronously enter mark-mode; the IO thread returns the initial
    /// `MarkCursor` (typically the current cursor position).
    pub(super) fn enter_mark_mode_impl(&self, pane_id: PaneId) -> Option<MarkCursor> {
        let pane = self.panes.get(&pane_id)?;
        pane_io_roundtrip(pane, |reply| PaneIoCommand::EnterMarkMode { reply })
    }
}
