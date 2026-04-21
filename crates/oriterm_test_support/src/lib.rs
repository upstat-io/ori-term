//! Shared PTY/Term/VTE test driver for oriterm conformance suites.
//!
//! This crate is a dev-time helper, never published. It provides:
//!
//!   - [`PtySession`] — the canonical PTY/Term/VTE plumbing used by
//!     `oriterm_core/tests/vttest/`, `oriterm/src/gpu/visual_regression/vttest/`,
//!     and (Section 04+) `oriterm_core/tests/tack/`.
//!   - [`TerminfoEnv`] — runtime tic-compiled `extra/ori_term.info`
//!     bound to a child process via `TERM`/`TERMINFO`/`TERMINFO_DIRS`,
//!     so tack and infocmp consult `ori_term`'s pinned terminfo entry
//!     instead of the host's `xterm-256color`.
//!
//! Before this crate existed, the PTY/Term/VTE plumbing was duplicated
//! byte-for-byte between two `VtTestSession` definitions.

pub mod catalog;
pub mod fixtures;
pub mod session;
pub mod spec_chain;
pub mod tack_framework;
pub mod terminfo;

pub use session::{
    PtyResponder, PtySession, infocmp_available, tack_available, tic_available, tool_available,
    vttest_available,
};
pub use tack_framework::{
    LiveSession, MenuStep, PhaseSpec, ScenarioOutcome, ScenarioRunner, ScenarioSpec, ScreenFacts,
    ScreenParserFn, TackNavigator, default_parser, grid_find_field, grid_has_paren_token,
    grid_has_token, grid_line_starts_with, is_unverified_anchor, is_unverified_menu_key,
    unverified_anchor, unverified_menu_key,
};
pub use terminfo::{
    TerminfoEnv, TerminfoVariant, decode_terminfo_string, infocmp_dump, infocmp_query,
    infocmp_respects_terminfo_env,
};
