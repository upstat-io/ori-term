//! DECRQPSR (CSI Ps $ w) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECRQPSR`
//! Apex: EffectPtyWrite
//!
//! Mode 1 (cursor info) is `verified-with-deviation` — the cursor
//! payload is a stub (line/col plus filler slots). Mode 2 serializes
//! the real tab-stop vector.

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{SpecHarness, last_pty_write, pty_writes};

#[test]
fn decrqpsr_mode2_serializes_default_tab_stops() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[2$w");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1bP2$u1/9/17/25/33/41/49/57/65/73\x1b\\");
}

#[test]
fn decrqpsr_mode2_reflects_runtime_tab_stop_mutations() {
    let mut h = SpecHarness::with_size(5, 10);
    // Clear every stop, then place a single stop at column 4 (1-based).
    h.feed(b"\x1b[3g");
    h.feed(b"\x1b[1;4H\x1bH"); // cursor to col 4, HTS
    h.feed(b"\x1b[2$w");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP2$u4\x1b\\");
}

#[test]
fn decrqpsr_mode1_emits_dcs_header_and_terminator() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[1$w");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert!(
        bytes.starts_with(b"\x1bP1$u"),
        "mode-1 reply must carry the DCS 1 $ u header, got {:?}",
        String::from_utf8_lossy(&bytes),
    );
    assert!(bytes.ends_with(b"\x1b\\"));
}

#[test]
fn decrqpsr_unknown_mode_does_not_reply() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[9$w"); // mode 9 unknown
    let writes: Vec<_> = pty_writes(&h).map(|(b, _)| b.to_vec()).collect();
    assert!(writes.is_empty(), "unknown DECRQPSR mode must not reply");
}
