//! Visual verification chain harness (rungs 5-8).
//!
//! Wraps `CoreSpecHarness` (headless, rungs 1-4 in `oriterm_test_support`)
//! with GPU infrastructure for visual rungs: frame-input assembly (rung 5),
//! GPU instance buffer observation (rung 6), texture render (rung 7), and
//! golden image comparison (rung 8).
//!
//! Lives under `oriterm/src/gpu/visual_regression/` because rungs 5-8 need
//! `pub(super)` access to `headless_env_with_hinting`, `render_to_pixels`,
//! and `compare_with_reference`. See Section 04.3b crate boundary decision.

pub mod observers;
pub mod visual_harness;

#[cfg(test)]
mod pilots;
#[cfg(test)]
mod tests;
