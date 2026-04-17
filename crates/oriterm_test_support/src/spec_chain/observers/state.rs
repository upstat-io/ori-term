//! State rung observer (rung 3a).
//!
//! Asserts that the terminal's final state matches expected values
//! (cursor position, etc.). Takes a `Term` reference alongside the
//! expectation since terminal state is not serialized into `SpecOutcome`.

use oriterm_core::Term;
use oriterm_core::effect::sink::QueueingEffectSink;

use crate::spec_chain::api::RungResult;
use crate::spec_chain::scenario::{RungName, StateExpectation};

/// Observe the state rung: does the terminal's cursor position match?
///
/// Takes the `Term` directly — state is read from the live terminal,
/// not from `SpecOutcome`. This is the pragmatic approach; a full
/// `GridSnapshot` serialization can be added when more state fields
/// are needed (renderable cells, mode flags, etc.).
pub fn observe_state(term: &Term<QueueingEffectSink>, expected: &StateExpectation) -> RungResult {
    let cursor = term.grid().cursor();
    let cursor_line = cursor.line();
    let cursor_col = cursor.col().0;

    if let Some(exp_line) = expected.cursor_line {
        if cursor_line != exp_line {
            return RungResult::fail(
                RungName::State,
                format!("cursor line: expected {exp_line}, got {cursor_line}"),
            );
        }
    }

    if let Some(exp_col) = expected.cursor_col {
        if cursor_col != exp_col {
            return RungResult::fail(
                RungName::State,
                format!("cursor col: expected {exp_col}, got {cursor_col}"),
            );
        }
    }

    RungResult::pass(RungName::State)
}
