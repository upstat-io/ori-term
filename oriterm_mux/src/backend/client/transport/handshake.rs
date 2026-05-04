//! `Hello` / `HelloAck` / `SetCapabilities` handshake.
//!
//! Runs synchronously on the main thread before the writer + reader
//! threads are spawned. Returns the `client_id` assigned by the daemon
//! and leaves the stream positioned just after the `SetCapabilities`
//! fire-and-forget frame.

use std::io;
use std::path::Path;

use oriterm_ipc::ClientStream;

use crate::id::ClientId;
use crate::protocol::{MuxPdu, ProtocolCodec};

/// Send `Hello`, read `HelloAck`, send `SetCapabilities`. Returns the
/// daemon-assigned `ClientId`.
///
/// `path` is only used in the success log message.
pub(super) fn run_handshake(stream: &mut ClientStream, path: &Path) -> io::Result<ClientId> {
    // 1. Send Hello.
    let pid = std::process::id();
    ProtocolCodec::encode_frame(
        stream,
        1,
        &MuxPdu::Hello {
            pid,
            protocol_version: crate::protocol::CURRENT_PROTOCOL_VERSION,
            features: crate::protocol::FEAT_ZSTD,
        },
    )?;

    // 2. Read HelloAck (blocking, no timeout — daemon responds quickly).
    let frame = ProtocolCodec::new().decode_frame(stream).map_err(|e| {
        io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("handshake failed: {e}"),
        )
    })?;

    let client_id = match frame.pdu {
        MuxPdu::HelloAck {
            client_id,
            protocol_version,
            features,
        } => {
            log::info!(
                "connected to daemon at {}, assigned {client_id} \
                 (server v={protocol_version}, features=0x{features:X})",
                path.display(),
            );
            client_id
        }
        MuxPdu::Error { message } => {
            return Err(io::Error::other(format!(
                "daemon rejected handshake: {message}"
            )));
        }
        other => {
            return Err(io::Error::other(format!(
                "unexpected handshake response: {other:?}"
            )));
        }
    };

    // 3. Advertise capabilities (fire-and-forget, no ack expected).
    let cap_seq = 2; // seq 1 was Hello; RPC seqs start from 3.
    ProtocolCodec::encode_frame(
        stream,
        cap_seq,
        &MuxPdu::SetCapabilities {
            flags: crate::protocol::messages::CAP_SNAPSHOT_PUSH,
        },
    )?;

    Ok(client_id)
}
