//! Widget modifiers — reusable wrappers for visibility, display, and pointer behavior.
//!
//! Modifiers wrap a child widget and alter its participation in layout,
//! paint, traversal, or hit testing without changing the child itself.

pub mod pointer_events;
pub mod visibility;

pub use pointer_events::PointerEventsWidget;
pub use visibility::{VisibilityMode, VisibilityWidget};

#[cfg(test)]
mod tests;
