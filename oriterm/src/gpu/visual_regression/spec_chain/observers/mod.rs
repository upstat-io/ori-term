//! Visual rung observers (rungs 5-8) for the verification chain.
//!
//! Each observer is a pure function of captured data + expectation,
//! returning a `RungResult`. Headless rung observers (1-4) live in
//! `oriterm_test_support::spec_chain::observers`.

mod frame_input;
mod gpu_instance;

pub use frame_input::observe_frame_input;
pub use gpu_instance::observe_gpu_instance;
