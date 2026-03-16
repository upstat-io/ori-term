//! Centralized interaction state manager.
//!
//! `InteractionManager` maintains `InteractionState` for every registered
//! widget in the tree. It computes hot state from hit-test paths, manages
//! active (mouse capture) state, and coordinates focus with `FocusManager`.

use std::collections::HashMap;

use crate::focus::FocusManager;
use crate::widget_id::WidgetId;

use super::lifecycle::LifecycleEvent;

/// Per-widget interaction state, managed by the framework.
///
/// Widgets query this via context methods — they never set it directly.
/// All fields default to `false`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "interaction state is a bitfield of 6 orthogonal boolean flags — not control flow"
)]
pub struct InteractionState {
    /// True when the pointer is over this widget or any descendant.
    /// Equivalent to GTK4's `contains-pointer`.
    hot: bool,
    /// True when the pointer is directly over this widget, not a descendant.
    /// Equivalent to GTK4's `is-pointer`.
    hot_direct: bool,
    /// True when the widget has captured mouse events (mouse-down without
    /// mouse-up). While active, the widget receives all mouse events
    /// regardless of pointer position.
    active: bool,
    /// True when this widget has keyboard focus.
    focused: bool,
    /// True when this widget or any descendant has keyboard focus.
    focus_within: bool,
    /// True when the widget is disabled (events are not routed).
    disabled: bool,
}

impl InteractionState {
    /// Creates a new state with all fields `false`.
    pub fn new() -> Self {
        Self {
            hot: false,
            hot_direct: false,
            active: false,
            focused: false,
            focus_within: false,
            disabled: false,
        }
    }

    /// Creates a disabled state (all interaction flags `false`, `disabled` = `true`).
    pub fn disabled() -> Self {
        Self {
            disabled: true,
            ..Self::new()
        }
    }

    /// True when the pointer is over this widget or any descendant.
    pub fn is_hot(&self) -> bool {
        self.hot
    }

    /// True when the pointer is directly over this widget (not a descendant).
    pub fn is_hot_direct(&self) -> bool {
        self.hot_direct
    }

    /// True when the widget has captured mouse events.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// True when this widget has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// True when this widget or any descendant has keyboard focus.
    pub fn has_focus_within(&self) -> bool {
        self.focus_within
    }

    /// True when the widget is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Static default state for unregistered widgets.
///
/// Returned by `get_state` for widgets not in the manager, avoiding panics
/// during widget tree rebuilds.
static DEFAULT_STATE: InteractionState = InteractionState {
    hot: false,
    hot_direct: false,
    active: false,
    focused: false,
    focus_within: false,
    disabled: false,
};

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
        }
    }

    /// Registers a widget for interaction state tracking.
    ///
    /// Inserts a new `InteractionState::new()` entry. Idempotent — if
    /// already registered, this is a no-op.
    pub fn register_widget(&mut self, widget_id: WidgetId) {
        self.states.entry(widget_id).or_default();
        self.pending_events
            .push(LifecycleEvent::WidgetAdded { widget_id });
    }

    /// Removes a widget from interaction state tracking.
    ///
    /// Clears the widget from `hot_path`, `active_widget`, and
    /// `focused_widget` if it held any of those roles. Generates appropriate
    /// lifecycle events for state changes.
    pub fn deregister_widget(&mut self, widget_id: WidgetId) {
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
            self.clear_all_focus_within();
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id,
                is_focused: false,
            });
        }

        self.pending_events
            .push(LifecycleEvent::WidgetRemoved { widget_id });
    }

    /// Updates the hot path from a pre-computed widget ancestor path.
    ///
    /// `new_path` is ordered root-to-leaf, produced by the Section 02 hit
    /// tester (`layout_hit_test_path`). The last element is `hot_direct`,
    /// all preceding elements are `hot` only. Widgets in the old path but
    /// not the new path get `HotChanged(false)`.
    ///
    /// Pass an empty slice when the pointer leaves the widget area.
    pub fn update_hot_path(&mut self, new_path: &[WidgetId]) {
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
            }
        }

        self.hot_path.clear();
        self.hot_path.extend_from_slice(new_path);
    }

    /// Sets a widget as active (mouse capture).
    ///
    /// Only one widget can be active at a time. If another widget was active,
    /// generates `ActiveChanged(false)` for it before activating the new one.
    pub fn set_active(&mut self, widget_id: WidgetId) {
        // Deactivate previous.
        if let Some(prev) = self.active_widget {
            if prev == widget_id {
                return; // Already active.
            }
            if let Some(state) = self.states.get_mut(&prev) {
                state.active = false;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id: prev,
                is_active: false,
            });
        }

        // Activate new.
        self.active_widget = Some(widget_id);
        if let Some(state) = self.states.get_mut(&widget_id) {
            state.active = true;
        }
        self.pending_events.push(LifecycleEvent::ActiveChanged {
            widget_id,
            is_active: true,
        });
    }

    /// Clears active state (mouse capture released).
    ///
    /// Must be called on mouse-up.
    pub fn clear_active(&mut self) {
        if let Some(prev) = self.active_widget.take() {
            if let Some(state) = self.states.get_mut(&prev) {
                state.active = false;
            }
            self.pending_events.push(LifecycleEvent::ActiveChanged {
                widget_id: prev,
                is_active: false,
            });
        }
    }

    /// Transfers keyboard focus to `widget_id`.
    ///
    /// Calls `focus_manager.set_focus(widget_id)` to keep `FocusManager` in
    /// sync. Generates `FocusChanged(false)` for the old focused widget and
    /// `FocusChanged(true)` for the new one. Updates `focus_within` for all
    /// ancestors using the stored parent map.
    pub fn request_focus(&mut self, widget_id: WidgetId, focus_manager: &mut FocusManager) {
        let old_focused = self.focused_widget;

        // Skip if already focused.
        if old_focused == Some(widget_id) {
            return;
        }

        // Unfocus old.
        if let Some(old_id) = old_focused {
            if let Some(state) = self.states.get_mut(&old_id) {
                state.focused = false;
            }
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id: old_id,
                is_focused: false,
            });
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

        // Update focus_within for all ancestors.
        self.update_focus_within(old_focused, Some(widget_id));
    }

    /// Removes keyboard focus.
    ///
    /// Calls `focus_manager.clear_focus()` to keep `FocusManager` in sync.
    pub fn clear_focus(&mut self, focus_manager: &mut FocusManager) {
        let old_focused = self.focused_widget.take();
        focus_manager.clear_focus();

        if let Some(old_id) = old_focused {
            if let Some(state) = self.states.get_mut(&old_id) {
                state.focused = false;
            }
            self.pending_events.push(LifecycleEvent::FocusChanged {
                widget_id: old_id,
                is_focused: false,
            });
        }

        self.update_focus_within(old_focused, None);
    }

    /// Sets a widget's disabled state.
    ///
    /// Queues a `WidgetDisabled` lifecycle event so the widget can perform
    /// cleanup (e.g., clearing pressed state, cancelling a drag).
    pub fn set_disabled(&mut self, widget_id: WidgetId, disabled: bool) {
        if let Some(state) = self.states.get_mut(&widget_id) {
            if state.disabled == disabled {
                return;
            }
            state.disabled = disabled;
        }

        self.pending_events.push(LifecycleEvent::WidgetDisabled {
            widget_id,
            disabled,
        });
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

    /// Stores a new parent map built from the layout tree.
    ///
    /// Called once after each layout pass. The map is used by
    /// `request_focus()` to walk ancestors for `focus_within` updates.
    pub fn set_parent_map(&mut self, map: HashMap<WidgetId, WidgetId>) {
        self.parent_map = map;
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

    /// Updates `focus_within` when focus moves from `old` to `new`.
    fn update_focus_within(
        &mut self,
        old_focused: Option<WidgetId>,
        new_focused: Option<WidgetId>,
    ) {
        // Clear focus_within on old ancestors.
        if let Some(old_id) = old_focused {
            for ancestor in self.ancestors(old_id) {
                if let Some(state) = self.states.get_mut(&ancestor) {
                    state.focus_within = false;
                }
            }
        }

        // Set focus_within on new ancestors.
        if let Some(new_id) = new_focused {
            for ancestor in self.ancestors(new_id) {
                if let Some(state) = self.states.get_mut(&ancestor) {
                    state.focus_within = true;
                }
            }
        }
    }

    /// Clears `focus_within` on all widgets.
    fn clear_all_focus_within(&mut self) {
        for state in self.states.values_mut() {
            state.focus_within = false;
        }
    }
}

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}
