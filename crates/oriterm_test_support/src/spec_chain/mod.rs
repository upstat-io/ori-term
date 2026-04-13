//! Verification chain harness for spec conformance tests.
//!
//! Each catalog row gets a test that drives the sequence through every
//! applicable rung (parser → dispatch → state → effect → renderable →
//! frame-input → gpu-instance → texture → golden) and asserts the
//! per-rung observation. A row is `verified` when every rung passes.
//!
//! See `plans/spec-conformance/00-overview.md` for the architecture.

mod api;
pub mod coverage;
pub mod observers;
mod recording_handler;
mod scenario;
pub mod uncataloged;

pub use api::{RungResult, SpecHarness, SpecOutcome};
pub use recording_handler::{DispatchArgs, DispatchCall, RecordingHandler};
pub use scenario::{
    ApexLayer, DispatchExpectation, EffectExpectation, FrameInputExpectation, GoldenExpectation,
    GpuInstanceExpectation, ParserExpectation, RenderableExpectation, RungName,
    ScenarioExpectations, SpecScenario, StateExpectation, TextureExpectation,
};
// PerformAction and PerformObserver live in the vendored VTE crate —
// re-export for convenience.
pub use vte::ansi::{PerformAction, PerformActionCollector, PerformObserver};

#[cfg(test)]
mod tests;
