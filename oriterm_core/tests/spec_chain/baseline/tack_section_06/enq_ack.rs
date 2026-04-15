//! Spec_chain conversion note for the tack `enq_ack` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/enq_ack/mod.rs`,
//! tack's `t -> u) test ENQ/ACK (DA1) handshake` test sends the `u9`
//! cap (declared as `\E[c` in `extra/ori_term.info:115`, i.e. a DA1
//! query — NOT a bare `ENQ` byte) and waits for a matching `u8` ACK
//! regex. The success path exercises DA1 round-trip, which is already
//! covered by `oriterm_core/tests/spec_chain/pilots/da1_query.rs`.
//!
//! # Catalog rows verified
//!
//! None. The scenario family's only distinctive catalog row is
//! `ECMA48-C0-ENQ` (`0x05`, Answerback / enquiry), which is currently
//! `status: missing` in the catalog pending **`BUG-08-6`**:
//!
//! > `[BUG-08-6][low]` **ENQ/Answerback not implemented** — Repro:
//! > vttest menu 6 sub-item 1 (answerback test). No response
//! > displayed. Detail: ENQ (0x05) control code not handled in VTE
//! > C0 dispatcher. WezTerm implements it (defaults to empty
//! > string), Alacritty does not. Would need: (1) add ENQ to VTE C0
//! > dispatch, (2) add handler method to Handler trait, (3)
//! > implement in Term.
//! > — `plans/bug-tracker/section-08-core-terminal.md:33`
//!
//! DA1 round-trip coverage (the actual byte sequence tack v1.08
//! sends as `tty_ENQ` because `u9=\E[c`) is already driven through
//! the DA1 pilot, so the enq_ack family contributes zero NEW
//! spec_chain coverage here — and cannot contribute `ECMA48-C0-ENQ`
//! coverage until BUG-08-6 is resolved.
//!
//! # Why the module file exists
//!
//! Declared as a stub module so the per-family conversion map in
//! `mod.rs` is complete: a future reader searching for "where is
//! tack `enq_ack` converted?" lands here and sees the blocked-on-bug
//! classification immediately. When BUG-08-6 is fixed, a
//! spec_chain test driving `ECMA48-C0-ENQ` (parser Execute 0x05,
//! dispatch to a new `answerback`/`enquiry` method, effect
//! `PtyWriteKind::Other` with the empty answerback byte string) will
//! land here, replacing the regression guard below.
//!
//! The test in this module is a **load-bearing regression guard**
//! against BUG-08-6, not a tautology. When the catalog row
//! `ECMA48-C0-ENQ` is flipped from `status: missing` to any other
//! value (i.e. someone implemented ENQ), the assertion below will
//! fail, forcing whoever fixes BUG-08-6 to open this file and
//! replace the guard with the real spec_chain test the module
//! rustdoc describes. Without this pin, BUG-08-6 could be silently
//! closed without the corresponding spec_chain coverage ever landing.

/// Regression guard: ECMA48-C0-ENQ catalog row must stay `missing` until BUG-08-6 is fixed.
///
/// Reads the catalog markdown directly and asserts the ENQ row's
/// verification-status column still reads `missing`. When BUG-08-6
/// is resolved, the catalog row's status will flip (to `verified`
/// or similar), this assertion will fail, and the failing test
/// reminds the implementer that the spec_chain coverage for this
/// family needs to land here — not just the implementation
/// elsewhere.
#[test]
fn ecma48_c0_enq_catalog_row_still_missing_pending_bug_08_6() {
    let catalog = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../plans/spec-conformance/catalog/ecma-48.md"
    ))
    .expect("catalog/ecma-48.md must exist");

    let row = catalog
        .lines()
        .find(|l| l.starts_with("| ECMA48-C0-ENQ "))
        .expect("ECMA48-C0-ENQ row must exist in catalog/ecma-48.md");

    assert!(
        row.contains("| missing |"),
        "ECMA48-C0-ENQ is no longer marked `missing` in the catalog — \
         BUG-08-6 has been fixed. Replace this regression guard with a \
         real spec_chain test driving the ENQ probe, per the module \
         rustdoc. Row line:\n  {row}"
    );
}
