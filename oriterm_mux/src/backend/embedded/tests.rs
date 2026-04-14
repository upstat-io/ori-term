//! Tests for EmbeddedMux backend.

use std::sync::Arc;

use super::EmbeddedMux;
use crate::PaneId;
use crate::backend::MuxBackend;
use crate::mux_event::MuxNotification;

/// No-op wakeup for tests (no event loop to wake).
fn test_wakeup() -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(|| {})
}

/// Drain notifications from the embedded backend into a `Vec`.
fn drain(mux: &mut EmbeddedMux) -> Vec<MuxNotification> {
    let mut buf = Vec::new();
    mux.drain_notifications(&mut buf);
    buf
}

// -- Object safety and basic queries --

/// `EmbeddedMux` implements `MuxBackend` (compile check via object safety).
#[test]
fn object_safe() {
    let mux = EmbeddedMux::new(test_wakeup());
    let _boxed: Box<dyn MuxBackend> = Box::new(mux);
}

/// `drain_notifications` returns empty when nothing has happened.
#[test]
fn drain_empty() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let mut buf = Vec::new();
    mux.drain_notifications(&mut buf);
    assert!(buf.is_empty());
}

/// `discard_notifications` clears pending notifications.
#[test]
fn discard_notifications() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    mux.discard_notifications();
}

/// `poll_events` with no pending events doesn't panic.
#[test]
fn poll_events_empty() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    mux.poll_events();
}

/// `event_tx` returns `Some` in embedded mode.
#[test]
fn event_tx_available() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(mux.event_tx().is_some());
}

/// `pane_ids` returns empty initially.
#[test]
fn pane_ids_empty() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(mux.pane_ids().is_empty());
}

// -- Pane entry queries (via inject_test_pane helper) --

/// `get_pane_entry` returns metadata for injected panes.
#[test]
fn get_pane_entry_after_inject() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let pid = PaneId::from_raw(100);
    mux.mux.inject_test_pane(pid);

    let entry = mux.get_pane_entry(pid).unwrap();
    assert_eq!(entry.pane, pid);
}

/// `get_pane_entry` returns `None` after close.
#[test]
fn pane_entry_gone_after_close() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let p1 = PaneId::from_raw(100);
    let p2 = PaneId::from_raw(101);
    mux.mux.inject_test_pane(p1);
    mux.mux.inject_test_pane(p2);

    mux.close_pane(p2);
    assert!(mux.get_pane_entry(p2).is_none());
    assert!(mux.get_pane_entry(p1).is_some());
}

// -- close_pane --

/// `close_pane` emits `PaneClosed` notification.
#[test]
fn close_pane_emits_notification() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let p1 = PaneId::from_raw(100);
    let p2 = PaneId::from_raw(101);
    mux.mux.inject_test_pane(p1);
    mux.mux.inject_test_pane(p2);

    mux.close_pane(p2);
    let notes = drain(&mut mux);
    assert!(
        notes
            .iter()
            .any(|n| matches!(n, MuxNotification::PaneClosed { pane_id, .. } if *pane_id == p2))
    );
}

// -- Send + daemon mode --

/// `EmbeddedMux` satisfies `Send` (prevents accidental `Rc`/`Cell` additions).
#[test]
fn embedded_mux_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<EmbeddedMux>();
}

/// `is_daemon_mode` returns false for embedded backend.
#[test]
fn is_not_daemon_mode() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(!mux.is_daemon_mode());
}

// -- IO command routing --

/// `scroll_display` on a non-existent pane is a no-op (no panic).
///
/// The actual command routing is verified by `test_scroll_display_command`
/// in `pane::io_thread::tests`. This test confirms the EmbeddedMux method
/// handles missing panes gracefully.
#[test]
fn scroll_display_missing_pane_is_noop() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let bogus = PaneId::from_raw(999);
    mux.scroll_display(bogus, 5);
}

/// `set_cell_dimensions` on a non-existent pane is a no-op (no panic).
///
/// The actual command routing is verified by
/// `test_set_cell_dimensions_command_marks_dirty` in
/// `pane::io_thread::tests`. This test confirms the EmbeddedMux method
/// handles missing panes gracefully — section 07.6 calls this method
/// from multiple pane-creation call sites and from window-wide
/// broadcasts, so it must never panic on a stale pane id.
#[test]
fn set_cell_dimensions_missing_pane_is_noop() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let bogus = PaneId::from_raw(999);
    mux.set_cell_dimensions(bogus, 8, 16);
}

/// `search_set_query` on a non-existent pane is a no-op (no panic).
///
/// The actual search command handling is verified by
/// `test_search_set_query_finds_matches` in `pane::io_thread::tests`.
#[test]
fn search_set_query_missing_pane_is_noop() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let bogus = PaneId::from_raw(999);
    mux.search_set_query(bogus, "foo".to_string());
}

/// `is_search_active` returns false for non-existent panes.
#[test]
fn search_active_missing_pane() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(!mux.is_search_active(PaneId::from_raw(999)));
}

// -- Cell-metric multi-pane integration --

/// Regression: a newly created pane (simulating a split) receives cell
/// metrics without needing a grid resize first.
///
/// Tests the backend dispatch layer: `EmbeddedMux::set_cell_dimensions`
/// enqueues the command to the correct pane and marks it snapshot-dirty.
/// The IO-thread handler (`PaneIoCommand::SetCellDimensions`) is tested
/// separately by `test_set_cell_dimensions_command_marks_dirty` in
/// `pane::io_thread::tests`, and the Term-level recomputation is tested
/// by `update_cell_coverage_recalculates_fixed_pixel_placements` in
/// `oriterm_core::image::tests`.
#[cfg(unix)]
#[test]
fn split_pane_receives_cell_metrics_without_resize() {
    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();

    // Spawn two panes (simulating a split).
    let p1 = mux.spawn_pane(&config, Theme::Dark).expect("spawn p1");
    let p2 = mux.spawn_pane(&config, Theme::Dark).expect("spawn p2");

    // Clear initial dirty flags from spawn.
    mux.poll_events();
    mux.clear_pane_snapshot_dirty(p1);
    mux.clear_pane_snapshot_dirty(p2);

    // Seed the "split" pane with cell metrics (no resize involved).
    mux.set_cell_dimensions(p2, 10, 20);

    assert!(
        mux.is_pane_snapshot_dirty(p2),
        "newly split pane must be dirty after receiving cell metrics"
    );
    // p1 should not be affected.
    assert!(
        !mux.is_pane_snapshot_dirty(p1),
        "sibling pane must NOT be dirtied by a targeted set_cell_dimensions"
    );

    mux.close_pane(p1);
    mux.close_pane(p2);
    mux.cleanup_closed_pane(p1);
    mux.cleanup_closed_pane(p2);
}

/// Regression: after a font change, BOTH panes in a split receive
/// updated cell metrics. Verifies the backend dispatch fans out to
/// multiple panes correctly. IO-thread processing is verified by
/// `new_window_pane_cell_metrics_reach_io_thread` and the unit test
/// `test_set_cell_dimensions_command_marks_dirty`.
#[cfg(unix)]
#[test]
fn both_split_panes_receive_updated_metrics_after_font_change() {
    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();

    let p1 = mux.spawn_pane(&config, Theme::Dark).expect("spawn p1");
    let p2 = mux.spawn_pane(&config, Theme::Dark).expect("spawn p2");

    // Initial seeding.
    mux.set_cell_dimensions(p1, 8, 16);
    mux.set_cell_dimensions(p2, 8, 16);
    mux.poll_events();
    mux.clear_pane_snapshot_dirty(p1);
    mux.clear_pane_snapshot_dirty(p2);

    // Simulate font change: updated cell metrics to both panes.
    mux.set_cell_dimensions(p1, 10, 20);
    mux.set_cell_dimensions(p2, 10, 20);

    assert!(
        mux.is_pane_snapshot_dirty(p1),
        "first pane must be dirty after font-change broadcast"
    );
    assert!(
        mux.is_pane_snapshot_dirty(p2),
        "second pane must be dirty after font-change broadcast"
    );

    mux.close_pane(p1);
    mux.close_pane(p2);
    mux.cleanup_closed_pane(p1);
    mux.cleanup_closed_pane(p2);
}

/// Regression: a pane in a new window receives cell metrics immediately
/// on creation — before any resize or DPI event arrives.
///
/// Tests the backend dispatch layer (same coverage scope as
/// `split_pane_receives_cell_metrics_without_resize`). The IO-thread
/// end-to-end path is verified by the companion test
/// `new_window_pane_cell_metrics_reach_io_thread`.
#[cfg(unix)]
#[test]
fn new_window_pane_receives_cell_metrics_on_creation() {
    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();
    let pane_id = mux.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

    // Clear initial dirty flag from spawn.
    mux.poll_events();
    mux.clear_pane_snapshot_dirty(pane_id);

    // Immediately seed cell metrics (simulating window creation path).
    mux.set_cell_dimensions(pane_id, 12, 24);

    assert!(
        mux.is_pane_snapshot_dirty(pane_id),
        "new-window pane must be dirty after initial cell metric seeding"
    );

    mux.close_pane(pane_id);
    mux.cleanup_closed_pane(pane_id);
}

/// End-to-end verification: `SetCellDimensions` is processed by the
/// IO thread (not just enqueued). After clearing the synchronous dirty
/// flag set by `EmbeddedMux::set_cell_dimensions`, we poll until the
/// IO thread produces a NEW snapshot — proving the command reached the
/// handler and triggered `grid_dirty`.
#[cfg(unix)]
#[test]
fn new_window_pane_cell_metrics_reach_io_thread() {
    use std::time::{Duration, Instant};

    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();
    let pane_id = mux.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

    // Wait for the initial shell-startup snapshot burst to settle.
    // Crucially, consume the initial snapshot via `refresh_pane_snapshot`
    // so `has_new()` returns false. Without this, `poll_events()` would
    // re-mark the pane dirty from the unconsumed spawn snapshot, making
    // the post-command dirty assertion a false positive.
    let settle = Instant::now() + Duration::from_secs(2);
    while Instant::now() < settle {
        mux.poll_events();
        mux.refresh_pane_snapshot(pane_id);
        mux.clear_pane_snapshot_dirty(pane_id);
        std::thread::sleep(Duration::from_millis(50));
    }

    // Send cell dimensions and immediately clear the synchronous dirty
    // flag. The IO thread hasn't processed the command yet (it runs on
    // a separate thread and poll_events hasn't been called).
    mux.set_cell_dimensions(pane_id, 12, 24);
    mux.clear_pane_snapshot_dirty(pane_id);

    // Poll until the IO thread processes SetCellDimensions, sets
    // grid_dirty, and produces a new snapshot. poll_events detects the
    // new snapshot via has_new_snapshot and re-sets snapshot_dirty.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut io_processed = false;
    while Instant::now() < deadline {
        mux.poll_events();
        if mux.is_pane_snapshot_dirty(pane_id) {
            io_processed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        io_processed,
        "IO thread must process SetCellDimensions and produce a new snapshot"
    );

    mux.close_pane(pane_id);
    mux.cleanup_closed_pane(pane_id);
}

// -- IO-thread snapshot integration --

/// Spawn a real pane, wait for the IO thread to produce a snapshot,
/// and verify `poll_events` marks it dirty and emits `PaneOutput`.
#[cfg(unix)]
#[test]
fn poll_events_uses_has_new_snapshot() {
    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();
    let pane_id = mux.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

    // Send a command to generate output.
    mux.send_input(pane_id, b"echo SNAPSHOT_TEST\n");

    // Poll until the IO thread produces a snapshot (max 5 seconds).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut saw_dirty = false;
    let mut saw_pane_output = false;
    while std::time::Instant::now() < deadline {
        mux.poll_events();
        if mux.is_pane_snapshot_dirty(pane_id) {
            saw_dirty = true;
        }
        let mut notifs = Vec::new();
        mux.drain_notifications(&mut notifs);
        if notifs
            .iter()
            .any(|n| matches!(n, MuxNotification::PaneOutput(id) if *id == pane_id))
        {
            saw_pane_output = true;
        }
        if saw_dirty && saw_pane_output {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(saw_dirty, "IO-thread snapshot should mark pane dirty");
    assert!(
        saw_pane_output,
        "poll_events should emit PaneOutput when IO thread produces a snapshot"
    );

    // Cleanup: close and drop the pane.
    mux.close_pane(pane_id);
    mux.cleanup_closed_pane(pane_id);
}

/// Spawn a pane, close it via `cleanup_closed_pane`, verify full lifecycle:
/// IO thread shutdown, no thread leaks, no lingering state.
#[cfg(unix)]
#[test]
fn cleanup_closed_pane_with_io_thread() {
    use oriterm_core::Theme;

    use crate::domain::SpawnConfig;

    let mut mux = EmbeddedMux::new(test_wakeup());
    let config = SpawnConfig::default();
    let pane_id = mux.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

    // Verify pane exists and has a snapshot.
    assert!(mux.pane_ids().contains(&pane_id));

    // Close and cleanup — should shut down IO thread, reader, writer.
    // `cleanup_closed_pane` drops the Pane on a background thread which
    // calls PaneIoHandle::shutdown() (joins IO thread) and PtyHandle::Drop
    // (kills PTY, joins reader/writer threads).
    mux.close_pane(pane_id);
    mux.cleanup_closed_pane(pane_id);

    // Pane should be fully gone from all maps.
    assert!(mux.pane_snapshot(pane_id).is_none());
    assert!(!mux.pane_ids().contains(&pane_id));
    assert!(!mux.is_pane_snapshot_dirty(pane_id));
    assert!(!mux.is_selection_dirty(pane_id));

    // Wait for the background drop thread to complete. If IO/reader/writer
    // threads leak (fail to join), this is where we'd see a timeout.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // After cleanup, poll_events should not crash or reference the dead pane.
    mux.poll_events();

    // No lingering snapshot or renderable cache.
    let mut buf = oriterm_core::RenderableContent::default();
    assert!(!mux.swap_renderable_content(pane_id, &mut buf));
}

/// TPR-6 semantic pin: `sync_pane_snapshot` returns `None` for an
/// unknown pane id rather than blocking on a non-existent IO thread.
///
/// We can't easily simulate the timeout case (the IO thread always
/// responds quickly under test load) but we CAN verify that the
/// missing-pane fast path returns None instead of producing a stale
/// snapshot from a different pane. This locks in the "guaranteed-fresh
/// or None" contract documented on the trait method.
#[test]
fn sync_pane_snapshot_returns_none_for_missing_pane() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let bogus = PaneId::from_raw(99_999);
    assert!(
        mux.sync_pane_snapshot(bogus).is_none(),
        "sync_pane_snapshot must return None for an unknown pane id, \
         not silently fall back to refresh_pane_snapshot's cached value",
    );
}
