//! Per-rung observer functions for the verification chain.
//!
//! Each observer is a pure function of the captured data + expectation,
//! returning a `RungResult`. Observers are composable and testable in
//! isolation.

mod dispatch;
mod effect;
mod parser;
mod renderable;
mod state;

pub use dispatch::observe_dispatch;
pub use effect::observe_effect;
pub use parser::observe_parser;
pub use renderable::observe_renderable;
pub use state::observe_state;

#[cfg(test)]
mod tests;
