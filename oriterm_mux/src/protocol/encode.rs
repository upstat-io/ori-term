//! Shared frame encode logic.
//!
//! Extracts the common encode algorithm used by both [`ProtocolCodec::encode_frame`]
//! (blocking client writes) and [`FrameWriter::queue`] (non-blocking server writes).
//! Both callers serialize the PDU, validate the size, construct the header, and
//! assemble the frame bytes — this module is the single canonical implementation.

use std::io;

use super::messages::MuxPdu;
use super::{FRAME_MAGIC, FrameHeader, MAX_PAYLOAD, PROTOCOL_VERSION};

/// Encode a PDU into a frame and append the bytes to `buf`.
///
/// Writes the 14-byte header followed by the bincode payload. The caller owns
/// `buf` and decides how to send the bytes (blocking write, non-blocking queue,
/// etc.). The `flags` parameter is currently always `0` — compression support
/// will set [`FLAG_COMPRESSED`](super::FLAG_COMPRESSED) when implemented.
pub(crate) fn encode_into_buf(buf: &mut Vec<u8>, seq: u32, pdu: &MuxPdu) -> io::Result<()> {
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

    let header = FrameHeader {
        magic: FRAME_MAGIC,
        version: PROTOCOL_VERSION,
        flags: 0,
        msg_type: pdu.msg_type() as u16,
        seq,
        payload_len,
    };

    buf.extend_from_slice(&header.encode());
    buf.extend_from_slice(&payload);
    Ok(())
}
