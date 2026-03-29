//! Typed actions for the keymap dispatch system.
//!
//! `KeymapAction` is a trait for semantic actions bound to keystrokes.
//! The `actions!` macro declares unit-struct actions that implement it.
//! Actions are identified by `name()` (e.g., `"widget::Activate"`) and
//! cloneable via `boxed_clone()` for dispatch through the keymap pipeline.

use std::any::Any;
use std::fmt;

/// A semantic action that can be bound to a keystroke in the keymap.
///
/// Actions are typed data — the keymap maps keystrokes to actions, and
/// the dispatch pipeline routes them to widgets via
/// `Widget::handle_keymap_action()`. Each action has a unique name
/// (e.g., `"widget::Activate"`) used for matching during dispatch.
pub trait KeymapAction: Any + fmt::Debug {
    /// Namespace-qualified name (e.g., `"widget::Activate"`).
    fn name(&self) -> &'static str;

    /// Clone this action into a new `Box<dyn KeymapAction>`.
    ///
    /// Required because `Keymap` retains ownership of its binding list
    /// and must clone actions when dispatching matched bindings.
    fn boxed_clone(&self) -> Box<dyn KeymapAction>;

    /// Upcast to `&dyn Any` for `downcast_ref()` in widget handlers.
    fn as_any(&self) -> &dyn Any;
}

/// Declares keymappable actions as unit structs implementing `KeymapAction`.
///
/// Usage:
/// ```ignore
/// actions!(widget, [Activate, Dismiss, FocusNext]);
/// ```
/// Expands to unit structs with `name()` returning `"widget::Activate"` etc.
#[macro_export]
macro_rules! actions {
    ($namespace:ident, [$($action:ident),* $(,)?]) => {
        $(
            /// A keymap action.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $action;

            impl $crate::action::keymap_action::KeymapAction for $action {
                fn name(&self) -> &'static str {
                    concat!(stringify!($namespace), "::", stringify!($action))
                }

                fn boxed_clone(&self) -> Box<dyn $crate::action::keymap_action::KeymapAction> {
                    Box::new(*self)
                }

                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }
            }
        )*
    };
}

// Declare core widget actions for controller migration.
actions!(
    widget,
    [
        Activate,
        NavigateUp,
        NavigateDown,
        Confirm,
        Dismiss,
        FocusNext,
        FocusPrev,
        IncrementValue,
        DecrementValue,
        ValueToMin,
        ValueToMax,
    ]
);

#[cfg(test)]
mod tests;
