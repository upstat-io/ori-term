//! Teseq-based escape sequence conformance tests.
//!
//! Uses GNU teseq/reseq to author human-readable escape sequence scenarios,
//! feeds them through `Term<RecordedListener>`, and validates terminal state
//! against insta golden snapshots.
//!
//! Requires `reseq` installed (`sudo apt install teseq`).
//! Tests gracefully skip when reseq is unavailable.
//!
//! # Commands
//!
//! - Run: `cargo test -p oriterm_core --test teseq`
//! - Run (release): `cargo test -p oriterm_core --test teseq --release`
//! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`
//!
//! # Test Count
//!
//! Expected: **176 tests** (168 `.teseq` scenarios + 7 pure-Rust tests + 1
//! scenario-variant).
//!
//! Validate: `cargo test -p oriterm_core --test teseq -- --list 2>/dev/null |
//! grep -c '::.*test$'`
//!
//! # Creating a New Scenario
//!
//! 1. Write a `.teseq` file in the appropriate `scenarios/` subdirectory.
//!    Three line types: `|text|` (literal text, trailing `.` = LF appended),
//!    `. LABEL/^X` (C0 control), `: Esc [ params` (escape/CSI introducer).
//!    OSC content must be on `|text|` lines, not inline on `: Esc` lines.
//! 2. Optionally add a `.toml` sidecar (same name) for terminal config:
//!    `[terminal] cols = 132`, `[setup] pre_feed = ["\\x1b[?40h"]`, etc.
//! 3. Add a test function in the family module that calls `run_scenario("name")`.
//! 4. Run the test: `cargo test -p oriterm_core --test teseq -- name`.
//!    On first run, insta creates a new snapshot for review.
//! 5. Review and accept: `cargo insta review` or `INSTA_UPDATE=1 cargo test ...`.
//!
//! # Registering a New Family Module
//!
//! 1. Create `oriterm_core/tests/teseq/family_name.rs` with a `run_scenario`
//!    function that includes `if !reseq_available() { return; }` as the first
//!    statement.
//! 2. Add `mod family_name;` to this file under the appropriate section comment.
//! 3. Create the scenario directory: `scenarios/family_dir/`.
//!
//! # Future Scenario Priorities
//!
//! | Sequence/Family | Bug Risk | Interop | Effort | Notes |
//! |-----------------|----------|---------|--------|-------|
//! | Tab stops (HTS/TBC) | High | High | Small | Implemented, unit-tested, no teseq coverage |
//! | DECSTR (CSI ! p) | High | Medium | Small | Soft reset, interacts with many modes |
//! | DCS (DECRQSS) | Medium | High | Medium | Response validation via PtyWrite, already impl |
//! | OSC color set (4/10/11) | Medium | Medium | Small | Only queries tested, not set operations |
//! | DECSCL | Low | Medium | Small | Conformance level, rarely used in practice |
//! | Standalone CJK placement | Low | Medium | Medium | Partial coverage via irm/wrap wide-char tests |
//! | DCS (Sixel, Kitty image) | Low | Low | Large | Complex protocols, separate plan likely |
//!
//! See `plans/teseq-conformance/section-07-verification.md` for full gap
//! analysis.

// Harness utilities are built in Section 01 and consumed by Sections 02-07.
// Suppress dead_code warnings for the incrementally-built test harness.
#![allow(dead_code)]

mod harness;

// Family modules (Section 02).
mod c0;
mod csi_cursor;
mod csi_erase;
mod csi_insert_delete;
mod esc;

// Family modules (Section 03).
mod csi_reports;

// Family modules (Section 04).
mod mode_interactions;

// Family modules (Section 05).
mod sgr;

// Family modules (Section 06).
mod osc;
mod workflows;
