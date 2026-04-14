//! Spec_chain conversions for tack section 06 (`t) tools`) scenarios.
//!
//! Tack section 06 exercises the tools sub-menu: status reports
//! (DA1/DA2/DA3/DSR/DECRQSS), SGR modes, ANSI character sets, and
//! ENQ/ACK handshake. Each scenario family is converted here into
//! standalone protocol-level spec_chain tests driving the catalog
//! row(s) the scenario exercises directly through `SpecHarness` — no
//! PTY, no menu navigation, no tack runtime dependency.
//!
//! # Per-family conversion map
//!
//! | Scenario family            | Catalog rows exercised                                                 | File                          |
//! |---                         |---                                                                     |---                            |
//! | `tools_menu_inventory`     | (none — menu inventory sentinel, no protocol bytes)                    | `tools_menu_inventory.rs`     |
//! | `status_reports_inventory` | `ECMA48-CSI-DA2`, `ECMA48-CSI-DA3`, `ECMA48-CSI-DSR-5`, `ECMA48-CSI-DSR-6` | `status_reports_inventory.rs` |
//! | `status_reports`           | (none — grid-parsing helpers; zero new rows)                           | `status_reports.rs`           |
//! | `sgr_modes`                | `ECMA48-SGR-0`, `ECMA48-SGR-1`, `ECMA48-SGR-4`, `ECMA48-SGR-7`         | `sgr_modes.rs`                |
//! | `character_sets`           | `ECMA48-ESC-0`, `ECMA48-ESC-B`, `ECMA48-C0-SO`, `ECMA48-C0-SI`         | `character_sets.rs`           |
//! | `enq_ack`                  | (none — blocked on `BUG-08-6`; `ECMA48-C0-ENQ` remains `missing`)      | `enq_ack.rs`                  |
//!
//! DA1 (`ECMA48-CSI-DA1`) is covered by the pre-existing DA1 pilot at
//! `oriterm_core/tests/spec_chain/pilots/da1_query.rs` — tack's
//! status_reports_inventory `da1` sub-test reuses that coverage rather
//! than duplicating it here.
//!
//! Empirical reality (tack v1.08, captured 2026-04-08): tack section
//! 06's tools sub-menu is NOT uniformly sequential — several
//! sub-tests are batched together or emit their probes as part of
//! multi-step handshakes. The scenario framework captures this in
//! inventory modules (`tools_menu_inventory`, `status_reports_inventory`)
//! that pin the discovered menu graph; the spec_chain conversion
//! below targets the protocol bytes each probe sends, not the
//! captured screen text.

mod character_sets;
mod enq_ack;
mod sgr_modes;
mod status_reports;
mod status_reports_inventory;
mod tools_menu_inventory;
