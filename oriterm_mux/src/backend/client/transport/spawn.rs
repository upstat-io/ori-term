//! Reader / writer thread spawn helpers.
//!
//! Extracted from `mod.rs` so [`ClientTransport::connect`] stays under
//! the 100-line function-size budget AND `mod.rs` stays under the
//! 500-line file-size budget.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[cfg(unix)]
use std::os::fd::RawFd;

use oriterm_core::effect::ResponseTokenId;
use oriterm_ipc::ClientStream;

use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;
use crate::{PaneId, PaneSnapshot};

use super::types::{PendingClientReply, SendRequest};
use super::{reader, writer};

/// Spawn the writer thread that drains `send_rx` and encodes outbound
/// frames.
#[allow(clippy::needless_pass_by_value, reason = "ownership moves into thread")]
pub(super) fn spawn_writer_thread(
    write_stream: ClientStream,
    send_rx: mpsc::Receiver<SendRequest>,
    pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>,
    alive: Arc<AtomicBool>,
    outstanding_ping_seq: Arc<AtomicU64>,
) -> io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("mux-client-writer".into())
        .spawn(move || {
            writer::writer_loop(write_stream, send_rx, pending, alive, outstanding_ping_seq);
        })
        .map_err(|e| io::Error::other(format!("failed to spawn writer thread: {e}")))
}

/// Spawn the reader thread that consumes inbound frames and dispatches
/// them to the shared pending map and notification channel.
#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    reason = "ownership moves into thread; per-thread state shape is the boundary"
)]
pub(super) fn spawn_reader_thread(
    read_stream: ClientStream,
    notif_tx: mpsc::Sender<MuxNotification>,
    wakeup: Arc<dyn Fn() + Send + Sync>,
    alive: Arc<AtomicBool>,
    pushed_snapshots: Arc<Mutex<HashMap<PaneId, PaneSnapshot>>>,
    pending_replies: Arc<Mutex<HashMap<ResponseTokenId, PendingClientReply>>>,
    pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>,
    outstanding_ping_seq: Arc<AtomicU64>,
    #[cfg(unix)] wake_read: RawFd,
) -> io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("mux-client-reader".into())
        .spawn(move || {
            reader::reader_loop(
                read_stream,
                notif_tx,
                wakeup,
                alive,
                pushed_snapshots,
                pending_replies,
                pending,
                outstanding_ping_seq,
                #[cfg(unix)]
                wake_read,
            );
            #[cfg(unix)]
            #[allow(unsafe_code, reason = "close(2) for self-pipe read end")]
            unsafe {
                libc::close(wake_read);
            }
        })
        .map_err(|e| io::Error::other(format!("failed to spawn reader thread: {e}")))
}
