//! Tests for VTE handler (Print, Execute, and CSI sequences).
//!
//! Feed raw bytes through `vte::ansi::Processor` → `Term<RecordingListener>`
//! and verify grid state and events.
//!
//! `RecordingListener`, `term_with_recorder`, `term_with_recorder_sized`,
//! and `feed` were promoted into the sibling `super::test_helpers`
//! module by the 06.5.a refactor so the same helpers can be reused
//! by `super::tack_cap_xcheck::tests` (Section 06.5's direct-VTE
//! cap-xcheck matrix). See that module's rustdoc for the rationale.
//!
//! Test file was split into topical submodules for navigability
//! (spec-conformance plan Section 09.0). `tests.rs` → `tests/` directory
//! module. No test-body changes — only module paths.

mod core;
mod dcs;
mod esc;
mod image;
mod modes;
mod osc;
mod private_modes_keyboard;
mod private_modes_mouse;
mod private_modes_screen;
mod private_modes_sync;
mod private_modes_theme;
mod sgr;
mod status_reports;
