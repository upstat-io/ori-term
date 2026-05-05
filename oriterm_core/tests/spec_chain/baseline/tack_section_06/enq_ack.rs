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
//! `status: missing` in the catalog pending **``**
//!
//! > `[][low]` **ENQ/Answerback not implemented** — Repro
//! > vttest menu 6 sub-item 1 (answerback test). No response
//! > displayed. Detail: ENQ (0x05) control code not handled in VTE
//! > C0 dispatcher. WezTerm implements it (defaults to empty
//! > string), Alacritty does not. Would need: (1) add ENQ to VTE C0
//! > dispatch, (2) add handler method to Handler trait, (3)
//! > implement in Term.
//! > — `bug-tracker/section-08-core-terminal.md:33`
//!
//! DA1 round-trip coverage (the actual byte sequence tack v1.08
//! sends as `tty_ENQ` because `u9=\E[c`) is already driven through
//! the DA1 pilot, so the enq_ack family contributes zero NEW
//! spec_chain coverage here — and cannot contribute `ECMA48-C0-ENQ`
//! coverage until is resolved.
//!
//! # Why the module file exists
//!
//! Declared as a stub module so the per-family conversion map in
//! `mod.rs` is complete: a future reader searching for "where is
//! tack `enq_ack` converted?" lands here and sees the blocked-on-bug
//! classification immediately. When is fixed, a
//! spec_chain test driving `ECMA48-C0-ENQ` (parser Execute 0x05,
//! dispatch to a new `answerback`/`enquiry` method, effect
//! `PtyWriteKind::Other` with the empty answerback byte string) will
//! land here, replacing the regression guard below.
//!
//! The test in this module is a **load-bearing regression guard**
//! against, not a tautology. When the catalog row
//! `ECMA48-C0-ENQ` is flipped from `status: missing` to any other
//! value (i.e. someone implemented ENQ), the assertion below will
//! fail, forcing whoever fixes to open this file and
//! replace the guard with the real spec_chain test the module
//! rustdoc describes. Without this pin, could be silently
//! closed without the corresponding spec_chain coverage ever landing.

/// Pins: the ECMA-48 C0 ENQ catalog row remains `missing` until
/// lands. Reads the catalog markdown directly and asserts the ENQ row's
/// verification-status column still reads `missing`. When is
/// resolved, the catalog row's status will flip (to `verified` or similar),
/// this assertion will fail, and the failing test reminds the implementer
/// that the spec_chain coverage for this family needs to land here — not
/// just the implementation elsewhere.
///
/// Anchor: / HYG-13.1-011.
#[test]
fn ecma48_c0_enq_catalog_row_still_missing() {
    // The spec-conformance catalog lives in the wrapper repo. When the test
    // runs from a standalone term_repo checkout (no wrapper present), the
    // file is absent — graceful skip
    // Skip Protocol`. Path discovery via the SSOT helper introduced in
    //; never reintroduce ad-hoc `manifest_dir.parent()` arithmetic.
    let Some(catalog_dir) = oriterm_test_support::paths::catalog_dir() else {
        eprintln!("SKIP: ECMA-48 catalog not present (term_repo running without wrapper)");
        return;
    };
    let catalog_path = catalog_dir.join("ecma-48.md");
    // Wrapper is confirmed present (catalog_dir is Some); a read failure here
    // is a real I/O error, not a graceful-skip case. Propagate per
    // Handling at Boundaries`.
    let catalog = std::fs::read_to_string(&catalog_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", catalog_path.display()));

    let row = catalog
        .lines()
        .find(|l| l.starts_with("| ECMA48-C0-ENQ "))
        .expect("ECMA48-C0-ENQ row must exist in catalog/ecma-48.md");

    assert!(
        row.contains("| missing |"),
        "ECMA48-C0-ENQ is no longer marked `missing` in the catalog — \
 has been fixed. Replace this regression guard with a \
 real spec_chain test driving the ENQ probe, per the module \
 rustdoc. Row line:\n {row}"
    );
}
