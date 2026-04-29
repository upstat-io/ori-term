use std::io::{self, Cursor, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::PtyReader;

/// A reader that yields `data` once and then returns EOF.
struct OneShotReader {
    inner: Cursor<Vec<u8>>,
}

impl OneShotReader {
    fn new(data: Vec<u8>) -> Self {
        Self {
            inner: Cursor::new(data),
        }
    }
}

impl Read for OneShotReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

/// A reader that blocks until shutdown is set, then returns EOF.
struct BlockingReader {
    shutdown: Arc<AtomicBool>,
}

impl Read for BlockingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // Spin briefly until shutdown, simulating a blocking PTY read.
        while !self.shutdown.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(0) // EOF
    }
}

/// Write known bytes, verify they arrive on the channel.
#[test]
fn pty_reader_forwards_bytes() {
    let data = b"hello world".to_vec();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let shutdown = Arc::new(AtomicBool::new(false));

    let reader = PtyReader::new(
        Box::new(OneShotReader::new(data.clone())),
        byte_tx,
        Arc::clone(&shutdown),
    );
    let handle = reader.spawn().unwrap();

    let mut received = Vec::new();
    while let Ok(chunk) = byte_rx.recv_timeout(std::time::Duration::from_secs(2)) {
        received.extend_from_slice(&chunk);
    }
    handle.join().unwrap();

    assert_eq!(received, data);
}

/// Setting shutdown flag stops the reader.
#[test]
fn pty_reader_shutdown_flag_stops_loop() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let (byte_tx, _byte_rx) = crossbeam_channel::unbounded();

    let reader = PtyReader::new(
        Box::new(BlockingReader {
            shutdown: Arc::clone(&shutdown),
        }),
        byte_tx,
        Arc::clone(&shutdown),
    );
    let handle = reader.spawn().unwrap();

    // Set shutdown — the BlockingReader will return EOF.
    shutdown.store(true, Ordering::Release);
    handle.join().unwrap();
}

/// EOF causes clean exit.
#[test]
fn pty_reader_eof_exits_cleanly() {
    let (byte_tx, _byte_rx) = crossbeam_channel::unbounded();
    let shutdown = Arc::new(AtomicBool::new(false));

    let reader = PtyReader::new(
        Box::new(OneShotReader::new(Vec::new())), // empty → immediate EOF
        byte_tx,
        Arc::clone(&shutdown),
    );
    let handle = reader.spawn().unwrap();
    handle.join().unwrap();
}

/// Large buffer forwarding — 500KB of data arrives intact.
#[test]
fn pty_reader_large_buffer_forwarding() {
    let data: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let shutdown = Arc::new(AtomicBool::new(false));

    let reader = PtyReader::new(
        Box::new(OneShotReader::new(data.clone())),
        byte_tx,
        Arc::clone(&shutdown),
    );
    let handle = reader.spawn().unwrap();

    let mut received = Vec::new();
    while let Ok(chunk) = byte_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        received.extend_from_slice(&chunk);
    }
    handle.join().unwrap();

    assert_eq!(received.len(), data.len());
    assert_eq!(received, data);
}

// --- BUG-11-002: bounded-channel disconnect-unblock contract ---

/// Pin 5 — bounded-channel disconnect unblocks blocked sender.
///
/// Regression: BUG-11-002. The bounded byte-channel back-pressure design
/// relies on `crossbeam_channel`'s disconnect-unblock contract: when the
/// receiver is dropped, a blocked or pending `send()` on the sender
/// returns `Err(SendError)` rather than hanging. This UNIT-tests the
/// crossbeam contract directly so the bounded-channel design can rely on
/// it without spinning up a `PaneIoThread` (whose IO thread would
/// deadlock when the byte queue saturates — verified during /tpr-review
/// round 3, opencode F1/F2 + gemini F1 + codex F4).
/// See: bug-tracker/plans/BUG-11-002/section-03-tdd-matrix.md §"Semantic pins" Pin 5.
#[test]
fn reader_send_returns_err_on_disconnect() {
    // Bounded channel of capacity 1 — easy to fill.
    let (byte_tx, byte_rx) = crossbeam_channel::bounded::<Vec<u8>>(1);

    // Saturate the bounded channel so the next send must block.
    byte_tx
        .send(Vec::new())
        .expect("first send fits within capacity");

    // Spawn a sender that blocks on send (capacity is full).
    let tx_blocked = byte_tx.clone();
    let join = std::thread::spawn(move || tx_blocked.send(Vec::new()));

    // Drop the receiver — blocked sender unblocks with SendError.
    // (If the sender hadn't yet entered send(), it sees the disconnected
    // channel on entry and returns SendError immediately. Either branch
    // satisfies the assertion — wall-clock-free per
    // `.claude/rules/tests.md §Wall-Clock-Free Testing`.)
    drop(byte_rx);

    let result = join.join().expect("sender thread joined cleanly");
    assert!(
        result.is_err(),
        "byte_tx.send must return Err(SendError) after byte_rx drops, got {result:?}"
    );
}

/// Interrupted reads (EINTR) are retried, not treated as fatal.
#[test]
fn pty_reader_interrupted_read_retries() {
    /// Reader that returns EINTR once, then real data, then EOF.
    struct InterruptReader {
        state: u8,
    }

    impl Read for InterruptReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.state {
                0 => {
                    self.state = 1;
                    Err(io::Error::from(io::ErrorKind::Interrupted))
                }
                1 => {
                    self.state = 2;
                    let data = b"after-eintr";
                    let n = data.len().min(buf.len());
                    buf[..n].copy_from_slice(&data[..n]);
                    Ok(n)
                }
                _ => Ok(0), // EOF
            }
        }
    }

    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let shutdown = Arc::new(AtomicBool::new(false));

    let reader = PtyReader::new(
        Box::new(InterruptReader { state: 0 }),
        byte_tx,
        Arc::clone(&shutdown),
    );
    let handle = reader.spawn().unwrap();

    let mut received = Vec::new();
    while let Ok(chunk) = byte_rx.recv_timeout(std::time::Duration::from_secs(2)) {
        received.extend_from_slice(&chunk);
    }
    handle.join().unwrap();

    assert_eq!(received, b"after-eintr");
}
