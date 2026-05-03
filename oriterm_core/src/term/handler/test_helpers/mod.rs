//! Test-only helpers shared between `handler::tests` and
//! `handler::tack_cap_xcheck::tests`.
//!
//! This module exists because Rust's `#[cfg(test)] mod tests;` sibling
//! files do NOT cross-share visibility — `handler::tests`'s private
//! items cannot be reached from `handler::tack_cap_xcheck::tests` even
//! when both are gated by `#[cfg(test)]`. The 06.5.a refactor extracts
//! `RecordingListener`, `term_with_recorder`, `term_with_recorder_sized`,
//! and the `feed` helper into this module so both consumers can `use
//! super::test_helpers::*` (or `super::super::test_helpers::*` from
//! a child like `tack_cap_xcheck/tests.rs`) and share a single
//! canonical implementation.
//!
//! After effect-cutover §01.3, the recording sink consumes
//! [`Effect`](crate::effect::Effect) directly via [`EffectSink`] (the
//! legacy `EventListener` shim was deleted). To preserve the existing
//! assertion contract used by ~70 in-crate handler tests, each captured
//! effect is rendered into a string that mirrors the original
//! `Event` `Debug` format (`Title(text)`, `ColorRequest(index)`, etc.)
//! via [`legacy_format_effect`]. This is a TEST-ONLY convenience: it
//! lives behind the same module gate as the recorder, has no production
//! consumers, and exists solely so the assertion strings the existing
//! tests grew up with remain meaningful after the migration.
//!
//! Visibility is `pub(super)` (i.e., visible from `handler::`) so
//! the sibling test modules under `handler::` can consume the
//! helpers, but the items remain invisible to consumers outside the
//! `handler` module tree.

use std::sync::{Arc, Mutex};

use vte::ansi::Processor;

use crate::effect::sink::EffectSink;
use crate::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PresentationEffect, PtyEffect,
    QueueingEffectSink, UiEffect,
};
use crate::term::Term;
use crate::theme::Theme;

/// Effect sink that records every pushed effect, formatted as a
/// legacy-`Event`-style string so historical assertion patterns
/// (`"Title(...)"`, `"ColorRequest(...)"`, `"ClipboardStore(...)"`)
/// remain valid after the effect-cutover deletion of `Event::*`
/// variants. See module-level documentation for the migration context.
///
/// The `Arc<Mutex<_>>` interior allows the sink to be cloned before
/// passing it into `Term::new` (which consumes the sink) while still
/// letting the test inspect the recorded strings afterwards via the
/// cloned handle.
#[derive(Clone, Default)]
pub(in crate::term::handler) struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingListener {
    /// Construct a fresh recording sink with an empty event log.
    pub(in crate::term::handler) fn new() -> Self {
        Self::default()
    }

    /// Snapshot the recorded events as a `Vec<String>` (each entry is
    /// the legacy-`Event`-style rendering of one captured `Effect`).
    pub(in crate::term::handler) fn events(&self) -> Vec<String> {
        self.events.lock().expect("lock poisoned").clone()
    }
}

impl EffectSink for RecordingListener {
    fn push(&self, effect: Effect) {
        if let Some(rendered) = legacy_format_effect(&effect) {
            self.events.lock().expect("lock poisoned").push(rendered);
        }
    }

    fn drain_into(&self, _out: &mut Vec<Effect>) {
        // Tests inspect the recorded strings via `events()`; raw effect
        // drainage is unused by this sink.
    }
}

/// Render an `Effect` into the legacy `Event` `Debug` format used by
/// pre-cutover handler tests. Returns `None` for variants that have
/// no historical `Event` counterpart and were never asserted on by
/// the test contract — keeping the noise floor low.
fn legacy_format_effect(effect: &Effect) -> Option<String> {
    fn clipboard_label(selection: ClipboardSelection) -> &'static str {
        match selection {
            ClipboardSelection::Clipboard => "Clipboard",
            ClipboardSelection::Primary | ClipboardSelection::Select => "Selection",
        }
    }

    Some(match effect {
        Effect::Host(HostEffect::Bell) => "Bell".to_string(),
        Effect::Host(HostEffect::UrgencyHint) => "UrgencyHint".to_string(),
        Effect::Host(HostEffect::TitleSet { value: Some(t) }) => format!("Title({t})"),
        Effect::Host(HostEffect::TitleSet { value: None }) => "ResetTitle".to_string(),
        Effect::Host(HostEffect::IconNameSet { value: Some(n) }) => format!("IconName({n})"),
        Effect::Host(HostEffect::IconNameSet { value: None }) => "ResetIconName".to_string(),
        Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
            format!("ClipboardStore({}, {data})", clipboard_label(*selection))
        }
        Effect::Host(HostEffect::CwdSet { cwd }) => format!("Cwd({cwd})"),
        Effect::Host(HostEffect::CommandComplete { duration }) => {
            format!("CommandComplete({duration:?})")
        }
        Effect::Host(HostEffect::ChildExit { code }) => format!("ChildExit({code})"),
        Effect::Host(HostEffect::DesktopNotification { .. })
        | Effect::Host(HostEffect::ClearPendingNotifications)
        | Effect::Host(HostEffect::AudioRequest(_))
        | Effect::Host(HostEffect::PrintRequest(_)) => return None,
        Effect::Pty(PtyEffect::Write { bytes, .. }) => {
            let s = String::from_utf8_lossy(bytes);
            format!("PtyWrite({s})")
        }
        Effect::Ui(UiEffect::CursorBlinkChanged { .. }) => "CursorBlinkingChange".to_string(),
        Effect::Ui(UiEffect::MouseCursorDirty) => "MouseCursorDirty".to_string(),
        Effect::HostRequest(HostRequest::ClipboardLoad { selection, .. }) => {
            format!("ClipboardLoad({})", clipboard_label(*selection))
        }
        Effect::HostRequest(HostRequest::ColorQuery { index, .. }) => {
            format!("ColorRequest({index})")
        }
        Effect::Presentation(PresentationEffect::Begin)
        | Effect::Presentation(PresentationEffect::Commit { .. })
        | Effect::Presentation(PresentationEffect::Abort { .. }) => return None,
    })
}

/// Create a `Term` with 24 lines, 80 columns, and a recording sink.
pub(in crate::term::handler) fn term_with_recorder() -> (Term<RecordingListener>, RecordingListener)
{
    term_with_recorder_sized(24, 80)
}

/// Create a `Term` with the given dimensions and a recording sink.
pub(in crate::term::handler) fn term_with_recorder_sized(
    lines: usize,
    cols: usize,
) -> (Term<RecordingListener>, RecordingListener) {
    let recorder = RecordingListener::new();
    let term = Term::new(lines, cols, 0, Theme::default(), recorder.clone());
    (term, recorder)
}

/// Create a `Term` with 24 lines, 80 columns, and a
/// `QueueingEffectSink` for direct `Effect` observation.
///
/// Use `term.effect_sink().drain_into(&mut effects)` to inspect
/// the structured `Effect` variants emitted by the handler.
pub(in crate::term::handler) fn term_with_effect_sink() -> Term<QueueingEffectSink> {
    Term::new(24, 80, 0, Theme::default(), QueueingEffectSink::new())
}

/// Feed raw bytes through the VTE processor into a handler.
///
/// Constructs a fresh `Processor` per call so test bodies are
/// stateless across `feed` calls. Tests that need a long-lived
/// processor (e.g., for multi-byte sequences split across calls)
/// should construct their own `Processor` instead.
pub(in crate::term::handler) fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

#[cfg(test)]
mod tests;
