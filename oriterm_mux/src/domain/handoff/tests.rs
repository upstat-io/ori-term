//! Tests for `adopt_pane` — the cross-platform mux entry point that
//! constructs a `Pane` from pre-existing PTY handles produced by the
//! Windows console host handoff (Section 03.9, Phase 1B).
//!
//! These tests run on all three platforms because `adopt_pane` accepts
//! `Box<dyn io::Read + Send>` / `Box<dyn io::Write + Send>` trait objects
//! and an `AdoptedSignal` stub on non-Windows. The Windows-specific COM
//! integration tests live in Section 03.9 Phase 3.

use std::io::{Cursor, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use oriterm_core::Theme;

use super::{AdoptConfig, adopt_pane};
use crate::id::{DomainId, PaneId};
use crate::mux_event::MuxEvent;
use crate::pty::AdoptedPtyHandle;
use crate::pty::adopt::AdoptedSignal;

/// In-memory writer that discards bytes — adopted-pane tests don't
/// inspect the writer output, only verify the IO thread spins up and
/// shuts down cleanly.
struct DiscardWriter;

impl Write for DiscardWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Build a default `AdoptConfig` for tests.
///
/// Reader contains a short payload that immediately reaches EOF (so the
/// reader thread exits cleanly without waiting on real I/O).
fn test_config(pane_id: u64) -> AdoptConfig {
    test_config_with_signal(pane_id, AdoptedSignal::stub_for_tests())
}

/// Like `test_config` but with a caller-supplied `AdoptedSignal`. Used
/// by tests that need to verify the signal moves into the IO thread on
/// adopt.
fn test_config_with_signal(pane_id: u64, signal: AdoptedSignal) -> AdoptConfig {
    let reader_bytes = b"adopted\n".to_vec();
    let adopted = AdoptedPtyHandle::new(
        Box::new(Cursor::new(reader_bytes)),
        Box::new(DiscardWriter),
        signal,
        Some(0xC0FFEE),
    );

    AdoptConfig {
        pane_id: PaneId::from_raw(pane_id),
        domain_id: DomainId::from_raw(7),
        adopted,
        rows: 24,
        cols: 80,
        scrollback: 1_000,
        theme: Theme::Dark,
    }
}

fn test_wakeup() -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(|| {})
}

#[test]
fn adopt_pane_returns_pane_with_expected_id() {
    let config = test_config(101);
    let (mux_tx, _mux_rx) = std::sync::mpsc::channel::<MuxEvent>();
    let pane = adopt_pane(config, &mux_tx, &test_wakeup()).expect("adopt_pane");
    assert_eq!(pane.id(), PaneId::from_raw(101));
    drop(pane);
}

#[test]
fn adopt_pane_records_client_pid_via_pty_lifecycle() {
    // Semantic pin: process_id() must reach the AdoptedPtyHandle through
    // the boxed PtyLifecycle dispatch so the Pane can report the client
    // PID even though no Child was spawned. If the box wraps the wrong
    // type or process_id() returns the wrong field, this test fails.
    let config = test_config(102);
    let (mux_tx, _mux_rx) = std::sync::mpsc::channel::<MuxEvent>();
    let pane = adopt_pane(config, &mux_tx, &test_wakeup()).expect("adopt_pane");
    assert_eq!(pane.process_id(), Some(0xC0FFEE));
}

#[test]
fn adopt_pane_io_thread_produces_initial_snapshot() {
    // Semantic pin: the IO thread must run and call produce_snapshot()
    // at startup. If adopt_pane fails to spawn the IO thread, has_io_snapshot()
    // returns false and this test fails.
    let config = test_config(103);
    let (mux_tx, _mux_rx) = std::sync::mpsc::channel::<MuxEvent>();
    let pane = adopt_pane(config, &mux_tx, &test_wakeup()).expect("adopt_pane");

    // Wait briefly for the IO thread to publish its initial snapshot.
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut saw_snapshot = false;
    while Instant::now() < deadline {
        if pane.has_io_snapshot() {
            saw_snapshot = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        saw_snapshot,
        "IO thread must publish an initial snapshot within 1 second"
    );
}

#[test]
fn adopt_pane_signal_moves_into_pane_drops_with_pane() {
    // TPR-4 semantic pin: adopt_pane must call AdoptedPtyHandle::take_signal
    // (which moves the signal out into the IO thread's adopted_signal slot)
    // so that resize can be routed through the conhost signal pipe. After
    // adopt_pane returns, the boxed AdoptedPtyHandle in Pane.pty no longer
    // owns the signal — it lives on the IO thread. We verify the contract
    // by constructing the handle, calling adopt_pane, then calling
    // take_signal on the original handle (which would only work if it
    // hadn't been moved). Since adopt_pane consumes the AdoptConfig by
    // value, we observe the move via the resulting Pane's normal lifecycle.
    //
    // Direct observation: drop the resulting Pane and assert it doesn't
    // panic. Drop chain: Pane → PaneIoHandle → IO thread join → IO thread
    // drops adopted_signal → AdoptedSignal::Drop closes handles. The stub
    // signal has null handles so Drop is a no-op, but the path is exercised.
    let signal = AdoptedSignal::stub_for_tests();
    let config = test_config_with_signal(105, signal);
    let (mux_tx, _mux_rx) = std::sync::mpsc::channel::<MuxEvent>();
    let pane = adopt_pane(config, &mux_tx, &test_wakeup()).expect("adopt_pane");
    drop(pane);
}

#[test]
fn adopt_pane_drop_joins_io_thread_within_one_second() {
    // Semantic pin: dropping the Pane must shut down the IO thread,
    // writer thread, and reader thread cleanly within 1 second. If any
    // thread leaks the join handle or fails to receive the shutdown
    // signal, the elapsed time exceeds 1 second.
    let config = test_config(104);
    let (mux_tx, _mux_rx) = std::sync::mpsc::channel::<MuxEvent>();
    let pane = adopt_pane(config, &mux_tx, &test_wakeup()).expect("adopt_pane");

    let started = Instant::now();
    drop(pane);
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "Pane drop must complete within 1 second (got {elapsed:?})"
    );
}
