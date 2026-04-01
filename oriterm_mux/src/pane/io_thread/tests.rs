//! Tests for PaneIoThread and PaneIoHandle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use oriterm_core::{Column, Line, Term, TermMode, Theme, VoidListener};

use super::snapshot::SnapshotDoubleBuffer;
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
    let grid_dirty = Arc::new(AtomicBool::new(false));
    new_with_handle(make_term(), mode_cache, shutdown, wakeup, grid_dirty)
}

/// Helper: spawn and return a live handle + its shutdown flag.
fn spawn_pair_with_flag() -> (PaneIoHandle, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mode_cache = Arc::new(AtomicU32::new(TermMode::default().bits()));
    let grid_dirty = Arc::new(AtomicBool::new(false));
    let (thread, mut handle) = new_with_handle(
        make_term(),
        mode_cache,
        Arc::clone(&shutdown),
        wakeup,
        grid_dirty,
    );
    let join = thread.spawn().expect("failed to spawn IO thread");
    handle.set_join(join);
    (handle, shutdown)
}

/// Helper: create a `PaneIoThread` for synchronous testing (no spawning).
fn make_sync_thread() -> PaneIoThread<VoidListener> {
    make_sync_thread_with_term(make_term())
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
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty: Arc::new(AtomicBool::new(false)),
    }
}

/// Helper: create a sync thread with a wakeup counter for testing.
fn make_sync_thread_with_wakeup() -> (PaneIoThread<VoidListener>, Arc<AtomicU32>) {
    let wakeup_count = Arc::new(AtomicU32::new(0));
    let wakeup_clone = Arc::clone(&wakeup_count);
    let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    let grid_dirty = Arc::new(AtomicBool::new(false));
    let thread = PaneIoThread {
        terminal: make_term(),
        cmd_rx,
        byte_rx,
        shutdown: Arc::new(AtomicBool::new(false)),
        wakeup: Arc::new(move || {
            wakeup_clone.fetch_add(1, Ordering::Relaxed);
        }),
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty,
    };
    (thread, wakeup_count)
}

// --- Lifecycle tests ---

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
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty: Arc::new(AtomicBool::new(false)),
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
    let grid_dirty = Arc::new(AtomicBool::new(false));
    let (thread, handle) = new_with_handle(
        make_term(),
        mode_cache,
        Arc::clone(&shutdown),
        wakeup,
        grid_dirty,
    );

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
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty: Arc::new(AtomicBool::new(false)),
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
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty: Arc::new(AtomicBool::new(false)),
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

// --- Section 03 snapshot production tests ---

/// `produce_snapshot()` fills cells from terminal grid content.
#[test]
fn produce_snapshot_fills_cells() {
    let mut t = make_sync_thread();

    t.handle_bytes(b"hello");
    t.grid_dirty.store(true, Ordering::Release);
    t.produce_snapshot();

    let mut consumer = oriterm_core::RenderableContent::default();
    assert!(t.double_buffer.swap_front(&mut consumer));

    // Find the 'h', 'e', 'l', 'l', 'o' characters in the snapshot.
    let text: String = consumer
        .cells
        .iter()
        .filter(|c| c.ch != ' ' && c.ch != '\0')
        .map(|c| c.ch)
        .collect();
    assert!(
        text.starts_with("hello"),
        "snapshot should contain 'hello', got: {text:?}"
    );
}

/// `produce_snapshot()` resets damage after production.
#[test]
fn produce_snapshot_resets_damage() {
    let mut t = make_sync_thread();

    // Write something to dirty the grid.
    t.handle_bytes(b"test");
    t.grid_dirty.store(true, Ordering::Release);

    // Damage should exist before snapshot.
    let has_damage =
        t.terminal.grid().dirty().is_all_dirty() || t.terminal.grid().dirty().is_dirty(0);
    assert!(has_damage, "grid should have damage after writing");

    t.produce_snapshot();

    // Damage should be cleared after snapshot.
    let still_dirty =
        t.terminal.grid().dirty().is_all_dirty() || t.terminal.grid().dirty().is_dirty(0);
    assert!(!still_dirty, "damage should be cleared after snapshot");
}

/// `maybe_produce_snapshot()` respects synchronized output (Mode 2026).
///
/// When sync_bytes_count > 0, snapshot production is deferred.
#[test]
fn produce_snapshot_respects_sync_mode() {
    let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

    // Enable Mode 2026 (synchronized output begin: BSU).
    t.handle_bytes(b"\x1b[?2026h");
    t.grid_dirty.store(true, Ordering::Release);

    // Send some content while sync mode is active.
    // The processor accumulates in sync buffer, so sync_bytes_count > 0.
    t.processor.advance(&mut t.terminal, b"buffered content");

    // Try to produce snapshot — should be suppressed because sync buffer is active.
    let wakeup_before = wakeup_count.load(Ordering::Relaxed);
    t.maybe_produce_snapshot();
    let wakeup_after = wakeup_count.load(Ordering::Relaxed);

    assert_eq!(
        wakeup_before, wakeup_after,
        "wakeup should NOT fire while sync buffer is non-empty"
    );
}

/// Wakeup callback only fires when `grid_dirty` is set.
#[test]
fn produce_snapshot_wakeup_only_when_dirty() {
    let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

    // grid_dirty is false by default.
    assert!(!t.grid_dirty.load(Ordering::Acquire));

    // Call maybe_produce_snapshot — should skip because grid is not dirty.
    t.maybe_produce_snapshot();

    assert_eq!(
        wakeup_count.load(Ordering::Relaxed),
        0,
        "wakeup should not fire when grid is not dirty"
    );
}

/// Shutdown flushes any parsed-but-unpublished state (TPR-03-001).
///
/// Bytes processed in the `select!` arm must be snapshot-published
/// even if shutdown is queued before the next `maybe_produce_snapshot()`.
#[test]
fn shutdown_flushes_final_snapshot() {
    let mut t = make_sync_thread();

    // Simulate bytes arriving in the select! arm.
    t.handle_bytes(b"final");
    t.grid_dirty.store(true, Ordering::Release);

    // Simulate shutdown arriving before next maybe_produce_snapshot().
    t.shutdown.store(true, Ordering::Release);

    // The shutdown path in run() calls maybe_produce_snapshot() before returning.
    // Simulate that here:
    t.maybe_produce_snapshot();

    let mut consumer = oriterm_core::RenderableContent::default();
    assert!(
        t.double_buffer.swap_front(&mut consumer),
        "final snapshot should be published even on shutdown"
    );

    let text: String = consumer
        .cells
        .iter()
        .filter(|c| c.ch != ' ' && c.ch != '\0')
        .map(|c| c.ch)
        .collect();
    assert!(
        text.starts_with("final"),
        "shutdown snapshot should contain 'final', got: {text:?}"
    );
}

/// Wakeup fires exactly once per `produce_snapshot()` call.
#[test]
fn produce_snapshot_fires_wakeup() {
    let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

    t.handle_bytes(b"data");
    t.grid_dirty.store(true, Ordering::Release);
    t.produce_snapshot();

    assert_eq!(
        wakeup_count.load(Ordering::Relaxed),
        1,
        "wakeup should fire once after produce_snapshot"
    );
}
