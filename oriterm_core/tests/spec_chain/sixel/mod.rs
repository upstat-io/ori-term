//! Sixel state-machine spec-conformance tests (§12.1).
//!
//! Drives every sixel DCS-level operator end-to-end through the
//! `SpecHarness` → `Term` → `SixelParser` seam. Each test names the
//! catalog row ID(s) it verifies from `plans/spec-conformance/catalog/sixel.md`.

pub mod grid_integration;
pub mod invariants;
pub mod state_machine;
