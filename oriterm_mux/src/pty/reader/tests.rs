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
