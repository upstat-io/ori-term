//! `notcurses-info` visual pilot — captures the full notcurses-info output as
//! ori_term's terminal grid renders it, pins via a golden PNG.
//!
//! Apex: `GoldenImage`
//!
//! See: bug-tracker/plans/BUG-06-073/
//!
//! **What this pins.** Driving the captured `notcurses-info` byte stream
//! (`plans/spec-conformance/captures/notcurses-info-full.cap`) through
//! ori_term's `Term` and rendering via `VisualSpecHarness` produces a
//! deterministic PNG of how ori_term would visually present a real
//! `notcurses-info` run. Any cure that changes ori_term's PARSE / GRID /
//! RENDER behavior on those bytes changes the resulting PNG, so this
//! pilot fails — observable signal that the cure took effect.
//!
//! **What this does NOT pin.** Transport-timing bugs (the Windows ConPTY
//! byte-leak that lets ori_term replies arrive after `notcurses-info`
//! exits, ESC-stripped, into bash's stdin) are NOT observable here —
//! replay feeds bytes to `Term` synchronously, so timing is irrelevant.
//! Cure verification for the transport-timing class still needs operator
//! Windows-binary visual verification per
//! `feedback_visual_bugs_need_operator_verification.md`.
//!
//! **Capture provenance.** The `.cap` fixture was generated on Linux WSL
//! via `oriterm_core/tests/notcurses_info_full_capture.rs` (env-gated by
//! `ORITERM_CAPTURE_NOTCURSES_INFO=1`). The capture is the byte stream
//! `notcurses-info` emitted during a real PTY session under
//! `oriterm_test_support::PtySession` with `TERM=xterm-256color`,
//! 142×54 grid.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, RungName, ScenarioExpectations, SpecScenario,
};

use super::super::visual_harness::VisualSpecHarness;

/// Load the captured `notcurses-info` byte stream from the wrapper repo's
/// `captures/` directory. Three-state return distinguishes (a) wrapper
/// absent → graceful SKIP, (b) capture readable, (c) wrapper present but
/// capture unreadable (propagate I/O error).
///
/// `Box::leak` upgrades the runtime `Vec<u8>` to the `&'static [u8]`
/// shape `SpecScenario` requires. The leak is bounded to one allocation
/// per test process (acceptable — test binaries are short-lived) and the
/// scenario genuinely uses the bytes for the entire program lifetime.
fn captured_notcurses_info_bytes() -> Option<&'static [u8]> {
    let captures = oriterm_test_support::paths::captures_dir()?;
    let path = captures.join("notcurses-info-full.cap");
    let bytes = std::fs::read(&path).ok()?;
    Some(Box::leak(bytes.into_boxed_slice()))
}

/// Pilot: replay captured `notcurses-info` bytes through every visual rung;
/// pin the rendered grid against a committed PNG golden.
///
/// First run with `ORITERM_UPDATE_GOLDEN=1` captures the PNG to
/// `oriterm/tests/references/notcurses_info_visual.png`. Subsequent runs
/// strict-compare against that PNG.
#[test]
fn notcurses_info_visual_drives_every_rung_green() {
    let Some(bytes) = captured_notcurses_info_bytes() else {
        eprintln!(
            "SKIP: plans/spec-conformance/captures/notcurses-info-full.cap not present \
             (run `ORITERM_CAPTURE_NOTCURSES_INFO=1 cargo test -p oriterm_core --test \
             notcurses_info_full_capture` to regenerate)"
        );
        return;
    };
    let Some(mut harness) = VisualSpecHarness::with_size(54, 142) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "TEST-NOTCURSES-INFO-VISUAL-REPLAY",
        bytes,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            golden: Some(GoldenExpectation {
                golden_name: Some("notcurses_info_visual"),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);

    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    let rung_names: Vec<_> = results.iter().map(|r| r.rung_name).collect();
    assert_eq!(
        rung_names.len(),
        8,
        "GoldenImage apex should produce exactly 8 rung results, got: {rung_names:?}"
    );
    assert_eq!(
        *rung_names.last().unwrap(),
        RungName::GoldenImage,
        "last rung should be GoldenImage"
    );
}
