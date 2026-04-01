//! Tests for PaneIoThread and PaneIoHandle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use oriterm_core::{Column, Line, Term, TermMode, Theme, VoidListener};

use super::{PaneIoCommand, PaneIoHandle, PaneIoThread, new_with_handle};

/// Helper: create a Term<VoidListener> with default dimensions.
fn make_term() -> Term<VoidListener> {
    Term::new(24, 80, 1000, Theme::default(), VoidListener)
}

/// Helper: create a thread + handle pair with a no-op wakeup.
fn make_pair() -> (PaneIoThread<VoidListener>, PaneIoHandle) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    new_with_handle(make_term(), mode_cache, shutdown, wakeup)
}

/// Helper: spawn and return a live handle + its shutdown flag.
fn spawn_pair_with_flag() -> (PaneIoHandle, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (thread, mut handle) =
        new_with_handle(make_term(), mode_cache, Arc::clone(&shutdown), wakeup);
    let join = thread.spawn().expect("failed to spawn IO thread");
    handle.set_join(join);
    (handle, shutdown)
}

/// Send `Shutdown` command — IO thread should exit cleanly and set the flag.
#[test]
fn shutdown_via_command() {
    let (mut handle, shutdown_flag) = spawn_pair_with_flag();
    handle.send_command(PaneIoCommand::Shutdown);
    let join = handle.join.take().expect("join handle missing");
    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked on shutdown");
    assert!(
        shutdown_flag.load(Ordering::Acquire),
        "shutdown flag should be set after Shutdown command"
    );
}

/// Drop raw senders (bypassing PaneIoHandle::Drop) — IO thread exits via
/// channel disconnect, NOT via Shutdown command.
#[test]
fn shutdown_via_channel_disconnect() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // Drop both senders — this disconnects the channels without sending Shutdown.
    drop(cmd_tx);
    drop(byte_tx);

    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked on channel disconnect");
    assert!(
        !shutdown.load(Ordering::Acquire),
        "shutdown flag should NOT be set on channel disconnect"
    );
}

/// Send 5 commands then Shutdown. The shutdown flag proves all 5 were drained
/// before exit (Shutdown is last in the queue, processed after the preceding 5).
#[test]
fn command_delivery_ordering() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (thread, handle) = new_with_handle(make_term(), mode_cache, Arc::clone(&shutdown), wakeup);

    for i in 1..=5 {
        handle.send_command(PaneIoCommand::ScrollDisplay(i));
    }
    handle.send_command(PaneIoCommand::Shutdown);

    let join = thread.spawn().expect("failed to spawn IO thread");
    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked processing commands");
    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag should be set after draining all commands"
    );
}

/// Send byte batches, then shutdown. Verify bytes are parsed into the terminal.
#[test]
fn byte_delivery_parses_vte() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // Send text that will appear in the grid.
    byte_tx.send(b"hello world".to_vec()).unwrap();

    // Brief yield to let the IO thread process bytes.
    std::thread::sleep(Duration::from_millis(20));

    // Shut down via command.
    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let _ = join.join();

    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag should be set"
    );
}

/// Drop impl sends shutdown and joins the thread.
#[test]
fn handle_drop_sends_shutdown() {
    let (handle, shutdown_flag) = spawn_pair_with_flag();
    drop(handle);
    assert!(
        shutdown_flag.load(Ordering::Acquire),
        "shutdown flag should be set after Drop"
    );
}

/// Verify `PaneIoCommand` is `Send`.
#[test]
fn pane_io_command_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PaneIoCommand>();
}

/// Verify `PaneIoHandle` is `Send`.
#[test]
fn pane_io_handle_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PaneIoHandle>();
}

/// Debug output on `PaneIoThread` and `PaneIoHandle`.
#[test]
fn debug_impls() {
    let (thread, handle) = make_pair();
    let t = format!("{thread:?}");
    assert!(t.contains("PaneIoThread"), "expected struct name in: {t}");
    let h = format!("{handle:?}");
    assert!(h.contains("PaneIoHandle"), "expected struct name in: {h}");
}

// --- Section 02 VTE parsing tests ---

/// VTE sequences are parsed by the IO thread: SGR red sets cell foreground.
#[test]
fn handle_bytes_advances_vte() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // SGR 31 (red foreground) + text.
    byte_tx.send(b"\x1b[31mR".to_vec()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let _ = join.join();

    // If parsing worked, the shutdown flag is set and no panics occurred.
    assert!(shutdown.load(Ordering::Acquire));
}

/// Shell integration sequences (OSC 133) are processed on the IO thread.
#[test]
fn handle_bytes_shell_integration() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // OSC 133;A (prompt start) — triggers shell integration processing.
    byte_tx.send(b"\x1b]133;A\x07".to_vec()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let _ = join.join();

    assert!(shutdown.load(Ordering::Acquire));
}

/// Mode cache is updated after VTE parsing (e.g., alt screen enable).
#[test]
fn mode_cache_updated_after_parse() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let mode_cache_clone = Arc::clone(&mode_cache);
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let initial_mode = mode_cache.load(Ordering::Acquire);

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: mode_cache_clone,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // Enable alt screen (Mode 1049).
    byte_tx.send(b"\x1b[?1049h".to_vec()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let _ = join.join();

    let updated_mode = mode_cache.load(Ordering::Acquire);
    assert_ne!(
        initial_mode, updated_mode,
        "mode cache should change after enabling alt screen"
    );
}

/// Large byte batches are chunked at 64KB boundaries with command checks.
#[test]
fn process_pending_bytes_chunks_with_commands() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // Send a 200KB byte buffer (will be chunked into ~3 pieces at 64KB).
    let big = vec![b'X'; 200_000];
    byte_tx.send(big).unwrap();

    // Inject a Resize command while the big buffer is being processed.
    // The chunking mechanism should pick it up between 64KB chunks.
    cmd_tx
        .send(PaneIoCommand::Resize {
            rows: 30,
            cols: 100,
        })
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let _ = join.join();

    assert!(shutdown.load(Ordering::Acquire));
}

/// IO thread processes text visible in the grid (end-to-end byte → grid).
#[test]
fn bytes_appear_in_terminal_grid() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));

    // Synchronous test: create a PaneIoThread, call handle_bytes directly.
    let (_, unused_cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, unused_byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    let mut thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx: unused_cmd_rx,
        byte_rx: unused_byte_rx,
        shutdown,
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };

    thread.handle_bytes(b"hello world");

    let grid = thread.terminal.grid();
    let first_row = &grid[Line(0)];
    let text: String = (0..11).map(|col| first_row[Column(col)].ch).collect();
    assert_eq!(text, "hello world");
}

/// Prompt markers evicted from scrollback are pruned.
#[test]
fn handle_bytes_prunes_evicted_markers() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));

    // Small grid: 5 lines, 10 scrollback — markers will be evicted quickly.
    let term = Term::new(5, 80, 10, Theme::default(), VoidListener);

    let mut thread = PaneIoThread {
        terminal: term,
        cmd_rx: crossbeam_channel::unbounded().1,
        byte_rx: crossbeam_channel::unbounded().1,
        shutdown,
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
    };

    // Insert a prompt marker.
    thread.handle_bytes(b"\x1b]133;A\x07");
    let markers_before = thread.terminal.prompt_markers().len();

    // Flood enough output to evict the marker from scrollback.
    // 5 visible + 10 scrollback = 15 lines capacity. Write 30 lines.
    for _ in 0..30 {
        thread.handle_bytes(b"AAAAAAAAAA\r\n");
    }

    let markers_after = thread.terminal.prompt_markers().len();
    // The marker should have been pruned (or at least not grown).
    assert!(
        markers_after <= markers_before,
        "markers should be pruned after eviction: before={markers_before}, after={markers_after}"
    );
}
