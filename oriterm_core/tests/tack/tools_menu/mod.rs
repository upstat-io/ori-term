//! Tack `t) tools` submenu scenarios.
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
//! - `status_reports_inventory` (06.0.b) + `status_reports` (06.1) —
//!   nested sub-submenu walker for `s) ANSI status reports`.
//! - `sgr_modes` (06.2) — 80-mode SGR table capture.
//! - `character_sets` (06.3) — DEC special graphics G1 bank table.
//! - `enq_ack` (06.4) — ENQ/ACK handshake success-path capture.
//! - 06.6 doc-only stubs for the interactive tools (`echo_tool`,
//!   `reply_tool`, `hex_output`, `change_debug_level`,
//!   `performance_testing`, `send_reset_init`). Each carries a
//!   module rustdoc explaining why the corresponding tack tool is
//!   excluded from end-to-end automation.

pub mod change_debug_level;
pub mod character_sets;
pub mod echo_tool;
pub mod enq_ack;
pub mod hex_output;
pub mod performance_testing;
pub mod reply_tool;
pub mod send_reset_init;
pub mod sgr_modes;
pub mod status_reports;
pub mod status_reports_inventory;
pub mod tools_menu_inventory;
