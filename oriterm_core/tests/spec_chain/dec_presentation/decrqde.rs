//! DECRQDE (CSI " v) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECRQDE`
//! Apex: EffectPtyWrite
//!
//! Reply format: `CSI Ph;Pw;Pml;Pmt;Pmp " w` where Ph = visible rows,
//! Pw = visible cols, Pml = left margin, Pmt = top margin, Pmp = page
//! number. ori_term has no multi-page support → margin + page fields
//! are the constant `1`.

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{SpecHarness, last_pty_write};

#[test]
fn decrqde_reply_echoes_grid_dimensions() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[\"v");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1b[24;80;1;1;1\"w");
}

#[test]
fn decrqde_reply_tracks_custom_grid_size() {
    let mut h = SpecHarness::with_size(10, 40);
    h.feed(b"\x1b[\"v");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1b[10;40;1;1;1\"w");
}
