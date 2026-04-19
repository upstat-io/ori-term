//! Spec-conformance verification chain integration tests.
//!
//! Each pilot scenario drives a sequence through the `SpecHarness`
//! verification chain and asserts per-rung observations. These are
//! integration tests (not unit tests) because they exercise the full
//! VTE → Term → Effect pipeline end-to-end.
//!
//! # Commands
//!
//! - Run: `cargo test -p oriterm_core --test spec_chain`

mod baseline;
mod osc;
mod pilots;
mod private_modes;
