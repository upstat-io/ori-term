//! Spec_chain conversion note for the tack `status_reports` family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports/mod.rs`,
//! this family is a **helper module**: it provides grid-parsing
//! utilities (`extract_response_for_label`, `is_primary_da_response`,
//! etc.) that the `status_reports_inventory` scenario calls internally
//! to extract DA/DSR/DECRQSS responses from tack's captured walk grid.
//!
//! # Catalog rows verified
//!
//! None. The helpers do not emit any protocol bytes — they only parse
//! already-captured grid text produced by the status_reports_inventory
//! scenario. Their sibling `#[cfg(test)] mod tests;` in the
//! oriterm_test_support crate covers correctness of the extraction
//! logic directly.
//!
//! All five DA/DSR catalog rows that tack's status-reports probes
//! exercise are driven to `verified` in `status_reports_inventory.rs`:
//! `ECMA48-CSI-DA1` (via the existing pilot), `ECMA48-CSI-DA2`,
//! `ECMA48-CSI-DA3`, `ECMA48-CSI-DSR-5`, `ECMA48-CSI-DSR-6`.
//!
//! # Why the module file exists
//!
//! Declared as a stub module so the per-family conversion map in
//! `mod.rs` is complete: a future reader searching for "where is tack
//! `status_reports` converted?" lands here and sees the helper-only
//! classification immediately.
//!
//! No `#[test]` functions live here — empty-body tests violate
//! `.claude/rules/tests.md` §Test Hygiene Rule 1. The `//!` rustdoc
//! is the artifact; `mod.rs` pulls this file into the module tree.
