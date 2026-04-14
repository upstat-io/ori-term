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
//! land here.

#[test]
fn enq_ack_contributes_zero_protocol_rows_pending_bug_08_6() {
    // Presence-documents the deliberate zero-row status and the
    // blocker (BUG-08-6). Presence in the test run keeps this file
    // part of the rustdoc corpus so the blocker reference stays
    // discoverable via `cargo doc` and file search.
}
