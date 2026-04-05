//! Shared frame decode logic.
//!
//! Extracts the common decode algorithm used by both [`ProtocolCodec`] (blocking
//! client reads) and [`FrameReader`] (non-blocking server reads). Both callers
//! maintain their own `Vec<u8>` buffer and delegate here for the actual
//! header-parse → validate → deserialize → drain sequence.

use super::msg_type::MsgType;
use super::{FLAG_COMPRESSED, FRAME_MAGIC, FrameHeader, HEADER_LEN, MAX_PAYLOAD};
use crate::protocol::codec::{DecodeError, DecodedFrame};
use crate::protocol::messages::MuxPdu;

/// Try to decode one complete frame from a buffer.
///
/// Returns `Some(Ok(frame))` if a full frame was decoded and consumed,
/// `Some(Err(e))` on a decode error (malformed bytes are consumed),
/// or `None` if there aren't enough bytes yet.
///
/// On success or error, consumed bytes are drained from `buf`.
/// If the `COMPRESSED` flag is set in the header, the payload is
/// decompressed with zstd before bincode deserialization.
pub(crate) fn try_decode_from_buf(buf: &mut Vec<u8>) -> Option<Result<DecodedFrame, DecodeError>> {
    if buf.len() < HEADER_LEN {
        return None;
    }

    let header = FrameHeader::decode(
        buf[..HEADER_LEN]
            .try_into()
            .expect("checked length >= HEADER_LEN"),
    );

    // Validate magic bytes — early detection of non-oriterm connections.
    if header.magic != FRAME_MAGIC {
        buf.drain(..HEADER_LEN);
        return Some(Err(DecodeError::BadMagic(header.magic)));
    }

    // Validate payload size.
    if header.payload_len > MAX_PAYLOAD {
        buf.drain(..HEADER_LEN);
        return Some(Err(DecodeError::PayloadTooLarge(header.payload_len)));
    }

    // Validate message type. Wait for the full frame to be buffered,
    // then drain all of it to keep the stream aligned.
    if MsgType::from_u16(header.msg_type).is_none() {
        let total = HEADER_LEN + header.payload_len as usize;
        if buf.len() < total {
            return None; // Not enough data yet — wait for full frame.
        }
        buf.drain(..total);
        return Some(Err(DecodeError::UnknownMsgType(header.msg_type)));
    }

    let total = HEADER_LEN + header.payload_len as usize;
    if buf.len() < total {
        return None;
    }

    // Extract and drain the raw payload bytes.
    let raw_payload = buf[HEADER_LEN..total].to_vec();
    buf.drain(..total);

    // Decompress if the COMPRESSED flag is set.
    let payload_bytes = if header.flags & FLAG_COMPRESSED != 0 {
        match zstd::decode_all(raw_payload.as_slice()) {
            Ok(decompressed) => decompressed,
            Err(e) => {
                return Some(Err(DecodeError::Io(e)));
            }
        }
    } else {
        raw_payload
    };

    // Deserialize the (possibly decompressed) payload.
    let result: Result<MuxPdu, _> = bincode::deserialize(&payload_bytes);

    match result {
        Ok(pdu) => Some(Ok(DecodedFrame {
            seq: header.seq,
            pdu,
        })),
        Err(e) => Some(Err(DecodeError::Deserialize(e))),
    }
}
