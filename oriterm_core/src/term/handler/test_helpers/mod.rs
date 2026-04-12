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
//! canonical implementation. Algorithmic DRY: 06.5's 21 in-crate
//! direct-VTE tests would otherwise need to copy-paste the
//! `RecordingListener` skeleton into `tack_cap_xcheck/tests.rs`,
//! which is a `LEAK:algorithmic-duplication` finding.
//!
//! Visibility is `pub(super)` (i.e., visible from `handler::`) so
//! the sibling test modules under `handler::` can consume the
//! helpers, but the items remain invisible to consumers outside the
//! `handler` module tree.
//!
//! The strict TDD ordering used to land this move ensured zero
//! behavioral change.

use std::sync::{Arc, Mutex};

use vte::ansi::Processor;

use crate::effect::{LegacyEventSink, QueueingEffectSink};
use crate::event::{Event, EventListener};
use crate::term::Term;
use crate::theme::Theme;

/// Event listener that records all events for assertions.
///
/// Each `send_event` call appends the `Debug`-formatted event to an
/// internal `Vec<String>` so tests can assert on event sequences.
/// The `Arc<Mutex<_>>` interior allows the listener to be cloned
/// before passing it into `Term::new` (which consumes the listener)
/// while still letting the test inspect the recorded events
/// afterwards via the cloned handle.
#[derive(Clone)]
pub(in crate::term::handler) struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingListener {
    /// Construct a fresh recording listener with an empty event log.
    pub(in crate::term::handler) fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot the recorded events as a `Vec<String>` (each entry
    /// is the `Debug` formatting of one captured `Event`).
    pub(in crate::term::handler) fn events(&self) -> Vec<String> {
        self.events.lock().expect("lock poisoned").clone()
    }
}

impl EventListener for RecordingListener {
    fn send_event(&self, event: Event) {
        self.events
            .lock()
            .expect("lock poisoned")
            .push(format!("{event:?}"));
    }
}

/// Create a `Term` with 24 lines, 80 columns, and a recording
/// listener.
pub(in crate::term::handler) fn term_with_recorder()
-> (Term<LegacyEventSink<RecordingListener>>, RecordingListener) {
    term_with_recorder_sized(24, 80)
}

/// Create a `Term` with the given dimensions and a recording
/// listener.
pub(in crate::term::handler) fn term_with_recorder_sized(
    lines: usize,
    cols: usize,
) -> (Term<LegacyEventSink<RecordingListener>>, RecordingListener) {
    let listener = RecordingListener::new();
    let term = Term::new(
        lines,
        cols,
        0,
        Theme::default(),
        LegacyEventSink::new(listener.clone()),
    );
    (term, listener)
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
