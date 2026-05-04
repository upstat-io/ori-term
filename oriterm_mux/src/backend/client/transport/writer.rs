//! Background writer thread for the IPC transport.
//!
//! The writer thread owns the cloned write half of the IPC stream. It
//! drains outbound requests from the send channel, encodes frames, and
//! sends periodic health-check pings. Splitting writes onto a dedicated
//! thread (separate from the reader thread) prevents the head-of-line
//! block under sustained backpressure: a slow daemon drain can stall the
//! writer for seconds while the reader still services replies and push
//! notifications. See BUG-11-047.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use oriterm_ipc::ClientStream;

use crate::protocol::{MuxPdu, ProtocolCodec};

use super::{PING_INTERVAL, SendRequest};

/// Bounded recv timeout so the writer wakes periodically to fire health
/// pings and observe the `alive` flag, even when no application traffic
/// is queued.
const SEND_RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Background writer thread event loop.
///
/// Drains `send_rx`, encodes frames onto `write_stream`, and emits
/// heartbeat pings every [`PING_INTERVAL`]. On any write error or send
/// channel disconnect, sets `alive = false`, drains pending RPC reply
/// senders so callers see `BrokenPipe` instead of a 5-second timeout, and
/// returns.
///
/// `outstanding_ping_seq` uses `0` as the "no pending ping" sentinel; the
/// reader thread clears it on `PingAck`. Ping seq numbers count down from
/// `u32::MAX` and skip `0` to avoid colliding with the notification sentinel.
#[allow(
    clippy::needless_pass_by_value,
    reason = "ownership required — values are moved into the spawned thread"
)]
pub(super) fn writer_loop(
    mut write_stream: ClientStream,
    send_rx: mpsc::Receiver<SendRequest>,
    pending: Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>,
    alive: Arc<AtomicBool>,
    outstanding_ping_seq: Arc<AtomicU64>,
) {
    let mut last_ping_sent = Instant::now();
    let mut ping_seq_counter = u32::MAX;

    loop {
        if !alive.load(Ordering::Acquire) {
            drain_pending(&pending);
            return;
        }

        // 1. Drain outbound requests with bounded timeout so we periodically
        //    wake to check the ping interval and the alive flag.
        match send_rx.recv_timeout(SEND_RECV_TIMEOUT) {
            Ok(req) => {
                if let Some(reply_tx) = req.reply_tx {
                    pending
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(req.seq, reply_tx);
                }
                if let Err(e) = ProtocolCodec::encode_frame(&mut write_stream, req.seq, &req.pdu) {
                    log::error!("mux-client-writer: write error: {e}");
                    alive.store(false, Ordering::Release);
                    drain_pending(&pending);
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                alive.store(false, Ordering::Release);
                drain_pending(&pending);
                return;
            }
        }

        // 2. Health-check ping.
        if last_ping_sent.elapsed() >= PING_INTERVAL {
            if outstanding_ping_seq.load(Ordering::Acquire) != 0 {
                log::warn!("mux-client-writer: ping timeout, marking connection dead");
                alive.store(false, Ordering::Release);
                drain_pending(&pending);
                return;
            }
            let seq = ping_seq_counter;
            ping_seq_counter = next_ping_seq(ping_seq_counter);
            // Publish the outstanding seq BEFORE writing the ping so a fast
            // PingAck reply cannot race ahead of the store and be dropped by
            // the reader thread's `expected_ping == 0` sentinel check.
            // Codex round 0 / BUG-11-047 §06 finding F1 pin.
            outstanding_ping_seq.store(u64::from(seq), Ordering::Release);
            if let Err(e) = ProtocolCodec::encode_frame(&mut write_stream, seq, &MuxPdu::Ping) {
                log::error!("mux-client-writer: ping write error: {e}");
                // Clear the published seq so a future writer (or the
                // shutdown path) does not observe a stale outstanding ping.
                outstanding_ping_seq.store(0, Ordering::Release);
                alive.store(false, Ordering::Release);
                drain_pending(&pending);
                return;
            }
            last_ping_sent = Instant::now();
        }
    }
}

/// Wake every in-flight RPC reply receiver with `Disconnected` so callers
/// observe `BrokenPipe` instead of waiting `RPC_TIMEOUT` for a phantom
/// response. Codex-001 round 1 finding pin.
fn drain_pending(pending: &Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>) {
    let drained: Vec<_> = pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .drain()
        .collect();
    drop(drained);
}

/// Decrement the ping seq counter, skipping `0` which is reserved as the
/// "no outstanding ping" sentinel for the shared `Arc<AtomicU64>`.
fn next_ping_seq(current: u32) -> u32 {
    match current.wrapping_sub(1) {
        0 => u32::MAX,
        n => n,
    }
}
