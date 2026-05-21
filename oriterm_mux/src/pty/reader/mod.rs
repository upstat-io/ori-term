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

use super::adopt::{AdoptedPtyHandle, ExitSignal};

/// PTY read buffer size.
/// 128 KB matches `WezTerm`. Smaller buffers cause the reader to return
/// from `read()` more frequently, which drains the `ConPTY` output pipe
/// in smaller chunks. This gives conhost more opportunities to process
/// the input pipe between output flushes — critical for Ctrl+C delivery
/// during sustained output flooding. Alacritty uses 1 MB but
/// doesn't use `ConPTY` (it uses non-blocking I/O with polling).
const READ_BUFFER_SIZE: usize = 0x2_0000; // 128 KB

/// Per-pane memory budget for the reader → IO thread byte queue heap.
/// `BYTE_CHANNEL_CAPACITY` is derived from this so the cap stays in
/// lockstep with `READ_BUFFER_SIZE` if either changes. 1 MiB per pane
/// is the back-pressure trigger point: tighter starves the child via
/// kernel PTY back-pressure; looser defeats the bounded-growth invariant
/// pinned by §03 Pin 1 in .
pub(crate) const BYTE_CHANNEL_MEMORY_BUDGET: usize = 1024 * 1024; // 1 MiB

/// Bounded capacity for the reader → IO thread byte channel, in messages.
/// `BYTE_CHANNEL_MEMORY_BUDGET / READ_BUFFER_SIZE` = 1 MiB / 128 KiB =
/// 8 messages. The reader's `byte_tx.send(...)` blocks when the queue
/// is full, propagating back-pressure through the kernel PTY buffer to
/// the child process. On Windows (`ConPTY`), the existing 1 ms inter-read
/// sleep below is now secondary back-pressure — the bounded `send()` is
/// the primary mechanism. Regression:.
pub(crate) const BYTE_CHANNEL_CAPACITY: usize = BYTE_CHANNEL_MEMORY_BUDGET / READ_BUFFER_SIZE;

const _: () = assert!(
    BYTE_CHANNEL_CAPACITY >= 1,
    "BYTE_CHANNEL_MEMORY_BUDGET too small relative to READ_BUFFER_SIZE",
);

/// PTY byte forwarder — reads shell output and sends to the IO thread.
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
    /// Optional exit signal — set when the reader thread exits via any
    /// path (EOF, error, shutdown). For spawned PTYs this is `None` —
    /// `PtyHandle::wait` blocks on the underlying `Child` directly. For
    /// adopted PTYs (Section 03.9 Windows handoff) this is `Some(_)` so
    /// [`AdoptedPtyHandle::wait`] can wake when the reader observes EOF.
    exit_signal: Option<ExitSignal>,
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
            exit_signal: None,
        }
    }

    /// Attach an exit signal to wake an [`AdoptedPtyHandle::wait`] caller
    /// when the reader thread exits.
    /// Used by `oriterm_mux::domain::handoff::adopt_pane` for Section 03.9
    /// Windows Default Terminal handoff. Spawned PTYs do not call this.
    #[must_use]
    pub fn with_exit_signal(mut self, signal: ExitSignal) -> Self {
        self.exit_signal = Some(signal);
        self
    }

    /// Spawn the reader thread. Returns a join handle.
    pub fn spawn(self) -> io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || self.run())
    }

    /// Main read loop — runs until PTY closes or shutdown is signaled.
    /// After each read, sleeps 1ms so that `ConPTY`'s conhost can process
    /// the input pipe between output bursts. Without this, the tight read
    /// loop keeps the output pipe drained so aggressively that conhost
    /// never gets a scheduling window to handle input — making Ctrl+C
    /// unresponsive during sustained output flooding. `WezTerm`
    /// achieves the same effect by doing VTE parsing inline on its reader
    /// thread (~5-10ms per 128KB chunk).
    fn run(mut self) {
        self.read_loop();

        // Signal exit to any AdoptedPtyHandle::wait caller. Runs on every
        // exit path (EOF, error, shutdown) because read_loop returns
        // before this point in all cases.
        if let Some(signal) = self.exit_signal.take() {
            AdoptedPtyHandle::deliver_exit(&signal);
        }
    }

    /// Inner read loop — extracted so [`Self::run`] can run cleanup after
    /// it returns regardless of which exit path was taken.
    fn read_loop(&mut self) {
        let mut buf = vec![0u8; READ_BUFFER_SIZE];
        // Throughput tracking: log bytes/sec read from PTY every 1 sec when
        // sustained over 256 KB/sec. Localizes whether ConPTY/WSL upstream
        // is the bottleneck (low MB/s observed) or our IO thread/parser is
        // (high MB/s observed but still drops downstream).
        let mut throughput_window_start = std::time::Instant::now();
        let mut throughput_bytes: usize = 0;

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

            throughput_bytes += n;
            let window_elapsed = throughput_window_start.elapsed();
            if window_elapsed >= std::time::Duration::from_secs(1) {
                let kb_per_sec = (throughput_bytes as f64 / 1024.0) / window_elapsed.as_secs_f64();
                if throughput_bytes >= 256 * 1024 {
                    log::info!(
                    target: "oriterm_mux::pty::reader::throughput",
                    "pty read throughput {kb_per_sec:.0} KB/s ({} bytes in {:.2?})",
                    throughput_bytes,
                    window_elapsed,
                    );
                }
                throughput_window_start = std::time::Instant::now();
                throughput_bytes = 0;
            }

            // Forward the raw bytes to the IO thread. Measure send-block
            // duration — when `byte_tx` is full (CAP 8), `send` blocks
            // waiting for the IO thread to drain. Sustained block durations
            // >= 10ms indicate backpressure from a saturated consumer.
            let send_start = std::time::Instant::now();
            if self.byte_tx.send(buf[..n].to_vec()).is_err() {
                // IO thread channel disconnected — shut down.
                break;
            }
            let send_blocked_ms = send_start.elapsed().as_millis();
            if send_blocked_ms >= 10 {
                log::warn!(
                target: "oriterm_mux::pty::reader::send_block",
                "byte_tx.send blocked {send_blocked_ms}ms (IO thread backpressure; consumer slower than producer)"
                );
            }

            // ConPTY only: pause between reads so conhost can process input.
            // WezTerm achieves this naturally because its reader does VTE
            // parsing inline (~5-10ms per 128KB chunk). Our reader offloads
            // parsing to the IO thread and loops back to read() instantly,
            // starving conhost's input thread. A 1ms sleep
            // after each read gives conhost scheduling time to handle
            // Ctrl+C between output bursts.
            // Conditional: skip the sleep when this read returned any
            // substantial batch (> 4 KB). ConPTY's slave→master shuttle
            // delivers in chunks well below the 128 KB read buffer; a
            // 64 KB threshold misclassified 25 MB/s graphics streams as
            // "partial" and the sleep still fired per read, capping
            // throughput. 4 KB is small enough that any real burst skips
            // the sleep, yet large enough that key-echo (typically 1-32
            // bytes) and idle wakeups still hit it — preserving the
            // conhost-scheduling slot that makes Ctrl+C responsive during
            // text floods.
            // Unix PTYs don't need this — the kernel scheduler provides
            // natural interleaving between the child process and our reader.
            #[cfg(windows)]
            if n < 4096 {
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
}

#[cfg(test)]
mod tests;
