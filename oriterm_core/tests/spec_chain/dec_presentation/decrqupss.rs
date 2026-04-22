//! DECRQUPSS (CSI & u) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECRQUPSS`
//! Apex: EffectPtyWrite
//!
//! `verified-with-deviation`: constant reply identifying ISO Latin-1.
//! ori_term has no NRCS charset selection.

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

#[test]
fn decrqupss_reply_is_iso_latin1_constant() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[&u");
    let (bytes, kind) = pty_writes(&h).last().expect("expected a PtyEffect::Write");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1bP1!u%5\x1b\\");
}
