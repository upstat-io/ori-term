//! Parser rung observer (rung 1).
//!
//! Asserts that the parser tokenized the expected sequence by checking
//! `outcome.perform_actions` for a `PerformAction` matching the expected
//! category, intermediates, and final byte. Supports CSI, ESC, Execute,
//! Print, DCS (Hook), OSC, and APC action types.

use vte::ansi::PerformAction;

use crate::spec_chain::api::{RungResult, SpecOutcome};
use crate::spec_chain::scenario::{ParserExpectation, RungName};

/// Observe the parser rung: does `outcome.perform_actions` contain an
/// entry matching the expected action char, params, and intermediates?
pub fn observe_parser(outcome: &SpecOutcome, expected: &ParserExpectation) -> RungResult {
    let found = outcome.perform_actions.iter().any(|action| match action {
        // CSI and DCS (Hook) share the same matching logic: action char +
        // intermediates + flattened params.
        PerformAction::CsiDispatch {
            params,
            intermediates,
            action,
            ..
        }
        | PerformAction::Hook {
            params,
            intermediates,
            action,
            ..
        } => {
            *action == expected.action
                && intermediates.as_slice() == expected.intermediates
                && params_match(params, expected.params)
        }
        PerformAction::EscDispatch {
            intermediates,
            byte,
            ..
        } => *byte == expected.action as u8 && intermediates.as_slice() == expected.intermediates,
        PerformAction::Execute { byte } => {
            *byte == expected.action as u8 && expected.intermediates.is_empty()
        }
        PerformAction::Print { c } => *c == expected.action && expected.intermediates.is_empty(),
        PerformAction::OscDispatch { params, .. } => {
            // OSC has no final byte — match on the full first param as
            // the command number (ASCII digits). The expected.params
            // carries the command number as a u16 value for comparison.
            if params.is_empty() || expected.params.is_empty() {
                return false;
            }
            let first = &params[0];
            // Parse the ASCII command number from the first param.
            let cmd_str = std::str::from_utf8(first).unwrap_or("");
            let cmd_num: u16 = cmd_str.parse().unwrap_or(u16::MAX);
            cmd_num == expected.params[0]
        }
        // APC: not matchable via ParserExpectation (no structured params).
        // APC-based sequences are observed at the dispatch rung instead.
        PerformAction::ApcStart
        | PerformAction::ApcEnd
        | PerformAction::ApcPut { .. }
        | PerformAction::Put { .. }
        | PerformAction::Unhook => false,
    });

    if found {
        RungResult::pass(RungName::Parser)
    } else {
        RungResult::fail(
            RungName::Parser,
            format!(
                "no PerformAction matching action='{}', params={:?}, intermediates={:?}; \
                 got {} actions: {:?}",
                expected.action,
                expected.params,
                expected.intermediates,
                outcome.perform_actions.len(),
                outcome.perform_actions
            ),
        )
    }
}

/// Check if CSI/DCS params match the expected flat params slice.
///
/// Params are `Vec<Vec<u16>>` (subparam groups). The expected
/// params are a flat `&[u16]` — we flatten and compare.
fn params_match(actual: &[Vec<u16>], expected: &[u16]) -> bool {
    let flat: Vec<u16> = actual.iter().flat_map(|sub| sub.iter().copied()).collect();
    flat == expected
}
