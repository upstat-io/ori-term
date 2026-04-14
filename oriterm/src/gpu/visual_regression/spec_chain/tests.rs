use oriterm_core::Rgb;

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GpuInstanceExpectation, RungName, ScenarioExpectations,
    SpecScenario,
};

use super::observers;
use super::visual_harness::VisualSpecHarness;
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::prepared_frame::PreparedFrame;

// ── VisualSpecHarness construction ─────────────────────────────────

#[test]
fn visual_harness_creates_with_default_dimensions() {
    let Some(harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };
    let term = harness.core().term();
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);
}

#[test]
fn visual_harness_creates_with_custom_dimensions() {
    let Some(harness) = VisualSpecHarness::with_size(10, 40) else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };
    let term = harness.core().term();
    assert_eq!(term.grid().lines(), 10);
    assert_eq!(term.grid().cols(), 40);
}

// ── Frame input observer ───────────────────────────────────────────

#[test]
fn frame_input_observer_passes_matching_dimensions() {
    let input = FrameInput::test_grid(80, 24, "");
    let exp = FrameInputExpectation::default_grid();
    let result = observers::observe_frame_input(&input, &exp);
    assert!(result.passed, "expected pass: {result:?}");
}

#[test]
fn frame_input_observer_fails_wrong_cols() {
    let input = FrameInput::test_grid(40, 24, "");
    let exp = FrameInputExpectation::grid(80, 24);
    let result = observers::observe_frame_input(&input, &exp);
    assert!(!result.passed);
    assert!(
        result.failure.as_deref().unwrap().contains("columns"),
        "failure message should mention columns: {:?}",
        result.failure
    );
}

#[test]
fn frame_input_observer_fails_wrong_rows() {
    let input = FrameInput::test_grid(80, 10, "");
    let exp = FrameInputExpectation::grid(80, 24);
    let result = observers::observe_frame_input(&input, &exp);
    assert!(!result.passed);
    assert!(
        result.failure.as_deref().unwrap().contains("rows"),
        "failure message should mention rows: {:?}",
        result.failure
    );
}

#[test]
fn frame_input_observer_skips_none_fields() {
    let input = FrameInput::test_grid(80, 24, "");
    let exp = FrameInputExpectation {
        cols: None,
        rows: None,
        cursor_visible: None,
        reverse_video: None,
    };
    let result = observers::observe_frame_input(&input, &exp);
    assert!(
        result.passed,
        "all-None expectation should pass: {result:?}"
    );
}

// ── GPU instance observer ──────────────────────────────────────────

#[test]
fn gpu_instance_observer_passes_non_empty() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };
    // Feed some text so the prepare phase produces instances.
    harness.core_mut().feed(b"Hello, GPU!");

    let scenario = SpecScenario {
        catalog_row_id: "TEST-GPU-INST",
        bytes: b"X",
        apex_layer: ApexLayer::GpuInstance,
        setup: b"",
        expectations: ScenarioExpectations {
            gpu_instance: Some(GpuInstanceExpectation::non_empty()),
            ..ScenarioExpectations::default()
        },
    };
    let results = harness.run_visual_scenario(&scenario);

    // All rungs through GpuInstance should pass.
    for r in &results {
        assert!(r.passed, "rung {:?} failed: {:?}", r.rung_name, r.failure);
    }
    // GpuInstance rung should be present.
    assert!(
        results.iter().any(|r| r.rung_name == RungName::GpuInstance),
        "GpuInstance rung should be in results"
    );
}

#[test]
fn gpu_instance_observer_fails_min_threshold() {
    // Create a prepared frame with zero instances.
    let prepared = PreparedFrame::new(ViewportSize::new(640, 480), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    let exp = GpuInstanceExpectation::at_least(100, 100);
    let result = observers::observe_gpu_instance(&prepared, &exp);
    assert!(!result.passed);
    assert!(
        result.failure.as_deref().unwrap().contains("backgrounds"),
        "failure message should mention backgrounds: {:?}",
        result.failure
    );
}

// ── Visual scenario end-to-end ─────────────────────────────────────

#[test]
fn visual_scenario_runs_through_frame_input_rung() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "TEST-FRAME-INPUT",
        bytes: b"Hello",
        apex_layer: ApexLayer::FrameInput,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            ..ScenarioExpectations::default()
        },
    };
    let results = harness.run_visual_scenario(&scenario);

    for r in &results {
        assert!(r.passed, "rung {:?} failed: {:?}", r.rung_name, r.failure);
    }
    assert!(
        results.iter().any(|r| r.rung_name == RungName::FrameInput),
        "FrameInput rung should be in results"
    );
}

#[test]
fn visual_scenario_stops_at_first_failure() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "TEST-EARLY-STOP",
        bytes: b"Hello",
        apex_layer: ApexLayer::GpuInstance,
        setup: b"",
        expectations: ScenarioExpectations {
            // Wrong dimensions → FrameInput rung will fail.
            frame_input: Some(FrameInputExpectation::grid(999, 999)),
            gpu_instance: Some(GpuInstanceExpectation::non_empty()),
            ..ScenarioExpectations::default()
        },
    };
    let results = harness.run_visual_scenario(&scenario);

    // Should stop at FrameInput failure — GpuInstance should NOT be reached.
    let last = results.last().expect("should have at least one result");
    assert_eq!(last.rung_name, RungName::FrameInput);
    assert!(!last.passed);
    assert!(
        !results.iter().any(|r| r.rung_name == RungName::GpuInstance),
        "GpuInstance rung should NOT be reached after FrameInput failure"
    );
}
