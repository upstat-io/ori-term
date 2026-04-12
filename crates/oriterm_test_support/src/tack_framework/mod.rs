//! Scenario catalog framework for tack-driven conformance tests.
//!
//! Design rationale: semantic IDs, menu navigation as data,
//! deterministic `wait_for` synchronization, per-scenario parsers,
//! tokenized grid checks, pre-existing-anchor guard.
//!
//! # Scenarios module
//!
//! `pub mod scenarios;` (added in 04.1.b) is the catalog of `pub
//! const ScenarioSpec` values and per-scenario parsers consumed by
//! both text tests (`oriterm_core/tests/tack/`) and GPU goldens
//! (`oriterm/src/gpu/visual_regression/tack/`). Section 04 owns the
//! module skeleton; Sections 05-08 add submodules under it.
//!
//! # Internal dependencies
//!
//! [`crate::session::PtySession::send`] (and therefore the future
//! `TackNavigator::navigate`) calls `wait(300)` internally to drain
//! output before returning. The framework as a whole is "no fixed
//! sleeps in the navigator loop"; the 300 ms drain inside `send` is
//! the canonical post-write quiesce and is documented here so future
//! readers don't double-add it.
//!
//! `TackNavigator` (added in 04.2) uses
//! [`crate::session::PtySession::wait_for_any`] for the
//! primary+alternate anchor matching, NOT `catch_unwind` on
//! `wait_for_with_context` — panic-as-control-flow is banned.

pub mod cap_coverage;
pub mod navigator;
pub mod parser;
pub mod runner;
pub mod scenarios;
pub mod spec;

pub use cap_coverage::{
    ALL_CONTRIBUTIONS, CapCoverageContribution, covered_caps, exempt_caps, parse_declared_caps,
};
pub use navigator::TackNavigator;
pub use parser::tokens::{
    grid_find_field, grid_has_paren_token, grid_has_token, grid_line_starts_with,
};
pub use parser::{ScreenFacts, ScreenParserFn, default_parser};
pub use runner::{LiveSession, ScenarioOutcome, ScenarioRunner};
pub use spec::{
    MenuStep, PhaseSpec, ScenarioSpec, is_unverified_anchor, is_unverified_menu_key,
    unverified_anchor, unverified_menu_key,
};
