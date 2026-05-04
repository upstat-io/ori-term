//! Background reader thread for the IPC transport.
//!
//! The reader thread owns the cloned read half of the IPC stream. It
//! reads frames, dispatches replies to the shared pending map, and routes
//! push notifications. Outbound writes belong to the writer thread
//! (`writer.rs`); the reader is read-only so a backpressured write never
//! blocks reply delivery (BUG-11-047).

// Platform FFI for poll(2), pipe read/drain.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};

#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};

use oriterm_core::effect::{ResponseToken, ResponseTokenId};

use oriterm_ipc::ClientStream;

use crate::mux_event::MuxNotification;
use crate::protocol::{DecodedFrame, MuxPdu, ProtocolCodec};
use crate::{PaneId, PaneSnapshot};

use super::super::notification::pdu_to_notification;
use super::{PendingClientReply, READ_POLL_INTERVAL};

/// Dispatch a received notification PDU.
///
/// `NotifyPaneSnapshot` and `NotifyPaneOutput` are intercepted here
/// (stored/invalidated in the shared snapshot map). The 4 BUG-11-011
/// host-request / desktop-notification variants are also intercepted —
/// they need the shared `pending_replies` map (host-request side) or a
/// direct `MuxNotification` translation (desktop-notification side).
/// Other notifications go through [`pdu_to_notification`].
fn dispatch_notification(
    pdu: MuxPdu,
    pushed_snapshots: &Mutex<HashMap<PaneId, PaneSnapshot>>,
    pending_replies: &Mutex<HashMap<ResponseTokenId, PendingClientReply>>,
    notif_tx: &mpsc::Sender<MuxNotification>,
    wakeup: &dyn Fn(),
) {
    match pdu {
        MuxPdu::NotifyPaneSnapshot { pane_id, snapshot } => {
            pushed_snapshots
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(pane_id, snapshot);
            let _ = notif_tx.send(MuxNotification::PaneOutput(pane_id));
            (wakeup)();
        }
        MuxPdu::NotifyPaneOutput { pane_id } => {
            pushed_snapshots
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&pane_id);
            let _ = notif_tx.send(MuxNotification::PaneOutput(pane_id));
            (wakeup)();
        }
        MuxPdu::NotifyHostClipboardLoad {
            pane_id,
            request_id,
            selection,
            clipboard_char,
            terminator,
        } => {
            let token = ResponseToken::<String>::new();
            pending_replies
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(
                    token.slot_id(),
                    PendingClientReply::Clipboard { request_id },
                );
            let _ = notif_tx.send(MuxNotification::HostClipboardLoad {
                pane_id,
                selection: selection.into(),
                clipboard_char,
                terminator,
                reply: token,
            });
            (wakeup)();
        }
        MuxPdu::NotifyHostColorQuery {
            pane_id,
            request_id,
            prefix,
            index,
            terminator,
        } => {
            let token = ResponseToken::<oriterm_core::color::Rgb>::new();
            pending_replies
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(token.slot_id(), PendingClientReply::Color { request_id });
            let _ = notif_tx.send(MuxNotification::HostColorQuery {
                pane_id,
                prefix,
                index: index as usize,
                terminator,
                reply: token,
            });
            (wakeup)();
        }
        MuxPdu::NotifyDesktopNotification {
            pane_id,
            source,
            title,
            body,
        } => {
            let _ = notif_tx.send(MuxNotification::DesktopNotification {
                pane_id,
                source: source.into(),
                title,
                body,
            });
            (wakeup)();
        }
        MuxPdu::NotifyClearPendingDesktopNotifications { pane_id } => {
            let _ = notif_tx.send(MuxNotification::ClearPendingDesktopNotifications(pane_id));
            (wakeup)();
        }
        other => {
            if let Some(notif) = pdu_to_notification(other) {
                let _ = notif_tx.send(notif);
                (wakeup)();
            }
        }
    }
}

/// Non-blocking check for data available on the socket.
///
/// Returns `true` if the socket has data ready to read, without blocking.
/// Used after successfully decoding a frame to avoid the expensive blocking
/// `decode_frame` retry that would sleep for `READ_POLL_INTERVAL` (1ms+
/// due to kernel timer granularity) when no more data is available.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "poll(2) for non-blocking socket readability check"
)]
fn socket_has_data(stream: &ClientStream) -> bool {
    let mut pfd = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    (unsafe { libc::poll(&raw mut pfd, 1, 0) }) > 0
}

/// Wait for readability on the socket or a wake signal from the main thread.
///
/// Uses `poll(2)` to block until either the IPC socket has data or the
/// wake pipe is signalled. Returns immediately if either fd is already ready.
/// Falls back to a short sleep on non-Unix platforms.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "poll(2) + read(2) for socket/wake-pipe multiplexing"
)]
fn wait_for_readable(stream: &ClientStream, wake_read: RawFd, timeout_ms: i32) -> bool {
    let mut fds = [
        libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: wake_read,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    let ret = unsafe { libc::poll(fds.as_mut_ptr(), 2, timeout_ms) };
    if ret > 0 && fds[1].revents & libc::POLLIN != 0 {
        // Drain the wake pipe (non-blocking, consume all pending bytes).
        let mut buf = [0u8; 64];
        unsafe { while libc::read(wake_read, buf.as_mut_ptr().cast(), buf.len()) > 0 {} }
    }
    // Return whether the socket has data.
    ret > 0 && fds[0].revents & libc::POLLIN != 0
}

/// Background reader thread event loop.
///
/// Owns the cloned read half of the IPC stream. Pure reader: blocks on
/// socket readability via `poll(2)` (with the wake pipe as a shutdown
/// signal), reads frames, and dispatches them to the shared pending map
/// (RPC replies) or notification channel (push notifications). Outbound
/// writes are owned by the writer thread (BUG-11-047 architectural split).
///
/// `outstanding_ping_seq` is shared with the writer thread: writer sets
/// it on Ping send, reader clears it on `PingAck`. `0` is the "no
/// outstanding ping" sentinel.
#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    reason = "ownership required — values are moved into the spawned thread"
)]
pub(super) fn reader_loop(
    mut stream: ClientStream,
    notif_tx: mpsc::Sender<MuxNotification>,
    wakeup: Arc<dyn Fn() + Send + Sync>,
    alive: Arc<AtomicBool>,
    pushed_snapshots: Arc<Mutex<HashMap<PaneId, PaneSnapshot>>>,
    pending_replies: Arc<Mutex<HashMap<ResponseTokenId, PendingClientReply>>>,
    pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>,
    outstanding_ping_seq: Arc<AtomicU64>,
    #[cfg(unix)] wake_read: RawFd,
) {
    // Set a short read timeout as a safety net for edge cases where poll(2)
    // says readable but the full frame hasn't arrived yet.
    if let Err(e) = stream.set_read_timeout(Some(READ_POLL_INTERVAL)) {
        log::error!("mux-client-reader: failed to set read timeout: {e}");
        alive.store(false, Ordering::Release);
        return;
    }

    let mut codec = ProtocolCodec::new();

    loop {
        if !alive.load(Ordering::Acquire) {
            return;
        }

        // 1. Wait for socket data or wake signal (shutdown).
        //
        // `poll(2)` returns immediately if either fd is already ready, so
        // a single 1-second-bounded call covers both the "data already
        // waiting" and "block until ready or shutdown" paths. The bound
        // gives `alive`-flag observation cadence on quiet connections.
        #[cfg(unix)]
        let socket_ready = wait_for_readable(&stream, wake_read, 1000);
        #[cfg(not(unix))]
        let socket_ready = {
            std::thread::sleep(READ_POLL_INTERVAL);
            true // Always try to read on non-Unix.
        };

        if !socket_ready {
            continue;
        }

        // 2. Read and dispatch all available frames.
        if !read_and_dispatch_frames(
            &mut stream,
            &mut codec,
            &pending,
            &outstanding_ping_seq,
            &pushed_snapshots,
            &pending_replies,
            &notif_tx,
            &*wakeup,
        ) {
            alive.store(false, Ordering::Release);
            return;
        }
    }
}

/// Read and dispatch all available frames from the socket.
///
/// After each successful decode, checks socket readability via `poll(0)` to
/// avoid the expensive blocking `decode_frame` retry (1ms+ due to kernel
/// timer granularity on WSL2). Returns `false` if the connection is dead.
#[allow(
    clippy::too_many_arguments,
    reason = "reader thread state — grouping would add indirection"
)]
fn read_and_dispatch_frames(
    stream: &mut ClientStream,
    codec: &mut ProtocolCodec,
    pending: &Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>,
    outstanding_ping_seq: &AtomicU64,
    pushed_snapshots: &Mutex<HashMap<PaneId, PaneSnapshot>>,
    pending_replies: &Mutex<HashMap<ResponseTokenId, PendingClientReply>>,
    notif_tx: &mpsc::Sender<MuxNotification>,
    wakeup: &dyn Fn(),
) -> bool {
    loop {
        match codec.decode_frame(stream) {
            Ok(DecodedFrame { seq, pdu }) => {
                let expected_ping = outstanding_ping_seq.load(Ordering::Acquire);
                if expected_ping != 0 && expected_ping == u64::from(seq) && pdu == MuxPdu::PingAck {
                    outstanding_ping_seq.store(0, Ordering::Release);
                    #[cfg(unix)]
                    if !codec.has_buffered_data() && !socket_has_data(stream) {
                        break;
                    }
                    continue;
                }

                if seq == 0 || pdu.is_notification() {
                    dispatch_notification(pdu, pushed_snapshots, pending_replies, notif_tx, wakeup);
                } else {
                    let entry = pending
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .remove(&seq);
                    if let Some(reply_tx) = entry {
                        let _ = reply_tx.send(pdu);
                    } else {
                        log::warn!(
                            "mux-client-reader: no pending request for seq={seq}, dropping response"
                        );
                    }
                }

                // Break early if no more data in buffer or on socket —
                // avoids blocking read() that sleeps for the timeout.
                #[cfg(unix)]
                if !codec.has_buffered_data() && !socket_has_data(stream) {
                    break;
                }
            }
            Err(crate::protocol::DecodeError::UnknownMsgType(t)) => {
                log::warn!("mux-client-reader: unknown msg_type 0x{t:04x}, skipping");
            }
            Err(crate::protocol::DecodeError::Io(ref e))
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(crate::protocol::DecodeError::Io(ref e))
                if e.kind() == io::ErrorKind::UnexpectedEof =>
            {
                log::info!("mux-client-reader: daemon disconnected (EOF)");
                return false;
            }
            Err(e) => {
                log::error!("mux-client-reader: decode error: {e}");
                return false;
            }
        }
    }
    true
}
