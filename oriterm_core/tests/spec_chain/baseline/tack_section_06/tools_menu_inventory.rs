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
//!
//! No `#[test]` functions live here — empty-body tests violate
//! §Test Hygiene Rule 1 ("every test file
//! must contain at least one assertion"). The module's `//!` rustdoc
//! is the documentation artifact; `mod.rs` pulls this file into the
//! crate's module tree so the rustdoc is discoverable via `cargo doc`.
