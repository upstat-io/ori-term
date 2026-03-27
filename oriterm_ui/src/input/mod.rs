//! Widget-level input handling: event types, hit testing, and dispatch.
//!
//! Distinct from `hit_test` (window chrome hit testing). This module handles
//! widget-tree traversal, mouse/keyboard event dispatch, and two-phase
//! event propagation (Capture + Bubble).

pub mod dispatch;
mod event;
mod hit_test;

pub use crate::hit_test_behavior::HitTestBehavior;
pub use dispatch::{DeliveryAction, plan_propagation};
pub use event::{
    EventPhase, InputEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    ScrollDelta,
};
pub use hit_test::{
    HitEntry, WidgetHitTestResult, layout_hit_test, layout_hit_test_clipped,
    layout_hit_test_disabled_at, layout_hit_test_path,
};

#[cfg(test)]
mod tests;
