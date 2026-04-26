//! Core verification chain harness API.
//!
//! `SpecHarness` wraps `Term<QueueingEffectSink>` (per Section 03 contract)
//! with two recording layers operating in a single pass:
//! - `PerformActionCollector` (rung 1): captures raw `Perform` callbacks
//! - `RecordingHandler` (rung 2): captures semantic `Handler` method calls

use oriterm_core::Term;
use oriterm_core::Theme;
use oriterm_core::effect::Effect;
use oriterm_core::effect::sink::{EffectSink, QueueingEffectSink};

use vte::ansi::{PerformAction, PerformActionCollector, Processor};

use super::observers;
use super::recording_handler::{DispatchCall, RecordingHandler};
use super::scenario::{RungName, SpecScenario};
use super::uncataloged::UncatalogedDetector;

/// Headless verification chain harness for spec conformance tests.
///
/// Wraps `Term<QueueingEffectSink>` with two recording layers
/// operating in a single pass through `advance_with_observer()`.
pub struct SpecHarness {
    handler: RecordingHandler<QueueingEffectSink>,
    processor: Processor,
    perform_observer: PerformActionCollector,
    outcome: SpecOutcome,
    uncataloged: UncatalogedDetector,
}

/// Accumulated observations from feeding bytes through the harness.
///
/// Rungs 1-4 are populated by `SpecHarness::feed()`. Rungs 5-8 are
/// populated by `VisualSpecHarness` (section 04.3b, lives in `oriterm`).
#[derive(Debug, Default, Clone)]
pub struct SpecOutcome {
    /// Rung 1: raw parser actions recorded by `PerformActionCollector`.
    pub perform_actions: Vec<PerformAction>,
    /// Rung 2: semantic handler calls recorded by `RecordingHandler`.
    pub dispatched_calls: Vec<DispatchCall>,
    /// Rung 3b: effects drained from `QueueingEffectSink`.
    pub effects_emitted: Vec<Effect>,
}

/// Result of running a single rung.
#[derive(Debug, Clone)]
pub struct RungResult {
    /// Which rung was executed.
    pub rung_name: RungName,
    /// Whether the rung's assertion passed.
    pub passed: bool,
    /// Failure description (populated when `passed` is false).
    pub failure: Option<String>,
}

impl RungResult {
    /// Create a passing result.
    pub fn pass(rung: RungName) -> Self {
        Self {
            rung_name: rung,
            passed: true,
            failure: None,
        }
    }

    /// Create a failing result.
    pub fn fail(rung: RungName, msg: impl Into<String>) -> Self {
        Self {
            rung_name: rung,
            passed: false,
            failure: Some(msg.into()),
        }
    }
}

/// Default terminal dimensions for harness tests.
const HARNESS_LINES: usize = 24;
const HARNESS_COLS: usize = 80;
const HARNESS_SCROLLBACK: usize = 1000;

impl SpecHarness {
    /// Create a new harness with default terminal dimensions (24×80, 1000
    /// scrollback, dark theme).
    pub fn new() -> Self {
        Self::with_size(HARNESS_LINES, HARNESS_COLS)
    }

    /// Create a new harness with custom terminal dimensions.
    pub fn with_size(lines: usize, cols: usize) -> Self {
        let sink = QueueingEffectSink::new();
        let term = Term::new(lines, cols, HARNESS_SCROLLBACK, Theme::default(), sink);
        let handler = RecordingHandler::new(term);
        let processor = Processor::new();
        let perform_observer = PerformActionCollector::new();
        Self {
            handler,
            processor,
            perform_observer,
            outcome: SpecOutcome::default(),
            uncataloged: UncatalogedDetector::new(),
        }
    }

    /// Feed bytes through the parser and dispatch.
    ///
    /// Uses `Processor::advance_with_observer()` (vendored VTE shim) that:
    /// 1. Records raw `Perform` callbacks via `PerformObserver` (rung 1)
    /// 2. Delegates to the canonical `Performer` which calls `Handler`
    ///    methods on `RecordingHandler` (rung 2: semantic handler calls)
    /// 3. `RecordingHandler` delegates to `Term` (rung 3: state/effects)
    ///
    /// Effects are drained from `Term`'s owned `QueueingEffectSink` via
    /// `drain_into(&self)` (interior `Mutex`).
    pub fn feed(&mut self, bytes: &[u8]) {
        self.processor
            .advance_with_observer(&mut self.handler, &mut self.perform_observer, bytes);
        // Drain rung 1 recordings.
        self.perform_observer
            .drain_into(&mut self.outcome.perform_actions);
        // Drain rung 2 recordings.
        self.handler
            .drain_calls_into(&mut self.outcome.dispatched_calls);
        // Drain effects from Term's owned QueueingEffectSink.
        self.handler
            .term()
            .effect_sink()
            .drain_into(&mut self.outcome.effects_emitted);
        // Feed observed parser actions to the uncataloged detector.
        self.uncataloged.feed_actions(&self.outcome.perform_actions);
    }

    /// Feed a scenario's setup + input bytes without running observers.
    ///
    /// Clears prior recordings before feeding the scenario's own bytes,
    /// so observations reflect only the scenario input. Used by
    /// `VisualSpecHarness` (04.3b) which needs to run its own rung
    /// iteration with GPU observers for rungs 5-8.
    pub fn prepare_scenario(&mut self, scenario: &SpecScenario) {
        if !scenario.setup.is_empty() {
            self.feed(scenario.setup);
        }
        self.outcome.perform_actions.clear();
        self.outcome.dispatched_calls.clear();
        self.outcome.effects_emitted.clear();
        self.feed(scenario.bytes);
    }

    /// Observe a single rung against the captured data.
    ///
    /// Handles headless rungs 1-4 (Parser, Dispatch, State, Effect,
    /// Renderable). Visual rungs 5-8 return `pass` — they are handled
    /// by `VisualSpecHarness` in the `oriterm` crate.
    pub fn observe_rung(
        &self,
        rung: RungName,
        expectations: &super::scenario::ScenarioExpectations,
    ) -> RungResult {
        match rung {
            RungName::Parser => match &expectations.parser {
                Some(e) => observers::observe_parser(&self.outcome, e),
                None => RungResult::pass(rung),
            },
            RungName::Dispatch => match &expectations.dispatch {
                Some(e) => observers::observe_dispatch(&self.outcome, e),
                None => RungResult::pass(rung),
            },
            RungName::State => match &expectations.state {
                Some(e) => observers::observe_state(self.handler.term(), e),
                None => RungResult::pass(rung),
            },
            RungName::Effect => match &expectations.effect {
                Some(e) => observers::observe_effect(&self.outcome, e),
                None => RungResult::pass(rung),
            },
            RungName::Renderable => match expectations.renderable {
                Some(e) => observers::observe_renderable(self.handler.term(), e),
                None => RungResult::pass(rung),
            },
            // Visual rungs 5-8: handled by VisualSpecHarness (04.3b/04.4).
            RungName::FrameInput
            | RungName::GpuInstance
            | RungName::TextureRender
            | RungName::GoldenImage => RungResult::pass(rung),
        }
    }

    /// Run a scenario through every applicable rung, stopping at the
    /// first failure.
    ///
    /// For each rung in the scenario's chain, the corresponding observer
    /// checks the captured data against the scenario's expectations.
    /// Rungs with `None` expectations pass unconditionally (the scenario
    /// doesn't assert that rung). Stops at the first failing rung.
    pub fn run_scenario(&mut self, scenario: &SpecScenario) -> Vec<RungResult> {
        self.prepare_scenario(scenario);

        let mut results = Vec::new();
        for &rung in scenario.applicable_rungs() {
            let result = self.observe_rung(rung, &scenario.expectations);
            let failed = !result.passed;
            results.push(result);
            if failed {
                break;
            }
        }

        results
    }

    /// Borrow the accumulated outcome (for observer assertions).
    pub fn outcome(&self) -> &SpecOutcome {
        &self.outcome
    }

    /// Borrow the inner `Term` (for state rung assertions).
    pub fn term(&self) -> &Term<QueueingEffectSink> {
        self.handler.term()
    }

    /// Mutably borrow the inner `Term` (for calling methods like `set_theme()`).
    pub fn term_mut(&mut self) -> &mut Term<QueueingEffectSink> {
        self.handler.term_mut()
    }

    /// Drain any pending effects from the `Term`'s effect sink into the outcome.
    ///
    /// Call after `term_mut()` operations that push effects (e.g. `set_theme()`
    /// with mode 2031 enabled) so they appear in `outcome().effects_emitted`.
    pub fn drain_effects(&mut self) {
        self.handler
            .term()
            .effect_sink()
            .drain_into(&mut self.outcome.effects_emitted);
    }
}

impl Default for SpecHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SpecHarness {
    fn drop(&mut self) {
        // Spool files go under `target/spec-chain-uncataloged/` (repo-local,
        // cleaned by `cargo clean`). The reader is `spec-coverage-report`,
        // which uses the same SSOT helper to find this dir — keep both ends
        // routed through `paths::term_workspace_root()` per
        // `.claude/rules/test-organization.md §Wrapper/Subrepo Path Discovery`.
        let dir = crate::paths::term_workspace_root().join("target/spec-chain-uncataloged");
        if let Err(e) = self.uncataloged.serialize_to_dir(&dir) {
            eprintln!("warning: failed to serialize uncataloged tuples: {e}");
        }
    }
}

impl SpecHarness {
    /// Borrow the uncataloged detector (for test assertions).
    pub fn uncataloged(&self) -> &UncatalogedDetector {
        &self.uncataloged
    }
}
