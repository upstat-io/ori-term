//! Non-blocking frame I/O for mio streams.
//!
//! [`FrameReader`] accumulates bytes from non-blocking reads and greedily
//! decodes complete frames. [`send_frame`] is a thin wrapper around
//! [`ProtocolCodec::encode_frame`] for clarity at call sites.

use std::io::{self, Write};

use crate::MuxPdu;
use crate::protocol::{DecodeError, DecodedFrame};

/// Result of a single `read_from` call.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadStatus {
    /// At least one byte was read.
    GotData,
    /// The peer closed the connection (EOF).
    Closed,
    /// No data available right now (`WouldBlock`).
    WouldBlock,
}

/// Accumulates bytes from a non-blocking stream and decodes frames.
///
/// The reader buffers partial headers and payloads across `read_from` calls.
/// After each `read_from`, call `try_decode` in a loop to drain all complete
/// frames before returning to the event loop.
pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    /// Create a new empty reader.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
        }
    }

    /// Append raw bytes to the internal buffer.
    ///
    /// Called after the caller reads bytes from the stream separately (to
    /// avoid double-mutable-borrow of `ClientConnection`).
    pub fn extend(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Try to decode one complete frame from the buffer.
    ///
    /// Returns `Some(Ok(frame))` if a full frame was decoded and consumed,
    /// `Some(Err(e))` on a decode error (the malformed bytes are consumed),
    /// or `None` if there aren't enough bytes yet.
    pub fn try_decode(&mut self) -> Option<Result<DecodedFrame, DecodeError>> {
        crate::protocol::decode::try_decode_from_buf(&mut self.buf)
    }
}

/// Per-connection outgoing frame buffer.
///
/// Frames are serialized to an internal buffer via [`queue`]. The caller
/// then calls [`flush_to`] to write as much as possible to the non-blocking
/// stream. If a write returns `WouldBlock`, the remaining bytes stay in the
/// buffer and are retried on the next writable event.
pub struct FrameWriter {
    buf: Vec<u8>,
}

impl FrameWriter {
    /// Create a new empty writer.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(256),
        }
    }

    /// Serialize a frame and append it to the outgoing buffer.
    ///
    /// When `compress` is true, payloads above the compression threshold are
    /// zstd-compressed. The caller should set this based on the per-connection
    /// negotiated features (`FEAT_ZSTD`).
    pub fn queue(&mut self, seq: u32, pdu: &MuxPdu, compress: bool) -> io::Result<()> {
        crate::protocol::encode::encode_into_buf(&mut self.buf, seq, pdu, compress)
    }

    /// Write as much buffered data as possible to the stream.
    ///
    /// Returns `Ok(())` even if some data remains (caller should check
    /// [`has_pending`] and register `WRITABLE` interest if so).
    pub fn flush_to<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        while !self.buf.is_empty() {
            match writer.write(&self.buf) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
                Ok(n) => {
                    self.buf.drain(..n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Whether there is unsent data in the buffer.
    pub fn has_pending(&self) -> bool {
        !self.buf.is_empty()
    }

    /// Number of bytes buffered but not yet written.
    pub fn pending_bytes(&self) -> usize {
        self.buf.len()
    }
}
