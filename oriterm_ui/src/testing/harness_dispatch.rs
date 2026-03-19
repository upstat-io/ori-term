//! Event dispatch and lifecycle delivery for the test harness.
//!
//! Implements `process_event()` (the internal entry point replicating the full
//! dispatch pipeline) and `deliver_lifecycle_events()` (pre-paint mutation
//! phase for lifecycle and animation frame delivery).

use std::collections::HashMap;
use std::time::Duration;

use crate::action::{Keystroke, build_context_stack};
use crate::animation::AnimFrameEvent;
use crate::controllers::ControllerRequests;
use crate::input::InputEvent;
use crate::input::dispatch::tree::{TreeDispatchResult, deliver_event_to_tree};
use crate::input::layout_hit_test_path;
use crate::pipeline::{
    apply_dispatch_requests, collect_layout_bounds, dispatch_keymap_action, prepaint_widget_tree,
    prepare_widget_tree,
};

use super::harness::WidgetTestHarness;

impl WidgetTestHarness {
    /// Dispatches an input event through the full framework pipeline.
    ///
    /// Steps:
    /// 1. For mouse events: hit test and update hot path.
    /// 2. Drain and deliver lifecycle events from hot path update.
    /// 3. Dispatch event through widget tree (hit test -> propagation -> controllers).
    /// 4. Apply controller requests (SET_ACTIVE, CLEAR_ACTIVE, REQUEST_FOCUS, etc.).
    /// 5. Drain and deliver lifecycle events from request application.
    /// 6. Collect emitted actions into `pending_actions`.
    /// 7. Forward PAINT/ANIM_FRAME request flags to scheduler.
    pub(super) fn process_event(&mut self, event: InputEvent) {
        // Step 1: update hot path for mouse events.
        if let Some(pos) = event.pos() {
            self.mouse_pos = pos;
            let hit_result = layout_hit_test_path(&self.layout, pos);
            let ids = hit_result.widget_ids();
            self.interaction.update_hot_path(&ids);
        }

        // Step 2: deliver lifecycle events from hot path update.
        self.deliver_lifecycle_events();

        // Step 3: dispatch event through widget tree.
        // For keyboard events, try keymap lookup first. If a binding matches,
        // dispatch the action directly; otherwise fall through to controllers.
        let focus_path = self.interaction.focus_ancestor_path();
        let result = match event {
            InputEvent::KeyDown { key, modifiers } => {
                let keystroke = Keystroke::new(key, modifiers);
                let context_stack = build_context_stack(&self.key_contexts, &focus_path);
                if let Some(action) = self.keymap.lookup(keystroke, &context_stack) {
                    let mut r = TreeDispatchResult::new();
                    r.handled = true;
                    match action.name() {
                        "widget::FocusNext" => {
                            r.requests = ControllerRequests::FOCUS_NEXT;
                            r.source = focus_path.last().copied();
                        }
                        "widget::FocusPrev" => {
                            r.requests = ControllerRequests::FOCUS_PREV;
                            r.source = focus_path.last().copied();
                        }
                        _ => {
                            if let Some(focused_id) = focus_path.last().copied() {
                                if let Some(widget_action) = dispatch_keymap_action(
                                    &mut *self.widget,
                                    focused_id,
                                    &*action,
                                    self.viewport,
                                ) {
                                    r.actions.push(widget_action);
                                }
                                r.source = Some(focused_id);
                            }
                        }
                    }
                    self.last_keymap_handled = Some(key);
                    r
                } else {
                    deliver_event_to_tree(
                        &mut *self.widget,
                        &event,
                        self.viewport,
                        Some(&self.layout),
                        self.interaction.active_widget(),
                        &focus_path,
                        self.clock,
                        None,
                    )
                }
            }
            InputEvent::KeyUp { key, .. } if self.last_keymap_handled == Some(key) => {
                // Suppress KeyUp for keys the keymap already handled.
                self.last_keymap_handled = None;
                TreeDispatchResult::new()
            }
            _ => deliver_event_to_tree(
                &mut *self.widget,
                &event,
                self.viewport,
                Some(&self.layout),
                self.interaction.active_widget(),
                &focus_path,
                self.clock,
                None,
            ),
        };

        // Step 4: apply controller requests.
        apply_dispatch_requests(
            result.requests,
            result.source,
            &mut self.interaction,
            &mut self.focus,
        );

        // Step 5: deliver lifecycle events from request application.
        self.deliver_lifecycle_events();

        // Step 6: collect emitted actions.
        self.pending_actions.extend(result.actions);

        // Step 7: forward request flags to scheduler.
        self.flush_frame_requests();
    }

    /// Drains pending lifecycle events and delivers them to the widget tree.
    ///
    /// Mirrors `prepare_widget_tree` from `pipeline.rs`: walks the widget
    /// tree depth-first delivering lifecycle events (HotChanged, ActiveChanged,
    /// FocusChanged) and updating visual state animators.
    pub(super) fn deliver_lifecycle_events(&mut self) {
        let events = self.interaction.drain_events();
        if events.is_empty() {
            return;
        }
        prepare_widget_tree(
            &mut *self.widget,
            &mut self.interaction,
            &events,
            None, // anim_event — handled separately by tick_animation_frame()
            Some(&self.frame_requests),
            self.clock,
        );
        self.run_prepaint();
        self.flush_frame_requests();
    }

    /// Ticks one animation frame at the current time.
    ///
    /// 1. Promote deferred repaints from RenderScheduler.
    /// 2. Take anim frame requests from RenderScheduler.
    /// 3. Build AnimFrameEvent with delta since last frame.
    /// 4. Walk widget tree delivering anim frames and updating visual state.
    /// 5. Forward new frame_requests flags back to RenderScheduler.
    pub(super) fn tick_animation_frame(&mut self, delta: Duration) {
        self.scheduler.promote_deferred(self.clock);
        let anim_widgets = self.scheduler.take_anim_frames();
        if anim_widgets.is_empty() {
            return;
        }
        let anim_event = AnimFrameEvent {
            delta_nanos: delta.as_nanos() as u64,
            now: self.clock,
        };
        prepare_widget_tree(
            &mut *self.widget,
            &mut self.interaction,
            &[], // lifecycle events already drained
            Some(&anim_event),
            Some(&self.frame_requests),
            self.clock,
        );
        self.run_prepaint();
        self.flush_frame_requests();
    }

    // -- Public time control API --

    /// Advances the simulated clock by `duration`.
    ///
    /// Ticks animation frames for all widgets that requested them.
    /// Multiple calls accumulate: `advance_time(100ms) + advance_time(100ms)` = 200ms total.
    pub fn advance_time(&mut self, duration: Duration) {
        self.clock += duration;
        self.tick_animation_frame(duration);
    }

    /// Advances time in 16ms steps until no widgets request animation frames.
    ///
    /// Panics after 300 steps (4.8 seconds simulated) to prevent infinite loops
    /// from buggy animations.
    pub fn run_until_stable(&mut self) {
        let step = Duration::from_millis(16);
        for i in 0..300 {
            self.clock += step;
            self.scheduler.promote_deferred(self.clock);
            let anim_widgets = self.scheduler.take_anim_frames();
            if anim_widgets.is_empty() && !self.scheduler.has_pending_work(self.clock) {
                return;
            }
            if !anim_widgets.is_empty() {
                let anim_event = AnimFrameEvent {
                    delta_nanos: step.as_nanos() as u64,
                    now: self.clock,
                };
                prepare_widget_tree(
                    &mut *self.widget,
                    &mut self.interaction,
                    &[],
                    Some(&anim_event),
                    Some(&self.frame_requests),
                    self.clock,
                );
                self.run_prepaint();
                self.flush_frame_requests();
            }
            if i == 299 {
                panic!(
                    "run_until_stable: still unstable after 300 steps (4.8s simulated). \
                     A widget is continuously requesting animation frames."
                );
            }
        }
    }

    /// Runs the prepaint phase for the widget tree.
    ///
    /// Collects layout bounds from the layout tree and calls
    /// `prepaint_widget_tree` to resolve visual state on each widget.
    pub(super) fn run_prepaint(&mut self) {
        let mut bounds_map = HashMap::new();
        collect_layout_bounds(&self.layout, &mut bounds_map);
        prepaint_widget_tree(
            &mut *self.widget,
            &bounds_map,
            Some(&self.interaction),
            &self.theme,
            self.clock,
            Some(&self.frame_requests),
        );
    }

    /// Forwards accumulated frame request flags to the scheduler.
    pub(super) fn flush_frame_requests(&mut self) {
        if self.frame_requests.anim_frame_requested() {
            // Request anim frames for all registered widgets that need them.
            // In a real app, this is per-widget; in tests we use a simpler model.
            self.scheduler.request_anim_frame(self.widget.id());
        }
        self.frame_requests.reset();
    }
}
