//! Section 06.5 direct-VTE cap xcheck — synchronized output mode 2026.
//!
//! `Sync` toggles `TermMode::SYNC_UPDATE` via DECSET/DECRST 2026.
//! `oriterm_core/src/term/handler/modes.rs:83` is the canonical
//! handler.

use crate::term::TermMode;

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Sync"];

#[test]
fn tack_cap_xcheck_sync_cap_value_matches() {
    // fix: pin the literal Sync declaration so a
    // terminfo edit that changes the DECSET/DECRST 2026 cap value
    // triggers a test failure BEFORE the round-trip tests below.
    assert_cap_value_matches("Sync", "\\E[?2026%?%p1%{1}%-%tl%eh%;");
}

#[test]
fn tack_cap_xcheck_sync_enters_sync_update() {
    assert_cap_declared("Sync");
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[?2026h");
    assert!(
        term.mode().contains(TermMode::SYNC_UPDATE),
        "DECSET 2026 must set TermMode::SYNC_UPDATE"
    );
}

#[test]
fn tack_cap_xcheck_sync_exits_sync_update() {
    let (mut term, _l) = term_with_recorder();
    // Pre-seed SYNC_UPDATE, then clear via DECRST 2026.
    feed(&mut term, b"\x1b[?2026h");
    feed(&mut term, b"\x1b[?2026l");
    assert!(
        !term.mode().contains(TermMode::SYNC_UPDATE),
        "DECRST 2026 must clear TermMode::SYNC_UPDATE"
    );
}
