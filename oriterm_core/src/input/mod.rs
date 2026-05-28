//! Input-related types shared across the App and core encoders.
//!
//! Houses the canonical `Modifiers` bitflags type per §16.3 SSOT cure
//! (Decision 10 Option A consequence — encoder home moved to core, so
//! its modifier-state input type moved too). App layer keeps the
//! `From<winit::ModifiersState>` impl (winit dep stays App-side).

pub mod modifiers;

pub use modifiers::Modifiers;
