//! Pipeline methods for `WindowRoot`.
//!
//! Layout computation, event dispatch, prepare/rebuild phases. These methods
//! orchestrate the per-frame pipeline that `WidgetTestHarness` (testing) and
//! `WindowContext`/`DialogWindowContext` (production) both need.

use std::collections::HashMap;

use crate::action::{Keystroke, build_context_stack, collect_key_contexts};
use crate::controllers::ControllerRequests;
use crate::input::InputEvent;
use crate::input::dispatch::tree::{TreeDispatchResult, deliver_event_to_tree};
use crate::input::layout_hit_test_path;
use crate::interaction::build_parent_map;
use crate::layout::compute_layout;
use crate::overlay::OverlayEventResult;
use crate::pipeline::{
    apply_dispatch_requests, collect_focusable_ids, collect_layout_bounds, dispatch_keymap_action,
    prepaint_widget_tree, prepare_widget_tree, register_widget_tree,
};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::TextMeasurer;
use crate::widgets::contexts::LayoutCtx;

use super::WindowRoot;

impl WindowRoot {
    /// Recomputes layout from the root widget.
    ///
    /// Calls the widget's `layout()` method with the current viewport
    /// constraints, then rebuilds the parent map, re-registers widgets,
    /// collects key contexts, and rebuilds focus order.
    pub fn compute_layout(&mut self, measurer: &dyn TextMeasurer, theme: &UiTheme) {
        let ctx = LayoutCtx { measurer, theme };
        let layout_box = self.widget.layout(&ctx);
        self.layout = compute_layout(&layout_box, self.viewport);

        // Rebuild parent map for focus_within tracking.
        let parent_map = build_parent_map(&self.layout);
        self.interaction.set_parent_map(parent_map);

        // Register all widget IDs with InteractionManager (idempotent).
        register_widget_tree(&mut *self.widget, &mut self.interaction);

        // Collect key contexts for keymap scope gating.
        self.key_contexts.clear();
        collect_key_contexts(&mut *self.widget, &mut self.key_contexts);

        // Rebuild focus order from tree traversal.
        let mut focusable = Vec::new();
        collect_focusable_ids(&mut *self.widget, &mut focusable);
        self.focus.set_focus_order(focusable);

        self.dirty = true;
    }

    /// Dispatches an event through overlays first, then the widget tree.
    ///
    /// Overlay events take priority — if an overlay handles the event,
    /// the main widget tree does not see it.
    ///
    /// Steps:
    /// 1. For mouse events: hit test and update hot path.
    /// 2. Deliver lifecycle events from hot path update.
    /// 3. For mouse events: route through overlay manager first.
    /// 4. If overlay did not consume: dispatch to widget tree.
    /// 5. Apply controller requests.
    /// 6. Deliver lifecycle events from request application.
    /// 7. Collect emitted actions.
    /// 8. Forward frame request flags to scheduler.
    pub fn dispatch_event(
        &mut self,
        event: &InputEvent,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        now: std::time::Instant,
    ) {
        // Step 1: update hot path for mouse events.
        if let Some(pos) = event.pos() {
            let hit_result = layout_hit_test_path(&self.layout, pos);
            let ids = hit_result.widget_ids();
            self.interaction.update_hot_path(&ids);
        }

        // Step 2: deliver lifecycle events from hot path update.
        self.deliver_lifecycle_events(now, measurer, theme);

        // Step 3: overlay routing for mouse events.
        // If an overlay consumes the event, skip widget tree dispatch.
        let overlay_consumed = if let Some(mouse_event) = (*event).to_mouse_event() {
            let focused = self.focus.focused();
            let result = self.overlays.process_mouse_event(
                &mouse_event,
                measurer,
                theme,
                focused,
                &mut self.layer_tree,
                &mut self.layer_animator,
                now,
            );
            !matches!(result, OverlayEventResult::PassThrough)
        } else {
            false
        };

        // Step 4: dispatch event to widget tree (if overlay didn't consume it).
        let result = if overlay_consumed {
            TreeDispatchResult::new()
        } else {
            self.dispatch_to_tree(event, now)
        };

        // Step 5: apply controller requests.
        apply_dispatch_requests(
            result.requests,
            result.source,
            &mut self.interaction,
            &mut self.focus,
        );

        // Step 6: deliver lifecycle events from request application.
        self.deliver_lifecycle_events(now, measurer, theme);

        // Step 7: collect emitted actions.
        self.pending_actions.extend(result.actions);

        // Step 8: forward request flags to scheduler.
        self.flush_frame_requests();
    }

    /// Runs the pre-paint phase (lifecycle delivery, animation ticks).
    ///
    /// Delivers pending lifecycle events and advances animations for all
    /// widgets that requested animation frames.
    pub fn prepare(&mut self, now: std::time::Instant) {
        let events = self.interaction.drain_events();
        if !events.is_empty() {
            prepare_widget_tree(
                &mut *self.widget,
                &mut self.interaction,
                &events,
                None,
                Some(&self.frame_requests),
                now,
            );
        }
        self.run_prepaint(now, &UiTheme::dark());
        self.flush_frame_requests();
    }

    /// Registers all widgets and rebuilds focus order.
    ///
    /// Called after structural changes (tab add/remove/switch, pane
    /// split/close, widget replacement).
    pub fn rebuild(&mut self) {
        register_widget_tree(&mut *self.widget, &mut self.interaction);

        self.key_contexts.clear();
        collect_key_contexts(&mut *self.widget, &mut self.key_contexts);

        let mut focusable = Vec::new();
        collect_focusable_ids(&mut *self.widget, &mut focusable);
        self.focus.set_focus_order(focusable);

        self.dirty = true;
    }

    // -- Internal helpers --

    /// Dispatches an event through the widget tree (keymap + controller pipeline).
    fn dispatch_to_tree(
        &mut self,
        event: &InputEvent,
        now: std::time::Instant,
    ) -> TreeDispatchResult {
        let focus_path = self.interaction.focus_ancestor_path();
        match *event {
            InputEvent::KeyDown { key, modifiers } => {
                let keystroke = Keystroke::new(key, modifiers);
                let ctx_stack = build_context_stack(&self.key_contexts, &focus_path);
                if let Some(action) = self.keymap.lookup(keystroke, &ctx_stack) {
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
                            if let Some(id) = focus_path.last().copied() {
                                if let Some(wa) = dispatch_keymap_action(
                                    &mut *self.widget,
                                    id,
                                    &*action,
                                    self.viewport,
                                ) {
                                    r.actions.push(wa);
                                }
                                r.source = Some(id);
                            }
                        }
                    }
                    self.last_keymap_handled = Some(key);
                    r
                } else {
                    deliver_event_to_tree(
                        &mut *self.widget,
                        event,
                        self.viewport,
                        Some(&self.layout),
                        self.interaction.active_widget(),
                        &focus_path,
                        now,
                        None,
                    )
                }
            }
            InputEvent::KeyUp { key, .. } if self.last_keymap_handled == Some(key) => {
                self.last_keymap_handled = None;
                TreeDispatchResult::new()
            }
            _ => deliver_event_to_tree(
                &mut *self.widget,
                event,
                self.viewport,
                Some(&self.layout),
                self.interaction.active_widget(),
                &focus_path,
                now,
                None,
            ),
        }
    }

    /// Drains pending lifecycle events and delivers them to the widget tree.
    fn deliver_lifecycle_events(
        &mut self,
        now: std::time::Instant,
        _measurer: &dyn TextMeasurer,
        theme: &UiTheme,
    ) {
        let events = self.interaction.drain_events();
        if events.is_empty() {
            return;
        }
        prepare_widget_tree(
            &mut *self.widget,
            &mut self.interaction,
            &events,
            None,
            Some(&self.frame_requests),
            now,
        );
        self.run_prepaint(now, theme);
        self.flush_frame_requests();
    }

    /// Runs the prepaint phase: collects layout bounds and resolves visual state.
    fn run_prepaint(&mut self, now: std::time::Instant, theme: &UiTheme) {
        let mut bounds_map: HashMap<WidgetId, crate::geometry::Rect> = HashMap::new();
        collect_layout_bounds(&self.layout, &mut bounds_map);
        prepaint_widget_tree(
            &mut *self.widget,
            &bounds_map,
            Some(&self.interaction),
            theme,
            now,
            Some(&self.frame_requests),
        );
    }

    /// Forwards accumulated frame request flags to the scheduler.
    fn flush_frame_requests(&mut self) {
        if self.frame_requests.anim_frame_requested() {
            self.scheduler.request_anim_frame(self.widget.id());
            self.dirty = true;
        }
        self.frame_requests.reset();
    }
}
