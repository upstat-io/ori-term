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
            // OSC matching: action is ignored (OSC has no final byte),
            // params are matched as flattened bytes. The first param's
            // first byte is used as the "action" for matching.
            if expected.params.is_empty() && !params.is_empty() {
                // Match any OSC if no specific params expected.
                true
            } else if let Some(first) = params.first() {
                // Match on first param byte as the "command number".
                !first.is_empty() && first[0] == expected.action as u8
            } else {
                false
            }
        }
        PerformAction::ApcStart | PerformAction::ApcEnd => {
            // APC start/end: no params, no intermediates.
            expected.params.is_empty() && expected.intermediates.is_empty()
        }
        PerformAction::ApcPut { .. } | PerformAction::Put { .. } | PerformAction::Unhook => false,
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
