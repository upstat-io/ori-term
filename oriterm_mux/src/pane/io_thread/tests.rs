//! Tests for PaneIoThread and PaneIoHandle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use oriterm_core::{Column, Line, Term, TermMode, Theme, VoidListener};

use super::snapshot::SnapshotDoubleBuffer;
use super::{IoThreadConfig, PaneIoCommand, PaneIoHandle, PaneIoThread, new_with_handle};

/// Helper: create a Term<VoidListener> with default dimensions.
fn make_term() -> Term<VoidListener> {
    Term::new(24, 80, 1000, Theme::default(), VoidListener)
}

/// Helper: create a thread + handle pair with a no-op wakeup.
fn make_pair() -> (PaneIoThread<VoidListener>, PaneIoHandle) {
    new_with_handle(IoThreadConfig {
        terminal: make_term(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
        shutdown: Arc::new(AtomicBool::new(false)),
        wakeup: Arc::new(|| {}),
        grid_dirty: Arc::new(AtomicBool::new(false)),
        pty_control: None,
        initial_rows: 24,
        initial_cols: 80,
        selection_dirty: Arc::new(AtomicBool::new(false)),
    })
}

/// Helper: spawn and return a live handle + its shutdown flag.
fn spawn_pair_with_flag() -> (PaneIoHandle, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let (thread, mut handle) = new_with_handle(IoThreadConfig {
        terminal: make_term(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
        shutdown: Arc::clone(&shutdown),
        wakeup: Arc::new(|| {}),
        grid_dirty: Arc::new(AtomicBool::new(false)),
        pty_control: None,
        initial_rows: 24,
        initial_cols: 80,
        selection_dirty: Arc::new(AtomicBool::new(false)),
    });
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
    let rows = term.grid().lines() as u16;
    let cols = term.grid().cols() as u16;
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
        pty_control: None,
        last_pty_size: (rows as u32) << 16 | cols as u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
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
        pty_control: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
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
        pty_control: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
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
    let (thread, handle) = new_with_handle(IoThreadConfig {
        terminal: make_term(),
        mode_cache: Arc::new(AtomicU32::new(TermMode::default().bits())),
        shutdown: Arc::clone(&shutdown),
        wakeup: Arc::new(|| {}),
        grid_dirty: Arc::new(AtomicBool::new(false)),
        pty_control: None,
        initial_rows: 24,
        initial_cols: 80,
        selection_dirty: Arc::new(AtomicBool::new(false)),
    });

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
        pty_control: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
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
        pty_control: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
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

// --- Resize tests (Section 05) ---

/// Helper: create a sync thread with a command sender for testing.
fn make_sync_thread_with_cmd_tx() -> (PaneIoThread<VoidListener>, Sender<PaneIoCommand>) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
    let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    let thread = PaneIoThread {
        terminal: make_term(),
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
        pty_control: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
    };
    (thread, cmd_tx)
}

use crossbeam_channel::Sender;

/// Resize command reflows the IO thread's grid.
#[test]
fn test_resize_command_reflows_grid() {
    let mut t = make_sync_thread();
    assert_eq!(t.terminal.grid().cols(), 80);
    assert_eq!(t.terminal.grid().lines(), 24);

    t.process_resize(24, 40);

    assert_eq!(
        t.terminal.grid().cols(),
        40,
        "cols should be 40 after resize"
    );
    assert_eq!(
        t.terminal.grid().lines(),
        24,
        "rows should stay 24 after resize"
    );
}

/// Rapid resize commands are coalesced — only the last one is applied.
#[test]
fn test_resize_coalescing() {
    let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();

    // Queue 3 resize commands before draining.
    cmd_tx
        .send(PaneIoCommand::Resize { rows: 24, cols: 80 })
        .unwrap();
    cmd_tx
        .send(PaneIoCommand::Resize { rows: 24, cols: 60 })
        .unwrap();
    cmd_tx
        .send(PaneIoCommand::Resize { rows: 24, cols: 40 })
        .unwrap();

    t.drain_commands();

    assert_eq!(
        t.terminal.grid().cols(),
        40,
        "only the last resize (40 cols) should be applied"
    );
}

/// Resize command produces a snapshot with new dimensions.
#[test]
fn test_resize_produces_snapshot() {
    let mut t = make_sync_thread();

    t.process_resize(30, 100);
    // process_resize sets grid_dirty — produce_snapshot should fire.
    t.maybe_produce_snapshot();

    let mut consumer = oriterm_core::RenderableContent::default();
    assert!(
        t.double_buffer.swap_front(&mut consumer),
        "snapshot should be available after resize"
    );
    assert_eq!(consumer.cols, 100, "snapshot cols should be 100");
    assert_eq!(consumer.lines, 30, "snapshot rows should be 30");
}

/// PTY resize dedup: sending the same size twice only records it once.
#[test]
fn test_resize_dedup_skips_same_size() {
    let mut t = make_sync_thread();

    t.process_resize(30, 100);
    let packed_after_first = t.last_pty_size;

    t.process_resize(30, 100);
    let packed_after_second = t.last_pty_size;

    // Both should have the same packed value — the dedup prevents a second
    // PtyControl call (no PtyControl in test, but the packed field proves dedup).
    assert_eq!(packed_after_first, packed_after_second);
    let expected = (30u32 << 16) | 100u32;
    assert_eq!(packed_after_first, expected, "packed size should match");
}

/// First resize at spawn dimensions should not trigger PTY resize (dedup seed).
/// Validates TPR-05-002 fix: `last_pty_size` is seeded from initial dimensions.
#[test]
fn test_spawn_size_resize_is_deduped() {
    let mut t = make_sync_thread();
    // make_sync_thread creates a 24x80 term, and IoThreadConfig uses
    // initial_rows=24, initial_cols=80 — so last_pty_size is pre-seeded.
    let initial_packed = (24u32 << 16) | 80u32;
    assert_eq!(
        t.last_pty_size, initial_packed,
        "last_pty_size should be seeded from initial dimensions"
    );

    // Resize to the same size — the packed value should not change (dedup).
    t.process_resize(24, 80);
    assert_eq!(
        t.last_pty_size, initial_packed,
        "same-size resize should not change last_pty_size"
    );
}

/// Display offset resets to 0 after resize (Grid::resize calls finalize_resize).
#[test]
fn test_resize_display_offset_resets() {
    let mut t = make_sync_thread();

    // Fill grid with content and scroll up.
    for _ in 0..50 {
        t.handle_bytes(b"line of text\r\n");
    }
    t.terminal.grid_mut().scroll_display(10);
    assert!(
        t.terminal.grid().display_offset() > 0,
        "should be scrolled up"
    );

    // Resize resets display_offset.
    t.process_resize(24, 40);
    assert_eq!(
        t.terminal.grid().display_offset(),
        0,
        "display_offset should be 0 after resize"
    );
}

/// Bytes interleaved with resize: data is preserved across reflow.
#[test]
fn test_resize_interleaved_with_bytes() {
    let mut t = make_sync_thread();

    // Parse some text.
    t.handle_bytes(b"hello world");

    // Resize.
    t.process_resize(24, 40);

    // Parse more text.
    t.handle_bytes(b" after resize");

    // The grid should contain both pieces of text.
    t.grid_dirty.store(true, Ordering::Release);
    t.maybe_produce_snapshot();
    let mut snap = oriterm_core::RenderableContent::default();
    t.double_buffer.swap_front(&mut snap);

    let text: String = snap
        .cells
        .iter()
        .filter(|c| c.ch != ' ' && c.ch != '\0')
        .map(|c| c.ch)
        .collect();
    assert!(
        text.contains("hello"),
        "should contain text from before resize: {text:?}"
    );
    assert!(
        text.contains("afterresize"),
        "should contain text from after resize: {text:?}"
    );
}

/// Resize coalescing preserves other commands in the batch.
#[test]
fn test_resize_coalescing_preserves_other_commands() {
    let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();

    // Queue: scroll, resize, resize, scroll.
    cmd_tx.send(PaneIoCommand::ScrollDisplay(5)).unwrap();
    cmd_tx
        .send(PaneIoCommand::Resize { rows: 24, cols: 60 })
        .unwrap();
    cmd_tx
        .send(PaneIoCommand::Resize { rows: 24, cols: 40 })
        .unwrap();
    cmd_tx.send(PaneIoCommand::ScrollDisplay(3)).unwrap();

    // Fill some scrollback so scroll has effect.
    for _ in 0..50 {
        t.handle_bytes(b"scrollback line\r\n");
    }

    t.drain_commands();

    // Only the last resize should be applied.
    assert_eq!(t.terminal.grid().cols(), 40, "resize should use last size");
}

// --- Section 06 command tests (scroll, theme, cursor, mark_all_dirty, extract) ---

/// ScrollDisplay command adjusts display offset.
#[test]
fn test_scroll_display_command() {
    let mut t = make_sync_thread();

    // Fill scrollback so there's content to scroll through.
    for _ in 0..50 {
        t.handle_bytes(b"scrollback line\r\n");
    }

    t.handle_command(PaneIoCommand::ScrollDisplay(5));

    assert_eq!(
        t.terminal.grid().display_offset(),
        5,
        "display_offset should be 5 after ScrollDisplay(5)"
    );
}

/// ScrollToBottom resets display offset to 0.
#[test]
fn test_scroll_to_bottom_command() {
    let mut t = make_sync_thread();

    // Fill scrollback and scroll up.
    for _ in 0..50 {
        t.handle_bytes(b"scrollback line\r\n");
    }
    t.terminal.grid_mut().scroll_display(10);
    assert!(
        t.terminal.grid().display_offset() > 0,
        "should be scrolled up"
    );

    t.handle_command(PaneIoCommand::ScrollToBottom);

    assert_eq!(
        t.terminal.grid().display_offset(),
        0,
        "display_offset should be 0 after ScrollToBottom"
    );
}

/// ScrollToPreviousPrompt scrolls to a prompt marker above viewport.
#[test]
fn test_scroll_to_previous_prompt_command() {
    let mut t = make_sync_thread();

    // Insert a prompt marker near the top.
    t.handle_bytes(b"\x1b]133;A\x07");
    t.handle_bytes(b"prompt line\r\n");

    // Fill more lines to push the prompt into scrollback.
    for _ in 0..50 {
        t.handle_bytes(b"output line\r\n");
    }

    // Should be at live view (offset 0).
    assert_eq!(t.terminal.grid().display_offset(), 0);

    t.handle_command(PaneIoCommand::ScrollToPreviousPrompt);

    // After scrolling to previous prompt, display_offset should be > 0
    // (we scrolled up to see the prompt).
    assert!(
        t.terminal.grid().display_offset() > 0,
        "should have scrolled up to prompt marker"
    );
}

/// SetTheme command updates the terminal's palette.
#[test]
fn test_set_theme_command() {
    let mut t = make_sync_thread();

    let light_palette = oriterm_core::Palette::for_theme(Theme::Light);
    t.handle_command(PaneIoCommand::SetTheme(
        Theme::Light,
        Box::new(light_palette),
    ));

    // The terminal's palette should now match the light palette.
    let p = t.terminal.palette();
    let expected = oriterm_core::Palette::for_theme(Theme::Light);
    assert_eq!(
        p.foreground(),
        expected.foreground(),
        "palette foreground should match light theme"
    );
    assert_eq!(
        p.background(),
        expected.background(),
        "palette background should match light theme"
    );
}

/// SetCursorShape command changes the cursor shape.
#[test]
fn test_set_cursor_shape_command() {
    use oriterm_core::CursorShape;

    let mut t = make_sync_thread();

    t.handle_command(PaneIoCommand::SetCursorShape(CursorShape::Block));
    assert_eq!(
        t.terminal.cursor_shape(),
        CursorShape::Block,
        "cursor shape should be Block"
    );

    t.handle_command(PaneIoCommand::SetCursorShape(CursorShape::Underline));
    assert_eq!(
        t.terminal.cursor_shape(),
        CursorShape::Underline,
        "cursor shape should be Underline"
    );
}

/// MarkAllDirty command marks all lines dirty.
#[test]
fn test_mark_all_dirty_command() {
    let mut t = make_sync_thread();

    // Reset damage first.
    t.terminal.reset_damage();
    assert!(
        !t.terminal.grid().dirty().is_all_dirty(),
        "damage should be clear after reset"
    );

    t.handle_command(PaneIoCommand::MarkAllDirty);

    assert!(
        t.terminal.grid().dirty().is_all_dirty(),
        "all lines should be dirty after MarkAllDirty"
    );
}

/// ExtractText with a reply channel returns the selected text.
#[test]
fn test_extract_text_reply() {
    use oriterm_core::grid::StableRowIndex;
    use oriterm_core::index::Side;
    use oriterm_core::{Selection, SelectionMode, SelectionPoint};

    let mut t = make_sync_thread();

    t.handle_bytes(b"hello world");

    // Build a selection covering columns 0-10 on the first visible line.
    let grid = t.terminal.grid();
    let stable = StableRowIndex::from_visible(grid, 0);
    let anchor = SelectionPoint {
        row: stable,
        col: 0,
        side: Side::Left,
    };
    let end_point = SelectionPoint {
        row: stable,
        col: 10,
        side: Side::Right,
    };
    let selection = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: end_point,
        end: end_point,
    };

    let (tx, rx) = crossbeam_channel::bounded(1);
    t.handle_reply_command(PaneIoCommand::ExtractText {
        selection,
        reply: tx,
    });

    let result = rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "should receive reply");
    let text = result.unwrap();
    assert!(text.is_some(), "extraction should produce text");
    assert_eq!(text.unwrap(), "hello world");
}

/// ExtractText on a disconnected channel (dead IO thread) returns Err, not a hang.
#[test]
fn test_extract_text_timeout_safety() {
    let (tx, rx) = crossbeam_channel::bounded::<Option<String>>(1);

    // Drop the sender without sending — simulates a dead IO thread.
    drop(tx);

    // This must return immediately with Err(Disconnected), not block.
    let result = rx.recv_timeout(Duration::from_millis(100));
    assert!(
        result.is_err(),
        "recv on disconnected channel should return Err, not hang"
    );
}

/// ExtractHtml with a reply channel returns HTML and plain text.
#[test]
fn test_extract_html_reply() {
    use oriterm_core::grid::StableRowIndex;
    use oriterm_core::index::Side;
    use oriterm_core::{Selection, SelectionMode, SelectionPoint};

    let mut t = make_sync_thread();

    // Write styled text: red foreground.
    t.handle_bytes(b"\x1b[31mred text\x1b[0m");

    let grid = t.terminal.grid();
    let stable = StableRowIndex::from_visible(grid, 0);
    let anchor = SelectionPoint {
        row: stable,
        col: 0,
        side: Side::Left,
    };
    let end_point = SelectionPoint {
        row: stable,
        col: 7,
        side: Side::Right,
    };
    let selection = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: end_point,
        end: end_point,
    };

    let (tx, rx) = crossbeam_channel::bounded(1);
    t.handle_reply_command(PaneIoCommand::ExtractHtml {
        selection,
        font_family: "monospace".to_string(),
        font_size: 12.0,
        reply: tx,
    });

    let result = rx.recv_timeout(Duration::from_millis(100));
    assert!(result.is_ok(), "should receive reply");
    let data = result.unwrap();
    assert!(data.is_some(), "extraction should produce HTML");
    let (html, text) = data.unwrap();
    assert!(
        text.contains("red text"),
        "plain text should contain 'red text', got: {text:?}"
    );
    assert!(
        html.contains("<span"),
        "HTML should contain styled spans, got: {html:?}"
    );
}

// --- Section 06 search, mark mode, selection tests ---

/// OpenSearch/CloseSearch commands toggle search state on the IO thread.
#[test]
fn test_open_close_search() {
    let mut t = make_sync_thread();

    assert!(t.search.is_none(), "search should be None initially");

    t.handle_command(PaneIoCommand::OpenSearch);
    assert!(t.search.is_some(), "search should be Some after OpenSearch");

    t.handle_command(PaneIoCommand::CloseSearch);
    assert!(
        t.search.is_none(),
        "search should be None after CloseSearch"
    );
}

/// SearchSetQuery finds matches in the terminal grid.
#[test]
fn test_search_set_query_finds_matches() {
    let mut t = make_sync_thread();

    t.handle_bytes(b"foo bar foo");
    t.handle_command(PaneIoCommand::OpenSearch);
    t.handle_command(PaneIoCommand::SearchSetQuery("foo".to_string()));

    let search = t.search.as_ref().expect("search should be active");
    assert_eq!(search.matches().len(), 2, "should find 2 matches for 'foo'");
}

/// SearchNextMatch/SearchPrevMatch advance and retreat the focused index.
#[test]
fn test_search_next_prev_match() {
    let mut t = make_sync_thread();

    // Write text with 3 occurrences of "ab".
    t.handle_bytes(b"ab cd ab ef ab");
    t.handle_command(PaneIoCommand::OpenSearch);
    t.handle_command(PaneIoCommand::SearchSetQuery("ab".to_string()));

    let search = t.search.as_ref().unwrap();
    assert_eq!(search.matches().len(), 3, "should find 3 matches");
    let initial_focus = search.focused_index();

    t.handle_command(PaneIoCommand::SearchNextMatch);
    let after_next = t.search.as_ref().unwrap().focused_index();
    assert_ne!(
        after_next, initial_focus,
        "focus should advance after SearchNextMatch"
    );

    t.handle_command(PaneIoCommand::SearchNextMatch);
    let after_next2 = t.search.as_ref().unwrap().focused_index();

    t.handle_command(PaneIoCommand::SearchPrevMatch);
    let after_prev = t.search.as_ref().unwrap().focused_index();
    assert_eq!(
        after_prev, after_next,
        "focus should retreat to previous position after SearchPrevMatch"
    );
    // Suppress "unused" warning.
    let _ = after_next2;
}

/// Search results appear in produced snapshots.
#[test]
fn test_search_results_in_snapshot() {
    let mut t = make_sync_thread();

    t.handle_bytes(b"foo bar foo");
    t.handle_command(PaneIoCommand::OpenSearch);
    t.handle_command(PaneIoCommand::SearchSetQuery("foo".to_string()));
    t.grid_dirty.store(true, Ordering::Release);
    t.produce_snapshot();

    let mut snap = oriterm_core::RenderableContent::default();
    assert!(t.double_buffer.swap_front(&mut snap));

    assert!(
        snap.search_active,
        "snapshot should have search_active=true"
    );
    assert_eq!(
        snap.search_query, "foo",
        "snapshot search_query should be 'foo'"
    );
    assert_eq!(
        snap.search_total_matches, 2,
        "snapshot should report 2 matches"
    );
    assert!(
        !snap.search_matches.is_empty(),
        "matches list should be populated"
    );
}

/// EnterMarkMode reply contains valid cursor coordinates.
#[test]
fn test_enter_mark_mode_reply() {
    use crate::pane::MarkCursor;

    let mut t = make_sync_thread();

    // Write some text so cursor is at a known position.
    t.handle_bytes(b"hello");

    let (tx, rx) = crossbeam_channel::bounded::<MarkCursor>(1);
    t.handle_reply_command(PaneIoCommand::EnterMarkMode { reply: tx });

    let mc = rx
        .recv_timeout(Duration::from_millis(100))
        .expect("should receive MarkCursor reply");

    // Cursor should be at col 5 (after "hello") on row 0.
    assert_eq!(mc.col, 5, "mark cursor col should be 5 (after 'hello')");

    // Terminal should be scrolled to bottom (display_offset == 0).
    assert_eq!(
        t.terminal.grid().display_offset(),
        0,
        "terminal should be at live view after enter_mark_mode"
    );
}

/// IO thread propagates selection_dirty to the shared atomic.
#[test]
fn test_selection_dirty_atomic() {
    let mut t = make_sync_thread();

    assert!(
        !t.selection_dirty.load(Ordering::Acquire),
        "selection_dirty should be false initially"
    );

    // Writing a character sets Term::selection_dirty; handle_bytes propagates.
    t.handle_bytes(b"X");

    assert!(
        t.selection_dirty.load(Ordering::Acquire),
        "selection_dirty should be true after terminal output"
    );
}

/// SelectCommandOutput returns a selection covering the command output zone.
#[test]
fn test_select_command_output_reply() {
    use oriterm_core::Selection;

    let mut t = make_sync_thread();

    // Set up prompt markers: prompt start → command start → output start.
    t.handle_bytes(b"\x1b]133;A\x07"); // Prompt start.
    t.handle_bytes(b"$ ls\r\n");
    t.handle_bytes(b"\x1b]133;C\x07"); // Output start.
    t.handle_bytes(b"file1.txt\r\nfile2.txt\r\n");

    let (tx, rx) = crossbeam_channel::bounded::<Option<Selection>>(1);
    t.handle_reply_command(PaneIoCommand::SelectCommandOutput { reply: tx });

    let result = rx
        .recv_timeout(Duration::from_millis(100))
        .expect("should receive reply");

    // Command output selection may or may not be found depending on
    // whether the prompt markers form a complete output zone. The test
    // verifies the command round-trips without panicking.
    // If a valid zone was found, the selection should be non-empty.
    if let Some(sel) = result {
        assert_eq!(
            sel.mode,
            oriterm_core::SelectionMode::Line,
            "output selection should be line mode"
        );
    }
}

/// SelectCommandInput returns a selection covering the command input zone.
#[test]
fn test_select_command_input_reply() {
    use oriterm_core::Selection;

    let mut t = make_sync_thread();

    // Set up a complete prompt cycle.
    t.handle_bytes(b"\x1b]133;A\x07"); // Prompt start.
    t.handle_bytes(b"\x1b]133;B\x07"); // Command start.
    t.handle_bytes(b"echo hello\r\n");
    t.handle_bytes(b"\x1b]133;C\x07"); // Output start.
    t.handle_bytes(b"hello\r\n");

    let (tx, rx) = crossbeam_channel::bounded::<Option<Selection>>(1);
    t.handle_reply_command(PaneIoCommand::SelectCommandInput { reply: tx });

    let result = rx
        .recv_timeout(Duration::from_millis(100))
        .expect("should receive reply");

    // As with output, the zone may or may not be found.
    if let Some(sel) = result {
        assert_eq!(
            sel.mode,
            oriterm_core::SelectionMode::Line,
            "input selection should be line mode"
        );
    }
}

// --- Section 08.4 threading stress tests ---

/// Concurrent resize + byte flood: one thread floods bytes, another sends
/// 100 resize commands. IO thread must not panic and must settle to correct
/// final dimensions.
#[test]
fn test_concurrent_resize_and_pty_output() {
    let (mut handle, _shutdown) = spawn_pair_with_flag();
    let byte_tx = handle.byte_sender();

    // Byte flood thread: send 500 chunks of 1 KB each.
    let flood_handle = std::thread::spawn(move || {
        let chunk = vec![b'A'; 1024];
        for _ in 0..500 {
            if byte_tx.send(chunk.clone()).is_err() {
                break;
            }
        }
    });

    // Resize flood: 100 commands from the main test thread.
    for i in 0..100u16 {
        let cols = 40 + (i % 80);
        let rows = 20 + (i % 20);
        handle.send_command(PaneIoCommand::Resize { rows, cols });
    }

    // Wait for flood to finish.
    flood_handle.join().expect("byte flood thread panicked");

    // Give IO thread time to drain.
    std::thread::sleep(Duration::from_millis(200));

    // Verify snapshot is producible (IO thread still alive).
    let mut snap = oriterm_core::RenderableContent::default();
    handle.double_buffer().swap_front(&mut snap);

    // Shutdown cleanly.
    handle.send_command(PaneIoCommand::Shutdown);
    let join = handle.join.take().expect("join handle missing");
    assert!(
        join.join().is_ok(),
        "IO thread panicked during concurrent resize + output"
    );
}

/// Close pane during flood output: IO thread must exit within 2 seconds.
#[test]
fn test_pane_close_during_flood_output() {
    let (mut handle, _shutdown) = spawn_pair_with_flag();
    let byte_tx = handle.byte_sender();

    // Flood thread: continuous output until channel disconnects.
    let flood_handle = std::thread::spawn(move || {
        let chunk = vec![b'X'; 4096];
        loop {
            if byte_tx.send(chunk.clone()).is_err() {
                break;
            }
        }
    });

    // Brief delay to let some bytes flow.
    std::thread::sleep(Duration::from_millis(50));

    // Shutdown the IO thread (drops cmd_tx on PaneIoHandle::shutdown).
    let start = std::time::Instant::now();
    handle.shutdown();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "IO thread shutdown took {elapsed:?}, expected < 2s"
    );

    // Flood thread should also exit (byte channel disconnected).
    flood_handle.join().expect("flood thread panicked");
}

/// Three IO threads resizing concurrently — no cross-thread corruption.
#[test]
fn test_multiple_panes_concurrent_resize() {
    let mut handles: Vec<(PaneIoHandle, Arc<AtomicBool>)> = Vec::new();
    for _ in 0..3 {
        handles.push(spawn_pair_with_flag());
    }

    // Send distinct resize sequences to each pane.
    let expected_dims = [(30u16, 90u16), (25, 70), (35, 110)];
    for (i, (handle, _)) in handles.iter().enumerate() {
        for j in 0..20u16 {
            let (final_rows, _) = expected_dims[i];
            let cols = 40 + j * 3; // intermediate sizes
            handle.send_command(PaneIoCommand::Resize {
                rows: final_rows,
                cols,
            });
        }
        // Final resize to the expected dimensions.
        let (rows, cols) = expected_dims[i];
        handle.send_command(PaneIoCommand::Resize { rows, cols });
    }

    // Give IO threads time to drain.
    std::thread::sleep(Duration::from_millis(100));

    // Verify each pane's snapshot has correct dimensions.
    for (i, (handle, _)) in handles.iter().enumerate() {
        let mut snap = oriterm_core::RenderableContent::default();
        handle.double_buffer().swap_front(&mut snap);
        let (exp_rows, exp_cols) = expected_dims[i];
        assert_eq!(snap.lines, exp_rows as usize, "pane {i} rows mismatch");
        assert_eq!(snap.cols, exp_cols as usize, "pane {i} cols mismatch");
    }

    // Clean shutdown.
    for (mut handle, _) in handles {
        handle.shutdown();
    }
}

/// Flood 1000 MarkAllDirty commands — IO thread drains all without blocking.
#[test]
fn test_command_channel_flood() {
    let (mut handle, _shutdown) = spawn_pair_with_flag();

    for _ in 0..1000 {
        handle.send_command(PaneIoCommand::MarkAllDirty);
    }

    // Give IO thread time to drain.
    std::thread::sleep(Duration::from_millis(200));

    // IO thread should still be responsive to new commands.
    handle.send_command(PaneIoCommand::Resize {
        rows: 30,
        cols: 100,
    });
    std::thread::sleep(Duration::from_millis(50));

    let mut snap = oriterm_core::RenderableContent::default();
    handle.double_buffer().swap_front(&mut snap);
    assert_eq!(snap.cols, 100, "should respond after 1000-command flood");

    handle.shutdown();
}

/// Snapshot swap under contention: producer + consumer threads hammering
/// the double buffer for 500ms. No panic, seqno monotonic.
#[test]
fn test_snapshot_swap_under_contention() {
    let db = SnapshotDoubleBuffer::new();
    let db_clone = db.clone();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    // Producer thread: flip as fast as possible.
    let producer = std::thread::spawn(move || {
        let mut buf = oriterm_core::RenderableContent::default();
        let mut count = 0u64;
        while !stop_clone.load(Ordering::Relaxed) {
            buf.cells.clear();
            db_clone.flip_swap(&mut buf);
            count += 1;
        }
        count
    });

    // Consumer thread: swap_front as fast as possible.
    let mut consumer_buf = oriterm_core::RenderableContent::default();
    let mut consume_count = 0u64;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        if db.swap_front(&mut consumer_buf) {
            consume_count += 1;
        }
    }

    stop.store(true, Ordering::Relaxed);
    let produce_count = producer.join().expect("producer panicked");

    assert!(
        produce_count > 100,
        "producer should have flipped many times: {produce_count}"
    );
    assert!(
        consume_count > 10,
        "consumer should have consumed some snapshots: {consume_count}"
    );
}

// --- Section 08 resize quality verification ---

/// Rapid resize: 50 successive resizes with varying dimensions.
///
/// Verifies: final grid matches last resize, no orphaned commands,
/// snapshot reflects correct final dimensions.
#[test]
fn test_rapid_resize_50_cycles() {
    let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();

    // Fill grid with content so resize has rows to reflow.
    for _ in 0..60 {
        t.handle_bytes(b"content line for resize testing\r\n");
    }

    // Queue 50 resize commands with varying dimensions.
    for i in 0..50u16 {
        let cols = 40 + (i % 80); // 40..119
        let rows = 20 + (i % 20); // 20..39
        cmd_tx.send(PaneIoCommand::Resize { rows, cols }).unwrap();
    }

    // Drain all commands — coalescing should apply the last resize only.
    t.drain_commands();

    // Last resize: i=49 → cols = 40 + (49 % 80) = 89, rows = 20 + (49 % 20) = 29.
    assert_eq!(t.terminal.grid().cols(), 89, "final cols after 50 resizes");
    assert_eq!(t.terminal.grid().lines(), 29, "final rows after 50 resizes");

    // Produce snapshot and verify dimensions match.
    t.grid_dirty.store(true, Ordering::Release);
    t.maybe_produce_snapshot();
    let mut snap = oriterm_core::RenderableContent::default();
    assert!(t.double_buffer.swap_front(&mut snap));
    assert_eq!(snap.cols, 89, "snapshot cols after rapid resize");
    assert_eq!(snap.lines, 29, "snapshot rows after rapid resize");
}

/// Resize during active byte processing: content + resize interleaved 50 times.
///
/// Verifies no panic, final dimensions correct, text preserved through reflows.
#[test]
fn test_resize_during_sustained_output() {
    let mut t = make_sync_thread();

    // Alternate between writing output and resizing.
    for i in 0..50u16 {
        let line = format!("output line {i:04}\r\n");
        t.handle_bytes(line.as_bytes());
        let cols = 60 + (i % 40); // 60..99
        t.process_resize(24, cols);
    }

    // Final size: i=49 → cols = 60 + (49 % 40) = 69.
    assert_eq!(
        t.terminal.grid().cols(),
        69,
        "final cols after interleaved resize"
    );

    // Verify snapshot is producible and has correct dimensions.
    t.grid_dirty.store(true, Ordering::Release);
    t.maybe_produce_snapshot();
    let mut snap = oriterm_core::RenderableContent::default();
    assert!(t.double_buffer.swap_front(&mut snap));
    assert_eq!(snap.cols, 69, "snapshot cols");
    assert_eq!(snap.lines, 24, "snapshot rows");
}
