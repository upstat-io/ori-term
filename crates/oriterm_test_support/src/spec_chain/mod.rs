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
pub mod effect_filters;
pub mod observers;
mod recording_handler;
mod scenario;
pub mod sixel_fixtures;
pub mod temp_dir_guard;
pub mod uncataloged;

pub use api::{RungResult, SpecHarness, SpecOutcome};
pub use effect_filters::{
    assert_no_pty_writes, count_exact_pty_writes, expect_single_pty_write, last_pty_write,
    pty_write_concat, pty_write_contains, pty_writes, pty_writes_of_kind,
};
pub use recording_handler::{DispatchArgs, DispatchCall, RecordingHandler};
pub use scenario::{
    ApexLayer, DispatchExpectation, EffectExpectation, FrameInputExpectation, GoldenExpectation,
    GpuInstanceExpectation, ParserExpectation, RenderableExpectation, RungName,
    ScenarioExpectations, SpecScenario, StateExpectation, TextureExpectation,
};
pub use temp_dir_guard::TempDirGuard;
// PerformAction and PerformObserver live in the vendored VTE crate —
// re-export for convenience.
pub use vte::ansi::{PerformAction, PerformActionCollector, PerformObserver};

#[cfg(test)]
mod tests;
