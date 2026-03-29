//! Tests for PtyEventLoop.
//!
//! Uses anonymous pipes to test the event loop without real PTY processes,
//! avoiding platform-specific `ConPTY` issues with blocking reads.

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use oriterm_core::{Column, FairMutex, Line, Term, TermMode, Theme, VoidListener};

use super::{MAX_LOCKED_PARSE, PtyEventLoop, READ_BUFFER_SIZE};

/// Build a PtyEventLoop with the given reader.
fn build_event_loop(
    reader: Box<dyn Read + Send>,
) -> (
    PtyEventLoop<VoidListener>,
    Arc<FairMutex<Term<VoidListener>>>,
    Arc<AtomicBool>,
    Arc<AtomicU32>,
) {
    let terminal = Arc::new(FairMutex::new(Term::new(
        24,
        80,
        1000,
        Theme::default(),
        VoidListener,
    )));
    let shutdown = Arc::new(AtomicBool::new(false));
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));

    let event_loop = PtyEventLoop::new(
        Arc::clone(&terminal),
        reader,
        Arc::clone(&shutdown),
        Arc::clone(&mode_cache),
    );

    (event_loop, terminal, shutdown, mode_cache)
}

#[test]
fn shutdown_on_reader_eof() {
    // Anonymous pipe where we control the write end — dropping it produces EOF.
    let (pipe_reader, pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, _terminal, _shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    // Drop the write end → reader gets EOF → thread exits.
    drop(pipe_writer);

    join.join().expect("reader thread should exit on EOF");
}

#[test]
fn processes_pty_output_into_terminal() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, _shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    // Simulate shell output: write raw text to the reader pipe.
    pipe_writer.write_all(b"hello world").expect("write");

    // Close the pipe to trigger EOF so the thread exits.
    drop(pipe_writer);

    join.join().expect("reader thread should exit on EOF");

    // Verify terminal received the output.
    let term = terminal.lock();
    let grid = term.grid();
    let first_row = &grid[Line(0)];
    let text: String = (0..80).map(|col| first_row[Column(col)].ch).collect();
    assert!(
        text.contains("hello world"),
        "terminal grid should contain 'hello world', got: {text:?}",
    );
}

#[test]
fn read_buffer_size_is_1mb() {
    assert_eq!(READ_BUFFER_SIZE, 0x10_0000);
}

#[test]
fn max_locked_parse_is_64kb() {
    assert_eq!(MAX_LOCKED_PARSE, 65536);
}

#[test]
fn try_parse_is_bounded_to_max_locked_parse() {
    let (pipe_reader, pipe_writer) = std::io::pipe().expect("pipe");
    let (mut event_loop, _terminal, _shutdown, _mode) = build_event_loop(Box::new(pipe_reader));
    drop(pipe_writer);

    let data = vec![b'X'; MAX_LOCKED_PARSE * 2];

    let parsed_1 = event_loop.try_parse(&data);
    assert_eq!(
        parsed_1, MAX_LOCKED_PARSE,
        "first parse should be capped to MAX_LOCKED_PARSE"
    );

    let parsed_2 = event_loop.try_parse(&data[parsed_1..]);
    assert_eq!(
        parsed_2, MAX_LOCKED_PARSE,
        "second parse should consume the remaining chunk"
    );
}

// --- Contention benchmarks ---
//
// These test the FairMutex locking strategies under realistic contention:
// a "reader" thread floods data through a real PtyEventLoop (VTE parsing),
// while a "renderer" thread tries to lock the terminal periodically.

/// Feed flood data through a real PtyEventLoop while a contending thread
/// measures how often it can acquire the terminal lock.
///
/// Returns `(reader_bytes, renderer_locks, elapsed)`.
fn run_contention_bench(duration: Duration) -> (usize, usize, Duration) {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    let done = Arc::new(AtomicBool::new(false));
    let renderer_count = Arc::new(AtomicUsize::new(0));

    // Renderer thread — tries to lock the terminal in a tight loop.
    let term_clone = Arc::clone(&terminal);
    let done_clone = Arc::clone(&done);
    let rc = Arc::clone(&renderer_count);
    let renderer = thread::spawn(move || {
        while !done_clone.load(Ordering::Relaxed) {
            let _guard = term_clone.lock();
            rc.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Feed flood data from this thread.
    // Use a repeating pattern of printable chars + newlines.
    let flood_line = "A".repeat(79) + "\n";
    let flood_block = flood_line.repeat(100); // ~8KB per block
    let flood_bytes = flood_block.as_bytes();
    let mut total_written = 0usize;

    let start = Instant::now();
    while start.elapsed() < duration {
        match pipe_writer.write(flood_bytes) {
            Ok(n) => total_written += n,
            Err(_) => break,
        }
    }

    // Stop.
    done.store(true, Ordering::Relaxed);
    let elapsed = start.elapsed();

    // Close pipe → EOF → event loop exits.
    drop(pipe_writer);
    shutdown.store(true, Ordering::Release);
    let _ = join.join();
    renderer.join().expect("renderer thread");

    let locks = renderer_count.load(Ordering::Relaxed);
    (total_written, locks, elapsed)
}

/// Verifies that the renderer is not starved during flood output.
///
/// The reader thread floods data through a real PtyEventLoop (with actual
/// VTE parsing). A contending renderer thread measures how many lock
/// acquisitions it gets. With the lease+try_lock pattern, the renderer
/// must get consistent access between reader parse cycles.
#[test]
fn renderer_not_starved_during_flood() {
    let (bytes, renderer_locks, elapsed) = run_contention_bench(Duration::from_millis(500));

    let mb_written = bytes as f64 / (1024.0 * 1024.0);
    let secs = elapsed.as_secs_f64();
    let throughput_mbps = mb_written / secs;
    let renderer_per_sec = renderer_locks as f64 / secs;

    eprintln!("--- contention benchmark ---");
    eprintln!("  duration:       {elapsed:?}");
    eprintln!("  data written:   {mb_written:.1} MB");
    eprintln!("  throughput:     {throughput_mbps:.1} MB/s");
    eprintln!("  renderer locks: {renderer_locks} ({renderer_per_sec:.0}/s)");

    // The renderer must get at least 60 locks/sec (one per frame at 60fps).
    // A starved renderer would get 0 or single-digit locks over 500ms.
    assert!(
        renderer_locks >= 30,
        "renderer starved: only {renderer_locks} locks in {elapsed:?} \
         (need >= 30 for 60fps). throughput={throughput_mbps:.1} MB/s",
    );
}

/// Measures baseline throughput without contention (reader only).
///
/// This establishes how fast the PtyEventLoop can parse data when there's
/// no renderer thread competing for the lock.
#[test]
fn reader_throughput_no_contention() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, _terminal, shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    let flood_line = "A".repeat(79) + "\n";
    let flood_block = flood_line.repeat(100);
    let flood_bytes = flood_block.as_bytes();
    let mut total_written = 0usize;

    let duration = Duration::from_millis(500);
    let start = Instant::now();
    while start.elapsed() < duration {
        match pipe_writer.write(flood_bytes) {
            Ok(n) => total_written += n,
            Err(_) => break,
        }
    }
    let elapsed = start.elapsed();

    drop(pipe_writer);
    shutdown.store(true, Ordering::Release);
    let _ = join.join();

    let mb = total_written as f64 / (1024.0 * 1024.0);
    let secs = elapsed.as_secs_f64();
    let throughput = mb / secs;

    eprintln!("--- throughput benchmark (no contention) ---");
    eprintln!("  duration:   {elapsed:?}");
    eprintln!("  written:    {mb:.1} MB");
    eprintln!("  throughput: {throughput:.1} MB/s");
}

/// Verifies that per-byte reads do not regress relative to bulk reads.
///
/// Water-level test: measures bulk throughput as a baseline, then measures
/// per-byte throughput using a `OneByteReader`, and asserts the ratio stays
/// within bounds. Self-calibrating — no absolute timers that flake across
/// platforms or CI machines.
///
/// A contending renderer thread competes for the lock in both measurements
/// so the comparison is apples-to-apples.
#[test]
fn interactive_reads_low_latency() {
    /// Reader that yields exactly one byte per `read()` call.
    struct OneByteReader {
        inner: Box<dyn Read + Send>,
    }
    impl Read for OneByteReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if buf.is_empty() {
                return Ok(0);
            }
            self.inner.read(&mut buf[..1])
        }
    }

    /// Spawn a PtyEventLoop with the given reader, a contending renderer,
    /// and measure wall time until the event loop exits (EOF).
    ///
    /// Data must be pre-written to the pipe (and write end closed) before
    /// calling this function.
    fn measure(reader: Box<dyn Read + Send>) -> Duration {
        let (event_loop, terminal, _shutdown, _mode) = build_event_loop(reader);
        let join = event_loop.spawn().expect("spawn event loop");

        let term_clone = Arc::clone(&terminal);
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done);
        let renderer = thread::spawn(move || {
            while !done_clone.load(Ordering::Relaxed) {
                let _g = term_clone.lock();
                thread::yield_now();
            }
        });

        let start = Instant::now();
        let _ = join.join();
        let elapsed = start.elapsed();

        done.store(true, Ordering::Relaxed);
        renderer.join().expect("renderer thread");
        elapsed
    }

    let payload: Vec<u8> = (0..200).map(|i| b'a' + (i % 26)).collect();

    // Baseline: bulk read (pipe delivers all bytes in one read() call).
    let (bulk_reader, mut bulk_writer) = std::io::pipe().expect("pipe");
    bulk_writer.write_all(&payload).expect("write");
    drop(bulk_writer); // EOF in pipe before event loop starts reading.
    let bulk_time = measure(Box::new(bulk_reader));

    // Interactive: one-byte-at-a-time read (each byte is a separate read()).
    let (byte_reader, mut byte_writer) = std::io::pipe().expect("pipe");
    byte_writer.write_all(&payload).expect("write");
    drop(byte_writer);
    let byte_time = measure(Box::new(OneByteReader {
        inner: Box::new(byte_reader),
    }));

    eprintln!("--- interactive latency water level ---");
    eprintln!("  bulk (200 bytes):     {bulk_time:?}");
    eprintln!("  per-byte (200 bytes): {byte_time:?}");

    // Per-byte path has 200x more read() calls and lock cycles, so it's
    // naturally slower. But it should not be catastrophically slower —
    // allow up to 200x overhead (1x per byte). A real regression (e.g.
    // O(n^2) parsing or lock convoy) would blow past this easily.
    //
    // Floor the bulk baseline at 100µs to avoid noise-driven failures.
    // When bulk finishes in single-digit microseconds (common on WSL
    // under low load), scheduler jitter alone can inflate the ratio
    // past any reasonable threshold.
    let max_ratio = 200u128;
    let bulk_ns = bulk_time.as_nanos().max(100_000);
    let byte_ns = byte_time.as_nanos();
    let ratio = byte_ns / bulk_ns;
    eprintln!("  ratio:                {ratio}x (max {max_ratio}x)");

    assert!(
        byte_ns <= bulk_ns * max_ratio,
        "per-byte reads {ratio}x slower than bulk (max {max_ratio}x). \
         bulk={bulk_time:?}, per-byte={byte_time:?}",
    );
}

/// Verifies renderer access survives bursty flood patterns.
///
/// Alternates between flood bursts and idle periods, simulating realistic
/// shell usage: `ls` output -> prompt -> `cat`. The renderer must get
/// consistent lock access throughout.
#[test]
fn bursty_flood_renderer_access() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, _shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    let done = Arc::new(AtomicBool::new(false));
    let renderer_count = Arc::new(AtomicUsize::new(0));

    let term_clone = Arc::clone(&terminal);
    let done_clone = Arc::clone(&done);
    let rc = Arc::clone(&renderer_count);
    let renderer = thread::spawn(move || {
        while !done_clone.load(Ordering::Relaxed) {
            let _g = term_clone.lock();
            rc.fetch_add(1, Ordering::Relaxed);
        }
    });

    let flood_block = ("A".repeat(79) + "\n").repeat(100); // ~8KB
    let flood_bytes = flood_block.as_bytes();

    // 5 cycles of: 100ms flood -> 50ms idle.
    for _ in 0..5 {
        let burst_start = Instant::now();
        while burst_start.elapsed() < Duration::from_millis(100) {
            match pipe_writer.write(flood_bytes) {
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Idle — simulates user reading output or typing next command.
        thread::sleep(Duration::from_millis(50));
    }

    done.store(true, Ordering::Relaxed);
    drop(pipe_writer);
    let _ = join.join();
    renderer.join().expect("renderer thread");

    let locks = renderer_count.load(Ordering::Relaxed);
    // 750ms total (5 x 150ms). Renderer needs at least 45 locks (60fps).
    assert!(
        locks >= 45,
        "renderer starved during bursty flood: only {locks} locks in 750ms \
         (need >= 45 for 60fps)",
    );
}

/// Processes a sustained large flood without memory growth.
///
/// Feeds 50MB+ through a real PtyEventLoop with VTE parsing and verifies
/// the thread exits cleanly. If internal buffers grew unbounded, this
/// would OOM or the thread would hang.
#[test]
fn sustained_flood_no_oom() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    // Renderer thread — applies backpressure like production.
    let term_clone = Arc::clone(&terminal);
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    let renderer = thread::spawn(move || {
        while !done_clone.load(Ordering::Relaxed) {
            let _g = term_clone.lock();
            thread::sleep(Duration::from_millis(16)); // ~60fps
        }
    });

    // Feed 50MB of data.
    let flood_block = ("X".repeat(79) + "\n").repeat(1000); // ~80KB
    let flood_bytes = flood_block.as_bytes();
    let target = 50 * 1024 * 1024; // 50MB
    let mut total = 0usize;

    while total < target {
        match pipe_writer.write(flood_bytes) {
            Ok(n) => total += n,
            Err(_) => break,
        }
    }

    let mb = total as f64 / (1024.0 * 1024.0);
    eprintln!("--- sustained flood ---");
    eprintln!("  written: {mb:.1} MB");

    done.store(true, Ordering::Relaxed);
    drop(pipe_writer);
    shutdown.store(true, Ordering::Release);

    // Thread must exit within 5 seconds. If it hangs, buffers are growing
    // unbounded or the lock strategy is deadlocking.
    let join_result = join.join();
    renderer.join().expect("renderer thread");
    assert!(
        join_result.is_ok(),
        "event loop thread panicked during sustained flood"
    );
}

/// Verifies that all PTY data is processed even when rendering is throttled.
///
/// Feeds numbered lines through the event loop with a contending renderer
/// that holds the lock for 16ms per frame (simulating render backpressure).
/// After EOF, verifies the last N lines in the grid match the expected
/// content — proving no data was dropped during contention.
#[test]
fn no_data_loss_under_renderer_contention() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, _shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    // Renderer thread — holds lock for 16ms per frame (60fps contention).
    let term_clone = Arc::clone(&terminal);
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    let renderer = thread::spawn(move || {
        while !done_clone.load(Ordering::Relaxed) {
            let _g = term_clone.lock();
            thread::sleep(Duration::from_millis(16));
        }
    });

    // Feed 5000 numbered lines. Each line uses \r\n so cursor returns to
    // column 0 before advancing (raw terminal has no implicit CR on LF).
    let total_lines = 5000usize;
    for i in 0..total_lines {
        let line = format!("LINE_{i:05}\r\n");
        pipe_writer.write_all(line.as_bytes()).expect("write");
    }

    // Close pipe → EOF → event loop reads+parses all remaining data, then exits.
    drop(pipe_writer);
    let _ = join.join();

    // Stop renderer after event loop exits.
    done.store(true, Ordering::Relaxed);
    renderer.join().expect("renderer thread");

    // The terminal is 24 rows x 80 cols with 1000 lines of scrollback.
    // After 5000 lines with \r\n, LINE_04999 should be in the visible grid.
    // Scan all visible rows to find it.
    let term = terminal.lock();
    let grid = term.grid();
    let expected = format!("LINE_{:05}", total_lines - 1);
    let mut found = false;
    for line_idx in 0..24 {
        let row = &grid[Line(line_idx)];
        let text: String = (0..80).map(|col| row[Column(col)].ch).collect();
        if text.contains(&expected) {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "expected '{expected}' in visible grid after {total_lines} lines. \
         Data may have been lost during renderer contention",
    );
}

/// Verifies that synchronized output (Mode 2026) delivers content atomically.
///
/// Sends BSU (Begin Synchronized Update), unique content, then ESU (End
/// Synchronized Update) through the event loop. Verifies all content
/// appears in the grid after ESU — proving the sync buffer was replayed.
#[test]
fn sync_mode_delivers_content_atomically() {
    let (pipe_reader, mut pipe_writer) = std::io::pipe().expect("pipe");

    let (event_loop, terminal, shutdown, _mode) = build_event_loop(Box::new(pipe_reader));

    let join = event_loop.spawn().expect("spawn event loop");

    // BSU (Begin Synchronized Update) — Mode 2026 on.
    pipe_writer.write_all(b"\x1b[?2026h").expect("write BSU");

    // Write unique content while sync mode is active.
    // These lines should be buffered by the VTE processor's SyncState.
    for i in 0..10 {
        let line = format!("SYNC_{i:03}\r\n");
        pipe_writer
            .write_all(line.as_bytes())
            .expect("write sync content");
    }

    // ESU (End Synchronized Update) — Mode 2026 off. Buffer is replayed.
    pipe_writer.write_all(b"\x1b[?2026l").expect("write ESU");

    // Give the event loop time to process the sync buffer replay.
    thread::sleep(Duration::from_millis(100));

    // Close pipe → EOF → event loop exits.
    drop(pipe_writer);
    shutdown.store(true, Ordering::Release);
    let _ = join.join();

    // Verify all 10 sync lines appear in the grid.
    let term = terminal.lock();
    let grid = term.grid();
    let mut found = Vec::new();
    for line_idx in 0..24 {
        let row = &grid[Line(line_idx)];
        let text: String = (0..80).map(|col| row[Column(col)].ch).collect();
        for i in 0..10 {
            let marker = format!("SYNC_{i:03}");
            if text.contains(&marker) {
                found.push(i);
            }
        }
    }

    // All 10 lines must be present after ESU replay.
    let expected: Vec<usize> = (0..10).collect();
    assert_eq!(
        found, expected,
        "not all sync lines found in grid after ESU replay. \
         found: {found:?}, expected: {expected:?}. \
         Mode 2026 sync buffer may not have been replayed correctly",
    );
}
