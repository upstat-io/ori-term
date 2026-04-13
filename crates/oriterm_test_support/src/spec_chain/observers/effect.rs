//! Effect rung observer (rung 3b).
//!
//! Asserts that the expected `Effect` was emitted (or that NO effect
//! was emitted, depending on the expectation).

use oriterm_core::effect::Effect;

use crate::spec_chain::api::{RungResult, SpecOutcome};
use crate::spec_chain::scenario::{EffectExpectation, RungName};

/// Observe the effect rung: does `outcome.effects_emitted` contain
/// an effect matching the expected variant?
pub fn observe_effect(outcome: &SpecOutcome, expected: &EffectExpectation) -> RungResult {
    let found = outcome.effects_emitted.iter().any(|eff| {
        let variant_name = match eff {
            Effect::Pty(_) => "Pty",
            Effect::Host(_) => "Host",
            Effect::HostRequest(_) => "HostRequest",
            Effect::Ui(_) => "Ui",
            Effect::Presentation(_) => "Presentation",
        };
        variant_name == expected.variant
    });

    if found {
        RungResult::pass(RungName::Effect)
    } else {
        let variants: Vec<&str> = outcome
            .effects_emitted
            .iter()
            .map(|eff| match eff {
                Effect::Pty(_) => "Pty",
                Effect::Host(_) => "Host",
                Effect::HostRequest(_) => "HostRequest",
                Effect::Ui(_) => "Ui",
                Effect::Presentation(_) => "Presentation",
            })
            .collect();
        RungResult::fail(
            RungName::Effect,
            format!(
                "expected effect variant '{}' not found; got: {:?}",
                expected.variant, variants
            ),
        )
    }
}
