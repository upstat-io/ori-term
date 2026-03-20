//! Framework-managed widget interaction state.
//!
//! Provides the Hot/Active/Focus trifecta (inspired by Druid) so widgets
//! never need to track hover, press, or focus state themselves. The
//! `InteractionManager` computes state from hit-test paths and explicit
//! set/clear calls, then delivers `LifecycleEvent`s to notify widgets
//! of changes.

pub mod cursor_hide;
pub mod lifecycle;
mod manager;
pub mod mark_mode;
mod parent_map;
pub mod resize;
pub(crate) mod state;

pub use crate::sense::Sense;
pub use lifecycle::LifecycleEvent;
pub use manager::InteractionManager;
pub use parent_map::build_parent_map;
pub use state::InteractionState;

#[cfg(test)]
mod tests;
