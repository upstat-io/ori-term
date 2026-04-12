//! Doc-only stub for tack's `n) run standard tests` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::Duplicate` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//! Covered by: `x, a, c, m, p (run standard tests sequences each individual test)`.
//!
//! # Why this is a duplicate, not a separate scenario
//!
//! Pressing `n` from the begin-testing menu runs every test in
//! sequence (`x` modes-and-glitches, `a` ACS / SGR, `c` color,
//! `m` cursor movement, `p` padding+strings) interactively, with
//! `Press space to continue` prompts between each. The aggregate
//! captured grid would be a concatenation of every test's output
//! and the framework would have to reconstruct the per-test
//! boundaries — strictly weaker than running each test as its
//! own scenario.
//!
//! # Where the equivalent coverage lives
//!
//! Each component test has its own dedicated wrapper that drives
//! tack with a precise menu_path bypassing the `n) run standard
//! tests` sequencer:
//! - `oriterm_core/tests/tack/test_menu/modes.rs` (Section 04 +
//!   05.1 phase scenarios) — covers `x`.
//! - `oriterm_core/tests/tack/test_menu/acs.rs` and
//!   `graphic_rendition.rs` (Section 05.2) — cover `a`.
//! - `oriterm_core/tests/tack/test_menu/color.rs` (Section 05.3)
//!   — covers `c`.
//! - `oriterm_core/tests/tack/test_menu/cursor_movement.rs`
//!   (Section 05.4) — covers `m`.
//! - `oriterm_core/tests/tack/test_menu/padding.rs` (Section
//!   05.4b) — covers `p`.
//!
//! Per-test isolation is stronger than the `n)` sequencer because
//! each test runs in a fresh tack invocation with no inter-test
//! state leakage and no `Press space to continue` prompts to
//! synchronize on.
