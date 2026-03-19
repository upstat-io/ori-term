//! Per-widget interaction state flags.
//!
//! `InteractionState` is managed by `InteractionManager` — widgets query
//! it via context methods but never set it directly.

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
    pub(super) hot: bool,
    /// True when the pointer is directly over this widget, not a descendant.
    /// Equivalent to GTK4's `is-pointer`.
    pub(super) hot_direct: bool,
    /// True when the widget has captured mouse events (mouse-down without
    /// mouse-up). While active, the widget receives all mouse events
    /// regardless of pointer position.
    pub(super) active: bool,
    /// True when this widget has keyboard focus.
    pub(super) focused: bool,
    /// True when this widget or any descendant has keyboard focus.
    pub(super) focus_within: bool,
    /// True when the widget is disabled (events are not routed).
    pub(super) disabled: bool,
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

    /// Returns a copy with `hot` and `hot_direct` set.
    #[must_use]
    pub fn with_hot(mut self) -> Self {
        self.hot = true;
        self.hot_direct = true;
        self
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

#[cfg(test)]
impl InteractionState {
    /// Sets the `hot` flag. For use in unit tests only.
    pub(crate) fn set_hot(&mut self, hot: bool) {
        self.hot = hot;
    }

    /// Sets the `active` flag. For use in unit tests only.
    pub(crate) fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Sets the `focused` flag. For use in unit tests only.
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
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
pub(super) static DEFAULT_STATE: InteractionState = InteractionState {
    hot: false,
    hot_direct: false,
    active: false,
    focused: false,
    focus_within: false,
    disabled: false,
};
