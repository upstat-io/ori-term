//! Tack cursor-movement scenario const + parser.
//!
//! The plan envisioned [`parse_cursor_screen`] scanning for the
//! 8 cursor-related capabilities (`cup`, `hpa`, `vpa`, `csr`,
//! `cuu`, `cud`, `cub`, `cuf`) on the captured grid.
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's cursor movement test does NOT emit any of these
//! 8 cap names. The captured output (verified via `expect`
//! against tack v1.08 / xterm-256color on 2026-04-08) is:
//!
//! ```text
//! \x1B[H\x1B[2JThis line should start in the home position.
//! The rest of the screen should be clear. (clear) Done
//! ```
//!
//! That is — the cursor movement test on tack v1.08 only probes
//! the `clear` capability and reports `Done`. Tack DOES exercise
//! `cup`/`hpa`/`vpa`/`csr` INTERNALLY (the test fills the screen
//! with `garbage` lines, then uses cursor home + clear to wipe
//! it — so `cup` and `clear` are both being exercised end-to-end),
//! but only `(clear)` is surfaced as a parenthesized cap name on
//! the captured grid.
//!
//! This mirrors the 05.1/05.2/05.3 pattern — tack v1.08's
//! per-test screens consistently print only ONE cap-shortname
//! per test (`(am)`/`(os)`/`(bel)`/`(colors)`/`(pairs)`/`(clear)`)
//! plus a single descriptive header line.
//!
//! # Hybrid coverage strategy
//!
//! [`TACK_CURSOR_MOVEMENT`] is coded to the plan's spec — the
//! `menu_path` navigates the verified `n -> m -> n` path, the
//! `ready_anchor` is `Done`, the parser is the cursor-cap scanner
//! the plan envisioned. The 3 size-matrix `#[test] fn` wrappers
//! in `oriterm_core/tests/tack/test_menu/cursor_movement.rs`
//! use the hybrid strategy: assert `Done` plus the testable
//! semantic facts (`This line should start in the home position`,
//! `(clear)`) and snapshot the captured grid for visual
//! regression. They do NOT assert on `parsed.capability_labels`
//! because that vec is always empty against tack v1.08.
//!
//! Section 05.5's cap-coverage matrix should record ONLY `clear`
//! as covered by 05.4. The earlier draft of this rustdoc claimed
//! `cup` was transitively covered "since the test demonstrably
//! uses it to home the cursor" — but (Codex
//! /review-work) correctly rejected that claim. `clear` in
//! `extra/ori_term.info` is defined as `\E[H\E[2J`, which
//! already homes the cursor via a LITERAL escape sequence (not
//! via the parameterized `cup` capability). The observed "home
//! position" behavior is therefore explained entirely by
//! `clear`'s definition; `cup` itself is never invoked by tack's
//! cursor movement test on tack v1.08, so claiming it as
//! transitively covered would mask a real `cup` regression.
//! `cup`, `csr`, `hpa`, `vpa`, `cuu`, `cud`, `cub`, `cuf` all
//! must come from a different source — Section 07's GPU goldens
//! for actual cursor movement, vttest menus that DO emit per-cap
//! labels, or a future tack release.
//!
//! Note: tack's submenu prompt is `tack/test/move [n] >` (NOT
//! `tack/test/cursor` or `tack/test/cursor_movement`). The
//! `screen_id` and module name use the more descriptive
//! `cursor_movement` form for clarity, but the `menu_path`
//! anchor strings use the literal `move` form that tack emits.

use crate::tack_framework::parser::tokens::grid_has_token;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Parser for the cursor movement screen: scans for the 8
/// cursor-related capability short names (`cup`, `hpa`, `vpa`,
/// `csr`, `cuu`, `cud`, `cub`, `cuf`) using [`grid_has_token`]
/// (whitespace-bounded).
///
/// All 8 names are 3 characters and would false-positive on bare
/// `str::contains` against any English word containing them —
/// e.g. `cup` matches inside `cupboard`/`occupied`, `vpa` and
/// `hpa` inside arbitrary letter pairs. The tokenized helper is
/// the M3 fix from Section 04 and is mandatory.
///
/// **Empirical caveat (tack v1.08).** As of tack v1.08 the cursor
/// movement screen does NOT emit any of these names — the test
/// only outputs `This line should start in the home position. The
/// rest of the screen should be clear. (clear) Done`. The parser
/// returns an empty `capability_labels` list against the live
/// host. This is preserved as forward-compatible infrastructure
/// for a future tack release that draws per-cap labels on the
/// cursor movement screen.
///
/// Cursor MOVEMENT (the actual `cup`/`hpa`/`vpa`/`csr` pixel
/// effects) is the domain of Section 07's GPU goldens — this
/// parser only verifies the LABELS are present.
pub fn parse_cursor_screen(grid: &str) -> ScreenFacts {
    const CURSOR_CAPS: &[&str] = &["cup", "hpa", "vpa", "csr", "cuu", "cud", "cub", "cuf"];
    let mut found = Vec::new();
    for cap in CURSOR_CAPS {
        if grid_has_token(grid, cap) {
            found.push((*cap).to_string());
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

/// Tack cursor movement scenario.
///
/// Navigates the verified path from 05.0's
/// [`super::begin_testing_inventory::BEGIN_TESTING_INVENTORY`]:
/// `n` (enter test menu) → `m` (enter cursor movement sub-menu,
/// prompt becomes `tack/test/move [n] >` — tack uses the short
/// form `move`, not `cursor`) → `n` (run the standard cursor
/// movement test, terminator is `Done`).
///
/// Anchor strings empirically verified against tack v1.08
/// (2026-04-08) — see the module rustdoc for the captured-output
/// evidence.
pub const TACK_CURSOR_MOVEMENT: ScenarioSpec = ScenarioSpec {
    id: "tack_cursor_movement",
    screen_id: "tack_cursor_movement",
    menu_path: &[
        MenuStep::new(b"n", "tack/test [n] >"),
        MenuStep::new(b"m", "tack/test/move [n] >"),
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_cursor_screen,
};

#[cfg(test)]
mod tests;
