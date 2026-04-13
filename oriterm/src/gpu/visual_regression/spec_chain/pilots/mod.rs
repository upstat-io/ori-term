//! Visual pilot scenarios for the verification chain.
//!
//! Each pilot drives a sequence through every applicable visual rung
//! (1-8) and asserts all pass. Non-visual pilots (DA1) live under
//! `oriterm_core/tests/spec_chain/pilots/` — they don't need GPU access.

pub mod sixel_minimal;
