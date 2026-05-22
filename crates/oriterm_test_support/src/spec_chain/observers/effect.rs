//! Effect rung observer (rung 3b).
//!
//! Asserts that the expected `Effect` was emitted (or that NO effect
//! was emitted, depending on the expectation). Supports matching on
//! both the top-level family and optional sub-variant (e.g.
//! `PtyWriteKind::DeviceAttribute`).

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};

use crate::spec_chain::api::{RungResult, SpecOutcome};
use crate::spec_chain::scenario::{EffectExpectation, RungName};

/// Observe the effect rung: does `outcome.effects_emitted` contain
/// an effect matching the expected variant and optional sub-variant?
pub fn observe_effect(outcome: &SpecOutcome, expected: &EffectExpectation) -> RungResult {
    let found = outcome.effects_emitted.iter().any(|eff| {
        let (family, sub) = effect_names(eff);
        if family != expected.variant {
            return false;
        }
        match expected.sub_variant {
            Some(expected_sub) => sub == expected_sub,
            None => true,
        }
    });

    if found {
        RungResult::pass(RungName::Effect)
    } else {
        let got: Vec<String> = outcome
            .effects_emitted
            .iter()
            .map(|eff| {
                let (family, sub) = effect_names(eff);
                if sub.is_empty() {
                    family.to_string()
                } else {
                    format!("{family}::{sub}")
                }
            })
            .collect();
        let expected_desc = match expected.sub_variant {
            Some(sub) => format!("{}::{}", expected.variant, sub),
            None => expected.variant.to_string(),
        };
        RungResult::fail(
            RungName::Effect,
            format!("expected effect '{expected_desc}' not found; got: {got:?}"),
        )
    }
}

/// Extract the family name and sub-variant name from an `Effect`.
fn effect_names(eff: &Effect) -> (&'static str, &'static str) {
    match eff {
        Effect::Pty(pty) => ("Pty", pty_sub_name(pty)),
        Effect::Host(_) => ("Host", ""),
        Effect::HostRequest(_) => ("HostRequest", ""),
        Effect::Ui(_) => ("Ui", ""),
        Effect::Presentation(_) => ("Presentation", ""),
        Effect::ImageDecode(_) => ("ImageDecode", ""),
    }
}

/// Extract the `PtyWriteKind` name from a `PtyEffect`.
fn pty_sub_name(pty: &PtyEffect) -> &'static str {
    match pty {
        PtyEffect::Write { kind, .. } => match kind {
            PtyWriteKind::DeviceAttribute => "DeviceAttribute",
            PtyWriteKind::CursorReport => "CursorReport",
            PtyWriteKind::DeviceStatus => "DeviceStatus",
            PtyWriteKind::ModeReport => "ModeReport",
            PtyWriteKind::StatusString => "StatusString",
            PtyWriteKind::ImageProtocolReply => "ImageProtocolReply",
            PtyWriteKind::MouseEvent => "MouseEvent",
            PtyWriteKind::KeyboardEvent => "KeyboardEvent",
            PtyWriteKind::FocusEvent => "FocusEvent",
            PtyWriteKind::ChecksumReport => "ChecksumReport",
            PtyWriteKind::GraphicsAttributeReport => "GraphicsAttributeReport",
            PtyWriteKind::Answerback => "Answerback",
            PtyWriteKind::Other => "Other",
        },
    }
}
