//! PTY byte forwarder — reads shell output and sends to the IO thread.
//!
//! Formerly `PtyEventLoop` when it owned VTE parsing. Now a simple read
//! loop that forwards raw bytes via channel. VTE parsing is exclusively
//! owned by the Terminal IO thread ([`PaneIoThread`]).

use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;

/// PTY read buffer size.
///
/// Large buffer (1 MB, matching Alacritty) so the reader can accumulate
/// data while the IO thread is busy processing commands. Prevents
/// `ConPTY` back-pressure on Windows during flood output.
const READ_BUFFER_SIZE: usize = 0x10_0000; // 1 MB

/// PTY byte forwarder — reads shell output and sends to the IO thread.
///
/// Runs on a dedicated thread spawned by [`PtyReader::spawn`]. The main
/// loop reads from the PTY fd and forwards raw bytes to the Terminal IO
/// thread via a crossbeam channel. No VTE parsing — the IO thread owns
/// that exclusively.
pub struct PtyReader {
    /// PTY output reader (child → parent).
    reader: Box<dyn Read + Send>,
    /// Forwards raw PTY bytes to the Terminal IO thread.
    byte_tx: Sender<Vec<u8>>,
    /// Shared shutdown flag — set by the IO thread or writer thread on exit.
    shutdown: Arc<AtomicBool>,
}

impl PtyReader {
    /// Create a new PTY byte forwarder.
    pub fn new(
        reader: Box<dyn Read + Send>,
        byte_tx: Sender<Vec<u8>>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            reader,
            byte_tx,
            shutdown,
        }
    }

    /// Spawn the reader thread. Returns a join handle.
    pub fn spawn(self) -> io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || self.run())
    }

    /// Main read loop — runs until PTY closes or shutdown is signaled.
    fn run(mut self) {
        let mut buf = vec![0u8; READ_BUFFER_SIZE];

        loop {
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }

            let n = match self.reader.read(&mut buf) {
                Ok(0) => {
                    log::info!("PTY EOF");
                    break;
                }
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    log::info!("PTY read error, closing reader: {e}");
                    break;
                }
            };

            // Forward the raw bytes to the IO thread.
            if self.byte_tx.send(buf[..n].to_vec()).is_err() {
                // IO thread channel disconnected — shut down.
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests;
