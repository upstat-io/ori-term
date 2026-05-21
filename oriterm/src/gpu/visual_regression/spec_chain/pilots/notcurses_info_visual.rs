//! `notcurses-info` visual pilot — captures the full notcurses-info output as
//! ori_term's terminal grid renders it, pins via a golden PNG.
//!
//! Apex: `GoldenImage`
//!
//! See:
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
    ApexLayer, GoldenExpectation, RungName, ScenarioExpectations, SpecScenario, pty_writes,
};

use super::super::visual_harness::VisualSpecHarness;

/// Load the captured `notcurses-info` byte stream from the wrapper repo's
/// `captures/` directory. Three-state return distinguishes (a) wrapper
/// absent → graceful SKIP, (b) capture readable, (c) wrapper present but
/// capture unreadable (propagate I/O error).
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

/// Load the captured `notcurses-info` reply baseline — every byte
/// ori_term emitted back to `notcurses-info` during the original
/// PtySession capture, in emission order.
/// `None` distinguishes wrapper absent / file unreadable from a
/// genuine empty reply set (which the harness never produces for
/// `notcurses-info` — it always replies to DA1/DA2/DA3/XTVERSION
/// etc.). Three-state return mirrors `captured_notcurses_info_bytes`.
fn captured_notcurses_info_replies() -> Option<Vec<u8>> {
    let captures = oriterm_test_support::paths::captures_dir()?;
    let path = captures.join("notcurses-info-full.replies.cap");
    std::fs::read(&path).ok()
}

/// Pilot: replay captured `notcurses-info` bytes through every visual rung;
/// pin the rendered grid against a committed PNG golden.
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

    // Reply-pinning layer: collect every PtyEffect::Write ori_term emitted
    // while parsing the captured input, concat in emission order, and
    // strict-compare against the captured reply baseline. Any cure that
    // changes ori_term's reply emission for this byte stream (adds a
    // capability we currently don't advertise, fixes a malformed reply
    // shape, removes a stray response) makes this comparison fail and
    // signals the cure took effect.
    // What's pinned: PtyEffect::Write bytes only — DA1/DA2/DA3, XTVERSION,
    // CSI 14t / 18t, kitty graphics replies, XTSMGRAPHICS. Excluded:
    // HostRequest::ColorQuery + HostRequest::ClipboardLoad (those are
    // formatted at push-time by PtySession's PtyResponder, written back
    // to the live PTY but NOT captured into notcurses-info-full.replies.cap
    // — keeping the baseline pure-synchronous-DA/DSR/protocol-replies).
    let Some(reply_baseline) = captured_notcurses_info_replies() else {
        eprintln!(
            "SKIP-PARTIAL: notcurses-info-full.replies.cap not present; \
 visual rungs verified, reply-pinning skipped"
        );
        return;
    };

    let mut emitted: Vec<u8> = Vec::new();
    for (b, _kind) in pty_writes(harness.core()) {
        emitted.extend_from_slice(b);
    }

    if emitted != reply_baseline {
        let baseline_preview = format_byte_preview(&reply_baseline);
        let emitted_preview = format_byte_preview(&emitted);
        panic!(
            "ori_term's reply emission for the captured notcurses-info byte stream \
 diverged from the committed baseline.\n\
 \n\
 baseline ({} bytes): {baseline_preview}\n\
 emitted ({} bytes): {emitted_preview}\n\
 \n\
 If this divergence is the result of a deliberate cure for the \
 notcurses-info wordmark bug, regenerate the baseline:\n\
 \n\
 ORITERM_CAPTURE_NOTCURSES_INFO=1 cargo test -p oriterm_core \
 --test notcurses_info_full_capture\n\
 \n\
 then ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features \
 oriterm/gpu-tests --lib notcurses_info_visual\n\
 \n\
 Operator visual verification on the Windows binary is still required \
 for closure per feedback_visual_bugs_need_operator_verification.md.",
            reply_baseline.len(),
            emitted.len(),
        );
    }
}

/// Render a byte sequence as escape-debug-quoted text capped to the first
/// 120 bytes — keeps panic messages readable when assertions fail on
/// kilobyte-scale replies.
fn format_byte_preview(b: &[u8]) -> String {
    const CAP: usize = 120;
    let slice = if b.len() > CAP { &b[..CAP] } else { b };
    let tail = if b.len() > CAP { "...(truncated)" } else { "" };
    format!(
        "\"{}\"{tail}",
        String::from_utf8_lossy(slice).escape_debug()
    )
}

/// Cross-terminal reference image (WezTerm-rendered `notcurses-info`).
/// Lives at `oriterm/tests/references/notcurses_info_wezterm_reference.png`.
/// Operator-supplied snapshot of how `notcurses-info` SHOULD look on a
/// well-known terminal that already passes the full notcurses capability
/// detection chain. NOT a pixel-equal target — different terminal renders
/// with different font, cell size, color palette, kitty-graphics blit
/// resolution. Used as the visual benchmark for operator-side
/// side-by-side review when ori_term's golden diverges.
///
/// Cross-terminal pixel equality is structurally impossible (different
/// font / cell size / palette / kitty-blit resolution) — this test
/// confirms the reference exists, has non-degenerate dimensions, and
/// can be loaded alongside ori_term's own golden for operator-visual
/// side-by-side comparison per
/// `feedback_visual_bugs_need_operator_verification.md`.
#[test]
fn notcurses_info_wezterm_reference_image_is_loadable() {
    use super::super::super::reference_dir;
    let ref_dir = reference_dir();
    let wezterm_ref_path = ref_dir.join("notcurses_info_wezterm_reference.png");
    let ori_golden_path = ref_dir.join("notcurses_info_visual.png");

    assert!(
        wezterm_ref_path.exists(),
        "WezTerm reference image missing: {} — drop the operator screenshot \
 at this path to enable cross-terminal visual benchmarking",
        wezterm_ref_path.display(),
    );

    let wezterm_ref = image::open(&wezterm_ref_path)
        .unwrap_or_else(|e| panic!("failed to decode {}: {e}", wezterm_ref_path.display()))
        .into_rgba8();
    assert!(
        wezterm_ref.width() > 0 && wezterm_ref.height() > 0,
        "WezTerm reference image has degenerate dimensions: {}x{}",
        wezterm_ref.width(),
        wezterm_ref.height(),
    );

    // Ori_term's own golden — sibling pilot writes this on first
    // `ORITERM_UPDATE_GOLDEN=1` run. Both must coexist for the
    // operator-visual side-by-side review to be meaningful.
    if ori_golden_path.exists() {
        let ori_golden = image::open(&ori_golden_path)
            .unwrap_or_else(|e| panic!("failed to decode {}: {e}", ori_golden_path.display()))
            .into_rgba8();
        eprintln!(
            "notcurses-info visual benchmark loaded:\n \
 ori_term golden: {} ({}x{})\n \
 WezTerm reference: {} ({}x{})\n \
 Operator-visual review compares these side-by-side; cross-terminal \
 pixel equality is structurally impossible (different font, cell size, \
 color palette, kitty-blit resolution).",
            ori_golden_path.display(),
            ori_golden.width(),
            ori_golden.height(),
            wezterm_ref_path.display(),
            wezterm_ref.width(),
            wezterm_ref.height(),
        );
    } else {
        eprintln!(
            "ori_term golden absent ({}); WezTerm reference loaded ({}x{}). \
 Run `notcurses_info_visual_drives_every_rung_green` with \
 `ORITERM_UPDATE_GOLDEN=1` first to generate ori_term's own golden, \
 then operator-visual review can compare the two.",
            ori_golden_path.display(),
            wezterm_ref.width(),
            wezterm_ref.height(),
        );
    }
}
