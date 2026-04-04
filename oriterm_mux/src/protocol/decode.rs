//! Shared frame decode logic.
//!
//! Extracts the common decode algorithm used by both [`ProtocolCodec`] (blocking
//! client reads) and [`FrameReader`] (non-blocking server reads). Both callers
//! maintain their own `Vec<u8>` buffer and delegate here for the actual
//! header-parse → validate → deserialize → drain sequence.

use super::msg_type::MsgType;
use super::{FRAME_MAGIC, FrameHeader, HEADER_LEN, MAX_PAYLOAD};
use crate::protocol::codec::{DecodeError, DecodedFrame};
use crate::protocol::messages::MuxPdu;

/// Try to decode one complete frame from a buffer.
///
/// Returns `Some(Ok(frame))` if a full frame was decoded and consumed,
/// `Some(Err(e))` on a decode error (malformed bytes are consumed),
/// or `None` if there aren't enough bytes yet.
///
/// On success or error, consumed bytes are drained from `buf`.
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

    // Deserialize the payload.
    let payload = &buf[HEADER_LEN..total];
    let result: Result<MuxPdu, _> = bincode::deserialize(payload);
    buf.drain(..total);

    match result {
        Ok(pdu) => Some(Ok(DecodedFrame {
            seq: header.seq,
            pdu,
        })),
        Err(e) => Some(Err(DecodeError::Deserialize(e))),
    }
}
