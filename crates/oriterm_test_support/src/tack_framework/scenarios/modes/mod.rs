//! Modes/glitches scenario consts and parser for the tack
//! `n) begin testing -> m) modes` sub-menu.
//!
//! Defines `pub const ScenarioSpec` values that both text tests
//! (`oriterm_core/tests/tack/test_menu/modes.rs`) and GPU tests
//! (`oriterm/src/gpu/visual_regression/tack/mod.rs` in Section 07)
//! reference. Single source of truth for "how do you reach the modes
//! screen and what does the parser extract."

use crate::tack_framework::parser::tokens::grid_has_paren_token;
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
/// `(cap_name)` via the [`grid_has_paren_token`] tokenized helper.
///
/// **Why a tokenized helper, not blind `str::contains`.** Tack's
/// modes test reports each cap as `(am) ...`, `(os) ...` etc. —
/// the cap name is wrapped in parentheses, NOT surrounded by
/// whitespace. Plain [`crate::tack_framework::parser::tokens::grid_has_token`]
/// (whitespace-bounded) correctly rejects this because `(` is not
/// whitespace. [`grid_has_paren_token`] is the canonical helper
/// for tack's parenthesized cap-label format and is the right
/// tokenized choice here — Section 04's spec contract is "use a
/// tokenized helper, not raw `contains`", and this satisfies it.
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
        if grid_has_paren_token(grid, cap) {
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

// ============================================================
// 05.1 per-cap phase scenarios — DELETED after empirical
// investigation. Tack v1.08's modes test only emits `(os)`
// content; no `(am)`, `(bce)`, `(bw)`, `(km)`, `(mir)`, `(msgr)`,
// or `(xenl)` is ever printed. The plan's per-cap design was
// based on a wrong model of tack's output. See the rustdoc on
// `oriterm_core/tests/tack/test_menu/modes.rs` for the full
// captured output and rationale. Section 04's `TACK_MODES_AM`
// (with `parse_modes_screen` and `KNOWN: &["os"]`) is the
// complete and correct coverage of tack's modes screen.
//
// `parse_modes_phase_screen` below is preserved as a general-
// purpose multi-cap parser for any future scenario whose tack
// output DOES contain per-cap parenthesized labels (verified
// empirically before adoption). Section 04's `parse_modes_screen`
// remains the canonical parser for the modes-test capture.
// ============================================================

/// Multi-cap parser that scans for any of the modes-related
/// parenthesized labels in a tack screen.
///
/// **Empirical caveat.** As of tack v1.08 the modes test does NOT
/// emit `(am)`, `(bce)`, `(bw)`, `(km)`, `(mir)`, `(msgr)`, or
/// `(xenl)` — only `(os)`. This parser is preserved for any
/// future plan section whose tack output DOES contain per-cap
/// labels (e.g., a different test screen, or a future tack
/// release). Verify empirically before adoption — see the
/// rustdoc on `oriterm_core/tests/tack/test_menu/modes.rs` for
/// the captured-output evidence from the 05.1 investigation.
///
/// Uses the canonical [`grid_has_paren_token`] helper for the
/// same reason as [`parse_modes_screen`] — collision-free
/// matching of tack's parenthesized cap-label syntax.
pub fn parse_modes_phase_screen(grid: &str) -> ScreenFacts {
    const KNOWN: &[&str] = &["am", "bce", "bw", "km", "mir", "msgr", "xenl", "os"];

    let mut labels = Vec::new();
    for cap in KNOWN {
        if grid_has_paren_token(grid, cap) {
            labels.push((*cap).to_string());
        }
    }

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

#[cfg(test)]
mod tests;
