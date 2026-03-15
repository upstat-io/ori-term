//! Framework-managed widget interaction state.
//!
//! Provides the Hot/Active/Focus trifecta (inspired by Druid) so widgets
//! never need to track hover, press, or focus state themselves. The
//! `InteractionManager` computes state from hit-test paths and explicit
//! set/clear calls, then delivers `LifecycleEvent`s to notify widgets
//! of changes.

pub mod lifecycle;
mod manager;
mod parent_map;

pub use manager::{InteractionManager, InteractionState};
pub use parent_map::build_parent_map;

#[cfg(test)]
mod tests;
