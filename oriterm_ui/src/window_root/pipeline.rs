//! Pipeline methods for `WindowRoot`.
//!
//! Layout computation, event dispatch, prepare/rebuild phases. These methods
//! orchestrate the per-frame pipeline that `WidgetTestHarness` (testing) and
//! `WindowContext`/`DialogWindowContext` (production) both need.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::action::{Keystroke, build_context_stack, collect_key_contexts};
use crate::animation::AnimFrameEvent;
use crate::controllers::ControllerRequests;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::input::InputEvent;
use crate::input::dispatch::tree::{TreeDispatchResult, deliver_event_to_tree};
use crate::input::layout_hit_test_path;
use crate::interaction::build_parent_map;
use crate::interaction::lifecycle::LifecycleEvent;
use crate::layout::compute_layout;
use crate::overlay::OverlayEventResult;
use crate::pipeline::{
    apply_dispatch_requests, collect_all_widget_ids, collect_focusable_ids, collect_layout_bounds,
    dispatch_keymap_action, prepaint_widget_tree, prepare_widget_tree, register_widget_tree,
};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::TextMeasurer;
use crate::widgets::contexts::{DrawCtx, LayoutCtx};

use super::WindowRoot;

impl WindowRoot {
    /// Recomputes layout from the root widget.
    ///
    /// Calls the widget's `layout()` method with the current viewport
    /// constraints, then rebuilds the parent map, re-registers widgets,
    /// GCs stale registrations, collects key contexts, and rebuilds focus
    /// order. Safe to call after structural changes (widget add/remove).
    pub fn compute_layout(&mut self, measurer: &dyn TextMeasurer, theme: &UiTheme) {
        let ctx = LayoutCtx { measurer, theme };
        let layout_box = self.widget.layout(&ctx);
        self.layout = compute_layout(&layout_box, self.viewport);

        // Rebuild parent map for focus_within tracking.
        let parent_map = build_parent_map(&self.layout);
        self.interaction.set_parent_map(parent_map);

        // Register all widget IDs with InteractionManager (idempotent).
        register_widget_tree(&mut *self.widget, &mut self.interaction);

        // GC stale interaction registrations from previous tree structure
        // (e.g., after widget replacement or child changes). Matches the
        // GC in `rebuild()` so callers don't need to know which method
        // handles structural cleanup (TPR-04-006).
        let mut valid = Vec::new();
        collect_all_widget_ids(&mut *self.widget, &mut valid);
        let stale = self.interaction.gc_stale_widgets(&valid);
        self.mark_widgets_prepaint_dirty(&stale);

        // Collect key contexts for keymap scope gating.
        self.key_contexts.clear();
        collect_key_contexts(&mut *self.widget, &mut self.key_contexts);

        // Rebuild focus order and sync InteractionManager if focus dropped.
        let mut focusable = Vec::new();
        collect_focusable_ids(&mut *self.widget, &mut focusable);
        self.sync_focus_order(focusable);

        self.dirty = true;
    }

    /// Dispatches an event through overlays first, then the widget tree.
    ///
    /// Overlay events take priority — if an overlay handles the event,
    /// the main widget tree does not see it and the base tree's hot path
    /// is cleared so background widgets don't show hover state underneath.
    ///
    /// Steps:
    /// 1. For mouse events: route through overlay manager first.
    /// 2. For mouse events: update hot path (cleared if overlay consumed).
    /// 3. Deliver lifecycle events from hot path update.
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
        now: Instant,
    ) {
        // Step 1: overlay-first routing for mouse events.
        // Overlays take priority — if an overlay handles the event, the base
        // widget tree's hot path is cleared (not updated) so background widgets
        // don't animate hover state underneath the overlay.
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

        // Step 2: update hot path for mouse events.
        // When overlay consumed: clear the hot path so background widgets
        // lose hover state (they shouldn't be hot while an overlay is active).
        // When overlay did not consume: normal hit-test against widget tree.
        if let Some(pos) = event.pos() {
            let changed = if overlay_consumed {
                self.interaction.update_hot_path(&[])
            } else {
                let hit_result = layout_hit_test_path(&self.layout, pos);
                let ids = hit_result.widget_ids();
                self.interaction.update_hot_path(&ids)
            };
            self.mark_widgets_prepaint_dirty(&changed);
        }

        // Step 3: deliver lifecycle events from hot path update.
        self.deliver_lifecycle_events(now, measurer, theme);

        // Step 4: dispatch event to widget tree (if overlay didn't consume it).
        let result = if overlay_consumed {
            TreeDispatchResult::new()
        } else {
            self.dispatch_to_tree(event, now)
        };

        // Step 5: apply controller requests and mark changed widgets dirty.
        let dispatch_changed = apply_dispatch_requests(
            result.requests,
            result.source,
            &mut self.interaction,
            &mut self.focus,
        );
        self.mark_widgets_prepaint_dirty(&dispatch_changed);

        // Step 5.5: clear focus on mouse-down in non-focusable area.
        // If no controller requested focus on this click, and the event
        // is a mouse down that wasn't consumed by an overlay, clear focus
        // so clicking empty space unfocuses the current input.
        if matches!(event, InputEvent::MouseDown { .. })
            && !overlay_consumed
            && !result.requests.contains(ControllerRequests::REQUEST_FOCUS)
            && self.focus.focused().is_some()
        {
            let changed = self.interaction.clear_focus(&mut self.focus);
            self.mark_widgets_prepaint_dirty(&changed);
        }

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
    pub fn prepare(&mut self, now: Instant, theme: &UiTheme) {
        let events = self.interaction.drain_events();
        if !events.is_empty() {
            prepare_widget_tree(
                &mut *self.widget,
                &mut self.interaction,
                Some(&mut self.invalidation),
                &events,
                None,
                Some(&self.frame_requests),
                now,
            );
        }
        self.run_prepaint(now, theme, true);
        self.flush_frame_requests();
    }

    /// Ticks one animation frame.
    ///
    /// Promotes deferred repaints, takes animation frame requests from the
    /// scheduler, delivers animation events to widgets, runs prepaint, and
    /// flushes frame request flags.
    ///
    /// Bypasses selective walking (passes `None` for the invalidation tracker)
    /// because `flush_frame_requests` only schedules the root widget ID — we
    /// don't know which specific descendants are animating. Full walks during
    /// animation ticks are acceptable since animations are transient (~100-300ms).
    /// Once all animations complete this method returns `false` and the caller
    /// resumes selective walks for interaction-only frames.
    ///
    /// Returns `true` if any animation widgets were ticked.
    pub fn tick_animation(&mut self, delta: Duration, now: Instant, theme: &UiTheme) -> bool {
        self.scheduler.promote_deferred(now);
        let anim_widgets = self.scheduler.take_anim_frames();
        if anim_widgets.is_empty() {
            return false;
        }
        let anim_event = AnimFrameEvent {
            delta_nanos: delta.as_nanos() as u64,
            now,
        };
        // Pass None for tracker: animation ticks must do full tree walks
        // because production clears invalidation after each rendered frame,
        // and we only track the root widget ID in the scheduler — descendant
        // animating widgets would be skipped by selective walks.
        prepare_widget_tree(
            &mut *self.widget,
            &mut self.interaction,
            None,
            &[],
            Some(&anim_event),
            Some(&self.frame_requests),
            now,
        );
        // Full prepaint walk (selective: false) because the prepare step above
        // passed None for the tracker, so it couldn't mark animating widgets
        // dirty. A selective walk here would skip nested animating children.
        self.run_prepaint(now, theme, false);
        self.flush_frame_requests();
        true
    }

    /// Returns whether the scheduler has pending animation work.
    pub fn has_pending_animation_work(&self, now: Instant) -> bool {
        self.scheduler.has_pending_work(now)
    }

    /// Paints the widget tree and returns the resulting scene.
    ///
    /// Runs prepaint to resolve visual state, then paints. Handles the borrow
    /// splitting needed to pass `&InteractionManager` and `&FrameRequestFlags`
    /// to `DrawCtx` while painting through `&mut Widget`.
    pub fn paint(&mut self, measurer: &dyn TextMeasurer, theme: &UiTheme, now: Instant) -> Scene {
        self.run_prepaint(now, theme, true);
        let mut scene = Scene::new();
        let bounds = self.layout.rect;
        let mut ctx = DrawCtx {
            measurer,
            scene: &mut scene,
            bounds,
            now,
            theme,
            icons: None,
            interaction: Some(&self.interaction),
            widget_id: Some(self.widget.id()),
            frame_requests: Some(&self.frame_requests),
        };
        self.widget.paint(&mut ctx);
        scene
    }

    /// Registers all widgets and rebuilds focus order.
    ///
    /// Called after structural changes (tab add/remove/switch, pane
    /// split/close, widget replacement).
    pub fn rebuild(&mut self) {
        register_widget_tree(&mut *self.widget, &mut self.interaction);

        // GC stale interaction registrations from previous tree structure
        // (e.g., after replace_widget or internal child changes). Widgets
        // no longer reachable via for_each_child_mut are deregistered so
        // InteractionManager state doesn't grow monotonically (TPR-11-009).
        let mut valid = Vec::new();
        collect_all_widget_ids(&mut *self.widget, &mut valid);
        let stale = self.interaction.gc_stale_widgets(&valid);
        self.mark_widgets_prepaint_dirty(&stale);

        self.key_contexts.clear();
        collect_key_contexts(&mut *self.widget, &mut self.key_contexts);

        let mut focusable = Vec::new();
        collect_focusable_ids(&mut *self.widget, &mut focusable);
        self.sync_focus_order(focusable);

        self.dirty = true;
    }

    // -- Internal helpers --

    /// Updates focus order and syncs `InteractionManager` if focus is dropped.
    ///
    /// When the focused widget leaves the new order, `FocusManager` clears
    /// focus internally. This method detects that and calls
    /// `InteractionManager::clear_focus()` so both managers stay in sync.
    ///
    /// Used by `rebuild()`, `compute_layout()`, and dialog content handlers
    /// (`reset_dialog_settings`, page switch, keyboard dispatch) to maintain
    /// focus consistency after widget tree changes.
    pub fn sync_focus_order(&mut self, focusable: Vec<WidgetId>) {
        let had_focus = self.focus.focused().is_some();
        self.focus.set_focus_order(focusable);

        if had_focus && self.focus.focused().is_none() {
            let changed = self.interaction.clear_focus(&mut self.focus);
            self.mark_widgets_prepaint_dirty(&changed);
        }
    }

    /// Dispatches an event through the widget tree (keymap + controller pipeline).
    fn dispatch_to_tree(&mut self, event: &InputEvent, now: Instant) -> TreeDispatchResult {
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
        now: Instant,
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
            Some(&mut self.invalidation),
            &events,
            None,
            Some(&self.frame_requests),
            now,
        );
        self.run_prepaint(now, theme, true);
        self.flush_frame_requests();
    }

    /// Runs the prepaint phase: collects layout bounds and resolves visual state.
    ///
    /// When `selective` is true, uses the invalidation tracker to skip clean
    /// subtrees. When false, does a full walk (needed during `tick_animation`
    /// because the prepare phase passes `None` for the tracker and cannot mark
    /// animating widgets dirty for selective walks).
    fn run_prepaint(&mut self, now: Instant, theme: &UiTheme, selective: bool) {
        let mut bounds_map: HashMap<WidgetId, Rect> = HashMap::new();
        collect_layout_bounds(&self.layout, &mut bounds_map);
        let tracker = if selective {
            Some(&self.invalidation)
        } else {
            None
        };
        prepaint_widget_tree(
            &mut *self.widget,
            &bounds_map,
            Some(&self.interaction),
            theme,
            now,
            Some(&self.frame_requests),
            tracker,
        );
    }

    /// Runs `prepare_widget_tree` on every overlay widget.
    ///
    /// Handles borrow splitting: overlays, interaction, and frame requests all
    /// live on `WindowRoot`, so the caller cannot destructure them manually.
    ///
    /// Passes `None` for the invalidation tracker: overlay widgets don't
    /// participate in dirty tracking (interactions route through
    /// `OverlayManager`, not `InteractionManager` hot path), so selective
    /// walks would incorrectly skip all overlay descendants. Full walks are
    /// acceptable since overlay trees are small (dropdown menus, modals).
    pub fn prepare_overlay_widgets(&mut self, lifecycle_events: &[LifecycleEvent], now: Instant) {
        let interaction = &mut self.interaction;
        let flags = &self.frame_requests;
        self.overlays.for_each_widget_mut(|widget| {
            prepare_widget_tree(
                widget,
                interaction,
                None,
                lifecycle_events,
                None,
                Some(flags),
                now,
            );
        });
    }

    /// Runs `prepaint_widget_tree` on every overlay widget.
    ///
    /// Handles borrow splitting: overlays, interaction, and frame requests all
    /// live on `WindowRoot`, so the caller cannot destructure them manually.
    ///
    /// Passes `None` for the invalidation tracker: overlay widgets don't
    /// participate in dirty tracking, so selective walks would skip all
    /// overlay descendants. Full walks are acceptable since overlay trees
    /// are small.
    pub fn prepaint_overlay_widgets(
        &mut self,
        bounds_map: &HashMap<WidgetId, Rect>,
        theme: &UiTheme,
        now: Instant,
    ) {
        let interaction = &self.interaction;
        let flags = &self.frame_requests;
        self.overlays.for_each_widget_mut(|widget| {
            prepaint_widget_tree(
                widget,
                bounds_map,
                Some(interaction),
                theme,
                now,
                Some(flags),
                None,
            );
        });
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
