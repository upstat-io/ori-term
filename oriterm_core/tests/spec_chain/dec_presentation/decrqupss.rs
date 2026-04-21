//! DECRQUPSS (CSI & u) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECRQUPSS`
//! Apex: EffectPtyWrite
//!
//! `verified-with-deviation`: constant reply identifying ISO Latin-1.
//! ori_term has no NRCS charset selection.

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decrqupss_reply_is_iso_latin1_constant() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[&u");
    let (bytes, kind) = h
        .outcome()
        .effects_emitted
        .iter()
        .rev()
        .find_map(|eff| match eff {
            Effect::Pty(PtyEffect::Write { bytes, kind }) => Some((bytes.clone(), *kind)),
            _ => None,
        })
        .expect("expected a PtyEffect::Write");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1bP1!u%5\x1b\\");
}
