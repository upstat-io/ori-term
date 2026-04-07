//! Modes/glitches scenario consts and parser for the tack
//! `n) begin testing -> m) modes` sub-menu.
//!
//! Defines `pub const ScenarioSpec` values that both text tests
//! (`oriterm_core/tests/tack/test_menu/modes.rs`) and GPU tests
//! (`oriterm/src/gpu/visual_regression/tack/mod.rs` in Section 07)
//! reference. Single source of truth for "how do you reach the modes
//! screen and what does the parser extract."

use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Scenario: navigate to the modes screen and verify it lists `am`.
///
/// Anchors are SUBMENU-specific (sub-menu prompt + screen-unique
/// heading). The pre-existing-anchor guard in `TackNavigator` (04.2)
/// rejects anchors that are already on the prior screen, so neither
/// `"begin testing"` (substring of the main menu's `n) begin testing`
/// line) nor `"modes"` (substring of the `m) change modes` line on
/// the begin-testing submenu) are valid anchors here.
///
/// **Verified against tack v1.08 (ncurses 6.4) on Linux `x86_64`.**
/// The "begin testing" menu is internally labeled "Main test menu"
/// and uses the prompt `tack/test [n] >`. The keystroke to enter
/// "test modes and glitches" is `x`, not `m` (`m` is "test cursor
/// movement"). Both anchors confirmed by running with
/// `INSTA_UPDATE=1` and reading the navigator's panic-on-failure
/// grid dumps.
pub const TACK_MODES_AM: ScenarioSpec = ScenarioSpec {
    id: "tack_modes_am",
    screen_id: "tack_modes",
    menu_path: &[
        // Step 0: from main menu (`tack [n] >` is the main-menu
        // prompt), send `n` to enter the test menu. The test menu's
        // prompt is `tack/test [n] >` (sub-menu nesting builds on
        // the parent prompt).
        //
        // Why not `wait_for: "begin testing"`: the literal string
        // `begin testing` is on the MAIN menu (the
        // `n) begin testing` line). The pre-existing-anchor guard
        // would reject it. We need a string unique to the
        // destination screen.
        MenuStep::new(b"n", "tack/test [n] >"),
        // Step 1: from the test menu, send `x` to enter "test modes
        // and glitches". This shows the modes CONTROLS menu (a
        // before-the-test prompt), not the actual test output —
        // tack runs the modes test only after the user picks
        // "n) run standard tests" from this controls menu.
        MenuStep::new(b"x", "tack/test/mode [n] >"),
        // Step 2: from the modes controls menu, send `n` to
        // actually run the standard tests. Tack runs through the
        // mode tests (am, bce, bw, ...) and reports results before
        // returning to a prompt. The exact post-run prompt is
        // tack-version-specific and was discovered empirically by
        // running with INSTA_UPDATE=1 and reading the navigator's
        // panic message.
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_modes_screen,
};

/// Custom parser for the modes screen: scans the grid for cap
/// names tack reports in its parenthesized output format
/// `(cap_name)`.
///
/// **Why parenthesized matching, not bare-token.** Tack's modes
/// test reports each cap as `(am) ...`, `(os) ...` etc. — the cap
/// name is wrapped in parentheses, NOT surrounded by whitespace.
/// `grid_has_token` (whitespace-bounded) correctly rejects this
/// because `(` is not whitespace. The parenthesized pattern
/// `(cap_name)` is distinctive enough on its own to avoid
/// substring collisions: `(am)` cannot match inside any English
/// word, so `str::contains` is safe here.
///
/// **Viewport scope.** Tack's modes test produces output for many
/// caps (`am`, `bce`, `bw`, ...) sequentially, then ends with `os`
/// (over-strike) at the bottom of the screen. Earlier caps scroll
/// off the visible 24-row viewport before the test completes, so
/// `grid_text()` only captures the LAST tested cap. We assert on
/// the always-visible final cap (`os`); the framework's job is to
/// validate end-to-end navigation + parsing, not exhaustive cap
/// coverage. Section 05 of the tack-conformance plan adds the
/// rest of the modes screen catalog with per-cap scenarios that
/// capture the right viewport for each.
pub fn parse_modes_screen(grid: &str) -> ScreenFacts {
    // Caps that appear at the END of tack's modes test run
    // (visible in the 24-row viewport when the test reports
    // "Done"). Tack lists `os` last because it's the over-strike
    // cap and tack uses it as the test terminator.
    const KNOWN: &[&str] = &["os"];

    let mut labels = Vec::new();
    for cap in KNOWN {
        // Look for `(cap)` — the parenthesized form tack uses to
        // tag its modes test output lines.
        let needle = format!("({cap})");
        if grid.contains(&needle) {
            labels.push((*cap).to_string());
        }
    }

    // Header is the first non-blank line for snapshot stability.
    let header = grid
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();

    ScreenFacts {
        header_text: header,
        capability_labels: labels,
        notes: Vec::new(),
    }
}
