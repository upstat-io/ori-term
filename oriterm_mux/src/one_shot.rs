//! One-shot IPC operations.
//!
//! Lightweight functions for sending a single request to a running daemon
//! without starting a full [`MuxClient`]. Used by CLI flags like `--new-tab`
//! to signal the existing instance and exit.

use std::io;
use std::path::Path;
use std::time::Duration;

use oriterm_ipc::ClientStream;

use crate::protocol::{DecodeError, MuxPdu, ProtocolCodec};

/// Timeout for one-shot RPC reads (generous to cover daemon load).
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Send a "new tab" request to a running daemon and exit.
///
/// Connects to the daemon, completes the Hello handshake, sends
/// [`MuxPdu::RequestNewTab`], waits for [`MuxPdu::NewTabAck`], and
/// returns. The daemon broadcasts `NotifyNewTab` to other clients.
pub fn request_new_tab(socket_path: &Path) -> io::Result<()> {
    let mut stream = ClientStream::connect(socket_path)?;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut codec = ProtocolCodec::new();

    // Hello handshake.
    ProtocolCodec::encode_frame(
        &mut stream,
        1,
        &MuxPdu::Hello {
            pid: std::process::id(),
        },
    )?;
    let hello_resp = codec.decode_frame(&mut stream).map_err(decode_to_io)?;
    if !matches!(hello_resp.pdu, MuxPdu::HelloAck { .. }) {
        return Err(io::Error::other("unexpected hello response"));
    }

    // Send RequestNewTab.
    ProtocolCodec::encode_frame(&mut stream, 2, &MuxPdu::RequestNewTab)?;
    let ack = codec.decode_frame(&mut stream).map_err(decode_to_io)?;
    if !matches!(ack.pdu, MuxPdu::NewTabAck) {
        return Err(io::Error::other("unexpected new-tab response"));
    }

    Ok(())
}

/// Convert a decode error to `io::Error`.
fn decode_to_io(e: DecodeError) -> io::Error {
    match e {
        DecodeError::Io(io_err) => io_err,
        other => io::Error::other(other.to_string()),
    }
}
