//! Pane — the atomic per-shell unit in the mux model.
//!
//! Each `Pane` owns the full PTY ↔ terminal pipeline: a `Term<MuxEventProxy>`
//! wrapped in `Arc<FairMutex>`, the reader thread, and a `PaneNotifier` that
//! delivers keyboard input to the PTY. Lock-free atomics (`grid_dirty`,
//! `wakeup_pending`, `mode_cache`) allow the renderer and input handler to
//! query pane state without contending on the terminal lock.
//!
//! `Pane` is intentionally independent of `Tab` — the mux layer owns panes
//! directly. `Tab` will be replaced in Section 31/32.

mod mark_cursor;
mod shutdown;

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use oriterm_core::{FairMutex, SearchState, Selection, SelectionPoint, StableRowIndex, Term};
use oriterm_mux::{DomainId, PaneId};

pub(crate) use mark_cursor::MarkCursor;

use crate::mux_event::MuxEventProxy;
use crate::pty::{Msg, PtyControl, PtyHandle};

/// Sends input to the PTY and commands to the reader thread.
///
/// Duplicated from `tab::Notifier` to keep `pane` independent of `tab`.
/// Unified when Tab is replaced in Section 31/32.
#[allow(
    dead_code,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
pub(crate) struct PaneNotifier {
    /// Direct PTY writer — bypasses the reader thread's command channel.
    writer: std::sync::Mutex<Box<dyn io::Write + Send>>,
    /// Channel sender for commands (shutdown) to the reader thread.
    tx: mpsc::Sender<Msg>,
}

#[allow(
    dead_code,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
impl PaneNotifier {
    /// Create a new notifier with a direct PTY writer and command channel.
    pub(crate) fn new(writer: Box<dyn io::Write + Send>, tx: mpsc::Sender<Msg>) -> Self {
        Self {
            writer: std::sync::Mutex::new(writer),
            tx,
        }
    }

    /// Send raw bytes to the PTY (keyboard input, escape responses).
    pub(crate) fn notify(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        if let Ok(mut w) = self.writer.lock() {
            if let Err(e) = w.write_all(bytes) {
                log::warn!("PTY write failed: {e}");
            }
            let _ = w.flush();
        }
    }

    /// Request the reader thread to shut down.
    pub(crate) fn shutdown(&self) {
        let _ = self.tx.send(Msg::Shutdown);
    }
}

/// Pre-built parts for constructing a [`Pane`].
///
/// Groups all parameters for `Pane::from_parts` to stay under the clippy
/// argument limit. Produced by `LocalDomain::spawn_pane`.
#[allow(
    dead_code,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
pub(crate) struct PaneParts {
    /// Unique pane identifier.
    pub(crate) id: PaneId,
    /// Which domain spawned this pane.
    pub(crate) domain_id: DomainId,
    /// Shared terminal state.
    pub(crate) terminal: Arc<FairMutex<Term<MuxEventProxy>>>,
    /// Input/shutdown sender.
    pub(crate) notifier: PaneNotifier,
    /// PTY control handle for resize.
    pub(crate) pty_control: PtyControl,
    /// Reader thread join handle.
    pub(crate) reader_thread: JoinHandle<()>,
    /// PTY handle (child lifecycle).
    pub(crate) pty: PtyHandle,
    /// Grid dirty flag (lock-free).
    pub(crate) grid_dirty: Arc<AtomicBool>,
    /// Wakeup coalescing flag (lock-free).
    pub(crate) wakeup_pending: Arc<AtomicBool>,
    /// Mode bits cache (lock-free).
    pub(crate) mode_cache: Arc<AtomicU32>,
}

/// Owns all per-shell-session state: terminal, PTY handles, reader thread.
///
/// The atomic `Pane` unit in the mux model — one shell process, one grid,
/// one PTY connection. Created by `LocalDomain::spawn_pane`.
#[allow(
    dead_code,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
pub(crate) struct Pane {
    /// Unique pane identifier (from mux allocator).
    id: PaneId,
    /// Which domain spawned this pane.
    domain_id: DomainId,
    /// Shared terminal state (accessed by both render and PTY threads).
    terminal: Arc<FairMutex<Term<MuxEventProxy>>>,
    /// Sends input/shutdown to the PTY.
    notifier: PaneNotifier,
    /// PTY control handle for resize operations.
    pty_control: PtyControl,
    /// PTY reader thread join handle.
    reader_thread: Option<JoinHandle<()>>,
    /// Spawned PTY (reader/writer/control taken; child remains for lifecycle).
    pty: PtyHandle,
    /// Set by reader thread when new content is available.
    grid_dirty: Arc<AtomicBool>,
    /// Coalesces wakeup events from the reader thread.
    wakeup_pending: Arc<AtomicBool>,
    /// Lock-free cache of `TermMode::bits()` for hot-path queries.
    mode_cache: Arc<AtomicU32>,
    /// Last known window title (from OSC 0/2).
    title: String,
    /// Current working directory (from OSC 7).
    cwd: Option<String>,
    /// Bell indicator (set on bell event, cleared on focus).
    has_bell: bool,
    /// Active text selection, if any.
    selection: Option<Selection>,
    /// Mark mode cursor position (keyboard-driven selection).
    mark_cursor: Option<MarkCursor>,
    /// Active search state (query, matches, navigation).
    search: Option<SearchState>,
}

#[allow(
    dead_code,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
impl Pane {
    /// Construct a pane from pre-built parts.
    ///
    /// Called by `LocalDomain::spawn_pane` after setting up the PTY pipeline.
    pub(crate) fn from_parts(parts: PaneParts) -> Self {
        Self {
            id: parts.id,
            domain_id: parts.domain_id,
            terminal: parts.terminal,
            notifier: parts.notifier,
            pty_control: parts.pty_control,
            reader_thread: Some(parts.reader_thread),
            pty: parts.pty,
            grid_dirty: parts.grid_dirty,
            wakeup_pending: parts.wakeup_pending,
            mode_cache: parts.mode_cache,
            title: String::new(),
            cwd: None,
            has_bell: false,
            selection: None,
            mark_cursor: None,
            search: None,
        }
    }

    // -- Identity --

    /// Pane identity.
    pub(crate) fn id(&self) -> PaneId {
        self.id
    }

    /// Which domain spawned this pane.
    pub(crate) fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    // -- Lock-free accessors --

    /// Whether the pane's grid has new content to render.
    pub(crate) fn grid_dirty(&self) -> bool {
        self.grid_dirty.load(Ordering::Acquire)
    }

    /// Clear the grid dirty flag after rendering.
    pub(crate) fn clear_grid_dirty(&self) {
        self.grid_dirty.store(false, Ordering::Release);
    }

    /// Clear the wakeup pending flag after processing.
    pub(crate) fn clear_wakeup(&self) {
        self.wakeup_pending.store(false, Ordering::Release);
    }

    /// Current terminal mode bits (lock-free).
    ///
    /// Updated by the reader thread after each VTE chunk; read by the main
    /// thread for mouse reporting and cursor style without locking the terminal.
    pub(crate) fn mode(&self) -> u32 {
        self.mode_cache.load(Ordering::Acquire)
    }

    /// Refresh the mode cache from the terminal (must hold terminal lock).
    ///
    /// Called by the main thread under the terminal lock when processing
    /// wakeup events.
    pub(crate) fn refresh_mode_cache(&self) {
        let term = self.terminal.lock();
        self.mode_cache.store(term.mode().bits(), Ordering::Release);
    }

    // -- Terminal access --

    /// Shared terminal state for rendering.
    pub(crate) fn terminal(&self) -> &Arc<FairMutex<Term<MuxEventProxy>>> {
        &self.terminal
    }

    // -- Title / CWD / Bell --

    /// Last known window title (from OSC 0/2).
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Set the pane title.
    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Current working directory (from OSC 7).
    pub(crate) fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Set the current working directory.
    pub(crate) fn set_cwd(&mut self, cwd: String) {
        self.cwd = Some(cwd);
    }

    /// Whether the bell has fired since the pane was last focused.
    pub(crate) fn has_bell(&self) -> bool {
        self.has_bell
    }

    /// Clear the bell indicator (call when the pane gains focus).
    pub(crate) fn clear_bell(&mut self) {
        self.has_bell = false;
    }

    /// Set the bell indicator.
    pub(crate) fn set_bell(&mut self) {
        self.has_bell = true;
    }

    // -- Selection --

    /// Active text selection, if any.
    pub(crate) fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Replace the active selection.
    pub(crate) fn set_selection(&mut self, selection: Selection) {
        self.selection = Some(selection);
    }

    /// Clear the active selection.
    pub(crate) fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Update the endpoint of an active selection during drag.
    pub(crate) fn update_selection_end(&mut self, end: SelectionPoint) {
        if let Some(sel) = &mut self.selection {
            sel.end = end;
        }
    }

    /// Check whether terminal output has invalidated the selection.
    pub(crate) fn check_selection_invalidation(&mut self) {
        if self.selection.is_none() {
            let mut term = self.terminal.lock();
            if term.is_selection_dirty() {
                term.clear_selection_dirty();
            }
            return;
        }
        let mut term = self.terminal.lock();
        if term.is_selection_dirty() {
            term.clear_selection_dirty();
            drop(term);
            self.selection = None;
        }
    }

    // -- Mark cursor --

    /// Whether mark mode is active.
    pub(crate) fn is_mark_mode(&self) -> bool {
        self.mark_cursor.is_some()
    }

    /// Current mark cursor position.
    pub(crate) fn mark_cursor(&self) -> Option<MarkCursor> {
        self.mark_cursor
    }

    /// Enter mark mode at the terminal cursor position.
    pub(crate) fn enter_mark_mode(&mut self) {
        if self.mark_cursor.is_some() {
            return;
        }
        self.scroll_to_bottom();
        let mc = {
            let term = self.terminal.lock();
            let g = term.grid();
            let cursor = g.cursor();
            let abs_row = g.scrollback().len() + cursor.line();
            let stable = StableRowIndex::from_absolute(g, abs_row);
            MarkCursor {
                row: stable,
                col: cursor.col().0,
            }
        };
        self.mark_cursor = Some(mc);
    }

    /// Exit mark mode.
    pub(crate) fn exit_mark_mode(&mut self) {
        self.mark_cursor = None;
    }

    /// Update the mark cursor position.
    pub(crate) fn set_mark_cursor(&mut self, cursor: MarkCursor) {
        self.mark_cursor = Some(cursor);
    }

    // -- Search --

    /// Active search state, if any.
    pub(crate) fn search(&self) -> Option<&SearchState> {
        self.search.as_ref()
    }

    /// Mutable access to the active search state.
    pub(crate) fn search_mut(&mut self) -> Option<&mut SearchState> {
        self.search.as_mut()
    }

    /// Activate search.
    pub(crate) fn open_search(&mut self) {
        if self.search.is_none() {
            self.search = Some(SearchState::new());
        }
    }

    /// Close search.
    pub(crate) fn close_search(&mut self) {
        self.search = None;
    }

    /// Whether search is currently active.
    pub(crate) fn is_search_active(&self) -> bool {
        self.search.is_some()
    }

    // -- I/O operations --

    /// Send raw bytes to the PTY.
    pub(crate) fn write_input(&self, bytes: &[u8]) {
        self.notifier.notify(bytes);
    }

    /// Scroll to the live terminal position.
    pub(crate) fn scroll_to_bottom(&self) {
        let mut term = self.terminal.lock();
        if term.grid().display_offset() > 0 {
            term.grid_mut().scroll_display(isize::MIN);
        }
    }

    /// Scroll the viewport by `delta` lines.
    pub(crate) fn scroll_display(&self, delta: isize) {
        self.terminal.lock().grid_mut().scroll_display(delta);
    }

    /// Resize the terminal grids (with reflow). Does NOT resize the PTY.
    pub(crate) fn resize_grid(&self, rows: u16, cols: u16) {
        self.terminal.lock().resize(rows as usize, cols as usize);
    }

    /// Resize the OS PTY handle, sending SIGWINCH to the shell.
    pub(crate) fn resize_pty(&self, rows: u16, cols: u16) {
        if let Err(e) = self.pty_control.resize(rows, cols) {
            log::warn!("PTY resize failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests;
