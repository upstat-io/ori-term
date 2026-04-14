//! Unit tests for the spec_chain verification harness.

use super::*;

#[test]
fn harness_constructs() {
    let harness = SpecHarness::new();
    assert!(harness.outcome().perform_actions.is_empty());
    assert!(harness.outcome().dispatched_calls.is_empty());
    assert!(harness.outcome().effects_emitted.is_empty());
}

#[test]
fn feed_advances_parser_and_captures_effects() {
    let mut harness = SpecHarness::new();
    // Feed a DA1 query: CSI c — triggers identify_terminal(), which emits
    // a PtyEffect::Write with the DA1 response.
    harness.feed(b"\x1b[c");

    // Rung 1: parser should have captured the CSI dispatch.
    assert!(
        !harness.outcome().perform_actions.is_empty(),
        "expected perform_actions after feeding CSI c"
    );

    // Rung 2: handler should have recorded identify_terminal.
    assert!(
        harness
            .outcome()
            .dispatched_calls
            .iter()
            .any(|c| c.method == "identify_terminal"),
        "expected identify_terminal dispatch call, got: {:?}",
        harness.outcome().dispatched_calls
    );

    // Rung 3b: DA1 produces a PTY write effect (the response).
    assert!(
        !harness.outcome().effects_emitted.is_empty(),
        "expected effects after DA1 query"
    );
}

#[test]
fn feed_records_dispatch_calls() {
    let mut harness = SpecHarness::new();
    // CSI 5;10 H = CUP (Cursor Position) → handler.goto(4, 9)
    harness.feed(b"\x1b[5;10H");

    let goto_call = harness
        .outcome()
        .dispatched_calls
        .iter()
        .find(|c| c.method == "goto");
    assert!(goto_call.is_some(), "expected goto dispatch call");

    if let Some(call) = goto_call {
        match &call.args {
            DispatchArgs::Goto { line, col } => {
                // VTE dispatch converts 1-based to 0-based: row 5 → line 4, col 10 → col 9.
                assert_eq!(*line, 4, "expected line=4 (0-based from row 5)");
                assert_eq!(*col, 9, "expected col=9 (0-based from col 10)");
            }
            other => panic!("expected Goto args, got: {other:?}"),
        }
    }
}

#[test]
fn run_scenario_stops_at_first_failed_rung() {
    // Until observers are wired (04.2), run_scenario always passes all
    // rungs. This test verifies the structure: results.len() should equal
    // the number of applicable rungs, and the last rung should match the
    // apex.
    let mut harness = SpecHarness::new();
    let scenario = SpecScenario {
        catalog_row_id: "TEST-CUP",
        bytes: b"\x1b[5;10H",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations::default(),
    };
    let results = harness.run_scenario(&scenario);

    // ApexLayer::State has 3 rungs: Parser, Dispatch, State.
    assert_eq!(results.len(), 3, "expected 3 rungs for State apex");
    assert!(results.iter().all(|r| r.passed), "all rungs should pass");
    assert_eq!(
        results.last().map(|r| r.rung_name),
        Some(RungName::State),
        "last rung should be State"
    );
}

#[test]
fn apex_layer_determines_applicable_rungs() {
    // Verify rung counts for different apex layers.
    assert_eq!(ApexLayer::ParserOnly.rung_chain().len(), 1);
    assert_eq!(ApexLayer::Dispatch.rung_chain().len(), 2);
    assert_eq!(ApexLayer::State.rung_chain().len(), 3);
    assert_eq!(ApexLayer::Renderable.rung_chain().len(), 4);
    assert_eq!(ApexLayer::FrameInput.rung_chain().len(), 5);
    assert_eq!(ApexLayer::GpuInstance.rung_chain().len(), 6);
    assert_eq!(ApexLayer::TextureRender.rung_chain().len(), 7);
    assert_eq!(ApexLayer::GoldenImage.rung_chain().len(), 8);
    // Non-visual: parser + dispatch + effect = 3.
    assert_eq!(ApexLayer::EffectPtyWrite.rung_chain().len(), 3);
    assert_eq!(ApexLayer::EffectClipboard.rung_chain().len(), 3);
    assert_eq!(ApexLayer::EffectHostTitle.rung_chain().len(), 3);
}
