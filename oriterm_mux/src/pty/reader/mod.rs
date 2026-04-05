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
/// 128 KB matches `WezTerm`. Smaller buffers cause the reader to return
/// from `read()` more frequently, which drains the `ConPTY` output pipe
/// in smaller chunks. This gives conhost more opportunities to process
/// the input pipe between output flushes — critical for Ctrl+C delivery
/// during sustained output flooding (BUG-11-1). Alacritty uses 1 MB but
/// doesn't use `ConPTY` (it uses non-blocking I/O with polling).
const READ_BUFFER_SIZE: usize = 0x2_0000; // 128 KB

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
    ///
    /// After each read, sleeps 1ms so that `ConPTY`'s conhost can process
    /// the input pipe between output bursts. Without this, the tight read
    /// loop keeps the output pipe drained so aggressively that conhost
    /// never gets a scheduling window to handle input — making Ctrl+C
    /// unresponsive during sustained output flooding (BUG-11-1). `WezTerm`
    /// achieves the same effect by doing VTE parsing inline on its reader
    /// thread (~5-10ms per 128KB chunk).
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

            // ConPTY only: pause between reads so conhost can process input.
            // WezTerm achieves this naturally because its reader does VTE
            // parsing inline (~5-10ms per 128KB chunk). Our reader offloads
            // parsing to the IO thread and loops back to read() instantly,
            // starving conhost's input thread (BUG-11-1). A 1ms sleep
            // after each read gives conhost scheduling time to handle
            // Ctrl+C between output bursts. Throughput impact is minimal:
            // 128KB per 1ms = 128 MB/s, far above terminal needs.
            //
            // Unix PTYs don't need this — the kernel scheduler provides
            // natural interleaving between the child process and our reader.
            #[cfg(windows)]
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests;
