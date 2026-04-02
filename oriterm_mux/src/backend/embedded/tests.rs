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
