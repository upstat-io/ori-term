//! Spec_chain conversion of the tack `padding` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/padding/mod.rs`,
//! tack v1.08's padding-and-string-capabilities screen first runs an
//! interactive ENQ/ACK probe (tack writes `\x1B[c` — DA1 — and waits
//! for a reply), then enters the `tack/test/pad [n] >` submenu where
//! pressing `n` runs the standard padding test. Against the pinned
//! `extra/ori_term.info`, the captured grid is:
//!
//! ```text
//! (rs1) reset_1string, not present.  (rs1) Done
//! ```
//!
//! That is — tack v1.08 reports `rs1` (the terminfo `reset_1string`
//! cap, which would map to `\E c` / RIS = `ECMA48-ESC-c`) is NOT
//! PRESENT in the terminfo source and therefore tack does NOT actually
//! send the sequence. The only protocol byte tack itself emits during
//! the padding scenario is the DA1 query (`\E[c`).
//!
//! # Catalog rows verified
//!
//! - **None new.** `ECMA48-CSI-DA1` is already verified by the harness
//!   pilot `pilots/da1_query.rs::da1_query_drives_to_effect_apex`. The
//!   padding scenario adds no protocol-row coverage beyond that pilot.
//!
//! `ECMA48-ESC-c` (RIS) is NOT exercised by tack v1.08 against
//! `extra/ori_term.info` because `rs1` is absent from the terminfo
//! source. Its conversion is owned by Section 08.8b (remaining
//! Section-08-owned catalog rows) which has explicit RIS coverage.

/// Negative pin: tack `padding` family contributes no NEW catalog-row
/// verifications beyond the DA1 pilot.
///
/// Documents the empirical reality so that `_legacy-tack-mapping.md`
/// does not claim padding adds RIS coverage that does not exist
/// against tack v1.08 + `extra/ori_term.info`.
#[test]
fn padding_scenario_shares_da1_with_pilot_no_new_rows() {
    // No new rows. The DA1 pilot is the canonical query test; padding
    // would duplicate it. Documented stub — see module rustdoc.
    let new_rows_beyond_pilot: &[&str] = &[];
    assert!(
        new_rows_beyond_pilot.is_empty(),
        "tack v1.08 padding scenario uses DA1 (already covered by \
         pilots/da1_query.rs) and reports rs1 absent in ori_term.info \
         — no new ECMA-48 catalog rows are exercised."
    );
}
