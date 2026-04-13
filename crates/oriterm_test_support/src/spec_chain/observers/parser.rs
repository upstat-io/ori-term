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
            // OSC has no final byte or intermediates — match on first
            // param as the command number. The expected action char
            // represents the OSC command (e.g. '0' for OSC 0 title set).
            if params.is_empty() {
                return false;
            }
            let first = &params[0];
            if first.is_empty() {
                return false;
            }
            // The first param is the OSC command number as ASCII bytes.
            // Match against expected.action as u8.
            first[0] == expected.action as u8
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
