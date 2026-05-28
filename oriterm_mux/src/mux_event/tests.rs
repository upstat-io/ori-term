//! Tests for mux event types.
//!
//! After effect-cutover §01.3 the IO thread routes effects directly
//! via `effect_router`; this file covers `MuxEvent` / `MuxNotification`
//! enum surface (Debug, variant exhaustiveness) only.

use oriterm_core::effect::PtyWriteKind;

use crate::PaneId;

use super::MuxEvent;

#[test]
fn mux_event_debug_format() {
    let event = MuxEvent::PaneOutput(PaneId::from_raw(5));
    assert_eq!(format!("{event:?}"), "PaneOutput(Pane(5))");

    let event = MuxEvent::PaneExited {
        pane_id: PaneId::from_raw(3),
        exit_code: 1,
    };
    assert_eq!(format!("{event:?}"), "PaneExited(Pane(3), code=1)");
}

/// Debug format for all MuxEvent variants.
#[test]
fn mux_event_debug_all_variants() {
    let id = PaneId::from_raw(1);

    let cases = [
        (MuxEvent::PaneOutput(id), "PaneOutput(Pane(1))"),
        (
            MuxEvent::PaneExited {
                pane_id: id,
                exit_code: 0,
            },
            "PaneExited(Pane(1), code=0)",
        ),
        (
            MuxEvent::PaneTitleChanged {
                pane_id: id,
                title: "hello".to_string(),
            },
            "PaneTitleChanged(Pane(1), \"hello\")",
        ),
        (
            MuxEvent::PaneIconChanged {
                pane_id: id,
                icon_name: "\u{1f40d}".to_string(),
            },
            "PaneIconChanged(Pane(1), \"\u{1f40d}\")",
        ),
        (
            MuxEvent::PaneCwdChanged {
                pane_id: id,
                cwd: "/tmp".to_string(),
            },
            "PaneCwdChanged(Pane(1), \"/tmp\")",
        ),
        (
            MuxEvent::CommandComplete {
                pane_id: id,
                duration: std::time::Duration::from_secs(15),
            },
            "CommandComplete(Pane(1), 15s)",
        ),
        (MuxEvent::PaneBell(id), "PaneBell(Pane(1))"),
        (
            MuxEvent::PtyWrite {
                pane_id: id,
                kind: PtyWriteKind::Other,
                data: b"abc".to_vec(),
            },
            "PtyWrite(Pane(1), Other, 3 bytes)",
        ),
    ];

    for (event, expected) in &cases {
        assert_eq!(format!("{event:?}"), *expected);
    }

    let store = MuxEvent::ClipboardStore {
        pane_id: id,
        clipboard_type: oriterm_core::ClipboardType::Clipboard,
        text: "copied".to_string(),
    };
    let dbg = format!("{store:?}");
    assert!(dbg.contains("ClipboardStore"));
    assert!(dbg.contains("Clipboard"));

    // Variants added in effect-cutover 01.1 — Debug coverage per
    // `[low]`.
    let desk = MuxEvent::DesktopNotification {
        pane_id: id,
        source: oriterm_core::effect::NotificationSource::Osc99,
        title: "Hello".to_string(),
        body: String::new(),
    };
    assert_eq!(
        format!("{desk:?}"),
        "DesktopNotification(Pane(1), Osc99, \"Hello\")"
    );

    let clear = MuxEvent::ClearPendingDesktopNotifications(id);
    assert_eq!(
        format!("{clear:?}"),
        "ClearPendingDesktopNotifications(Pane(1))"
    );

    let host_clip = MuxEvent::HostClipboardLoad {
        pane_id: id,
        selection: oriterm_core::effect::ClipboardSelection::Clipboard,
        clipboard_char: b'c',
        terminator: "\x1b\\".to_string(),
        reply: oriterm_core::effect::ResponseToken::new(),
    };
    let dbg = format!("{host_clip:?}");
    assert!(dbg.contains("HostClipboardLoad"));
    assert!(dbg.contains("Clipboard"));

    let host_color = MuxEvent::HostColorQuery {
        pane_id: id,
        prefix: "11".to_string(),
        index: 0,
        terminator: "\x1b\\".to_string(),
        reply: oriterm_core::effect::ResponseToken::new(),
    };
    let dbg = format!("{host_color:?}");
    assert!(dbg.contains("HostColorQuery"));
}

// MuxNotification Debug format

#[test]
fn mux_notification_debug_all_variants() {
    use super::MuxNotification;

    let pid = PaneId::from_raw(1);

    let cases: Vec<(MuxNotification, &str)> = vec![
        (
            MuxNotification::PaneMetadataChanged(pid),
            "PaneMetadataChanged(Pane(1))",
        ),
        (MuxNotification::PaneOutput(pid), "PaneOutput(Pane(1))"),
        (
            MuxNotification::PaneClosed {
                pane_id: pid,
                exit_code: 0,
            },
            "PaneClosed(Pane(1), code=0)",
        ),
        (MuxNotification::PaneBell(pid), "PaneBell(Pane(1))"),
        (
            MuxNotification::CommandComplete {
                pane_id: pid,
                duration: std::time::Duration::from_secs(30),
            },
            "CommandComplete(Pane(1), 30s)",
        ),
    ];

    for (notif, expected) in &cases {
        assert_eq!(format!("{notif:?}"), *expected);
    }

    let store = MuxNotification::ClipboardStore {
        pane_id: pid,
        clipboard_type: oriterm_core::ClipboardType::Clipboard,
        text: "copied".to_string(),
    };
    let dbg = format!("{store:?}");
    assert!(dbg.contains("ClipboardStore"));
    assert!(dbg.contains("Clipboard"));

    // Variants added in effect-cutover 01.1 — Debug coverage per the
    // matrix-testing rule (). Surfaced by
    // `[low]`.
    let desk = MuxNotification::DesktopNotification {
        pane_id: pid,
        source: oriterm_core::effect::NotificationSource::Osc9,
        title: "T".to_string(),
        body: "B".to_string(),
    };
    let dbg = format!("{desk:?}");
    assert!(dbg.contains("DesktopNotification"));
    assert!(dbg.contains("Osc9"));
    assert!(dbg.contains("\"T\""));

    let clear = MuxNotification::ClearPendingDesktopNotifications(pid);
    assert_eq!(
        format!("{clear:?}"),
        "ClearPendingDesktopNotifications(Pane(1))"
    );

    let host_clip = MuxNotification::HostClipboardLoad {
        pane_id: pid,
        selection: oriterm_core::effect::ClipboardSelection::Clipboard,
        clipboard_char: b'c',
        terminator: "\x1b\\".to_string(),
        reply: oriterm_core::effect::ResponseToken::new(),
    };
    let dbg = format!("{host_clip:?}");
    assert!(dbg.contains("HostClipboardLoad"));
    assert!(dbg.contains("Clipboard"));

    let host_color = MuxNotification::HostColorQuery {
        pane_id: pid,
        prefix: "10".to_string(),
        index: 0,
        terminator: "\x1b\\".to_string(),
        reply: oriterm_core::effect::ResponseToken::new(),
    };
    let dbg = format!("{host_color:?}");
    assert!(dbg.contains("HostColorQuery"));
    assert!(dbg.contains("\"10\""));
}
