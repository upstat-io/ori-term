//! Tack color scenario const + parser.
//!
//! Color is the highest-value tack screen for `ori_term` in
//! principle: it would test `setaf`/`setab` for both ANSI 16
//! and 256-color modes, plus the named-color list. The plan
//! envisioned [`parse_color_screen`] scanning for the 8 ANSI
//! color names (`black`, `red`, `green`, `yellow`, `blue`,
//! `magenta`, `cyan`, `white`) on the captured grid.
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's color test does NOT emit any of the 8 named
//! ANSI colors. The captured output (verified via `expect`
//! against tack v1.08 / xterm-256color on 2026-04-08) is:
//!
//! ```text
//! \x1B[H\x1B[2JThis terminal can display 256 colors and 32767 color pairs.  (colors) (pairs)
//! (colors) (pairs) Done
//! ```
//!
//! That is — the color test on tack v1.08 only probes the
//! `colors` and `pairs` capabilities (number of colors and
//! number of color pairs) and reports `Done`. Tack tests
//! `setaf`/`setab` INTERNALLY but doesn't surface any visible
//! color sample text or named-color list to the captured grid.
//!
//! This mirrors the 05.1 / 05.2 discoveries — tack v1.08's
//! per-test screens consistently print only the cap shortname
//! in parentheses (`(am)`, `(os)`, `(bel)`, `(colors)`,
//! `(pairs)`) plus a single descriptive header line.
//!
//! # Hybrid coverage strategy
//!
//! [`TACK_COLOR`] is coded to the plan's spec — the
//! `menu_path` navigates the verified `n -> c -> n` path, the
//! `ready_anchor` is `Done`, the parser is the named-color
//! scanner the plan envisioned. The `#[test] fn` wrapper at
//! `oriterm_core/tests/tack/test_menu/color.rs` runs the
//! scenario at three sizes (80x24, 97x33, 120x40) and asserts
//! the captured grid contains `Done` plus the testable
//! semantic facts (`This terminal can display`, `(colors)`,
//! `(pairs)`) — the same hybrid pattern as 05.2's wrappers
//! pinning `(bel)`. The wrapper does NOT assert on the
//! parser's `capability_labels` field because that vec is
//! always empty against tack v1.08 (the parser is preserved
//! as forward-compatible infrastructure for a future tack
//! release that DOES emit named color samples).
//!
//! Section 05.5's cap-coverage matrix should record ONLY
//! `colors` and `pairs` as covered by 05.3 — `setaf`, `setab`,
//! `op`, and the named-color caps must come from a different
//! source (Section 07's GPU goldens for actual color
//! rendering, vttest menus that DO emit color samples, or a
//! future tack release).

use crate::tack_framework::parser::tokens::grid_has_token;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Parser for the color screen: scans for the 8 ANSI 16-color
/// names (`black`, `red`, `green`, `yellow`, `blue`, `magenta`,
/// `cyan`, `white`) using [`grid_has_token`] (whitespace-bounded).
///
/// All eight names are 3-7 characters and would false-positive
/// on bare `str::contains` against any English word containing
/// them — e.g. `red` matches inside `redirect`/`rendered`/`reduce`,
/// `blue` inside `bluefoot`/`bluebird`, `yellow` inside
/// `yellowish`/`yellowstone`. The tokenized helper is the M3
/// fix from Section 04 and is mandatory.
///
/// **Empirical caveat (tack v1.08).** As of tack v1.08 the color
/// screen does NOT emit any of these names — the test only
/// outputs `This terminal can display 256 colors and 32767
/// color pairs. (colors) (pairs) Done`. The parser returns an
/// empty `capability_labels` list against the live host. This
/// is preserved as forward-compatible infrastructure for a
/// future tack release that draws named color samples on the
/// color screen.
///
/// Color RENDERING (the actual `setaf`/`setab` pixel output)
/// is the domain of Section 07's GPU goldens — this parser
/// only verifies the LABELS are present.
pub fn parse_color_screen(grid: &str) -> ScreenFacts {
    const NAMED_COLORS: &[&str] = &[
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
    ];
    let mut found = Vec::new();
    for name in NAMED_COLORS {
        if grid_has_token(grid, name) {
            found.push((*name).to_string());
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
        capability_labels: found,
        notes: Vec::new(),
    }
}

/// Tack color scenario.
///
/// Navigates the verified path from 05.0's
/// [`super::begin_testing_inventory::BEGIN_TESTING_INVENTORY`]:
/// `n` (enter test menu) → `c` (enter color sub-menu, prompt
/// becomes `tack/test/color [n] >`) → `n` (run the standard
/// color test, terminator is `Done`).
///
/// Anchor strings empirically verified against tack v1.08
/// (2026-04-08) — see the module rustdoc for the captured-output
/// evidence.
pub const TACK_COLOR: ScenarioSpec = ScenarioSpec {
    id: "tack_color",
    screen_id: "tack_color",
    menu_path: &[
        MenuStep::new(b"n", "tack/test [n] >"),
        MenuStep::new(b"c", "tack/test/color [n] >"),
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_color_screen,
};

#[cfg(test)]
mod tests;
