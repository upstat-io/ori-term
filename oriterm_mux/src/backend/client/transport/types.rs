//! Shared types for the client transport: `SendRequest`,
//! `PendingClientReply`, and `ReaderThreadState`. Extracted from `mod.rs`
//! to keep the module under the 500-line file-organization limit.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use oriterm_core::effect::ResponseTokenId;

use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;
use crate::{PaneId, PaneSnapshot};

/// Bookkeeping for a host-request received from the daemon, awaiting the
/// App's `MuxBackend::fulfill_host_request` call ().
pub(in crate::backend::client) enum PendingClientReply {
    /// OSC 52 clipboard load — `request_id` echoed back in the reply PDU.
    Clipboard {
        /// Server-allocated id from the originating notification.
        request_id: u64,
    },
    /// OSC 4 / 10 / 11 / 12 color query.
    Color {
        /// Server-allocated id from the originating notification.
        request_id: u64,
    },
}

impl PendingClientReply {
    /// Echo-back `request_id` regardless of variant.
    pub(in crate::backend::client) fn request_id(&self) -> u64 {
        match self {
            Self::Clipboard { request_id } | Self::Color { request_id } => *request_id,
        }
    }
}

/// Shared state owned by the reader thread for the duration of its run.
///
/// Bundles the channels and `Arc`-shared handles the reader needs so the
/// reader-thread spawn + event loop pass a single owned value rather than a
/// long positional argument list. Every field moves into the spawned
/// reader thread and lives for its lifetime.
pub(super) struct ReaderThreadState {
    /// Notification channel to the main thread.
    pub(super) notif_tx: mpsc::Sender<MuxNotification>,
    /// Wakeup callback fired when push notifications arrive.
    pub(super) wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Connection-liveness flag, shared with the writer thread.
    pub(super) alive: Arc<AtomicBool>,
    /// Daemon-pushed snapshots keyed by pane, drained by the main thread.
    pub(super) pushed_snapshots: Arc<Mutex<HashMap<PaneId, PaneSnapshot>>>,
    /// Pending host-request replies keyed by local `ResponseToken` slot.
    pub(super) pending_replies: Arc<Mutex<HashMap<ResponseTokenId, PendingClientReply>>>,
    /// Pending RPC reply senders keyed by sequence number.
    pub(super) pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>,
    /// Outstanding health-check ping seq (`0` = none), shared with writer.
    pub(super) outstanding_ping_seq: Arc<AtomicU64>,
}

/// A request queued for the writer thread to send.
pub(in crate::backend::client) struct SendRequest {
    /// Sequence number assigned by the transport.
    pub(in crate::backend::client) seq: u32,
    /// PDU to encode and write.
    pub(in crate::backend::client) pdu: MuxPdu,
    /// Reply channel. `None` for fire-and-forget messages.
    pub(in crate::backend::client) reply_tx: Option<mpsc::Sender<MuxPdu>>,
}
