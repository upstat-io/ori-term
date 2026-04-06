//! Protocol data unit definitions.
//!
//! [`MuxPdu`] is the unified enum covering all messages: requests from client
//! to daemon, responses from daemon to client, and push notifications from
//! daemon to client. Each variant maps to a [`MsgType`] ID.

use serde::{Deserialize, Serialize};

use crate::id::{ClientId, PaneId};

use super::snapshot::{PaneSnapshot, WireSelection};

/// Client supports receiving `NotifyPaneSnapshot` pushed snapshots.
pub const CAP_SNAPSHOT_PUSH: u32 = 1;

/// All protocol messages — requests, responses, and notifications.
///
/// Each variant carries its own data. The bincode encoding includes the
/// enum discriminant, so the `msg_type` in the frame header is redundant
/// for deserialization but useful for pre-routing and debugging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuxPdu {
    // -- Requests (client → daemon) --
    /// Client handshake. Sent immediately after connecting.
    Hello {
        /// OS process ID of the connecting client.
        pid: u32,
        /// Protocol version the client speaks.
        protocol_version: u8,
        /// Feature flags the client supports (bitmask).
        features: u64,
    },

    /// Close a single pane.
    ClosePane {
        /// Pane to close.
        pane_id: PaneId,
    },

    /// Write input data to a pane's PTY. Fire-and-forget.
    Input {
        /// Target pane.
        pane_id: PaneId,
        /// Raw bytes to write.
        data: Vec<u8>,
    },

    /// Send a signal to a pane's child process group. Fire-and-forget.
    ///
    /// Bypasses the PTY writer when it's stalled (kernel buffer full).
    /// Used for Ctrl+C delivery during output flooding.
    SignalChild {
        /// Target pane.
        pane_id: PaneId,
        /// Signal to send (maps to `Signal` enum).
        signal: u8,
    },

    /// Resize a pane's terminal grid. Fire-and-forget.
    Resize {
        /// Target pane.
        pane_id: PaneId,
        /// New column count.
        cols: u16,
        /// New row count.
        rows: u16,
    },

    /// Subscribe to a pane's output. Returns current snapshot.
    Subscribe {
        /// Pane to subscribe to.
        pane_id: PaneId,
    },

    /// Unsubscribe from a pane's output.
    Unsubscribe {
        /// Pane to unsubscribe from.
        pane_id: PaneId,
    },

    /// Get a full snapshot of a pane's state.
    GetPaneSnapshot {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Liveness check. The daemon replies with [`PingAck`](Self::PingAck).
    Ping,

    /// Request graceful daemon shutdown. The daemon replies with
    /// [`ShutdownAck`](Self::ShutdownAck) and then exits.
    Shutdown,

    /// Scroll a pane's viewport by `delta` lines (positive = toward history).
    /// Fire-and-forget.
    ScrollDisplay {
        /// Target pane.
        pane_id: PaneId,
        /// Lines to scroll (positive = up into scrollback, negative = down).
        delta: i32,
    },

    /// Scroll a pane to the live terminal position (bottom). Fire-and-forget.
    ScrollToBottom {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Scroll to the nearest prompt in the given direction.
    ScrollToPrompt {
        /// Target pane.
        pane_id: PaneId,
        /// Direction: `-1` = previous (up), `+1` = next (down).
        direction: i8,
    },

    /// Set the theme and palette for a pane. Fire-and-forget.
    SetTheme {
        /// Target pane.
        pane_id: PaneId,
        /// Theme name: `"dark"` or `"light"`.
        theme: String,
        /// Full palette as 270 RGB triplets (same format as snapshot).
        palette_rgb: Vec<[u8; 3]>,
    },

    /// Set the cursor shape for a pane. Fire-and-forget.
    SetCursorShape {
        /// Target pane.
        pane_id: PaneId,
        /// Cursor shape discriminant (maps to `WireCursorShape`).
        shape: u8,
    },

    /// Set bold-as-bright behavior for a pane. Fire-and-forget.
    SetBoldIsBright {
        /// Target pane.
        pane_id: PaneId,
        /// Whether bold promotes ANSI colors to bright.
        enabled: bool,
    },

    /// Mark all grid lines dirty in a pane (forces full re-render).
    /// Fire-and-forget.
    MarkAllDirty {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Open search for a pane (initializes empty search state).
    /// Fire-and-forget.
    OpenSearch {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Close search and clear search state. Fire-and-forget.
    CloseSearch {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Update the search query. Recomputes matches against the full grid.
    /// Fire-and-forget.
    SearchSetQuery {
        /// Target pane.
        pane_id: PaneId,
        /// New search query text.
        query: String,
    },

    /// Navigate to the next search match. Fire-and-forget.
    SearchNextMatch {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Navigate to the previous search match. Fire-and-forget.
    SearchPrevMatch {
        /// Target pane.
        pane_id: PaneId,
    },

    /// Extract plain text from a selection.
    ExtractText {
        /// Target pane.
        pane_id: PaneId,
        /// Selection to extract from.
        selection: WireSelection,
    },

    /// Extract HTML and plain text from a selection.
    ExtractHtml {
        /// Target pane.
        pane_id: PaneId,
        /// Selection to extract from.
        selection: WireSelection,
        /// Font family name for the HTML wrapper.
        font_family: String,
        /// Font size in points × 100 (integer for deterministic comparison).
        font_size_x100: u16,
    },

    /// Client advertises protocol capabilities. Fire-and-forget.
    SetCapabilities {
        /// Bitmask of capability flags (e.g. [`CAP_SNAPSHOT_PUSH`]).
        flags: u32,
    },

    /// Spawn a new pane (shell process).
    SpawnPane {
        /// Shell program override (uses default shell if `None`).
        shell: Option<String>,
        /// Working directory override (uses current dir if `None`).
        cwd: Option<String>,
        /// Color theme: `"dark"`, `"light"`, or `None` for server default.
        theme: Option<String>,
    },

    /// Request that other clients create a new tab.
    ///
    /// The daemon broadcasts [`NotifyNewTab`](Self::NotifyNewTab) to all
    /// other connected clients and replies with [`NewTabAck`](Self::NewTabAck).
    RequestNewTab,

    /// Set the push priority for a pane (affects push interval).
    ///
    /// Fire-and-forget: no response expected. Priority values:
    /// - `0` = focused (4ms push)
    /// - `1` = visible unfocused (16ms push)
    /// - `2` = hidden (100ms push)
    SetPanePriority {
        /// Pane to set priority for.
        pane_id: PaneId,
        /// Priority level (0=focused, 1=visible, 2=hidden).
        priority: u8,
    },

    /// List all live pane IDs.
    ListPanes,

    /// Configure image protocol settings for a pane. Fire-and-forget.
    SetImageConfig {
        /// Target pane.
        pane_id: PaneId,
        /// Whether image protocols are enabled.
        enabled: bool,
        /// CPU-side image cache memory limit in bytes.
        memory_limit: u64,
        /// Maximum single image size in bytes.
        max_single: u64,
        /// Whether animated images play their frames.
        animation_enabled: bool,
    },

    // -- Responses (daemon → client) --
    /// Handshake acknowledgment.
    HelloAck {
        /// Assigned client ID for this connection.
        client_id: ClientId,
        /// Protocol version the server speaks.
        protocol_version: u8,
        /// Negotiated feature flags (intersection of client + server).
        features: u64,
    },

    /// Pane closed successfully.
    PaneClosedAck,

    /// Subscription established with current pane state.
    Subscribed {
        /// Current state of the subscribed pane.
        snapshot: PaneSnapshot,
    },

    /// Unsubscription confirmed.
    Unsubscribed,

    /// Full pane state snapshot.
    PaneSnapshotResp {
        /// Pane state.
        snapshot: PaneSnapshot,
    },

    /// Reply to a [`Ping`](Self::Ping) request.
    PingAck,

    /// Acknowledgment that the daemon will shut down.
    ShutdownAck,

    /// Response to [`ScrollToPrompt`](Self::ScrollToPrompt).
    ScrollToPromptAck {
        /// Whether a prompt was found and the viewport scrolled.
        scrolled: bool,
    },

    /// Response to [`ExtractText`](Self::ExtractText).
    ExtractTextResp {
        /// Extracted plain text.
        text: String,
    },

    /// Response to [`ExtractHtml`](Self::ExtractHtml).
    ExtractHtmlResp {
        /// Extracted HTML with inline styles.
        html: String,
        /// Plain text (same as `ExtractTextResp::text`).
        text: String,
    },

    /// Response to [`SpawnPane`](Self::SpawnPane).
    SpawnPaneResponse {
        /// ID of the newly created pane.
        pane_id: PaneId,
    },

    /// Response to [`ListPanes`](Self::ListPanes).
    ListPanesResponse {
        /// IDs of all live panes.
        pane_ids: Vec<PaneId>,
    },

    /// Acknowledgment for [`RequestNewTab`](Self::RequestNewTab).
    NewTabAck,

    /// Error response for a failed request.
    Error {
        /// Human-readable error description.
        message: String,
    },

    // -- Push notifications (daemon → client) --
    /// Pane has new output — the client should re-fetch the snapshot.
    NotifyPaneOutput {
        /// Pane with new output.
        pane_id: PaneId,
    },

    /// Pane's shell process exited.
    NotifyPaneExited {
        /// Pane that exited.
        pane_id: PaneId,
        /// Exit code from the child process (0 = clean exit).
        exit_code: i32,
    },

    /// Pane metadata changed (title, icon name, or CWD).
    NotifyPaneMetadataChanged {
        /// Pane with updated metadata.
        pane_id: PaneId,
        /// Current title text.
        title: String,
    },

    /// Bell fired in a pane.
    NotifyPaneBell {
        /// Pane that belled.
        pane_id: PaneId,
    },

    /// A command completed in a pane (OSC 133;D → duration).
    NotifyCommandComplete {
        /// Pane that completed a command.
        pane_id: PaneId,
        /// Command duration in milliseconds.
        duration_ms: u64,
    },

    /// OSC 52 clipboard store request forwarded from a pane.
    NotifyClipboardStore {
        /// Originating pane.
        pane_id: PaneId,
        /// Clipboard discriminant: 0 = Clipboard, 1 = Selection.
        clipboard_type: u8,
        /// Text to store.
        text: String,
    },

    /// OSC 52 clipboard load request forwarded from a pane.
    ///
    /// The `formatter` closure from the event is not serializable — the
    /// receiving client applies its own formatting when responding.
    NotifyClipboardLoad {
        /// Originating pane.
        pane_id: PaneId,
        /// Clipboard discriminant: 0 = Clipboard, 1 = Selection.
        clipboard_type: u8,
    },

    /// Another client requested a new tab. The receiving client should
    /// create a new tab in its active window.
    NotifyNewTab,

    /// Server-pushed pane snapshot (proactive, throttled to ~250fps / 4ms).
    ///
    /// Only sent to clients that advertised [`CAP_SNAPSHOT_PUSH`].
    NotifyPaneSnapshot {
        /// Pane this snapshot belongs to.
        pane_id: PaneId,
        /// Full pane state snapshot.
        snapshot: PaneSnapshot,
    },
    // Wire-compat: append-only — new variants must go at the end.
}

// `msg_type()`, `is_fire_and_forget()`, `is_notification()`, and
// `theme_to_wire()` live in sibling `pdu_traits.rs` (split for 500-line limit).
pub(crate) use super::pdu_traits::theme_to_wire;
