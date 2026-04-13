//! Visual rung observers (rungs 5-8) for the verification chain.
//!
//! Each observer is a pure function of captured data + expectation,
//! returning a `RungResult`. Headless rung observers (1-4) live in
//! `oriterm_test_support::spec_chain::observers`.

mod frame_input;
mod golden;
mod gpu_instance;
pub(super) mod texture;

pub use frame_input::observe_frame_input;
pub use golden::observe_golden_image;
pub use gpu_instance::observe_gpu_instance;
pub use texture::{RenderedPixels, observe_texture_render};

#[cfg(test)]
mod tests;
