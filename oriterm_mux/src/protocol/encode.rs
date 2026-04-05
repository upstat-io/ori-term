//! Shared frame encode logic.
//!
//! Extracts the common encode algorithm used by both [`ProtocolCodec::encode_frame`]
//! (blocking client writes) and [`FrameWriter::queue`] (non-blocking server writes).
//! Both callers serialize the PDU, validate the size, construct the header, and
//! assemble the frame bytes — this module is the single canonical implementation.

use std::io;

use super::messages::MuxPdu;
use super::{FLAG_COMPRESSED, FRAME_MAGIC, FrameHeader, MAX_PAYLOAD, PROTOCOL_VERSION};

/// Minimum payload size to attempt compression.
///
/// Payloads at or below this threshold are sent uncompressed — the zstd overhead
/// outweighs the savings for small messages like `Ping`, `Input`, `Resize`.
const COMPRESSION_THRESHOLD: usize = 4096;

/// Zstd compression level. Level 1 is the fastest and still gives good ratios
/// for repetitive terminal cell data.
const ZSTD_LEVEL: i32 = 1;

/// Encode a PDU into a frame and append the bytes to `buf`.
///
/// Writes the 14-byte header followed by the (possibly compressed) bincode
/// payload. When `compress` is true and the payload exceeds
/// [`COMPRESSION_THRESHOLD`], zstd compression is attempted. If compression
/// produces a larger result, the uncompressed payload is used instead.
///
/// The caller owns `buf` and decides how to send the bytes.
pub(crate) fn encode_into_buf(
    buf: &mut Vec<u8>,
    seq: u32,
    pdu: &MuxPdu,
    compress: bool,
) -> io::Result<()> {
    let payload =
        bincode::serialize(pdu).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let payload_len: u32 = payload.len().try_into().map_err(|_overflow| {
        io::Error::new(io::ErrorKind::InvalidData, "payload exceeds u32 capacity")
    })?;

    if payload_len > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("payload too large: {payload_len} bytes (max {MAX_PAYLOAD})"),
        ));
    }

    // Try compression if enabled and payload is large enough.
    let (wire_payload, flags) = if compress && payload.len() > COMPRESSION_THRESHOLD {
        match zstd::encode_all(payload.as_slice(), ZSTD_LEVEL) {
            Ok(compressed) if compressed.len() < payload.len() => (compressed, FLAG_COMPRESSED),
            // Compression didn't help or failed — send uncompressed.
            _ => (payload, 0),
        }
    } else {
        (payload, 0)
    };

    let wire_len: u32 = wire_payload.len().try_into().map_err(|_overflow| {
        io::Error::new(io::ErrorKind::InvalidData, "compressed payload exceeds u32")
    })?;

    let header = FrameHeader {
        magic: FRAME_MAGIC,
        version: PROTOCOL_VERSION,
        flags,
        msg_type: pdu.msg_type() as u16,
        seq,
        payload_len: wire_len,
    };

    buf.extend_from_slice(&header.encode());
    buf.extend_from_slice(&wire_payload);
    Ok(())
}
