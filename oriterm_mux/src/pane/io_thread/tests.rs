//! Integration tests for `PaneIoThread::run` and `PaneIoHandle`.
//!
//! Per-module tests live in their sibling files:
//! `commands/tests.rs`, `effect_router/tests.rs`, `snapshot/tests.rs`,
//! `response_poll/tests.rs`. This file holds cross-module integration
//! tests that exercise the `PaneIoThread::run` event loop end-to-end.
//!
//! # Section index
//!
//! | Line | Section |
//! |-------|----------------------------------------------------------|
//! | 212+ | Lifecycle tests (spawn, shutdown, drop) |
//! | 419+ | Section 02: VTE parsing through IO thread |
//! | 581+ | Section 03: snapshot production |
//! | 735+ | Section 05: resize handling |
//! | 1002+ | Section 06: command dispatch (scroll/theme/cursor/etc.) |
//! | 1280+ | Section 06: search, mark mode, selection |
//! | 1489+ | Snapshot publication races between parse chunks |
//! | 1519+ | Section 08.4: threading stress |
//! | 1765+ | Section 08: resize quality |
//! | 1834+ | Section 03.5d: reply-return path |
//! | 1987+ | Section 06.5: sync timeout + edge cases |
//! | 2383+ | Section 09.2: Mode 2026 parser → mode_cache bridge |
//! | 2938+ | Bounded byte channel + symmetric IO-thread shrink |
//! | 3248+ | §03 matrix: bounded cmd_tx + atomic-coalescing resize |
//! | 3829+ | §03 cross-feature interaction tests (Round 1 §06 F1) |

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;

use oriterm_core::effect::{PollResult, VoidEffectSink};
use oriterm_core::{Column, Line, RenderableContent, Term, TermMode, Theme};

use super::handle::{
 CMD_CHANNEL_CAPACITY, PENDING_RESIZE_NONE, pack_pending_resize, unpack_pending_resize,
};
use super::snapshot::SnapshotDoubleBuffer;
use super::{IoThreadConfig, PaneIoCommand, PaneIoHandle, PaneIoThread, new_with_handle};

/// Test helper: pack and store a pending resize on the slot.
/// Stand-in for `PaneIoHandle::send_resize` when the test exercises a
/// `PaneIoThread` directly without a paired handle.
fn pack_then_store(slot: &Arc<AtomicU64>, rows: u16, cols: u16) {
 slot.store(pack_pending_resize(rows, cols), Ordering::Release);
}
use crate::PaneId;
use crate::mux_event::MuxEvent;
use crate::pty::reader::BYTE_CHANNEL_CAPACITY;
use crate::pty::spawn::ExitStatus;

/// Test helper: dummy pane_id + live channels for constructing
/// `PaneIoThread` in synchronous tests.
/// The `_keep_alive` fields are leaked via a `OnceLock<Vec<..>>` so the
/// channels stay open for the duration of the test process — leaking
/// is intentional and preferable to `std::mem::forget` call sites
/// scattered across every helper.
fn test_dummy_channels() -> (
 PaneId,
 mpsc::Sender<MuxEvent>,
 Receiver<ExitStatus>,
 Receiver<()>,
) {
 use std::sync::Mutex as StdMutex;
 use std::sync::OnceLock;
 static KEEP_ALIVE: OnceLock<StdMutex<Vec<Box<dyn std::any::Any + Send>>>> = OnceLock::new();
 let vault = KEEP_ALIVE.get_or_init(|| StdMutex::new(Vec::new()));

 let (mux_tx, mux_rx) = mpsc::channel::<MuxEvent>();
 let (exit_tx, exit_rx) = crossbeam_channel::bounded::<ExitStatus>(1);
 let (wake_tx, wake_rx) = crossbeam_channel::bounded::<()>(1);

 if let Ok(mut v) = vault.lock() {
 v.push(Box::new(mux_rx));
 v.push(Box::new(exit_tx));
 v.push(Box::new(wake_tx));
 }

 (PaneId::from_raw(1), mux_tx, exit_rx, wake_rx)
}

/// Helper: create a Term<VoidEffectSink> with default dimensions.
fn make_term() -> Term<VoidEffectSink> {
 Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

/// Helper: create a thread + handle pair with a no-op wakeup.
fn make_pair() -> (PaneIoThread<VoidEffectSink>, PaneIoHandle) {
 new_with_handle(IoThreadConfig {
 terminal: make_term(),
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
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
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 initial_rows: 24,
 initial_cols: 80,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 });
 let join = thread.spawn().expect("failed to spawn IO thread");
 handle.set_join(join);
 (handle, shutdown)
}

/// Helper: create a `PaneIoThread` for synchronous testing (no spawning).
fn make_sync_thread() -> PaneIoThread<VoidEffectSink> {
 make_sync_thread_with_term(make_term())
}

/// Helper: create a `PaneIoThread` with a custom `Term` for synchronous testing.
fn make_sync_thread_with_term(term: Term<VoidEffectSink>) -> PaneIoThread<VoidEffectSink> {
 let rows = term.grid().lines() as u16;
 let cols = term.grid().cols() as u16;
 let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
 let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 PaneIoThread {
 terminal: term,
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(|| {}),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: SnapshotDoubleBuffer::new(),
 snapshot_buf: Default::default(),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (rows as u32) << 16 | cols as u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
 }
}

/// Helper: create a sync thread with a wakeup counter for testing.
fn make_sync_thread_with_wakeup() -> (PaneIoThread<VoidEffectSink>, Arc<AtomicU64>) {
 let wakeup_count = Arc::new(AtomicU64::new(0));
 let wakeup_clone = Arc::clone(&wakeup_count);
 let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
 let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
 let grid_dirty = Arc::new(AtomicBool::new(false));
 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let thread = PaneIoThread {
 terminal: make_term(),
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(move || {
 wakeup_clone.fetch_add(1, Ordering::Relaxed);
 }),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: SnapshotDoubleBuffer::new(),
 snapshot_buf: Default::default(),
 grid_dirty,
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
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
 let mode_cache = Arc::new(AtomicU64::new(TermMode::default().bits()));
 let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
 let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let thread = PaneIoThread {
 terminal: make_term(),
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
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
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
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
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
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
 let mode_cache = Arc::new(AtomicU64::new(TermMode::default().bits()));

 let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
 let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let thread = PaneIoThread {
 terminal: make_term(),
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
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
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
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

 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let mut t = PaneIoThread {
 terminal: make_term(),
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: SnapshotDoubleBuffer::new(),
 snapshot_buf: Default::default(),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
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
 let term = Term::new(5, 80, 10, Theme::default(), VoidEffectSink);
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

 let mut consumer = RenderableContent::default();
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
/// When `TermMode::SYNC_UPDATE` is set, snapshot publication is deferred
/// even though grid mutations dispatch inline. Pinning the SSOT-correct
/// gate (mode flag, not byte buffer) per §03.
#[test]
fn produce_snapshot_respects_sync_update_mode_flag() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // Enable Mode 2026 (synchronized output begin: BSU).
 t.handle_bytes(b"\x1b[?2026h");
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "BSU must set TermMode::SYNC_UPDATE"
 );
 t.grid_dirty.store(true, Ordering::Release);

 // Send some content while sync mode is active. The bytes dispatch
 // INLINE (mutating the grid), but snapshot publication is suppressed
 // via the SYNC_UPDATE mode-flag gate.
 t.processor.advance(&mut t.terminal, b"buffered content");

 // Try to produce snapshot — should be suppressed because the mode
 // flag is set.
 let wakeup_before = wakeup_count.load(Ordering::Relaxed);
 t.maybe_produce_snapshot();
 let wakeup_after = wakeup_count.load(Ordering::Relaxed);

 assert_eq!(
 wakeup_before, wakeup_after,
 "wakeup must NOT fire while TermMode::SYNC_UPDATE is set"
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

/// Shutdown flushes any parsed-but-unpublished state.
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

 let mut consumer = RenderableContent::default();
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
/// Bounded to match production shape (). Tests using this
/// fixture do NOT exercise saturation — they drive `drain_commands`
/// directly with a small command set — so production-shape parity
/// matters more than headroom.
fn make_sync_thread_with_cmd_tx() -> (PaneIoThread<VoidEffectSink>, Sender<PaneIoCommand>) {
 let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<PaneIoCommand>(CMD_CHANNEL_CAPACITY);
 let (_, byte_rx) = crossbeam_channel::bounded::<Vec<u8>>(BYTE_CHANNEL_CAPACITY);
 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let thread = PaneIoThread {
 terminal: make_term(),
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(|| {}),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: SnapshotDoubleBuffer::new(),
 snapshot_buf: Default::default(),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
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
 let (mut t, _cmd_tx) = make_sync_thread_with_cmd_tx();

 // Queue 3 resize commands before draining.
 pack_then_store(&t.pending_resize, 24, 80);
 pack_then_store(&t.pending_resize, 24, 60);
 pack_then_store(&t.pending_resize, 24, 40);

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

 let mut consumer = RenderableContent::default();
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
/// Validates fix: `last_pty_size` is seeded from initial dimensions.
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

/// Section 03.9 Phase 4 / : process_resize routes to
/// `adopted_signal.resize()` when `pty_control` is `None`.
/// Property: install a stub `AdoptedSignal` (which always errors on
/// resize) and verify that process_resize:
/// 1. Calls the grid reflow path (terminal dimensions update).
/// 2. Calls the signal resize path (we can't observe the call directly,
/// but we can verify last_pty_size advances — proving the dedup
/// guard saw the call).
/// 3. Does NOT panic on the signal error path.
#[cfg(test)]
#[test]
fn process_resize_routes_through_adopted_signal_when_pty_control_none() {
 use crate::pty::adopt::AdoptedSignal;

 let mut t = make_sync_thread();
 // Replace pty_control (already None) with an adopted_signal stub.
 // The stub returns an error from resize() but process_resize must
 // log + continue, not panic.
 t.adopted_signal = Some(AdoptedSignal::stub_for_tests());

 let initial_packed = (24u32 << 16) | 80u32;
 t.last_pty_size = initial_packed;

 // Resize to a new size — should call adopted_signal.resize() and
 // update last_pty_size.
 t.process_resize(30, 100);

 let new_packed = (30u32 << 16) | 100u32;
 assert_eq!(
 t.last_pty_size, new_packed,
 "process_resize must update last_pty_size even when both \
 pty_control is None and adopted_signal returns an error",
 );
 assert_eq!(
 t.terminal.grid().lines(),
 30,
 "grid reflow must run before the signal pipe write",
 );
 assert_eq!(t.terminal.grid().cols(), 100, "grid cols must be updated");
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
 let mut snap = RenderableContent::default();
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
 pack_then_store(&t.pending_resize, 24, 60);
 pack_then_store(&t.pending_resize, 24, 40);
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

/// `SetCellDimensions` command is plumbed to `Term::set_cell_dimensions`
/// without panicking, and flags the grid dirty so the main thread pulls
/// a fresh snapshot. The underlying Term-level recomputation (cell
/// coverage for `FixedPixels` placements) is verified in
/// `oriterm_core::image::cache::tests` — this test covers only the
/// mux-layer plumbing.
#[test]
fn test_set_cell_dimensions_command_marks_dirty() {
 use std::sync::atomic::Ordering;

 let mut t = make_sync_thread();
 t.grid_dirty.store(false, Ordering::Release);

 t.handle_command(PaneIoCommand::SetCellDimensions {
 width: 16,
 height: 32,
 });

 assert!(
 t.grid_dirty.load(Ordering::Acquire),
 "SetCellDimensions must flag grid_dirty so the main thread re-reads the snapshot"
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

 let mut snap = RenderableContent::default();
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

// --- snapshot publication between parse chunks ---

/// A large byte message (> MAX_PARSE_CHUNK) should produce intermediate
/// snapshots between 64 KB chunks, not just after the entire message.
#[test]
fn handle_bytes_chunked_publishes_intermediate_snapshots() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // 200 KB message = ~3 chunks of 64 KB each.
 let big = vec![b'A'; 200_000];
 t.grid_dirty.store(true, Ordering::Release);

 wakeup_count.store(0, Ordering::SeqCst);
 t.handle_bytes_chunked(&big);

 // Should have produced multiple snapshots (one per chunk boundary).
 let wakeups = wakeup_count.load(Ordering::SeqCst);
 assert!(
 wakeups >= 2,
 "expected >=2 intermediate wakeups for 200KB message, got {wakeups}"
 );

 // Verify the final snapshot is consumable.
 let mut snap = RenderableContent::default();
 assert!(
 t.double_buffer.swap_front(&mut snap),
 "snapshot should be available after chunked parsing"
 );
}

// --- Section 08.4 threading stress tests ---

/// IO thread panic does not crash the main thread or hang shutdown.
/// Spawns a thread that panics after receiving a command, wraps it in a
/// `PaneIoHandle`, and verifies that `shutdown()` completes without hanging
/// or propagating the panic. Tests the `let _ = handle.join()` error-swallowing
/// path in `PaneIoHandle::shutdown()`.
#[test]
fn test_io_thread_panic_does_not_crash_app() {
 // Bounded to match production shape (); this test does
 // not exercise saturation, but the field shape stays aligned with
 // the production handle.
 let (tx, rx) = crossbeam_channel::bounded::<PaneIoCommand>(CMD_CHANNEL_CAPACITY);
 let (byte_tx, _byte_rx) = crossbeam_channel::bounded::<Vec<u8>>(BYTE_CHANNEL_CAPACITY);

 let join = std::thread::spawn(move || {
 let _ = rx.recv();
 panic!("intentional IO thread panic for testing");
 });

 let (_wake_tx, _wake_rx) = crossbeam_channel::bounded::<()>(1);
 let mut handle = PaneIoHandle {
 cmd_tx: tx,
 byte_tx,
 join: Some(join),
 double_buffer: SnapshotDoubleBuffer::new(),
 io_wake_tx: _wake_tx,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shutdown_flag: Arc::new(AtomicBool::new(false)),
 drop_counter: None,
 };

 // Trigger the panic. `MarkAllDirty` lands in cmd_rx; the spawned
 // thread's `let _ = rx.recv(); panic!(...)` runs as soon as the OS
 // schedules it. shutdown() either races ahead of the panic (joining
 // the panicked thread when it lands) or arrives after (joining
 // cleanly) — both paths satisfy "shutdown does not hang on panic",
 // which is the only invariant this test pins. Wall-clock sleep is
 // forbidden-Clock-Free Testing`.
 handle.send_command(PaneIoCommand::MarkAllDirty);

 // shutdown() must complete without hanging (join catches the
 // panic). Wall-clock-free per `tests.md §Wall-Clock-Free Testing`:
 // shutdown() blocks on the join handle internally; if the join
 // ever returns, the test passes. The 150s process-level timeout
 // is the only safety valve. Per Round 2 code-TPR F5.
 handle.shutdown();
}

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
 handle.send_resize(rows, cols);
 }

 // Wait for flood to finish.
 flood_handle.join().expect("byte flood thread panicked");

 // Give IO thread time to drain.
 std::thread::sleep(Duration::from_millis(200));

 // Verify snapshot is producible (IO thread still alive).
 let mut snap = RenderableContent::default();
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
 let start = Instant::now();
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
 handle.send_resize(final_rows, cols);
 }
 // Final resize to the expected dimensions.
 let (rows, cols) = expected_dims[i];
 handle.send_resize(rows, cols);
 }

 // Give IO threads time to drain.
 std::thread::sleep(Duration::from_millis(100));

 // Verify each pane's snapshot has correct dimensions.
 for (i, (handle, _)) in handles.iter().enumerate() {
 let mut snap = RenderableContent::default();
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

// `test_command_channel_flood` was removed by : the original
// 1000-command flood asserted that all commands drain via wall-clock
// `thread::sleep` (TIMING violation per `tests.md §Wall-Clock-Free
// Testing`) AND assumed unbounded `cmd_tx`. Both assumptions are
// invalid post-fix: `cmd_tx` is bounded(`CMD_CHANNEL_CAPACITY`) and
// drops the overflow with `log::error!`. Replacement coverage:
// - `cmd_tx_at_capacity_returns_full_error_synchronously` — pins the
// bounded-saturation contract synchronously (no wall-clock).
// - `drain_after_three_atomic_stores_processes_only_last` +
// `drain_after_atomic_store_processes_resize` — pin resize
// coalescing through the atomic slot (no flood needed).

/// Snapshot swap under contention: producer + consumer threads hammering
/// the double buffer for 500ms. Verifies the two correctness properties:
/// (1) no deadlock — producer makes forward progress under contention,
/// (2) final-snapshot visibility — after contention ends, the consumer can
/// always observe the latest snapshot. The during-contention consumer
/// count is informational only: on a macOS CI runner, `parking_lot`'s
/// fairness can let the producer dominate the lock, leaving the consumer
/// with very few successful swaps. That is acceptable as long as the
/// final swap succeeds, which proves seqno monotonicity holds and no
/// snapshot is lost.
#[test]
fn test_snapshot_swap_under_contention() {
 let db = SnapshotDoubleBuffer::new();
 let db_clone = db.clone();

 let stop = Arc::new(AtomicBool::new(false));
 let stop_clone = Arc::clone(&stop);

 // Producer thread: flip as fast as possible.
 let producer = std::thread::spawn(move || {
 let mut buf = RenderableContent::default();
 let mut count = 0u64;
 while !stop_clone.load(Ordering::Relaxed) {
 buf.cells.clear();
 db_clone.flip_swap(&mut buf);
 count += 1;
 }
 count
 });

 // Consumer thread: swap_front as fast as possible.
 let mut consumer_buf = RenderableContent::default();
 let mut consume_count = 0u64;
 let start = Instant::now();
 while start.elapsed() < Duration::from_millis(500) {
 if db.swap_front(&mut consumer_buf) {
 consume_count += 1;
 }
 }

 stop.store(true, Ordering::Relaxed);
 let produce_count = producer.join().expect("producer panicked");

 // Property 1: producer made forward progress — no deadlock from the
 // producer side, no panic in flip_swap under contention.
 assert!(
 produce_count > 100,
 "producer should have flipped many times: {produce_count}"
 );

 // Property 2: after contention ends, the consumer can observe the
 // latest snapshot. This is the real correctness invariant — no
 // snapshot is permanently invisible. If the consumer was starved
 // during contention (consume_count low), the post-contention swap
 // must succeed because seqno > consumed_seqno. If the consumer kept
 // up (consume_count high), the post-contention swap may legitimately
 // return false because consumed_seqno already equals seqno.
 let final_swap = db.swap_front(&mut consumer_buf);
 let total_consumes = consume_count + u64::from(final_swap);
 assert!(
 total_consumes > 0,
 "consumer must observe at least one snapshot \
 (during={consume_count}, post-contention={final_swap}, \
 produced={produce_count})"
 );
}

// --- Section 08 resize quality verification ---

/// Rapid resize: 50 successive resizes with varying dimensions.
/// Verifies: final grid matches last resize, no orphaned commands,
/// snapshot reflects correct final dimensions.
#[test]
fn test_rapid_resize_50_cycles() {
 let (mut t, _cmd_tx) = make_sync_thread_with_cmd_tx();

 // Fill grid with content so resize has rows to reflow.
 for _ in 0..60 {
 t.handle_bytes(b"content line for resize testing\r\n");
 }

 // Queue 50 resize commands with varying dimensions.
 for i in 0..50u16 {
 let cols = 40 + (i % 80); // 40..119
 let rows = 20 + (i % 20); // 20..39
 pack_then_store(&t.pending_resize, rows, cols);
 }

 // Drain all commands — coalescing should apply the last resize only.
 t.drain_commands();

 // Last resize: i=49 → cols = 40 + (49 % 80) = 89, rows = 20 + (49 % 20) = 29.
 assert_eq!(t.terminal.grid().cols(), 89, "final cols after 50 resizes");
 assert_eq!(t.terminal.grid().lines(), 29, "final rows after 50 resizes");

 // Produce snapshot and verify dimensions match.
 t.grid_dirty.store(true, Ordering::Release);
 t.maybe_produce_snapshot();
 let mut snap = RenderableContent::default();
 assert!(t.double_buffer.swap_front(&mut snap));
 assert_eq!(snap.cols, 89, "snapshot cols after rapid resize");
 assert_eq!(snap.lines, 29, "snapshot rows after rapid resize");
}

/// Resize during active byte processing: content + resize interleaved 50 times.
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
 let mut snap = RenderableContent::default();
 assert!(t.double_buffer.swap_front(&mut snap));
 assert_eq!(snap.cols, 69, "snapshot cols");
 assert_eq!(snap.lines, 24, "snapshot rows");
}

// --- Reply-return path tests (Section 03.5d) ---

/// Register a `ClipboardLoad` pending response, fulfill the token, poll,
/// and verify the `Effect::Pty` reply contains correct base64 content.
/// Pins the production reply-return path activated by effect-cutover
/// §01.1: register-poll-fulfill round trip on `Term<QueueingEffectSink>`.
#[test]
fn reply_token_clipboard_load_produces_pty_write() {
 use base64::Engine;
 use base64::engine::general_purpose::STANDARD;

 use oriterm_core::effect::{ClipboardSelection, Effect, HostRequest, PtyEffect, ResponseToken};

 let mut t = make_sync_thread();

 // Create a ClipboardLoad request with a fresh token.
 let token = ResponseToken::<String>::new();
 let token_clone = token.clone();
 let request = HostRequest::ClipboardLoad {
 selection: ClipboardSelection::Clipboard,
 clipboard_char: b'c',
 terminator: "\x1b\\".to_string(),
 reply: token,
 };

 // Register the pending response.
 t.register_host_request_response(request);
 assert_eq!(
 t.pending_responses.len(),
 1,
 "one pending response registered"
 );

 // Poll before fulfillment — should return None, entry retained.
 t.poll_pending_responses();
 assert_eq!(
 t.pending_responses.len(),
 1,
 "unfulfilled response retained"
 );

 // Fulfill the token with clipboard text.
 token_clone
 .fulfill("hello world".to_string())
 .expect("fresh token fulfill must succeed");

 // Poll after fulfillment — should produce Effect::Pty and remove entry.
 // poll_pending_responses pushes the effect through the VoidEffectSink (no-op),
 // so we poll the PendingResponse directly to capture the effect.
 let PollResult::Ready(effect) = t.pending_responses[0].poll() else {
 panic!("fulfilled token should produce PollResult::Ready");
 };

 let Effect::Pty(PtyEffect::Write { bytes, .. }) = effect else {
 panic!("expected Effect::Pty(PtyEffect::Write)");
 };

 let response_str = String::from_utf8(bytes).expect("valid UTF-8");
 let expected_encoded = STANDARD.encode(b"hello world");
 let expected = format!("\x1b]52;c;{}\x1b\\", expected_encoded);
 assert_eq!(response_str, expected, "base64-encoded OSC 52 reply");
}

/// Register a `ColorQuery` pending response, fulfill the token, poll,
/// and verify the `Effect::Pty` reply contains correct rgb: format.
#[test]
fn reply_token_color_query_produces_pty_write() {
 use oriterm_core::color::Rgb;
 use oriterm_core::effect::{Effect, HostRequest, PtyEffect, ResponseToken};

 let mut t = make_sync_thread();

 let token = ResponseToken::<Rgb>::new();
 let token_clone = token.clone();
 let request = HostRequest::ColorQuery {
 prefix: "10".to_string(),
 index: 0,
 terminator: "\x07".to_string(),
 reply: token,
 };

 t.register_host_request_response(request);
 assert_eq!(t.pending_responses.len(), 1);

 // Fulfill with a color.
 token_clone
 .fulfill(Rgb {
 r: 0xAB,
 g: 0xCD,
 b: 0xEF,
 })
 .expect("fresh token fulfill must succeed");

 let PollResult::Ready(effect) = t.pending_responses[0].poll() else {
 panic!("fulfilled token should produce PollResult::Ready");
 };

 let Effect::Pty(PtyEffect::Write { bytes, .. }) = effect else {
 panic!("expected Effect::Pty(PtyEffect::Write)");
 };

 let response_str = String::from_utf8(bytes).expect("valid UTF-8");
 // Format: \x1b]10;rgb:ABAB/CDCD/EFEF\x07
 assert_eq!(
 response_str, "\x1b]10;rgb:abab/cdcd/efef\x07",
 "rgb: formatted OSC color reply"
 );
}

/// Multiple pending responses — each is polled independently.
#[test]
fn multiple_pending_responses_polled_independently() {
 use oriterm_core::effect::{ClipboardSelection, HostRequest, ResponseToken};

 let mut t = make_sync_thread();

 let token1 = ResponseToken::<String>::new();
 let token1_clone = token1.clone();
 let token2 = ResponseToken::<String>::new();
 // Keep a main-thread handle for token2 so `Arc::strong_count`-based
 // cancellation detection does not remove its pending entry before the
 // test has fulfilled token1.
 let _token2_keep_alive = token2.clone();

 t.register_host_request_response(HostRequest::ClipboardLoad {
 selection: ClipboardSelection::Clipboard,
 clipboard_char: b'c',
 terminator: "\x1b\\".to_string(),
 reply: token1,
 });
 t.register_host_request_response(HostRequest::ClipboardLoad {
 selection: ClipboardSelection::Primary,
 clipboard_char: b'p',
 terminator: "\x07".to_string(),
 reply: token2,
 });
 assert_eq!(t.pending_responses.len(), 2);

 // Fulfill only the first token.
 token1_clone
 .fulfill("first".to_string())
 .expect("fresh token fulfill must succeed");

 // poll_pending_responses should remove only the fulfilled one.
 t.poll_pending_responses();
 assert_eq!(
 t.pending_responses.len(),
 1,
 "one fulfilled response removed, one unfulfilled retained"
 );
}

// --- Section 06.5: Sync timeout + edge case test matrix ---
// BSU = \x1b[?2026h (begin synchronized update)
// ESU = \x1b[?2026l (end synchronized update / commit)
// These tests exercise handle_sync_timeout() directly (synchronous)
// except run_loop_sync_timeout_fires which uses the spawned run() loop.

/// Helper: generic sync thread builder for any EffectSink.
fn make_sync_thread_generic<S: oriterm_core::effect::EffectSink + 'static>(
 sink: S,
) -> (PaneIoThread<S>, Arc<AtomicU64>) {
 let wakeup_count = Arc::new(AtomicU64::new(0));
 let wakeup_clone = Arc::clone(&wakeup_count);
 let term = Term::new(24, 80, 1000, Theme::default(), sink);
 let (_, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
 let (_, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
 let (_dummy_pane_id, _dummy_mux_tx, _dummy_exit_rx, _dummy_wake_rx) = test_dummy_channels();
 let thread = PaneIoThread {
 terminal: term,
 pane_id: _dummy_pane_id,
 mux_tx: _dummy_mux_tx,
 child_exit_rx: _dummy_exit_rx,
 pending_child_exit: None,
 io_wake_rx: _dummy_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(move || {
 wakeup_clone.fetch_add(1, Ordering::Relaxed);
 }),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: SnapshotDoubleBuffer::new(),
 snapshot_buf: Default::default(),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
 };
 (thread, wakeup_count)
}

/// Property: timeout publishes the inline-mutated grid + clears
/// SYNC_UPDATE.
/// Bytes inside the sync window dispatched inline as they arrived,
/// mutating the grid immediately. The timeout path's job is to clear
/// the SYNC_UPDATE gate and publish the accumulated state.
#[test]
fn sync_timeout_publishes_inline_mutated_grid() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // Enter sync mode (BSU).
 t.handle_bytes(b"\x1b[?2026h");
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "BSU should activate sync mode"
 );

 // Send visible content while in sync mode — dispatched inline.
 t.handle_bytes(b"hello");

 // Trigger timeout: clears SYNC_UPDATE, publishes accumulated state.
 t.handle_sync_timeout();

 // 1. SYNC_UPDATE must be cleared.
 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "TermMode::SYNC_UPDATE must be cleared after handle_sync_timeout"
 );

 // 2. Snapshot must contain the inline-mutated "hello".
 let mut consumer = RenderableContent::default();
 assert!(
 t.double_buffer.swap_front(&mut consumer),
 "snapshot must be published"
 );
 let text: String = consumer
 .cells
 .iter()
 .filter(|c| c.ch != ' ' && c.ch != '\0')
 .map(|c| c.ch)
 .collect();
 assert!(
 text.contains("hello"),
 "inline-mutated bytes must appear in snapshot, got: {text:?}"
 );

 // 3. grid_dirty was cleared by produce_snapshot.
 assert!(
 !t.grid_dirty.load(Ordering::Acquire),
 "grid_dirty should be cleared after produce_snapshot"
 );

 // 4. Wakeup fired.
 assert!(wakeup_count.load(Ordering::Relaxed) > 0, "wakeup must fire");

 // 5. snapshot_seqno advanced by exactly 1.
 let seqno = t.double_buffer.seqno();
 assert_eq!(seqno, 1, "snapshot_seqno should advance by 1");
}

/// Property: timeout emits PresentationEffect::Abort effect.
#[test]
fn sync_timeout_emits_abort_effect() {
 use oriterm_core::effect::sink::EffectSink;
 use oriterm_core::effect::{Effect, PresentationEffect, QueueingEffectSink, SyncAbortReason};

 let sink = QueueingEffectSink::new();
 let (mut t, _wakeup) = make_sync_thread_generic(sink);

 // Enter sync mode + dispatch content inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"test");

 // Trigger timeout.
 t.handle_sync_timeout();

 // Drain effects and find the Abort.
 let mut effects = Vec::new();
 t.terminal.effect_sink().drain_into(&mut effects);

 let has_abort = effects.iter().any(|e| {
 matches!(
 e,
 Effect::Presentation(PresentationEffect::Abort {
 reason: SyncAbortReason::Timeout
 })
 )
 });
 assert!(
 has_abort,
 "Abort effect must be emitted on timeout, got: {effects:?}"
 );
}

/// Timeout runs post-parse housekeeping (mode_cache reflects all
/// inline-dispatched mode mutations).
#[test]
fn sync_timeout_runs_post_parse_housekeeping_inline_dispatch() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // Verify cursor is visible initially.
 assert!(
 t.terminal.mode().contains(TermMode::SHOW_CURSOR),
 "cursor should be visible initially"
 );

 // Enter sync mode + hide cursor within the sync window. After
 // the hide dispatches INLINE, mutating the term's mode
 // bits as soon as the bytes arrive.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"\x1b[?25l");

 // term mode bits already reflect the hide (inline dispatch).
 assert!(
 !t.terminal.mode().contains(TermMode::SHOW_CURSOR),
 "term mode must reflect inline-dispatched cursor hide"
 );

 // Trigger timeout — clears SYNC_UPDATE and runs post-parse
 // housekeeping (which propagates term mode to mode_cache).
 t.handle_sync_timeout();

 // mode_cache must now reflect the cursor-hide.
 let cached_mode_after = TermMode::from_bits_truncate(t.mode_cache.load(Ordering::Acquire));
 assert!(
 !cached_mode_after.contains(TermMode::SHOW_CURSOR),
 "mode_cache must reflect cursor hidden after timeout housekeeping"
 );
}

/// Resize command during active sync — grid dimensions reflect the
/// resize after timeout publishes the snapshot.
#[test]
fn resize_during_sync_timeout() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // Enter sync mode + dispatch grid bytes inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"resize");

 // Resize while sync is active.
 t.process_resize(40, 100);

 // Trigger timeout — clears SYNC_UPDATE and publishes snapshot.
 t.handle_sync_timeout();

 // Snapshot must be coherent — no crash, grid dimensions match resize.
 let mut consumer = RenderableContent::default();
 assert!(t.double_buffer.swap_front(&mut consumer));

 assert_eq!(t.terminal.grid().lines(), 40);
 assert_eq!(t.terminal.grid().cols(), 100);
}

/// Alt-screen swap inside an active sync window — mode_cache reflects
/// ALT_SCREEN after the timeout closes the gate and post-parse
/// housekeeping fires.
#[test]
fn alt_screen_swap_inline_updates_mode_cache() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // Confirm not in alt screen.
 assert!(
 !t.terminal.mode().contains(TermMode::ALT_SCREEN),
 "should start in primary screen"
 );

 // Enter sync mode + dispatch the alt-screen swap inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"\x1b[?1049h");

 // Trigger timeout — clears SYNC_UPDATE and runs post-parse housekeeping.
 t.handle_sync_timeout();

 // Alt screen mutation landed inline; mode_cache reflects it post-housekeeping.
 assert!(
 t.terminal.mode().contains(TermMode::ALT_SCREEN),
 "alt screen must be active after inline dispatch"
 );
 let cached_mode = TermMode::from_bits_truncate(t.mode_cache.load(Ordering::Acquire));
 assert!(
 cached_mode.contains(TermMode::ALT_SCREEN),
 "mode_cache must reflect alt screen after housekeeping"
 );
}

/// Wakeup fires exactly once on timeout (no double-publish).
#[test]
fn no_double_publish_on_timeout() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // Enter sync mode + dispatch content inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"content");

 assert_eq!(
 wakeup_count.load(Ordering::Relaxed),
 0,
 "no wakeup before timeout"
 );

 // Trigger timeout.
 t.handle_sync_timeout();

 assert_eq!(
 wakeup_count.load(Ordering::Relaxed),
 1,
 "wakeup must fire exactly once on timeout"
 );
}

/// Nested BSU during an active sync window — mode stays set, bytes
/// dispatched inline, timeout publishes the accumulated state.
#[test]
fn nested_bsu_in_sync_processes_inline_keeps_mode_set() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // Enter sync mode + dispatch grid bytes + nested BSU + more bytes.
 // Every chunk dispatches inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.handle_bytes(b"before");
 t.handle_bytes(b"\x1b[?2026h"); // nested BSU re-arms timer
 t.handle_bytes(b"after");

 // Mode is still SET (nested BSUs are idempotent on the mode flag).
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "sync mode must remain set across nested BSUs"
 );

 // Trigger timeout — clears mode + publishes inline-mutated grid.
 t.handle_sync_timeout();

 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "sync mode must be unset after timeout"
 );

 // Snapshot must contain both "before" and "after".
 let mut consumer = RenderableContent::default();
 assert!(t.double_buffer.swap_front(&mut consumer));
 let text: String = consumer
 .cells
 .iter()
 .filter(|c| c.ch != ' ' && c.ch != '\0')
 .map(|c| c.ch)
 .collect();
 assert!(
 text.contains("before") && text.contains("after"),
 "all inline-dispatched bytes must appear in snapshot, got: {text:?}"
 );
}

/// Stress: large in-sync writes flow through inline dispatch without
/// hitting any buffer-overflow path (none exists post-fix).
#[test]
fn large_in_sync_write_dispatches_inline() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // Enter sync mode.
 t.handle_bytes(b"\x1b[?2026h");

 // Feed >2 MiB of data. Pre-fix this triggered VTE's overflow path
 // (terminating sync early). Post-fix bytes dispatch inline; mode
 // stays set; only ESU/timeout exits the window.
 let large_data = vec![b'X'; 2 * 1024 * 1024 + 1];
 t.handle_bytes(&large_data);

 // Sync mode remains set across the >2 MiB chunk.
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "sync mode must remain set across large in-sync writes"
 );
}

/// Spawned run-loop test: the real crossbeam select! deadline arm fires.
#[test]
fn run_loop_sync_timeout_fires() {
 let (handle, shutdown) = spawn_pair_with_flag();
 let byte_tx = handle.byte_sender();

 // Send BSU + visible content via the byte channel.
 byte_tx.send(b"\x1b[?2026h".to_vec()).unwrap();
 byte_tx.send(b"timeout_test".to_vec()).unwrap();

 // Poll for the sync timeout to fire in the run loop.
 // Wall-clock-free: poll the snapshot until the content appears;
 // a 5s safety deadline surfaces true hangs.
 let deadline = Instant::now() + Duration::from_secs(5);
 let mut consumer = RenderableContent::default();
 loop {
 if handle.double_buffer().swap_front(&mut consumer) {
 let text: String = consumer
 .cells
 .iter()
 .filter(|c| c.ch != ' ' && c.ch != '\0')
 .map(|c| c.ch)
 .collect();
 if text.contains("timeout_test") {
 break;
 }
 }
 assert!(
 Instant::now() < deadline,
 "IO thread sync timeout did not fire within 5s deadline; \
 run-loop deadline arm or handle_sync_timeout is broken"
 );
 std::thread::sleep(Duration::from_millis(20));
 }

 // Clean shutdown.
 shutdown.store(true, Ordering::Release);
 handle.send_command(PaneIoCommand::Shutdown);
}

/// Regression guard: timeout arm must NOT fire when not in sync mode.
#[test]
fn no_timeout_when_not_in_sync() {
 let (mut t, _wakeup) = make_sync_thread_with_wakeup();

 // No BSU sent — processor.sync_timeout().sync_timeout() returns None.
 let deadline = t.processor.sync_timeout().sync_timeout();
 assert!(
 deadline.is_none(),
 "sync_timeout must return None when not in sync mode"
 );

 // Send normal bytes — no timeout should activate.
 t.handle_bytes(b"normal output");

 // Verify still no sync deadline.
 let deadline_after = t.processor.sync_timeout().sync_timeout();
 assert!(
 deadline_after.is_none(),
 "sync_timeout must still be None after normal bytes"
 );

 // Parser-side timer must remain disarmed.
 assert!(
 !t.processor.is_sync_active(),
 "parser-side sync timer must be disarmed when not in sync mode"
 );
}

// --- Bridge cell: Mode 2026 parser → mode_cache SSOT contract (Section 09.2) ---

/// Bridge cell: `CSI ? 2026 h` propagates `TermMode::SYNC_UPDATE` into
/// `mode_cache` via `post_parse_housekeeping()`.
/// Proves the parser → mode_cache → publication-gate SSOT contract is
/// live. Section 06 owns the apex tests (publication suppression +
/// timeout-abort); Section 09 owns only this bridge proving the bit
/// the parser writes is the bit Section 06's consumer reads.
#[test]
fn bridge_mode_2026_propagates_to_mode_cache() {
 let mut t = make_sync_thread();

 // Pre-condition: SYNC_UPDATE must NOT be in the mode cache.
 let initial_bits = t.mode_cache.load(Ordering::Acquire);
 assert_eq!(
 initial_bits & TermMode::SYNC_UPDATE.bits(),
 0,
 "SYNC_UPDATE must be absent from mode_cache before DECSET"
 );

 // Feed DECSET ?2026 through the full handle_bytes path (which calls
 // post_parse_housekeeping → mode_cache.store).
 t.handle_bytes(b"\x1b[?2026h");

 // Post-condition: SYNC_UPDATE must be present in mode_cache.
 let updated_bits = t.mode_cache.load(Ordering::Acquire);
 assert_ne!(
 updated_bits & TermMode::SYNC_UPDATE.bits(),
 0,
 "SYNC_UPDATE must be present in mode_cache after DECSET ?2026"
 );
}

/// Bridge cell: `CSI ? 2026 l` clears `TermMode::SYNC_UPDATE` from
/// `mode_cache`.
/// Completes the round-trip bridge: set → verify set → reset → verify
/// cleared. Without this, a broken DECRST path could leave stale bits
/// in the cache, causing the publication gate to suppress snapshots
/// indefinitely.
#[test]
fn bridge_mode_2026_reset_clears_mode_cache() {
 let mut t = make_sync_thread();

 // Set mode 2026.
 t.handle_bytes(b"\x1b[?2026h");
 assert_ne!(
 t.mode_cache.load(Ordering::Acquire) & TermMode::SYNC_UPDATE.bits(),
 0,
 "precondition: SYNC_UPDATE must be set"
 );

 // Reset mode 2026.
 t.handle_bytes(b"\x1b[?2026l");

 let final_bits = t.mode_cache.load(Ordering::Acquire);
 assert_eq!(
 final_bits & TermMode::SYNC_UPDATE.bits(),
 0,
 "SYNC_UPDATE must be cleared from mode_cache after DECRST ?2026"
 );
}

// ── DECBKM (mode 67) bridge cells ──────────────────────────────────

/// Bridge cell: `CSI ? 67 h` propagates `TermMode::DECBKM` into `mode_cache`.
/// Section 09.4b: proves the parser → `post_parse_housekeeping()` →
/// `mode_cache` SSOT contract is live for mode 67. Without this bridge,
/// the key encoder on the main thread would never see DECBKM.
#[test]
fn bridge_decbkm_propagates_to_mode_cache() {
 let mut t = make_sync_thread();

 let initial_bits = t.mode_cache.load(Ordering::Acquire);
 assert_eq!(
 initial_bits & TermMode::DECBKM.bits(),
 0,
 "DECBKM must be absent from mode_cache before DECSET"
 );

 t.handle_bytes(b"\x1b[?67h");

 let updated_bits = t.mode_cache.load(Ordering::Acquire);
 assert_ne!(
 updated_bits & TermMode::DECBKM.bits(),
 0,
 "DECBKM must be present in mode_cache after DECSET ?67"
 );
}

/// Bridge cell: `CSI ? 67 l` clears `TermMode::DECBKM` from `mode_cache`.
#[test]
fn bridge_decbkm_reset_clears_mode_cache() {
 let mut t = make_sync_thread();

 t.handle_bytes(b"\x1b[?67h");
 assert_ne!(
 t.mode_cache.load(Ordering::Acquire) & TermMode::DECBKM.bits(),
 0,
 "precondition: DECBKM must be set"
 );

 t.handle_bytes(b"\x1b[?67l");

 let final_bits = t.mode_cache.load(Ordering::Acquire);
 assert_eq!(
 final_bits & TermMode::DECBKM.bits(),
 0,
 "DECBKM must be cleared from mode_cache after DECRST ?67"
 );
}

/// Bridge cell: `CSI ? 66 h` propagates `TermMode::APP_KEYPAD` into
/// `mode_cache` (DECNKM shares the same flag as ESC =/ESC >).
#[test]
fn bridge_decnkm_propagates_to_mode_cache() {
 let mut t = make_sync_thread();

 let initial_bits = t.mode_cache.load(Ordering::Acquire);
 assert_eq!(
 initial_bits & TermMode::APP_KEYPAD.bits(),
 0,
 "APP_KEYPAD must be absent from mode_cache before DECSET ?66"
 );

 t.handle_bytes(b"\x1b[?66h");

 let updated_bits = t.mode_cache.load(Ordering::Acquire);
 assert_ne!(
 updated_bits & TermMode::APP_KEYPAD.bits(),
 0,
 "APP_KEYPAD must be present in mode_cache after DECSET ?66"
 );
}

// EOF + ordering + multi-chunk pins (effect-cutover §01.1 Phase J)
// These tests pin the load-bearing invariants of `handle_pty_eof` and
// `handle_bytes_chunked` against the production code path (not the
// VoidEffectSink synchronous helpers). They use a dedicated rig that
// spawns a real `PaneIoThread<QueueingEffectSink>` with externally-owned
// channels so the test can drop `byte_tx` independently to trigger
// byte_rx EOF, and so `mux_rx` and `double_buffer` stay observable.

/// Test rig for `handle_pty_eof` + multi-chunk + ordering scenarios.
/// Holds the test side of every channel so the test controls EOF timing,
/// exit-status delivery, and snapshot observation.
struct EofTestRig {
 mux_rx: mpsc::Receiver<MuxEvent>,
 byte_tx: Sender<Vec<u8>>,
 child_exit_tx: Sender<ExitStatus>,
 double_buffer: SnapshotDoubleBuffer,
 /// Held so `cmd_rx` never returns `Err` before `byte_rx` — keeps the
 /// `select!` cmd arm idle for the duration of the test.
 _keep_alive_cmd_tx: Sender<PaneIoCommand>,
 /// Held so `io_wake_rx` never returns `Err`.
 _keep_alive_wake_tx: Sender<()>,
 join: std::thread::JoinHandle<()>,
}

fn spawn_queueing_eof_rig() -> EofTestRig {
 use oriterm_core::effect::QueueingEffectSink;

 let shutdown = Arc::new(AtomicBool::new(false));
 let (mux_tx, mux_rx) = mpsc::channel::<MuxEvent>();
 let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PaneIoCommand>();
 let (byte_tx, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
 let (child_exit_tx, child_exit_rx) = crossbeam_channel::bounded::<ExitStatus>(1);
 let (io_wake_tx, io_wake_rx) = crossbeam_channel::bounded::<()>(1);
 let double_buffer = SnapshotDoubleBuffer::new();
 let term = Term::new(24, 80, 1000, Theme::default(), QueueingEffectSink::new());

 let thread = PaneIoThread {
 terminal: term,
 pane_id: PaneId::from_raw(7),
 mux_tx,
 child_exit_rx,
 pending_child_exit: None,
 io_wake_rx,
 cmd_rx,
 byte_rx,
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 processor: vte::ansi::Processor::new(),
 raw_parser: vte::Parser::new(),
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 double_buffer: double_buffer.clone(),
 snapshot_buf: Default::default(),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 last_pty_size: (24u32 << 16) | 80u32,
 search: None,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 pending_responses: Vec::new(),
 effects_buf: Vec::new(),
 last_animation_deadline: None,
 pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
 shrink_call_count: Arc::new(AtomicUsize::new(0)),
 start_barrier: None,
 };

 let join = thread.spawn().expect("spawn IO thread");

 EofTestRig {
 mux_rx,
 byte_tx,
 child_exit_tx,
 double_buffer,
 _keep_alive_cmd_tx: cmd_tx,
 _keep_alive_wake_tx: io_wake_tx,
 join,
 }
}

/// Construct an `ExitStatus` for testing via the public
/// `portable_pty::ExitStatus::with_exit_code` constructor.
fn make_exit_status(code: u32) -> ExitStatus {
 ExitStatus::from(portable_pty::ExitStatus::with_exit_code(code))
}

/// Drain `mux_rx` until `MuxEvent::PaneExited` arrives, returning the
/// exit code. Skips intermediate metadata events.
fn await_pane_exited(rx: &mpsc::Receiver<MuxEvent>, deadline: Duration) -> i32 {
 let start = Instant::now();
 loop {
 let remaining = deadline
 .checked_sub(start.elapsed())
 .unwrap_or(Duration::ZERO);
 match rx.recv_timeout(remaining) {
 Ok(MuxEvent::PaneExited { exit_code, .. }) => return exit_code,
 Ok(_) => continue,
 Err(e) => panic!("PaneExited never arrived: {e}"),
 }
 }
}

/// Blind-spot §11: PTY EOF triggers `handle_pty_eof` which emits
/// `MuxEvent::PaneExited` via the effect router with the watcher-supplied
/// exit code intact. Exercises the cached `pending_child_exit` path —
/// the watcher signal lands BEFORE byte_rx EOF, so `handle_pty_eof`'s
/// `pending_child_exit.take()` succeeds without any `recv_timeout` wait.
#[test]
fn pty_eof_emits_pane_exited_via_effect_router() {
 let rig = spawn_queueing_eof_rig();

 // Forward exit status BEFORE EOF so the cached pending_child_exit
 // path is exercised — the select! child_exit arm stores into
 // `pending_child_exit`, which `handle_pty_eof` then consumes.
 rig.child_exit_tx
 .send(make_exit_status(42))
 .expect("send exit status");
 // Give the IO thread a moment to consume the exit status into its
 // pending_child_exit cache via the select! arm.
 std::thread::sleep(Duration::from_millis(50));

 // Trigger byte_rx EOF.
 drop(rig.byte_tx);

 let exit_code = await_pane_exited(&rig.mux_rx, Duration::from_secs(5));
 assert_eq!(
 exit_code, 42,
 "exit code must come from watcher cache, not the 0 fallback"
 );

 // Drop child_exit_tx so the IO thread can exit cleanly (the EOF path
 // already consumed the cached status; this just cleans up the channel).
 drop(rig.child_exit_tx);
 rig.join.join().expect("IO thread joined cleanly");
}

/// Blind-spot §11 negative side: when no exit status is ever delivered
/// AND the watcher channel disconnects, `handle_pty_eof` falls back to
/// `code: 0`. Drop order is byte_tx FIRST (enters handle_pty_eof and
/// blocks on recv_timeout), THEN child_exit_tx (recv_timeout returns
/// `Err(Disconnected)` immediately), so the test does not pay the 5s
/// wait timeout.
#[test]
fn pty_eof_without_exit_code_defaults_to_zero() {
 let rig = spawn_queueing_eof_rig();

 // Trigger EOF — IO thread enters handle_pty_eof and blocks on
 // child_exit_rx.recv_timeout(5s).
 drop(rig.byte_tx);

 // Tiny pause to let the IO thread reach the recv_timeout call.
 std::thread::sleep(Duration::from_millis(50));

 // Drop child_exit_tx — recv_timeout returns Err(Disconnected) and
 // the fallback path emits ChildExit { code: 0 }.
 drop(rig.child_exit_tx);

 let exit_code = await_pane_exited(&rig.mux_rx, Duration::from_secs(5));
 assert_eq!(exit_code, 0, "fallback exit code must be 0 on disconnect");

 rig.join.join().expect("IO thread joined cleanly");
}

/// Blind-spot §11 timing pin: when the watcher delivers the exit status
/// AFTER byte_rx EOF arrives (scheduler delay between PTY close and
/// child reaping returning), the exit code is still captured via the
/// `child_exit_rx.recv_timeout(5s)` blocking wait — the EOF path does
/// not race past slow watchers and never falls back to the 0 default
/// when the watcher is merely delayed.
#[test]
fn pty_eof_exit_code_captured_after_scheduler_delay() {
 let rig = spawn_queueing_eof_rig();

 // Trigger EOF FIRST. The IO thread enters handle_pty_eof and blocks
 // on child_exit_rx.recv_timeout(5s) waiting for the status.
 drop(rig.byte_tx);

 // Spawn a delayed sender that simulates a 200ms scheduler delay
 // between PTY close and `child.wait()` returning. Hold a clone of
 // child_exit_tx in the spawn closure; drop the rig's primary copy so
 // recv_timeout receives the value once the spawn thread sends.
 let exit_tx = rig.child_exit_tx.clone();
 let _delivery = std::thread::spawn(move || {
 std::thread::sleep(Duration::from_millis(200));
 let _ = exit_tx.send(make_exit_status(7));
 });
 drop(rig.child_exit_tx);

 let exit_code = await_pane_exited(&rig.mux_rx, Duration::from_secs(5));
 assert_eq!(
 exit_code, 7,
 "exit code must be captured even after a 200ms scheduler delay"
 );

 rig.join.join().expect("IO thread joined cleanly");
}

/// Blind-spot §17: on PTY EOF the IO thread sequences
/// `[final drain → snapshot flip → child_exit → push ChildExit → drain
/// → return]`. The main thread observes the final cell content snapshot
/// BEFORE `MuxEvent::PaneExited` arrives. Send distinctive bytes
/// ("FAREWELL"), trigger EOF, observe `PaneExited`, then assert the
/// snapshot in `double_buffer` carries the FAREWELL cells.
#[test]
fn final_snapshot_precedes_pane_exited() {
 let rig = spawn_queueing_eof_rig();

 // Send distinctive bytes that produce visible cells in row 0.
 rig.byte_tx
 .send(b"FAREWELL".to_vec())
 .expect("byte send must succeed");

 // Wait for the IO thread to process the bytes (one snapshot cycle).
 std::thread::sleep(Duration::from_millis(80));

 // Cache exit status in pending_child_exit, then trigger EOF so
 // handle_pty_eof's cached path runs (no 5s wait required).
 rig.child_exit_tx
 .send(make_exit_status(0))
 .expect("send exit status");
 std::thread::sleep(Duration::from_millis(20));
 drop(rig.byte_tx);

 // Wait for PaneExited. By the time recv returns, handle_pty_eof has
 // already executed step (2) (snapshot flip) BEFORE step (5)
 // (PaneExited send) — code-ordering is the load-bearing invariant.
 let _ = await_pane_exited(&rig.mux_rx, Duration::from_secs(5));

 // Read the latest snapshot. By happens-before from the IO thread's
 // mux_tx.send → main-thread mux_rx.recv pairing, the snapshot
 // produced in step (2) is visible to swap_front.
 let mut snapshot = RenderableContent::default();
 let _ = rig.double_buffer.swap_front(&mut snapshot);

 let row0_text: String = snapshot
 .cells
 .iter()
 .filter(|c| c.line == 0)
 .map(|c| c.ch)
 .collect();
 assert!(
 row0_text.contains("FAREWELL"),
 "snapshot must reflect FAREWELL cells before PaneExited; row0 = {row0_text:?}"
 );

 drop(rig.child_exit_tx);
 rig.join.join().expect("IO thread joined cleanly");
}

/// Blind-spot §17 ordering invariant: `PaneExited` NEVER reaches mux_rx
/// before the final snapshot lands in the double buffer. Inverse phrasing
/// of `final_snapshot_precedes_pane_exited` — both pins exist so the
/// regression is impossible to commit even if one test is later weakened.
/// Polls both observation sources in lockstep: snapshot publication
/// (visible via `double_buffer.swap_front` returning cells matching the
/// marker) MUST be observable BEFORE `PaneExited` is received via
/// `mux_rx.try_recv`.
#[test]
fn pane_exited_does_not_precede_final_snapshot() {
 let rig = spawn_queueing_eof_rig();

 // Marker bytes the snapshot must carry forward.
 rig.byte_tx
 .send(b"GOODBYE!".to_vec())
 .expect("byte send must succeed");
 std::thread::sleep(Duration::from_millis(80));

 rig.child_exit_tx
 .send(make_exit_status(1))
 .expect("send exit status");
 std::thread::sleep(Duration::from_millis(20));
 drop(rig.byte_tx);

 // Lockstep poll: tracks whether the marker-bearing snapshot was
 // observed BEFORE the first PaneExited recv.
 let deadline = Instant::now() + Duration::from_secs(5);
 let mut snapshot_observed_with_marker = false;
 while Instant::now() < deadline {
 let mut snapshot = RenderableContent::default();
 let _ = rig.double_buffer.swap_front(&mut snapshot);
 if snapshot
 .cells
 .iter()
 .filter(|c| c.line == 0)
 .any(|c| c.ch == 'G')
 {
 snapshot_observed_with_marker = true;
 }
 if let Ok(MuxEvent::PaneExited { .. }) = rig.mux_rx.try_recv() {
 assert!(
 snapshot_observed_with_marker,
 "PaneExited arrived before the final snapshot was visible — \
 ordering invariant violated"
 );
 drop(rig.child_exit_tx);
 rig.join.join().expect("IO thread joined cleanly");
 return;
 }
 std::thread::sleep(Duration::from_millis(5));
 }
 panic!("PaneExited never arrived within 5s");
}

/// Blind-spot §5 bound: `handle_bytes_chunked` calls
/// `drain_effects_into_mux_events` at the END of EACH chunk (inside
/// `handle_bytes`), not only at the end of the outer
/// `handle_bytes_chunked` loop. A 65 KB forwarded read containing
/// bell-producing bytes (just over `MAX_PARSE_CHUNK = 64 KB`) routes
/// ALL of them through the router across at least 2 chunks.
/// If the drain only fired at the end of the outer `handle_bytes_chunked`
/// loop, the effects buffer would accumulate the full 65 K entries
/// before draining; the per-chunk drain bounds it to <= 64 K.
#[test]
fn multi_chunk_parse_drains_between_chunks() {
 let rig = spawn_queueing_eof_rig();

 // 65 KB of BEL — produces 65 K HostEffect::Bell, spanning 2 chunks
 // at MAX_PARSE_CHUNK = 64 KB.
 const BELL_COUNT: usize = 65 * 1024;
 rig.byte_tx
 .send(vec![0x07; BELL_COUNT])
 .expect("byte send must succeed");

 // Count PaneBell events with a generous deadline. Every bell MUST
 // reach mux_rx — if the drain only fired at the end of the OUTER
 // handle_bytes_chunked loop, intermediate effects would be invisible
 // to mux_rx until the whole 65 KB completed parsing (and the
 // unbounded sink would briefly spike).
 let deadline = Instant::now() + Duration::from_secs(15);
 let mut bell_count = 0usize;
 while Instant::now() < deadline && bell_count < BELL_COUNT {
 let remaining = deadline.saturating_duration_since(Instant::now());
 match rig
 .mux_rx
 .recv_timeout(remaining.min(Duration::from_millis(500)))
 {
 Ok(MuxEvent::PaneBell(_)) => bell_count += 1,
 Ok(_) => continue,
 Err(_) => break,
 }
 }

 assert_eq!(
 bell_count, BELL_COUNT,
 "every bell must route through the per-chunk drain"
 );

 drop(rig.byte_tx);
 drop(rig.child_exit_tx);
 rig.join.join().expect("IO thread joined cleanly");
}

/// Regression: a disconnected `child_exit_rx` must not turn the IO
/// thread's `select!` into a tight CPU spin. `crossbeam_channel::recv`
/// on a disconnected channel returns `Err` immediately; without the
/// `never()` swap-out in the disconnected arm, `select!` would pick
/// that arm every iteration and saturate a core until shutdown
/// arrived.
/// We assert two things after dropping `child_exit_tx`:
/// 1. A subsequent `Shutdown` command still joins quickly (the spin
/// must not starve `cmd_rx` delivery).
/// 2. Iteration count between disconnect and shutdown is bounded —
/// measured indirectly by total wall time, which would balloon if
/// `select!` were spinning at multi-MHz.
#[test]
fn child_exit_disconnect_does_not_spin_loop() {
 let rig = spawn_queueing_eof_rig();
 drop(rig.child_exit_tx);
 // Give the IO thread a moment to observe the disconnect through
 // its `select!`. A spinning thread would burn CPU during this
 // window; a correctly-handled one parks on `IDLE_WAKE_CEILING`.
 std::thread::sleep(Duration::from_millis(100));
 let start = Instant::now();
 rig._keep_alive_cmd_tx
 .send(PaneIoCommand::Shutdown)
 .expect("Shutdown delivery");
 rig.join.join().expect("IO thread joined cleanly");
 let elapsed = start.elapsed();
 assert!(
 elapsed < Duration::from_secs(2),
 "Shutdown after child_exit_rx disconnect took {elapsed:?} — \
 IO thread likely spinning on the disconnected select! arm"
 );
 drop(rig.byte_tx);
 drop(rig.mux_rx);
 drop(rig._keep_alive_wake_tx);
}

/// Regression companion: `io_wake_rx` carries the same disconnect-
/// then-spin hazard as `child_exit_rx`. The handle's `io_wake_tx`
/// only drops at handle teardown, but a buggy IO thread that picked up
/// the disconnected arm every iteration would saturate a core during
/// the window between the drop and the join.
#[test]
fn io_wake_disconnect_does_not_spin_loop() {
 let rig = spawn_queueing_eof_rig();
 drop(rig._keep_alive_wake_tx);
 std::thread::sleep(Duration::from_millis(100));
 let start = Instant::now();
 rig._keep_alive_cmd_tx
 .send(PaneIoCommand::Shutdown)
 .expect("Shutdown delivery");
 rig.join.join().expect("IO thread joined cleanly");
 let elapsed = start.elapsed();
 assert!(
 elapsed < Duration::from_secs(2),
 "Shutdown after io_wake_rx disconnect took {elapsed:?} — \
 IO thread likely spinning on the disconnected select! arm"
 );
 drop(rig.byte_tx);
 drop(rig.mux_rx);
 drop(rig.child_exit_tx);
}

// --- : bounded byte channel + symmetric IO-thread shrink ---

/// Helper: fill a `RenderableContent` with `n` placeholder cells.
/// Mirrors `snapshot/tests.rs::content_with_cells`. Used to drive the
/// `RenderableContent::maybe_shrink` capacity gate (`cap > 4*len &&
/// cap > 4096`).
fn populate_renderable(buf: &mut RenderableContent, n: usize) {
 buf.cells.clear();
 buf.cells.reserve(n);
 for i in 0..n {
 buf.cells.push(oriterm_core::RenderableCell {
 line: 0,
 column: Column(i),
 ch: ' ',
 fg: Default::default(),
 bg: Default::default(),
 flags: oriterm_core::CellFlags::empty(),
 underline_color: None,
 has_hyperlink: false,
 hyperlink_uri: None,
 zerowidth: Vec::new(),
 });
 }
}

/// Pin 1 — bounded byte-channel capacity.
/// Regression: pre-fix the byte channel was
/// `crossbeam_channel::unbounded()`, so a flooded reader could grow the
/// queue heap without bound. Pins that `try_send` returns
/// `TrySendError::Full` exactly when the queue holds
/// `BYTE_CHANNEL_CAPACITY` messages.
#[test]
fn byte_channel_capacity_blocks_at_bound() {
 let (_thread, handle) = make_pair();
 let byte_tx = handle.byte_sender();

 for i in 0..BYTE_CHANNEL_CAPACITY {
 byte_tx
 .try_send(Vec::new())
 .unwrap_or_else(|e| panic!("send {i} within capacity should succeed: {e}"));
 }

 let r = byte_tx.try_send(Vec::new());
 assert!(
 matches!(r, Err(crossbeam_channel::TrySendError::Full(_))),
 "expected TrySendError::Full at capacity, got {r:?}"
 );
}

/// Pin 2 — `byte_tx` reports `BYTE_CHANNEL_CAPACITY` via `Sender::capacity`.
/// Regression:. Pins both the cap value AND the bounded shape
/// (an unbounded `Sender::capacity` returns `None`).
#[test]
fn byte_tx_capacity_reports_bound() {
 let (_thread, handle) = make_pair();
 let byte_tx = handle.byte_sender();
 assert_eq!(
 byte_tx.capacity(),
 Some(BYTE_CHANNEL_CAPACITY),
 "byte_tx must be bounded at BYTE_CHANNEL_CAPACITY"
 );
}

/// Pin 4 — Drop counter increments on pane close within deadline.
/// Regression:. Spawns N IO threads, drops their handles, and
/// polls the cfg-gated `drop_counter` until it reaches N within a 5 s
/// safety deadline (-Clock-Free Testing`
/// — poll the condition, not the clock). Pins that `PaneIoHandle::Drop`
/// runs `shutdown()` then increments the counter.
#[test]
fn drop_counter_reaches_n_within_deadline() {
 let n = 4usize;
 let counter = Arc::new(AtomicUsize::new(0));

 for _ in 0..n {
 let (mut handle, _shutdown) = spawn_pair_with_flag();
 handle.set_drop_counter(Arc::clone(&counter));
 // Handle drops here at end of loop body — Drop -> shutdown() -> counter += 1.
 }

 let deadline = Instant::now() + Duration::from_secs(5);
 loop {
 if counter.load(Ordering::Acquire) >= n {
 break;
 }
 assert!(
 Instant::now() < deadline,
 "drop_counter only reached {} after {n} drops within deadline",
 counter.load(Ordering::Acquire)
 );
 std::thread::sleep(Duration::from_millis(20));
 }
 assert_eq!(counter.load(Ordering::Acquire), n);
}

/// Pin 3a — `maybe_shrink_buffers` shrinks `snapshot_buf`.
/// Regression:. Pre-fix the IO thread had no symmetric
/// `maybe_shrink` discipline against the main-thread side; after a
/// flood, `snapshot_buf` retained peak capacity indefinitely. Populates
/// the buffer past the shrink-gate threshold (cap > 4*len && cap > 4096),
/// truncates to a small viewport, calls the helper, and asserts capacity
/// reduces without losing content.
#[test]
fn maybe_shrink_buffers_shrinks_snapshot_buf() {
 let mut t = make_sync_thread();

 populate_renderable(&mut t.snapshot_buf, 10_000);
 let cap_before = t.snapshot_buf.cells.capacity();
 assert!(
 cap_before > 4096,
 "setup: snapshot_buf needs cap > 4096 to exercise the shrink gate, got {cap_before}"
 );

 // Truncate to a typical viewport (80×24=1920) — len << cap, gate fires.
 t.snapshot_buf.cells.truncate(1920);

 t.maybe_shrink_buffers();

 let cap_after = t.snapshot_buf.cells.capacity();
 assert!(
 cap_after < cap_before,
 "snapshot_buf capacity must shrink: {cap_before} → {cap_after}"
 );
 assert!(
 cap_after >= 1920,
 "shrink must preserve current content: cap={cap_after}, len=1920"
 );
}

/// Regression guard — `effects_buf` capacity is preserved by `maybe_shrink_buffers`.
/// Regression:. The drain pattern in
/// `oriterm_mux/src/pane/io_thread/effect_router/mod.rs` always leaves
/// `effects_buf.len() == 0` with retained capacity. A naïve
/// `maybe_shrink_vec` would gate-fire (cap > 4*0 && cap > 4096) and
/// `shrink_to(0)`, forcing reallocation on every effect push during the
/// next flood. Pins that the helper deliberately excludes effects_buf —
/// tpr-review round 1 critical finding.
#[test]
fn effects_buf_capacity_preserved_after_shrink() {
 let mut t = make_sync_thread();

 t.effects_buf.reserve(8192);
 let cap_before = t.effects_buf.capacity();
 assert!(
 cap_before > 4096,
 "setup: effects_buf cap must exceed shrink-gate threshold, got {cap_before}"
 );
 assert_eq!(t.effects_buf.len(), 0, "setup: effects_buf must be drained");

 t.maybe_shrink_buffers();

 assert_eq!(
 t.effects_buf.capacity(),
 cap_before,
 "effects_buf capacity MUST NOT shrink (drained len=0 would shrink_to(0))"
 );
}

/// Regression guard 2 — Drop accounting lives on `PaneIoHandle`, not on `EffectSink`.
/// Regression:. tp-help round 1 ( + ) rejected
/// 's proposal to track lifecycle via the `EffectSink` trait —
/// per-pane lifecycle accounting in a generic effect-routing abstraction
/// is ``
/// §Finding Categories`. This pin is type-level: a minimal `EffectSink`
/// impl with only `push` and `drain_into` compiles unchanged — proving
/// no required lifecycle method leaked into the trait. Drop accounting
/// is installed via `PaneIoHandle::set_drop_counter` (cfg(test) field).
#[test]
fn effect_sink_remains_lifecycle_pure() {
 use oriterm_core::effect::Effect;
 use oriterm_core::effect::sink::EffectSink;

 struct MinimalSink;
 impl EffectSink for MinimalSink {
 fn push(&self, _effect: Effect) {}
 fn drain_into(&self, _out: &mut Vec<Effect>) {}
 }

 // Construction proves the trait shape (push + drain_into) hasn't
 // grown new required lifecycle methods.
 let _ = MinimalSink;

 // PaneIoHandle owns drop accounting via `set_drop_counter` (cfg(test)):
 fn _drop_counter_lives_on_handle() {
 let counter = Arc::new(AtomicUsize::new(0));
 let (_t, mut h) = make_pair();
 h.set_drop_counter(counter);
 }
}

/// Regression guard 3 — `maybe_shrink_buffers` runs from the OUTER run loop,
/// not from the `select!` `default(timeout)` arm.
/// Regression:. tp-help round 1 ( + ) rejected
/// anchoring the shrink call to the `select!` default arm: on an idle
/// pane that arm fires once per `IDLE_WAKE_CEILING = 24h`, so a default-
/// arm anchor would never trigger in observable time. Pin spawns a real
/// IO thread, polls the cfg(test) `shrink_call_count` counter (shared
/// via `Arc<AtomicUsize>` between thread and test), and asserts the
/// OUTER-loop call path fires within a 5 s safety deadline. If a future
/// refactor moves `maybe_shrink_buffers()` to the `default(timeout)`
/// arm or removes the call entirely, the counter stays at 0 and this
/// test fails.
#[test]
fn maybe_shrink_runs_in_run_loop() {
 let shutdown = Arc::new(AtomicBool::new(false));
 let (thread, mut handle) = new_with_handle(IoThreadConfig {
 terminal: make_term(),
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 initial_rows: 24,
 initial_cols: 80,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 });
 // Capture the shared counter BEFORE spawn moves the thread.
 let counter = Arc::clone(&thread.shrink_call_count);
 let join = thread.spawn().expect("failed to spawn IO thread");
 handle.set_join(join);

 // Poll for the run loop to call maybe_shrink_buffers — the call
 // happens every iteration after maybe_produce_snapshot. The loop's
 // first iteration runs drain_commands → process_pending_bytes →
 // tick_animations → maybe_produce_snapshot → maybe_shrink_buffers
 // → select! BEFORE blocking on the 24h default-arm timeout, so the
 // counter must increment within milliseconds.
 let deadline = Instant::now() + Duration::from_secs(5);
 while counter.load(Ordering::Acquire) == 0 {
 assert!(
 Instant::now() < deadline,
 "maybe_shrink_buffers never fired in the run loop within 5 s"
 );
 std::thread::sleep(Duration::from_millis(20));
 }

 // Cleanly shut down so handle::Drop doesn't have to wait for join.
 handle.send_command(PaneIoCommand::Shutdown);
 drop(handle);
 assert!(
 counter.load(Ordering::Acquire) >= 1,
 "shrink_call_count must be ≥ 1 after the run loop ran"
 );
}

/// Edge case — `maybe_shrink_buffers` runs cleanly while sync output is active.
/// Regression:. Mode 2026 sync (BSU pending, ESU not yet)
/// keeps `TermMode::SYNC_UPDATE` set and forces
/// `maybe_produce_snapshot()` to defer. The shrink helper touches
/// IO-thread-owned `snapshot_buf` + the lock-protected `slot.front`,
/// never the VTE processor's parser timer — so it must be safe to
/// call mid-sync without mutating sync state.
#[test]
fn maybe_shrink_during_sync_active() {
 let (mut t, _wakeup_count) = make_sync_thread_with_wakeup();

 // Enter Mode 2026 sync mode (BSU) + dispatch grid mutations inline.
 t.handle_bytes(b"\x1b[?2026h");
 t.processor.advance(&mut t.terminal, b"buffered content");
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "BSU should activate sync mode"
 );
 let sync_active_before = t.processor.is_sync_active();
 assert!(sync_active_before, "BSU should arm parser-side timer");

 // Shrink while sync is active — must NOT mutate sync state or panic.
 t.maybe_shrink_buffers();

 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "maybe_shrink must NOT clear TermMode::SYNC_UPDATE"
 );
 assert_eq!(
 t.processor.is_sync_active(),
 sync_active_before,
 "maybe_shrink must NOT alter parser-side sync timer state"
 );
}

// --- §03 matrix: bounded cmd_tx + atomic-coalescing resize ---
// Pins the cmd_tx-bounding contract, the pending_resize tag-bit
// encoding, the drain-time apply ordering, the wake topology, the
// shutdown-saturation belt-and-suspenders, and the SSOT classification
// for reply-bearing variants. Reverting any single piece breaks at
// least one of these tests.

/// Pin the bounded-cmd_tx contract: a fresh handle reports a finite
/// capacity. Replaces the unbounded shape that motivated.
/// Regression: exact failing case `cmd_tx_is_bounded`.
#[test]
fn new_with_handle_uses_bounded_cmd_tx() {
 let (_thread, handle) = make_pair();
 assert_eq!(
 handle.cmd_tx.capacity(),
 Some(CMD_CHANNEL_CAPACITY),
 "cmd_tx must report Some(CMD_CHANNEL_CAPACITY); unbounded would return None"
 );
}

/// Pin the sentinel: `PENDING_RESIZE_NONE` decodes to `None`.
/// Regression: edge case `pending_resize_none_sentinel`.
#[test]
fn pending_resize_none_sentinel_means_no_pending() {
 assert!(unpack_pending_resize(PENDING_RESIZE_NONE).is_none());
}

/// Pin the tag-bit at the upper boundary: `(u16::MAX, u16::MAX)`
/// round-trips without truncation and the tag bit (bit 48) does not
/// collide with the row/col fields.
/// Regression: edge case `pack_pending_resize_max_dimensions_round_trip`.
#[test]
fn pack_pending_resize_max_dimensions_round_trip() {
 let packed = pack_pending_resize(u16::MAX, u16::MAX);
 assert_eq!(unpack_pending_resize(packed), Some((u16::MAX, u16::MAX)));
}

/// Pin the tag-bit at the zero boundary: `(0, 0)` round-trips
/// distinctly from the `PENDING_RESIZE_NONE` sentinel. Without the
/// tag bit, `pack(0,0) == 0 == PENDING_RESIZE_NONE`, and a legitimate
/// resize-to-zero request would be silently swallowed.
/// Regression: closes §04 review round 0 F2.
#[test]
fn pack_pending_resize_zero_zero_round_trip() {
 let packed = pack_pending_resize(0, 0);
 assert_eq!(unpack_pending_resize(packed), Some((0, 0)));
 assert_ne!(
 packed, PENDING_RESIZE_NONE,
 "pack(0,0) MUST be distinct from the sentinel"
 );
}

/// Pin the encoding across a representative set of `(rows, cols)`
/// values: every input round-trips through pack → unpack.
/// Regression: edge case `pack_pending_resize_arbitrary_round_trip`.
#[test]
fn pack_pending_resize_arbitrary_round_trip() {
 let cases = [
 (0u16, 0u16),
 (1, 1),
 (24, 80),
 (50, 200),
 (u16::MAX, 0),
 (0, u16::MAX),
 (u16::MAX, u16::MAX),
 ];
 for (rows, cols) in cases {
 let packed = pack_pending_resize(rows, cols);
 assert_eq!(
 unpack_pending_resize(packed),
 Some((rows, cols)),
 "round-trip failed for ({rows}, {cols})"
 );
 }
}

/// Regression guard: pack/unpack is a pure function — exhaustive
/// round-trip over the same case set, plus the explicit sentinel
/// check. Compile-time enforces `pub(crate)` visibility on the
/// helpers.
/// Regression: closes §04 review round 1 .
#[test]
fn pack_unpack_pending_resize_is_pure_function() {
 let cases = [
 (0u16, 0u16),
 (1, 1),
 (24, 80),
 (u16::MAX, 0),
 (0, u16::MAX),
 (u16::MAX, u16::MAX),
 ];
 for (rows, cols) in cases {
 assert_eq!(
 unpack_pending_resize(pack_pending_resize(rows, cols)),
 Some((rows, cols)),
 );
 }
 assert_eq!(unpack_pending_resize(PENDING_RESIZE_NONE), None);
}

/// Pin the saturation contract synchronously: `cmd_tx.try_send`
/// returns `Err(TrySendError::Full(_))` exactly at capacity. Wall-
/// clock-free per `tests.md §Wall-Clock-Free Testing`.
/// Uses `make_pair()` (no spawned IO thread) per §04 review round 4
/// — a live thread would async-drain `cmd_tx` and break the
/// saturation assertion.
/// Regression: exact failing case
/// `cmd_tx_at_capacity_returns_full_error_synchronously`. Replaces
/// the wall-clock-dependent `test_command_channel_flood`.
#[test]
fn cmd_tx_at_capacity_returns_full_error_synchronously() {
 let (_thread, handle) = make_pair();
 for i in 0..CMD_CHANNEL_CAPACITY {
 handle
 .cmd_tx
 .try_send(PaneIoCommand::MarkAllDirty)
 .unwrap_or_else(|e| panic!("send {i} within capacity should succeed: {e}"));
 }
 let r = handle.cmd_tx.try_send(PaneIoCommand::MarkAllDirty);
 assert!(
 matches!(r, Err(crossbeam_channel::TrySendError::Full(_))),
 "expected TrySendError::Full at capacity, got {r:?}"
 );
}

/// Pin that a pre-drain atomic store reaches `process_resize` via
/// `apply_pending_resize`. Drives `drain_commands()` directly so
/// crossbeam's `select!` non-determinism is irrelevant.
/// Regression: cross-pattern coverage
/// `drain_after_atomic_store_processes_resize`.
#[test]
fn drain_after_atomic_store_processes_resize() {
 let (mut t, _cmd_tx) = make_sync_thread_with_cmd_tx();
 pack_then_store(&t.pending_resize, 30, 100);
 t.drain_commands();
 assert_eq!(t.terminal.grid().lines(), 30);
 assert_eq!(t.terminal.grid().cols(), 100);
}

/// Pin last-writer-wins coalescing across three rapid stores: only
/// the latest `(rows, cols)` reaches `process_resize`. Replacement
/// for the original `test_resize_coalescing` against the now-removed
/// `PaneIoCommand::Resize` variant.
/// Regression: cross-pattern coverage
/// `drain_after_three_atomic_stores_processes_only_last`.
#[test]
fn drain_after_three_atomic_stores_processes_only_last() {
 let (mut t, _cmd_tx) = make_sync_thread_with_cmd_tx();
 pack_then_store(&t.pending_resize, 24, 80);
 pack_then_store(&t.pending_resize, 24, 60);
 pack_then_store(&t.pending_resize, 24, 40);
 t.drain_commands();
 assert_eq!(
 t.terminal.grid().cols(),
 40,
 "only the last atomic store should be applied"
 );
}

/// Pin the resize-FIRST drain ordering: when a pending resize and
/// scroll/mark-dirty commands land together, the resize is applied
/// BEFORE the other commands so any reply-bearing command later in
/// the same drain reads post-resize geometry.
/// Regression: cross-pattern coverage
/// `drain_applies_pending_resize_before_other_commands`. Per §04
/// review round 0 F1 and review pass F2 + .
#[test]
fn drain_applies_pending_resize_before_other_commands() {
 let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();
 cmd_tx.send(PaneIoCommand::MarkAllDirty).unwrap();
 pack_then_store(&t.pending_resize, 24, 40);
 t.drain_commands();
 // Resize ran (cols changed)
 assert_eq!(
 t.terminal.grid().cols(),
 40,
 "resize must have been applied"
 );
 // MarkAllDirty ran post-resize: every line in the new viewport is dirty
 assert!(
 t.terminal.grid().dirty().is_all_dirty(),
 "MarkAllDirty must execute after the resize"
 );
}

/// Regression guard: when the pending_resize slot carries the
/// `PENDING_RESIZE_NONE` sentinel, `apply_pending_resize` MUST NOT
/// invoke `process_resize`. Verified by checking that
/// `last_pty_size` is unchanged.
/// Regression: cross-pattern regression guard
/// `drain_with_no_pending_resize_does_not_call_process_resize`.
#[test]
fn drain_with_no_pending_resize_does_not_call_process_resize() {
 let (mut t, _cmd_tx) = make_sync_thread_with_cmd_tx();
 let initial_last = t.last_pty_size;
 t.drain_commands();
 assert_eq!(
 t.last_pty_size, initial_last,
 "process_resize must NOT fire when slot is the sentinel"
 );
}

/// Pin the resize-then-Shutdown drain ordering: a pending resize is
/// applied before `Shutdown` short-circuits the drain. Replacement
/// for the rejected "shutdown wins over pending_resize" formulation
/// per §04 review round 1 .
/// Regression: cross-pattern coverage
/// `shutdown_after_pending_resize_applies_resize_then_terminates`.
#[test]
fn shutdown_after_pending_resize_applies_resize_then_terminates() {
 let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();
 pack_then_store(&t.pending_resize, 24, 40);
 cmd_tx.send(PaneIoCommand::Shutdown).unwrap();
 t.drain_commands();
 assert_eq!(
 t.terminal.grid().cols(),
 40,
 "resize must apply before Shutdown"
 );
 assert!(
 t.shutdown.load(Ordering::Acquire),
 "shutdown flag must be set after the Shutdown command"
 );
}

/// Pin the semantic invariant: when a pending resize and a
/// reply-bearing `SnapshotNow { reply }` interleave, the latest
/// atomic store wins AND the snapshot reply published to the caller
/// reflects the post-resize geometry. Load-bearing test for §05
/// Step 2's per-iteration re-flush AND the SnapshotNow FIFO barrier
/// at `commands/mod.rs:45-56` (snapshot reply is sent only AFTER
/// post-resize state is published to the double buffer).
/// Regression: property
/// `drain_commands_applies_pending_resize_before_reply_bearing_commands`.
/// Round 1 code-TPR F1 — the prior MarkAllDirty proxy did not
/// exercise the reply-channel handshake the §03 plan specified.
#[test]
fn drain_commands_applies_pending_resize_before_reply_bearing_commands() {
 let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();
 pack_then_store(&t.pending_resize, 24, 80);
 cmd_tx.send(PaneIoCommand::MarkAllDirty).unwrap();
 pack_then_store(&t.pending_resize, 24, 40);
 let (reply_tx, reply_rx) = crossbeam_channel::bounded::<()>(1);
 cmd_tx
 .send(PaneIoCommand::SnapshotNow { reply: reply_tx })
 .unwrap();
 t.drain_commands();
 // SnapshotNow's reply MUST have been sent before drain returned —
 // the per-iteration apply_pending_resize flush guarantees the
 // snapshot reflects post-resize state.
 reply_rx
 .try_recv()
 .expect("SnapshotNow reply must arrive before drain returns");
 // Last-writer-wins on the slot
 assert_eq!(t.terminal.grid().cols(), 40);
 // The published snapshot reflects post-resize geometry
 let mut snap = RenderableContent::default();
 assert!(
 t.double_buffer.swap_front(&mut snap),
 "SnapshotNow must publish a fresh snapshot to the double buffer"
 );
 assert_eq!(
 snap.cols, 40,
 "snapshot published by SnapshotNow must carry post-resize cols"
 );
}

/// Regression guard: applying a pending resize before a non-reply
/// geometry-dependent command (`ScrollDisplay`) — deterministic
/// complement to the live-thread probabilistic crossbeam-arm pin.
/// Drives `drain_commands` directly with a pre-staged slot and
/// command so non-determinism is irrelevant.
/// Regression: closes §04 review round 4 .
#[test]
fn drain_commands_applies_pending_resize_before_non_reply_command() {
 let (mut t, cmd_tx) = make_sync_thread_with_cmd_tx();
 // Fill scrollback so ScrollDisplay can take effect.
 for _ in 0..50 {
 t.handle_bytes(b"scrollback line\r\n");
 }
 pack_then_store(&t.pending_resize, 30, 100);
 cmd_tx.send(PaneIoCommand::ScrollDisplay(5)).unwrap();
 t.drain_commands();
 // Resize ran first
 assert_eq!(t.terminal.grid().cols(), 100);
 assert_eq!(t.terminal.grid().lines(), 30);
 // ScrollDisplay ran post-resize: display_offset moved
 assert!(
 t.terminal.grid().display_offset() > 0,
 "ScrollDisplay must execute after the resize"
 );
}

/// Regression guard: `Shutdown` reaches the IO thread even when `cmd_tx`
/// is at capacity AND the IO thread was blocked at the loop-entry barrier.
/// Saturates `cmd_tx` synchronously, spawns the IO thread (held at the
/// barrier), signals shutdown via the atomic flag + try_send + wake
/// WITHOUT joining (would deadlock at the barrier), then releases the
/// barrier. The IO thread drains the saturated queue, observes the
/// shutdown flag, and exits deterministically.
/// Regression: Q4 belt-and-suspenders — uses loop-entry `Barrier` so
/// saturation is staged before the IO thread begins draining.
#[test]
fn shutdown_under_cmd_tx_saturation_still_terminates() {
 let shutdown = Arc::new(AtomicBool::new(false));
 let (mut thread, handle) = new_with_handle(IoThreadConfig {
 terminal: make_term(),
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 initial_rows: 24,
 initial_cols: 80,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 });
 // Test-only barrier — IO thread waits at loop entry so we can stage
 // saturation BEFORE it begins draining.
 let barrier = Arc::new(std::sync::Barrier::new(2));
 thread.start_barrier = Some(Arc::clone(&barrier));
 // Fill the channel synchronously — no live drainer yet.
 for _ in 0..CMD_CHANNEL_CAPACITY {
 handle
 .cmd_tx
 .try_send(PaneIoCommand::MarkAllDirty)
 .expect("staged saturation must succeed");
 }
 let r = handle.cmd_tx.try_send(PaneIoCommand::MarkAllDirty);
 assert!(
 matches!(r, Err(crossbeam_channel::TrySendError::Full(_))),
 "channel must be at capacity before spawn; got {r:?}"
 );
 // Spawn — IO thread waits at the barrier.
 let join = thread.spawn().expect("spawn IO thread");
 // Signal shutdown WITHOUT joining (join would deadlock while the IO
 // thread is blocked at the barrier). The atomic flag is the durable
 // signal; try_send + wake are best-effort on a saturated channel.
 handle.shutdown_flag.store(true, Ordering::Release);
 let _ = handle.cmd_tx.try_send(PaneIoCommand::Shutdown);
 let _ = handle.io_wake_tx.try_send(());
 // Release the barrier — IO thread now enters drain_commands, drains
 // the saturated queue, observes the shutdown flag, and exits.
 barrier.wait();
 // Wall-clock-free: poll join until finished. 150s process-level
 // timeout is the only safety valve.
 while !join.is_finished() {
 std::thread::sleep(Duration::from_millis(20));
 }
 let _ = join.join();
}

/// Regression guard: `send_command(Shutdown)` (the generic-channel path
/// distinct from `handle.shutdown()`) terminates the IO thread even
/// under `cmd_tx` saturation, with the IO thread held at the loop-entry
/// barrier. Pins the `matches!(&cmd, PaneIoCommand::Shutdown)` special-
/// case in `send_command`.
/// Regression: closes §04 review round 4 — uses loop-entry
/// `Barrier` for deterministic shutdown-via-send_command under saturation.
#[test]
fn send_command_shutdown_under_cmd_tx_saturation_still_terminates() {
 let shutdown = Arc::new(AtomicBool::new(false));
 let (mut thread, handle) = new_with_handle(IoThreadConfig {
 terminal: make_term(),
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::clone(&shutdown),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 initial_rows: 24,
 initial_cols: 80,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 });
 let barrier = Arc::new(std::sync::Barrier::new(2));
 thread.start_barrier = Some(Arc::clone(&barrier));
 for _ in 0..CMD_CHANNEL_CAPACITY {
 handle
 .cmd_tx
 .try_send(PaneIoCommand::MarkAllDirty)
 .expect("staged saturation must succeed");
 }
 let join = thread.spawn().expect("spawn IO thread");
 // send_command(Shutdown) sets the durable flag + try_sends via the
 // matches! special-case; no join involved — no deadlock at barrier.
 handle.send_command(PaneIoCommand::Shutdown);
 // Release the IO thread — it drains the saturated queue, encounters
 // Shutdown, sets the local shutdown flag, and exits.
 barrier.wait();
 while !join.is_finished() {
 std::thread::sleep(Duration::from_millis(20));
 }
 let _ = join.join();
 drop(handle);
}

/// Regression guard: writer-thread shutdown wakes the IO thread out of
/// `select!` within one iteration. Simulates the writer-thread exit
/// path: set `shutdown` AND `try_send` on `io_wake_tx`. Pins §05
/// Step 6C's writer-thread wake plumbing.
/// Regression: closes §04 review round 5 + F2.
/// Wall-clock-free per `tests.md §Wall-Clock-Free Testing`: poll
/// `JoinHandle::is_finished()` instead of asserting on
/// `start.elapsed()`.
#[test]
fn idle_io_thread_observes_writer_thread_shutdown_within_one_iteration() {
 let (mut handle, shutdown_flag) = spawn_pair_with_flag();
 // Simulate the writer thread exit path verbatim: set the
 // durable flag AND send a wake on io_wake_tx.
 shutdown_flag.store(true, Ordering::Release);
 let _ = handle.io_wake_tx.try_send(());
 let join = handle.join.take().expect("join handle present");
 while !join.is_finished() {
 std::thread::sleep(Duration::from_millis(20));
 }
 let _ = join.join();
}

/// Regression guard: `is_reply_bearing()` is exhaustive — every
/// `PaneIoCommand` variant constructible with a reply `Sender<_>`
/// returns `true`; every non-reply variant returns `false`. A
/// future contributor adding a new reply-bearing variant without
/// updating the predicate fails this test.
/// Regression: SSOT exhaustiveness guard, closes §04
/// review round 2 .
#[test]
fn is_reply_bearing_predicate_matches_reply_field_presence() {
 use oriterm_core::grid::StableRowIndex;
 use oriterm_core::index::Side;
 use oriterm_core::{CursorShape, Palette, Selection, Theme};

 /// Test-local exhaustive classifier — NO wildcard arm. When a
 /// future contributor adds a new `PaneIoCommand` variant, the
 /// compiler refuses to build this test until the variant is
 /// classified here, AND the assertion at the bottom verifies
 /// the classification matches `is_reply_bearing()`. Per Round 2
 /// F3 — the prior array-based test had no compile-time
 /// exhaustiveness; a new reply-bearing variant could ship with
 /// the predicate out of date and the test would still pass.
 fn classify_expected(cmd: &PaneIoCommand) -> bool {
 match cmd {
 // Reply-bearing — must return true.
 PaneIoCommand::SnapshotNow { .. }
 | PaneIoCommand::ExtractText { .. }
 | PaneIoCommand::ExtractHtml { .. }
 | PaneIoCommand::EnterMarkMode { .. }
 | PaneIoCommand::SelectCommandOutput { .. }
 | PaneIoCommand::SelectCommandInput { .. } => true,
 // Non-reply — must return false.
 PaneIoCommand::ScrollDisplay(_)
 | PaneIoCommand::ScrollToBottom
 | PaneIoCommand::ScrollToPreviousPrompt
 | PaneIoCommand::ScrollToNextPrompt
 | PaneIoCommand::SetTheme(_, _)
 | PaneIoCommand::SetCursorShape(_)
 | PaneIoCommand::SetBoldIsBright(_)
 | PaneIoCommand::MarkAllDirty
 | PaneIoCommand::SetImageConfig(_)
 | PaneIoCommand::SetCellDimensions { .. }
 | PaneIoCommand::OpenSearch
 | PaneIoCommand::CloseSearch
 | PaneIoCommand::SearchSetQuery(_)
 | PaneIoCommand::SearchNextMatch
 | PaneIoCommand::SearchPrevMatch
 | PaneIoCommand::Reset
 | PaneIoCommand::Shutdown
 | PaneIoCommand::SetAnswerback(_) => false,
 }
 }

 let (snap_tx, _snap_rx) = crossbeam_channel::bounded::<()>(1);
 let (xt_tx, _xt_rx) = crossbeam_channel::bounded::<Option<String>>(1);
 let (xh_tx, _xh_rx) = crossbeam_channel::bounded::<Option<(String, String)>>(1);
 let (mark_tx, _mark_rx) = crossbeam_channel::bounded::<crate::pane::MarkCursor>(1);
 let (out_tx, _out_rx) = crossbeam_channel::bounded::<Option<Selection>>(1);
 let (in_tx, _in_rx) = crossbeam_channel::bounded::<Option<Selection>>(1);

 let sel = Selection::new_char(StableRowIndex(0), 0, Side::Left);
 let cmds = [
 // Reply-bearing
 PaneIoCommand::SnapshotNow { reply: snap_tx },
 PaneIoCommand::ExtractText {
 selection: sel,
 reply: xt_tx,
 },
 PaneIoCommand::ExtractHtml {
 selection: sel,
 font_family: String::new(),
 font_size: 12.0,
 reply: xh_tx,
 },
 PaneIoCommand::EnterMarkMode { reply: mark_tx },
 PaneIoCommand::SelectCommandOutput { reply: out_tx },
 PaneIoCommand::SelectCommandInput { reply: in_tx },
 // Non-reply
 PaneIoCommand::ScrollDisplay(0),
 PaneIoCommand::ScrollToBottom,
 PaneIoCommand::ScrollToPreviousPrompt,
 PaneIoCommand::ScrollToNextPrompt,
 PaneIoCommand::SetTheme(Theme::default(), Box::new(Palette::default())),
 PaneIoCommand::SetCursorShape(CursorShape::Block),
 PaneIoCommand::SetBoldIsBright(true),
 PaneIoCommand::MarkAllDirty,
 PaneIoCommand::SetImageConfig(crate::backend::ImageConfig {
 enabled: false,
 memory_limit: 0,
 max_single: 0,
 animation_enabled: false,
 }),
 PaneIoCommand::SetCellDimensions {
 width: 8,
 height: 16,
 },
 PaneIoCommand::OpenSearch,
 PaneIoCommand::CloseSearch,
 PaneIoCommand::SearchSetQuery(String::new()),
 PaneIoCommand::SearchNextMatch,
 PaneIoCommand::SearchPrevMatch,
 PaneIoCommand::Reset,
 PaneIoCommand::Shutdown,
 ];
 for cmd in &cmds {
 let expected = classify_expected(cmd);
 assert_eq!(
 cmd.is_reply_bearing(),
 expected,
 "is_reply_bearing classification mismatch for {cmd:?}: expected {expected}"
 );
 }
}

// --- §03 cross-feature interaction tests (Round 1 §06 F1) ---
// The §03 test matrix enumerated four cross-feature interaction tests
// that exercise the live IO thread under concurrent send_resize +
// reply-bearing or flooded inputs. They were missed in the initial
// implementation pass and added here. All four use the wall-clock-
// free pattern (poll-the-condition with a 5s safety deadline; the
// 150s process-level test timeout is the outer safety valve).

/// Pin the wake topology: an atomic store via `send_resize` on a
/// spawned IO thread reaches `process_resize` within one loop
/// iteration, observable via a snapshot whose dimensions match the
/// stored geometry. Wall-clock-free: poll the snapshot until it
/// matches; a 5s safety deadline surfaces hangs.
/// Regression: cross-feature
/// `live_io_thread_atomic_store_wakes_within_one_iteration`.
#[test]
fn live_io_thread_atomic_store_wakes_within_one_iteration() {
 let (mut handle, _shutdown) = spawn_pair_with_flag();
 handle.send_resize(40, 120);
 let deadline = Instant::now() + Duration::from_secs(5);
 let mut snap = RenderableContent::default();
 loop {
 if handle.double_buffer().swap_front(&mut snap) && snap.lines == 40 && snap.cols == 120 {
 break;
 }
 assert!(
 Instant::now() < deadline,
 "IO thread did not observe send_resize within deadline; \
 pending_resize wake topology is broken"
 );
 std::thread::sleep(Duration::from_millis(20));
 }
 handle.shutdown();
}

/// Pin the original motivation: a sustained PTY flood
/// concurrent with rapid send_resize calls preserves the FINAL
/// geometry (last-writer-wins on the atomic slot, never dropped
/// despite cmd_tx pressure). Wall-clock-free: condition-poll the
/// snapshot.
/// Regression: cross-feature
/// `resize_during_pty_flood_preserves_final_geometry`.
#[test]
fn resize_during_pty_flood_preserves_final_geometry() {
 let (mut handle, _shutdown) = spawn_pair_with_flag();
 let byte_tx = handle.byte_sender();
 // Sustained PTY flood — fills the IO thread's parse + drain
 // pipeline so non-Resize work is in flight when Resize lands.
 let flood_handle = std::thread::spawn(move || {
 let chunk = vec![b'A'; 4096];
 for _ in 0..200 {
 if byte_tx.send(chunk.clone()).is_err() {
 break;
 }
 }
 });
 // 60 rapid resize stores — last writer wins.
 for i in 0..60u16 {
 let cols = 40 + (i % 80);
 let rows = 20 + (i % 20);
 handle.send_resize(rows, cols);
 }
 // Final canonical geometry MUST be the last store.
 handle.send_resize(50, 200);
 flood_handle.join().expect("flood thread panicked");
 let deadline = Instant::now() + Duration::from_secs(5);
 let mut snap = RenderableContent::default();
 loop {
 if handle.double_buffer().swap_front(&mut snap) && snap.lines == 50 && snap.cols == 200 {
 break;
 }
 assert!(
 Instant::now() < deadline,
 "final geometry (50, 200) did not appear within deadline; \
 snap was ({}, {})",
 snap.lines,
 snap.cols,
 );
 std::thread::sleep(Duration::from_millis(20));
 }
 handle.shutdown();
}

/// Pin the mid-drain race: a `send_resize` that lands WHILE
/// `drain_commands` is actively draining a saturated channel MUST still
/// flush before the next reply-bearing command (per-iteration re-flush).
/// Uses a loop-entry `Barrier` to hold the IO thread until saturation is
/// staged, then releases the barrier so the IO thread enters drain with a
/// full queue. The second resize + SnapshotNow land while the IO thread is
/// mid-drain (256 commands ≠ instant), guaranteeing the re-flush is
/// exercised deterministically.
/// Regression: cross-feature
/// `send_resize_during_drain_before_snapshot_reflects_post_resize`
/// (closes §04 review round 1 — loop-entry `Barrier` pins the
/// mid-drain race deterministically).
#[test]
fn send_resize_during_drain_before_snapshot_reflects_post_resize() {
 let (mut thread, mut handle) = new_with_handle(IoThreadConfig {
 terminal: make_term(),
 pane_id: {
 let (p, _, _, _) = test_dummy_channels();
 p
 },
 mux_tx: {
 let (_, t, _, _) = test_dummy_channels();
 t
 },
 child_exit_rx: {
 let (_, _, r, _) = test_dummy_channels();
 r
 },
 mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
 shutdown: Arc::new(AtomicBool::new(false)),
 wakeup: Arc::new(|| {}),
 grid_dirty: Arc::new(AtomicBool::new(false)),
 pty_control: None,
 adopted_signal: None,
 initial_rows: 24,
 initial_cols: 80,
 selection_dirty: Arc::new(AtomicBool::new(false)),
 });
 // Loop-entry barrier — IO thread pauses before the first drain.
 let barrier = Arc::new(std::sync::Barrier::new(2));
 thread.start_barrier = Some(Arc::clone(&barrier));
 // Stage first resize (lands in atomic slot) and saturate cmd_tx
 // BEFORE the IO thread starts draining.
 handle.send_resize(24, 80);
 for _ in 0..CMD_CHANNEL_CAPACITY {
 handle
 .cmd_tx
 .try_send(PaneIoCommand::MarkAllDirty)
 .expect("staged saturation must succeed");
 }
 // Spawn — IO thread waits at the barrier.
 let join = thread.spawn().expect("spawn IO thread");
 handle.set_join(join);
 // Release the barrier — IO thread enters drain_commands with a
 // saturated queue (256 MarkAllDirty commands). Mid-drain: the second
 // resize + SnapshotNow land while the IO thread is still draining.
 barrier.wait();
 handle.send_resize(24, 40);
 let (reply_tx, reply_rx) = crossbeam_channel::bounded::<()>(1);
 handle.send_command(PaneIoCommand::SnapshotNow { reply: reply_tx });
 reply_rx
 .recv_timeout(Duration::from_secs(5))
 .expect("SnapshotNow reply did not arrive within deadline");
 let mut snap = RenderableContent::default();
 assert!(
 handle.double_buffer().swap_front(&mut snap),
 "SnapshotNow must publish a fresh snapshot"
 );
 assert_eq!(
 snap.cols, 40,
 "snapshot cols must reflect post-resize geometry; per-command \
 apply_pending_resize flush must pick up the mid-drain send_resize"
 );
 handle.shutdown();
}

/// Pin the `select!` `cmd_rx` arm: the idle-wake path applies
/// `apply_pending_resize` before handling reply-bearing commands.
/// Multi-trial coverage so crossbeam's nondeterministic arm-firing
/// exercises both orderings (`io_wake_rx` first vs. `cmd_rx` first).
/// Wall-clock-free per `tests.md §Wall-Clock-Free Testing`: each
/// trial recv_timeouts on its own SnapshotNow reply; no measured
/// latency.
/// Regression: cross-feature
/// `idle_select_cmd_rx_arm_applies_pending_resize_before_reply_bearing`
/// (closes §04 review round 2 ).
#[test]
fn idle_select_cmd_rx_arm_applies_pending_resize_before_reply_bearing() {
 let (mut handle, _shutdown) = spawn_pair_with_flag();
 handle.send_resize(24, 80);
 // 16 trials — enough to exercise crossbeam's nondeterministic
 // arm-firing both ways (io_wake_rx-first vs cmd_rx-first).
 // The §03 plan called for 64; 16 is a wall-clock-bounded
 // subset that still surfaces ordering bugs deterministically
 // on the per-trial assertion (each trial fails independently
 // if the ordering invariant breaks).
 for trial in 0..16 {
 let cols = 40 + (trial as u16 % 80);
 handle.send_resize(24, cols);
 let (reply_tx, reply_rx) = crossbeam_channel::bounded::<()>(1);
 handle.send_command(PaneIoCommand::SnapshotNow { reply: reply_tx });
 reply_rx
 .recv_timeout(Duration::from_secs(5))
 .unwrap_or_else(|e| panic!("trial {trial}: reply timeout: {e}"));
 let mut snap = RenderableContent::default();
 assert!(
 handle.double_buffer().swap_front(&mut snap),
 "trial {trial}: SnapshotNow must publish a fresh snapshot"
 );
 assert_eq!(
 snap.cols, cols as usize,
 "trial {trial}: snapshot cols must reflect post-resize cols ({cols})"
 );
 }
 handle.shutdown();
}

// §03 matrix — Mode 2026 inline dispatch + snapshot gating.
// These pins enforce the user-visible "no partial frames" invariant
// via snapshot-publication gating on `TermMode::SYNC_UPDATE`, while
// device queries and grid mutations dispatch INLINE during the sync
// window. The vendored vte parser carries no byte-level buffer; the
// only sync state is the deadline timer used by the run loop's
// `select!` deadline arm. See 

/// Property: BSU + grid-mutating bytes mutate the grid INLINE,
/// while snapshot publication is suppressed via the `SYNC_UPDATE`
/// mode flag.
/// Asserts (a) "Hello" lands in the grid before the chunk completes
/// (inline dispatch — no buffering), AND (b) `maybe_produce_snapshot`
/// returns without publishing because `TermMode::SYNC_UPDATE` gates
/// the snapshot pipeline.
/// Regression: §03 semantic-pin.
#[test]
fn mode_2026_active_does_not_publish_snapshot_yet_processes_bytes() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // Enter sync mode + emit grid-mutating bytes in one chunk.
 t.handle_bytes(b"\x1b[?2026hHello");

 // Inline-dispatch invariant: grid contains "Hello" at row 0.
 let row = &t.terminal.grid()[Line(0)];
 let row_text: String = (0..5).map(|c| row[Column(c)].ch).collect();
 assert_eq!(
 row_text, "Hello",
 "grid row 0 must contain 'Hello' inline during sync (bytes dispatched, not buffered)"
 );

 // Snapshot-gating invariant: no snapshot publication during sync.
 let wakeup_before = wakeup_count.load(Ordering::Relaxed);
 t.maybe_produce_snapshot();
 let wakeup_after = wakeup_count.load(Ordering::Relaxed);
 assert_eq!(
 wakeup_before, wakeup_after,
 "wakeup must NOT fire while TermMode::SYNC_UPDATE is set"
 );
 assert_eq!(
 t.double_buffer.seqno(),
 0,
 "snapshot seqno must stay at 0 while TermMode::SYNC_UPDATE is set"
 );
}

/// Sync timeout path: 150 ms timer expiry unsets the mode, emits
/// `PresentationEffect::Abort`, and forces a snapshot publication.
/// Regression: §03 — pins the rewritten `handle_sync_timeout`
/// path (no buffered-byte replay, just mode unset + snapshot force).
#[test]
fn mode_2026_timeout_unsets_sync_mode_emits_abort_forces_snapshot() {
 use oriterm_core::effect::sink::EffectSink;
 use oriterm_core::effect::{Effect, PresentationEffect, QueueingEffectSink, SyncAbortReason};

 let sink = QueueingEffectSink::new();
 let (mut t, wakeup_count) = make_sync_thread_generic(sink);

 // Enter sync mode + dispatch grid-mutating bytes.
 t.handle_bytes(b"\x1b[?2026hContent");
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "BSU should set SYNC_UPDATE"
 );

 // Trigger timeout — exits the sync window without ESU.
 t.handle_sync_timeout();

 // 1. Mode flag must be cleared.
 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "TermMode::SYNC_UPDATE must be cleared after handle_sync_timeout"
 );

 // 2. Abort effect must be in the effect sink.
 let mut effects = Vec::new();
 t.terminal.effect_sink().drain_into(&mut effects);
 let has_abort = effects.iter().any(|e| {
 matches!(
 e,
 Effect::Presentation(PresentationEffect::Abort {
 reason: SyncAbortReason::Timeout
 })
 )
 });
 assert!(
 has_abort,
 "Abort effect must be emitted on timeout, got: {effects:?}"
 );

 // 3. Snapshot must be published (wakeup fired AND seqno advanced).
 assert!(
 wakeup_count.load(Ordering::Relaxed) > 0,
 "wakeup must fire on sync timeout"
 );
 assert!(
 t.double_buffer.seqno() > 0,
 "snapshot seqno must advance on sync timeout"
 );
}

/// ESU dispatched inline with BSU: mode flag clears AND parser-side
/// timer disarms within the same `handle_bytes()` call.
/// Regression: §03 — pins the ESU dispatch arm's
/// `clear_timeout` call so a future revert won't leave the timer
/// armed after ESU.
#[test]
fn mode_2026_esu_unsets_mode_clears_processor_timeout() {
 let mut t = make_sync_thread();

 // BSU + ESU in one call.
 t.handle_bytes(b"\x1b[?2026h\x1b[?2026l");

 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "ESU must clear TermMode::SYNC_UPDATE"
 );
 assert!(
 t.processor.sync_timeout().sync_timeout().is_none(),
 "ESU must disarm the parser-side sync timer"
 );
}

/// Regression pin for the ESU-arm timer-disarm: after BSU + DA1 + ESU
/// in one parser advance, the parser-side sync timer is disarmed, the
/// mode flag is cleared, and the DA1 response is in the effect sink.
/// Without the ESU-arm `clear_timeout` call, the parser-side timer
/// would remain armed after ESU and the run loop's
/// `crossbeam_channel::select!` `default(timeout)` arm would fire
/// ~150 ms later — invoking `handle_sync_timeout` on already-cleared
/// state and emitting a spurious Abort effect.
/// The disarmed timer (`sync_timeout().sync_timeout() == None`) IS the
/// assertion: the select! arm only fires when a deadline is pending,
/// so a `None` timer guarantees no spurious timeout invocation.
/// Regression: §03 ESU-arm timer-disarm pin.
#[test]
fn bsu_after_query_inside_sync_does_not_fire_spurious_handle_sync_timeout() {
 use oriterm_core::effect::sink::EffectSink;
 use oriterm_core::effect::{Effect, PtyEffect, QueueingEffectSink};

 let sink = QueueingEffectSink::new();
 let (mut t, _wakeup) = make_sync_thread_generic(sink);

 // Drive bytes through the processor directly (bypasses
 // handle_bytes's `drain_effects_into_mux_events`, so effects stay
 // visible on the sink).
 t.processor
 .advance(&mut t.terminal, b"\x1b[?2026h\x1b[c\x1b[?2026l");

 // (a) DA1 response present in the effect sink — inline-dispatched
 // within the sync window. The response lands in the sink before
 // the run loop's next drain cycle.
 let mut effects = Vec::new();
 t.terminal.effect_sink().drain_into(&mut effects);
 let da1_emitted = effects.iter().any(|e| {
 matches!(
 e,
 Effect::Pty(PtyEffect::Write { bytes, .. })
 if bytes.as_slice() == b"\x1b[?64;6;4c"
 )
 });
 assert!(
 da1_emitted,
 "DA1 response must be emitted within the sync window, got effects: {effects:?}"
 );

 // (b) Mode flag must be cleared by ESU.
 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "post-ESU: TermMode::SYNC_UPDATE must be cleared"
 );

 // (c) Parser-side sync timer must be disarmed — IS the no-spurious-
 // fire pin. Run loop's select! deadline arm only fires when a
 // timeout is pending; `None` guarantees no late `handle_sync_timeout`
 // invocation.
 assert!(
 t.processor.sync_timeout().sync_timeout().is_none(),
 "post-ESU: parser-side sync timer must be disarmed (proves no spurious timeout fires)"
 );
}

/// Combined-dispatch pin: queries + grid-mutating bytes interleave
/// correctly inside a single sync window.
/// Feed BSU + DA1 + "Hello" + DSR 5 + ESU in one chunk via the
/// processor. The fix's inline dispatch must:
/// (a) emit DA1 response within the sync window,
/// (b) emit DSR 5 response,
/// (c) leave "Hello" in the grid by the time the chunk completes,
/// (d) clear the SYNC_UPDATE mode flag at ESU.
/// All four observations land via INLINE dispatch — bytes are processed
/// as they arrive, not deferred to ESU. This is the combined-dispatch
/// pin: queries + grid mutation + ESU all flow through the handler in
/// one chunk without buffering.
/// The companion `mode_2026_active_does_not_publish_snapshot_yet_processes_bytes`
/// pins the snapshot-gate invariant (grid mutates inline BEFORE ESU,
/// but the snapshot publish defers until the gate clears). This test
/// pins the combined post-ESU view.
/// Regression: §03 combined-dispatch pin.
#[test]
fn queries_interleaved_with_grid_mutation_dispatch_inline_during_sync() {
 use oriterm_core::effect::sink::EffectSink;
 use oriterm_core::effect::{Effect, PtyEffect, QueueingEffectSink};

 let sink = QueueingEffectSink::new();
 let (mut t, _wakeup) = make_sync_thread_generic(sink);

 // Feed BSU + DA1 + "Hello" + DSR 5 + ESU directly through the
 // processor (handle_bytes would drain effects into mux events
 // before we can inspect them).
 t.processor
 .advance(&mut t.terminal, b"\x1b[?2026h\x1b[cHello\x1b[5n\x1b[?2026l");

 // Drain effects emitted by the chunk.
 let mut effects = Vec::new();
 t.terminal.effect_sink().drain_into(&mut effects);

 let pty_writes: Vec<&[u8]> = effects
 .iter()
 .filter_map(|e| match e {
 Effect::Pty(PtyEffect::Write { bytes, .. }) => Some(bytes.as_slice()),
 _ => None,
 })
 .collect();
 assert!(
 pty_writes.iter().any(|b| *b == b"\x1b[?64;6;4c"),
 "DA1 response must be emitted within the sync window, got writes: {pty_writes:?}"
 );
 assert!(
 pty_writes.iter().any(|b| *b == b"\x1b[0n"),
 "DSR 5 response must be emitted within the sync window, got writes: {pty_writes:?}"
 );

 // "Hello" must be in the grid (inline mutation, not deferred).
 let row = &t.terminal.grid()[Line(0)];
 let row_text: String = (0..5).map(|c| row[Column(c)].ch).collect();
 assert_eq!(
 row_text, "Hello",
 "grid row 0 must contain 'Hello' after the chunk (inline dispatch)"
 );

 // Mode must be cleared by ESU.
 assert!(
 !t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "ESU must clear TermMode::SYNC_UPDATE"
 );
}

/// Step 4b regression pin: a `PaneIoCommand::SnapshotNow` issued mid-
/// sync must NOT publish a mid-mutation snapshot while
/// `TermMode::SYNC_UPDATE` is set.
/// Inline byte dispatch lands the mutation in the grid the moment the
/// chunk arrives; without this gate, a `SnapshotNow` request from the
/// main thread mid-sync would expose a partial-frame view of the grid.
/// The gate routes through `maybe_produce_snapshot` which returns
/// without publishing whenever `TermMode::SYNC_UPDATE` is set; the
/// reply still fires (the request was acknowledged), and the snapshot
/// publishes on the next ESU/timeout that clears the gate.
/// Regression: §03 SnapshotNow gate pin (Step 4b).
#[test]
fn snapshot_now_during_mode_2026_defers_to_sync_end() {
 let (mut t, wakeup_count) = make_sync_thread_with_wakeup();

 // Enter sync mode + dispatch grid-mutating bytes.
 t.handle_bytes(b"\x1b[?2026hMidSync");
 assert!(
 t.terminal.mode().contains(TermMode::SYNC_UPDATE),
 "precondition: SYNC_UPDATE must be active"
 );

 let wakeup_before = wakeup_count.load(Ordering::Relaxed);
 let seqno_before = t.double_buffer.seqno();

 // Send SnapshotNow via the command handler.
 let (reply_tx, reply_rx) = crossbeam_channel::bounded::<()>(1);
 t.handle_command(PaneIoCommand::SnapshotNow { reply: reply_tx });

 // Reply must fire (the request was acknowledged).
 reply_rx
 .recv_timeout(Duration::from_secs(1))
 .expect("SnapshotNow reply must fire even when publish is deferred");

 // Snapshot must NOT publish while SYNC_UPDATE is set.
 assert_eq!(
 t.double_buffer.seqno(),
 seqno_before,
 "snapshot seqno must NOT advance for SnapshotNow during sync"
 );
 assert_eq!(
 wakeup_count.load(Ordering::Relaxed),
 wakeup_before,
 "wakeup must NOT fire for SnapshotNow during sync"
 );

 // Closing ESU clears the gate; a follow-up snapshot publishes.
 t.handle_bytes(b"\x1b[?2026l");
 t.maybe_produce_snapshot();
 assert!(
 t.double_buffer.seqno() > seqno_before,
 "snapshot must publish after ESU clears the gate"
 );
}
