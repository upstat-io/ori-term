//! Spec_chain conversion note for the tack `tools_menu_inventory` family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/tools_menu_inventory/mod.rs`,
//! this family is an **inventory sentinel**: it pins the empirical
//! menu graph of tack v1.08's `t) tools` sub-menu (keys `s/g/c/h/e/r/p/i/u/d/q/?`)
//! and classifies each entry as `Scenario` / `DelegatedToSection` /
//! `ExcludedInteractive` / `MenuMeta`. It does not emit any protocol
//! bytes of its own — every tool sub-test is covered by a dedicated
//! scenario family (`status_reports`, `sgr_modes`, `character_sets`,
//! `enq_ack`).
//!
//! # Catalog rows verified
//!
//! None. An inventory sentinel has no protocol-layer semantics.
//!
//! # Why the module file exists
//!
//! Declared as a stub module so the per-family conversion map in
//! `mod.rs` is complete and self-documenting: future readers searching
//! for "where is tack `tools_menu_inventory` converted?" land here and
//! see immediately that the family was deliberately left without a
//! protocol test (with the reason). Without this file, the absence
//! would look like an oversight.

#[test]
fn tools_menu_inventory_contributes_zero_protocol_rows() {
    // No assertion body needed — the purpose of this test is to
    // document the deliberate zero-row status via its presence + name
    // and force CI to re-run this file (keeping the documentation
    // alive under rustdoc + test harness discovery) if any future
    // commit renames the tools_menu_inventory family.
}
