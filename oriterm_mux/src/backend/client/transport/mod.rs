//! IPC transport layer: connection, reader thread, RPC roundtrip.
//!
//! [`ClientTransport`] manages the IPC connection to the mux daemon.
//! A background reader thread owns the stream and multiplexes outbound
//! requests with inbound responses and push notifications.

// Platform FFI for self-pipe wakeup (pipe2, poll, read, write, close).

mod handshake;
mod reader;
mod wake_pipe;
mod writer;

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(unix)]
use std::os::fd::RawFd;

use oriterm_ipc::ClientStream;

use oriterm_core::effect::ResponseTokenId;

use crate::id::ClientId;
use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;
use crate::{PaneId, PaneSnapshot};

/// Bookkeeping for a host-request received from the daemon, awaiting the
/// App's `MuxBackend::fulfill_host_request` call (BUG-11-011).
pub(super) enum PendingClientReply {
    /// OSC 52 clipboard load — `request_id` echoed back in the reply PDU.
    Clipboard {
        /// Server-allocated id from the originating notification.
        request_id: u64,
    },
    /// OSC 4 / 10 / 11 / 12 color query.
    Color {
        /// Server-allocated id from the originating notification.
        request_id: u64,
    },
}

impl PendingClientReply {
    /// Echo-back `request_id` regardless of variant.
    pub(super) fn request_id(&self) -> u64 {
        match self {
            Self::Clipboard { request_id } | Self::Color { request_id } => *request_id,
        }
    }
}

/// RPC timeout for blocking responses.
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Read timeout on the socket for interleaving reads with send-channel drains.
///
/// 1ms keeps RPC round-trips fast (sub-2ms median) at negligible CPU cost
/// for a single reader thread (~1000 polls/s when idle).
const READ_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Interval between health-check pings.
///
/// If the previous ping is still outstanding when the next interval fires,
/// the connection is declared dead (implicit timeout = `PING_INTERVAL`).
const PING_INTERVAL: Duration = Duration::from_secs(5);

/// A request queued for the reader thread to send.
struct SendRequest {
    /// Sequence number assigned by the transport.
    seq: u32,
    /// PDU to encode and write.
    pdu: MuxPdu,
    /// Reply channel. `None` for fire-and-forget messages.
    reply_tx: Option<mpsc::Sender<MuxPdu>>,
}

/// IPC transport to the mux daemon.
///
/// Manages background **reader** and **writer** threads (BUG-11-047
/// architectural split): the writer drains the outbound mpsc and writes
/// frames to its half of the cloned stream; the reader pulls inbound
/// frames from its half and dispatches them to the shared pending map or
/// notification channel. Splitting reads from writes lets the reader
/// always service replies even while the writer is blocked on a
/// backpressured `write()`.
///
/// The main thread sends requests via the mpsc channel and blocks on
/// per-request oneshot replies. Push notifications are buffered in a
/// separate channel.
pub(super) struct ClientTransport {
    /// Channel to queue outbound requests for the writer thread.
    /// Wrapped in `Option` so `Drop` can close the channel before joining
    /// the writer thread (Rust drops fields *after* `Drop::drop` runs).
    send_tx: Option<mpsc::Sender<SendRequest>>,
    /// Channel to receive push notifications from the reader thread.
    notif_rx: mpsc::Receiver<MuxNotification>,
    /// Monotonic sequence counter for request/response correlation.
    next_seq: u32,
    /// Writer thread handle (joined on drop, before the reader).
    writer_handle: Option<JoinHandle<()>>,
    /// Reader thread handle (joined on drop, after the writer).
    reader_handle: Option<JoinHandle<()>>,
    /// Client ID assigned by the daemon during handshake.
    client_id: ClientId,
    /// Set to `false` when either thread exits.
    alive: Arc<AtomicBool>,
    /// Shared snapshot slot: reader thread inserts pushed snapshots,
    /// main thread takes them at render time. Bounds memory to `O(num_panes)`.
    pushed_snapshots: Arc<Mutex<HashMap<PaneId, PaneSnapshot>>>,
    /// Coalescing flag: prevents redundant `PostMessage` wakeup syscalls
    /// during flood output. Set by the guarded wakeup closure, cleared
    /// by [`clear_wakeup_pending`](Self::clear_wakeup_pending) in `poll_events`.
    wakeup_pending: Arc<AtomicBool>,
    /// Write end of the self-pipe used to wake the reader thread on
    /// shutdown (Unix only).
    #[cfg(unix)]
    wake_write: RawFd,
    /// Pending host-request replies received from the daemon — keyed by
    /// the local `ResponseToken`'s `slot_id` so `MuxClient::fulfill_host_request`
    /// can look up the original `request_id` and echo it back in
    /// `MuxPdu::ReplyHostRequest`. Shared with the reader thread (which
    /// inserts on `NotifyHostClipboardLoad`/`NotifyHostColorQuery`) and
    /// the main thread (which removes on fulfill).
    pending_replies: Arc<Mutex<HashMap<ResponseTokenId, PendingClientReply>>>,
}

impl ClientTransport {
    /// Connect to the daemon at `path` and perform the Hello handshake.
    ///
    /// `wakeup` is called when push notifications arrive, so the event loop
    /// can wake and process them.
    #[allow(
        clippy::too_many_lines,
        reason = "linear setup sequence — channels, threads, struct init; splitting would scatter ownership"
    )]
    pub(super) fn connect(path: &Path, wakeup: Arc<dyn Fn() + Send + Sync>) -> io::Result<Self> {
        let mut stream = ClientStream::connect(path)?;
        let client_id = handshake::run_handshake(&mut stream, path)?;

        // Set up channels.
        let (send_tx, send_rx) = mpsc::channel::<SendRequest>();
        let (notif_tx, notif_rx) = mpsc::channel::<MuxNotification>();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_reader = Arc::clone(&alive);
        let alive_writer = Arc::clone(&alive);
        let pushed_snapshots = Arc::new(Mutex::new(HashMap::new()));
        let pushed_snapshots_reader = Arc::clone(&pushed_snapshots);

        // Wrap wakeup with coalescing flag to prevent redundant PostMessage
        // syscalls during flood output (hundreds per second → at most one).
        let wakeup_pending = Arc::new(AtomicBool::new(false));
        let guarded_wakeup = {
            let pending = wakeup_pending.clone();
            Arc::new(move || {
                if !pending.swap(true, Ordering::Release) {
                    (wakeup)();
                }
            }) as Arc<dyn Fn() + Send + Sync>
        };

        // Create self-pipe for waking the reader thread on shutdown.
        #[cfg(unix)]
        let (wake_read, wake_write) = wake_pipe::create_self_pipe()?;

        let pending_replies: Arc<Mutex<HashMap<ResponseTokenId, PendingClientReply>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_replies_reader = Arc::clone(&pending_replies);

        // Pending RPC reply senders, shared between writer (insert on
        // dispatch) and reader (remove on response).
        let pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_writer = Arc::clone(&pending);
        let pending_reader = Arc::clone(&pending);

        // Outstanding ping seq, shared between writer (set on Ping send) and
        // reader (clear on PingAck). 0 = no outstanding ping.
        let outstanding_ping_seq = Arc::new(AtomicU64::new(0));
        let outstanding_ping_seq_writer = Arc::clone(&outstanding_ping_seq);
        let outstanding_ping_seq_reader = Arc::clone(&outstanding_ping_seq);

        // Split the stream into independent read + write halves so the
        // reader thread can drain replies while the writer thread is
        // blocked on a backpressured write (BUG-11-047).
        let write_stream = stream
            .try_clone()
            .map_err(|e| io::Error::other(format!("failed to clone IPC stream for writer: {e}")))?;
        let read_stream = stream;

        // Spawn writer thread first; reader thread depends on the writer
        // for the heartbeat ping cadence.
        let writer_handle = std::thread::Builder::new()
            .name("mux-client-writer".into())
            .spawn(move || {
                writer::writer_loop(
                    write_stream,
                    send_rx,
                    pending_writer,
                    alive_writer,
                    outstanding_ping_seq_writer,
                );
            })
            .map_err(|e| io::Error::other(format!("failed to spawn writer thread: {e}")))?;

        // Spawn reader thread.
        let reader_handle = std::thread::Builder::new()
            .name("mux-client-reader".into())
            .spawn(move || {
                reader::reader_loop(
                    read_stream,
                    notif_tx,
                    guarded_wakeup,
                    alive_reader,
                    pushed_snapshots_reader,
                    pending_replies_reader,
                    pending_reader,
                    outstanding_ping_seq_reader,
                    #[cfg(unix)]
                    wake_read,
                );
                #[cfg(unix)]
                #[allow(unsafe_code, reason = "close(2) for self-pipe read end")]
                unsafe {
                    libc::close(wake_read);
                }
            })
            .map_err(|e| io::Error::other(format!("failed to spawn reader thread: {e}")))?;

        Ok(Self {
            send_tx: Some(send_tx),
            notif_rx,
            next_seq: 3, // seq 1 = Hello, seq 2 = SetCapabilities
            writer_handle: Some(writer_handle),
            reader_handle: Some(reader_handle),
            client_id,
            alive,
            pushed_snapshots,
            wakeup_pending,
            #[cfg(unix)]
            wake_write,
            pending_replies,
        })
    }

    /// The client ID assigned by the daemon.
    pub(super) fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// Send a request and block until the response arrives.
    ///
    /// Returns `Err` on timeout, transport death, or daemon error response.
    pub(super) fn rpc(&mut self, pdu: MuxPdu) -> io::Result<MuxPdu> {
        if !self.is_alive() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "daemon connection lost",
            ));
        }

        let seq = self.alloc_seq();
        let (reply_tx, reply_rx) = mpsc::channel();

        let tx = self
            .send_tx
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "transport shut down"))?;
        tx.send(SendRequest {
            seq,
            pdu,
            reply_tx: Some(reply_tx),
        })
        .map_err(|_send_err| io::Error::new(io::ErrorKind::BrokenPipe, "reader thread gone"))?;
        self.signal_wake();

        match reply_rx.recv_timeout(RPC_TIMEOUT) {
            Ok(MuxPdu::Error { message }) => Err(io::Error::other(message)),
            Ok(response) => Ok(response),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "RPC timed out after 5s",
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "reply channel disconnected",
            )),
        }
    }

    /// Send a message without waiting for a response.
    ///
    /// Used for `Input` and `Resize` PDUs once snapshot-based rendering
    /// is wired (the app will send input/resize through the client transport
    /// rather than through the local `Pane`).
    pub(super) fn fire_and_forget(&mut self, pdu: MuxPdu) {
        if !self.is_alive() {
            return;
        }
        let seq = self.alloc_seq();
        if let Some(ref tx) = self.send_tx {
            let _ = tx.send(SendRequest {
                seq,
                pdu,
                reply_tx: None,
            });
            self.signal_wake();
        }
    }

    /// Fallible version of `fire_and_forget` — surfaces transport
    /// liveness / mpsc failures as `io::Error` so callers (notably
    /// `MuxClient::fulfill_host_request`) can propagate them rather than
    /// silently dropping data (codex-004 round 1 finding).
    pub(super) fn try_fire_and_forget(&mut self, pdu: MuxPdu) -> io::Result<()> {
        if !self.is_alive() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "daemon connection lost",
            ));
        }
        let seq = self.alloc_seq();
        let tx = self
            .send_tx
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "transport shut down"))?;
        tx.send(SendRequest {
            seq,
            pdu,
            reply_tx: None,
        })
        .map_err(|_send_err| io::Error::new(io::ErrorKind::BrokenPipe, "reader thread gone"))?;
        self.signal_wake();
        Ok(())
    }

    /// Remove + return the pending reply entry for `slot_id`, if any.
    ///
    /// Called by `MuxClient::fulfill_host_request` to recover the
    /// daemon-allocated `request_id` that must be echoed back in
    /// `ReplyHostRequest`.
    pub(super) fn take_pending_reply(
        &self,
        slot_id: ResponseTokenId,
    ) -> Option<PendingClientReply> {
        self.pending_replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&slot_id)
    }

    /// Drain pending push notifications into `out`.
    pub(super) fn poll_notifications(&self, out: &mut Vec<MuxNotification>) {
        while let Ok(n) = self.notif_rx.try_recv() {
            out.push(n);
        }
    }

    /// Take a pushed snapshot for a specific pane, if one exists.
    ///
    /// Called at render time (not poll time) so that bare-dirty
    /// invalidations arriving between poll and render are respected.
    pub(super) fn take_pushed_snapshot(&self, pane_id: PaneId) -> Option<PaneSnapshot> {
        self.pushed_snapshots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&pane_id)
    }

    /// Remove any pushed snapshot for a pane (fire-and-forget invalidation).
    ///
    /// Called by fire-and-forget mutation methods so that a stale push
    /// from before the mutation is not used on the next render.
    pub(super) fn invalidate_pushed_snapshot(&self, pane_id: PaneId) {
        self.pushed_snapshots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&pane_id);
    }

    /// Whether the reader thread is still running.
    pub(super) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    /// Whether a wakeup has been posted since the last `clear_wakeup_pending`.
    ///
    /// Used by the event loop to skip `poll_events` when no PTY activity
    /// has occurred since the last poll.
    pub(super) fn has_pending_wakeup(&self) -> bool {
        self.wakeup_pending.load(Ordering::Acquire)
    }

    /// Clear the wakeup-pending flag so the next notification posts a wakeup.
    ///
    /// Called at the start of `poll_events` on the main thread.
    pub(super) fn clear_wakeup_pending(&self) {
        self.wakeup_pending.store(false, Ordering::Release);
    }

    /// Write a byte to the self-pipe to wake the reader thread immediately.
    #[cfg(unix)]
    fn signal_wake(&self) {
        #[allow(unsafe_code, reason = "write(2) to self-pipe for reader thread wakeup")]
        unsafe {
            libc::write(self.wake_write, [1u8].as_ptr().cast(), 1);
        }
    }

    /// No-op on non-Unix platforms (reader uses timeout-based polling).
    #[cfg(not(unix))]
    fn signal_wake(&self) {
        let _ = self;
    }

    /// Allocate the next sequence number.
    fn alloc_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        if self.next_seq == 0 {
            // Skip 0 — reserved for notifications.
            self.next_seq = 1;
        }
        seq
    }
}

#[cfg(all(test, unix))]
impl ClientTransport {
    /// Set the next sequence number for testing wraparound behavior.
    pub(super) fn test_set_next_seq(&mut self, val: u32) {
        self.next_seq = val;
    }
}

impl Drop for ClientTransport {
    fn drop(&mut self) {
        // 1. Close the send channel so the writer thread sees `Disconnected`
        //    on `recv_timeout` and exits, draining its pending RPC reply
        //    senders so callers see `BrokenPipe` (Codex-001 round 1 pin).
        self.send_tx.take();

        // 2. Join the writer thread first. Once it exits it sets
        //    `alive = false` (or it was already false from an earlier write
        //    error), which the reader observes on its next loop iteration.
        if let Some(handle) = self.writer_handle.take() {
            let _ = handle.join();
        }

        // 3. Belt-and-suspenders: ensure the reader sees `alive = false`
        //    even on the rare path where the writer never ran a loop body.
        self.alive.store(false, Ordering::Release);

        // 4. Wake the reader from poll(2) so it observes `alive = false`
        //    immediately and exits.
        self.signal_wake();
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }

        #[cfg(unix)]
        #[allow(unsafe_code, reason = "close(2) for self-pipe write end")]
        unsafe {
            libc::close(self.wake_write);
        }
    }
}

#[cfg(test)]
mod tests;
