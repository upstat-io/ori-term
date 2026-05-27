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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;

use std::thread::JoinHandle;

use crate::{DomainId, PaneId};
use oriterm_core::term::cwd_short_path;
use oriterm_core::{RenderableContent, SearchState, Selection};

pub use mark_cursor::MarkCursor;

use crate::pane::io_thread::{PaneIoCommand, PaneIoHandle};
use crate::pty::{Msg, PtyLifecycle};

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
    ///
    /// Boxed as a trait object so the same `Pane` type can hold either a
    /// spawned [`PtyHandle`](crate::pty::PtyHandle) or an adopted handle
    /// (Phase 1B introduces `AdoptedPtyHandle` for the Windows default
    /// terminal handoff path in Section 03.9).
    pub pty: Box<dyn PtyLifecycle + Send>,
    /// Lock-free mode bits cache (shared with IO thread).
    pub mode_cache: Arc<AtomicU64>,
    /// Terminal IO thread handle (owns command + byte channels).
    pub io_handle: PaneIoHandle,
    /// Shared selection-dirty flag (passed to IO thread).
    pub io_selection_dirty: Arc<AtomicBool>,
    /// Write-stall detection flag (shared with writer thread).
    pub write_stalled: Arc<AtomicBool>,
    /// Dup'd PTY master fd for `tcgetpgrp()` (Unix only).
    #[cfg(unix)]
    pub master_fd: Option<std::os::unix::io::OwnedFd>,
}

/// Owns all per-shell-session state: IO thread handle, PTY handles, threads.
///
/// The atomic `Pane` unit in the mux model — one shell process, one grid,
/// one PTY connection. Created by `LocalDomain::spawn_pane`.
pub struct Pane {
    /// Unique pane identifier (from mux allocator).
    id: PaneId,
    /// Which domain spawned this pane.
    domain_id: DomainId,
    /// Sends input/shutdown to the PTY.
    notifier: PaneNotifier,
    /// PTY reader thread join handle (detached on drop).
    #[allow(
        dead_code,
        reason = "RAII ownership — held to keep the reader thread joined to the pane's lifetime; detached on drop, never read"
    )]
    reader_thread: Option<JoinHandle<()>>,
    /// PTY writer thread join handle (detached on drop).
    #[allow(
        dead_code,
        reason = "RAII ownership — held to keep the writer thread joined to the pane's lifetime; detached on drop, never read"
    )]
    writer_thread: Option<JoinHandle<()>>,
    /// Terminal IO thread handle — all terminal access goes through commands.
    ///
    /// Drops cleanly on pane close via `PaneIoHandle::Drop`, which sends
    /// `Shutdown` and joins the thread.
    io_handle: PaneIoHandle,
    /// Lock-free selection-dirty flag (set by IO thread, read/cleared by main thread).
    io_selection_dirty: Arc<AtomicBool>,
    /// Spawned or adopted PTY (reader/writer/control taken; child lifecycle
    /// dispatched through [`PtyLifecycle`]).
    pty: Box<dyn PtyLifecycle + Send>,
    /// Lock-free cache of `TermMode::bits()` for hot-path queries.
    ///
    /// Shared with the IO thread — the IO thread writes after each VTE parse,
    /// the main thread reads for mouse reporting and cursor style.
    mode_cache: Arc<AtomicU64>,
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
    /// Write-stall detection flag (shared with writer thread).
    ///
    /// Set by the writer thread before a potentially-blocking `write()`,
    /// cleared after. The main thread checks this when the user presses
    /// Ctrl+C — if stalled, it sends SIGINT directly to the child process
    /// group, bypassing the blocked PTY writer.
    write_stalled: Arc<AtomicBool>,
    /// Child process ID (fallback for signal delivery on Windows).
    child_pid: Option<u32>,
    /// Dup'd PTY master fd for `tcgetpgrp()` (Unix only).
    ///
    /// The original master fd is owned by the IO thread via `PtyControl`.
    /// This is a `dup()`'d copy so we can query the PTY's foreground
    /// process group from the main thread without locking the IO thread.
    /// Used by [`signal_child`](Self::signal_child) on Unix to route
    /// SIGINT to the foreground job (e.g. `yes`, `cat`) rather than the
    /// shell's process group.
    #[cfg(unix)]
    master_fd: Option<std::os::unix::io::OwnedFd>,
}

impl Pane {
    /// Construct a pane from pre-built parts.
    ///
    /// Called by `LocalDomain::spawn_pane` after setting up the PTY pipeline.
    pub fn from_parts(parts: PaneParts) -> Self {
        let child_pid = parts.pty.process_id();
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
            has_unseen_output: false,
            selection: None,
            mark_cursor: None,
            search: None,
            search_active: Arc::new(AtomicBool::new(false)),
            write_stalled: parts.write_stalled,
            child_pid,
            #[cfg(unix)]
            master_fd: parts.master_fd,
        }
    }

    // -- Identity --

    /// Pane identity.
    pub fn id(&self) -> PaneId {
        self.id
    }

    /// Which domain spawned this pane.
    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    /// Child process ID, if known.
    ///
    /// For spawned panes, this is the shell's PID. For adopted panes
    /// (Section 03.9 Windows handoff), this is the client process ID
    /// reported by the console host. Used for direct signal delivery
    /// when the PTY writer is stalled.
    pub fn process_id(&self) -> Option<u32> {
        self.child_pid
    }

    // -- Lock-free accessors --

    /// Current terminal mode bits (lock-free).
    ///
    /// Updated by the IO thread after each VTE chunk; read by the main
    /// thread for mouse reporting and cursor style without locking.
    pub fn mode(&self) -> u64 {
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
    /// Used for all terminal state mutations: scroll, theme, cursor
    /// shape, search, text extraction, etc. Resize is routed
    /// separately through [`Self::send_resize`] (atomic coalescing
    /// slot) — it never traverses the bounded command channel.
    pub fn send_io_command(&self, cmd: PaneIoCommand) {
        self.io_handle.send_command(cmd);
    }

    /// Request a grid + PTY resize.
    ///
    /// Routed through the IO thread's atomic `pending_resize` slot
    /// rather than the bounded command channel. Last-writer-wins:
    /// during drag-resize-during-flood, only the latest dimensions
    /// are applied. Never blocks; never drops state.
    pub fn send_resize(&self, rows: u16, cols: u16) {
        self.io_handle.send_resize(rows, cols);
    }

    /// Borrow the pane's IO thread handle.
    ///
    /// Crate-internal access so the embedded backend can call the
    /// `fulfill_clipboard_load` / `fulfill_color_query` helpers to
    /// signal a fulfilled host-request response.
    pub(crate) fn io_handle(&self) -> &PaneIoHandle {
        &self.io_handle
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
    pub fn last_command_duration(&self) -> Option<std::time::Duration> {
        self.last_command_duration
    }

    /// Store the duration of a completed command.
    pub fn set_last_command_duration(&mut self, duration: std::time::Duration) {
        self.last_command_duration = Some(duration);
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

    /// Whether the PTY writer thread is blocked on a `write()` call.
    ///
    /// When `true`, the kernel PTY buffer is full (the child isn't reading
    /// stdin). Keyboard input queued via [`write_input`](Self::write_input)
    /// won't reach the child until the buffer drains. Use
    /// [`signal_child`](Self::signal_child) to send SIGINT directly.
    pub fn is_write_stalled(&self) -> bool {
        self.write_stalled.load(Ordering::Acquire)
    }

    /// Send a signal directly to the child process group.
    ///
    /// Bypasses the PTY writer when it's stalled. On Unix, queries the
    /// PTY's foreground process group via `tcgetpgrp(master_fd)` so the
    /// signal kills the foreground job (e.g. `yes`, `cat`) — matching
    /// the kernel's behavior for keyboard Ctrl+C through a non-stalled
    /// PTY. Falls back to the shell's process group when no master fd
    /// is available (handoff/adopted PTY) or `tcgetpgrp` returns no
    /// foreground group. On Windows, uses `GenerateConsoleCtrlEvent`
    /// for `CTRL_C_EVENT`.
    ///
    /// Returns `true` if the signal was sent (or the foreground job
    /// already exited between lookup and delivery), `false` if the
    /// child PID is unknown or the syscall failed.
    pub fn signal_child(&self, signal: Signal) -> bool {
        self.send_signal_platform(signal)
    }
}

/// Cross-platform signal type for direct child process signaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Interrupt (Ctrl+C) — `SIGINT` on Unix, `CTRL_C_EVENT` on Windows.
    Interrupt,
}

/// Resolve the target process group for a signal delivery (Unix only).
///
/// Returns `(pgid, resolved_via_tcgetpgrp)`. The boolean indicates whether
/// the PGID came from a positive `tcgetpgrp` result (`true`) or from the
/// shell-PID fallback (`false`). Callers gate ESRCH-as-success behavior
/// on the boolean AND on `pgid != shell_pid`: a distinct foreground job
/// that exited between lookup and signal IS the desired outcome (`true`);
/// a missing shell — whether reached via tcgetpgrp returning the shell
/// PID (no job control) or via the fallback path — IS a real failure
/// (`false`).
#[cfg(unix)]
#[allow(unsafe_code, reason = "libc::tcgetpgrp requires unsafe FFI call")]
fn resolve_target_pgid(
    pid: u32,
    master_fd: Option<&std::os::unix::io::OwnedFd>,
) -> (libc::pid_t, bool) {
    use std::os::unix::io::AsRawFd;
    if let Some(fd) = master_fd {
        // SAFETY: tcgetpgrp is a standard POSIX syscall. The dup'd master
        // fd is owned by Pane for its full lifetime (per
        // domain/local.rs:111-121). Result <= 0 means no foreground group
        // is set on the PTY.
        let pgid = unsafe { libc::tcgetpgrp(fd.as_raw_fd()) };
        if pgid > 0 {
            return (pgid, true);
        }
    }
    (pid as libc::pid_t, false)
}

#[cfg(unix)]
impl Pane {
    /// Unix signal delivery via tcgetpgrp-resolved foreground PGID.
    #[allow(unsafe_code, reason = "libc::kill requires unsafe FFI call")]
    fn send_signal_platform(&self, signal: Signal) -> bool {
        let Some(pid) = self.child_pid else {
            return false;
        };
        let sig = match signal {
            Signal::Interrupt => libc::SIGINT,
        };
        let (target_pgid, resolved_via_tcgetpgrp) =
            resolve_target_pgid(pid, self.master_fd.as_ref());
        // SAFETY: kill() is a standard POSIX syscall. Negative PID targets
        // the process group identified by target_pgid.
        let result = unsafe { libc::kill(-target_pgid, sig) };
        if result == 0 {
            return true;
        }
        let err = std::io::Error::last_os_error();
        // ESRCH-as-success applies ONLY when the resolved PGID is BOTH
        // sourced from a positive tcgetpgrp call AND distinct from the
        // shell PID. That distinguishes "the foreground job we wanted to
        // interrupt has already exited" (success) from "the shell itself
        // is gone" (failure — covers both the no-job-control case where
        // tcgetpgrp returned the shell PID and the no-master-fd fallback
        // path). EPERM and other errno values fall through to log::warn +
        // false; we never retry against the shell PID.
        let pgid_distinct_from_shell = target_pgid != pid as libc::pid_t;
        if resolved_via_tcgetpgrp
            && pgid_distinct_from_shell
            && err.raw_os_error() == Some(libc::ESRCH)
        {
            return true;
        }
        log::warn!("kill(-{target_pgid}, {sig}) failed: {err}");
        false
    }
}

#[cfg(windows)]
impl Pane {
    /// Windows signal delivery via `GenerateConsoleCtrlEvent`.
    #[allow(
        unsafe_code,
        reason = "GenerateConsoleCtrlEvent requires unsafe FFI call"
    )]
    fn send_signal_platform(&self, signal: Signal) -> bool {
        use windows_sys::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};

        let Some(pid) = self.child_pid else {
            return false;
        };
        let event = match signal {
            Signal::Interrupt => CTRL_C_EVENT,
        };
        // SAFETY: GenerateConsoleCtrlEvent is a standard Win32 console API.
        // The PID comes from portable-pty's Child::process_id().
        let result = unsafe { GenerateConsoleCtrlEvent(event, pid) };
        if result == 0 {
            log::warn!(
                "GenerateConsoleCtrlEvent({event}, {pid}) failed: {}",
                std::io::Error::last_os_error()
            );
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests;
