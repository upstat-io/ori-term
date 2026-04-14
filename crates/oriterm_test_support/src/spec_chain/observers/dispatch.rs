//! Dispatch rung observer (rung 2).
//!
//! Asserts that the expected semantic `Handler` method was called by
//! checking `outcome.dispatched_calls` for an entry with the expected
//! method name.

use crate::spec_chain::api::{RungResult, SpecOutcome};
use crate::spec_chain::scenario::{DispatchExpectation, RungName};

/// Observe the dispatch rung: does `outcome.dispatched_calls` contain
/// an entry with the expected method name?
pub fn observe_dispatch(outcome: &SpecOutcome, expected: &DispatchExpectation) -> RungResult {
    let found = outcome
        .dispatched_calls
        .iter()
        .any(|call| call.method == expected.method);

    if found {
        RungResult::pass(RungName::Dispatch)
    } else {
        let methods: Vec<&str> = outcome.dispatched_calls.iter().map(|c| c.method).collect();
        RungResult::fail(
            RungName::Dispatch,
            format!(
                "expected handler method '{}' not found; got: {:?}",
                expected.method, methods
            ),
        )
    }
}
