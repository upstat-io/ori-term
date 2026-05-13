//! Mux backend abstraction.
//!
//! [`MuxBackend`] defines the interface between the client app and the
//! multiplexer state. Two implementations exist:
//!
//! - [`EmbeddedMux`] — in-process mux for single-process mode. Wraps
//!   [`InProcessMux`](crate::in_process::InProcessMux) and owns `Pane` structs directly.
//! - [`MuxClient`] — IPC client for daemon mode. Sends requests to a
//!   [`MuxServer`](crate::server::MuxServer) over a Unix socket / named pipe.

pub mod client;
pub mod embedded;
mod wakeup;

use std::io;
use std::sync::mpsc;

use oriterm_core::Theme;
use oriterm_core::color::Rgb;
use oriterm_core::effect::ResponseToken;
use oriterm_core::selection::Selection;

use crate::PaneSnapshot;
use crate::domain::SpawnConfig;
use crate::in_process::ClosePaneResult;
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::MarkCursor;
use crate::registry::PaneEntry;
use crate::{DomainId, PaneId};

pub use self::client::MuxClient;
pub use self::embedded::EmbeddedMux;

/// Payload for [`MuxBackend::fulfill_host_request`].
///
/// Carries the `ResponseToken` the main thread extracted from a
/// `MuxNotification::HostClipboardLoad` / `HostColorQuery`, paired with
/// the value the main thread resolved (clipboard text, palette color).
/// The embedded backend forwards the fulfillment to the owning pane's
/// `PaneIoHandle`; the daemon backend rejects today and will gain a
/// reply-PDU wire in a follow-up plan.
#[derive(Debug)]
pub enum HostReply {
    /// Reply to an OSC 52 clipboard read request.
    ClipboardLoad {
        /// Token carried by the originating notification.
        token: ResponseToken<String>,
        /// Clipboard text read by the main thread.
        text: String,
    },
    /// Reply to an OSC color query.
    ColorQuery {
        /// Token carried by the originating notification.
        token: ResponseToken<Rgb>,
        /// Resolved `Rgb` value.
        color: Rgb,
    },
}

/// Image protocol configuration for a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageConfig {
    /// Whether image protocols are enabled.
    pub enabled: bool,
    /// CPU-side image cache memory limit in bytes.
    pub memory_limit: usize,
    /// Maximum single image size in bytes.
    pub max_single: usize,
    /// Whether animated images play their frames.
    pub animation_enabled: bool,
}

/// Per-pane parameters for [`MuxBackend::adopt_pane`].
///
/// Bundles the terminal dimensions, scrollback size, theme, and any
/// startup metadata so the trait method stays under the hygiene rule's
/// argument limit. The `AdoptedPtyHandle` is passed separately because
/// callers typically own it via move semantics already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptPaneRequest {
    /// Initial terminal rows (typically from `TERMINAL_STARTUP_INFO.dwYCountChars`).
    pub rows: u16,
    /// Initial terminal columns (typically from `TERMINAL_STARTUP_INFO.dwXCountChars`).
    pub cols: u16,
    /// Scrollback buffer size in lines (from the user's config).
    pub scrollback: usize,
    /// Color theme for the new pane.
    pub theme: Theme,
    /// Initial pane title (typically from `TERMINAL_STARTUP_INFO.pszTitle`,
    /// e.g. the title embedded in a `.lnk` shortcut). Empty string means
    /// "no explicit title" — the pane will fall back to its CWD-derived
    /// or shell-set title via the standard `Pane::effective_title` chain.
    pub initial_title: String,
    /// Initial pane icon name/path (typically from
    /// `TERMINAL_STARTUP_INFO.pszIconPath`, e.g. the icon embedded in a
    /// `.lnk` shortcut). `None` if the COM caller did not supply one.
    /// Stored on the pane via `Pane::set_icon_name` and consumed by the
    /// tab bar to render a per-pane icon.
    pub initial_icon: Option<String>,
}

/// Abstraction over in-process and daemon-mode multiplexer access.
///
/// The App calls trait methods identically regardless of whether
/// terminal state lives in-process ([`EmbeddedMux`]) or in a remote
/// daemon ([`MuxClient`]). All methods are synchronous.
pub trait MuxBackend {
    // Event pump

    /// Whether a PTY wakeup has arrived since the last `poll_events` call.
    ///
    /// Used by the event loop to skip `poll_events` when no PTY activity
    /// has occurred. Conservative default: always returns `true` so
    /// existing code compiles before both backends implement.
    fn has_pending_wakeup(&self) -> bool {
        true
    }

    /// Drain `MuxEvent`s from PTY reader threads and emit notifications.
    ///
    /// In embedded mode, this processes the mpsc channel. In client mode,
    /// this is a no-op (the reader thread pushes directly).
    fn poll_events(&mut self);

    /// Drain accumulated notifications into the caller's buffer.
    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>);

    /// Discard all pending notifications.
    fn discard_notifications(&mut self);

    /// Look up a pane's metadata entry.
    fn get_pane_entry(&self, pane_id: PaneId) -> Option<PaneEntry>;

    // Pane operations

    /// Spawn a pane with a new PTY process.
    ///
    /// The client owns tab/window grouping — the mux creates the pane
    /// and manages its PTY lifecycle. Returns `PaneId` for the new pane.
    fn spawn_pane(&mut self, config: &SpawnConfig, theme: Theme) -> io::Result<PaneId>;

    /// Adopt a pane from a Windows console handoff.
    ///
    /// Wraps the pre-existing PTY handles delivered by `conhost.exe`'s
    /// `ITerminalHandoff3::EstablishPtyHandoff` callback (Section 03.9
    /// Phase 3) into a [`Pane`](crate::pane::Pane). Embedded mux uses
    /// [`InProcessMux::adopt_standalone_pane`](crate::in_process::InProcessMux::adopt_standalone_pane);
    /// daemon mode rejects the call because the COM server is a
    /// `REGCLS_SINGLEUSE` standalone process that cannot relay handoffs
    /// over IPC.
    ///
    /// Default impl returns `Err(io::Error::other("not supported"))`
    /// so non-handoff backends compile without changes.
    fn adopt_pane(
        &mut self,
        _adopted: crate::pty::AdoptedPtyHandle,
        _request: AdoptPaneRequest,
    ) -> io::Result<PaneId> {
        Err(io::Error::other(
            "default-terminal handoff requires embedded mux mode",
        ))
    }

    /// Close a single pane.
    fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult;

    // Grid operations

    /// Resize a pane's terminal grid and PTY.
    ///
    /// In embedded mode, resizes the old Term for dual-Term consistency and
    /// sends a `Resize` command to the IO thread (which does reflow + PTY
    /// resize). In daemon mode, sends a fire-and-forget `Resize` PDU.
    fn resize_pane_grid(&mut self, pane_id: PaneId, rows: u16, cols: u16);

    // Mode query

    /// Terminal mode bits for a pane (raw `u64`).
    ///
    /// In embedded mode, reads the lock-free atomic cache.
    /// In daemon mode, reads from the cached snapshot. Widened to
    /// `u64` in plan `plans/spec-conformance/section-08` §08.3 for
    /// DECLRMM (mode 69 = bit 32).
    fn pane_mode(&self, pane_id: PaneId) -> Option<u64>;

    // Theme + palette + cursor operations

    /// Apply a theme and palette to a pane's terminal.
    fn set_pane_theme(&mut self, pane_id: PaneId, theme: Theme, palette: oriterm_core::Palette);

    /// Change the cursor shape for a pane.
    fn set_cursor_shape(&mut self, pane_id: PaneId, shape: oriterm_core::CursorShape);

    /// Set whether bold text promotes ANSI colors 0–7 to bright 8–15.
    fn set_bold_is_bright(&mut self, pane_id: PaneId, enabled: bool);

    /// Update the ENQ answerback string emitted on `\x05` per ECMA-48
    /// §8.3.40 + `WezTerm` parity (`term/src/terminalstate/performer.rs:473-478`).
    ///
    /// Empty bytes (default) suppress emission. Non-empty bytes are
    /// written verbatim to the PTY on each ENQ byte received.
    /// Implementations MUST NOT mark the pane snapshot dirty —
    /// answerback affects only the ENQ outbound write and never
    /// changes any rendered cell.
    fn set_answerback(&mut self, pane_id: PaneId, bytes: Vec<u8>);

    /// Mark all lines in a pane as dirty (forces full re-render).
    fn mark_all_dirty(&mut self, pane_id: PaneId);

    /// Apply image protocol configuration to a pane.
    fn set_image_config(&mut self, pane_id: PaneId, config: ImageConfig);

    /// Propagate cell pixel dimensions to a pane's terminal so
    /// `FixedPixels` image placements compute correct cell coverage
    /// after font-size or DPI changes. Separate from `set_image_config`
    /// because cell metrics are runtime state (font rasterization
    /// output), not static TOML config.
    fn set_cell_dimensions(&mut self, pane_id: PaneId, width: u16, height: u16);

    // Scroll operations

    /// Scroll the viewport by `delta` lines (positive = toward history).
    fn scroll_display(&mut self, pane_id: PaneId, delta: isize);

    /// Scroll to the live terminal position (bottom).
    fn scroll_to_bottom(&mut self, pane_id: PaneId);

    /// Scroll to the nearest prompt above the current viewport.
    fn scroll_to_previous_prompt(&mut self, pane_id: PaneId);

    /// Scroll to the nearest prompt below the current viewport.
    fn scroll_to_next_prompt(&mut self, pane_id: PaneId);

    // Search operations

    /// Open search for a pane (initializes empty search state).
    fn open_search(&mut self, pane_id: PaneId);

    /// Close search and clear search state.
    fn close_search(&mut self, pane_id: PaneId);

    /// Update the search query. Recomputes matches against the full grid.
    fn search_set_query(&mut self, pane_id: PaneId, query: String);

    /// Navigate to the next search match.
    fn search_next_match(&mut self, pane_id: PaneId);

    /// Navigate to the previous search match.
    fn search_prev_match(&mut self, pane_id: PaneId);

    /// Whether search is currently active for a pane.
    fn is_search_active(&self, pane_id: PaneId) -> bool;

    // Clipboard text extraction

    /// Extract plain text from a selection range.
    ///
    /// Returns `None` if the pane doesn't exist or the selection is empty.
    fn extract_text(&mut self, pane_id: PaneId, selection: &Selection) -> Option<String>;

    /// Extract HTML (with inline styles) and plain text from a selection.
    ///
    /// `font_family` and `font_size` are used for the HTML wrapper.
    /// Returns `None` if the pane doesn't exist or the selection is empty.
    fn extract_html(
        &mut self,
        pane_id: PaneId,
        selection: &Selection,
        font_family: &str,
        font_size: f32,
    ) -> Option<(String, String)>;

    // Input

    /// Send raw bytes to a pane's PTY.
    ///
    /// In embedded mode, delegates to [`Pane::write_input`].
    /// In daemon mode, sends a fire-and-forget `Input` PDU to the daemon.
    fn send_input(&mut self, pane_id: PaneId, data: &[u8]);

    /// Whether the PTY writer thread for a pane is blocked on a write.
    ///
    /// When `true`, the kernel PTY buffer is full and keyboard input
    /// queued via [`send_input`](Self::send_input) won't reach the child.
    /// Use [`signal_child`](Self::signal_child) to send Ctrl+C directly.
    ///
    /// Receiver is `&mut self` because the daemon backend round-trips this
    /// query through the IPC transport (the transport's reply-channel
    /// allocation requires `&mut self`); the embedded backend's body only
    /// reads an `AtomicBool` and is `&mut self`-safe trivially.
    fn is_write_stalled(&mut self, _pane_id: PaneId) -> bool {
        false
    }

    /// Send a signal directly to a pane's child process group.
    ///
    /// Bypasses the PTY writer when stalled. Returns `true` if sent.
    fn signal_child(&mut self, _pane_id: PaneId, _signal: crate::Signal) -> bool {
        false
    }

    // Pane metadata

    /// Current working directory of a pane (from OSC 7).
    ///
    /// Borrows from the cached snapshot's `cwd` field.
    fn pane_cwd(&self, pane_id: PaneId) -> Option<&str> {
        self.pane_snapshot(pane_id).and_then(|s| s.cwd.as_deref())
    }

    /// Record a bell on a pane in the backend's local `bell_panes` set.
    ///
    /// Called by the App's `MuxNotification::PaneBell` /
    /// `DesktopNotification` / `CommandComplete` arms after the focus
    /// gate decides the bell is "background-worthy." Both `EmbeddedMux`
    /// and `MuxClient` override this with a `bell_panes.insert`. The
    /// trait default is a no-op for any future backend that opts out.
    fn set_bell(&mut self, _pane_id: PaneId) {}

    /// Clear the bell on a pane.
    ///
    /// Called from the focused-pane bell arm and the focus-change clear
    /// sweep. Both real backends override with `bell_panes.remove`; the
    /// trait default is a no-op.
    fn clear_bell(&mut self, _pane_id: PaneId) {}

    /// Whether the bell is currently active for a pane.
    ///
    /// Default fallback for backends that don't override. Both
    /// `EmbeddedMux` and `MuxClient` override this with their local
    /// `bell_panes` set — bells are client-local UI state, not
    /// server-replicated. The default returns `false` because no
    /// snapshot field carries bell state any more.
    fn has_bell(&self, _pane_id: PaneId) -> bool {
        false
    }

    /// Mark a pane as having unseen output.
    ///
    /// Called when a non-active pane receives output. The tab bar reads this
    /// via the snapshot to show a "modified" indicator dot.
    fn set_unseen_output(&mut self, _pane_id: PaneId) {}

    /// Clear the unseen output flag for a pane.
    ///
    /// Called when a pane becomes the active/focused tab.
    fn mark_output_seen(&mut self, _pane_id: PaneId) {}

    /// Whether a pane has output the user hasn't seen.
    ///
    /// In embedded mode, reads directly from the [`Pane`] (avoids snapshot
    /// staleness). In daemon mode, reads from the cached pushed snapshot.
    fn has_unseen_output(&self, pane_id: PaneId) -> bool {
        self.pane_snapshot(pane_id)
            .is_some_and(|s| s.has_unseen_output)
    }

    /// Clean up a closed pane's resources.
    ///
    /// In embedded mode, removes the pane from storage and drops it on a
    /// background thread (PTY kill, reader join, child reap). In client
    /// mode this is a no-op — the daemon owns pane resources.
    fn cleanup_closed_pane(&mut self, _pane_id: PaneId) {}

    /// Build a `Selection` covering the nearest command output zone.
    ///
    /// Uses shell integration markers to find the output region around
    /// the viewport center. Returns `None` if no zone is found or shell
    /// integration is not active.
    fn select_command_output(&self, _pane_id: PaneId) -> Option<Selection> {
        None
    }

    /// Build a `Selection` covering the nearest command input zone.
    ///
    /// Uses shell integration markers to find the input region around
    /// the viewport center. Returns `None` if no zone is found or shell
    /// integration is not active.
    fn select_command_input(&self, _pane_id: PaneId) -> Option<Selection> {
        None
    }

    /// Enter mark mode: scrolls to bottom on the IO thread, returns the
    /// cursor position as a `MarkCursor`. The IO thread owns the authoritative
    /// terminal state, so cursor reads must go through this reply path.
    fn enter_mark_mode(&mut self, pane_id: PaneId) -> Option<MarkCursor> {
        let _ = pane_id;
        None
    }

    /// All pane IDs currently stored in the backend.
    fn pane_ids(&self) -> Vec<PaneId>;

    /// Subscribe to a pane's notification stream.
    ///
    /// In daemon mode, sends a `Subscribe` PDU and caches the initial snapshot.
    /// In embedded mode, does nothing (already "subscribed" in-process).
    fn subscribe(&mut self, _pane_id: PaneId) -> io::Result<()> {
        Ok(())
    }

    /// Unsubscribe from a pane's notification stream.
    ///
    /// In daemon mode, sends an `Unsubscribe` PDU.
    fn unsubscribe(&mut self, _pane_id: PaneId) -> io::Result<()> {
        Ok(())
    }

    // Event channel

    /// Event sender for spawning new panes (embedded: mpsc; client: None).
    fn event_tx(&self) -> Option<&mpsc::Sender<MuxEvent>>;

    /// Default domain ID for spawning.
    fn default_domain(&self) -> DomainId;

    /// Whether the daemon connection is alive.
    ///
    /// Always `true` for embedded mode (no remote connection).
    /// In daemon mode, reflects the transport's liveness state.
    fn is_connected(&self) -> bool {
        true
    }

    /// Whether this backend is running in daemon (IPC client) mode.
    ///
    /// Embedded mode returns `false`. Client mode returns `true`.
    fn is_daemon_mode(&self) -> bool;

    // Snapshot access

    /// Swap the cached [`RenderableContent`] for a pane into `target`.
    ///
    /// In embedded mode, [`refresh_pane_snapshot`](Self::refresh_pane_snapshot)
    /// captures the `RenderableContent` extracted from the terminal. This
    /// method swaps it directly into the caller's `FrameInput.content`,
    /// bypassing the `RenderableContent → WireCell → RenderableContent`
    /// round-trip that the snapshot path requires.
    ///
    /// Returns `true` if the swap succeeded (embedded mode). Returns `false`
    /// in daemon mode (caller must use `pane_snapshot()` + conversion).
    fn swap_renderable_content(
        &mut self,
        _pane_id: PaneId,
        _target: &mut oriterm_core::RenderableContent,
    ) -> bool {
        false
    }

    /// Cached snapshot for a pane.
    ///
    /// Returns the most recently cached snapshot, or `None` if no snapshot
    /// has been built/fetched yet.
    fn pane_snapshot(&self, pane_id: PaneId) -> Option<&PaneSnapshot>;

    /// Look up decoded image pixel data for `(pane_id, image_id)`.
    ///
    /// Daemon-mode clients return their cached `Arc<RenderableImageData>` (cheap
    /// refcount clone). Embedded backends bypass the extract path entirely via
    /// `swap_renderable_content` so the default `None` is correct. Used by the
    /// extract path closure when a `WirePlacement` arrives without its
    /// `WireImageData` (the server filtered out the bytes because the client
    /// already has them).
    /// See: bug-tracker/plans/BUG-06-072/
    fn pane_image_data(
        &self,
        _pane_id: PaneId,
        _image_id: oriterm_core::ImageId,
    ) -> Option<std::sync::Arc<oriterm_core::RenderableImageData>> {
        None
    }

    /// Whether the cached snapshot for `pane_id` is stale.
    fn is_pane_snapshot_dirty(&self, pane_id: PaneId) -> bool;

    /// Build (embedded) or fetch (daemon) a fresh snapshot and cache it.
    fn refresh_pane_snapshot(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot>;

    /// Synchronously fetch a fresh snapshot, draining any in-flight IO
    /// commands first.
    ///
    /// Unlike [`refresh_pane_snapshot`](Self::refresh_pane_snapshot),
    /// which is fast-path push-driven and may return stale data, this
    /// method:
    /// 1. Sends a `SnapshotNow` IO command to the pane's IO thread.
    /// 2. Waits for the IO thread to process all earlier commands (FIFO)
    ///    and publish a fresh snapshot to the double buffer.
    /// 3. Returns the fresh snapshot, owned (no shared borrow).
    ///
    /// Used by tests and any caller that needs deterministic
    /// "scroll then read" semantics. Production render code should
    /// continue to use the async push pipeline via
    /// `refresh_pane_snapshot`.
    fn sync_pane_snapshot(&mut self, pane_id: PaneId) -> Option<PaneSnapshot>;

    /// Clear the dirty flag for a pane's cached snapshot.
    fn clear_pane_snapshot_dirty(&mut self, pane_id: PaneId);

    /// Whether the terminal's `selection_dirty` flag is set for a pane.
    ///
    /// The flag is set when terminal output modifies grid content that would
    /// invalidate a text selection (character printing, scrolling, erasing,
    /// etc.). It is NOT set by cursor movement, SGR changes, or other
    /// non-content-modifying operations.
    ///
    /// In embedded mode, reads the flag from the terminal. In daemon mode,
    /// returns `false` — the daemon propagates invalidation via snapshot
    /// changes instead.
    fn is_selection_dirty(&self, _pane_id: PaneId) -> bool {
        false
    }

    /// Clear the terminal's `selection_dirty` flag for a pane.
    ///
    /// Must be called after checking `is_selection_dirty()` to prevent
    /// the flag from being re-read on subsequent poll cycles.
    fn clear_selection_dirty(&mut self, _pane_id: PaneId) {}

    /// Shrink renderable content caches if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste. Default is a no-op
    /// (daemon mode doesn't cache `RenderableContent`).
    fn maybe_shrink_renderable_caches(&mut self) {}

    /// Fulfill a host-request response.
    ///
    /// Called by the main thread after it resolves the value for a
    /// `MuxNotification::HostClipboardLoad` / `HostColorQuery` token.
    /// The embedded backend looks up the pane's `PaneIoHandle` and
    /// signals the wake channel so the IO thread's `select!` wakes
    /// within one iteration. The daemon backend returns `Err` until a
    /// reply-PDU wire design lands (tracked separately — see
    /// `plans/effect-cutover/section-01-migrate-mux-consumer.md §01.4`).
    ///
    /// A duplicate fulfill is logged (the
    /// [`oriterm_core::effect::AlreadyFulfilled`] error is caught and
    /// collapsed to `Ok(())`) — routing bugs surface as log noise, not
    /// as IO errors, because the first fulfill is authoritative.
    fn fulfill_host_request(&mut self, _pane_id: PaneId, _reply: HostReply) -> io::Result<()> {
        Err(io::Error::other(
            "host-request fulfillment not supported on this backend",
        ))
    }
}
