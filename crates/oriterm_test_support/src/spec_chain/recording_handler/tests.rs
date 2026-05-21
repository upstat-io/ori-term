//! Tests for `RecordingHandler` typed dispatch-arg capture.
//!
//! The `DispatchArgs::KittyApc` variant closes the gap surfaced by
//! HYG-13.1-003: before its introduction, every kitty APC payload
//! collapsed into the generic `Other { method: "apc_dispatch" }` arm,
//! preventing matrix tests from asserting per-cell dispatch identity
//! (action × format × transmission). Tests here pin the typed variant
//! for representative (action, format, transmission) cells plus the
//! malformed-APC fallback.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oriterm_core::image::kitty::{KittyAction, KittyTransmission};

use super::DispatchArgs;
use crate::spec_chain::SpecHarness;
use crate::spec_chain::kitty_fixtures::kitty_apc;

/// Extract the first `KittyApc` dispatch call's command, panicking if
/// no such variant was recorded.
fn first_kitty_command(h: &SpecHarness) -> &oriterm_core::image::kitty::KittyCommand {
    h.outcome()
        .dispatched_calls
        .iter()
        .find_map(|c| match &c.args {
            DispatchArgs::KittyApc { command } => Some(command.as_ref()),
            _ => None,
        })
        .expect("expected one DispatchArgs::KittyApc")
}

/// Pins: an APC with the `G` discriminator plus a valid kitty control +
/// base64 payload records `DispatchArgs::KittyApc` with the parsed
/// `KittyCommand` — closing HYG-13.1-003's gap where every kitty APC
/// previously collapsed into `Other { method: "apc_dispatch" }`.
/// Anchor: HYG-13.1-003 §13.1 close-out.
#[test]
fn apc_dispatch_g_prefix_records_typed_kitty_command() {
    let mut h = SpecHarness::new();
    let rgba_1x1 = STANDARD.encode([255_u8, 0, 0, 255]);
    h.feed(&kitty_apc(b"a=t,f=32,t=d,s=1,v=1", &rgba_1x1));

    let cmd = first_kitty_command(&h);
    assert_eq!(cmd.action, KittyAction::Transmit);
    assert_eq!(cmd.format, 32);
    assert_eq!(cmd.transmission, KittyTransmission::Direct);
}

/// Pins: the per-action parse surfaces `KittyAction::Query` for `a=q` (plus
/// `image_id`) — proves the action-dispatch matrix in `kitty/actions.rs`
/// can distinguish Query from other actions at the recorded-call rung.
/// Anchor: HYG-13.1-003 §13.1 close-out.
#[test]
fn apc_dispatch_records_per_action_kitty_command() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=q,i=1", ""));

    let cmd = first_kitty_command(&h);
    assert_eq!(
        cmd.action,
        KittyAction::Query,
        "a=q must be captured as KittyAction::Query",
    );
    assert_eq!(cmd.image_id, Some(1));
}

/// Pins: the per-transmission parse surfaces `KittyTransmission::File` for
/// `t=f` — proves the transmission-dispatch matrix in `kitty/actions.rs`
/// can distinguish File from Direct / TempFile / SharedMemory at the
/// recorded-call rung. Anchor: HYG-13.1-003 §13.1 close-out.
#[test]
fn apc_dispatch_records_per_transmission_kitty_command() {
    let mut h = SpecHarness::new();
    let path_b64 = STANDARD.encode("/tmp/unused.rgba");
    h.feed(&kitty_apc(b"a=t,f=32,t=f,s=1,v=1", &path_b64));

    let cmd = first_kitty_command(&h);
    assert_eq!(cmd.transmission, KittyTransmission::File);
}

/// Regression guard: a non-`G` APC payload must NOT record the typed
/// variant — the record stays on the `Other` fallback arm. Proves the
/// first-byte gate runs before parsing so malformed APCs of other
/// protocols are not misclassified. Anchor: HYG-13.1-003 §13.1 close-out.
#[test]
fn apc_dispatch_non_g_prefix_falls_back_to_other() {
    let mut h = SpecHarness::new();
    // `\x1b_X;stuff\x1b\\` — APC with `X` discriminator, not kitty.
    h.feed(b"\x1b_X;unknown\x1b\\");

    let kitty_count = h
        .outcome()
        .dispatched_calls
        .iter()
        .filter(|c| matches!(c.args, DispatchArgs::KittyApc { .. }))
        .count();
    assert_eq!(kitty_count, 0, "non-G APC must not record KittyApc");

    let other_apc = h.outcome().dispatched_calls.iter().any(|c| {
        matches!(
            c.args,
            DispatchArgs::Other {
                method: "apc_dispatch"
            }
        )
    });
    assert!(other_apc, "non-G APC must fall back to Other arm");
}

/// Regression guard: a kitty APC with malformed base64 payload falls back
/// to `Other` rather than recording a partial command. Matches
/// `parse_kitty_command`'s documented `KittyError::InvalidBase64`
/// contract. Anchor: HYG-13.1-003 §13.1 close-out.
#[test]
fn apc_dispatch_invalid_base64_payload_falls_back_to_other() {
    let mut h = SpecHarness::new();
    // `%%%%` is not valid base64; the parser's `decode_base64` rejects.
    h.feed(b"\x1b_Ga=t,f=32,s=1,v=1;%%%%\x1b\\");

    let kitty_count = h
        .outcome()
        .dispatched_calls
        .iter()
        .filter(|c| matches!(c.args, DispatchArgs::KittyApc { .. }))
        .count();
    assert_eq!(
        kitty_count, 0,
        "invalid-base64 kitty APC must not record a typed KittyApc",
    );

    let other_apc = h.outcome().dispatched_calls.iter().any(|c| {
        matches!(
            c.args,
            DispatchArgs::Other {
                method: "apc_dispatch"
            }
        )
    });
    assert!(
        other_apc,
        "invalid-base64 kitty APC must fall back to Other arm",
    );
}
