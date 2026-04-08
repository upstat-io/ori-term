//! Tack `t) tools` submenu scenarios — see
//! `plans/tack-conformance/section-06-tools-menu-scenarios.md`.
//!
//! This module holds the `#[test] fn` wrappers for Section 06. The
//! const `ScenarioSpec`/`PhaseSpec` values and per-scenario parsers
//! live in `oriterm_test_support::tack_framework::scenarios::*`. Each
//! file here is a thin wrapper that imports its consts and calls
//! `ScenarioRunner::run(...)`.
//!
//! # Submodule layout
//!
//! - `tools_menu_inventory` (06.0) — discovery + drift gate for the
//!   `t)` submenu key set.
//! - Future (06.0.b / 06.1-06.6): `status_reports_inventory`,
//!   `status_reports`, `sgr_modes`, `character_sets`, `enq_ack`, plus
//!   doc-only stubs for the interactive tools (echo/reply/hex/debug/
//!   perf/reset).

pub mod status_reports;
pub mod status_reports_inventory;
pub mod tools_menu_inventory;
