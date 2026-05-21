//! IPC wire protocol for daemon ↔ window communication.
//!
//! Binary framing with a fixed 14-byte header followed by a bincode-encoded
//! payload. Designed for low-latency local IPC (Unix sockets / named pipes).
//!
//! # Frame format
//!
//! ```text
//! ┌────────────┬───────────┬──────────┬──────────┬──────────┬───────────────┐
//! │ magic(u16) │ ver(u8) │ flags(u8)│ type(u16)│ seq(u32) │ payload_len(u32)│
//! ├────────────┴───────────┴──────────┴──────────┴──────────┴───────────────┤
//! │ payload (bincode-encoded MuxPdu variant) │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - `magic`: `0x4F54` ("OT") — early detection of non-oriterm connections.
//! - `ver`: protocol version. See [`PROTOCOL_VERSION`] for the
//! canonical value (currently `2`); the constant doc comment
//! carries the reason for the latest bump.
//! - `flags`: `0x01` = `COMPRESSED` (payload is zstd-compressed). Unknown bits
//! are silently ignored on decode for forward compatibility.
//! - `type`: message type ID for pre-routing and debugging.
//! - `seq`: request/response correlation. Notifications use `seq = 0`.
//! - `payload_len`: u32, max 16 MiB.
//! - payload: bincode-serialized variant fields.

mod codec;
pub(crate) mod decode;
pub(crate) mod encode;
pub(crate) mod host_request;
pub(crate) mod messages;
pub(crate) mod msg_type;
mod pdu_traits;
pub mod snapshot;

pub use codec::{DecodeError, DecodedFrame, ProtocolCodec};
pub use host_request::{HostReplyPayload, WireClipboardSelection, WireNotificationSource};
pub use messages::MuxPdu;
pub(crate) use pdu_traits::theme_to_wire;

/// Client supports receiving `NotifyPaneSnapshot` pushed snapshots.
pub const CAP_SNAPSHOT_PUSH: u32 = 1;
// Re-export for server/tests.rs (test-only consumer outside protocol module).
#[cfg(test)]
pub(crate) use msg_type::MsgType;
pub use snapshot::{
    OSC22_KNOWN_ICONS, PaneSnapshot, WireCell, WireCellFlags, WireCursor, WireCursorShape, WireRgb,
    WireSearchMatch, WireSelection, decode_cursor_icon, encode_cursor_icon,
};

/// Frame header size in bytes.
pub const HEADER_LEN: usize = 14;

/// Maximum payload size (80 MiB).
/// Sized to comfortably accommodate the core image cache's per-image limit
/// (`DEFAULT_MAX_SINGLE_IMAGE = 64 MiB` at `oriterm_core/src/image/cache/mod.rs:20`)
/// plus cell payload headroom. Raised from 16 MiB at `PROTOCOL_VERSION` v3
/// because daemon-mode image rendering requires shipping pixel data over the wire.
/// Compression at `protocol/encode.rs:27-54` applies automatically when payload exceeds
/// `COMPRESSION_THRESHOLD` and zstd shrinks it.
pub const MAX_PAYLOAD: u32 = 80 * 1024 * 1024;

/// Magic bytes identifying an oriterm IPC frame (`0x4F54` = "OT").
pub const FRAME_MAGIC: u16 = 0x4F54;

/// Current protocol version. Incremented on breaking wire changes.
/// v2 — bumped when `PaneSnapshot::has_bell` and `MuxPdu::ClearBell`
/// were stripped from the wire (bell state moved to client-local
/// `bell_panes`).
/// v3 — bumped when `PaneSnapshot` gained `images`/`image_data`/`images_dirty` fields.
/// `Eq` derive dropped from `PaneSnapshot` and `MuxPdu` because `WirePlacement` carries
/// f32 fields (f32 implements `PartialEq` only). `MAX_PAYLOAD` raised from 16 MiB to
/// 80 MiB to ship kitty/sixel/iTerm2 pixel data over the wire.
/// vN → vN+1 is a hard break in bincode-encoded `PaneSnapshot` / `MuxPdu` layout; a
/// mismatched peer would silently misdecode every snapshot frame. The Hello handshake
/// rejects any non-equal version pair so the mismatch surfaces as a connection error
/// instead.
/// SSOT: this constant is the single literal source for the wire
/// version; [`CURRENT_PROTOCOL_VERSION`] is a re-export, NOT an
/// independent definition. A reviewer who bumps one without the
/// other would otherwise produce a frame-header version that
/// disagrees with the Hello payload — caught at compile-time now
/// because the alias propagates the bump automatically.
pub const PROTOCOL_VERSION: u8 = 3;

/// Flag: payload is zstd-compressed.
pub const FLAG_COMPRESSED: u8 = 0x01;

/// Current IPC protocol version for Hello/HelloAck negotiation.
/// Re-export of [`PROTOCOL_VERSION`] — the frame-header version
/// (used in every encode via `FrameHeader.version`) and the
/// handshake version (used in `MuxPdu::Hello` / `MuxPdu::HelloAck`
/// payload) MUST agree. Defined as an alias rather than a duplicate
/// literal so a future bump touches one place.
pub const CURRENT_PROTOCOL_VERSION: u8 = PROTOCOL_VERSION;

/// Feature flag: client and server support zstd compression.
pub const FEAT_ZSTD: u64 = 1;

/// Frame header on the wire (14 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameHeader {
    /// Magic bytes (`0x4F54`). Must match [`FRAME_MAGIC`] on decode.
    pub magic: u16,
    /// Protocol version.
    pub version: u8,
    /// Flags (e.g., [`FLAG_COMPRESSED`]). Unknown bits are ignored on decode.
    pub flags: u8,
    /// Message type ID (for routing and debugging).
    pub msg_type: u16,
    /// Request/response correlation. `0` for fire-and-forget and notifications.
    pub seq: u32,
    /// Length of the bincode-encoded payload in bytes.
    pub payload_len: u32,
}

impl FrameHeader {
    /// Encode the header into a 14-byte buffer.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..2].copy_from_slice(&self.magic.to_le_bytes());
        buf[2] = self.version;
        buf[3] = self.flags;
        buf[4..6].copy_from_slice(&self.msg_type.to_le_bytes());
        buf[6..10].copy_from_slice(&self.seq.to_le_bytes());
        buf[10..14].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    /// Decode a header from a 14-byte buffer.
    /// Does not validate the magic or version — callers must check those.
    pub fn decode(buf: &[u8; HEADER_LEN]) -> Self {
        let magic = u16::from_le_bytes([buf[0], buf[1]]);
        let version = buf[2];
        let flags = buf[3];
        let msg_type = u16::from_le_bytes([buf[4], buf[5]]);
        let seq = u32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]);
        let payload_len = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
        Self {
            magic,
            version,
            flags,
            msg_type,
            seq,
            payload_len,
        }
    }
}

#[cfg(test)]
mod tests;
