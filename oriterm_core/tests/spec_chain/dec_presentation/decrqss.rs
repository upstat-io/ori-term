//! DECRQSS (DCS $ q Pt ST) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECRQSS`
//! Apex: EffectPtyWrite
//!
//! DECRQSS replies with `DCS 1 $ r Pt ST` for recognized Pt targets (echoing
//! the queried setting back to the host) or `DCS 0 $ r ST` for unknown Pt.
//! §09A.9 extends the existing DECRQSS dispatcher with DECSCUSR (`q`) and
//! DECSCA (`"q`) branches on top of the baseline (DECSCL, DECSTBM, SGR,
//! DECSLRM).

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{SpecHarness, last_pty_write};

/// Pins: a fresh terminal with no DECSCUSR setter yet replies `DCS 1 $ r 1 q ST`
/// to `DCS $ q q ST` — default cursor style is Ps=1 (blinking block).
/// Anchor: catalog row `DECPRES-DECRQSS` (DECSCUSR branch).
#[test]
fn decrqss_decscusr_default_reports_ps1_blink_block() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1bP$q q\x1b\\");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1bP1$r1 q\x1b\\");
}

/// Pins: DECRQSS echoes the most-recent DECSCUSR setter — after `CSI 4 SP q`
/// (steady underline) the reply carries Ps=4, proving the query reads the
/// live cursor-style state rather than a cached default.
/// Anchor: catalog row `DECPRES-DECRQSS` (DECSCUSR branch).
#[test]
fn decrqss_decscusr_tracks_csi_space_q_setter() {
    let mut h = SpecHarness::with_size(24, 80);
    // DECSCUSR 4 = steady underline.
    h.feed(b"\x1b[4 q");
    h.feed(b"\x1bP$q q\x1b\\");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1$r4 q\x1b\\");
}

/// Pins: DECSCUSR=6 (steady bar) is echoed verbatim by DECRQSS — matrix
/// companion to `decrqss_decscusr_tracks_csi_space_q_setter` covering a
/// different Ps value so the query cannot be passing by aliasing to a single
/// constant. Anchor: catalog row `DECPRES-DECRQSS` (DECSCUSR branch).
#[test]
fn decrqss_decscusr_tracks_steady_bar() {
    let mut h = SpecHarness::with_size(24, 80);
    // DECSCUSR 6 = steady bar.
    h.feed(b"\x1b[6 q");
    h.feed(b"\x1bP$q q\x1b\\");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1$r6 q\x1b\\");
}

/// Pins: with no DECSCA setter, `DCS $ q " q ST` replies `DCS 1 $ r 2 " q ST`
/// — Ps=2 is the VT525 default meaning "unprotected" (DECSCA baseline).
/// Anchor: catalog row `DECPRES-DECRQSS` (DECSCA branch).
#[test]
fn decrqss_decsca_default_reports_unprotected() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1bP$q\"q\x1b\\");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::StatusString);
    assert_eq!(bytes, b"\x1bP1$r2\"q\x1b\\");
}

/// Pins: after `CSI 1 " q` sets DECSCA to protected, DECRQSS echoes Ps=1
/// rather than the default Ps=2 — proves the DECSCA branch reads the live
/// character-attribute mode, not a hardcoded default.
/// Anchor: catalog row `DECPRES-DECRQSS` (DECSCA branch).
#[test]
fn decrqss_decsca_reports_protected_after_setter() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[1\"q"); // DECSCA 1 → protected.
    h.feed(b"\x1bP$q\"q\x1b\\");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1$r1\"q\x1b\\");
}

/// Negative pin: an unrecognized Pt (here, bare `Z`) replies with the
/// invalid-request shape `DCS 0 $ r ST` — proves DECRQSS does NOT silently
/// drop unknown targets and does NOT echo Pt back verbatim with Ps=1.
/// Anchor: catalog row `DECPRES-DECRQSS` (invalid-Pt branch).
#[test]
fn decrqss_unknown_pt_reports_invalid() {
    let mut h = SpecHarness::with_size(24, 80);
    // DCS $ q Z ST — unrecognized Pt.
    h.feed(b"\x1bP$qZ\x1b\\");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP0$r\x1b\\");
}

/// Negative pin: the baseline DECSCL (`" p`) branch continues to reply
/// `DCS 1 $ r 64;1 " p ST` after §09A.9 widens the match table — guards
/// against accidentally shadowing pre-existing targets when adding DECSCUSR /
/// DECSCA branches. Anchor: catalog row `DECPRES-DECRQSS` (DECSCL branch).
#[test]
fn decrqss_decscl_baseline_still_replies() {
    // Negative pin: baseline DECSCL branch must continue to reply after §09A.9
    // extends the match table — guards against accidentally shadowing the
    // pre-existing targets when adding new ones.
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1bP$q\"p\x1b\\");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1$r64;1\"p\x1b\\");
}
