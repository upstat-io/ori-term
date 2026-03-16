//! Lifecycle events for widget interaction state changes.
//!
//! These events fire automatically when the framework detects interaction
//! state transitions (hot, active, focus, disabled). They replace the
//! current synthetic `HoverEvent::Enter/Leave` pattern.

use crate::widget_id::WidgetId;

/// A lifecycle event fired by `InteractionManager` when interaction state changes.
///
/// Events are queued during state updates and delivered to widgets via
/// `Widget::lifecycle()` (defined in the widget trait overhaul, Section 08).
/// Until then, consumers drain events via `InteractionManager::drain_events()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Hot state changed. Fires BEFORE the triggering `MouseMove`.
    ///
    /// `is_hot: true` means the pointer entered this widget's subtree.
    /// `is_hot: false` means the pointer left.
    HotChanged {
        /// The widget whose hot state changed.
        widget_id: WidgetId,
        /// New hot state.
        is_hot: bool,
    },

    /// Active (mouse capture) state changed.
    ///
    /// `is_active: true` on mouse-down, `false` on mouse-up or displacement
    /// by another widget's `set_active`.
    ActiveChanged {
        /// The widget whose active state changed.
        widget_id: WidgetId,
        /// New active state.
        is_active: bool,
    },

    /// Keyboard focus changed.
    ///
    /// `is_focused: true` when focus is gained, `false` when lost.
    FocusChanged {
        /// The widget whose focus state changed.
        widget_id: WidgetId,
        /// New focus state.
        is_focused: bool,
    },

    /// Widget added to the tree (for initialization).
    WidgetAdded {
        /// The widget that was added.
        widget_id: WidgetId,
    },

    /// Widget removed from the tree (for cleanup).
    WidgetRemoved {
        /// The widget that was removed.
        widget_id: WidgetId,
    },

    /// Widget disabled state changed.
    ///
    /// Fires when `InteractionManager::set_disabled()` is called so widgets
    /// can perform cleanup on disable (e.g., clearing pressed state,
    /// cancelling an in-flight drag).
    WidgetDisabled {
        /// The widget whose disabled state changed.
        widget_id: WidgetId,
        /// New disabled state (`true` = now disabled).
        disabled: bool,
    },
}

impl LifecycleEvent {
    /// Returns the widget ID this event targets.
    pub fn widget_id(&self) -> WidgetId {
        match *self {
            Self::HotChanged { widget_id, .. }
            | Self::ActiveChanged { widget_id, .. }
            | Self::FocusChanged { widget_id, .. }
            | Self::WidgetAdded { widget_id }
            | Self::WidgetRemoved { widget_id }
            | Self::WidgetDisabled { widget_id, .. } => widget_id,
        }
    }
}
