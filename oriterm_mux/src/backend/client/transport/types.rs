//! Shared types for the client transport: `SendRequest` and
//! `PendingClientReply`. Extracted from `mod.rs` to keep the module under
//! the 500-line file-organization limit.

use std::sync::mpsc;

use crate::protocol::MuxPdu;

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

/// A request queued for the writer thread to send.
pub(in crate::backend::client) struct SendRequest {
    /// Sequence number assigned by the transport.
    pub(in crate::backend::client) seq: u32,
    /// PDU to encode and write.
    pub(in crate::backend::client) pdu: MuxPdu,
    /// Reply channel. `None` for fire-and-forget messages.
    pub(in crate::backend::client) reply_tx: Option<mpsc::Sender<MuxPdu>>,
}
