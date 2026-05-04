//! Tests for the split reader/writer transport architecture.
//!
//! Pins the BUG-11-047 architectural invariant: outbound writes happen on
//! a dedicated `writer.rs` thread; the `reader.rs` thread is read-only.
//! Source-grep semantic pins lock the split so a regression that re-merges
//! the threads fails to compile (or fails the source-grep pin) rather than
//! silently re-introducing the head-of-line block under backpressure.

#[test]
fn reader_does_not_drain_send_rx() {
    let reader_src = include_str!("reader.rs");
    let bad_patterns = [
        "send_rx.try_recv",
        "send_rx.recv(",
        "send_rx.recv_timeout",
        "send_rx: mpsc::Receiver<SendRequest>",
    ];
    for pat in bad_patterns {
        assert!(
            !reader_src.contains(pat),
            "BUG-11-047 architectural invariant: reader.rs MUST NOT drain or own \
             send_rx (found `{pat}`). Outbound writes belong in writer.rs so the \
             reader thread can always read replies regardless of write backpressure."
        );
    }
}

#[test]
fn reader_does_not_send_pings() {
    let reader_src = include_str!("reader.rs");
    // Match `Ping` followed by punctuation — `MuxPdu::Ping` as a value, not
    // the `MuxPdu::PingAck` variant which the reader still observes.
    let bad_patterns = ["MuxPdu::Ping,", "MuxPdu::Ping)", "encode_frame(&mut stream"];
    for pat in bad_patterns {
        assert!(
            !reader_src.contains(pat),
            "BUG-11-047 architectural invariant: reader.rs MUST NOT call \
             `encode_frame` or send `MuxPdu::Ping` (found `{pat}`). The writer \
             thread owns the heartbeat pings; merging them back into the reader \
             re-introduces the head-of-line block under backpressure."
        );
    }
}

#[test]
fn writer_module_owns_send_rx_drain() {
    let writer_src = include_str!("writer.rs");
    assert!(
        writer_src.contains("send_rx"),
        "BUG-11-047 architectural invariant: writer.rs MUST own send_rx draining."
    );
    assert!(
        writer_src.contains("encode_frame"),
        "BUG-11-047 architectural invariant: writer.rs MUST own outbound \
         encode_frame calls."
    );
}

#[test]
fn writer_module_owns_ping_heartbeat() {
    let writer_src = include_str!("writer.rs");
    assert!(
        writer_src.contains("MuxPdu::Ping"),
        "BUG-11-047 architectural invariant: writer.rs MUST own the heartbeat \
         Ping send (moved here from reader.rs so a backpressured write does \
         not block the heartbeat)."
    );
}

#[test]
fn writer_module_drains_pending_on_error_exit() {
    let writer_src = include_str!("writer.rs");
    assert!(
        writer_src.contains("drain_pending") || writer_src.contains("drain()"),
        "BUG-11-047 / Codex review pin: writer.rs MUST drain pending RPC reply \
         senders on error exit so callers see Disconnected (BrokenPipe) \
         instead of waiting RPC_TIMEOUT (5s) for a phantom response."
    );
}

#[test]
fn pending_map_is_shared_arc_mutex() {
    let mod_src = include_str!("mod.rs");
    assert!(
        mod_src.contains("Arc<Mutex<HashMap<u32, mpsc::Sender<MuxPdu>>>>"),
        "BUG-11-047 architectural invariant: the pending RPC reply-sender map \
         must be Arc<Mutex<HashMap<...>>> so writer.rs (insert) and reader.rs \
         (remove) can share it across thread boundaries. Pre-fix layout (a \
         reader-local HashMap) is what enabled the head-of-line block."
    );
}

#[test]
fn writer_module_is_declared_in_mod_rs() {
    let mod_src = include_str!("mod.rs");
    assert!(
        mod_src.contains("mod writer;") || mod_src.contains("mod writer ;"),
        "BUG-11-047: mod.rs must declare `mod writer;` so the writer thread \
         lives in its canonical home (transport/writer.rs)."
    );
}

#[cfg(unix)]
mod backpressure_pins {
    //! Behavioral pin: the reader thread continues to dispatch incoming
    //! frames while the writer thread is blocked on a backpressured
    //! `write()` to the IPC socket. Pre-fix this scenario hangs because the
    //! single reactor owns both directions.

    use std::os::unix::net::UnixListener;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::id::{ClientId, PaneId};
    use crate::protocol::{MuxPdu, ProtocolCodec};

    use super::super::ClientTransport;

    /// Push notifications keep arriving via the reader thread even when
    /// the writer thread is mid-flight on a large outbound payload.
    ///
    /// Test shape: server stops draining the client's outbound socket once
    /// the handshake completes. The client queues a 2 MiB `Input` PDU
    /// which the writer thread tries to encode — but since the server is
    /// not draining, the writer blocks in `write()`. After a short settle
    /// to ensure the writer is firmly inside `write()`, the test signals
    /// the server to emit a `NotifyPaneOutput`. That byte lands on the
    /// client's RCVBUF *while the writer is blocked*. Post-fix: reader
    /// thread is independent and dispatches the notification within ~ms.
    /// Pre-fix (single reactor): the dispatch waits behind the 2 MiB
    /// encode and never arrives within the deadline.
    #[test]
    fn reader_dispatches_notifications_while_writer_is_backpressured() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("test.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let (signal_tx, signal_rx) = std::sync::mpsc::channel::<()>();

        let server_handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut codec = ProtocolCodec::new();

            // Hello / HelloAck.
            let frame = codec.decode_frame(&mut stream).unwrap();
            assert!(matches!(frame.pdu, MuxPdu::Hello { .. }));
            ProtocolCodec::encode_frame(
                &mut stream,
                frame.seq,
                &MuxPdu::HelloAck {
                    client_id: ClientId::from_raw(99),
                    protocol_version: crate::protocol::CURRENT_PROTOCOL_VERSION,
                    features: 0,
                },
            )
            .unwrap();
            // SetCapabilities (fire-and-forget).
            let _ = codec.decode_frame(&mut stream);

            // Wait for the test thread to signal that the writer is
            // backpressured (Input has been queued + a small settling
            // delay). Then emit the notification — it lands on the
            // client's RCVBUF while the writer is stuck in `write()`.
            let _ = signal_rx.recv();

            ProtocolCodec::encode_frame(
                &mut stream,
                0,
                &MuxPdu::NotifyPaneOutput {
                    pane_id: PaneId::from_raw(2),
                },
            )
            .unwrap();

            // Hold the connection open long enough for the assertion to
            // observe the notification. Don't drain the client's outbound
            // buffer — that's what creates the backpressure.
            thread::sleep(Duration::from_millis(800));
        });

        // No-op wakeup — we measure notification arrival via `notif_rx`
        // directly, not the wakeup count, because the transport's
        // coalescing flag suppresses second wakeups until
        // `clear_wakeup_pending` is called.
        let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});

        let mut transport = ClientTransport::connect(&sock, wakeup).unwrap();

        // Queue a 2 MiB Input PDU. The server is not draining, so the
        // client's writer thread blocks inside `encode_frame`.
        let big_payload = vec![b'x'; 2 * 1024 * 1024];
        transport.fire_and_forget(MuxPdu::Input {
            pane_id: PaneId::from_raw(1),
            data: big_payload,
        });

        // Brief settle so the writer is firmly inside `write()` before
        // the server emits the notification.
        thread::sleep(Duration::from_millis(100));
        let _ = signal_tx.send(());

        // Poll notif_rx until the notification arrives or the deadline
        // expires. Per `tests.md §Wall-Clock-Free Testing`,
        // deadline-as-safety not deadline-as-signal — the assertion is on
        // the notification arrival, not on the timer.
        let deadline = Instant::now() + Duration::from_millis(700);
        let mut got_notif = false;
        while Instant::now() < deadline {
            let mut notifs = Vec::new();
            transport.poll_notifications(&mut notifs);
            if !notifs.is_empty() {
                got_notif = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        assert!(
            got_notif,
            "BUG-11-047 architectural invariant: reader thread must dispatch \
             notifications while writer thread is backpressured on a 2 MiB \
             write. Pre-fix: same thread owns both reads and writes, so the \
             notification waits behind the 2 MiB encode_frame and never \
             arrives within the deadline."
        );

        drop(transport);
        let _ = server_handle.join();
    }
}
