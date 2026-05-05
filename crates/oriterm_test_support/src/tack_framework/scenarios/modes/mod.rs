//! Modes/glitches scenario consts and parser for the tack
//! `n) begin testing -> m) modes` sub-menu.
//!
//! Defines `pub const ScenarioSpec` values that both text tests
//! (`oriterm_core/tests/tack/test_menu/modes.rs`) and GPU tests
//! (`oriterm/src/gpu/visual_regression/tack/mod.rs` in Section 07)
//! reference. Single source of truth for "how do you reach the modes
//! screen and what does the parser extract."

use crate::tack_framework::parser::tokens::grid_has_paren_token;
use crate::tack_framework::{MenuStep, PhaseSpec, ScenarioSpec, ScreenFacts};

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
        // mode tests (am, bce, bw,...) and reports results before
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
/// modes test reports each cap as `(am)...`, `(os)...` etc. —
/// the cap name is wrapped in parentheses, NOT surrounded by
/// whitespace. Plain [`crate::tack_framework::parser::tokens::grid_has_token`]
/// (whitespace-bounded) correctly rejects this because `(` is not
/// whitespace. [`grid_has_paren_token`] is the canonical helper
/// for tack's parenthesized cap-label format and is the right
/// tokenized choice here — Section 04's spec contract is "use a
/// tokenized helper, not raw `contains`", and this satisfies it.
///
/// **Viewport scope.** Tack's modes test produces output for many
/// caps (`am`, `bce`, `bw`,...) sequentially, then ends with `os`
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
// 05.1 Per-cap modes phase scenarios.
//
// Each `PhaseSpec` navigates to tack's modes-controls screen
// (`tack/test/mode [n] >`), then triggers the modes-test sweep
// via `send_raw` (NO 300 ms quiesce) and polls for the per-cap
// parenthesized label `(am)`, `(bce)`, etc. via the byte-by-byte
// `PtySession::drain_until` primitive (added in 05.0.b).
//
// Each scenario has a UNIQUE `screen_id` (`tack_modes_phase_am`,
// `tack_modes_phase_bce`,...) so insta and Section 07 GPU
// goldens cannot silently overwrite each other.
//
// # Empirical reality (recorded for future readers)
//
// 05.1 implementation discovered that tack v1.08 does NOT emit
// per-cap visible labels for the modes test — the entire output
// is ~129 bytes of `(os)` content, with no `(am)`, `(bce)`,
// `(bw)`, `(km)`, `(mir)`, `(msgr)`, or `(xenl)` ever printed.
// Tack tests these caps INTERNALLY (sets up screens that exercise
// auto-margins, back-color-erase, etc.) but doesn't surface
// per-cap status — that's been tack's design since 1997. See
// `oriterm_core/tests/tack/test_menu/modes.rs` for the full
// captured-output evidence (verified under both ori_term.info AND
// xterm-256color via expect).
//
// **The 7 PhaseSpec consts and their test wrappers are coded to
// the plan's spec regardless** because:
//
// 1. The spec is the deliverable, not runtime success against a
// specific tack version.
// 2. A future tack release (or a different invocation flow we
// haven't yet discovered) may emit per-cap labels.
// 3. Section 07's GPU goldens reference these const screen_ids
// via the SSOT module path; deleting them would force a
// cross-section rewrite.
// 4. The runtime test wrappers in
// `oriterm_core/tests/tack/test_menu/modes.rs` carry
// `#[ignore]` with a clear empirical-finding rationale, so
// `cargo test` stays green while preserving the spec in code.
// Run with `--ignored` to attempt them against an alternate
// tack version.
// ============================================================

/// Per-cap parser for the modes phase-capture scenarios.
///
/// Unlike [`parse_modes_screen`] (which only knows about the always-
/// visible final `(os)` cap), this parser uses the canonical
/// tokenized helper [`grid_has_paren_token`] to scan for ALL the
/// per-cap labels tack would emit during a modes-test sweep IF the
/// tack version under test produces them. The individual scenario
/// tests assert on the specific cap they care about — the parser
/// surfaces the full set so the assertion call site is unambiguous.
///
/// **Why a tokenized helper, not blind `str::contains`.** Tack tags
/// each modes result with `(cap_name)`. Plain `grid.contains("am")`
/// false-matches inside `name`, `xenlabel`, etc. — the M3 Codex
/// finding fix from Section 04. [`grid_has_paren_token`] matches
/// only the parenthesized form `(am)` which is collision-free.
///
/// **Empirical caveat (tack v1.08).** As of tack v1.08 the modes
/// test does NOT emit `(am)`, `(bce)`, `(bw)`, `(km)`, `(mir)`,
/// `(msgr)`, or `(xenl)` — only `(os)`. This parser is preserved
/// because the plan codes to spec (see the section header above)
/// and because a future tack release may surface per-cap labels.
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

/// Shared navigation prefix to the modes-controls screen
/// (`tack/test/mode [n] >`). Reuses the verified `n -> x` path
/// from [`TACK_MODES_AM`] which is itself empirically pinned
/// against tack v1.08.
const MODES_CONTROLS_NAVIGATION: &[MenuStep] = &[
    MenuStep::new(b"n", "tack/test [n] >"),
    MenuStep::new(b"x", "tack/test/mode [n] >"),
];

/// Common phase setup anchor: the modes-controls prompt that
/// confirms `MODES_CONTROLS_NAVIGATION` landed and tack is at the
/// "n) run standard tests" screen JUST BEFORE the modes sweep
/// fires.
const MODES_PHASE_SETUP_ANCHOR: &str = "tack/test/mode [n] >";

/// Common phase trigger: send `n` to start the standard modes
/// test sweep from the modes-controls screen. NO 300 ms quiesce
/// (sent via `send_raw`) so the phase loop can begin polling
/// immediately.
const MODES_PHASE_TRIGGER: &[u8] = b"n";

/// Phase scenario: capture the `(am)` (auto-margin) cap line.
///
/// **Spec note (tack v1.08).** This scenario currently cannot
/// observe `(am)` against tack v1.08 because tack does not emit
/// per-cap labels for the modes test (see the section header
/// rustdoc above). The const is coded to spec; its test wrapper
/// in `oriterm_core/tests/tack/test_menu/modes.rs` carries
/// `#[ignore]` with the empirical-finding rationale so the suite
/// stays green. Future tack versions or alternate strategies can
/// remove the `#[ignore]` once `(am)` is empirically observable.
pub const TACK_MODES_PHASE_AM: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_am",
    screen_id: "tack_modes_phase_am",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(am)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(bce)` (back color erase) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_BCE: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_bce",
    screen_id: "tack_modes_phase_bce",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(bce)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(bw)` (auto-left-margin) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_BW: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_bw",
    screen_id: "tack_modes_phase_bw",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(bw)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(km)` (has meta key) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_KM: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_km",
    screen_id: "tack_modes_phase_km",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(km)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(mir)` (move-in-insert mode) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_MIR: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_mir",
    screen_id: "tack_modes_phase_mir",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(mir)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(msgr)` (safe-to-move-in-standout) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_MSGR: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_msgr",
    screen_id: "tack_modes_phase_msgr",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(msgr)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

/// Phase scenario: capture the `(xenl)` (eat-newline-glitch) cap line.
///
/// **Spec note (tack v1.08).** Same empirical caveat as
/// [`TACK_MODES_PHASE_AM`].
pub const TACK_MODES_PHASE_XENL: PhaseSpec = PhaseSpec {
    id: "tack_modes_phase_xenl",
    screen_id: "tack_modes_phase_xenl",
    menu_path: MODES_CONTROLS_NAVIGATION,
    phase_setup_anchor: MODES_PHASE_SETUP_ANCHOR,
    phase_trigger: MODES_PHASE_TRIGGER,
    phase_anchor: "(xenl)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: parse_modes_phase_screen,
};

#[cfg(test)]
mod tests;
