//! Client connection tracking.
//!
//! Each client process that connects to the daemon is tracked as a
//! [`ClientConnection`] with a unique [`ClientId`] and the mio [`Token`]
//! used for event dispatching.

use std::collections::{HashMap, HashSet};

use mio::Token;
use oriterm_core::ImageId;

use crate::MuxPdu;
use crate::PaneId;
use crate::id::ClientId;

use super::frame_io::{FrameReader, FrameWriter};
use super::ipc::IpcStream;

/// A connected client process.
pub(crate) struct ClientConnection {
    /// Unique connection identifier.
    id: ClientId,
    /// IPC stream to the client.
    stream: IpcStream,
    /// mio token for event routing.
    token: Token,
    /// Non-blocking frame reader accumulating partial frames.
    frame_reader: FrameReader,
    /// Non-blocking frame writer buffering outgoing frames.
    frame_writer: FrameWriter,
    /// Panes this client is subscribed to for push notifications.
    subscribed_panes: HashSet<PaneId>,
    /// Per-pane push priority (0=focused, 1=visible, 2=hidden).
    pane_priorities: HashMap<PaneId, u8>,
    /// Protocol capabilities advertised by the client.
    capabilities: u32,
    /// Per-pane set of `ImageId`s this client has already received pixel data
    /// for. Used by `push_snapshot_to_subscribers` + `GetPaneSnapshot` to
    /// project per-client `image_data` — when an `ImageId` is in this set AND
    /// the snapshot's `images_dirty == false`, the projection omits the wire
    /// `WireImageData` for that ID (client resolves via its `image_cache`).
    ///
    /// Server-driven invalidation: when `SnapshotCache.image_data_store`
    /// evicts a `(PaneId, ImageId)` under memory pressure, that ID is removed
    /// from EVERY connection's `sent_images[pane_id]` so the next snapshot
    /// referencing the ID re-includes its pixel data.
    /// See: bug-tracker/plans/BUG-06-072/
    sent_images: HashMap<PaneId, HashSet<ImageId>>,
}

impl ClientConnection {
    /// Create a new connection with the given ID and stream.
    pub(super) fn new(id: ClientId, stream: IpcStream, token: Token) -> Self {
        Self {
            id,
            stream,
            token,
            frame_reader: FrameReader::new(),
            frame_writer: FrameWriter::new(),
            subscribed_panes: HashSet::new(),
            pane_priorities: HashMap::new(),
            capabilities: 0,
            sent_images: HashMap::new(),
        }
    }

    /// Whether this connection has already been sent the pixel data for
    /// `(pane_id, image_id)`. Used by per-client snapshot projection to decide
    /// whether the wire `WireImageData` for an `ImageId` can be omitted.
    pub(super) fn has_sent_image(&self, pane_id: PaneId, image_id: ImageId) -> bool {
        self.sent_images
            .get(&pane_id)
            .is_some_and(|s| s.contains(&image_id))
    }

    /// Mark `(pane_id, image_id)` as sent to this client. Called after a
    /// successful `queue_frame` of a snapshot whose projection included the
    /// pixel data for that ID.
    pub(super) fn mark_image_sent(&mut self, pane_id: PaneId, image_id: ImageId) {
        self.sent_images.entry(pane_id).or_default().insert(image_id);
    }

    /// Clear every `sent_images` entry for `pane_id`. Called when
    /// `RenderableContent.images_dirty == true` — the server is about to resend
    /// every visible ID, so the per-client tracking starts fresh.
    pub(super) fn clear_sent_images(&mut self, pane_id: PaneId) {
        if let Some(s) = self.sent_images.get_mut(&pane_id) {
            s.clear();
        }
    }

    /// Drop the `(pane_id, image_id)` entry from `sent_images`. Called when
    /// the server evicts `(pane_id, image_id)` from its `image_data_store` —
    /// the next snapshot referencing the ID must re-include its pixel data.
    pub(super) fn forget_sent_image(&mut self, pane_id: PaneId, image_id: ImageId) {
        if let Some(s) = self.sent_images.get_mut(&pane_id) {
            s.remove(&image_id);
        }
    }

    /// Drop every `sent_images` entry for `pane_id`. Called on pane close.
    pub(super) fn drop_sent_images_for_pane(&mut self, pane_id: PaneId) {
        self.sent_images.remove(&pane_id);
    }

    /// Connection identifier.
    pub(super) fn id(&self) -> ClientId {
        self.id
    }

    /// Mutable access to the IPC stream.
    pub(super) fn stream_mut(&mut self) -> &mut IpcStream {
        &mut self.stream
    }

    /// mio token assigned to this connection.
    pub(super) fn token(&self) -> Token {
        self.token
    }

    /// Mutable access to the frame reader.
    pub(super) fn frame_reader_mut(&mut self) -> &mut FrameReader {
        &mut self.frame_reader
    }

    /// Queue a frame for sending and attempt to flush.
    ///
    /// If the stream returns `WouldBlock`, the remaining bytes are kept
    /// in the write buffer. The caller should register `WRITABLE` interest
    /// when [`has_pending_writes`] returns `true`.
    pub(super) fn queue_frame(&mut self, seq: u32, pdu: &MuxPdu) -> std::io::Result<()> {
        // TODO(34.8): pass `self.compress_enabled` based on negotiated FEAT_ZSTD.
        self.frame_writer.queue(seq, pdu, false)?;
        self.frame_writer.flush_to(&mut self.stream)
    }

    /// Flush any buffered outgoing data to the stream.
    pub(super) fn flush_writes(&mut self) -> std::io::Result<()> {
        self.frame_writer.flush_to(&mut self.stream)
    }

    /// Whether there is unsent data in the write buffer.
    pub(super) fn has_pending_writes(&self) -> bool {
        self.frame_writer.has_pending()
    }

    /// Add a pane subscription.
    pub(super) fn subscribe(&mut self, pane_id: PaneId) {
        self.subscribed_panes.insert(pane_id);
    }

    /// Remove a pane subscription.
    pub(super) fn unsubscribe(&mut self, pane_id: PaneId) {
        self.subscribed_panes.remove(&pane_id);
        self.pane_priorities.remove(&pane_id);
    }

    /// All pane IDs this client is subscribed to.
    pub(super) fn subscribed_panes(&self) -> &HashSet<PaneId> {
        &self.subscribed_panes
    }

    /// Set protocol capabilities advertised by the client.
    pub(super) fn set_capabilities(&mut self, flags: u32) {
        self.capabilities = flags;
    }

    /// Whether the client advertised a given capability flag.
    pub(super) fn has_capability(&self, flag: u32) -> bool {
        self.capabilities & flag != 0
    }

    /// Set the push priority for a pane.
    pub(super) fn set_pane_priority(&mut self, pane_id: PaneId, priority: u8) {
        self.pane_priorities.insert(pane_id, priority);
    }

    /// Push priority for a pane (0=focused if unset).
    pub(super) fn pane_priority(&self, pane_id: PaneId) -> u8 {
        self.pane_priorities.get(&pane_id).copied().unwrap_or(0)
    }

    /// Number of bytes buffered but not yet flushed to the stream.
    pub(super) fn pending_write_bytes(&self) -> usize {
        self.frame_writer.pending_bytes()
    }
}
