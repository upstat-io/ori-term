//! Unit tests for mux pump helpers.

use std::time::Duration;

use oriterm_core::effect::NotificationSource;
use oriterm_mux::{MuxNotification, PaneId};

use super::{format_duration_body, purge_pending_desktop_notifications};

fn dn(pane_id: PaneId, title: &str) -> MuxNotification {
    MuxNotification::DesktopNotification {
        pane_id,
        source: NotificationSource::Osc9,
        title: title.to_owned(),
        body: String::new(),
    }
}

/// Regression: `[high]` — staging-buffer purge for
/// `ClearPendingDesktopNotifications` must remove preceding
/// `DesktopNotification` entries for the same pane that landed in
/// earlier IO-thread batches and accumulated in `notification_buf`.
#[test]
fn purge_drops_preceding_desktop_notifications_for_same_pane() {
    let pane = PaneId::from_raw(1);
    let mut buf = vec![
        dn(pane, "A"),
        dn(pane, "B"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "C"),
    ];
    purge_pending_desktop_notifications(&mut buf);
    assert_eq!(buf.len(), 2, "got {buf:?}");
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(p) if *p == pane
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::DesktopNotification { title, .. } if title == "C"
    ));
}

#[test]
fn purge_only_targets_matching_pane() {
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);
    let mut buf = vec![
        dn(pane_a, "A1"),
        dn(pane_b, "B1"),
        MuxNotification::ClearPendingDesktopNotifications(pane_a),
    ];
    purge_pending_desktop_notifications(&mut buf);
    assert_eq!(buf.len(), 2);
    assert!(matches!(
        &buf[0],
        MuxNotification::DesktopNotification { title, .. } if title == "B1"
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::ClearPendingDesktopNotifications(p) if *p == pane_a
    ));
}

#[test]
fn purge_handles_multiple_clear_markers() {
    let pane = PaneId::from_raw(1);
    let mut buf = vec![
        dn(pane, "A"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "B"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "C"),
    ];
    purge_pending_desktop_notifications(&mut buf);
    // A dropped (precedes first clear). B dropped (precedes second clear).
    // Both clear markers retained. C survives (follows both clears).
    assert_eq!(buf.len(), 3, "got {buf:?}");
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
        &buf[2],
        MuxNotification::DesktopNotification { title, .. } if title == "C"
    ));
}

#[test]
fn format_seconds_only() {
    assert_eq!(
        format_duration_body(Duration::from_secs(12)),
        "Completed in 12s"
    );
}

#[test]
fn format_minutes_and_seconds() {
    assert_eq!(
        format_duration_body(Duration::from_secs(150)),
        "Completed in 2m 30s"
    );
}

#[test]
fn format_hours_and_minutes() {
    assert_eq!(
        format_duration_body(Duration::from_secs(3900)),
        "Completed in 1h 5m"
    );
}

#[test]
fn format_exactly_one_minute() {
    assert_eq!(
        format_duration_body(Duration::from_secs(60)),
        "Completed in 1m 0s"
    );
}

#[test]
fn format_exactly_one_hour() {
    assert_eq!(
        format_duration_body(Duration::from_secs(3600)),
        "Completed in 1h 0m"
    );
}

#[test]
fn format_zero_seconds() {
    assert_eq!(format_duration_body(Duration::ZERO), "Completed in 0s");
}
