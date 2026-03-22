//! Centralized interaction state manager.
//!
//! `InteractionManager` maintains `InteractionState` for every registered
//! widget in the tree. It computes hot state from hit-test paths, manages
//! active (mouse capture) state, and coordinates focus with `FocusManager`.

use std::collections::{HashMap, HashSet};

use crate::focus::FocusManager;
use crate::widget_id::WidgetId;

use super::lifecycle::LifecycleEvent;
use super::state::{DEFAULT_STATE, InteractionState};

/// Centralized manager for framework-owned widget interaction state.
///
/// Maintains `InteractionState` for every registered widget. Hot state is
/// computed from hit-test paths, active state via explicit set/clear, and
/// focus via coordination with `FocusManager`.
pub struct InteractionManager {
    /// Per-widget interaction state, keyed by `WidgetId`.
    states: HashMap<WidgetId, InteractionState>,
    /// Currently hot widget stack (ordered root to deepest).
    hot_path: Vec<WidgetId>,
    /// Currently active widget (only one at a time).
    active_widget: Option<WidgetId>,
    /// Currently focused widget (only one at a time).
    focused_widget: Option<WidgetId>,
    /// Pending lifecycle events to deliver on next frame.
    pending_events: Vec<LifecycleEvent>,
    /// child → parent map built from the last layout pass.
    parent_map: HashMap<WidgetId, WidgetId>,
    /// Tracks which widgets have received `WidgetAdded` delivery (debug only).
    #[cfg(debug_assertions)]
    widget_added_delivered: HashSet<WidgetId>,
}

impl InteractionManager {
    /// Creates an empty manager with no registered widgets.
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            hot_path: Vec::new(),
            active_widget: None,
            focused_widget: None,
            pending_events: Vec::new(),
            parent_map: HashMap::new(),
            #[cfg(debug_assertions)]
            widget_added_delivered: HashSet::new(),
        }
    }

    /// Registers a widget for interaction state tracking.
    ///
    /// Pushes `WidgetAdded` only on first registration. Subsequent calls
    /// for the same `widget_id` are no-ops.
    pub fn register_widget(&mut self, widget_id: WidgetId) {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = self.states.entry(widget_id) {
            e.insert(InteractionState::default());
            self.pending_events
                .push(LifecycleEvent::WidgetAdded { widget_id });
        }
    }

    /// Removes a widget from interaction state tracking.
    ///
    /// Clears the widget from `hot_path`, `active_widget`, and
    /// `focused_widget` if it held any of those roles. Generates appropriate
    /// lifecycle events for state changes.
    ///
    /// Returns all widget IDs whose interaction state changed (the widget
    /// itself plus any cleared hot/active/focus state).
    pub fn deregister_widget(&mut self, widget_id: WidgetId) -> Vec<WidgetId> {
        let mut changed = vec![widget_id];
        self.states.remove(&widget_id);

        // Clear from hot path and emit HotChanged(false) for removed widgets.
        if self.hot_path.contains(&widget_id) {
            self.hot_path.retain(|&id| id != widget_id);
            self.pending_events.push(LifecycleEvent::HotChanged {
                widget_id,
                is_hot: false,
            });
        }

        // Clear active state.
        if self.active_widget == Some(widget_id) {
            self.active_widget = None;
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id,
                is_active: false,
            });
        }

        // Clear focus state.
        if self.focused_widget == Some(widget_id) {
            self.focused_widget = None;
            // Collect ancestors whose focus_within was cleared.
            for ancestor in self.ancestors(widget_id) {
                if let Some(state) = self.states.get_mut(&ancestor) {
                    if state.focus_within {
                        state.focus_within = false;
                        changed.push(ancestor);
                    }
                }
            }
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id,
                is_focused: false,
            });
        }

        #[cfg(debug_assertions)]
        self.widget_added_delivered.remove(&widget_id);

        self.pending_events
            .push(LifecycleEvent::WidgetRemoved { widget_id });

        changed
    }

    /// Updates the hot path from a pre-computed widget ancestor path.
    ///
    /// `new_path` is ordered root-to-leaf, produced by the Section 02 hit
    /// tester (`layout_hit_test_path`). The last element is `hot_direct`,
    /// all preceding elements are `hot` only. Widgets in the old path but
    /// not the new path get `HotChanged(false)`.
    ///
    /// Pass an empty slice when the pointer leaves the widget area.
    ///
    /// Returns all widget IDs whose hot state changed (both newly-hot and
    /// newly-not-hot). Empty if the path is identical to the previous one.
    pub fn update_hot_path(&mut self, new_path: &[WidgetId]) -> Vec<WidgetId> {
        let mut changed = Vec::new();

        // Find widgets that left the hot path.
        for &old_id in &self.hot_path {
            if !new_path.contains(&old_id) {
                if let Some(state) = self.states.get_mut(&old_id) {
                    state.hot = false;
                    state.hot_direct = false;
                }
                self.pending_events.push(LifecycleEvent::HotChanged {
                    widget_id: old_id,
                    is_hot: false,
                });
                changed.push(old_id);
            }
        }

        // Update hot state for widgets in the new path.
        let len = new_path.len();
        for (i, &id) in new_path.iter().enumerate() {
            let is_leaf = i == len - 1;
            let was_hot = self.hot_path.contains(&id);

            if let Some(state) = self.states.get_mut(&id) {
                state.hot = true;
                state.hot_direct = is_leaf;
            }

            if !was_hot {
                self.pending_events.push(LifecycleEvent::HotChanged {
                    widget_id: id,
                    is_hot: true,
                });
                changed.push(id);
            }
        }

        self.hot_path.clear();
        self.hot_path.extend_from_slice(new_path);

        changed
    }

    /// Sets a widget as active (mouse capture).
    ///
    /// Only one widget can be active at a time. If another widget was active,
    /// generates `ActiveChanged(false)` for it before activating the new one.
    ///
    /// Returns all widget IDs whose active state changed (previous + new).
    /// Empty if the widget was already active.
    pub fn set_active(&mut self, widget_id: WidgetId) -> Vec<WidgetId> {
        // Deactivate previous.
        if let Some(prev) = self.active_widget {
            if prev == widget_id {
                return Vec::new(); // Already active.
            }
            if let Some(state) = self.states.get_mut(&prev) {
                state.active = false;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id: prev,
                is_active: false,
            });

            // Activate new.
            self.active_widget = Some(widget_id);
            if let Some(state) = self.states.get_mut(&widget_id) {
                state.active = true;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id,
                is_active: true,
            });
            vec![prev, widget_id]
        } else {
            // Activate new (no previous).
            self.active_widget = Some(widget_id);
            if let Some(state) = self.states.get_mut(&widget_id) {
                state.active = true;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id,
                is_active: true,
            });
            vec![widget_id]
        }
    }

    /// Clears active state (mouse capture released).
    ///
    /// Must be called on mouse-up. Returns the previously-active widget ID,
    /// if any.
    pub fn clear_active(&mut self) -> Option<WidgetId> {
        if let Some(prev) = self.active_widget.take() {
            if let Some(state) = self.states.get_mut(&prev) {
                state.active = false;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id: prev,
                is_active: false,
            });
            Some(prev)
        } else {
            None
        }
    }

    /// Transfers keyboard focus to `widget_id`.
    ///
    /// Calls `focus_manager.set_focus(widget_id)` to keep `FocusManager` in
    /// sync. Generates `FocusChanged(false)` for the old focused widget and
    /// `FocusChanged(true)` for the new one. Updates `focus_within` for all
    /// ancestors using the stored parent map.
    ///
    /// Returns all widget IDs whose focus or `focus_within` state changed
    /// (old focused, new focused, plus ancestors whose `focus_within` toggled).
    /// Empty if the widget was already focused.
    pub fn request_focus(
        &mut self,
        widget_id: WidgetId,
        focus_manager: &mut FocusManager,
    ) -> Vec<WidgetId> {
        let old_focused = self.focused_widget;

        // Skip if already focused.
        if old_focused == Some(widget_id) {
            return Vec::new();
        }

        let mut changed = Vec::new();

        // Unfocus old.
        if let Some(old_id) = old_focused {
            if let Some(state) = self.states.get_mut(&old_id) {
                state.focused = false;
            }
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id: old_id,
                is_focused: false,
            });
            changed.push(old_id);
        }

        // Focus new.
        self.focused_widget = Some(widget_id);
        focus_manager.set_focus(widget_id);

        if let Some(state) = self.states.get_mut(&widget_id) {
            state.focused = true;
        }
        self.pending_events.push(LifecycleEvent::FocusChanged {
            widget_id,
            is_focused: true,
        });
        changed.push(widget_id);

        // Update focus_within for all ancestors, collecting those that changed.
        self.collect_focus_within_changes(old_focused, Some(widget_id), &mut changed);

        changed
    }

    /// Removes keyboard focus.
    ///
    /// Calls `focus_manager.clear_focus()` to keep `FocusManager` in sync.
    /// Returns all widget IDs whose focus or `focus_within` state changed
    /// (old focused + ancestors whose `focus_within` was cleared).
    pub fn clear_focus(&mut self, focus_manager: &mut FocusManager) -> Vec<WidgetId> {
        let old_focused = self.focused_widget.take();
        focus_manager.clear_focus();

        let mut changed = Vec::new();

        if let Some(old_id) = old_focused {
            if let Some(state) = self.states.get_mut(&old_id) {
                state.focused = false;
            }
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id: old_id,
                is_focused: false,
            });
            changed.push(old_id);
        }

        self.collect_focus_within_changes(old_focused, None, &mut changed);

        changed
    }

    /// Sets a widget's disabled state.
    ///
    /// Queues a `WidgetDisabled` lifecycle event so the widget can perform
    /// cleanup (e.g., clearing pressed state, cancelling a drag).
    /// Returns the widget ID if its disabled state actually changed, `None`
    /// if it was already in the requested state.
    pub fn set_disabled(&mut self, widget_id: WidgetId, disabled: bool) -> Option<WidgetId> {
        if let Some(state) = self.states.get_mut(&widget_id) {
            if state.disabled == disabled {
                return None;
            }
            state.disabled = disabled;
        }

        self.pending_events.push(LifecycleEvent::WidgetDisabled {
            widget_id,
            disabled,
        });
        Some(widget_id)
    }

    /// Returns whether `widget_id` has been registered.
    pub fn is_registered(&self, id: WidgetId) -> bool {
        self.states.contains_key(&id)
    }

    /// Records that `WidgetAdded` has been delivered for this widget (debug only).
    #[cfg(debug_assertions)]
    pub fn mark_widget_added_delivered(&mut self, widget_id: WidgetId) {
        self.widget_added_delivered.insert(widget_id);
    }

    /// Returns whether `WidgetAdded` has ever been delivered (debug only).
    #[cfg(debug_assertions)]
    pub fn was_widget_added_delivered(&self, widget_id: WidgetId) -> bool {
        self.widget_added_delivered.contains(&widget_id)
    }

    /// Returns interaction state for a widget.
    ///
    /// Returns a static default (all `false`) for unregistered widgets
    /// instead of panicking. This handles edge cases during widget tree
    /// rebuilds.
    pub fn get_state(&self, widget_id: WidgetId) -> &InteractionState {
        self.states.get(&widget_id).unwrap_or(&DEFAULT_STATE)
    }

    /// Returns and clears pending lifecycle events.
    pub fn drain_events(&mut self) -> Vec<LifecycleEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Removes widget registrations not present in `valid_ids`.
    ///
    /// Call after widget tree replacement + re-registration to clean up
    /// stale entries from the previous tree. Returns widget IDs whose
    /// state changed (for dirty marking).
    pub fn gc_stale_widgets(&mut self, valid_ids: &[WidgetId]) -> Vec<WidgetId> {
        let valid: HashSet<WidgetId> = valid_ids.iter().copied().collect();
        let stale: Vec<WidgetId> = self
            .states
            .keys()
            .filter(|id| !valid.contains(id))
            .copied()
            .collect();
        let mut changed = Vec::new();
        for id in stale {
            changed.extend(self.deregister_widget(id));
        }
        changed
    }

    /// Stores a new parent map built from the layout tree.
    ///
    /// Called once after each layout pass. The map is used by
    /// `request_focus()` to walk ancestors for `focus_within` updates.
    pub fn set_parent_map(&mut self, map: HashMap<WidgetId, WidgetId>) {
        self.parent_map = map;
    }

    /// Returns a reference to the child → parent map.
    ///
    /// Used by callers that need to pass the parent map to
    /// `InvalidationTracker::mark()` for dirty-ancestor propagation.
    pub fn parent_map_ref(&self) -> &HashMap<WidgetId, WidgetId> {
        &self.parent_map
    }

    /// Returns the currently active widget, if any.
    pub fn active_widget(&self) -> Option<WidgetId> {
        self.active_widget
    }

    /// Returns the currently focused widget, if any.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Returns the root-to-leaf ancestor path for the focused widget.
    ///
    /// Walks the parent map from the focused widget to the root, then
    /// reverses to produce a root-to-leaf path suitable for keyboard event
    /// propagation (Capture: root → focused, Bubble: focused → root).
    /// Returns an empty `Vec` if no widget is focused.
    pub fn focus_ancestor_path(&self) -> Vec<WidgetId> {
        let Some(focused) = self.focused_widget else {
            return Vec::new();
        };

        let mut path = vec![focused];
        let mut current = focused;
        while let Some(&parent) = self.parent_map.get(&current) {
            path.push(parent);
            current = parent;
        }
        path.reverse();
        path
    }

    /// Collects all ancestors of `widget_id` using the parent map.
    fn ancestors(&self, widget_id: WidgetId) -> Vec<WidgetId> {
        let mut result = Vec::new();
        let mut current = widget_id;
        while let Some(&parent) = self.parent_map.get(&current) {
            result.push(parent);
            current = parent;
        }
        result
    }

    /// Updates `focus_within` when focus moves from `old` to `new`,
    /// collecting ancestor IDs whose `focus_within` actually changed.
    fn collect_focus_within_changes(
        &mut self,
        old_focused: Option<WidgetId>,
        new_focused: Option<WidgetId>,
        changed: &mut Vec<WidgetId>,
    ) {
        // Clear focus_within on old ancestors.
        if let Some(old_id) = old_focused {
            for ancestor in self.ancestors(old_id) {
                if let Some(state) = self.states.get_mut(&ancestor) {
                    if state.focus_within {
                        state.focus_within = false;
                        changed.push(ancestor);
                    }
                }
            }
        }

        // Set focus_within on new ancestors (may re-set some already cleared).
        if let Some(new_id) = new_focused {
            for ancestor in self.ancestors(new_id) {
                if let Some(state) = self.states.get_mut(&ancestor) {
                    if !state.focus_within {
                        state.focus_within = true;
                        changed.push(ancestor);
                    }
                }
            }
        }
    }
}

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}
