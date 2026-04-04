//! Frame codec for encoding and decoding protocol messages.
//!
//! Provides [`ProtocolCodec`] which reads/writes framed [`MuxPdu`] messages
//! from any `Read`/`Write` stream. Decoding buffers partial reads internally,
//! so timeouts or short reads never cause frame misalignment.

use std::io::{self, Read, Write};

use super::MAX_PAYLOAD;
use super::messages::MuxPdu;

/// A decoded frame: sequence number + PDU.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Sequence number from the header (for request/response correlation).
    pub seq: u32,
    /// Decoded protocol message.
    pub pdu: MuxPdu,
}

/// Errors from frame decoding.
#[derive(Debug)]
pub enum DecodeError {
    /// I/O error reading from the stream.
    Io(io::Error),
    /// Magic bytes do not match `0x4F54`.
    BadMagic(u16),
    /// Payload exceeds [`MAX_PAYLOAD`].
    PayloadTooLarge(u32),
    /// Unknown message type ID in the header.
    UnknownMsgType(u16),
    /// Bincode deserialization failed.
    Deserialize(bincode::Error),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::BadMagic(m) => write!(f, "bad magic bytes: 0x{m:04X} (expected 0x4F54)"),
            Self::PayloadTooLarge(n) => {
                write!(f, "payload too large: {n} bytes (max {MAX_PAYLOAD})")
            }
            Self::UnknownMsgType(t) => write!(f, "unknown message type: 0x{t:04X}"),
            Self::Deserialize(e) => write!(f, "deserialize error: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Deserialize(e) => Some(e),
            Self::BadMagic(_) | Self::PayloadTooLarge(_) | Self::UnknownMsgType(_) => None,
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<bincode::Error> for DecodeError {
    fn from(e: bincode::Error) -> Self {
        Self::Deserialize(e)
    }
}

/// Codec for encoding and decoding framed protocol messages.
///
/// Encoding is straightforward (serialize + write header + payload).
/// Decoding accumulates bytes in an internal buffer across calls.
/// Partial reads (from timeouts or non-blocking streams) are safely
/// buffered — no bytes are lost, and frame alignment is preserved.
pub struct ProtocolCodec {
    /// Accumulation buffer for incoming bytes. Partial headers and payloads
    /// persist across `decode_frame` calls. Bytes are consumed only when a
    /// complete frame is decoded (or a malformed frame is skipped).
    buf: Vec<u8>,
}

impl Default for ProtocolCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolCodec {
    /// Create a new codec with an empty decode buffer.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
        }
    }

    /// Whether the internal buffer contains data that may form a complete
    /// frame. The caller can use this to avoid blocking on `poll(2)` when
    /// buffered data already contains the next frame.
    pub fn has_buffered_data(&self) -> bool {
        !self.buf.is_empty()
    }

    /// Encode a PDU and write it as a framed message.
    ///
    /// Writes the header followed by the bincode payload atomically
    /// (single `write_all` call for the assembled frame).
    pub fn encode_frame<W: Write>(writer: &mut W, seq: u32, pdu: &MuxPdu) -> io::Result<()> {
        let mut buf = Vec::new();
        super::encode::encode_into_buf(&mut buf, seq, pdu, false)?;
        writer.write_all(&buf)?;
        writer.flush()
    }

    /// Decode a single framed message from a stream.
    ///
    /// Reads bytes into an internal buffer until a complete frame is
    /// available. Returns `DecodeError::Io` with `WouldBlock` if a timeout
    /// or non-blocking read returns before enough data arrives — partial
    /// reads are buffered safely and will be completed on the next call.
    ///
    /// Returns `DecodeError::Io(UnexpectedEof)` if the stream closes.
    pub fn decode_frame<R: Read>(&mut self, reader: &mut R) -> Result<DecodedFrame, DecodeError> {
        // Fast path: try to decode from existing buffer contents.
        if let Some(result) = self.try_decode() {
            return result;
        }

        // Read in a loop until we have a complete frame, EOF, or timeout.
        let mut tmp = [0u8; 8192];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) => {
                    return Err(DecodeError::Io(io::Error::from(
                        io::ErrorKind::UnexpectedEof,
                    )));
                }
                Ok(n) => {
                    self.buf.extend_from_slice(&tmp[..n]);
                    if let Some(result) = self.try_decode() {
                        return result;
                    }
                    // Need more data — continue reading.
                }
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    return Err(DecodeError::Io(io::Error::from(io::ErrorKind::WouldBlock)));
                }
                Err(e) => return Err(DecodeError::Io(e)),
            }
        }
    }

    /// Try to decode one complete frame from the internal buffer.
    ///
    /// Returns `Some(Ok(frame))` if a full frame was decoded and consumed,
    /// `Some(Err(e))` on a decode error (malformed bytes are consumed),
    /// or `None` if there aren't enough bytes yet.
    fn try_decode(&mut self) -> Option<Result<DecodedFrame, DecodeError>> {
        super::decode::try_decode_from_buf(&mut self.buf)
    }
}
