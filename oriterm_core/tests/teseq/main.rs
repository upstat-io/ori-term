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
//! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`

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
