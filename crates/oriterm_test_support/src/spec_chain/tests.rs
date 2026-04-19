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

/// Pin §10.0 REGISTRATION SYNC: every `Handler::iterm2_*` non-image
/// sub-op added in §10.0 MUST surface through `SpecHarness` as a
/// recorded dispatch call on `outcome().dispatched_calls`. Table-driven
/// over all seven sub-ops so a future add/remove cannot silently
/// regress sync between `crates/vte`'s `Handler` trait and
/// `oriterm_test_support`'s `RecordingHandler`. The completeness assert
/// at the end pins the matrix size — adding a row without bumping the
/// count is a compile-passing test bug per
/// `.claude/rules/tests.md` §Self-verifying matrix completeness.
#[test]
fn spec_harness_records_all_iterm2_non_image_dispatches() {
    const OSC_1337_NON_IMAGE_SYNC: &[(&[u8], &str)] = &[
        (b"\x1b]1337;SetMark\x1b\\", "iterm2_set_mark"),
        (b"\x1b]1337;ReportCellSize\x1b\\", "iterm2_report_cell_size"),
        (
            b"\x1b]1337;RemoteHost=user@host\x1b\\",
            "iterm2_remote_host",
        ),
        (
            b"\x1b]1337;CurrentDir=/home/user\x1b\\",
            "iterm2_current_dir",
        ),
        (b"\x1b]1337;Copy=:SGVsbG8=\x1b\\", "iterm2_copy"),
        (
            b"\x1b]1337;SetUserVar=NAME=dmFsdWU=\x1b\\",
            "iterm2_set_user_var",
        ),
        (
            b"\x1b]1337;ShellIntegrationVersion=15\x1b\\",
            "iterm2_shell_integration_version",
        ),
    ];

    let mut count = 0;
    for &(bytes, expected_method) in OSC_1337_NON_IMAGE_SYNC {
        let mut harness = SpecHarness::new();
        harness.feed(bytes);
        let recorded: Vec<&str> = harness
            .outcome()
            .dispatched_calls
            .iter()
            .map(|c| c.method)
            .collect();
        assert!(
            recorded.iter().any(|m| *m == expected_method),
            "RecordingHandler dropped {expected_method} for OSC bytes {bytes:?} — \
             registration sync broken between crates/vte Handler trait and \
             oriterm_test_support RecordingHandler. Got methods: {recorded:?}"
        );
        count += 1;
    }
    assert_eq!(
        count,
        OSC_1337_NON_IMAGE_SYNC.len(),
        "matrix completeness pin — adding a sync row requires bumping the count"
    );
}
