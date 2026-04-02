//! Pane — the atomic per-shell unit in the mux model.
//!
//! Each `Pane` owns a `PaneIoHandle` that communicates with the Terminal IO
//! thread via channels. The IO thread exclusively owns `Term<T>` — the main
//! thread never locks terminal state. Lock-free atomics (`mode_cache`,
//! `io_selection_dirty`) allow the renderer and input handler to query pane
//! state without contending on any lock.
//!
//! `Pane` is the atomic per-shell unit in the mux model — the mux layer
//! owns panes directly with no higher-level grouping.

pub(crate) mod io_thread;
mod mark_cursor;
mod selection;
mod shutdown;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;

use std::thread::JoinHandle;

use crate::{DomainId, PaneId};
use oriterm_core::term::cwd_short_path;
use oriterm_core::{RenderableContent, SearchState, Selection};

pub use mark_cursor::MarkCursor;

use crate::pane::io_thread::{PaneIoCommand, PaneIoHandle};
use crate::pty::{Msg, PtyHandle};

/// Sends input to the PTY and commands to the reader thread.
///
/// All writes flow through the `mpsc` channel to the PTY reader thread,
/// which owns the actual PTY writer. This prevents blocking the main
/// thread when the PTY kernel buffer is full (e.g. during flood output).
pub struct PaneNotifier {
    /// Channel sender for input and shutdown commands to the reader thread.
    tx: mpsc::Sender<Msg>,
}

impl PaneNotifier {
    /// Create a new notifier with a command channel to the reader thread.
    pub fn new(tx: mpsc::Sender<Msg>) -> Self {
        Self { tx }
    }

    /// Send raw bytes to the PTY (keyboard input, escape responses).
    ///
    /// Non-blocking — enqueues via the channel. The reader thread drains
    /// the queue and writes to the PTY fd.
    pub fn notify(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        if let Err(e) = self.tx.send(Msg::Input(bytes.to_vec())) {
            log::warn!("PTY channel send failed: {e}");
        }
    }

    /// Request the reader thread to shut down.
    pub fn shutdown(&self) {
        let _ = self.tx.send(Msg::Shutdown);
    }
}

/// Pre-built parts for constructing a [`Pane`].
///
/// Groups all parameters for `Pane::from_parts` to stay under the clippy
/// argument limit. Produced by `LocalDomain::spawn_pane`.
pub struct PaneParts {
    /// Unique pane identifier.
    pub id: PaneId,
    /// Which domain spawned this pane.
    pub domain_id: DomainId,
    /// Input/shutdown sender.
    pub notifier: PaneNotifier,
    /// Reader thread join handle.
    pub reader_thread: JoinHandle<()>,
    /// Writer thread join handle.
    pub writer_thread: JoinHandle<()>,
    /// PTY handle (child lifecycle).
    pub pty: PtyHandle,
    /// Lock-free mode bits cache (shared with IO thread).
    pub mode_cache: Arc<AtomicU32>,
    /// Terminal IO thread handle (owns command + byte channels).
    pub io_handle: PaneIoHandle,
    /// Shared selection-dirty flag (passed to IO thread).
    pub io_selection_dirty: Arc<AtomicBool>,
}

/// Owns all per-shell-session state: IO thread handle, PTY handles, threads.
///
/// The atomic `Pane` unit in the mux model — one shell process, one grid,
/// one PTY connection. Created by `LocalDomain::spawn_pane`.
pub struct Pane {
    /// Unique pane identifier (from mux allocator).
    id: PaneId,
    /// Which domain spawned this pane.
    #[allow(dead_code, reason = "read when multi-domain routing is wired to App")]
    domain_id: DomainId,
    /// Sends input/shutdown to the PTY.
    notifier: PaneNotifier,
    /// PTY reader thread join handle (detached on drop).
    #[allow(
        dead_code,
        reason = "holds JoinHandle for thread lifetime — detached on drop"
    )]
    reader_thread: Option<JoinHandle<()>>,
    /// PTY writer thread join handle (detached on drop).
    #[allow(
        dead_code,
        reason = "holds JoinHandle for thread lifetime — detached on drop"
    )]
    writer_thread: Option<JoinHandle<()>>,
    /// Terminal IO thread handle — all terminal access goes through commands.
    ///
    /// Drops cleanly on pane close via `PaneIoHandle::Drop`, which sends
    /// `Shutdown` and joins the thread.
    io_handle: PaneIoHandle,
    /// Lock-free selection-dirty flag (set by IO thread, read/cleared by main thread).
    io_selection_dirty: Arc<AtomicBool>,
    /// Spawned PTY (reader/writer/control taken; child remains for lifecycle).
    pty: PtyHandle,
    /// Lock-free cache of `TermMode::bits()` for hot-path queries.
    ///
    /// Shared with the IO thread — the IO thread writes after each VTE parse,
    /// the main thread reads for mouse reporting and cursor style.
    mode_cache: Arc<AtomicU32>,
    /// Last known window title (from OSC 0/2).
    title: String,
    /// Icon name (from OSC 0/1) for tab icons.
    icon_name: Option<String>,
    /// Current working directory (from OSC 7).
    cwd: Option<String>,
    /// Whether the current title was explicitly set via OSC 0/2.
    ///
    /// Authoritative source — `Term` does not track this. Set by
    /// `set_title()` (true when non-empty) and cleared by `set_cwd()`.
    /// When `false`, `effective_title()` prefers CWD-based title.
    has_explicit_title: bool,
    /// Duration of the last completed command (from OSC 133 C→D timing).
    last_command_duration: Option<std::time::Duration>,
    /// Bell indicator (set on bell event, cleared on focus).
    has_bell: bool,
    /// Unseen output indicator (set when output arrives while not focused).
    ///
    /// Cleared when the pane becomes the active/focused tab. Used by the
    /// tab bar to show a "modified" dot on background tabs with new output.
    has_unseen_output: bool,
    /// Active text selection, if any.
    selection: Option<Selection>,
    /// Mark mode cursor position (keyboard-driven selection).
    mark_cursor: Option<MarkCursor>,
    /// Active search state (query, matches, navigation).
    search: Option<SearchState>,
    /// Lock-free search active flag (mirrors IO thread's search state).
    ///
    /// Set by `EmbeddedMux::open_search()`, cleared by `close_search()`.
    /// Allows `is_search_active()` to work without locking the terminal
    /// or requiring a reply channel to the IO thread.
    search_active: Arc<AtomicBool>,
}

impl Pane {
    /// Construct a pane from pre-built parts.
    ///
    /// Called by `LocalDomain::spawn_pane` after setting up the PTY pipeline.
    pub fn from_parts(parts: PaneParts) -> Self {
        Self {
            id: parts.id,
            domain_id: parts.domain_id,
            notifier: parts.notifier,
            reader_thread: Some(parts.reader_thread),
            writer_thread: Some(parts.writer_thread),
            io_handle: parts.io_handle,
            io_selection_dirty: parts.io_selection_dirty,
            pty: parts.pty,
            mode_cache: parts.mode_cache,
            title: String::new(),
            icon_name: None,
            cwd: None,
            has_explicit_title: false,
            last_command_duration: None,
            has_bell: false,
            has_unseen_output: false,
            selection: None,
            mark_cursor: None,
            search: None,
            search_active: Arc::new(AtomicBool::new(false)),
        }
    }

    // -- Identity --

    /// Pane identity.
    #[allow(dead_code, reason = "used when pane CRUD is fully wired to App")]
    pub fn id(&self) -> PaneId {
        self.id
    }

    /// Which domain spawned this pane.
    #[allow(dead_code, reason = "used when multi-domain routing is wired to App")]
    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    // -- Lock-free accessors --

    /// Current terminal mode bits (lock-free).
    ///
    /// Updated by the IO thread after each VTE chunk; read by the main
    /// thread for mouse reporting and cursor style without locking.
    pub fn mode(&self) -> u32 {
        self.mode_cache.load(Ordering::Acquire)
    }

    /// Whether the IO thread's terminal has flagged selection-dirty.
    pub fn is_io_selection_dirty(&self) -> bool {
        self.io_selection_dirty.load(Ordering::Acquire)
    }

    /// Clear the IO-thread selection-dirty flag.
    pub fn clear_io_selection_dirty(&self) {
        self.io_selection_dirty.store(false, Ordering::Release);
    }

    // -- IO thread access --

    /// Swap the latest IO-thread-produced snapshot into `buf`.
    ///
    /// Returns `true` if a new snapshot was available. When `false`, `buf`
    /// is unchanged — the caller should use the previously cached content.
    /// Delegates to [`SnapshotDoubleBuffer::swap_front()`].
    pub fn swap_io_snapshot(&self, buf: &mut RenderableContent) -> bool {
        self.io_handle.double_buffer().swap_front(buf)
    }

    /// Whether the IO thread has produced a new snapshot not yet consumed.
    pub fn has_io_snapshot(&self) -> bool {
        self.io_handle.double_buffer().has_new()
    }

    /// Send a command to the IO thread.
    ///
    /// Used for all terminal state mutations: scroll, theme, cursor shape,
    /// resize, search, text extraction, etc.
    pub fn send_io_command(&self, cmd: PaneIoCommand) {
        self.io_handle.send_command(cmd);
    }

    // -- Title / CWD / Bell --

    /// Set the pane title (from OSC 0/2 via `MuxEvent::PaneTitleChanged`).
    pub fn set_title(&mut self, title: String) {
        self.has_explicit_title = !title.is_empty();
        self.title = title;
    }

    /// Icon name (from OSC 0/1) for tab icon detection.
    pub fn icon_name(&self) -> Option<&str> {
        self.icon_name.as_deref()
    }

    /// Set the icon name.
    pub fn set_icon_name(&mut self, name: String) {
        if name.is_empty() {
            self.icon_name = None;
        } else {
            self.icon_name = Some(name);
        }
    }

    /// Resolved display title with 3-source priority:
    /// 1. Explicit title from OSC 0/2.
    /// 2. Short path from CWD (last component).
    /// 3. Fallback to raw title (may be empty).
    pub fn effective_title(&self) -> &str {
        if self.has_explicit_title {
            return &self.title;
        }
        if let Some(ref cwd) = self.cwd {
            return cwd_short_path(cwd);
        }
        &self.title
    }

    /// Current working directory (from OSC 7).
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Set the current working directory (clears explicit title flag).
    pub fn set_cwd(&mut self, cwd: String) {
        self.has_explicit_title = false;
        self.cwd = Some(cwd);
    }

    /// Duration of the last completed command.
    #[allow(
        dead_code,
        reason = "read when command notification UI is wired to App"
    )]
    pub fn last_command_duration(&self) -> Option<std::time::Duration> {
        self.last_command_duration
    }

    /// Store the duration of a completed command.
    pub fn set_last_command_duration(&mut self, duration: std::time::Duration) {
        self.last_command_duration = Some(duration);
    }

    /// Whether the bell has fired since the pane was last focused.
    #[allow(dead_code, reason = "used when bell indicator is wired to App")]
    pub fn has_bell(&self) -> bool {
        self.has_bell
    }

    /// Clear the bell indicator (call when the pane gains focus).
    #[allow(dead_code, reason = "used when bell indicator is wired to App")]
    pub fn clear_bell(&mut self) {
        self.has_bell = false;
    }

    /// Set the bell indicator.
    pub fn set_bell(&mut self) {
        self.has_bell = true;
    }

    /// Whether the pane has output the user hasn't seen yet.
    ///
    /// Set when output arrives while the pane is not focused. Cleared
    /// when the pane becomes the active tab. Drives the tab bar's
    /// "modified" indicator dot.
    pub fn has_unseen_output(&self) -> bool {
        self.has_unseen_output
    }

    /// Mark this pane as having unseen output.
    pub fn set_unseen_output(&mut self) {
        self.has_unseen_output = true;
    }

    /// Clear the unseen output flag (call when the pane gains focus).
    pub fn mark_output_seen(&mut self) {
        self.has_unseen_output = false;
    }

    // -- Mark cursor --

    /// Whether mark mode is active.
    pub fn is_mark_mode(&self) -> bool {
        self.mark_cursor.is_some()
    }

    /// Current mark cursor position.
    pub fn mark_cursor(&self) -> Option<MarkCursor> {
        self.mark_cursor
    }

    /// Exit mark mode.
    pub fn exit_mark_mode(&mut self) {
        self.mark_cursor = None;
    }

    /// Update the mark cursor position.
    pub fn set_mark_cursor(&mut self, cursor: MarkCursor) {
        self.mark_cursor = Some(cursor);
    }

    // -- I/O operations --

    /// Send raw bytes to the PTY.
    pub fn write_input(&self, bytes: &[u8]) {
        self.notifier.notify(bytes);
    }
}

#[cfg(test)]
mod tests;
