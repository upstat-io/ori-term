//! Tests for `host_request` wire types — round-trip + From conversions.

use oriterm_core::effect::{ClipboardSelection, NotificationSource};

use super::{HostReplyPayload, WireClipboardSelection, WireNotificationSource};

// -- WireClipboardSelection --

/// Every `WireClipboardSelection` variant survives `core → wire → core` round-trip.
#[test]
fn clipboard_selection_round_trip_via_core_to_wire_to_core() {
    for selection in [
        ClipboardSelection::Clipboard,
        ClipboardSelection::Primary,
        ClipboardSelection::Select,
    ] {
        let wire: WireClipboardSelection = selection.into();
        let back: ClipboardSelection = wire.into();
        assert_eq!(selection, back, "round-trip must preserve {selection:?}");
    }
}

/// Every `WireClipboardSelection` variant survives `wire → core → wire` round-trip.
#[test]
fn clipboard_selection_round_trip_via_wire_to_core_to_wire() {
    for wire in [
        WireClipboardSelection::Clipboard,
        WireClipboardSelection::Primary,
        WireClipboardSelection::Select,
    ] {
        let core: ClipboardSelection = wire.into();
        let back: WireClipboardSelection = core.into();
        assert_eq!(wire, back, "round-trip must preserve {wire:?}");
    }
}

/// `from_u8` decodes the 3 documented discriminants and falls back to default
/// for unknown values (forward compatibility — matches `WireCursorShape`).
#[test]
fn wire_clipboard_selection_from_u8_decodes_known_and_defaults_unknown() {
    assert_eq!(
        WireClipboardSelection::from_u8(0),
        WireClipboardSelection::Clipboard
    );
    assert_eq!(
        WireClipboardSelection::from_u8(1),
        WireClipboardSelection::Primary
    );
    assert_eq!(
        WireClipboardSelection::from_u8(2),
        WireClipboardSelection::Select
    );
    assert_eq!(
        WireClipboardSelection::from_u8(99),
        WireClipboardSelection::default(),
        "unknown discriminant must default to Clipboard"
    );
}

// -- WireNotificationSource --

/// Every `WireNotificationSource` variant survives `core → wire → core` round-trip.
#[test]
fn notification_source_round_trip_via_core_to_wire_to_core() {
    for source in [
        NotificationSource::Osc9,
        NotificationSource::Osc99,
        NotificationSource::Osc777,
    ] {
        let wire: WireNotificationSource = source.into();
        let back: NotificationSource = wire.into();
        assert_eq!(source, back, "round-trip must preserve {source:?}");
    }
}

/// Every `WireNotificationSource` variant survives `wire → core → wire` round-trip.
#[test]
fn notification_source_round_trip_via_wire_to_core_to_wire() {
    for wire in [
        WireNotificationSource::Osc9,
        WireNotificationSource::Osc99,
        WireNotificationSource::Osc777,
    ] {
        let core: NotificationSource = wire.into();
        let back: WireNotificationSource = core.into();
        assert_eq!(wire, back, "round-trip must preserve {wire:?}");
    }
}

#[test]
fn wire_notification_source_from_u8_decodes_known_and_defaults_unknown() {
    assert_eq!(
        WireNotificationSource::from_u8(0),
        WireNotificationSource::Osc9
    );
    assert_eq!(
        WireNotificationSource::from_u8(1),
        WireNotificationSource::Osc99
    );
    assert_eq!(
        WireNotificationSource::from_u8(2),
        WireNotificationSource::Osc777
    );
    assert_eq!(
        WireNotificationSource::from_u8(255),
        WireNotificationSource::default(),
        "unknown discriminant must default to Osc9"
    );
}

// -- HostReplyPayload --

#[test]
fn host_reply_payload_clipboard_load_round_trips_via_bincode() {
    let payload = HostReplyPayload::ClipboardLoad {
        text: "hello daemon".into(),
    };
    let encoded = bincode::serialize(&payload).expect("encode");
    let decoded: HostReplyPayload = bincode::deserialize(&encoded).expect("decode");
    assert_eq!(payload, decoded);
}

#[test]
fn host_reply_payload_color_query_round_trips_via_bincode() {
    let payload = HostReplyPayload::ColorQuery {
        rgb: [0x12, 0x34, 0x56],
    };
    let encoded = bincode::serialize(&payload).expect("encode");
    let decoded: HostReplyPayload = bincode::deserialize(&encoded).expect("decode");
    assert_eq!(payload, decoded);
}
