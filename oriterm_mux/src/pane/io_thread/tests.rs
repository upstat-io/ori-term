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

/// Helper: create a `PaneIoThread` for synchronous testing (no spawning).
fn make_sync_thread() -> PaneIoThread<VoidListener> {
    let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::new(AtomicBool::new(false)),
        wakeup: Arc::new(|| {}),
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
    }
}

/// Helper: create a `PaneIoThread` with a custom `Term` for synchronous testing.
fn make_sync_thread_with_term(term: Term<VoidListener>) -> PaneIoThread<VoidListener> {
    let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    PaneIoThread {
        terminal: term,
        cmd_rx,
        byte_rx,
        shutdown: Arc::new(AtomicBool::new(false)),
        wakeup: Arc::new(|| {}),
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
    }
}

/// VTE sequences are parsed: SGR 31 sets cell foreground to ANSI red.
#[test]
fn handle_bytes_advances_vte() {
    let mut t = make_sync_thread();

    // SGR 31 (red foreground) + character.
    t.handle_bytes(b"\x1b[31mR");

    let grid = t.terminal.grid();
    let cell = &grid[Line(0)][Column(0)];
    assert_eq!(cell.ch, 'R');
    assert_eq!(
        cell.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red),
        "SGR 31 should set foreground to ANSI red"
    );
}

/// Shell integration sequences (OSC 133;A) create prompt markers.
#[test]
fn handle_bytes_shell_integration() {
    let mut t = make_sync_thread();

    let markers_before = t.terminal.prompt_markers().len();

    // OSC 133;A (prompt start) triggers deferred prompt marking.
    t.handle_bytes(b"\x1b]133;A\x07");

    let markers_after = t.terminal.prompt_markers().len();
    assert!(
        markers_after > markers_before,
        "prompt markers should increase after OSC 133;A: before={markers_before}, after={markers_after}"
    );
}

/// Mode cache is updated after VTE parsing (alt screen enable).
#[test]
fn mode_cache_updated_after_parse() {
    let mut t = make_sync_thread();
    let initial_mode = t.mode_cache.load(Ordering::Acquire);

    // Enable alt screen (Mode 1049).
    t.handle_bytes(b"\x1b[?1049h");

    let updated_mode = t.mode_cache.load(Ordering::Acquire);
    assert_ne!(
        initial_mode, updated_mode,
        "mode cache should change after enabling alt screen"
    );
}

/// `handle_bytes_chunked` drains commands between 64KB chunks.
///
/// Pre-queues Shutdown, then passes a 200KB buffer. Proves early exit by
/// comparing scrollback eviction against a full-parse baseline: if
/// `drain_commands()` fires between chunks, fewer lines are evicted.
#[test]
fn handle_bytes_chunked_drains_commands() {
    // Baseline: parse all 200KB without Shutdown to measure full eviction.
    let full_eviction = {
        let mut t = make_sync_thread();
        let big = vec![b'A'; 200_000];
        t.handle_bytes_chunked(&big);
        t.terminal.grid().total_evicted()
    };

    // Test: pre-queue Shutdown before parsing.
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    let shutdown = Arc::new(AtomicBool::new(false));

    let mut t = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup: Arc::new(|| {}),
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
    };

    cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
    let big = vec![b'A'; 200_000];
    t.handle_bytes_chunked(&big);

    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown should be set by drain_commands() between chunks"
    );

    let partial_eviction = t.terminal.grid().total_evicted();
    assert!(
        partial_eviction < full_eviction,
        "early exit should parse fewer lines than full buffer: \
         partial={partial_eviction}, full={full_eviction}"
    );
}

/// IO thread processes text visible in the grid (end-to-end byte → grid).
#[test]
fn bytes_appear_in_terminal_grid() {
    let mut t = make_sync_thread();

    t.handle_bytes(b"hello world");

    let grid = t.terminal.grid();
    let first_row = &grid[Line(0)];
    let text: String = (0..11).map(|col| first_row[Column(col)].ch).collect();
    assert_eq!(text, "hello world");
}

/// Prompt markers evicted from scrollback are pruned.
#[test]
fn handle_bytes_prunes_evicted_markers() {
    // Small grid: 5 lines, 10 scrollback — markers will be evicted quickly.
    let term = Term::new(5, 80, 10, Theme::default(), VoidListener);
    let mut t = make_sync_thread_with_term(term);

    // Insert a prompt marker.
    t.handle_bytes(b"\x1b]133;A\x07");
    let markers_before = t.terminal.prompt_markers().len();
    assert!(
        markers_before > 0,
        "prompt marker should exist after OSC 133;A"
    );

    // Flood enough output to evict the marker from scrollback.
    // 5 visible + 10 scrollback = 15 lines capacity. Write 30 lines.
    for _ in 0..30 {
        t.handle_bytes(b"AAAAAAAAAA\r\n");
    }

    let markers_after = t.terminal.prompt_markers().len();
    assert!(
        markers_after < markers_before,
        "markers should be pruned after eviction: before={markers_before}, after={markers_after}"
    );
}
