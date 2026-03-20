//! Event dispatch and lifecycle delivery for the test harness.
//!
//! Implements `process_event()` (delegates to `WindowRoot::dispatch_event()`)
//! and time/animation control methods.

use std::time::Duration;

use crate::input::InputEvent;

use super::harness::WidgetTestHarness;

impl WidgetTestHarness {
    /// Dispatches an input event through the full framework pipeline.
    ///
    /// Delegates to `WindowRoot::dispatch_event()` which handles:
    /// hit testing, hot path updates, lifecycle events, overlay routing,
    /// keymap lookup, controller dispatch, request application, and
    /// frame request forwarding.
    pub(super) fn process_event(&mut self, event: InputEvent) {
        // Track mouse position on the harness (WindowRoot doesn't store it).
        if let Some(pos) = event.pos() {
            self.mouse_pos = pos;
        }
        self.root
            .dispatch_event(&event, &self.measurer, &self.theme, self.clock);
    }

    // -- Public time control API --

    /// Advances the simulated clock by `duration`.
    ///
    /// Ticks animation frames for all widgets that requested them.
    /// Multiple calls accumulate: `advance_time(100ms) + advance_time(100ms)` = 200ms total.
    pub fn advance_time(&mut self, duration: Duration) {
        self.clock += duration;
        self.root.tick_animation(duration, self.clock, &self.theme);
    }

    /// Advances time in 16ms steps until no widgets request animation frames.
    ///
    /// Panics after 300 steps (4.8 seconds simulated) to prevent infinite loops
    /// from buggy animations.
    pub fn run_until_stable(&mut self) {
        let step = Duration::from_millis(16);
        for i in 0..300 {
            self.clock += step;
            let ticked = self.root.tick_animation(step, self.clock, &self.theme);
            if !ticked && !self.root.has_pending_animation_work(self.clock) {
                return;
            }
            if i == 299 {
                panic!(
                    "run_until_stable: still unstable after 300 steps (4.8s simulated). \
                     A widget is continuously requesting animation frames."
                );
            }
        }
    }
}
